#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
serial="${1:-${ANDROID_SERIAL:-emulator-5554}}"
variant="${WITNESS_ANDROID_VARIANT:-benchmarkRelease}"
package="app.witness"
ids="$root/.icp-local/ids.json"
jni="$root/client/native/witness/src/main/jniLibs"

command -v adb >/dev/null || { echo "adb not found on PATH" >&2; exit 1; }
[[ -f "$ids" ]] || { echo "no .icp-local/ids.json; run scripts/deploy-local.sh" >&2; exit 2; }
[[ "$(adb -s "$serial" get-state 2>/dev/null || true)" == "device" ]] || { echo "device $serial is not connected" >&2; exit 3; }

gateway="$(sed -n 's/.*"gateway"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$ids" | head -1)"
port="${gateway##*:}"
port="${port%%/*}"
[[ "$port" =~ ^[0-9]+$ ]] || port=8090

if ! curl -fsS -o /dev/null --max-time 3 "http://127.0.0.1:$port/api/v2/status"; then
  echo "warning: no replica answering on 127.0.0.1:$port; the app will show it can't reach the network" >&2
fi

adb -s "$serial" reverse "tcp:$port" "tcp:$port" >/dev/null
echo "adb reverse tcp:$port -> tcp:$port on $serial"

if ! ls "$jni"/*/libwitness_android.so >/dev/null 2>&1; then
  "$root/scripts/build-android-native.sh"
fi

task="assemble$(printf %s "${variant:0:1}" | tr "[:lower:]" "[:upper:]")${variant:1}"
(cd "$root/client" && ./gradlew --quiet ":androidApp:$task")
apk="$(ls "$root/client/androidApp/build/outputs/apk/$variant"/*.apk | head -1)"
[[ -f "$apk" ]] || { echo "no apk for $variant" >&2; exit 4; }

local_sum="$(shasum -a 256 "$apk" | cut -d' ' -f1)"
installed="$({ adb -s "$serial" shell pm path "$package" 2>/dev/null || true; } | sed -n 's/^package://p' | tr -d '\r' | head -1)"
remote_sum=""
[[ -z "$installed" ]] || remote_sum="$({ adb -s "$serial" shell sha256sum "$installed" 2>/dev/null || true; } | cut -d' ' -f1)"

if [[ "$remote_sum" == "$local_sum" ]]; then
  echo "$(basename "$apk") already installed"
else
  adb -s "$serial" install -r "$apk" >/dev/null || {
    echo "install failed (see adb's reason above); a differently signed $package must be uninstalled first, which deletes its keys" >&2
    exit 5
  }
  echo "installed $(basename "$apk")"
fi

adb -s "$serial" shell am start -W -n "$package/.MainActivity" >/dev/null
echo "launched $package on $serial"
