use super::{screen, window, CaptureRegion, MonitorInfo, WindowInfo};
use super::save_to_cache;
use tauri::State;
use std::sync::Mutex;

/// 全屏截图状态
pub struct CaptureState {
    /// 最近一次全屏截图的缓存路径
    pub last_fullscreen_path: Mutex<Option<String>>,
}

impl CaptureState {
    pub fn new() -> Self {
        Self {
            last_fullscreen_path: Mutex::new(None),
        }
    }
}

#[tauri::command]
pub async fn capture_fullscreen(
    state: State<'_, CaptureState>,
) -> Result<String, String> {
    let img = screen::capture_all_monitors()?;
    let path = save_to_cache(&img, "fullscreen")?;
    let path_str = path.to_string_lossy().to_string();

    let mut last = state.last_fullscreen_path.lock().unwrap();
    *last = Some(path_str.clone());

    Ok(path_str)
}

#[tauri::command]
pub async fn capture_region(
    state: State<'_, CaptureState>,
    region: CaptureRegion,
) -> Result<String, String> {
    let fullscreen_path = {
        let last = state.last_fullscreen_path.lock().unwrap();
        last.clone()
    };

    match fullscreen_path {
        Some(path) => {
            let img = image::open(&path)
                .map_err(|e| format!("加载缓存截图失败: {}", e))?;
            let cropped = screen::crop_region(&img, &region);
            let output_path = save_to_cache(&cropped, "region")?;
            Ok(output_path.to_string_lossy().to_string())
        }
        None => {
            // 如果没有缓存，重新截取全屏再裁剪
            let img = screen::capture_all_monitors()?;
            let cropped = screen::crop_region(&img, &region);
            let output_path = save_to_cache(&cropped, "region")?;
            Ok(output_path.to_string_lossy().to_string())
        }
    }
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
            // 无窗口句柄时等同于全屏截图
            let img = screen::capture_all_monitors()?;
            let path = save_to_cache(&img, "window")?;
            Ok(path.to_string_lossy().to_string())
        }
    }
}

#[tauri::command]
pub async fn capture_monitors() -> Result<Vec<MonitorInfo>, String> {
    screen::list_monitors()
}
