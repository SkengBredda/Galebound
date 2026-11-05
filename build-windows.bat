@echo off
setlocal
title Build Galebound for Windows
cd /d "%~dp0"
set OUT=GaleBound_Windows

where cargo >nul 2>nul
if errorlevel 1 (
  echo Rust is not installed. Install it from https://rustup.rs and run this again.
  pause
  exit /b 1
)

echo Building Galebound (the first build downloads libraries and takes a few minutes)...
cargo build --release --workspace
if errorlevel 1 (
  echo.
  echo The build failed. Copy the red "error" lines and send them to Claude.
  pause
  exit /b 1
)

if exist "%OUT%" rmdir /s /q "%OUT%"
mkdir "%OUT%"
copy /y "target\release\galebound.exe" "%OUT%\Galebound.exe" >nul
copy /y "target\release\galebound-server.exe" "%OUT%\GaleboundServer.exe" >nul
copy /y README.md "%OUT%\README.md" >nul
copy /y PATCH_NOTES.md "%OUT%\PATCH_NOTES.md" >nul
copy /y allow-firewall.bat "%OUT%\allow-firewall.bat" >nul
(
  echo @echo off
  echo title Galebound Server
  echo cd /d "%%~dp0"
  echo GaleboundServer.exe %%*
  echo pause
) > "%OUT%\Start Server.bat"

echo.
echo Done! Your game is in the %OUT% folder:
echo   Galebound.exe        - the game (give this to your friends)
echo   Start Server.bat     - runs GaleboundServer.exe for hosting
explorer "%OUT%"
pause
