#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

if [[ -z "${WITNESS_SMOKE_SKIP_DEPLOY:-}" ]]; then
  scripts/deploy-local.sh
fi

[[ -f "$root/.icp-local/ids.json" ]] || { echo "no .icp-local/ids.json; run scripts/deploy-local.sh" >&2; exit 2; }

cargo run --quiet -p witness-smoke
