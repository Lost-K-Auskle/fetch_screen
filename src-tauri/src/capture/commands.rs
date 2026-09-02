use super::{screen, window, CaptureRegion, MonitorInfo};
use super::{save_to_cache, save_to_cache_jpeg};
use serde::{Deserialize, Serialize};
use tauri::State;
use std::sync::Mutex;

/// 全屏截图状态
pub struct CaptureState {
    /// 最近一次全屏截图的缓存路径
    pub last_fullscreen_path: Mutex<Option<String>>,
    /// 待选区覆盖层读取的图像数据（窗口创建后由 overlay 通过 get_overlay_payload 取走）
    pub pending_overlay: Mutex<Option<OverlayPayload>>,
    /// 选区覆盖层的干净底图（RGBA 原始像素 + 宽高），供 capture_region 无损裁剪。
    /// 底图在 overlay 窗口创建前截取，不含 overlay 的遮罩/选区边框。
    pub pending_overlay_base: Mutex<Option<(Vec<u8>, u32, u32)>>,
    /// 待预览窗口读取的截图（窗口创建后由 preview 通过 get_preview_payload 取走）
    pub pending_preview: Mutex<Option<PreviewPayload>>,
    /// 当前贴图窗口 label（最近一次创建的预览窗口）
    pub current_pin_label: Mutex<Option<String>>,
    /// 当前贴图是否鼠标穿透
    pub pin_passthrough: Mutex<bool>,
    /// 穿透时的小把手窗口 label（双击切回交互）
    pub pin_handle_label: Mutex<Option<String>>,
    /// 待滚动截图的选区（滚动截图工具栏读取后启动捕获）
    pub pending_scroll_region: Mutex<Option<CaptureRegion>>,
    /// 待滚动截图区域边框浮层读取的数据
    pub pending_scroll_frame: Mutex<Option<ScrollFramePayload>>,
}

impl CaptureState {
    pub fn new() -> Self {
        Self {
            last_fullscreen_path: Mutex::new(None),
            pending_overlay: Mutex::new(None),
            pending_overlay_base: Mutex::new(None),
            pending_preview: Mutex::new(None),
            current_pin_label: Mutex::new(None),
            pin_passthrough: Mutex::new(false),
            pin_handle_label: Mutex::new(None),
            pending_scroll_region: Mutex::new(None),
            pending_scroll_frame: Mutex::new(None),
        }
    }
}

/// 预览窗口的数据（路径 + 显示用 JPEG data URL）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewPayload {
    pub path: String,
    pub data_url: String,
}

/// 全屏截图覆盖层的数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayPayload {
    /// 虚拟桌面截图路径
    pub image_path: String,
    /// 虚拟桌面物理原点 (图像 (0,0) 对应的屏幕坐标)
    pub origin_x: i32,
    pub origin_y: i32,
    /// 图像尺寸 (物理像素)
    pub width: u32,
    pub height: u32,
    /// 显示用 JPEG data URL（规避 asset 协议在超大窗口下的加载问题）
    pub image_data_url: String,
}

/// 滚动截图区域边框浮层的数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollFramePayload {
    /// 选区（图像坐标，0,0 = 虚拟桌面原点）
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    /// 虚拟桌面物理尺寸（用于全屏 canvas 尺寸）
    pub desktop_width: u32,
    pub desktop_height: u32,
}

/// 根据配置决定是否在截图前隐藏主窗口；返回是否执行了隐藏。
/// 隐藏后需等待 DWM 重新合成（约 1 帧），否则 GDI 截到的仍是含主窗口的旧画面。
fn hide_main_window_for_capture(app: &tauri::AppHandle) -> bool {
    use tauri::Manager;
    let hide_ui = crate::system::config::load_config(app)
        .map(|c| c.hide_ui_on_capture)
        .unwrap_or(true);
    if !hide_ui {
        return false;
    }
    if let Some(main) = app.get_webview_window("main") {
        if main.is_visible().unwrap_or(true) {
            // WDA_EXCLUDEFROMCAPTURE：即使 hide() 的 DWM 重合成有延迟，
            // 也能保证任何时刻的 GDI/WGC 截屏都不含主窗口（彻底消除"定格残留"）
            if let Ok(hwnd) = main.hwnd() {
                set_window_excluded_from_capture(hwnd.0 as isize, true);
            }
            let _ = main.hide();
            wait_for_dwm_recompose();
            // 轮询确认主窗口真正从屏幕消失（DWM 重合成需要时间），
            // 否则 GDI 截屏会截到"定格"的主窗口画面
            wait_for_main_window_gone(app);
            return true;
        }
    }
    false
}

