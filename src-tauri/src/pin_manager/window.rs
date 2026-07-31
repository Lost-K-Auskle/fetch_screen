use super::{PinId, PinState, PinManagerState};
use tauri::{AppHandle, Manager, WebviewWindowBuilder, WebviewUrl, Emitter};

/// 创建贴图窗口
pub fn create_pin(
    app: &AppHandle,
    image_path: &str,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<PinId, String> {
    let pin_manager = app.state::<PinManagerState>();

    {
        let pins = pin_manager.pins.lock().unwrap();
        if pins.len() >= pin_manager.max_pins {
            return Err(format!("贴图已达上限 ({})", pin_manager.max_pins));
        }
    }

    let pin_id = uuid::Uuid::new_v4().to_string();
    let label = format!("pin_{}", pin_id);

    let cache_path = {
        let src = std::path::Path::new(image_path);
        let cache_dir = super::super::capture::ensure_cache_dir();
        let dest = cache_dir.join(format!("pin_{}.png", pin_id));
        if src != dest {
            std::fs::copy(src, &dest).map_err(|e| format!("缓存贴图失败: {}", e))?;
        }
        dest
    };

    let _pin_win = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("pin.html".into()))
        .title("")
        .inner_size(width as f64, height as f64)
        .position(x as f64, y as f64)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .visible(true)
        .build()
        .map_err(|e| format!("创建贴图窗口失败: {}", e))?;

    // Default: mouse passthrough
    if let Some(win) = tauri::Manager::get_webview_window(app, &label) {
        win.set_ignore_cursor_events(true).ok();
    }

    // Send image path via app emit
    let cache_path_str = cache_path.to_string_lossy().to_string();
    let _ = app.emit("pin:load", cache_path_str.clone());

    let pin_state = PinState {
        id: pin_id.clone(),
        image_path: cache_path_str,
        x, y, width, height,
        opacity: 1.0,
        scale: 1.0,
        rotation: 0.0,
        click_through: true,
        is_minimized: false,
    };

    pin_manager.pins.lock().unwrap().push(pin_state);
    Ok(pin_id)
}

/// 关闭贴图窗口
pub fn close_pin(app: &AppHandle, pin_id: &str) -> Result<(), String> {
    let label = format!("pin_{}", pin_id);
    if let Some(win) = tauri::Manager::get_webview_window(app, &label) {
        win.close().map_err(|e| format!("关闭窗口失败: {}", e))?;
    }

    let pin_manager = app.state::<PinManagerState>();
    let mut pins = pin_manager.pins.lock().unwrap();
    pins.retain(|p| p.id != pin_id);
    Ok(())
}

/// 设置贴图透明度 (Phase 2: native WS_EX_LAYERED)
pub fn set_opacity(pin_id: &str, alpha: f64) -> Result<(), String> {
    let label = format!("pin_{}", pin_id);
    let _ = (label, alpha);
    Ok(())
}

/// 切换贴图交互模式
pub fn toggle_interaction(app: &AppHandle, pin_id: &str) -> Result<bool, String> {
    let label = format!("pin_{}", pin_id);
    let win = tauri::Manager::get_webview_window(app, &label)
        .ok_or_else(|| format!("贴图窗口 {} 不存在", pin_id))?;

    let pin_manager = app.state::<PinManagerState>();
    let mut pins = pin_manager.pins.lock().unwrap();
    let pin = pins.iter_mut()
        .find(|p| p.id == pin_id)
        .ok_or_else(|| format!("贴图 {} 不在管理器中", pin_id))?;

    let new_mode = !pin.click_through;
    win.set_ignore_cursor_events(new_mode)
        .map_err(|e| format!("切换穿透失败: {}", e))?;

    pin.click_through = new_mode;
    Ok(new_mode)
}
