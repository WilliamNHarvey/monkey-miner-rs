#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
DIST_DIR="dist/monkey-miner-${OS}-${ARCH}"

cargo build --release

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"
cp target/release/monkey-miner "$DIST_DIR/"
cp -R assets "$DIST_DIR/"
chmod +x "$DIST_DIR/monkey-miner"

printf 'Built %s\n' "$DIST_DIR/monkey-miner"
printf 'Run with: cd %s && ./monkey-miner\n' "$DIST_DIR"
