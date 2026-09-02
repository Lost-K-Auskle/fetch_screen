/// 滚动截图命令层
use super::controller::{self, ScrollMode};
use super::ScrollCaptureConfig;
use tauri::State;
use tauri::Emitter;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub struct ScrollCaptureState {
    pub active: Arc<AtomicBool>,
    /// 每次启动捕获时递增；旧线程结束时据此判断自己是否已过时，
    /// 避免"取消旧捕获 + 启动新捕获"时旧线程残留事件/状态干扰。
    pub gen: Arc<AtomicU64>,
    pub config: Mutex<ScrollCaptureConfig>,
}

impl ScrollCaptureState {
    pub fn new() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(false)),
            gen: Arc::new(AtomicU64::new(0)),
            config: Mutex::new(ScrollCaptureConfig::default()),
        }
    }
}

/// 启动滚动长截图（非阻塞）。
/// 若已有捕获在跑，会先取消旧的再启动新的（自愈"滚动截图已在运行中"）。
#[tauri::command]
pub async fn start_scroll_capture(
    app: tauri::AppHandle,
    state: State<'_, ScrollCaptureState>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    mode: Option<String>,
) -> Result<(), String> {
    // 先递增代号，让旧线程（如果还在）立即"过期"
    let my_gen = state.gen.fetch_add(1, Ordering::SeqCst) + 1;

    if state.active.load(Ordering::SeqCst) {
        // 已有捕获在跑：取消它，并稍等旧线程退出循环
        state.active.store(false, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_millis(300));
    }

    let config = state.config.lock().unwrap().clone();
    state.active.store(true, Ordering::SeqCst);

    let region = crate::capture::CaptureRegion { x, y, width, height };
    let active_flag = state.active.clone();
    let gen_flag = state.gen.clone();

    let scroll_mode = match mode.as_deref() {
        Some("auto") => ScrollMode::Auto,
        _ => ScrollMode::Manual,
    };

    std::thread::spawn(move || {
        let result = controller::run_scroll_capture(&config, &region, &active_flag, &app, scroll_mode);
        // 只有自己这代仍在运行，才推送结果并清理 active（旧线程不干扰新捕获）
        if gen_flag.load(Ordering::SeqCst) == my_gen {
            match result {
                Ok(path) => {
                    log::info!("滚动截图完成: {}", path);
                    // 自动复制到剪贴板（后台线程）
                    let copy_path = path.clone();
                    std::thread::spawn(move || {
                        if let Err(e) = crate::system::clipboard::copy_image_to_clipboard(std::path::Path::new(&copy_path)) {
                            log::warn!("自动复制到剪贴板失败: {}", e);
                        }
                    });
                    let _ = app.emit("scroll:complete", serde_json::json!({"path": path}));
                }
                Err(e) => {
                    log::error!("滚动截图失败: {}", e);
                    let _ = app.emit("scroll:error", serde_json::json!({"message": e}));
                }
            }
            active_flag.store(false, Ordering::SeqCst);
        }
    });

    Ok(())
}

/// 请求停止正在进行的滚动截图（线程会以当前已拼接的内容收尾）
#[tauri::command]
pub async fn stop_scroll_capture(
    state: State<'_, ScrollCaptureState>,
) -> Result<(), String> {
    state.active.store(false, Ordering::SeqCst);
    log::info!("滚动截图已请求停止");
    Ok(())
}
