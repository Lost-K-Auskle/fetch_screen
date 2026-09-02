use super::{ScrollCaptureConfig, ScrollProgress, OffsetResult};
use super::capture_loop::CaptureLoop;
use super::column_match;
use super::scroll_input::ScrollInput;
use crate::capture::CaptureRegion;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;

/// 滚动模式
#[derive(Debug, Clone, PartialEq)]
pub enum ScrollMode {
    /// 用户手动滚动，我们持续截帧拼接
    Manual,
    /// 系统自动发送滚轮事件
    Auto,
}

/// 运行滚动截图主循环
pub fn run_scroll_capture(
    config: &ScrollCaptureConfig,
    region: &CaptureRegion,
    active: &Arc<AtomicBool>,
    app: &tauri::AppHandle,
    mode: ScrollMode,
) -> Result<String, String> {
    // 选区坐标是虚拟桌面图像相对坐标（0,0 = 虚拟桌面原点）；
    // BitBlt 截帧 / SetCursorPos 移动光标都需要屏幕绝对坐标，这里统一加上原点偏移。
    let (ox, oy) = crate::capture::screen::virtual_desktop_origin();
    let abs_region = CaptureRegion {
        x: region.x + ox,
        y: region.y + oy,
        width: region.width,
        height: region.height,
    };
    log::info!(
        "滚动截图: 图像坐标 ({},{}), 屏幕绝对 ({},{})",
        region.x, region.y, abs_region.x, abs_region.y
    );
    match mode {
        ScrollMode::Manual => run_manual_capture(config, &abs_region, active, app),
        ScrollMode::Auto => run_auto_capture(config, &abs_region, active, app),
    }
}