/// 设置窗口是否从屏幕捕获中排除（WDA_EXCLUDEFROMCAPTURE）。
/// 该属性让 GDI BitBlt / PrintWindow / WGC 等所有截屏 API 都看不到此窗口，
/// 是消除"隐藏后定格残留"的可靠手段（Win10 2004+ 支持）。
fn set_window_excluded_from_capture(hwnd: isize, exclude: bool) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowDisplayAffinity, WINDOW_DISPLAY_AFFINITY,
    };
    const WDA_NONE: u32 = 0x0000_0000;
    const WDA_EXCLUDEFROMCAPTURE: u32 = 0x0000_0011;
    unsafe {
        let _ = SetWindowDisplayAffinity(
            HWND(hwnd as *mut _),
            WINDOW_DISPLAY_AFFINITY(if exclude { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE }),
        );
    }
}

/// 等待 DWM 完成重新合成：hide() 后若 GDI 立刻 BitBlt，会截到仍含主窗口的旧画面。
/// 固定 sleep 在混合 DPI 多显示器下不可靠，DwmFlush 阻塞直到桌面窗口管理器空闲。
fn wait_for_dwm_recompose() {
    use windows::Win32::Graphics::Dwm::DwmFlush;
    unsafe {
        let _ = DwmFlush();
    }
    // DwmFlush 保证命令已提交；再补一帧时长，确保 GDI 重定向表面已更新
    std::thread::sleep(std::time::Duration::from_millis(16));
}

/// 主窗口背景色 #0f1116 (RGB 15,17,22)，用于确认 hide() 后窗口确实从屏幕消失
const APP_BG_R: i32 = 15;
const APP_BG_G: i32 = 17;
const APP_BG_B: i32 = 22;

