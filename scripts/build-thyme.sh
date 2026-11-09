#!/usr/bin/env bash
# Builds Skyward for the Xiaomi 10S (thyme) on top of a stock boot image.
#
# The stock boot image is not shipped here. Take it from the phone's own
# inactive slot while the running one is `_a`:
#
#   adb shell dd if=/dev/block/by-name/boot_b of=/tmp/boot_b.img bs=1M
#   adb pull /tmp/boot_b.img boot-b-stock.img
#
# If what you have instead is an image an earlier build wrote, `strip` turns it
# back into a clean baseline:
#
#   target/release/skyward-pack strip --image old.img --out stock-boot.img
#
# On Windows the build runs from a path without spaces, and a short one: rustc's
# raw-dylib import libraries carry the build path in a symbol, and a space in it
# breaks the link.
#
# Needs: rust (x86_64-pc-windows-gnu, target aarch64-linux-android), the NDK
# at $NDK, and a MinGW gcc on PATH (w64devkit) for the host tools and for the
# C and assembly in ring.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NDK="${NDK:-${ANDROID_NDK_HOME:?set NDK or ANDROID_NDK_HOME to the Android NDK (r29)}}"
HOST="${NDK_HOST:-$(ls "$NDK/toolchains/llvm/prebuilt" | head -1)}"
export PATH="$NDK/toolchains/llvm/prebuilt/$HOST/bin:$PATH"
export CC_aarch64_linux_android=clang.exe
export CFLAGS_aarch64_linux_android="--target=aarch64-linux-android30"
export AR_aarch64_linux_android=llvm-ar.exe

STOCK="${STOCK:?set STOCK to the phone's stock boot image}"
OUT="${OUT:-$ROOT/target/thyme}"

cd "$ROOT"
mkdir -p "$OUT"

echo "==> the packer, for the computer"
cargo build --release -p skyward-pack

echo "==> Skyward, for the phone"
cargo build --release --target aarch64-linux-android -p skyward

echo "==> the image, on top of the stock boot image"
target/release/skyward-pack recovery \
  --stock "$STOCK" \
  --binary target/aarch64-linux-android/release/skyward \
  --config devices/thyme.toml \
  --partition boot \
  --font resources/fonts/MiSansVF.ttf \
  --mono resources/fonts/KirkMono-Regular.ttf \
  --mono-license resources/fonts/KirkMono-OFL.txt \
  --out "$OUT/skyward-thyme.img"

echo "==> wrote $OUT/skyward-thyme.img"
