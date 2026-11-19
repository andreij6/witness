#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

canisters=(registry index_0 pages media_0 web)
out_dir="$root/.icp-local"
ids_file="$out_dir/ids.json"

if ! icp network ping local >/dev/null 2>&1; then
  echo "starting local network"
  icp network start -d
fi

id_of() {
  icp canister status "$1" --id-only 2>/dev/null | tr -d '[:space:]' || true
}

for c in "${canisters[@]}"; do
  if [[ -z "$(id_of "$c")" ]]; then
    icp canister create "$c" --quiet >/dev/null
  fi
done

ids=()
for c in "${canisters[@]}"; do
  id="$(id_of "$c")"
  [[ -n "$id" ]] || { echo "no canister id for $c" >&2; exit 1; }
  ids+=("$id")
done
registry_id="${ids[0]}" index_id="${ids[1]}" pages_id="${ids[2]}" media_id="${ids[3]}" web_id="${ids[4]}"

gateway="$(icp network status local --json | jq -r '.gateway_url' | sed 's:/*$::')"
scheme="${gateway%%://*}"
hostport="${gateway#*://}"
port="${hostport##*:}"
origin_of() {
  echo "$scheme://$1.localhost:$port"
}

registry_args="(record { index = vec { principal \"$index_id\" }; media = vec { principal \"$media_id\" }; pages = principal \"$pages_id\" })"
origins="opt record { site = \"$(origin_of "$pages_id")\"; case = \"$(origin_of "$pages_id")\"; app = \"$(origin_of "$web_id")\"; api = \"$gateway\"; canister = \"$(origin_of '*')\" }"
peer_args() {
  echo "(record { registry = principal \"$registry_id\"; shard = $1 : nat8; origins = $2 })"
}

ca_root_hex() {
  local prefix="4449444c026e016d7b010001" hex len=0 shift=0 byte
  hex="$(icp canister call registry ca_root '()' --query -o hex 2>/dev/null | tr -d '[:space:]')" || return 0
  [[ "$hex" == "$prefix"* ]] || return 0
  hex="${hex#"$prefix"}"
  while [[ -n "$hex" ]]; do
    byte=$((16#${hex:0:2}))
    hex="${hex:2}"
    len=$((len | (byte & 0x7f) << shift))
    shift=$((shift + 7))
    ((byte & 0x80)) || break
  done
  if ((${#hex} == 2 * len)); then echo "$hex"; fi
}

icp deploy registry --yes --args "$registry_args"
icp deploy index_0 --yes --args "$(peer_args 0 null)"
icp deploy media_0 --yes --args "$(peer_args 0 "$origins")"

ca_root=""
for _ in $(seq 1 30); do
  ca_root="$(ca_root_hex)"
  [[ -n "$ca_root" ]] && break
  sleep 1
done
[[ -n "$ca_root" ]] || echo "warning: registry has no CA root yet; clients fall back to the DEV query" >&2

mkdir -p "$out_dir"
{
  printf '{\n  "network": "local",\n  "gateway": "%s",\n  "ca_root": "%s",\n  "canisters": {\n' "$gateway" "$ca_root"
  for i in "${!canisters[@]}"; do
    sep=","; [[ $i -eq $((${#canisters[@]} - 1)) ]] && sep=""
    printf '    "%s": { "id": "%s", "url": "%s://%s.localhost:%s/" }%s\n' "${canisters[$i]}" "${ids[$i]}" "$scheme" "${ids[$i]}" "$port" "$sep"
  done
  printf '  }\n}\n'
} > "$ids_file"

integrity="$root/target/web/integrity.json"
WITNESS_WEB_INTEGRITY="$integrity" scripts/stage-web.sh
pins="$(jq -r 'if length == 0 then "null" else "opt vec { " + (map("record { path = \"" + .path + "\"; sri = \"" + .sri + "\" }") | join("; ")) + " }" end' "$integrity")"
staged="$(shasum -a 256 "$integrity")"
icp deploy pages --yes --args "$(peer_args 0 "$origins" | sed 's/)$//'), $pins)"

WITNESS_WEB_INTEGRITY="$integrity" icp deploy web --yes
[[ "$(shasum -a 256 "$integrity")" == "$staged" ]] || { echo "web build changed after pages pinned it; rerun scripts/deploy-local.sh" >&2; exit 1; }
boot_sri="$(jq -r '.[] | select(.path == "boot.mjs") | .sri' "$integrity")"
if [[ -n "$boot_sri" ]]; then
  served="sha384-$(curl -fsS --max-time 30 "$(origin_of "$web_id")/boot.mjs" | openssl dgst -sha384 -binary | openssl base64 -A)"
  [[ "$served" == "$boot_sri" ]] || { echo "web serves a boot.mjs that pages did not pin" >&2; exit 1; }
  curl -fsS --max-time 30 "$(origin_of "$pages_id")/v/pin-check" | grep -qF "integrity=\"$boot_sri\"" ||
    { echo "pages case shell does not pin boot.mjs" >&2; exit 1; }
  echo "case shell pins boot.mjs and $(($(jq length "$integrity") - 1)) more code files"
fi

jq . "$ids_file" >/dev/null
echo
for i in "${!canisters[@]}"; do
  printf '%-9s %s  %s://%s.localhost:%s/\n' "${canisters[$i]}" "${ids[$i]}" "$scheme" "${ids[$i]}" "$port"
done
echo "ids written to $ids_file"
