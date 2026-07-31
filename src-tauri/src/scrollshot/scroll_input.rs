use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Foundation::*;

/// 滚动输入控制器
pub struct ScrollInput {
    current_hwnd: Option<isize>,
}

impl ScrollInput {
    pub fn new() -> Self {
        Self { current_hwnd: None }
    }

    pub fn move_to(&self, x: i32, y: i32) -> Result<(), String> {
        unsafe {
            SetCursorPos(x, y).map_err(|e| format!("移动光标失败: {:?}", e))?;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        Ok(())
    }

    pub fn get_window_at_cursor(&mut self) -> Result<isize, String> {
        let mut point = POINT::default();
        unsafe {
            GetCursorPos(&mut point).map_err(|e| format!("获取光标位置失败: {:?}", e))?;
            let hwnd = WindowFromPoint(point);
            if hwnd.0.is_null() {
                return Err("光标处无窗口".to_string());
            }
            let root = GetAncestor(hwnd, GA_ROOT);
            self.current_hwnd = Some(root.0 as isize);
            Ok(root.0 as isize)
        }
    }

    pub fn scroll(&self, delta: i32) -> Result<(), String> {
        let hwnd = self.current_hwnd
            .map(|h| HWND(h as *mut _))
            .ok_or_else(|| "未设置目标窗口".to_string())?;

        unsafe {
            let _ = SetForegroundWindow(hwnd);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));

        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: 0,
                    dy: 0,
                    mouseData: delta as u32,
                    dwFlags: MOUSEEVENTF_WHEEL,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        unsafe {
            let sent = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
            if sent == 0 {
                return Err("SendInput 失败".to_string());
            }
        }

        Ok(())
    }

    pub fn scroll_via_message(&self, delta: i32) -> Result<(), String> {
        let hwnd = self.current_hwnd
            .map(|h| HWND(h as *mut _))
            .ok_or_else(|| "未设置目标窗口".to_string())?;

        let wparam = WPARAM((delta as usize) << 16);
        unsafe {
            let mut point = POINT::default();
            GetCursorPos(&mut point).map_err(|_| "获取光标失败".to_string())?;
            // ScreenToClient returns BOOL in windows-rs
            ScreenToClient(hwnd, &mut point).ok();
            let lparam = LPARAM((point.y as isize) << 16 | (point.x as isize));

            let _ = SendMessageW(hwnd, WM_MOUSEWHEEL, wparam, lparam);
        }
        Ok(())
    }

    pub fn page_down(&self) -> Result<(), String> {
        let inputs = [
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_NEXT,
                        wScan: 0,
                        dwFlags: KEYBD_EVENT_FLAGS(0), // KEYDOWN
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_NEXT,
                        wScan: 0,
                        dwFlags: KEYBD_EVENT_FLAGS(2), // KEYUP
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
        ];

        unsafe {
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }
        Ok(())
    }
}