/// 采样屏幕区域，判断是否仍包含应用主窗口的背景色（说明窗口还没从屏幕消失）
fn region_has_app_bg(x: i32, y: i32, w: i32, h: i32) -> bool {
    use windows::Win32::Foundation::*;
    use windows::Win32::Graphics::Gdi::*;

    let sw = w.min(48).max(8);
    let sh = h.min(48).max(8);
    let sx = x + (w - sw) / 2;
    let sy = y + (h - sh) / 2;

    unsafe {
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbmp = CreateCompatibleBitmap(hdc_screen, sw, sh);
        let old = SelectObject(hdc_mem, hbmp);
        let _ = BitBlt(hdc_mem, 0, 0, sw, sh, hdc_screen, sx, sy, SRCCOPY);

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: sw,
                biHeight: -sh,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0 as u32,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut buf = vec![0u8; (sw * sh * 4) as usize];
        let lines = GetDIBits(
            hdc_mem,
            hbmp,
            0,
            sh as u32,
            Some(buf.as_mut_ptr() as _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        SelectObject(hdc_mem, old);
        let _ = DeleteObject(hbmp);
        let _ = DeleteDC(hdc_mem);
        ReleaseDC(None, hdc_screen);

        if lines != sh {
            return true; // 读取失败时保守认为窗口还在
        }

        let mut dark = 0u32;
        let mut total = 0u32;
        for px in buf.chunks_exact(4) {
            let b = px[0] as i32;
            let g = px[1] as i32;
            let r = px[2] as i32;
            if (r - APP_BG_R).abs() < 24 && (g - APP_BG_G).abs() < 24 && (b - APP_BG_B).abs() < 24 {
                dark += 1;
            }
            total += 1;
        }
        dark as f32 / total as f32 > 0.5
    }
}

/// 等待主窗口真正从屏幕消失：hide() 后 DWM 重合成需要时间，
/// 轮询主窗口区域直到背景色消失（最多 ~320ms）。
fn wait_for_main_window_gone(app: &tauri::AppHandle) {
    use tauri::Manager;
    if let Some(main) = app.get_webview_window("main") {
        if let (Ok(pos), Ok(size)) = (main.outer_position(), main.outer_size()) {
            let x = pos.x as i32;
            let y = pos.y as i32;
            let w = size.width as i32;
            let h = size.height as i32;
            if w <= 0 || h <= 0 {
                return;
            }
            for _ in 0..20 {
                if !region_has_app_bg(x, y, w, h) {
                    return; // 主窗口已从屏幕消失
                }
                std::thread::sleep(std::time::Duration::from_millis(16));
            }
        }
    }
}

/// 恢复主窗口显示
fn show_main_window(app: &tauri::AppHandle) {
    use tauri::Manager;
    if let Some(main) = app.get_webview_window("main") {
        // 解除截屏排除，恢复正常
        if let Ok(hwnd) = main.hwnd() {
            set_window_excluded_from_capture(hwnd.0 as isize, false);
        }
        let _ = main.show();
    }
}

#[tauri::command]
pub async fn capture_fullscreen(
    app: tauri::AppHandle,
    state: State<'_, CaptureState>,
) -> Result<String, String> {
    log::info!("capture_fullscreen 被调用");

    // 若开启"截图时隐藏 UI"，先隐藏主窗口，避免主窗口被截进全屏图
    let hid = hide_main_window_for_capture(&app);

    let img = match screen::capture_virtual_desktop_gdi() {
        Ok((img, _info)) => img,
        Err(e) => {
            if hid { show_main_window(&app); }
            log::error!("capture_fullscreen 截屏失败: {}", e);
            return Err(format!("截屏失败: {}", e));
        }
    };

    // 恢复主窗口
    if hid { show_main_window(&app); }

    let path = match save_to_cache_jpeg(&img, "fullscreen") {
        Ok(p) => p,
        Err(e) => {
            log::error!("capture_fullscreen 保存缓存失败: {}", e);
            return Err(e);
        }
    };
    let path_str = path.to_string_lossy().to_string();

    // 自动复制到剪贴板（后台线程，不阻塞预览显示）
    let copy_path = path.clone();
    std::thread::spawn(move || {
        if let Err(e) = crate::system::clipboard::copy_image_to_clipboard(&copy_path) {
            log::warn!("自动复制到剪贴板失败: {}", e);
        }
    });

    let mut last = state.last_fullscreen_path.lock().unwrap();
    *last = Some(path_str.clone());
    log::info!("capture_fullscreen 完成: {}", path_str);
    Ok(path_str)
}

/// 打开全屏选区覆盖层：先截屏 → 存 payload → 建透明 overlay 窗口（无黑底闪烁）
/// 关键：先截屏，再隐藏主窗口，最后才显示 overlay。overlay 透明无背景，图片加载完前用户看不到变化。
#[tauri::command]
pub async fn open_region_overlay(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

    log::info!("open_region_overlay 被调用");

    // Step 1: 依据配置先隐藏主窗口，确保区域截图的背景截图不含主窗口。
    // hide 后需等 DWM 重合成（hide_main_window_for_capture 内已处理）。
    // 恢复由前端 region:complete / region:cancelled 负责（getCurrentWindow().show()）。
    let hid = hide_main_window_for_capture(&app);

    // Step 2: 截屏（主窗口已隐藏，背景干净）
    let (img, info) = match screen::capture_virtual_desktop_gdi() {
        Ok(v) => v,
        Err(e) => {
            if hid { show_main_window(&app); }
            log::error!("open_region_overlay 截屏失败: {}", e);
            return Err(format!("截屏失败: {}", e));
        }
    };
    let data_url = match super::encode_jpeg_data_url(&img) {
        Ok(d) => d,
        Err(e) => {
            if hid { show_main_window(&app); }
            log::error!("open_region_overlay 编码失败: {}", e);
            return Err(e);
        }
    };
    log::info!("open_region_overlay 截屏完成: {}x{}", info.width, info.height);

    // Step 2: 存 payload + 干净底图（供 capture_region 无损裁剪，不含 overlay 遮罩/边框）
    {
        let state = app.state::<CaptureState>();
        let mut pending = state.pending_overlay.lock().unwrap();
        *pending = Some(OverlayPayload {
            image_path: String::new(),
            origin_x: info.x,
            origin_y: info.y,
            width: info.width,
            height: info.height,
            image_data_url: data_url,
        });
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        *state.pending_overlay_base.lock().unwrap() = Some((rgba.into_raw(), w, h));
    }

    // Step 3: 创建透明 overlay 窗口（先不显示）
    let label = format!("overlay_{}", uuid::Uuid::new_v4());
    let result = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("overlay.html".into()))
        .title("")
        .inner_size(info.width as f64, info.height as f64)
        .position(info.x as f64, info.y as f64)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .transparent(true)   // 透明窗口 — 图片加载完前桌面透出，无黑底闪烁
        .visible(false)      // 等前端 img.onload 后再 show()
        .build();
    if let Err(e) = result {
        if hid { show_main_window(&app); }
        return Err(format!("创建选区窗口失败: {}", e));
    }

    Ok(())
}

/// overlay 窗口挂载后调用，读取待显示的截图数据（克隆，幂等）
#[tauri::command]
pub async fn get_overlay_payload(
    state: State<'_, CaptureState>,
) -> Result<Option<OverlayPayload>, String> {
    let pending = state.pending_overlay.lock().unwrap();
    Ok(pending.clone())
}

/// 截图后显示浮窗预览（等比例，供用户选择置顶/关闭/删除）
#[tauri::command]
pub async fn show_preview(app: tauri::AppHandle, image_path: String) -> Result<(), String> {
    use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

    log::info!("show_preview: {}", image_path);

    let img = image::open(&image_path).map_err(|e| format!("加载图片失败: {}", e))?;
    let (iw, ih) = (img.width(), img.height());
    let monitors = screen::list_monitors()?;
    let primary = monitors.iter().find(|m| m.is_primary).unwrap_or(&monitors[0]);
    let max_w = (primary.width as f64 * 0.45).max(200.0);
    let max_h = (primary.height as f64 * 0.40).max(200.0);
    let scale = (max_w / iw as f64).min(max_h / ih as f64).min(1.0);
    let w = ((iw as f64 * scale).round() as u32).max(120);
    let h = ((ih as f64 * scale).round() as u32).max(120);

    let data_url = super::encode_jpeg_data_url(&img)?;
    {
        let state = app.state::<CaptureState>();
        *state.pending_preview.lock().unwrap() = Some(PreviewPayload {
            path: image_path.clone(),
            data_url,
        });
    }

    let label = format!("preview_{}", uuid::Uuid::new_v4());
    let margin = 24.0;
    let x = primary.x as f64 + primary.width as f64 - w as f64 - margin;
    let y = primary.y as f64 + primary.height as f64 - h as f64 - margin;
    let win = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("preview.html".into()))
        .title("")
        .inner_size(w as f64, h as f64)
        .position(x, y)
        .decorations(false)
        .always_on_top(false)
        .skip_taskbar(true)
        .resizable(true)
        .min_inner_size(120.0, 120.0)
        .transparent(true)
        .visible(true)
        .build()
        .map_err(|e| format!("创建预览窗口失败: {}", e))?;
    let _ = win.show();
    let _ = win.set_focus();

    let state = app.state::<CaptureState>();
    *state.current_pin_label.lock().unwrap() = Some(label);
    *state.pin_passthrough.lock().unwrap() = false;
    Ok(())
}

