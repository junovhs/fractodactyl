@echo off
rem Mandelbrot explorer (EXPL-01): double-click to build fd if needed, start `fd explore`
rem (unless it is already running) and open it in Chrome. Close the minimised
rem "fd explore" window to stop the renderer.
cd /d "%~dp0.."
where cargo >nul 2>nul && cargo build --release -p fd-cli
if not exist "target\release\fd.exe" (
  echo fd.exe was not built; install Rust ^(cargo^) and try again.
  pause
  exit /b 1
)
powershell -NoProfile -Command "try { (New-Object Net.Sockets.TcpClient('127.0.0.1', 8737)).Close(); exit 0 } catch { exit 1 }"
if errorlevel 1 (
  start "fd explore" /min "target\release\fd.exe" explore --port 8737
  "%SystemRoot%\System32\timeout.exe" /t 1 /nobreak >nul
)
if exist "%ProgramFiles%\Google\Chrome\Application\chrome.exe" (
  start "" "%ProgramFiles%\Google\Chrome\Application\chrome.exe" http://127.0.0.1:8737/
) else (
  start "" http://127.0.0.1:8737/
)
