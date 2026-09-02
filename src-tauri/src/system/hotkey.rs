use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// Register all global hotkeys (non-fatal if any fail)
pub fn register_global_hotkeys(app: &AppHandle) -> Result<(), String> {
    let config = super::config::load_config(app).unwrap_or_default();

    // Register screenshot region hotkey
    match config.hotkeys.screenshot.parse::<Shortcut>() {
        Ok(shortcut) => {
            let event = "hotkey:screenshot_region".to_string();
            let name = config.hotkeys.screenshot.clone();
            match app.global_shortcut().on_shortcut(shortcut, move |app, _sc, evt| {
                // global-hotkey 在 Windows 上每次触发会连发 Pressed + Released 两个事件，
                // 只在按下时执行动作，避免重复触发。
                if evt.state != ShortcutState::Pressed {
                    return;
                }
                log::info!("区域截图热键触发");
                let _ = app.emit(event.as_str(), ());
            }) {
                Ok(_) => log::info!("已注册区域截图热键: {}", name),
                Err(e) => log::warn!("注册区域截图热键失败: {}", e),
            }
        }
        Err(e) => log::warn!("解析区域截图热键失败: {}", e),
    }

    // Register full screenshot hotkey
    match config.hotkeys.screenshot_full.parse::<Shortcut>() {
        Ok(shortcut) => {
            let event = "hotkey:screenshot_full".to_string();
            let name = config.hotkeys.screenshot_full.clone();
            match app.global_shortcut().on_shortcut(shortcut, move |app, _sc, evt| {
                if evt.state != ShortcutState::Pressed {
                    return;
                }
                log::info!("全屏截图热键触发");
                let _ = app.emit(event.as_str(), ());
            }) {
                Ok(_) => log::info!("已注册全屏截图热键: {}", name),
                Err(e) => log::warn!("注册全屏截图热键失败: {}", e),
            }
        }
        Err(e) => log::warn!("解析全屏截图热键失败: {}", e),
    }

    // Register scroll capture hotkey
    match config.hotkeys.scrollshot.parse::<Shortcut>() {
        Ok(shortcut) => {
            let event = "hotkey:scroll_capture".to_string();
            let name = config.hotkeys.scrollshot.clone();
            match app.global_shortcut().on_shortcut(shortcut, move |app, _sc, evt| {
                if evt.state != ShortcutState::Pressed {
                    return;
                }
                log::info!("滚动截图热键触发");
                let _ = app.emit(event.as_str(), ());
            }) {
                Ok(_) => log::info!("已注册滚动截图热键: {}", name),
                Err(e) => log::warn!("注册滚动截图热键失败: {}", e),
            }
        }
        Err(e) => log::warn!("解析滚动截图热键失败: {}", e),
    }

    // Register pin-last hotkey (toggle 贴图鼠标穿透)
    match config.hotkeys.pin_last.parse::<Shortcut>() {
        Ok(shortcut) => {
            let name = config.hotkeys.pin_last.clone();
            match app.global_shortcut().on_shortcut(shortcut, move |app, _sc, evt| {
                if evt.state != ShortcutState::Pressed {
                    return;
                }
                log::info!("贴图穿透热键触发");
                // 切换当前贴图窗口的鼠标穿透
                let state = app.state::<crate::capture::commands::CaptureState>();
                let label = state.current_pin_label.lock().unwrap().clone();
                let current = *state.pin_passthrough.lock().unwrap();
                if let Some(label) = label {
                    if let Some(win) = app.get_webview_window(&label) {
                        let new = !current;
                        let _ = win.set_ignore_cursor_events(new);
                        *state.pin_passthrough.lock().unwrap() = new;
                        let _ = win.emit("pin:passthrough", new);
                        log::info!("贴图穿透切换为: {}", new);
                    }
                }
            }) {
                Ok(_) => log::info!("已注册贴图穿透热键: {}", name),
                Err(e) => log::warn!("注册贴图穿透热键失败: {}", e),
            }
        }
        Err(e) => log::warn!("解析贴图穿透热键失败: {}", e),
    }

    Ok(())
}
