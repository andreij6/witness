#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
if icp network ping local >/dev/null 2>&1; then
  icp network stop local
else
  echo "local network is not running"
fi
