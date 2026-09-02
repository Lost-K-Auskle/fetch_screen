use super::OffsetResult;

/// Level 1: 列采样 MAD (Mean Absolute Difference) 快速匹配。
///
/// 采样 3 条全高列，转 1D 灰度信号。对候选滚动量 s（像素），
/// 比较 prev 第 [s, H) 行与 next 第 [0, H-s) 行（滚动后重叠的内容应一致），
/// 搜索最小 MAD 得到实际滚动量 s。
/// 适用：文本/UI 内容（~90% 场景）；失效：纯色、重复网格。
///
/// 返回的 `dy` 是**实际滚动像素量**，`append_frame` 据此把新帧底部 s 行作为新内容追加。
pub fn compute_offset(
    prev_frame: &[u8],
    next_frame: &[u8],
    width: u32,
    height: u32,
    hint_dy: i32,
    overlap_ratio: f32,
    direction: &str,
) -> OffsetResult {
    if width < 10 || height < 10 {
        return OffsetResult { dy: 0, dx: 0, confidence: 0.0, algorithm: "mad_too_small" };
    }

    // 采样 3 条列（全高）
    let col_positions = [width / 4, width / 2, width * 3 / 4];
    let prev_cols: Vec<Vec<u8>> = col_positions
        .iter()
        .map(|&c| extract_column(prev_frame, width, height, c))
        .collect();
    let next_cols: Vec<Vec<u8>> = col_positions
        .iter()
        .map(|&c| extract_column(next_frame, width, height, c))
        .collect();

    // 搜索范围：先在重叠区 [2, overlap_ratio*height] 内搜（覆盖常规滚动速度）；
    // 若匹配不够好（快速滚动导致帧间位移超出重叠区），扩大到全高兜底。
    // hint 附近的候选优先（利于早停）。
    let full_range = height as i32 - 1;
    let overlap_range = ((height as f32 * overlap_ratio) as i32).clamp(16, full_range);
    let mut candidates: Vec<i32> = (2..=overlap_range).collect();
    candidates.sort_by_key(|&s| (s - hint_dy).abs());

    let mut best_s = hint_dy.clamp(2, full_range);
    let mut best_mad = f64::MAX;

    for s in &candidates {
        let mad = mad_3cols(&prev_cols, &next_cols, *s as u32, height);
        if mad < best_mad {
            best_mad = mad;
            best_s = *s;
        }
        if mad < 5.0 {
            break; // 早停
        }
    }

    // 重叠区内没找到足够好的匹配（快速滚动），扩大到全高兜底
    if best_mad >= 15.0 && overlap_range < full_range {
        let mut full: Vec<i32> = ((overlap_range + 1)..=full_range).collect();
        full.sort_by_key(|&s| (s - hint_dy).abs());
        for s in full {
            let mad = mad_3cols(&prev_cols, &next_cols, s as u32, height);
            if mad < best_mad {
                best_mad = mad;
                best_s = s;
            }
            if mad < 5.0 {
                break;
            }
        }
    }

    let confidence = if best_mad < 3.0 {
        0.95
    } else if best_mad < 8.0 {
        0.85
    } else if best_mad < 15.0 {
        0.7
    } else if best_mad < 30.0 {
        0.5
    } else {
        0.3
    };

    let dy = if direction == "vertical" { best_s } else { 0 };
    let dx = if direction == "horizontal" { best_s } else { 0 };

    OffsetResult { dy, dx, confidence, algorithm: "mad" }
}

/// 提取一条全高列的灰度信号
fn extract_column(frame: &[u8], width: u32, height: u32, col: u32) -> Vec<u8> {
    let col = col.min(width - 1);
    let mut signal = Vec::with_capacity(height as usize);
    for row in 0..height {
        let idx = ((row * width + col) * 4) as usize;
        if idx + 2 < frame.len() {
            let gray = (frame[idx] as u32 * 299
                + frame[idx + 1] as u32 * 587
                + frame[idx + 2] as u32 * 114) / 1000;
            signal.push(gray as u8);
        } else {
            signal.push(0);
        }
    }
    signal
}

/// 3 列在滚动量 s 下的 MAD：prev[j] vs next[j - s]，j ∈ [s, H)
fn mad_3cols(prev_cols: &[Vec<u8>], next_cols: &[Vec<u8>], s: u32, height: u32) -> f64 {
    let mut total = 0u64;
    let mut n = 0u64;
    for c in 0..3 {
        let pc = &prev_cols[c];
        let nc = &next_cols[c];
        for j in s..height {
            let ji = j as usize;
            let nj = (j - s) as usize;
            if ji < pc.len() && nj < nc.len() {
                total += (pc[ji] as i32 - nc[nj] as i32).abs() as u64;
                n += 1;
            }
        }
    }
    if n == 0 { f64::MAX } else { total as f64 / n as f64 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_column_all_white() {
        let width = 4u32;
        let height = 4u32;
        let frame = vec![255u8; (width * height * 4) as usize];
        let signal = extract_column(&frame, width, height, 1);
        assert_eq!(signal.len(), 4);
        assert!(signal.iter().all(|&v| v == 255));
    }

    #[test]
    fn test_compute_offset_detects_scroll() {
        let width = 200u32;
        let height = 300u32;
        // 每行独特的亮度纹理，保证只有正确的滚动量才会匹配
        let pattern = |abs_row: u32| -> u8 { ((abs_row * 37 + 11) % 256) as u8 };
        let make = |row_offset: u32| -> Vec<u8> {
            let mut buf = vec![0u8; (width * height * 4) as usize];
            for row in 0..height {
                let g = pattern(row + row_offset);
                for col in 0..width {
                    let idx = ((row * width + col) * 4) as usize;
                    buf[idx] = g;
                    buf[idx + 1] = g;
                    buf[idx + 2] = g;
                    buf[idx + 3] = 255;
                }
            }
            buf
        };

        let frame_a = make(0);
        let frame_b = make(50); // 内容向上滚了 50px

        let off = compute_offset(&frame_a, &frame_b, width, height, 40, 0.6, "vertical");
        assert!(
            (off.dy - 50).abs() <= 1,
            "期望 dy≈50，实际 {}", off.dy
        );
        assert!(off.confidence > 0.8, "置信度过低: {}", off.confidence);
    }
}
