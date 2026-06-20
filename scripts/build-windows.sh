#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

TARGET="x86_64-pc-windows-gnu"
DIST_DIR="dist/monkey-miner-windows-x86_64"
EXE="target/${TARGET}/release/monkey-miner.exe"

if ! rustup target list --installed | grep -qx "$TARGET"; then
    printf 'Missing Rust target: %s\n' "$TARGET" >&2
    printf 'Install it with: rustup target add %s\n' "$TARGET" >&2
    exit 1
fi

if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
    printf 'Missing Windows GNU linker: x86_64-w64-mingw32-gcc\n' >&2
    printf 'On macOS, install it with: brew install mingw-w64\n' >&2
    exit 1
fi

CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc \
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
    cargo build --release --target "$TARGET"

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"
cp "$EXE" "$DIST_DIR/"
cp -R assets "$DIST_DIR/"

printf 'Built %s/monkey-miner.exe\n' "$DIST_DIR"
printf 'Run on Windows with: %s\\monkey-miner.exe\n' "${DIST_DIR//\//\\}"
