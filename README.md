# 📷 Fetch Screen

融合 **Lark 滚动截图** 与 **Snipaste 贴图置顶** 的 Windows 桌面截图工具。

## 功能

- 🔲 **区域截图** — 拖拽选区，实时尺寸显示，像素级放大镜
- 🖥️ **全屏截图** — 一键捕获所有显示器
- 📜 **滚动长截图** — 手动/自动双模式，三级回退拼接算法
- 📌 **贴图置顶** — 截图一键贴到屏幕顶层，鼠标穿透/缩放/透明度/旋转
- ✏️ **标注编辑** — 矩形、箭头、文字、画笔、马赛克、序号标注
- 🔤 **OCR 提取** — 离线文字识别 (P2)

## 技术栈

- **框架**: Tauri v2
- **后端**: Rust (截图引擎、图像拼接、Windows API)
- **前端**: React 18 + TypeScript + Canvas API + Zustand
- **拼接算法**: 列采样 MAD → 边缘 FFT 相位相关 → ORB 特征匹配

## 开发

```bash
# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 构建
npm run tauri build
```

## 目录结构

```
fetch_screen/
├── src/                     # 前端 (React)
│   ├── pages/overlay/       # 截图选区覆盖层
│   ├── pages/pin/           # 贴图窗口
│   ├── pages/editor/        # 标注编辑器
│   ├── stores/              # Zustand 状态管理
│   └── components/          # 通用组件
├── src-tauri/               # 后端 (Rust)
│   ├── src/
│   │   ├── capture/         # 截图引擎
│   │   ├── scrollshot/      # 滚动截图引擎
│   │   ├── pin_manager/     # 贴图管理
│   │   └── system/          # 系统集成
│   └── Cargo.toml
└── package.json
```

## 许可证

MIT
