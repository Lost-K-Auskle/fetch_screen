use super::AppConfig;
use super::config;
use super::clipboard;
use tauri::AppHandle;

#[tauri::command]
pub async fn get_config() -> Result<AppConfig, String> {
    let path = config::config_path();
    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("读取配置失败: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("解析配置失败: {}", e))
    } else {
        Ok(AppConfig::default())
    }
}

#[tauri::command]
pub async fn save_config(config: AppConfig) -> Result<(), String> {
    config::save_config(&config)
}

#[tauri::command]
pub async fn copy_to_clipboard(image_path: String) -> Result<(), String> {
    let path = std::path::Path::new(&image_path);
    clipboard::copy_image_to_clipboard(path)
}

#[tauri::command]
pub async fn save_to_file(
    image_path: String,
    dest_path: Option<String>,
    format: Option<String>,
) -> Result<String, String> {
    let src = std::path::Path::new(&image_path);
    let img = image::open(src)
        .map_err(|e| format!("加载图片失败: {}", e))?;

    let dest = match dest_path {
        Some(p) => std::path::PathBuf::from(p),
        None => {
            let config = AppConfig::default();
            let now = chrono::Local::now();
            let filename = format!(
                "screenshot_{}.{}",
                now.format("%Y%m%d_%H%M%S"),
                format.as_deref().unwrap_or(&config.save.format)
            );
            std::path::PathBuf::from(&config.save.path).join(filename)
        }
    };

    let out_format = format
        .unwrap_or_else(|| "png".to_string());

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败: {}", e))?;
    }

    match out_format.as_str() {
        "jpg" | "jpeg" => {
            img.save(dest.with_extension("jpg"))
                .map_err(|e| format!("保存失败: {}", e))?;
        }
        "webp" => {
            img.save(dest.with_extension("webp"))
                .map_err(|e| format!("保存失败: {}", e))?;
        }
        _ => {
            img.save(&dest)
                .map_err(|e| format!("保存失败: {}", e))?;
        }
    }

    Ok(dest.to_string_lossy().to_string())
}
