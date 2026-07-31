/// 滚动截图命令层
use super::{controller, ScrollCaptureConfig};
use tauri::State;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct ScrollCaptureState {
    pub active: Arc<AtomicBool>,
    pub config: Mutex<ScrollCaptureConfig>,
}

impl ScrollCaptureState {
    pub fn new() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(false)),
            config: Mutex::new(ScrollCaptureConfig::default()),
        }
    }
}

#[tauri::command]
pub async fn start_scroll_capture(
    state: State<'_, ScrollCaptureState>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<String, String> {
    if state.active.load(Ordering::SeqCst) {
        return Err("滚动截图已在运行中".to_string());
    }

    let config = state.config.lock().unwrap().clone();
    state.active.store(true, Ordering::SeqCst);

    let region = crate::capture::CaptureRegion { x, y, width, height };

    // Arc allows sharing AtomicBool across threads
    let active_flag = state.active.clone();
    let flag_for_thread = active_flag.clone();
    let result = std::thread::spawn(move || {
        controller::run_scroll_capture(&config, &region, &flag_for_thread)
    })
    .join()
    .map_err(|_| "滚动截图线程异常".to_string())?
    .map_err(|e| {
        active_flag.store(false, Ordering::SeqCst);
        e
    })?;

    state.active.store(false, Ordering::SeqCst);
    Ok(result)
}

#[tauri::command]
pub async fn stop_scroll_capture(
    state: State<'_, ScrollCaptureState>,
) -> Result<(), String> {
    state.active.store(false, Ordering::SeqCst);
    Ok(())
}
