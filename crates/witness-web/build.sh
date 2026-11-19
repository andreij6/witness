#!/bin/sh
set -eu
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
cd "$root"
cargo build -p witness-web --target wasm32-unknown-unknown --profile wasm
wasm-bindgen --target web --remove-name-section --remove-producers-section \
  --out-dir "$here/pkg" "$root/target/wasm32-unknown-unknown/wasm/witness_web.wasm"
wasm="$here/pkg/witness_web_bg.wasm"
echo "wasm-bindgen: $(wc -c < "$wasm") bytes"
if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
    --enable-mutable-globals --enable-reference-types --enable-multivalue \
    "$wasm" -o "$wasm.opt"
  mv "$wasm.opt" "$wasm"
  echo "wasm-opt -Oz: $(wc -c < "$wasm") bytes"
fi
