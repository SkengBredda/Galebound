@echo off
title Allow Galebound through Windows Firewall
rem Lets friends reach your Galebound server:
rem  - removes any "block" rules Windows made for the Galebound server
rem    (made when only "Private networks" was ticked; blocks win over allows)
rem  - opens TCP port 7777 for incoming connections on every network type
net session >nul 2>&1
if errorlevel 1 (
  echo Asking for administrator permission...
  powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
  exit /b
)
echo Removing Galebound block rules...
powershell -NoProfile -Command "Get-NetFirewallRule -Direction Inbound | Where-Object { $_.Action -eq 'Block' -and ($_.DisplayName -like '*galebound*' -or (($_ | Get-NetFirewallApplicationFilter).Program -like '*galebound*')) } | ForEach-Object { Write-Host ('  removed: ' + $_.DisplayName + ' (' + $_.Profile + ')'); Remove-NetFirewallRule -Name $_.Name }"
netsh advfirewall firewall delete rule name="Galebound Server" >nul 2>&1
netsh advfirewall firewall add rule name="Galebound Server" dir=in action=allow protocol=TCP localport=7777 profile=any
if errorlevel 1 (
  echo Could not add the firewall rule.
  pause
  exit /b 1
)
echo.
echo Done: Windows Firewall now lets friends connect to port 7777.
echo.
echo ---- Report (send this to Claude if friends still cannot connect) ----
powershell -NoProfile -Command "Get-NetConnectionProfile | ForEach-Object { 'Network: ' + $_.InterfaceAlias + ' = ' + $_.NetworkCategory }; Get-NetFirewallRule -Direction Inbound | Where-Object { $_.DisplayName -like '*galebound*' -or (($_ | Get-NetFirewallApplicationFilter).Program -like '*galebound*') } | ForEach-Object { 'Rule: ' + $_.DisplayName + ' | ' + $_.Action + ' | ' + $_.Profile + ' | enabled=' + $_.Enabled }; Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.IPAddress -notlike '127.*' -and $_.IPAddress -notlike '169.254.*' } | ForEach-Object { 'PC address: ' + $_.IPAddress + ' (' + $_.InterfaceAlias + ')' }; $l = Get-NetTCPConnection -LocalPort 7777 -State Listen -ErrorAction SilentlyContinue; if ($l) { $l | ForEach-Object { 'Server listening on ' + $_.LocalAddress + ':7777' } } else { 'Server is NOT running (start it, then run this again)' }"
echo ----------------------------------------------------------------------
pause
