@echo off
setlocal
title Agentic Council

rem Keep this file ASCII-only. Windows cmd.exe can misparse Unicode batch files.
pushd "%~dp0" >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Could not open the folder that contains this script.
    echo Move the Agentic Council folder to a normal local folder and try again.
    pause
    exit /b 1
)

rem Verify node_modules are present.
if not exist "%CD%\node_modules" (
    echo [ERROR] Dependencies are not installed.
    echo Run install.bat first to set up the project, then use run.bat to launch.
    popd
    pause
    exit /b 1
)

rem Locate a private Node.js copy installed by install-windows.ps1, or fall
rem back to the system PATH. Private copies include their version in the
rem directory name (for example node-v22.x.x-win-x64).
for /d %%D in ("%LOCALAPPDATA%\AgenticCouncil\tools\node-v*-win-*") do (
    if exist "%%~fD\npm.cmd" set "PATH=%%~fD;%PATH%"
)

where npm >nul 2>&1
if errorlevel 1 (
    echo [ERROR] npm was not found.
    echo Run install.bat first to install Node.js and the project dependencies.
    popd
    pause
    exit /b 1
)

echo.
echo  Starting Agentic Council...
echo  Keep this window open while the app is running.
echo  Close it to stop the app.
echo.

rem Remove bounded, disposable Rust build caches before launching. The helper
rem verifies every deletion remains inside src-tauri\target and skips cleanup
rem if another Cargo build owns the target directory.
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%CD%\install-windows.ps1" -MaintainCacheOnly
if errorlevel 1 (
    echo [ERROR] Rust build-cache maintenance failed.
    popd
    pause
    exit /b 1
)

set "CARGO_INCREMENTAL=0"
npm run tauri -- dev
set "EXIT_CODE=%ERRORLEVEL%"
popd

if not "%EXIT_CODE%"=="0" (
    echo.
    echo The app exited with an error. The message is shown above.
    pause
)

exit /b %EXIT_CODE%
