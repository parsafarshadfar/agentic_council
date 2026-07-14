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

rem Locate npm via the AgenticCouncil tools folder (installed by install-windows.ps1)
rem or fall back to the system PATH.
set "TOOLS_NODE=%LOCALAPPDATA%\AgenticCouncil\tools\nodejs"
if exist "%TOOLS_NODE%\npm.cmd" (
    set "PATH=%TOOLS_NODE%;%PATH%"
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

npm run tauri -- dev
set "EXIT_CODE=%ERRORLEVEL%"
popd

if not "%EXIT_CODE%"=="0" (
    echo.
    echo The app exited with an error. The message is shown above.
    pause
)

exit /b %EXIT_CODE%
