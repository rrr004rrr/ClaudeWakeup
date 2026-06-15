@echo off
chcp 65001 >nul
REM 編譯獨立的 ClaudeWakeup.exe（release，已最佳化檔案大小）。
cargo build --release
if %ERRORLEVEL% neq 0 (
    echo 編譯失敗。
    exit /b %ERRORLEVEL%
)
echo.
echo 已產生：target\release\ClaudeWakeup.exe
