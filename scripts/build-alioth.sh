#!/usr/bin/env bash
# Builds Skyward for the Redmi K40 (alioth) on top of a stock boot image.
#
# This phone keeps its recovery in the boot image and has no recovery
# partition of its own: the image is written to the boot partition of a slot,
# and the system image that slot held is replaced by it.
#
# The stock boot image is taken out of the phone maker's full update package,
# reading only what that partition takes of it, and checked against the sum
# the device configuration names:
#
#   target/release/skyward-pack stock --config devices/alioth.toml \
#     --out target/alioth/stock-boot.img
#
# If the phone is at hand, its own inactive slot gives the same image while
# the running one is `_a`:
#
#   adb shell dd if=/dev/block/by-name/boot_b of=/tmp/boot_b.img bs=1M
#   adb pull /tmp/boot_b.img boot-b-stock.img
#
# Needs: rust with the aarch64-linux-android target, the NDK at $NDK, and
# cargo-ndk on PATH.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
NDK="${NDK:-${ANDROID_NDK_HOME:?set NDK or ANDROID_NDK_HOME to the Android NDK (r29)}}"
HOST="${NDK_HOST:-$(ls "$NDK/toolchains/llvm/prebuilt" | head -1)}"
export PATH="$NDK/toolchains/llvm/prebuilt/$HOST/bin:$PATH"
export CC_aarch64_linux_android=clang
export CFLAGS_aarch64_linux_android="--target=aarch64-linux-android30"
export AR_aarch64_linux_android=llvm-ar

OUT="${OUT:-$ROOT/target/alioth}"
STOCK="${STOCK:-$OUT/stock-boot.img}"

cd "$ROOT"
mkdir -p "$OUT"

echo "==> the packer, for the computer"
cargo build --release -p skyward-pack

if [ ! -f "$STOCK" ]; then
  echo "==> the stock boot image, out of the full update package"
  target/release/skyward-pack stock --config devices/alioth.toml --out "$STOCK"
fi

echo "==> Skyward, for the phone"
cargo build --release --target aarch64-linux-android -p skyward

echo "==> the second system's first program, linked statically"
RUSTFLAGS="-C target-feature=+crt-static" \
  cargo build --release --target aarch64-linux-android -p skyward-system-init \
  --target-dir target/system-init

echo "==> the image, on top of the stock boot image"
target/release/skyward-pack recovery \
  --stock "$STOCK" \
  --binary target/aarch64-linux-android/release/skyward \
  --system-init target/system-init/aarch64-linux-android/release/system-init \
  --config devices/alioth.toml \
  --partition boot \
  --font resources/fonts/MiSansVF.ttf \
  --mono resources/fonts/KirkMono-Regular.ttf \
  --mono-license resources/fonts/KirkMono-OFL.txt \
  --out "$OUT/skyward-alioth.img"

echo "==> wrote $OUT/skyward-alioth.img"
