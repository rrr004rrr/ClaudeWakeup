@echo off
chcp 65001 >nul
REM 在「開機啟動」資料夾建立（或移除）捷徑，讓 ClaudeWakeup 隨登入自動執行。
REM 用法：install-startup.bat          -> 安裝
REM       install-startup.bat remove   -> 移除

setlocal
set "EXE=%~dp0ClaudeWakeup.exe"
if not exist "%EXE%" set "EXE=%~dp0target\release\ClaudeWakeup.exe"
set "LNK=%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup\ClaudeWakeup.lnk"

if /I "%~1"=="remove" (
    if exist "%LNK%" del "%LNK%" & echo 已移除開機捷徑。
    if not exist "%LNK%" echo 目前沒有開機捷徑。
    goto :eof
)

if not exist "%EXE%" (
    echo 找不到執行檔，請先執行 build.bat 進行編譯。
    exit /b 1
)

powershell -NoProfile -Command ^
  "$s=(New-Object -ComObject WScript.Shell).CreateShortcut('%LNK%');" ^
  "$s.TargetPath='%EXE%';" ^
  "$s.WorkingDirectory='%~dp0';" ^
  "$s.Description='ClaudeWakeup 工具列';" ^
  "$s.Save()"

echo 已建立開機捷徑：%LNK%
endlocal
