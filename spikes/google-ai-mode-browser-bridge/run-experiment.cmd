@echo off
setlocal

tasklist /FI "IMAGENAME eq moe-desktop.exe" 2>NUL | findstr /I /C:"moe-desktop.exe" >NUL
if not errorlevel 1 (
  echo M.O.E. is already running.
  echo Close every M.O.E. window, then run this file again.
  pause
  exit /b 1
)

pushd "%~dp0\..\.."
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
set "CARGO_TARGET_DIR=%CD%\target\google-ai-browser-bridge-experiment"
set "MOE_EXPERIMENT_GOOGLE_AI_BRIDGE=1"

echo Building the M.O.E. Google AI Browser Bridge experiment...
call npm.cmd run tauri:build --workspace @moe/desktop
if errorlevel 1 (
  echo.
  echo Build failed. Nothing was started.
  popd
  pause
  exit /b 1
)

echo.
echo Starting the experimental M.O.E....
start "" "%CARGO_TARGET_DIR%\release\moe-desktop.exe"
popd
exit /b 0
