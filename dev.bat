@echo off
chcp 65001 >nul
cd /d "%~dp0"

echo.
echo ╔════════════════════════════════════════╗
echo ║   cc-gui — Claude Code Desktop GUI     ║
echo ║   Development Launcher                  ║
echo ╚════════════════════════════════════════╝
echo.

REM ---- 1. Set MinGW PATH ----
set "MINGW_BIN=%LocalAppData%\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\mingw64\bin"
set "PATH=%USERPROFILE%\.cargo\bin;%MINGW_BIN%;%PATH%"

REM ---- 2. Verify tools ----
where rustc >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] rustc not found. Please install Rust first.
    pause
    exit /b 1
)

where node >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] node not found. Please install Node.js first.
    pause
    exit /b 1
)

echo [OK] rustc: found
echo [OK] node:  found
echo [OK] MinGW: %MINGW_BIN%

REM ---- 3. Check node_modules ----
if not exist "node_modules" (
    echo.
    echo [INFO] Installing npm dependencies...
    call npm install
    if %errorlevel% neq 0 (
        pause
        exit /b 1
    )
)

REM ---- 4. Launch Tauri dev mode ----
echo.
echo [START] Launching Tauri dev mode...
echo.
call npx tauri dev

pause
