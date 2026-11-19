#!/usr/bin/env bash
set -euo pipefail

src="$(cd "$(dirname "$0")" && pwd)/android-local.sh"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

run() {
  local installed="$1" root="$work/$2"
  mkdir -p "$root/scripts" "$root/.icp-local" "$root/bin" "$root/client/androidApp/build/outputs/apk/benchmarkRelease" "$root/client/native/witness/src/main/jniLibs/x86_64"
  cp "$src" "$root/scripts/"
  echo '{"gateway": "http://localhost:8090"}' > "$root/.icp-local/ids.json"
  touch "$root/client/native/witness/src/main/jniLibs/x86_64/libwitness_android.so"
  printf apk > "$root/client/androidApp/build/outputs/apk/benchmarkRelease/app.apk"
  printf '#!/usr/bin/env bash\nexit 0\n' > "$root/client/gradlew"
  printf '#!/usr/bin/env bash\nexit 0\n' > "$root/bin/curl"
  cat > "$root/bin/adb" <<STUB
#!/usr/bin/env bash
echo "\$*" >> "$root/adb.log"
case "\$*" in
  *get-state*) echo device ;;
  *"pm path"*) if [[ "$installed" == yes ]]; then echo package:/data/app/base.apk; else exit 1; fi ;;
  *sha256sum*) echo "deadbeef  /data/app/base.apk" ;;
esac
STUB
  chmod +x "$root/client/gradlew" "$root/bin/curl" "$root/bin/adb"
  PATH="$root/bin:$PATH" bash "$root/scripts/android-local.sh" emulator-5554 > "$root/out.log" 2>&1
}

run no fresh || { cat "$work/fresh/out.log"; echo "FAIL: fresh install aborted"; exit 1; }
grep -q "install -r" "$work/fresh/adb.log" || { echo "FAIL: fresh install never installed"; exit 1; }
grep -q "am start" "$work/fresh/adb.log" || { echo "FAIL: fresh install never launched"; exit 1; }

run yes upgrade || { cat "$work/upgrade/out.log"; echo "FAIL: upgrade aborted"; exit 1; }
grep -q "install -r" "$work/upgrade/adb.log" || { echo "FAIL: changed apk was not reinstalled"; exit 1; }

echo "android-local.sh: fresh install and upgrade OK"