/// 手动模式：用户自己滚动，我们循环截帧 → 比对 → 拼接
fn run_manual_capture(
    config: &ScrollCaptureConfig,
    region: &CaptureRegion,
    active: &Arc<AtomicBool>,
    app: &tauri::AppHandle,
) -> Result<String, String> {
    let capture_loop = CaptureLoop::new(region);
    let mut stitcher = Stitcher::new(config, region.width);
    let mut hint_dy: Option<i32> = None;
    let mut no_change_since: Option<std::time::Instant> = None; // 内容无变化起始时刻（自动停止检测）
    // 强制限速：检测到过快滚动后进入限速模式，需连续 2 帧稳定匹配才恢复拼接
    let mut fast_streak: u32 = 0;

    log::info!("手动滚动捕获开始: {}x{} @ ({},{})", region.width, region.height, region.x, region.y);

    loop {
        if !active.load(Ordering::SeqCst) {
            log::info!("滚动截图被用户停止");
            break;
        }

        if stitcher.total_height >= config.max_length {
            log::info!("达到最大拼接长度 ({})", config.max_length);
            break;
        }

        // 已拼接过内容后，内容连续 3 秒无变化 → 用户停止滚动，自动完成。
        // 用时间而非帧数判断，避免高采集率下"停顿 1 秒即误判完成"。
        if stitcher.frame_count > 1 {
            if let Some(t) = no_change_since {
                if t.elapsed() >= std::time::Duration::from_secs(3) {
                    log::info!("内容长时间无变化，自动停止");
                    break;
                }
            }
        }

        let (frame_data, fw, fh) = capture_loop.capture_frame()?;

        if stitcher.is_empty() {
            stitcher.add_first_frame(&frame_data, fw, fh);
            emit_progress(app, &stitcher);
            std::thread::sleep(std::time::Duration::from_millis(10));
            continue;
        }

        // 快速检测：帧内容是否几乎相同（用户还没滚动）
        let similarity = simple_frame_similarity(stitcher.last_frame(), &frame_data);
        if similarity > 0.985 {
            if no_change_since.is_none() {
                no_change_since = Some(std::time::Instant::now());
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
            continue;
        }
        no_change_since = None;

        // 计算帧间偏移（实际滚动像素量）
        let raw = column_match::compute_offset(
            stitcher.last_frame(), &frame_data,
            fw, fh,
            hint_dy.unwrap_or(1),
            config.overlap_ratio, &config.direction,
        );

        // 匹配失败 → 用户滚动太快（或内容变化太快），无法可靠拼接。
        // 强制限速：警告持续显示，进入限速模式，暂停拼接直到用户放慢。
        if raw.confidence < 0.5 {
            emit_scroll_warning(app, "滚动太快，请放慢速度");
            fast_streak = 0;
            stitcher.resync(&frame_data, fw, fh);
            hint_dy = None;
            std::thread::sleep(std::time::Duration::from_millis(10));
            continue;
        }
        let offset = raw;
        let scroll_amt = if config.direction == "horizontal" { offset.dx } else { offset.dy };

        // 偏移过小 → 可能是重复帧，跳过
        if scroll_amt.abs() < 3 {
            if no_change_since.is_none() {
                no_change_since = Some(std::time::Instant::now());
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
            continue;
        }

        // 偏移异常大（超过帧高）→ 内容无重叠，无法拼接，重新同步基准帧
        if scroll_amt.abs() > fh as i32 {
            emit_scroll_warning(app, "滚动太快，请放慢速度");
            fast_streak = 0;
            stitcher.resync(&frame_data, fw, fh);
            hint_dy = None;
            std::thread::sleep(std::time::Duration::from_millis(10));
            continue;
        }

        // 限速模式：需连续 2 帧稳定匹配才恢复拼接（避免"快一下慢一下"的侥幸单帧）
        if fast_streak < 2 {
            fast_streak += 1;
            stitcher.resync(&frame_data, fw, fh);
            hint_dy = None;
            std::thread::sleep(std::time::Duration::from_millis(10));
            continue;
        }

        // 拼接
        stitcher.append_frame(&frame_data, fw, fh, &offset);
        emit_progress(app, &stitcher);
        hint_dy = Some(scroll_amt);

        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    stitcher.finalize()
}

/// 自动模式：SendInput 发送滚轮 → 截帧 → 拼接
fn run_auto_capture(
    config: &ScrollCaptureConfig,
    region: &CaptureRegion,
    active: &Arc<AtomicBool>,
    app: &tauri::AppHandle,
) -> Result<String, String> {
    let capture_loop = CaptureLoop::new(region);
    let mut scroll_input = ScrollInput::new();
    let mut stitcher = Stitcher::new(config, region.width);

    // 移动鼠标到选区中心，并记录光标下的目标窗口（SendInput 滚轮需要目标 hwnd）
    let center_x = region.x + region.width as i32 / 2;
    let center_y = region.y + region.height as i32 / 2;
    scroll_input.move_to(center_x, center_y)?;
    scroll_input.get_window_at_cursor()?;

    let mut hint_dy: Option<i32> = None;
    // 到底判定"无变化结束"计时：None = 尚未进入"内容无变化"状态
    let mut no_change_since: Option<std::time::Instant> = None;

    loop {
        if !active.load(Ordering::SeqCst) {
            break;
        }

        if stitcher.total_height >= config.max_length {
            log::info!("达到最大拼接长度 ({})", config.max_length);
            break;
        }

        let (frame_data, fw, fh) = capture_loop.capture_frame()?;

        if stitcher.is_empty() {
            stitcher.add_first_frame(&frame_data, fw, fh);
            emit_progress(app, &stitcher);
            scroll_input.scroll(config.scroll_step)?;
            std::thread::sleep(std::time::Duration::from_millis(config.scroll_delay_ms));
            continue;
        }

        // 滚动后帧内容几乎没变 → 已到达页面底部（或滚动无效）。
        // 不立即结束：持续 5 秒无变化才自动结束，避免用户误以为还在滚动。
        let similarity = simple_frame_similarity(stitcher.last_frame(), &frame_data);
        if similarity > 0.95 {
            let now = std::time::Instant::now();
            match no_change_since {
                None => no_change_since = Some(now),
                Some(t) => {
                    if now.duration_since(t) >= std::time::Duration::from_secs(5) {
                        log::info!("到达底部且约 5 秒无变化，自动结束");
                        break;
                    }
                }
            }
            scroll_input.scroll(config.scroll_step)?;
            std::thread::sleep(std::time::Duration::from_millis(config.scroll_delay_ms));
            continue;
        }
        no_change_since = None;

        let offset = match column_match::compute_offset(
            stitcher.last_frame(), &frame_data, fw, fh,
            hint_dy.unwrap_or(1),
            config.overlap_ratio, &config.direction,
        ) {
            r if r.confidence >= 0.5 => r,
            _ => match hint_dy {
                Some(h) => OffsetResult { dy: h, dx: 0, confidence: 0.4, algorithm: "hint_fallback" },
                None => check_end_of_scroll(&frame_data, stitcher.last_frame()),
            },
        };

        if offset.confidence < 0.3 {
            // 内容仍在变化但匹配失败（快速滚动/动图），跳过本帧继续，避免误判到底
            log::info!("拼接置信度过低 ({:.2})，跳过本帧", offset.confidence);
            no_change_since = None;
            scroll_input.scroll(config.scroll_step)?;
            std::thread::sleep(std::time::Duration::from_millis(config.scroll_delay_ms));
            continue;
        }

        // 偏移异常大（超过帧高）→ 内容无重叠，重新同步基准帧
        let scroll_amt = if config.direction == "horizontal" { offset.dx } else { offset.dy };
        if scroll_amt.abs() > fh as i32 {
            log::info!("滚动量超过帧高，重新同步基准帧");
            stitcher.resync(&frame_data, fw, fh);
            hint_dy = None;
            scroll_input.scroll(config.scroll_step)?;
            std::thread::sleep(std::time::Duration::from_millis(config.scroll_delay_ms));
            continue;
        }

        stitcher.append_frame(&frame_data, fw, fh, &offset);
        emit_progress(app, &stitcher);
        hint_dy = Some(scroll_amt);

        scroll_input.scroll(config.scroll_step)?;
        std::thread::sleep(std::time::Duration::from_millis(config.scroll_delay_ms));
    }

    stitcher.finalize()
}

fn emit_progress(app: &tauri::AppHandle, stitcher: &Stitcher) {
    let p = ScrollProgress {
        frame_count: stitcher.frame_count,
        total_height: stitcher.total_height,
        max_length: stitcher.max_length,
        preview_data_url: stitcher.preview_data_url(),
    };
    let _ = app.emit("scroll:progress", p);
}

/// 推送"滚动太快"警告给工具栏（用户需放慢滚动才能可靠拼接）
fn emit_scroll_warning(app: &tauri::AppHandle, message: &str) {
    let _ = app.emit("scroll:warning", serde_json::json!({"message": message}));
}

fn check_end_of_scroll(current: &[u8], previous: &[u8]) -> OffsetResult {
    let similarity = simple_frame_similarity(previous, current);
    if similarity > 0.95 {
        OffsetResult { dy: 0, dx: 0, confidence: 0.0, algorithm: "end_detect" }
    } else {
        OffsetResult { dy: 0, dx: 0, confidence: 0.4, algorithm: "low_confidence" }
    }
}

fn simple_frame_similarity(a: &[u8], b: &[u8]) -> f64 {
    if a.len() != b.len() { return 0.0; }
    let step = (a.len() / 2000).max(1);
    let mut matches = 0u64;
    let mut total = 0u64;
    for i in (0..a.len()).step_by(step) {
        if (a[i] as i32 - b[i] as i32).abs() < 5 { matches += 1; }
        total += 1;
    }
    if total == 0 { return 0.0; }
    matches as f64 / total as f64
}

/// 帧拼接器 — 逐帧累积像素为长图
struct Stitcher {
    canvas: Vec<u8>,
    total_height: u32,
    width: u32,
    last_frame_data: Vec<u8>,
    last_frame_width: u32,
    last_frame_height: u32,
    frame_count: u32,
    max_length: u32,
}

impl Stitcher {
    fn new(config: &ScrollCaptureConfig, width: u32) -> Self {
        Self {
            canvas: Vec::new(), total_height: 0, width,
            last_frame_data: Vec::new(), last_frame_width: 0, last_frame_height: 0,
            frame_count: 0, max_length: config.max_length,
        }
    }

    fn is_empty(&self) -> bool { self.frame_count == 0 }
    fn last_frame(&self) -> &[u8] { &self.last_frame_data }

    /// 重新同步基准帧（不追加内容）：滚动太快无法匹配时，用当前帧作为新的比对基准
    fn resync(&mut self, data: &[u8], width: u32, frame_h: u32) {
        self.last_frame_data = data.to_vec();
        self.last_frame_width = width;
        self.last_frame_height = frame_h;
    }

    fn add_first_frame(&mut self, data: &[u8], width: u32, height: u32) {
        self.canvas = data.to_vec();
        self.total_height = height;
        self.width = width;
        self.last_frame_data = data.to_vec();
        self.last_frame_width = width;
        self.last_frame_height = height;
        self.frame_count = 1;
    }

    fn append_frame(&mut self, data: &[u8], width: u32, frame_h: u32, offset: &OffsetResult) {
        // offset.dy 是本次实际滚动量 s（像素）—— 真正的新内容就是新帧底部的 s 行
        let scroll = offset.dy.max(0) as u32;
        let new_rows = scroll.min(frame_h).max(1);

        let new_h = self.total_height + new_rows;
        self.canvas.resize((new_h * self.width * 4) as usize, 0);

        // 从新帧底部取 new_rows 行（真正的新内容）追加到画布底部
        let src_row_start = frame_h - new_rows;
        for row in 0..new_rows {
            let src_start = ((src_row_start + row) * width * 4) as usize;
            let dst_start = ((self.total_height + row) * self.width * 4) as usize;
            let row_bytes = (self.width as usize * 4).min(data.len().saturating_sub(src_start));
            let copy_end = dst_start + row_bytes;
            if copy_end <= self.canvas.len() && src_start + row_bytes <= data.len() {
                self.canvas[dst_start..copy_end]
                    .copy_from_slice(&data[src_start..src_start + row_bytes]);
            }
        }

        self.total_height = new_h;
        self.last_frame_data = data.to_vec();
        self.last_frame_width = width;
        self.last_frame_height = frame_h;
        self.frame_count += 1;
    }

    /// 生成当前拼接结果的缩略图 data URL（供选区右侧实时预览）。
    /// 缩到最大 200px 宽，JPEG 编码，控制事件负载。
    fn preview_data_url(&self) -> Option<String> {
        if self.canvas.is_empty() || self.width == 0 || self.total_height == 0 {
            return None;
        }
        let img = image::RgbaImage::from_raw(self.width, self.total_height, self.canvas.clone())?;
        let dynamic = image::DynamicImage::ImageRgba8(img);
        let thumb = if self.width > 200 {
            let scale = 200.0 / self.width as f32;
            let th = ((self.total_height as f32 * scale).round() as u32).max(1);
            dynamic.resize(200, th, image::imageops::FilterType::Triangle)
        } else {
            dynamic
        };
        crate::capture::encode_jpeg_data_url(&thumb).ok()
    }

    fn finalize(&self) -> Result<String, String> {
        if self.canvas.is_empty() {
            return Err("没有捕获到任何帧".to_string());
        }
        let img = image::RgbaImage::from_raw(self.width, self.total_height, self.canvas.clone())
            .ok_or_else(|| "创建拼接图像失败".to_string())?;
        let dynamic = image::DynamicImage::ImageRgba8(img);

        let cache_dir = crate::capture::ensure_cache_dir();
        let output_path = cache_dir.join(format!("scroll_{}.png", uuid::Uuid::new_v4()));
        dynamic.save(&output_path)
            .map_err(|e| format!("保存拼接结果失败: {}", e))?;

        log::info!("滚动截图完成: {} ({}x{}, {} 帧)",
            output_path.display(), self.width, self.total_height, self.frame_count);
        Ok(output_path.to_string_lossy().to_string())
    }
}
