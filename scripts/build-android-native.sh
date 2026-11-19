#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
features="${1:-}"
api="${WITNESS_ANDROID_API:-26}"
module="$root/client/native/witness/src/main"
jni="$module/jniLibs"
kotlin="$module/kotlin"

if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  sdk="${ANDROID_HOME:-}"
  if [[ -z "$sdk" && -f "$root/client/local.properties" ]]; then
    sdk="$(sed -n 's/^sdk\.dir=//p' "$root/client/local.properties")"
  fi
  ndk="$(ls -d "$sdk"/ndk/* 2>/dev/null | sort -V | tail -1 || true)"
  [[ -n "$ndk" ]] || { echo "set ANDROID_NDK_HOME or ANDROID_HOME" >&2; exit 1; }
  export ANDROID_NDK_HOME="$ndk"
fi

export CARGO_PROFILE_RELEASE_PANIC=unwind

cd "$root"
rm -rf "$jni/arm64-v8a" "$jni/x86_64"
cargo ndk -t arm64-v8a -t x86_64 -P "$api" -o "$jni" \
  build -p witness-android --release ${features:+--features "$features"}

lib="$root/target/aarch64-linux-android/release/libwitness_android.so"
rm -rf "$kotlin/uniffi/witness_android"
cargo run -q -p witness-android --features bindgen --bin uniffi-bindgen -- \
  generate --library "$lib" --language kotlin --out-dir "$kotlin" --no-format

strip_tool="$(ls "$ANDROID_NDK_HOME"/toolchains/llvm/prebuilt/*/bin/llvm-strip | head -1)"
for so in "$jni"/arm64-v8a/libwitness_android.so "$jni"/x86_64/libwitness_android.so; do
  "$strip_tool" --strip-unneeded "$so"
done

ls -l "$jni"/*/libwitness_android.so "$kotlin"/uniffi/witness_android/*.kt
