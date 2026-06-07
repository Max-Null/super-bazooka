# cc-gui Development Launcher (PowerShell)
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

Write-Host ""
Write-Host "╔════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║   cc-gui — Claude Code Desktop GUI     ║" -ForegroundColor Cyan
Write-Host "║   Development Launcher                  ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""

# ---- 1. Locate MinGW ----
$WinGetBase = "$env:LocalAppData\Microsoft\WinGet\Packages"
$WinLibs = Get-ChildItem -Path $WinGetBase -Directory -Filter "BrechtSanders.WinLibs.*" -ErrorAction SilentlyContinue | Select-Object -First 1

if ($WinLibs) {
    $MingwBin = Join-Path $WinLibs.FullName "mingw64\bin"
    if (Test-Path (Join-Path $MingwBin "gcc.exe")) {
        $env:Path = "$env:USERPROFILE\.cargo\bin;$MingwBin;$env:Path"
        Write-Host "[OK]  MinGW found: $MingwBin" -ForegroundColor Green
    } else {
        Write-Host "[WARN] MinGW dir found but no gcc.exe — Rust build may fail" -ForegroundColor Yellow
    }
} else {
    Write-Host "[WARN] MinGW not found via WinGet — checking PATH..." -ForegroundColor Yellow
    # Fallback: try to find gcc in PATH
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
}

# ---- 2. Verify tools ----
$tools = @{
    "rustc" = $null
    "node"  = $null
    "npm"   = $null
}

foreach ($tool in $tools.Keys) {
    $found = Get-Command $tool -ErrorAction SilentlyContinue
    if (-not $found) {
        Write-Host "[ERROR] $tool not found in PATH" -ForegroundColor Red
        Read-Host "Press Enter to exit"
        exit 1
    }
    Write-Host "[OK]  $tool" -ForegroundColor Green
}

# ---- 3. Install npm deps if needed ----
if (-not (Test-Path "node_modules")) {
    Write-Host ""
    Write-Host "[INFO] Installing npm dependencies..." -ForegroundColor Yellow
    npm install
    if ($LASTEXITCODE -ne 0) {
        Read-Host "Press Enter to exit"
        exit 1
    }
}

# ---- 4. Launch ----
Write-Host ""
Write-Host "[START] Launching Tauri dev mode..." -ForegroundColor Cyan
Write-Host ""

npx tauri dev

Read-Host "Press Enter to exit"
