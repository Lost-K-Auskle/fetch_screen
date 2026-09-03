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

use std::sync::Mutex;

static CACHE_DIR: Mutex<Option<std::path::PathBuf>> = Mutex::new(None);

/// 缓存截图的路径，避免 IPC 传输像素数据。
/// 截图文件可自定义存放目录（AppConfig.cache_dir），此处返回运行时已同步的值；
/// 默认 %TEMP%/fetch_screen。
pub fn get_cache_dir() -> std::path::PathBuf {
    let g = CACHE_DIR.lock().unwrap();
    g.clone().unwrap_or_else(|| std::env::temp_dir().join("fetch_screen"))
}

/// 设置当前截图缓存目录（配置加载/保存时由 system 模块同步调用）。
/// 空路径忽略（保持默认），避免 save_to_cache 落到无效目录。
pub fn set_cache_dir(path: std::path::PathBuf) {
    if !path.as_os_str().is_empty() {
        *CACHE_DIR.lock().unwrap() = Some(path);
    }
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

/// 快速保存缓存 (JPEG，编码快且文件小，适合 overlay 显示图；裁剪用独立 GDI 无损路径)
pub fn save_to_cache_jpeg(img: &DynamicImage, prefix: &str) -> Result<std::path::PathBuf, String> {
    let dir = ensure_cache_dir();
    let path = dir.join(format!("{}_{}.jpg", prefix, uuid::Uuid::new_v4()));
    let mut out = std::io::BufWriter::new(
        std::fs::File::create(&path).map_err(|e| format!("创建缓存文件失败: {}", e))?,
    );
    img.write_to(&mut out, image::ImageFormat::Jpeg)
        .map_err(|e| format!("JPEG 编码失败: {}", e))?;
    Ok(path)
}

/// 将图像编码为 JPEG 的 data URL（base64），供 overlay 直接显示，规避 asset 协议问题
pub fn encode_jpeg_data_url(img: &DynamicImage) -> Result<String, String> {
    use std::io::Write;
    let mut buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Jpeg)
        .map_err(|e| format!("JPEG 编码失败: {}", e))?;
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &buf);
    Ok(format!("data:image/jpeg;base64,{}", b64))
}
