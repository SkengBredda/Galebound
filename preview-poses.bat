@echo off
setlocal
title Galebound pose preview
cd /d "%~dp0"

where cargo >nul 2>nul
if errorlevel 1 (
  echo Rust is not installed. Install it from https://rustup.rs and run this again.
  pause
  exit /b 1
)

echo Checking the poses and drawing pose_preview.png ...
echo (the first run takes a minute or two)
echo.
cargo test --release -p galebound -- strikes_land_in_their_hitboxes idle_is_relaxed
if errorlevel 1 (
  echo.
  echo A check failed. The lines above say which pose is off.
  echo The preview will still be drawn so you can see it.
  echo.
)
cargo test --release -p galebound pose_preview -- --ignored
if errorlevel 1 (
  echo.
  echo Couldn't draw the preview. Copy the red "error" lines and send them to Claude.
  pause
  exit /b 1
)
start "" "pose_preview.png"
