@echo off
rem Build the Android .apk on Windows 10/11 (phones / tablets, Android 5.0+).
rem
rem Needs Docker Desktop (https://www.docker.com/products/docker-desktop/),
rem installed with the WSL 2 option and running (whale icon in the tray).
rem The first build downloads the Android build tools (a few GB) and takes a
rem while; later builds are much quicker.
rem
rem Result: Galebound.apk in this folder. Copy it to your phone and open it
rem (allow "install unknown apps").
setlocal
cd /d "%~dp0"
set IMAGE=galebound-android-builder

docker version >nul 2>&1
if errorlevel 1 (
    echo Docker is not running. Install Docker Desktop, start it, wait until
    echo it says "Engine running", then run this again.
    pause
    exit /b 1
)

docker image inspect %IMAGE% >nul 2>&1
if errorlevel 1 (
    echo Preparing the Android build tools ^(first time only^)...
    docker build -t %IMAGE% -f android\Dockerfile android
    if errorlevel 1 goto failed
)

docker run --rm -v "%cd%":/root/src -v galebound-cargo:/root/.cargo/registry -w /root/src %IMAGE% cargo quad-apk build --release
if errorlevel 1 goto failed

for %%f in (target\android-artifacts\release\apk\*.apk) do (
    copy /y "%%f" Galebound.apk >nul
    echo.
    echo Done: %cd%\Galebound.apk
    pause
    exit /b 0
)
echo The build finished but no .apk was found - see the messages above.
pause
exit /b 1

:failed
echo.
echo The build failed - see the messages above.
pause
exit /b 1
