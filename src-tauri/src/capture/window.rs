use super::WindowInfo;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Foundation::*;
use image::DynamicImage;

/// 枚举所有顶层可见窗口
pub fn list_windows() -> Result<Vec<WindowInfo>, String> {
    let mut windows = Vec::new();

    unsafe {
        let mut handles: Vec<isize> = Vec::new();
        let lparam = LPARAM(&mut handles as *mut Vec<isize> as isize);

        let _ = EnumWindows(
            Some(enum_window_callback),
            lparam,
        );

        for hwnd_raw in handles {
            let hwnd = HWND(hwnd_raw as *mut _);
            if !IsWindowVisible(hwnd).as_bool() {
                continue;
            }
            if IsIconic(hwnd).as_bool() {
                continue;
            }

            // 获取窗口标题
            let mut title_buf = [0u16; 256];
            let title_len = GetWindowTextW(hwnd, &mut title_buf);
            let title = String::from_utf16_lossy(&title_buf[..title_len as usize]);

            // 空标题或系统窗口跳过
            if title.is_empty() || title == "Program Manager" {
                continue;
            }

            // 获取窗口类和矩形
            let mut class_buf = [0u16; 256];
            GetClassNameW(hwnd, &mut class_buf);
            let class_name = String::from_utf16_lossy(&class_buf[..]);

            let mut rect = RECT::default();
            if GetWindowRect(hwnd, &mut rect).is_ok() {
                let w = (rect.right - rect.left).max(0) as u32;
                let h = (rect.bottom - rect.top).max(0) as u32;

                if w > 0 && h > 0 {
                    windows.push(WindowInfo {
                        id: format!("{:?}", hwnd.0),
                        title,
                        class_name,
                        x: rect.left,
                        y: rect.top,
                        width: w,
                        height: h,
                        is_minimized: false,
                    });
                }
            }
        }
    }

    Ok(windows)
}

unsafe extern "system" fn enum_window_callback(
    hwnd: HWND,
    lparam: LPARAM,
) -> BOOL {
    let handles: &mut Vec<isize> = &mut *(lparam.0 as *mut Vec<isize>);
    handles.push(hwnd.0 as isize);
    BOOL(1)
}

/// 截取特定窗口
pub fn capture_window_by_hwnd(hwnd: isize) -> Result<DynamicImage, String> {
    use xcap::Monitor;

    let h = HWND(hwnd as *mut _);

    // 获取窗口位置
    let mut rect = RECT::default();
    unsafe {
        GetWindowRect(h, &mut rect).map_err(|e| format!("获取窗口位置失败: {:?}", e))?;
    }

    let x = rect.left;
    let y = rect.top;
    let w = (rect.right - rect.left).max(0) as u32;
    let h = (rect.bottom - rect.top).max(0) as u32;

    // 找到包含该窗口的显示器
    let monitors = Monitor::all().map_err(|e| format!("枚举显示器失败: {}", e))?;
    let monitor = monitors
        .iter()
        .find(|m| {
            x >= m.x()
                && y >= m.y()
                && (x + w as i32) <= (m.x() + m.width() as i32)
                && (y + h as i32) <= (m.y() + m.height() as i32)
        })
        .unwrap_or(&monitors[0]);

    // 截取并裁剪
    let full = monitor
        .capture_image()
        .map_err(|e| format!("截屏失败: {}", e))?;

    let ox = (x - monitor.x()).max(0) as u32;
    let oy = (y - monitor.y()).max(0) as u32;

    let sub = image::imageops::crop_imm(&full, ox, oy, w, h);
    Ok(DynamicImage::ImageRgba8(sub.to_image()))
}

/// 将窗口置于前台 (用于滚动截图前的焦点设置)
pub fn bring_to_front(hwnd: isize) -> Result<(), String> {
    let h = HWND(hwnd as *mut _);
    unsafe {
        if !IsWindowVisible(h).as_bool() || IsIconic(h).as_bool() {
            ShowWindow(h, SW_RESTORE);
        }
        SetForegroundWindow(h);
        // 等待前台切换完成
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Ok(())
}
