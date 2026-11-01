@echo off
title Allow Galebound through Windows Firewall
rem Lets friends reach your Galebound server: opens TCP port 7777 for
rem incoming connections on all network types (Tailscale is often "Public").
net session >nul 2>&1
if errorlevel 1 (
  echo Asking for administrator permission...
  powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
  exit /b
)
netsh advfirewall firewall delete rule name="Galebound Server" >nul 2>&1
netsh advfirewall firewall add rule name="Galebound Server" dir=in action=allow protocol=TCP localport=7777 profile=any
if errorlevel 1 (
  echo Could not add the firewall rule.
) else (
  echo Done: Windows Firewall now lets friends connect to port 7777.
  echo To undo it later, delete the "Galebound Server" rule in Windows Defender Firewall.
)
pause
