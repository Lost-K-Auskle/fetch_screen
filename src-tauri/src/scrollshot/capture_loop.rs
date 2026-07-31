use crate::capture::CaptureRegion;
use image::DynamicImage;

/// 帧捕获循环
/// 使用 BitBlt 从屏幕捕获指定区域的帧
pub struct CaptureLoop {
    region: CaptureRegion,
    monitor_index: usize,
}

impl CaptureLoop {
    pub fn new(region: &CaptureRegion) -> Self {
        Self {
            region: region.clone(),
            monitor_index: 0,
        }
    }

    /// 捕获当前帧，返回 RGBA 像素数据
    pub fn capture_frame(&self) -> Result<Vec<u8>, String> {
        use xcap::Monitor;

        let monitors = Monitor::all().map_err(|e| format!("枚举显示器失败: {}", e))?;
        let monitor = monitors
            .get(self.monitor_index)
            .ok_or_else(|| format!("显示器 {} 不存在", self.monitor_index))?;

        let full_img = monitor
            .capture_image()
            .map_err(|e| format!("捕获帧失败: {}", e))?;

        // 裁剪选区 (选区坐标是虚拟桌面坐标)
        let ox = (self.region.x - monitor.x()).max(0) as u32;
        let oy = (self.region.y - monitor.y()).max(0) as u32;
        let w = self.region.width;
        let h = self.region.height;

        let cropped = image::imageops::crop_imm(&full_img, ox, oy, w, h);
        let rgba = cropped.to_image().into_raw();

        Ok(rgba)
    }

    /// 捕获帧为 DynamicImage
    pub fn capture_frame_image(&self) -> Result<DynamicImage, String> {
        use xcap::Monitor;

        let monitors = Monitor::all().map_err(|e| format!("枚举显示器失败: {}", e))?;
        let monitor = monitors
            .get(self.monitor_index)
            .ok_or_else(|| format!("显示器 {} 不存在", self.monitor_index))?;

        let full_img = monitor
            .capture_image()
            .map_err(|e| format!("捕获帧失败: {}", e))?;

        let ox = (self.region.x - monitor.x()).max(0) as u32;
        let oy = (self.region.y - monitor.y()).max(0) as u32;
        let w = self.region.width;
        let h = self.region.height;

        let cropped = image::imageops::crop_imm(&full_img, ox, oy, w, h);
        Ok(DynamicImage::ImageRgba8(cropped.to_image()))
    }
}
