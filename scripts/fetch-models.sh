#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
out="${WITNESS_MODELS_DIR:-$root/models}"
base="${WITNESS_MODELS_URL:-https://cc-assets.netlify.app/watermarking/trustmark-models}"
encoder_sha="${WITNESS_ENCODER_Q_SHA256:-TODO}"
decoder_sha="${WITNESS_DECODER_Q_SHA256:-TODO}"

usage() {
  cat >&2 <<USAGE
usage: scripts/fetch-models.sh --download

Downloads the TrustMark Q models into $out and verifies pinned SHA-256 digests.
Nothing is downloaded unless --download is passed.

TODO (owner): pin the digests before first use. Obtain encoder_Q.onnx and decoder_Q.onnx
from the official TrustMark release, verify them out of band, then either set
WITNESS_ENCODER_Q_SHA256 and WITNESS_DECODER_Q_SHA256 or replace the TODO defaults in
this script. Override the source with WITNESS_MODELS_URL if the release location differs.
USAGE
}

[[ "${1:-}" == "--download" ]] || { usage; exit 2; }

for digest in "$encoder_sha" "$decoder_sha"; do
  if [[ ! "$digest" =~ ^[0-9a-f]{64}$ ]]; then
    echo "model digests are not pinned yet" >&2
    usage
    exit 3
  fi
done

mkdir -p "$out"
fetch() {
  local name="$1" want="$2" tmp
  tmp="$(mktemp "$out/.$name.XXXXXX")"
  trap 'rm -f "$tmp"' RETURN
  curl --fail --location --proto '=https' --tlsv1.2 --silent --show-error -o "$tmp" "$base/$name"
  local got
  got="$(shasum -a 256 "$tmp" | cut -d' ' -f1)"
  if [[ "$got" != "$want" ]]; then
    echo "$name: sha256 $got does not match pinned $want" >&2
    return 1
  fi
  mv "$tmp" "$out/$name"
  echo "$out/$name verified"
}

fetch encoder_Q.onnx "$encoder_sha"
fetch decoder_Q.onnx "$decoder_sha"
