pub mod commands;
pub mod screen;
pub mod window;

use image::DynamicImage;
use serde::{Deserialize, Serialize};

/// 截图选区 (物理像素坐标)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// 显示器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub id: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub is_primary: bool,
}

/// 窗口信息 (用于窗口截图模式)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub class_name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_minimized: bool,
}

/// 缓存截图的路径，避免 IPC 传输像素数据
pub fn get_cache_dir() -> std::path::PathBuf {
    std::env::temp_dir().join("fetch_screen")
}

pub fn ensure_cache_dir() -> std::path::PathBuf {
    let dir = get_cache_dir();
    std::fs::create_dir_all(&dir).ok();
    dir
}

/// 将 DynamicImage 保存到缓存目录，返回文件路径
pub fn save_to_cache(img: &DynamicImage, prefix: &str) -> Result<std::path::PathBuf, String> {
    let dir = ensure_cache_dir();
    let path = dir.join(format!("{}_{}.png", prefix, uuid::Uuid::new_v4()));
    img.save(&path).map_err(|e| format!("保存截图缓存失败: {}", e))?;
    Ok(path)
}
