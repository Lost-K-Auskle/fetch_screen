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
    /// 截图时是否隐藏 Fetch Screen 主 UI 窗口（避免主窗口被截进截图）
    #[serde(default = "default_hide_ui_on_capture")]
    pub hide_ui_on_capture: bool,
    /// 预览窗鼠标拖拽方式: "left_drag" = 左键拖窗 / Shift+左键平移; "shift_drag" = 左键平移 / Shift+左键拖窗
    #[serde(default = "default_preview_drag_mode")]
    pub preview_drag_mode: String,
    /// 预览窗无截图区域背景: "black" = 黑色 / "white" = 白色 / "hollow" = 镂空(透明)
    #[serde(default = "default_preview_bg_mode")]
    pub preview_bg_mode: String,
    /// 截图缓存目录（截图文件存放位置，可在设置中自定义；默认 %TEMP%/fetch_screen）
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,
    /// 截完图浮窗默认位置: "top_left" / "top_right" / "bottom_left" / "bottom_right" / "cursor"(截图位置自选=光标处)
    #[serde(default = "default_preview_position")]
    pub preview_position: String,
    /// 浮窗内平移组合热键的修饰键: "Shift" / "Ctrl" / "Alt"
    #[serde(default = "default_pan_modifier")]
    pub pan_modifier: String,
}

fn default_preview_drag_mode() -> String {
    "left_drag".to_string()
}

fn default_preview_bg_mode() -> String {
    "black".to_string()
}

fn default_cache_dir() -> String {
    std::env::temp_dir().join("fetch_screen").to_string_lossy().to_string()
}

fn default_preview_position() -> String {
    "bottom_right".to_string()
}

fn default_pan_modifier() -> String {
    "Shift".to_string()
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
                screenshot: "Alt+Shift+A".to_string(),
                screenshot_full: "Ctrl+Alt+A".to_string(),
                scrollshot: "Ctrl+Shift+A".to_string(),
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
            hide_ui_on_capture: true,
            preview_drag_mode: "left_drag".to_string(),
            preview_bg_mode: "black".to_string(),
            cache_dir: default_cache_dir(),
            preview_position: "bottom_right".to_string(),
            pan_modifier: "Shift".to_string(),
        }
    }
}

fn default_hide_ui_on_capture() -> bool {
    true
}

fn dirs_next() -> Option<String> {
    dirs::picture_dir()
        .or_else(|| dirs::desktop_dir())
        .map(|p| p.to_string_lossy().to_string())
}
