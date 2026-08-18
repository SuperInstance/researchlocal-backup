@echo off
REM =====================================================================
REM start_capture_tray.bat - TZ Pro Agent tray launcher
REM
REM Use this from the desktop shortcut or any other place you want a
REM double-click entry to the tray app. We use pythonw.exe (no console)
REM so the captain doesn't see a window flicker and disappear.
REM
REM This script also accepts a `link` argument to materialize a desktop
REM shortcut if you haven't got one yet:
REM
REM     start_capture_tray.bat link
REM
REM =====================================================================

setlocal
set "ROOT=%~dp0"
if "%ROOT:~-1%"=="\" set "ROOT=%ROOT:~0,-1%"

if /I "%1"=="link" goto L_LINK
if /I "%1"=="unlink" goto L_UNLINK

REM ---------- default: launch the tray app --------------------------
REM Kill any existing tray so we don't end up with duplicates.
taskkill /FI "WINDOWTITLE eq TZ Pro Agent tray*" /T /F >nul 2>&1
REM Use the same Python that's running this script to keep venvs aligned.
set "PYEXE=%LOCALAPPDATA%\Programs\Python\Python314\pythonw.exe"
if not exist "%PYEXE%" (
    set "PYEXE=pythonw.exe"
)
echo Starting TZ Pro Agent tray at %ROOT%...
start "" /B "%PYEXE%" -u "%ROOT%\tray_app.py"
endlocal
exit /b 0

:L_LINK
echo Creating desktop shortcut...
powershell -NoProfile -ExecutionPolicy Bypass -File "%ROOT%\scripts\_make_shortcut.ps1"
endlocal
exit /b 0

:L_UNLINK
echo Removing desktop shortcut...
del /Q "%USERPROFILE%\Desktop\TZ Pro Agent Tray.lnk" 2>nul
endlocal
exit /b 0
