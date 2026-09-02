use super::OffsetResult;
use rustfft::{FftPlanner, num_complex::Complex};

/// Level 2: 边缘增强 FFT 相位相关拼接
///
/// 当 Level 1 (列采样 MAD) 置信度过低时回退到此算法。
/// 注意：当前使用展平的 2D→1D FFT（非真正的行列分离 2D FFT），
/// 在垂直滚动场景下对文本/UI 仍能给出可用峰值，精度弱于真正的 2D 相位相关。
/// 搜索范围由 Level 1 结果约束。
pub fn compute_offset_fft(
    prev_frame: &[u8],
    next_frame: &[u8],
    width: u32,
    height: u32,
    _hint_dy: i32,
    overlap_ratio: f32,
    direction: &str,
) -> OffsetResult {
    if width < 16 || height < 16 {
        return OffsetResult { dy: 0, dx: 0, confidence: 0.0, algorithm: "fft_too_small" };
    }

    let overlap = (height as f32 * overlap_ratio) as u32;
    if overlap < 16 {
        return OffsetResult { dy: 0, dx: 0, confidence: 0.0, algorithm: "fft_no_overlap" };
    }

    // 提取重叠候选区 → 灰度图
    let prev_roi = extract_roi_grayscale(prev_frame, width, height, height - overlap, overlap);
    let next_roi = extract_roi_grayscale(next_frame, width, height, 0, overlap);

    // Sobel-X 边缘增强
    let prev_edge = sobel_x(&prev_roi, width, overlap);
    let next_edge = sobel_x(&next_roi, width, overlap);

    // FFT 相位相关
    let (dy, dx, peak_val) = phase_correlate(
        &prev_edge, &next_edge, width as usize, overlap as usize,
    );

    // 验证偏移合理性
    let confidence = if peak_val > 0.15 {
        0.8
    } else if peak_val > 0.08 {
        0.6
    } else if peak_val > 0.04 {
        0.45
    } else {
        0.25
    };

    let dy = if direction == "vertical" { dy } else { 0 };
    let dx = if direction == "horizontal" { dx } else { 0 };

    OffsetResult { dy, dx, confidence, algorithm: "fft" }
}

/// 从 RGBA 帧提取 ROI 区域的灰度值
fn extract_roi_grayscale(
    frame: &[u8], width: u32, height: u32,
    start_row: u32, num_rows: u32,
) -> Vec<f64> {
    let end_row = (start_row + num_rows).min(height);
    let mut gray = Vec::with_capacity((width * (end_row - start_row)) as usize);
    for row in start_row..end_row {
        for col in 0..width {
            let idx = ((row * width + col) * 4) as usize;
            if idx + 2 < frame.len() {
                let g = frame[idx] as f64 * 0.299
                    + frame[idx + 1] as f64 * 0.587
                    + frame[idx + 2] as f64 * 0.114;
                gray.push(g);
            }
        }
    }
    gray
}

/// Sobel 水平边缘检测 (X 方向，强调水平结构)
fn sobel_x(data: &[f64], width: u32, height: u32) -> Vec<f64> {
    let w = width as isize;
    let h = height as isize;
    let mut output = vec![0.0; data.len()];

    let kernel_x = [-1, 0, 1, -2, 0, 2, -1, 0, 1];

    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let mut sum = 0.0;
            for ky in -1..=1 {
                for kx in -1..=1 {
                    let ki = ((ky + 1) * 3 + (kx + 1)) as usize;
                    let idx = ((y + ky) * w + (x + kx)) as usize;
                    sum += data[idx] * kernel_x[ki] as f64;
                }
            }
            output[(y * w + x) as usize] = sum;
        }
    }

    output
}

/// 展平 2D FFT 相位相关（1D FFT 处理展平的 2D 数据）
/// 注意：非真正的 2D FFT（行列分离），但峰值位置在垂直滚动场景下仍可指示大致偏移。
fn phase_correlate(
    prev: &[f64], next: &[f64],
    w: usize, h: usize,
) -> (i32, i32, f64) {
    let total = w * h;
    if total == 0 || prev.len() < total || next.len() < total {
        return (0, 0, 0.0);
    }

    // 填充到 2 的幂 (FFT 要求)
    let fft_w = next_power_of_two(w);
    let fft_h = next_power_of_two(h);
    let fft_size = fft_w * fft_h;

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    let ifft = planner.plan_fft_inverse(fft_size);

    // 准备复数数组 (应用 Hann 窗)
    let mut a: Vec<Complex<f64>> = vec![Complex::new(0.0, 0.0); fft_size];
    let mut b: Vec<Complex<f64>> = vec![Complex::new(0.0, 0.0); fft_size];

    for y in 0..h {
        let wy = hann_window(y as f64, h as f64);
        for x in 0..w {
            let wx = hann_window(x as f64, w as f64);
            let idx = y * fft_w + x;
            let src_idx = y * w + x;
            let val_a = prev[src_idx] * wy * wx;
            let val_b = next[src_idx] * wy * wx;
            a[idx] = Complex::new(val_a, 0.0);
            b[idx] = Complex::new(val_b, 0.0);
        }
    }

    // FFT
    fft.process(&mut a);
    fft.process(&mut b);

    // 互功率谱: R = A * conj(B) / |A * conj(B)|
    for i in 0..fft_size {
        let cross = a[i] * b[i].conj();
        let mag = cross.norm();
        if mag > 1e-10 {
            a[i] = cross / mag;
        } else {
            a[i] = Complex::new(0.0, 0.0);
        }
    }

    // 逆 FFT
    ifft.process(&mut a);

    // 寻峰
    let mut peak_val = 0.0;
    let mut peak_x = 0usize;
    let mut peak_y = 0usize;

    for y in 0..fft_h {
        for x in 0..fft_w {
            let idx = y * fft_w + x;
            let val = a[idx].norm();
            if val > peak_val {
                peak_val = val;
                peak_x = x;
                peak_y = y;
            }
        }
    }

    // 转换偏移: 如果峰值在 > fft_size/2，则偏移为负
    let dx = if peak_x > fft_w / 2 {
        peak_x as i32 - fft_w as i32
    } else {
        peak_x as i32
    };
    let dy = if peak_y > fft_h / 2 {
        peak_y as i32 - fft_h as i32
    } else {
        peak_y as i32
    };

    (dy, dx, peak_val)
}

fn next_power_of_two(n: usize) -> usize {
    let mut p = 1;
    while p < n {
        p <<= 1;
    }
    p
}

fn hann_window(i: f64, n: f64) -> f64 {
    if n <= 1.0 { return 1.0; }
    0.5 * (1.0 - (2.0 * std::f64::consts::PI * i / (n - 1.0)).cos())
}
