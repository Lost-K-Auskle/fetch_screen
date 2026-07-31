use tauri::{
    AppHandle, Emitter, Manager, Runtime,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent},
};

/// 创建系统托盘
pub fn create_tray<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let quit_item = MenuItemBuilder::with_id("quit", "退出")
        .build(app)
        .map_err(|e| format!("创建托盘菜单项失败: {}", e))?;

    let screenshot_item = MenuItemBuilder::with_id("screenshot", "区域截图")
        .build(app)
        .map_err(|e| format!("创建托盘菜单项失败: {}", e))?;

    let menu = MenuBuilder::new(app)
        .item(&screenshot_item)
        .separator()
        .item(&quit_item)
        .build()
        .map_err(|e| format!("创建托盘菜单失败: {}", e))?;

    let app_clone = app.clone();
    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("Fetch Screen - 截图工具")
        .on_menu_event(move |app, event| {
            match event.id().as_ref() {
                "quit" => {
                    log::info!("用户请求退出");
                    app.exit(0);
                }
                "screenshot" => {
                    let _ = app.emit("tray:screenshot", ());
                }
                _ => {}
            }
        })
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(main_win) = app_clone.get_webview_window("main") {
                    let _ = main_win.show();
                    let _ = main_win.set_focus();
                }
            }
        })
        .build(app)
        .map_err(|e| format!("创建托盘图标失败: {}", e))?;

    Ok(())
}
