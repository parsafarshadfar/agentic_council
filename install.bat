@echo off
setlocal EnableDelayedExpansion
chcp 65001 >nul 2>&1

:: ============================================================
::  Agentic Council — One-Click Windows Installer
::  Double-click this file. It will install everything needed
::  and then launch the application for the first time.
::  Subsequent runs just launch the already-built app.
:: ============================================================

title Agentic Council Installer

echo.
echo  ╔══════════════════════════════════════════════════════╗
echo  ║        Agentic Council — One-Click Installer         ║
echo  ║   This window will set up everything automatically.  ║
echo  ║   Grab a coffee — first run takes 10-20 minutes.     ║
echo  ╚══════════════════════════════════════════════════════╝
echo.

:: ── Detect the folder where this script lives ──────────────
set "SCRIPT_DIR=%~dp0"
set "SCRIPT_DIR=%SCRIPT_DIR:~0,-1%"
echo [INFO] Project folder: %SCRIPT_DIR%
echo.

:: ── Check for admin rights (needed for winget in some setups)
net session >nul 2>&1
if %errorlevel% neq 0 (
    echo [INFO] Requesting administrator rights for installation...
    powershell -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
    exit /b
)

:: ── 1. Check / Install Node.js (v22 LTS) ───────────────────
echo [STEP 1/5] Checking Node.js...
where node >nul 2>&1
if %errorlevel% equ 0 (
    for /f "tokens=*" %%v in ('node --version 2^>nul') do set NODE_VER=%%v
    echo [OK] Node.js already installed: !NODE_VER!
) else (
    echo [INSTALL] Node.js not found. Installing via winget...
    echo           (This is safe and only happens once.)
    winget install --id OpenJS.NodeJS.LTS --exact --accept-package-agreements --accept-source-agreements --silent
    if !errorlevel! neq 0 (
        echo.
        echo [ERROR] winget could not install Node.js automatically.
        echo         Please download and install Node.js from:
        echo         https://nodejs.org/en/download
        echo         Then double-click this file again.
        pause
        exit /b 1
    )
    :: Refresh PATH for this session
    for /f "tokens=*" %%p in ('powershell -Command "[System.Environment]::GetEnvironmentVariable(\"PATH\",\"Machine\")"') do set "PATH=%%p;%PATH%"
    echo [OK] Node.js installed successfully.
)
echo.

:: ── 2. Check / Install Rust (rustup) ───────────────────────
echo [STEP 2/5] Checking Rust...
where rustup >nul 2>&1
if %errorlevel% equ 0 (
    echo [OK] Rust toolchain manager (rustup) already installed.
) else (
    echo [INSTALL] Rust not found. Installing via winget...
    winget install --id Rustlang.Rustup --exact --accept-package-agreements --accept-source-agreements --silent
    if !errorlevel! neq 0 (
        echo.
        echo [ERROR] winget could not install Rust automatically.
        echo         Please download the installer from:
        echo         https://www.rust-lang.org/tools/install
        echo         Run it, then double-click this file again.
        pause
        exit /b 1
    )
    :: Refresh PATH
    set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
    echo [OK] Rust installed successfully.
)
echo.

:: ── 3. Check / Install MSVC Build Tools ────────────────────
echo [STEP 3/5] Checking C++ Build Tools (required by Rust on Windows)...
:: Quick heuristic: check if cl.exe is findable or VS is installed
reg query "HKLM\SOFTWARE\Microsoft\VisualStudio\14.0" >nul 2>&1
if %errorlevel% equ 0 (
    echo [OK] Visual Studio detected.
) else (
    where cl >nul 2>&1
    if %errorlevel% equ 0 (
        echo [OK] C++ compiler detected.
    ) else (
        :: Check for Build Tools via registry
        reg query "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall" /s /f "Microsoft C++ Build Tools" >nul 2>&1
        if %errorlevel% equ 0 (
            echo [OK] C++ Build Tools already installed.
        ) else (
            echo [INSTALL] Installing Visual C++ Build Tools (required by Rust)...
            echo           This is a Microsoft package — large download, ~3-5 GB.
            winget install --id Microsoft.VisualStudio.2022.BuildTools --exact --accept-package-agreements --accept-source-agreements --silent --override "--add Microsoft.VisualStudio.Workload.VCTools --add Microsoft.VisualStudio.Component.Windows11SDK.22621 --passive --norestart"
            if !errorlevel! neq 0 (
                echo.
                echo [WARN] Could not install Build Tools automatically.
                echo        Please install them manually from:
                echo        https://visualstudio.microsoft.com/visual-cpp-build-tools/
                echo        Select "Desktop development with C++" during install.
                echo        Then double-click this file again.
                pause
                exit /b 1
            )
            echo [OK] C++ Build Tools installed.
        )
    )
)
echo.

:: ── 4. Set the exact Rust toolchain version ────────────────
echo [STEP 4/5] Setting up the required Rust toolchain version (1.97.0)...
:: Ensure cargo is in PATH
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
rustup toolchain install 1.97.0 --profile minimal --component rustfmt clippy
if %errorlevel% neq 0 (
    echo [ERROR] Failed to install Rust 1.97.0. Check your internet connection.
    pause
    exit /b 1
)
rustup default 1.97.0
echo [OK] Rust 1.97.0 set as default.
echo.

:: ── 5. Install npm dependencies ────────────────────────────
echo [STEP 5/5] Installing application dependencies...
cd /d "%SCRIPT_DIR%"
if not exist "node_modules" (
    echo [INSTALL] Running npm install (downloads ~500 MB once)...
    call npm install
    if !errorlevel! neq 0 (
        echo [ERROR] npm install failed. Check your internet connection.
        pause
        exit /b 1
    )
    echo [OK] Dependencies installed.
) else (
    echo [OK] Dependencies already installed.
)
echo.

:: ── Launch ─────────────────────────────────────────────────
echo  ╔══════════════════════════════════════════════════════╗
echo  ║  Setup complete! Launching Agentic Council...        ║
echo  ║  First launch compiles the Rust backend.             ║
echo  ║  This takes 5-15 minutes. Subsequent launches are    ║
echo  ║  instant. Please be patient!                         ║
echo  ╚══════════════════════════════════════════════════════╝
echo.
echo [LAUNCH] Starting app... (keep this window open until the app appears)
echo.

cd /d "%SCRIPT_DIR%"
call npm run tauri -- dev

if %errorlevel% neq 0 (
    echo.
    echo [ERROR] The application failed to start.
    echo         Please share the error messages above with the developer.
)

pause
