#!/usr/bin/env bash
set -euo pipefail

# RustPad Release Packaging Script (Linux / Windows / macOS)
# Author: Nour Yahyaoui <nourryahyaoui@gmail.com>

VERSION="$(grep -m1 '^version = ' Cargo.toml | cut -d'"' -f2)"
OUT_DIR="target/dist"
mkdir -p "$OUT_DIR"

echo "==> Running full workspace test suite for RustPad v${VERSION}..."
cargo test --workspace

echo "==> Building stripped release binary..."
cargo build --release -p rustpad

OS_NAME="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH_NAME="$(uname -m)"
BIN_ASSET="${OUT_DIR}/rustpad-v${VERSION}-${OS_NAME}-${ARCH_NAME}"
cp target/release/rustpad "$BIN_ASSET"
chmod +x "$BIN_ASSET"

tar -czf "${BIN_ASSET}.tar.gz" \
  -C target/release rustpad \
  -C ../.. README.md LICENSE CHANGELOG.md examples

(
  cd "$OUT_DIR"
  sha256sum "$(basename "$BIN_ASSET")" "$(basename "${BIN_ASSET}.tar.gz")" > SHA256SUMS.txt
)

echo "==> Packaged release artifacts in ${OUT_DIR}:"
ls -lh "$OUT_DIR"
cat "${OUT_DIR}/SHA256SUMS.txt"