/// 预览窗口挂载后调用，读取待预览的截图（幂等，不 take）
#[tauri::command]
pub async fn get_preview_payload(
    state: State<'_, CaptureState>,
) -> Result<Option<PreviewPayload>, String> {
    let p = state.pending_preview.lock().unwrap();
    Ok(p.clone())
}

/// 切换当前贴图窗口的鼠标穿透
#[tauri::command]
pub async fn set_pin_passthrough(app: tauri::AppHandle, enabled: bool) -> Result<bool, String> {
    use tauri::{Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

    let state = app.state::<CaptureState>();
    let label = state.current_pin_label.lock().unwrap().clone();
    let Some(label) = label else { return Err("没有活动的贴图窗口".to_string()); };
    let Some(win) = app.get_webview_window(&label) else { return Err("贴图窗口不存在".to_string()); };

    if let Some(h) = state.pin_handle_label.lock().unwrap().take() {
        if let Some(hw) = app.get_webview_window(&h) {
            let _ = hw.close();
        }
    }

    win.set_ignore_cursor_events(enabled)
        .map_err(|e| format!("切换穿透失败: {}", e))?;
    *state.pin_passthrough.lock().unwrap() = enabled;
    log::info!("贴图穿透: {}", enabled);

    if enabled {
        if let (Ok(pos), Ok(size)) = (win.outer_position(), win.outer_size()) {
            let hw = 18.0;
            let hh = 40.0;
            let hx = pos.x as f64 + size.width as f64 / 2.0 - hw / 2.0;
            let hy = pos.y as f64 - hh - 2.0;
            let hl = format!("pinhandle_{}", uuid::Uuid::new_v4());
            if let Ok(handle) = WebviewWindowBuilder::new(&app, &hl, WebviewUrl::App("pinhandle.html".into()))
                .title("")
                .inner_size(hw, hh)
                .position(hx, hy)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .transparent(true)
                .resizable(false)
                .visible(true)
                .build()
            {
                let _ = handle.set_position(tauri::Position::Physical(PhysicalPosition::new(hx as i32, hy as i32)));
                *state.pin_handle_label.lock().unwrap() = Some(hl);
                log::info!("已创建穿透把手");
            }
        }
    }

    Ok(enabled)
}

/// 删除截图文件
#[tauri::command]
pub async fn delete_image(path: String) -> Result<(), String> {
    log::info!("delete_image: {}", path);
    std::fs::remove_file(&path).map_err(|e| format!("删除失败: {}", e))
}

/// 在资源管理器中打开截图缓存目录
#[tauri::command]
pub async fn open_cache_dir() -> Result<(), String> {
    let dir = super::ensure_cache_dir();
    let dir_str = dir.to_string_lossy().to_string();
    std::process::Command::new("explorer")
        .arg(&dir_str)
        .spawn()
        .map_err(|e| format!("打开缓存目录失败: {}", e))?;
    log::info!("打开缓存目录: {}", dir_str);
    Ok(())
}

/// 调试日志
#[tauri::command]
pub fn debug_log_event(line: String) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(r"D:\rust\downloads\overlay_debug.log")
    {
        let _ = writeln!(f, "{}", line);
    }
}

