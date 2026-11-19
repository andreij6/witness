#!/bin/sh
set -eu
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../../.." && pwd)
"$root/crates/witness-web/build.sh"
mkdir -p "$here/wasm"
cp "$root/crates/witness-web/pkg/witness_web.js" "$root/crates/witness-web/pkg/witness_web_bg.wasm" "$root/crates/witness-web/pkg/witness_web.d.ts" "$here/wasm/"
