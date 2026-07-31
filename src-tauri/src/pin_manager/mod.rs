pub mod commands;
pub mod window;
pub mod hitbox;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// 贴图唯一标识
pub type PinId = String;

/// 贴图窗口状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinState {
    pub id: PinId,
    pub image_path: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub opacity: f64,
    pub scale: f64,
    pub rotation: f64,
    pub click_through: bool,
    pub is_minimized: bool,
}

/// 全局贴图管理器状态
pub struct PinManagerState {
    pub pins: Mutex<Vec<PinState>>,
    pub max_pins: usize,
}

impl PinManagerState {
    pub fn new() -> Self {
        Self {
            pins: Mutex::new(Vec::new()),
            max_pins: 8,
        }
    }

    pub fn active_count(&self) -> usize {
        self.pins.lock().unwrap().len()
    }
}
