pub mod commands;
pub mod controller;
pub mod capture_loop;
pub mod column_match;
pub mod stitching;
pub mod scroll_input;

use serde::{Deserialize, Serialize};

/// 滚动截图配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollCaptureConfig {
    /// 滚动方向: "vertical" | "horizontal"
    pub direction: String,
    /// 自动/手动模式
    pub auto_scroll: bool,
    /// 帧间滚动延迟 (ms)
    pub scroll_delay_ms: u64,
    /// 最大拼接总长度 (px)
    pub max_length: u32,
    /// 滚动步长 (滚轮刻度数)
    pub scroll_step: i32,
    /// 重叠检测候选区占比 (0.0-1.0)
    pub overlap_ratio: f32,
}

impl Default for ScrollCaptureConfig {
    fn default() -> Self {
        Self {
            direction: "vertical".to_string(),
            auto_scroll: true,
            scroll_delay_ms: 300,
            max_length: 30000,
            scroll_step: -120, // 负值 = 向下滚动
            overlap_ratio: 0.4,
        }
    }
}

/// 拼接结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StitchResult {
    /// 拼接后的图像路径
    pub image_path: String,
    /// 总高度/宽度 (取决于滚动方向)
    pub total_length: u32,
    /// 捕获的帧数
    pub frame_count: u32,
    /// 拼接置信度
    pub confidence: f64,
}

/// 拼接偏移结果
#[derive(Debug, Clone)]
pub struct OffsetResult {
    pub dy: i32,
    pub dx: i32,
    pub confidence: f64,
    pub algorithm: &'static str, // "mad" | "fft" | "orb"
}
