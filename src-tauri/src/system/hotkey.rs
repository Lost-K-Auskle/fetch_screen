use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

/// Register all global hotkeys (non-fatal if any fail)
pub fn register_global_hotkeys(app: &AppHandle) -> Result<(), String> {
    let config = super::config::load_config(app).unwrap_or_default();

    // Register screenshot region hotkey
    match config.hotkeys.screenshot.parse::<Shortcut>() {
        Ok(shortcut) => {
            let event = "hotkey:screenshot_region".to_string();
            let name = config.hotkeys.screenshot.clone();
            match app.global_shortcut().on_shortcut(shortcut, move |app, _sc, _event| {
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
            match app.global_shortcut().on_shortcut(shortcut, move |app, _sc, _event| {
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
            match app.global_shortcut().on_shortcut(shortcut, move |app, _sc, _event| {
                log::info!("滚动截图热键触发");
                let _ = app.emit(event.as_str(), ());
            }) {
                Ok(_) => log::info!("已注册滚动截图热键: {}", name),
                Err(e) => log::warn!("注册滚动截图热键失败: {}", e),
            }
        }
        Err(e) => log::warn!("解析滚动截图热键失败: {}", e),
    }

    Ok(())
}
