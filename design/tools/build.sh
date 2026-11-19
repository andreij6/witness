#!/bin/zsh
set -e
S=$(cd "$(dirname "$0")" && pwd)
P=$(cd "$S/.." && pwd)
DOC=$P/witness.figcraft
TMP=${TMPDIR:-/tmp}/witness-build; mkdir -p $TMP $TMP/fcout
cd $S
fc() { python3 -I fc.py "$@"; }
launch() {
  pkill -x FigCraft || true; sleep 1.2
  open -a FigCraft --args --control 47901 --automation-read-root $P --automation-write-root $TMP/fcout $DOC
  for i in {1..30}; do sleep 1; fc window.list --max=0 >/dev/null 2>&1 && break; done
  sleep 1.5
  for i in {1..6}; do
    fc window.list --max=400 | grep -q '"active": "witness"' && return 0
    fc ui.key '{"key":"Tab","ctrl":true}' --max=0 >/dev/null; sleep 0.5
  done
  echo "could not activate the witness tab"; fc window.list; return 1
}

if [ ! -f $P/icons/icons.json ]; then launch; python3 -I icons.py $TMP/fcout; fi
python3 -I gen.py $DOC
launch

mkdir -p $P/renders
python3 -I - "$DOC" "${FILTER:-}" <<'PYEOF' > $TMP/frames.txt
import json, sys, zipfile
d = json.loads(zipfile.ZipFile(sys.argv[1]).read("document.json"))
flt = sys.argv[2]
for i in d["nodes"]["1"]["children"]:
    n = d["nodes"][str(i)]
    if n["type"] != "frame" or (flt and not n["name"].startswith(tuple(flt.split(",")))):
        continue
    print(i, n["name"].lower().replace(" · ", "-").replace(" & ", "-").replace(" ", "-"))
PYEOF
while read id name; do
  rm -f $P/renders/$name.png
  fc ui.render "{\"ids\":[$id],\"scale\":2}" --out=$P/renders/$name.png --max=0 >/dev/null
done < $TMP/frames.txt
if [ -z "${FILTER:-}" ]; then
  find $P/renders -name "overview*.png" -delete
  for r in A B C D E W X; do
    IDS=$(python3 -I -c 'import json,sys,zipfile; d=json.loads(zipfile.ZipFile(sys.argv[1]).read("document.json")); n=d["nodes"]; print(json.dumps([i for i in n["1"]["children"] if n[str(i)]["type"]=="frame" and n[str(i)]["name"].startswith(sys.argv[2])]))' $DOC $r)
    [ "$IDS" = "[]" ] || fc ui.render "{\"ids\":$IDS,\"scale\":1}" --out=$P/renders/overview-$(echo $r | tr A-Z a-z).png --max=0 >/dev/null
  done
fi
ls $P/renders | wc -l
