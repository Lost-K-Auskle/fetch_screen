use super::{CaptureRegion, MonitorInfo};
use image::DynamicImage;
use xcap::Monitor;

/// 获取所有显示器列表
pub fn list_monitors() -> Result<Vec<MonitorInfo>, String> {
    let monitors = Monitor::all().map_err(|e| format!("枚举显示器失败: {}", e))?;
    monitors
        .into_iter()
        .enumerate()
        .map(|(i, m)| {
            Ok(MonitorInfo {
                id: format!("monitor_{}", i),
                name: m.name().to_string(),
                x: m.x(),
                y: m.y(),
                width: m.width(),
                height: m.height(),
                scale_factor: m.scale_factor() as f64,
                is_primary: m.is_primary(),
            })
        })
        .collect()
}

/// 全屏截图 — 单显示器
pub fn capture_monitor(index: usize) -> Result<(DynamicImage, MonitorInfo), String> {
    let monitors = Monitor::all().map_err(|e| format!("枚举显示器失败: {}", e))?;
    let monitor = monitors
        .get(index)
        .ok_or_else(|| format!("显示器 {} 不存在", index))?;

    let img = monitor
        .capture_image()
        .map_err(|e| format!("截屏失败: {}", e))?;

    let info = MonitorInfo {
        id: format!("monitor_{}", index),
        name: monitor.name().to_string(),
        x: monitor.x(),
        y: monitor.y(),
        width: monitor.width(),
        height: monitor.height(),
        scale_factor: monitor.scale_factor() as f64,
        is_primary: monitor.is_primary(),
    };

    Ok((DynamicImage::ImageRgba8(img), info))
}

/// 全屏截图 — 所有显示器
pub fn capture_all_monitors() -> Result<DynamicImage, String> {
    let monitors = Monitor::all().map_err(|e| format!("枚举显示器失败: {}", e))?;

    // 计算虚拟桌面总尺寸
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    let captures: Vec<_> = monitors
        .iter()
        .map(|m| {
            let img = m
                .capture_image()
                .map_err(|e| format!("截屏失败: {}", e))?;
            let x = m.x();
            let y = m.y();
            let w = img.width();
            let h = img.height();

            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x + w as i32);
            max_y = max_y.max(y + h as i32);

            Ok((x, y, DynamicImage::ImageRgba8(img)))
        })
        .collect::<Result<Vec<_>, String>>()?;

    // 创建虚拟画布
    let canvas_w = (max_x - min_x) as u32;
    let canvas_h = (max_y - min_y) as u32;
    let mut canvas = DynamicImage::new_rgba8(canvas_w, canvas_h);

    // 将各显示器截图合成
    use image::GenericImageView;
    for (x, y, img) in &captures {
        let ox = (x - min_x) as u32;
        let oy = (y - min_y) as u32;
        image::imageops::overlay(&mut canvas, img, ox as i64, oy as i64);
    }

    Ok(canvas)
}

/// 区域截图 — 从已捕获的全屏图中裁剪
pub fn crop_region(img: &DynamicImage, region: &CaptureRegion) -> DynamicImage {
    img.crop_imm(
        region.x as u32,
        region.y as u32,
        region.width,
        region.height,
    )
}
