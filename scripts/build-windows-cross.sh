#!/usr/bin/env bash
set -euo pipefail

# Cross-compile standalone Windows 64-bit rustpad.exe from Linux / WSL
# Author: Nour Yahyaoui <nourryahyaoui@gmail.com>

RELEASE_TAG="${1:-v1.0.0}"
UPLOAD_FLAG="${2:-}"
TARGET="x86_64-pc-windows-gnu"
OUT_DIR="target/dist"

if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
  echo "error: x86_64-w64-mingw32-gcc not found."
  echo "Install it first on Ubuntu/Debian/WSL:"
  echo "  sudo apt update && sudo apt install -y mingw-w64"
  exit 1
fi

echo "==> Ensuring Rust target ${TARGET} is installed..."
rustup target add "$TARGET"

echo "==> Cross-compiling standalone Windows executable (rustpad.exe)..."
cargo build --release --target "$TARGET" -p rustpad

mkdir -p "$OUT_DIR"
SRC_EXE="target/${TARGET}/release/rustpad.exe"
DEST_EXE="${OUT_DIR}/rustpad-${RELEASE_TAG}-windows-x86_64.exe"

cp "$SRC_EXE" "$DEST_EXE"
cp "$SRC_EXE" "${OUT_DIR}/rustpad.exe"

(
  cd "$OUT_DIR"
  sha256sum "$(basename "$DEST_EXE")" "rustpad.exe" > SHA256SUMS-windows.txt
)

echo "==> Cross-compiled Windows .exe ready:"
ls -lh "$DEST_EXE" "${OUT_DIR}/rustpad.exe"
cat "${OUT_DIR}/SHA256SUMS-windows.txt"

if [[ "$UPLOAD_FLAG" == "--upload" ]]; then
  echo "==> Uploading to GitHub Release ${RELEASE_TAG}..."
  gh release upload "$RELEASE_TAG" "$DEST_EXE" "${OUT_DIR}/rustpad.exe" "${OUT_DIR}/SHA256SUMS-windows.txt" --clobber
  echo "==> Uploaded to https://github.com/Nour-yahyaoui/rustpad/releases/tag/${RELEASE_TAG}"
else
  echo ""
  echo "To push this .exe to GitHub Releases, run:"
  echo "  gh release upload ${RELEASE_TAG} ${DEST_EXE} ${OUT_DIR}/rustpad.exe ${OUT_DIR}/SHA256SUMS-windows.txt --clobber"
fi
