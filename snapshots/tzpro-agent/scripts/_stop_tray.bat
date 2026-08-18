@echo off
REM stop_capture_tray.bat - clean stop of tray and daemon
REM
REM Tear down both the tray app and the capture_daemon if they're running.
REM Used both by the tray "Quit" link and by ad-hoc shutdowns from a
REM SSH/RDP session into the boat laptop.

set "ROOT=%~dp0"
if "%ROOT:~-1%"=="\" set "ROOT=%ROOT:~0,-1%"
set "ROOT=%ROOT%\.."
set "PYEXE=%LOCALAPPDATA%\Programs\Python\Python314\python.exe"
if not exist "%PYEXE%" set "PYEXE=python.exe"

echo Stopping capture_daemon...
"%PYEXE%" -u "%ROOT%\capture_daemon.py" stop

echo Stopping tray (pythonw.exe tray_app.py)...
taskkill /FI "IMAGENAME eq pythonw.exe" /FI "WINDOWTITLE eq TZ Pro Agent*" /T /F >nul 2>&1
REM Fallback: kill any pythonw running tray_app.py regardless of title
for /f "tokens=2" %%i in ('wmic process where "name='pythonw.exe' and CommandLine like '%%tray_app.py%%'" get ProcessId /value 2^>nul ^| find "ProcessId"') do (
    taskkill /PID %%i /F >nul 2>&1
)

echo Done.
