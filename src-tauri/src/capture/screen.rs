use super::{CaptureRegion, MonitorInfo};
use image::DynamicImage;
use xcap::Monitor;
use std::sync::OnceLock;

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

/// 区域截图 — 从已捕获的全屏图中裁剪（越界安全，自动 clamp）
pub fn crop_region(img: &DynamicImage, region: &CaptureRegion) -> DynamicImage {
    use image::GenericImageView;
    let (w, h) = img.dimensions();
    let x = region.x.max(0).min(w as i32) as u32;
    let y = region.y.max(0).min(h as i32) as u32;
    let cw = region.width.min(w.saturating_sub(x)).max(1);
    let ch = region.height.min(h.saturating_sub(y)).max(1);
    img.crop_imm(x, y, cw, ch)
}

/// 虚拟桌面信息 (物理像素坐标)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VirtualDesktopInfo {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// 虚拟桌面物理包围盒 (min_x, min_y, max_x, max_y)，首次计算后缓存。
/// 显示器布局极少变化，缓存可避免每次截图都走 xcap 的 Monitor::all() 枚举开销。
/// （GetSystemMetrics 在混合 DPI 下返回逻辑像素不可靠，故仍用 xcap 枚举一次。）
static VIRTUAL_DESKTOP_BOUNDS: OnceLock<(i32, i32, i32, i32)> = OnceLock::new();

fn virtual_desktop_bounds() -> (i32, i32, i32, i32) {
    *VIRTUAL_DESKTOP_BOUNDS.get_or_init(|| {
        let monitors = match Monitor::all() {
            Ok(m) => m,
            Err(_) => return (0, 0, 0, 0),
        };
        monitors.iter().fold(
            (i32::MAX, i32::MAX, i32::MIN, i32::MIN),
            |(mnx, mny, mxx, mxy), m| {
                (
                    mnx.min(m.x()),
                    mny.min(m.y()),
                    mxx.max(m.x() + m.width() as i32),
                    mxy.max(m.y() + m.height() as i32),
                )
            },
        )
    })
}

/// 虚拟桌面物理原点 (min_x, min_y)。
/// 截图的图像坐标 (0,0) 即此原点；BitBlt/SetCursorPos 等需要屏幕绝对坐标时加上它。
pub fn virtual_desktop_origin() -> (i32, i32) {
    let (mnx, mny, _, _) = virtual_desktop_bounds();
    if mnx == i32::MAX { (0, 0) } else { (mnx, mny) }
}

/// 用 GDI BitBlt 捕获整个虚拟桌面 (物理像素, 约 50-100ms，远快于 xcap 的 WGC)
/// 返回 (图像, 虚拟桌面物理坐标信息)。图像 (0,0) 对应物理桌面坐标 (info.x, info.y)。
pub fn capture_virtual_desktop_gdi() -> Result<(DynamicImage, VirtualDesktopInfo), String> {
    use windows::Win32::Foundation::*;
    use windows::Win32::Graphics::Gdi::*;

    // 物理包围盒（缓存，避免热路径反复枚举显示器）
    let (min_x, min_y, max_x, max_y) = virtual_desktop_bounds();
    let w = max_x - min_x;
    let h = max_y - min_y;
    if w <= 0 || h <= 0 {
        return Err("虚拟桌面尺寸无效".to_string());
    }

    unsafe {
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hbmp = CreateCompatibleBitmap(hdc_screen, w, h);
        let old = SelectObject(hdc_mem, hbmp);

        if let Err(e) = BitBlt(hdc_mem, 0, 0, w, h, hdc_screen, min_x, min_y, SRCCOPY | CAPTUREBLT) {
            SelectObject(hdc_mem, old);
            DeleteObject(hbmp);
            DeleteDC(hdc_mem);
            ReleaseDC(None, hdc_screen);
            return Err(format!("BitBlt 截屏失败: {:?}", e));
        }

        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h, // 自顶向下
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0 as u32,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut buf = vec![0u8; (w * h * 4) as usize];
        let lines = GetDIBits(
            hdc_mem,
            hbmp,
            0,
            h as u32,
            Some(buf.as_mut_ptr() as _),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        SelectObject(hdc_mem, old);
        DeleteObject(hbmp);
        DeleteDC(hdc_mem);
        ReleaseDC(None, hdc_screen);

        if lines != h {
            return Err(format!("读取像素失败 (GetDIBits 返回 {})", lines));
        }

        // BGRA -> RGBA, 并把 A 置 255 (GDI 32bpp 的 alpha 位是 0)
        for px in buf.chunks_exact_mut(4) {
            let r = px[2];
            px[2] = px[0];
            px[0] = r;
            px[3] = 255;
        }
        let rgba_img = image::RgbaImage::from_raw(w as u32, h as u32, buf)
            .ok_or_else(|| "创建图像失败".to_string())?;
        let img = DynamicImage::ImageRgba8(rgba_img);

        Ok((
            img,
            VirtualDesktopInfo {
                x: min_x,
                y: min_y,
                width: w as u32,
                height: h as u32,
            },
        ))
    }
}
