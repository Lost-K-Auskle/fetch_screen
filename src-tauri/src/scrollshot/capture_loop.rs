use crate::capture::CaptureRegion;
use windows::Win32::Graphics::Gdi::*;

/// 帧捕获循环 — 用 GDI BitBlt 从屏幕快速捕获选区帧 (~50ms/帧 vs xcap ~150ms)
pub struct CaptureLoop {
    region: CaptureRegion,
}

impl CaptureLoop {
    pub fn new(region: &CaptureRegion) -> Self {
        Self { region: region.clone() }
    }

    /// 捕获当前帧，返回 (RGBA 像素数据, 宽度, 高度)
    pub fn capture_frame(&self) -> Result<(Vec<u8>, u32, u32), String> {
        let w = self.region.width as i32;
        let h = self.region.height as i32;
        let x = self.region.x;
        let y = self.region.y;

        if w <= 0 || h <= 0 {
            return Err("选区尺寸无效".to_string());
        }

        unsafe {
            let hdc_screen = GetDC(None);
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            let hbmp = CreateCompatibleBitmap(hdc_screen, w, h);
            let old = SelectObject(hdc_mem, hbmp);

            // 不用 CAPTUREBLT：它会连带截取分层窗口（滚动选区边框浮层/工具栏），
            // 导致截图带上蓝色边框和半透明遮罩（"被框住+变暗不清晰"）。
            // 去掉它可排除这些浮层，让滚动截图和区域截图一样干净清晰。
            let blt_result = BitBlt(hdc_mem, 0, 0, w, h, hdc_screen, x, y, SRCCOPY);

            if blt_result.is_err() {
                SelectObject(hdc_mem, old);
                let _ = DeleteObject(hbmp);
                let _ = DeleteDC(hdc_mem);
                ReleaseDC(None, hdc_screen);
                return Err(format!("BitBlt 截帧失败: {:?}", blt_result.unwrap_err()));
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

            let data_size = (w * h * 4) as usize;
            let mut buf = vec![0u8; data_size];
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
            let _ = DeleteObject(hbmp);
            let _ = DeleteDC(hdc_mem);
            ReleaseDC(None, hdc_screen);

            if lines != h {
                return Err(format!("读取像素失败 (GetDIBits 返回 {})", lines));
            }

            // BGRA → RGBA，Alpha 置 255
            for px in buf.chunks_exact_mut(4) {
                let r = px[2];
                px[2] = px[0];
                px[0] = r;
                px[3] = 255;
            }

            Ok((buf, self.region.width, self.region.height))
        }
    }
}
