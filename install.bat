@echo off
setlocal
title Agentic Council Installer

rem Keep this file ASCII-only. Windows cmd.exe can misparse Unicode batch files.
pushd "%~dp0" >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Could not open the folder that contains this installer.
    echo Move the Agentic Council folder to a normal local folder and try again.
    pause
    exit /b 1
)

set "INSTALLER=%CD%\install-windows.ps1"
if not exist "%INSTALLER%" (
    echo [ERROR] install-windows.ps1 is missing.
    echo Download or extract the complete Agentic Council folder and try again.
    popd
    pause
    exit /b 1
)

powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%INSTALLER%"
set "EXIT_CODE=%ERRORLEVEL%"
popd

if not "%EXIT_CODE%"=="0" (
    echo.
    echo Setup did not complete. The useful error is shown above.
    echo You can safely run install.bat again after fixing it.
    pause
)

exit /b %EXIT_CODE%
