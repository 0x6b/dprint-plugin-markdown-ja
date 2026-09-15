#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
: "${ANDROID_NDK_HOME:?Set ANDROID_NDK_HOME to an installed Android NDK (r28 or newer recommended)}"
case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) host=linux-x86_64 ;;
    Darwin-*) host=darwin-x86_64 ;;
    *) echo "Unsupported NDK host" >&2; exit 2 ;;
esac
toolchain="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$host/bin"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$toolchain/aarch64-linux-android21-clang"
export CC_aarch64_linux_android="$CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"
export AR_aarch64_linux_android="$toolchain/llvm-ar"
# Explicit 16 KiB ELF alignment for current Android devices.
export CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384"
rustup target add --toolchain 1.92.0 aarch64-linux-android
cargo build --locked -p dprint-markdown-ja-formatter --profile formatter-release --lib --target aarch64-linux-android --target-dir ../target
stage=../target/aar-stage
rm -rf "$stage"
mkdir -p "$stage/jni/arm64-v8a" "$stage/classes" "$stage/META-INF"
javac --release 8 -d "$stage/classes" java/io/warpnine/markdownja/MarkdownFormatter.java
jar cf "$stage/classes.jar" -C "$stage/classes" .
rm -rf "$stage/classes"
cp ../target/aarch64-linux-android/formatter-release/libdprint_markdown_ja_formatter.so "$stage/jni/arm64-v8a/"
# Strip only the packaging copy with the target-aware NDK tool. Preserve the
# Cargo output for symbolication; do not depend on the consuming app's AGP.
# --strip-unneeded retains dynamic symbols, relocations, and unwind tables.
"$toolchain/llvm-strip" --strip-unneeded "$stage/jni/arm64-v8a/libdprint_markdown_ja_formatter.so"
cp android/AndroidManifest.xml "$stage/"
cp android/consumer-rules.pro "$stage/proguard.txt"
python3 scripts/license-notices.py
cp LICENSE ../target/THIRD_PARTY_NOTICES.md "$stage/META-INF/"
jar cf ../target/dprint-markdown-ja-formatter.aar -C "$stage" .
wc -c ../target/aarch64-linux-android/formatter-release/libdprint_markdown_ja_formatter.so \
    "$stage/jni/arm64-v8a/libdprint_markdown_ja_formatter.so" ../target/dprint-markdown-ja-formatter.aar
echo "Created ../target/dprint-markdown-ja-formatter.aar (arm64-v8a, API 21+)"
