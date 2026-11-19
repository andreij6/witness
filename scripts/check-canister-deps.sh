#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
target="wasm32-unknown-unknown"
pattern='^(getrandom feature "(js|wasm_js)"|(wasm-bindgen|tokio|reqwest|ort|c2pa) v)'
status=0
for manifest in canisters/*/Cargo.toml; do
  package="$(sed -n 's/^name = "\(.*\)"/\1/p' "$manifest" | head -1)"
  tree="$(cargo tree -p "$package" -e normal,features --all-features --target "$target" --prefix none)"
  banned="$(grep -E "$pattern" <<<"$tree" | sort -u || true)"
  if [[ -n "$banned" ]]; then
    echo "banned dependencies in $package $target tree:" >&2
    echo "$banned" >&2
    status=1
  else
    echo "$package $target dependency tree clean"
  fi
done
exit "$status"
