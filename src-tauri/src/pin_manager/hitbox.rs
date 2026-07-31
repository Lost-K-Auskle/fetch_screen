/// 贴图窗口的交互区域定义
/// WebView2 不支持 WS_EX_TRANSPARENT 的逐像素穿透，
/// 因此使用 hitbox 区域来判定何时允许鼠标交互。
#[derive(Debug, Clone)]
pub struct Hitbox {
    /// 工具栏区域 (底部 48px 横条)
    pub toolbar: Option<(u32, u32, u32, u32)>, // (x, y, w, h) relative to window
    /// 缩放手柄区域 (四角和边缘)
    pub resize_handles: Vec<(u32, u32, u32, u32)>,
    /// 关闭按钮区域
    pub close_button: Option<(u32, u32, u32, u32)>,
}

impl Hitbox {
    /// 为给定窗口尺寸创建默认 hitbox 布局
    pub fn for_window(width: u32, height: u32) -> Self {
        let bar_h = 48u32;
        Self {
            toolbar: if height > bar_h {
                Some((0, height - bar_h, width, bar_h))
            } else {
                None
            },
            resize_handles: vec![
                // 四角 (8px 热区)
                (0, 0, 8, 8),                    // 左上
                (width - 8, 0, 8, 8),            // 右上
                (0, height - 8, 8, 8),           // 左下
                (width - 8, height - 8, 8, 8),  // 右下
            ],
            close_button: Some((width - 36, 4, 32, 32)),
        }
    }

    /// 检测给定窗口内坐标是否命中任何交互区域
    pub fn hit_test(&self, wx: u32, wy: u32) -> bool {
        if let Some((x, y, w, h)) = self.toolbar {
            if wx >= x && wx < x + w && wy >= y && wy < y + h {
                return true;
            }
        }
        if let Some((x, y, w, h)) = self.close_button {
            if wx >= x && wx < x + w && wy >= y && wy < y + h {
                return true;
            }
        }
        for (x, y, w, h) in &self.resize_handles {
            if wx >= *x && wx < x + w && wy >= *y && wy < y + h {
                return true;
            }
        }
        false
    }
}
