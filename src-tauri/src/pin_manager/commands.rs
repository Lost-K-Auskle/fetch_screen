use super::{window, PinId};
use tauri::AppHandle;

#[tauri::command]
pub async fn create_pin_window(
    app: AppHandle,
    image_path: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<String, String> {
    window::create_pin(&app, &image_path, x, y, width, height)
}

#[tauri::command]
pub async fn close_pin_window(
    app: AppHandle,
    pin_id: String,
) -> Result<(), String> {
    window::close_pin(&app, &pin_id)
}

#[tauri::command]
pub async fn update_pin_opacity(
    pin_id: String,
    alpha: f64,
) -> Result<(), String> {
    window::set_opacity(&pin_id, alpha)
}

#[tauri::command]
pub async fn toggle_pin_interaction(
    app: AppHandle,
    pin_id: String,
) -> Result<bool, String> {
    window::toggle_interaction(&app, &pin_id)
}
