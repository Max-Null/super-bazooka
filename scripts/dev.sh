#!/usr/bin/env bash
# cc-gui — One-click development launcher (Git Bash / MSYS2 / WSL)
# Usage: bash scripts/dev.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$SCRIPT_DIR"

echo ""
echo "╔════════════════════════════════════════╗"
echo "║   cc-gui — Claude Code Desktop GUI     ║"
echo "║   Development Launcher                  ║"
echo "╚════════════════════════════════════════╝"
echo ""

# ---- 1. Locate MinGW ----
WINLIBS_DIR="$(find "$LOCALAPPDATA/Microsoft/WinGet/Packages" -maxdepth 1 -type d -name "BrechtSanders.WinLibs.*" 2>/dev/null | head -1)"
if [ -n "$WINLIBS_DIR" ] && [ -f "$WINLIBS_DIR/mingw64/bin/gcc.exe" ]; then
    MINGW_BIN="$WINLIBS_DIR/mingw64/bin"
else
    # Fallback: try the known install path
    MINGW_BIN="$LOCALAPPDATA/Microsoft/WinGet/Packages/BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe/mingw64/bin"
fi

# Convert to Unix path if running in MSYS2/Git Bash
if command -v cygpath &>/dev/null; then
    MINGW_BIN="$(cygpath -u "$MINGW_BIN" 2>/dev/null || echo "$MINGW_BIN")"
fi

export PATH="$HOME/.cargo/bin:$MINGW_BIN:$PATH"
export RUST_BACKTRACE=1

# ---- 2. Verify tools ----
check_tool() {
    if ! command -v "$1" &>/dev/null; then
        echo "[ERROR] $1 not found in PATH"
        exit 1
    fi
    echo "[OK]  $1"
}

check_tool rustc
check_tool node
check_tool npm

if [ -f "$MINGW_BIN/gcc.exe" ] || [ -f "$MINGW_BIN/gcc" ]; then
    echo "[OK]  MinGW (gcc)"
else
    echo "[WARN] gcc not found in MinGW path — Rust build may fail"
fi

# ---- 3. Install npm deps if needed ----
if [ ! -d "node_modules" ]; then
    echo ""
    echo "[INFO] Installing npm dependencies..."
    npm install
fi

# ---- 4. Launch Tauri dev mode ----
echo ""
echo "[START] Launching Tauri dev mode..."
echo ""

npx tauri dev
