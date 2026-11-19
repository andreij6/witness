#!/usr/bin/env bash
set -euo pipefail

package="$1"
features="${2:-}"
root="$(cd "$(dirname "$0")/.." && pwd)"
target="wasm32-unknown-unknown"
artifact="${package//-/_}"
out_dir="$root/target/canisters"
mkdir -p "$out_dir"

debug_keystore="$HOME/.android/debug.keystore"
if [[ "$features" == *dev-attestation* && -z "${WITNESS_DEBUG_DIGEST:-}" && -f "$debug_keystore" ]] && command -v keytool >/dev/null; then
  WITNESS_DEBUG_DIGEST="$(keytool -list -v -keystore "$debug_keystore" -storepass android 2>/dev/null \
    | awk '/SHA256:/ { gsub(":", "", $2); print $2; exit }')"
  export WITNESS_DEBUG_DIGEST
fi

cargo build --manifest-path "$root/Cargo.toml" --target "$target" --release \
  -p "$package" ${features:+--features "$features"}

wasm="$root/target/$target/release/$artifact.wasm"
did="$out_dir/$artifact.did"
shrunk="$out_dir/$artifact.wasm"

candid-extractor "$wasm" > "$did"
ic-wasm "$wasm" -o "$shrunk" shrink
ic-wasm "$shrunk" -o "$shrunk" metadata candid:service -f "$did" -v public
ic-wasm "$shrunk" -o "$shrunk" metadata witness:package -d "$package${features:+ +$features}" -v public

if [[ -n "${ICP_WASM_OUTPUT_PATH:-}" ]]; then
  cp "$shrunk" "$ICP_WASM_OUTPUT_PATH"
fi
echo "$shrunk"
