@echo off
chcp 65001 >nul
cd /d "%~dp0"

REM ============================================================
REM  Fetch Screen one-click launcher (double-click to run)
REM  Starts vite + tauri dev with correct Rust environment
REM ============================================================

set "RUSTUP_HOME=D:\rust\rustup"
set "CARGO_HOME=D:\rust\cargo"
set "CARGO_REGISTRIES_CRATES_IO_INDEX=sparse+https://rsproxy.cn/index/"
set "CARGO_REGISTRIES_CRATES_IO_API=https://rsproxy.cn/api/v1"

REM Prepend self-contained binutils to avoid 32-bit MinGW windres conflict
set "SELF=D:\rust\rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained"
set "PATH=%SELF%;D:\rust\cargo\bin;%PATH%"

echo ==============================================
echo   Fetch Screen launching...
echo     frontend: vite  (http://localhost:1420)
echo     backend:  tauri (first build is slow)
echo   Press Ctrl+C to stop
echo ==============================================

npm run tauri dev

pause
