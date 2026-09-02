# Fetch Screen

一款融合 **Lark 滚动长截图** 与 **Snipaste 贴图置顶** 的 Windows 桌面截图工具。

基于 **Tauri v2**（Rust 后端 + WebView2 前端）构建，界面简约、启动迅速。

## ✨ 功能

- 🖥️ **全屏截图** — 一键捕获所有显示器（多显示器合成，混合 DPI 适配）
- 🔲 **区域截图** — 独立全屏选区覆盖层：拖拽框选、尺寸提示、暗色遮罩
- 📜 **滚动长截图** — 框选区域后手动/自动滚动采集，MAD 列匹配拼接，带进度提示与"滚动太快"限速警告
- 🪟 **浮窗预览** — 截完图右下角浮窗展示，支持：置顶 / 穿透 / 透明度滑块 / 滚轮缩放 / 拖动
- 📌 **贴图置顶** — 截图一键钉在屏幕顶层，双击切换鼠标穿透，可缩放旋转
- 📋 **自动进剪贴板** — 截图成功后自动复制到剪贴板
- ⚙️ **自定义热键** — 区域 / 全屏 / 滚动 / 贴图穿透 快捷键可在设置中录制修改
- 🗂️ **缓存目录** — 一键打开截图缓存文件夹

> 标注编辑器、OCR 离线识别：**规划中，尚未实现**。

## 🛠️ 技术栈

| 层 | 技术 |
|----|------|
| 框架 | Tauri v2 |
| 后端 | Rust（截图引擎、图像拼接、Win32 API） |
| 前端 | React 18 + TypeScript + Canvas + Zustand |
| 滚动拼接 | 列采样 MAD 匹配（FFT 相位相关作为回退） |
| 构建 | Vite + Cargo（release 已启用 LTO） |

## 🚀 快速开始（开发）

```bash
# 1. 安装依赖
npm install

# 2. 开发模式（自动启动 Vite + 编译 Rust）
npm run tauri dev

# 3. 构建发布版（生成 exe 与 NSIS 安装包）
npm run tauri build
```

> **要求**：Windows 10/11。开发需安装 Rust（GNU 工具链）、Node.js ≥ 18；运行需系统自带或安装 WebView2 Runtime（Win11 自带）。

## 📦 分发方式

发布产物位于 `src-tauri/target/release/`：

| 方式 | 产物 | 说明 |
|------|------|------|
| **安装包（推荐）** | `bundle/nsis/Fetch Screen_0.1.0_x64-setup.exe` | 一键安装，自动创建开始菜单/桌面快捷方式 |
| **便携版** | `fetch-screen.exe` + `WebView2Loader.dll` | 两个文件放同一目录即可直接运行，无需安装 |

**便携版说明**：`fetch-screen.exe` 需与 `WebView2Loader.dll` 放在同一目录（Win11 系统自带 WebView2 运行时）。若目标机器是 Win10，请先安装 [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)。

## ⌨️ 默认热键

| 功能 | 默认快捷键 |
|------|-----------|
| 区域截图 | `Alt+Shift+A` |
| 全屏截图 | `Ctrl+Alt+A` |
| 滚动长截图 | `Ctrl+Shift+A` |
| 贴图穿透切换 | `Ctrl+Shift+P` |

> 可在主窗口「设置」中录制修改，并支持勾选「截图时隐藏 UI」。

## 📁 目录结构

```
fetch_screen/
├── src/                         # 前端 (React + TS)
│   ├── App.tsx                  # 主窗口（动作按钮 + 热键/托盘监听）
│   ├── pages/overlay/           # 全屏选区覆盖层 / 滚动选区 / 工具栏
│   ├── pages/preview/           # 截图像素级浮窗预览
│   ├── pages/pin/               # 贴图窗口
│   ├── components/              # 通用组件（设置弹窗等）
│   └── stores/                  # Zustand 状态管理
├── src-tauri/                   # 后端 (Rust)
│   ├── src/
│   │   ├── capture/             # 截图引擎（GDI 多屏合成 / 裁剪）
│   │   ├── scrollshot/          # 滚动长截图（帧捕获 / MAD 匹配 / 拼接）
│   │   ├── pin_manager/         # 贴图窗口管理
│   │   └── system/              # 托盘 / 热键 / 配置 / 剪贴板
│   ├── icons/                   # 应用图标
│   └── tauri.conf.json          # Tauri 配置（窗口 / 打包）
├── overlay.html                 # 覆盖层独立 HTML 入口
├── preview.html                 # 预览浮窗 HTML 入口
├── pin.html / pinhandle.html    # 贴图 / 穿透把手 HTML 入口
├── scroll_frame.html / scroll_toolbar.html  # 滚动截图相关 HTML 入口
└── package.json
```

## ⚙️ 配置与数据

- **配置文件**：`%APPDATA%\fetch-screen\config.json`（热键、截图隐藏 UI、预览背景等）
- **截图缓存**：`%TEMP%\fetch_screen\`（可在主窗口一键打开清理）
- **用户截图**：保存于系统「图片」目录

## 📄 许可证

MIT
