pub mod commands;
pub mod hotkey;
pub mod tray;
pub mod clipboard;
pub mod config;

use serde::{Deserialize, Serialize};

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub hotkeys: HotkeyConfig,
    pub save: SaveConfig,
    pub pin: PinConfig,
    pub scrollshot: ScrollshotConfig,
    pub annotation: AnnotationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    pub screenshot: String,
    pub screenshot_full: String,
    pub scrollshot: String,
    pub pin_last: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveConfig {
    pub path: String,
    pub format: String,
    pub quality: u8,
    pub naming: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinConfig {
    pub default_opacity: f64,
    pub default_click_through: bool,
    pub max_pin_count: usize,
    pub restore_on_startup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollshotConfig {
    pub mode: String,
    pub max_length: u32,
    pub scroll_delay_ms: u64,
    pub jpeg_quality: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationConfig {
    pub default_color: String,
    pub default_line_width: u32,
    pub font_size: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkeys: HotkeyConfig {
                screenshot: "Alt+S".to_string(),
                screenshot_full: "Ctrl+Shift+S".to_string(),
                scrollshot: "Ctrl+Alt+S".to_string(),
                pin_last: "Ctrl+Shift+P".to_string(),
            },
            save: SaveConfig {
                path: dirs_next().unwrap_or_default(),
                format: "png".to_string(),
                quality: 90,
                naming: "{date}_{time}_{seq}".to_string(),
            },
            pin: PinConfig {
                default_opacity: 1.0,
                default_click_through: true,
                max_pin_count: 8,
                restore_on_startup: false,
            },
            scrollshot: ScrollshotConfig {
                mode: "auto".to_string(),
                max_length: 30000,
                scroll_delay_ms: 300,
                jpeg_quality: 85,
            },
            annotation: AnnotationConfig {
                default_color: "#FF0000".to_string(),
                default_line_width: 3,
                font_size: 16,
            },
        }
    }
}

fn dirs_next() -> Option<String> {
    dirs::picture_dir()
        .or_else(|| dirs::desktop_dir())
        .map(|p| p.to_string_lossy().to_string())
}
