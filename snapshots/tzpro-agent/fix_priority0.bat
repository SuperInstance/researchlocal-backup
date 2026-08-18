@echo off
REM =====================================================================
REM fix_priority0.bat — Restore TZ Pro position relay end-to-end.
REM
REM What it does (in order):
REM   1. Kill any stale pythonw processes bound to TCP:6006 or :8654
REM      (leaves mini-agent.exe and hermes-construct alone)
REM   2. Start the NMEA bridge (COM6 -> TCP:6006 + HTTP:8654)
REM   3. Start the dashboard (Flask/Starlette on :8090)
REM   4. Probe /ready and /health until green, then exit
REM
REM Usage:  double-click, or invoke from any shell.
REM
REM This is the single button that fixes "TZ Pro can't see my position"
REM after a reboot. The scheduled-task installer (install_bridge_task.bat)
REM handles auto-start at boot, but requires Administrator.
REM =====================================================================

setlocal
set BRIDGE_DIR=C:\Users\casey\tzpro-agent
set BRIDGE_SCRIPT=nmea_bridge.py
set BRIDGE_PORT=COM6
set BRIDGE_BAUD=4800
set PYEXE=C:\Python314\pythonw.exe

if not exist "%PYEXE%" (
  echo ## pythonw.exe not found at %PYEXE%; falling back to PATH
  set "PYEXE=pythonw.exe"
)

echo.
echo === Priority-0 fix: restore TZ Pro GPS relay ===
echo.

REM --- 1. Kill any listener on :6006 / :8654 (last-resort netstat kill) ---
for /f "tokens=5" %%P in ('netstat -ano ^| findstr ":6006.*LISTENING"') do (
  echo ## Killing stale listener on :6006 PID %%P
  taskkill /f /pid %%P >nul 2>&1
)
for /f "tokens=5" %%P in ('netstat -ano ^| findstr ":8654.*LISTENING"') do (
  echo ## Killing stale listener on :8654 PID %%P
  taskkill /f /pid %%P >nul 2>&1
)
for /f "tokens=5" %%P in ('netstat -ano ^| findstr ":8090.*LISTENING"') do (
  echo ## Killing stale listener on :8090 PID %%P
  taskkill /f /pid %%P >nul 2>&1
)
timeout /t 2 /nobreak >nul

REM --- 2. Start the bridge (COM6 -> TCP:6006 + HTTP:8654) ---
echo ## Starting nmea_bridge on %BRIDGE_PORT% @ %BRIDGE_BAUD% baud...
cd /d "%BRIDGE_DIR%"
start "" "%PYEXE%" "%BRIDGE_SCRIPT%" --port %BRIDGE_PORT% --baud %BRIDGE_BAUD%

REM --- 3. Start the dashboard on :8090 ---
echo ## Starting dashboard.py on :8090...
start "" "%PYEXE%" "dashboard.py"

REM --- 4. Wait for both services to come up, then probe ---
set RETRIES=15
:LOOP
timeout /t 1 /nobreak >nul
set /a RETRIES=RETRIES-1
"%SystemRoot%\System32\curl.exe" -s --max-time 2 http://127.0.0.1:8654/ready > "%TEMP%\tzpro_ready.txt" 2>nul
findstr /C:"\"ready\": true" "%TEMP%\tzpro_ready.txt" >nul 2>&1
if not errorlevel 1 goto READY
if %RETRIES% leq 0 goto FAIL
goto LOOP

:READY
echo.
echo ## Bridge ready: 
type "%TEMP%\tzpro_ready.txt"
echo.
echo ## Dashboard:
"%SystemRoot%\System32\curl.exe" -s --max-time 2 http://127.0.0.1:8090/api/health
echo.
echo === DONE. TZ Pro should see your position on TCP 127.0.0.1:6006 ===
echo === Dashboard: http://127.0.0.1:8090/ ===
endlocal
exit /b 0

:FAIL
echo.
echo ## TIMEOUT: bridge did not become ready after 15 seconds.
echo ## Check the bridge log or run: python "%BRIDGE_DIR%\%BRIDGE_SCRIPT%" --diag
endlocal
exit /b 1