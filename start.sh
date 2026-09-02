#!/bin/bash
# ============================================================
#  Fetch Screen 一键启动脚本
#  用法：双击（Git Bash 关联 .sh）或命令行执行 `bash start.sh`
#  功能：配置 Rust 环境 → 启动前端 vite + 后端 tauri（dev 模式）
# ============================================================

# 切到脚本所在目录（项目根目录），保证从任意位置双击都能找到项目
cd "$(dirname "$0")" || exit 1

# ---- Rust 工具链环境（安装在 D 盘，使用 rsproxy 镜像）----
export RUSTUP_HOME='D:\rust\rustup'
export CARGO_HOME='D:\rust\cargo'
export RUSTUP_DIST_SERVER='https://rsproxy.cn/rustup'
export RUSTUP_UPDATE_ROOT='https://rsproxy.cn/rustup/rustup'
export CARGO_REGISTRIES_CRATES_IO_INDEX='sparse+https://rsproxy.cn/index/'
export CARGO_REGISTRIES_CRATES_IO_API='https://rsproxy.cn/api/v1'

# ---- PATH：self-contained binutils 置顶，避免 32 位 MinGW 的 windres 冲突 ----
SELF='D:\rust\rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained'
export PATH="$SELF:$CARGO_HOME/bin:$PATH"

# ---- 依赖检查 ----
if ! command -v npm >/dev/null 2>&1; then
  echo "[错误] 未找到 npm，请先安装 Node.js (https://nodejs.org)"
  echo "按回车退出..."
  read -r _
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "[错误] 未找到 cargo，Rust 环境未配置或已损坏"
  echo "按回车退出..."
  read -r _
  exit 1
fi

echo "=============================================="
echo "  Fetch Screen 正在启动..."
echo "    前端: vite  (http://localhost:1420)"
echo "    后端: tauri (cargo 编译，首次较慢)"
echo "  按 Ctrl+C 停止"
echo "=============================================="

# 启动（vite 会由 tauri 的 beforeDevCommand 自动拉起）
npm run tauri dev
status=$?

if [ $status -ne 0 ]; then
  echo ""
  echo "[错误] 启动失败（退出码 $status），请检查上方日志"
  echo "按回车退出..."
  read -r _
fi
