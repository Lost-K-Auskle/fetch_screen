use super::OffsetResult;

/// Level 1: 列采样 MAD (Mean Absolute Difference) 快速匹配
///
/// 原理：对文本/UI 内容而言，垂直方向的位置可通过少数列的亮度特征唯一确定。
/// 采样 3 条垂直列，转 1D 信号后搜索最小 MAD，复杂度 O(9*H) vs FFT 的 O(W*H*log(W*H))。
///
/// 适用：文本编辑器、网页、聊天窗口、代码编辑器 (~90% 场景)
/// 失效：纯色背景、重复网格图案、纯图片流
pub fn compute_offset(
    prev_frame: &[u8],
    next_frame: &[u8],
    hint_dy: i32,
    overlap_ratio: f32,
    direction: &str,
) -> OffsetResult {
    // 从帧数据推算图像尺寸 (假设正方形 RGBA)
    let prev_pixels = (prev_frame.len() / 4) as u32;
    let next_pixels = (next_frame.len() / 4) as u32;

    // 无法确定尺寸时回退到 FFT
    if prev_pixels == 0 || next_pixels == 0 || prev_pixels != next_pixels {
        return OffsetResult {
            dy: 0,
            dx: 0,
            confidence: 0.0,
            algorithm: "mad_fallback",
        };
    }

    // 估算宽高 (假设正方形区域)
    let side = (prev_pixels as f64).sqrt() as u32;
    if side < 10 {
        return OffsetResult {
            dy: 0,
            dx: 0,
            confidence: 0.0,
            algorithm: "mad_too_small",
        };
    }

    let width = side;
    let height = side;

    // 取重叠候选区
    let overlap_height = (height as f32 * overlap_ratio) as u32;
    if overlap_height < 8 {
        return OffsetResult {
            dy: 0,
            dx: 0,
            confidence: 0.0,
            algorithm: "mad_no_overlap",
        };
    }

    // 采样 3 条列 (25%, 50%, 75% 宽度)
    let col_positions = [width / 4, width / 2, width * 3 / 4];

    // 从 prev 底部提取列信号
    let prev_signals: Vec<Vec<u8>> = col_positions
        .iter()
        .map(|&col| extract_column_signal(prev_frame, width, height, col, height - overlap_height, overlap_height))
        .collect();

    // 从 next 顶部提取列信号
    let next_signals: Vec<Vec<u8>> = col_positions
        .iter()
        .map(|&col| extract_column_signal(next_frame, width, height, col, 0, overlap_height))
        .collect();

    // 以 hint_dy 为中心搜索最佳偏移
    let search_range = (overlap_height as i32 / 2).min(200);
    let search_start = (hint_dy - search_range).max(1);
    let search_end = (hint_dy + search_range).min(overlap_height as i32 - 1);

    let mut best_dy = hint_dy;
    let mut best_mad = f64::MAX;

    // 交替搜索 (从 hint 向外扩展)
    let mut offsets: Vec<i32> = Vec::new();
    offsets.push(hint_dy);
    for i in 1..=search_range {
        offsets.push(hint_dy + i);
        offsets.push(hint_dy - i);
    }

    for dy in offsets {
        if dy < 0 || dy as u32 >= overlap_height {
            continue;
        }

        let mad = compute_mad_3cols(
            &prev_signals,
            &next_signals,
            dy as u32,
            overlap_height,
        );

        if mad < best_mad {
            best_mad = mad;
            best_dy = dy;
        }

        // 早停：MAD < 阈值则直接接受
        if mad < 5.0 {
            break;
        }
    }

    // 将 MAD 映射到置信度
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

    let dy = if direction == "vertical" { best_dy } else { 0 };
    let dx = if direction == "horizontal" { best_dy } else { 0 };

    OffsetResult {
        dy,
        dx,
        confidence,
        algorithm: "mad",
    }
}

/// 从 RGBA 帧中提取一条列的灰度信号
fn extract_column_signal(
    frame: &[u8],
    width: u32,
    height: u32,
    col: u32,
    start_row: u32,
    num_rows: u32,
) -> Vec<u8> {
    let col = col.min(width - 1);
    let end_row = (start_row + num_rows).min(height);
    let mut signal = Vec::with_capacity((end_row - start_row) as usize);

    for row in start_row..end_row {
        let idx = ((row * width + col) * 4) as usize;
        if idx + 2 < frame.len() {
            // 灰度 = 0.299R + 0.587G + 0.114B
            let gray = (frame[idx] as u32 * 299
                + frame[idx + 1] as u32 * 587
                + frame[idx + 2] as u32 * 114)
                / 1000;
            signal.push(gray as u8);
        }
    }

    signal
}

/// 计算 3 条列在两个偏移位置的 MAD 均值
fn compute_mad_3cols(
    prev_signals: &[Vec<u8>],
    next_signals: &[Vec<u8>],
    dy: u32,
    overlap_height: u32,
) -> f64 {
    let mut total_diff = 0u64;
    let mut total_samples = 0u64;

    for col_idx in 0..3 {
        let prev = &prev_signals[col_idx];
        let next = &next_signals[col_idx];

        // prev 从顶部开始，next 从 dy 位置开始
        let max_i = (overlap_height - dy).min(prev.len() as u32).min(next.len() as u32);

        for i in 0..max_i as usize {
            if i < prev.len() && (i + dy as usize) < next.len() {
                let diff = (prev[i] as i32 - next[i + dy as usize] as i32).abs() as u64;
                total_diff += diff;
                total_samples += 1;
            }
        }
    }

    if total_samples == 0 {
        return f64::MAX;
    }

    total_diff as f64 / total_samples as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_signal_extraction() {
        // 创建 4x4 的 RGBA 测试帧 (全白)
        let width = 4u32;
        let height = 4u32;
        let frame = vec![255u8; (width * height * 4) as usize];

        let signal = extract_column_signal(&frame, width, height, 1, 0, 4);
        assert_eq!(signal.len(), 4);
        assert!(signal.iter().all(|&v| v == 255));
    }

    #[test]
    fn test_mad_computation() {
        let prev = vec![vec![100u8; 10], vec![100u8; 10], vec![100u8; 10]];
        let next = vec![vec![100u8; 10], vec![100u8; 10], vec![100u8; 10]];
        let mad = compute_mad_3cols(&prev, &next, 0, 10);
        assert_eq!(mad, 0.0);
    }
}
