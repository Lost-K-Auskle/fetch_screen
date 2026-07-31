use super::{ScrollCaptureConfig, StitchResult, OffsetResult};
use super::capture_loop::CaptureLoop;
use super::column_match;
use super::stitching;
use super::scroll_input::ScrollInput;
use crate::capture::CaptureRegion;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// 运行滚动截图主循环
pub fn run_scroll_capture(
    config: &ScrollCaptureConfig,
    region: &CaptureRegion,
    active: &Arc<AtomicBool>,
) -> Result<String, String> {
    let capture_loop = CaptureLoop::new(region);
    let scroll_input = ScrollInput::new();
    let mut stitcher = Stitcher::new(config);

    // 移动鼠标到选区中心 (确保滚轮事件在正确位置)
    let center_x = region.x + region.width as i32 / 2;
    let center_y = region.y + region.height as i32 / 2;
    scroll_input.move_to(center_x, center_y)?;

    // 预测偏移的滑动窗口
    let mut hint_dy: Option<i32> = None;

    loop {
        // 检查停止标志
        if !active.load(Ordering::SeqCst) {
            break;
        }

        // 检查长度限制
        if stitcher.total_length >= config.max_length {
            log::info!("达到最大拼接长度 ({})", config.max_length);
            break;
        }

        // 捕获当前帧
        let frame = capture_loop.capture_frame()?;

        // 第一帧直接作为起始帧
        if stitcher.is_empty() {
            stitcher.add_first_frame(&frame);
            if config.auto_scroll {
                scroll_input.scroll(config.scroll_step)?;
                std::thread::sleep(std::time::Duration::from_millis(config.scroll_delay_ms));
            }
            continue;
        }

        // 计算帧间偏移
        let offset: OffsetResult = if let Some(hint) = hint_dy {
            // Level 1: 列采样 MAD
            let mad_result = column_match::compute_offset(
                stitcher.last_frame(),
                &frame,
                hint,
                config.overlap_ratio,
                &config.direction,
            );
            if mad_result.confidence >= 0.7 {
                mad_result
            } else {
                // Level 2: FFT 相位相关
                let fft_result = stitching::compute_offset_fft(
                    stitcher.last_frame(),
                    &frame,
                    hint,
                    config.overlap_ratio,
                    &config.direction,
                );
                if fft_result.confidence >= 0.5 {
                    fft_result
                } else {
                    // Level 3: 检测是否到达底部
                    check_end_of_scroll(&frame, stitcher.last_frame(), &config.direction)
                }
            }
        } else {
            // 无预测，直接用 FFT
            stitching::compute_offset_fft(
                stitcher.last_frame(),
                &frame,
                0,
                config.overlap_ratio,
                &config.direction,
            )
        };

        // 低置信度 — 可能到达底部
        if offset.confidence < 0.3 {
            log::info!("拼接置信度过低 ({:.2})，可能已到达底部", offset.confidence);
            break;
        }

        // 拼接帧
        stitcher.append_frame(&frame, &offset);

        // 更新 hint
        hint_dy = Some(offset.dy);

        // 自动滚动
        if config.auto_scroll {
            scroll_input.scroll(config.scroll_step)?;
            std::thread::sleep(std::time::Duration::from_millis(config.scroll_delay_ms));
        }
    }

    // 导出最终结果
    let result = stitcher.finalize()?;
    Ok(result)
}

fn check_end_of_scroll(
    current: &[u8],
    previous: &[u8],
    direction: &str,
) -> OffsetResult {
    // 检测连续帧内容是否重复 (到达底部时帧不再变化)
    let similarity = simple_frame_similarity(previous, current);
    if similarity > 0.95 {
        OffsetResult {
            dy: 0,
            dx: 0,
            confidence: 0.0,
            algorithm: "end_detect",
        }
    } else {
        OffsetResult {
            dy: 0,
            dx: 0,
            confidence: 0.4,
            algorithm: "low_confidence",
        }
    }
}

fn simple_frame_similarity(a: &[u8], b: &[u8]) -> f64 {
    if a.len() != b.len() {
        return 0.0;
    }
    // 采样比对各减少计算量
    let step = (a.len() / 1000).max(1);
    let mut matches = 0u64;
    let mut total = 0u64;
    for i in (0..a.len()).step_by(step) {
        if (a[i] as i32 - b[i] as i32).abs() < 5 {
            matches += 1;
        }
        total += 1;
    }
    matches as f64 / total as f64
}

/// 帧拼接器
struct Stitcher {
    frames: Vec<(Vec<u8>, u32, u32)>, // (rgba_data, width, height)
    last_frame_data: Vec<u8>,
    last_frame_width: u32,
    last_frame_height: u32,
    total_length: u32,
    direction: String,
}

impl Stitcher {
    fn new(config: &ScrollCaptureConfig) -> Self {
        Self {
            frames: Vec::new(),
            last_frame_data: Vec::new(),
            last_frame_width: 0,
            last_frame_height: 0,
            total_length: 0,
            direction: config.direction.clone(),
        }
    }

    fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    fn last_frame(&self) -> &[u8] {
        &self.last_frame_data
    }

    fn add_first_frame(&mut self, data: &[u8]) {
        let (width, height) = self.extract_dims_from_context();
        self.last_frame_data = data.to_vec();
        self.last_frame_width = width;
        self.last_frame_height = height;
        self.frames.push((data.to_vec(), width, height));
        self.total_length = if self.direction == "vertical" { height } else { width };
    }

    fn append_frame(&mut self, data: &[u8], offset: &OffsetResult) {
        // 简化的拼接逻辑：跳过重叠部分追加
        // 完整实现需要图像 crate 支持
        let (width, height) = self.extract_dims_from_context();
        self.last_frame_data = data.to_vec();
        self.last_frame_width = width;
        self.last_frame_height = height;
        self.frames.push((data.to_vec(), width, height));

        let overlap = if self.direction == "vertical" {
            offset.dy.max(0) as u32
        } else {
            offset.dx.max(0) as u32
        };

        let new_length = height.saturating_sub(overlap);
        self.total_length += new_length;
    }

    fn extract_dims_from_context(&self) -> (u32, u32) {
        // 默认尺寸 — 实际使用时从帧数据推导
        (self.last_frame_width, self.last_frame_height)
    }

    fn finalize(&self) -> Result<String, String> {
        // 将累积帧拼接为最终图像
        // 简化实现：将所有帧保存为分帧，实际拼接在 column_match 和 stitching 模块中完成
        let cache_dir = crate::capture::ensure_cache_dir();
        let output_path = cache_dir.join(format!("scroll_{}.png", uuid::Uuid::new_v4()));

        // TODO: 完整拼接算法 — 合并 frames 为单一图像
        // 当前存根，实际拼接逻辑将在 Phase 2 完整实现
        if let Some((data, w, h)) = self.frames.first() {
            if let Some(img) = image::RgbaImage::from_raw(*w, *h, data.clone()) {
                let dynamic = image::DynamicImage::ImageRgba8(img);
                dynamic.save(&output_path)
                    .map_err(|e| format!("保存拼接结果失败: {}", e))?;
            }
        }

        Ok(output_path.to_string_lossy().to_string())
    }
}
