#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
tree=$(cargo tree -p witness-core -e normal,features --target wasm32-unknown-unknown --prefix none)
banned=$(grep -E '^(getrandom feature "(js|wasm_js)"|(wasm-bindgen|tokio|reqwest|ort|c2pa) v)' <<<"$tree" | sort -u || true)
if [[ -n "$banned" ]]; then
  echo "banned dependencies in witness-core wasm32 tree:" >&2
  echo "$banned" >&2
  exit 1
fi
echo "witness-core wasm32 dependency tree clean"