#[tauri::command]
pub async fn capture_region(
    app: tauri::AppHandle,
    state: State<'_, CaptureState>,
    region: CaptureRegion,
) -> Result<String, String> {
    log::info!("capture_region 被调用: x={} y={} {}x{}", region.x, region.y, region.width, region.height);

    // 优先从 overlay 打开时的干净底图裁剪（不含 overlay 的遮罩/选区边框）。
    // 底图是虚拟桌面图像坐标，region 也是图像坐标，直接裁剪即可。
    let cropped = {
        let base = state.pending_overlay_base.lock().unwrap().clone();
        match base {
            Some((raw, w, h)) => {
                let img = image::RgbaImage::from_raw(w, h, raw)
                    .ok_or_else(|| "创建底图失败".to_string())?;
                screen::crop_region(&image::DynamicImage::ImageRgba8(img), &region)
            }
            None => {
                // 兜底：现场重截（可能含 overlay 边框，但至少可用）
                let img = screen::capture_virtual_desktop_gdi()?.0;
                screen::crop_region(&img, &region)
            }
        }
    };
    let output_path = save_to_cache(&cropped, "region")?;

    // 自动复制到剪贴板（后台线程）
    let copy_path = output_path.clone();
    std::thread::spawn(move || {
        if let Err(e) = crate::system::clipboard::copy_image_to_clipboard(&copy_path) {
            log::warn!("自动复制到剪贴板失败: {}", e);
        }
    });

    // 裁剪完成后清理底图（下次 open_region_overlay 会重新存）
    *state.pending_overlay_base.lock().unwrap() = None;

    Ok(output_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn capture_window(hwnd: Option<i64>) -> Result<String, String> {
    match hwnd {
        Some(h) => {
            let img = window::capture_window_by_hwnd(h as isize)?;
            let path = save_to_cache(&img, "window")?;
            Ok(path.to_string_lossy().to_string())
        }
        None => {
            let img = screen::capture_virtual_desktop_gdi()?.0;
            let path = save_to_cache(&img, "window")?;
            Ok(path.to_string_lossy().to_string())
        }
    }
}

#[tauri::command]
pub async fn capture_monitors() -> Result<Vec<MonitorInfo>, String> {
    screen::list_monitors()
}

/// 打开滚动截图浮动工具栏（底部居中大面板，Lark 式）
#[tauri::command]
pub async fn open_scroll_toolbar(
    app: tauri::AppHandle,
    x: i32, y: i32, width: u32, height: u32,
) -> Result<(), String> {
    use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

    let state = app.state::<CaptureState>();
    *state.pending_scroll_region.lock().unwrap() = Some(CaptureRegion { x, y, width, height });

    let label = format!("scroll_toolbar_{}", uuid::Uuid::new_v4());
    let ww = 400.0;
    let wh = 60.0;

    // 主显示器底部居中
    let monitors = screen::list_monitors()?;
    let primary = monitors.iter().find(|m| m.is_primary).unwrap_or(&monitors[0]);
    let cx = (primary.x as f64 + primary.width as f64 / 2.0 - ww / 2.0).round();
    let cy = (primary.y as f64 + primary.height as f64 - wh - 60.0).round();

    WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("scroll_toolbar.html".into()))
        .title("滚动捕获")
        .inner_size(ww, wh)
        .position(cx, cy)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .transparent(true)
        // 不抢焦点：手动滚动模式下，用户需要先把焦点给到目标滚动窗口
        .focused(false)
        .visible(true)
        .build()
        .map_err(|e| format!("创建工具栏窗口失败: {}", e))?;

    Ok(())
}

/// 工具栏窗口挂载后读取待捕获的选区（幂等，不 take）
#[tauri::command]
pub async fn get_scroll_region(
    state: State<'_, CaptureState>,
) -> Result<Option<CaptureRegion>, String> {
    Ok(state.pending_scroll_region.lock().unwrap().clone())
}

/// 打开滚动截图区域边框浮层：全屏透明、点击穿透，只画选区边框，方便用户边滚边看选区
#[tauri::command]
pub async fn open_scroll_region_frame(
    app: tauri::AppHandle,
    x: i32, y: i32, width: u32, height: u32,
) -> Result<(), String> {
    use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

    // 虚拟桌面物理包围盒
    let monitors = screen::list_monitors()?;
    let (min_x, min_y, max_x, max_y) = monitors.iter().fold(
        (i32::MAX, i32::MAX, i32::MIN, i32::MIN),
        |(mnx, mny, mxx, mxy), m| {
            (
                mnx.min(m.x),
                mny.min(m.y),
                mxx.max(m.x + m.width as i32),
                mxy.max(m.y + m.height as i32),
            )
        },
    );
    let vw = (max_x - min_x) as u32;
    let vh = (max_y - min_y) as u32;

    let state = app.state::<CaptureState>();
    *state.pending_scroll_frame.lock().unwrap() = Some(ScrollFramePayload {
        x, y, width, height,
        desktop_width: vw,
        desktop_height: vh,
    });

    let label = format!("scroll_frame_{}", uuid::Uuid::new_v4());
    let win = WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("scroll_frame.html".into()))
        .title("")
        .inner_size(vw as f64, vh as f64)
        .position(min_x as f64, min_y as f64)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .transparent(true)
        .visible(true)
        .build()
        .map_err(|e| format!("创建选区边框窗口失败: {}", e))?;

    // 强制物理坐标精确覆盖虚拟桌面（混合 DPI 下避免左缘"缝"）
    let _ = win.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(vw, vh)));
    let _ = win.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(min_x, min_y)));

    // 点击穿透：让用户能直接滚动下层窗口
    let _ = win.set_ignore_cursor_events(true);

    Ok(())
}

/// 边框浮层窗口挂载后读取待绘制的选区（幂等，不 take）
#[tauri::command]
pub async fn get_scroll_frame(
    state: State<'_, CaptureState>,
) -> Result<Option<ScrollFramePayload>, String> {
    Ok(state.pending_scroll_frame.lock().unwrap().clone())
}
