#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
dist="$root/client/webApp/build/dist"
default_src="$dist/composeWebCompatibility/productionExecutable"
[[ -f "$default_src/index.html" ]] || default_src="$dist/wasmJs/productionExecutable"
src="${WITNESS_WEB_DIST:-$default_src}"
dest="$root/canisters/web/dist"

if [[ ! -f "$src/index.html" ]]; then
  echo "web: $src has no index.html, serving placeholder" >&2
  src="$root/canisters/web/dist-placeholder"
fi

rm -rf "$dest"
mkdir -p "$dest"
cp -R "$src/." "$dest/"
[[ -f "$dest/_redirects" ]] || cp "$root/canisters/web/_redirects" "$dest/_redirects"
[[ -f "$dest/404.html" ]] || cp "$root/canisters/web/dist-placeholder/404.html" "$dest/404.html"

ids="${WITNESS_IDS:-$root/.icp-local/ids.json}"
api="https://icp-api.io"
canister="https://*.icp0.io"
if [[ "${WITNESS_WEB_ENV:-local}" == "local" ]]; then
  if [[ -f "$ids" && -f "$dest/boot.mjs" ]]; then
    api="$(jq -r '.gateway | sub("/$"; "")' "$ids")"
    canister="$(jq -r '.gateway | sub("/$"; "") | sub("://"; "://*.")' "$ids")"
    jq -r '"export default " + ({
        host: .gateway,
        registry: .canisters.registry.id,
        canister: (.gateway | sub("://"; "://*.")),
        local: true,
        anchor: (.ca_root // "")
      } | tojson) + ";"' "$ids" > "$dest/witness-config.mjs"
    echo "web: local config from $ids (anchor: $(jq -r 'if (.ca_root // "") == "" then "DEV query" else "pinned" end' "$ids"))" >&2
  fi
elif [[ -f "$dest/boot.mjs" ]]; then
  anchor="${WITNESS_CA_ROOT:-}"
  registry="${WITNESS_REGISTRY_ID:-}"
  [[ "$anchor" =~ ^([0-9a-f]{2})+$ ]] || { echo "web: production needs WITNESS_CA_ROOT (lowercase hex DER of the pinned Witness CA root)" >&2; exit 1; }
  [[ -n "$registry" ]] || { echo "web: production needs WITNESS_REGISTRY_ID" >&2; exit 1; }
  jq -n -r --arg host "$api" --arg canister "$canister" --arg registry "$registry" --arg anchor "$anchor" \
    '"export default " + ({host: $host, registry: $registry, canister: $canister, local: false, anchor: $anchor} | tojson) + ";"' \
    > "$dest/witness-config.mjs"
  echo "web: production config with pinned anchor" >&2
fi

cat > "$dest/_headers" <<EOF
/*
  Content-Security-Policy: default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self' $api $canister blob:; img-src 'self' $canister blob: data:; style-src 'self' 'unsafe-inline'; font-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'
  X-Content-Type-Options: nosniff
  Referrer-Policy: no-referrer
EOF

integrity="${WITNESS_WEB_INTEGRITY:-$root/target/web/integrity.json}"
mkdir -p "$(dirname "$integrity")"
if [[ -f "$dest/boot.mjs" ]]; then
  (
    cd "$dest"
    find . -type f \( -name '*.js' -o -name '*.mjs' -o -name '*.wasm' \) | sed 's:^\./::' | LC_ALL=C sort |
      while IFS= read -r path; do
        [[ "$path" =~ ^[A-Za-z0-9._/-]{1,128}$ && "/$path/" != *"/../"* && "/$path/" != *"/./"* ]] ||
          { echo "web: $path cannot be pinned (unsafe name)" >&2; exit 1; }
        printf '%s\t%s\n' "$path" "sha384-$(openssl dgst -sha384 -binary "$path" | openssl base64 -A)"
      done
  ) | jq -R -s 'split("\n") | map(select(length > 0) | split("\t") | {path: .[0], sri: .[1]})' > "$integrity.tmp"
  (($(jq length "$integrity.tmp") <= 256)) || { echo "web: more than 256 code files to pin" >&2; exit 1; }
  mv "$integrity.tmp" "$integrity"
  echo "web: pinned $(jq length "$integrity") code files in $integrity" >&2
else
  echo '[]' > "$integrity"
fi
