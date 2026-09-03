pub mod capture;
pub mod pin_manager;
pub mod scrollshot;
pub mod system;

use tauri::Manager;

/// 应用入口 - 注册所有 Tauri 命令和插件
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .manage(capture::commands::CaptureState::new())
        .manage(pin_manager::PinManagerState::new())
        .manage(scrollshot::commands::ScrollCaptureState::new())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            log::info!("Fetch Screen 启动成功");

            // 初始化系统托盘
            system::tray::create_tray(app.handle())?;

            // 注册全局快捷键
            system::hotkey::register_global_hotkeys(app.handle())?;

            // 同步缓存目录（config.json 已存在时 load_config 不会触发 save_config，
            // 需显式读取配置并设置，保证 save_to_cache 使用用户自定义目录）
            if let Some(cfg) = system::config::load_config(app.handle()) {
                crate::capture::set_cache_dir(std::path::PathBuf::from(&cfg.cache_dir));
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            capture::commands::capture_fullscreen,
            capture::commands::capture_region,
            capture::commands::capture_window,
            capture::commands::capture_monitors,
            capture::commands::open_region_overlay,
            capture::commands::get_overlay_payload,
            capture::commands::open_cache_dir,
            capture::commands::set_pin_passthrough,
            capture::commands::show_preview,
            capture::commands::get_preview_payload,
            capture::commands::delete_image,
            capture::commands::debug_log_event,
            capture::commands::open_scroll_toolbar,
            capture::commands::get_scroll_region,
            capture::commands::open_scroll_region_frame,
            capture::commands::get_scroll_frame,
            pin_manager::commands::create_pin_window,
            pin_manager::commands::close_pin_window,
            pin_manager::commands::update_pin_opacity,
            pin_manager::commands::toggle_pin_interaction,
            scrollshot::commands::start_scroll_capture,
            scrollshot::commands::stop_scroll_capture,
            system::commands::get_config,
            system::commands::save_config,
            system::commands::update_hotkeys,
            system::commands::set_hotkeys_enabled,
            system::commands::copy_to_clipboard,
            system::commands::save_to_file,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Fetch Screen 失败");
}
