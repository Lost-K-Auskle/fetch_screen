use super::AppConfig;
use tauri::AppHandle;
use std::path::PathBuf;

/// 配置文件路径: %APPDATA%/fetch-screen/config.json
pub fn config_path() -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("fetch-screen").join("config.json")
}

/// 加载配置
pub fn load_config(_app: &AppHandle) -> Option<AppConfig> {
    let path = config_path();
    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<AppConfig>(&content) {
                Ok(config) => {
                    log::info!("配置加载成功: {:?}", path);
                    Some(config)
                }
                Err(e) => {
                    log::warn!("配置解析失败: {}，使用默认配置", e);
                    Some(AppConfig::default())
                }
            },
            Err(e) => {
                log::warn!("配置文件读取失败: {}，使用默认配置", e);
                Some(AppConfig::default())
            }
        }
    } else {
        // 首次运行: 创建默认配置
        let config = AppConfig::default();
        let _ = save_config(&config);
        Some(config)
    }
}

/// 保存配置
pub fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建配置目录失败: {}", e))?;
    }

    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;

    std::fs::write(&path, content)
        .map_err(|e| format!("写入配置文件失败: {}", e))?;

    log::info!("配置已保存: {:?}", path);
    Ok(())
}
