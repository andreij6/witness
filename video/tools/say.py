#!/usr/bin/env python3
"""One narration line -> a WAV, with the story narrator voice (Gemini TTS, Interactions REST API).

Run: python3 tools/video/tts/say.py --voice VOICE_ID --style "..." --text "..." --out file.wav
GEMINI_API_KEY comes from the env or ~/.env. Standard library only, one request, no silent retries:
exit 2 on a per-minute limit or other error (the caller retries), exit 3 when the daily quota is spent.
Tier 1 limits for gemini-3.8-flash-tts (2026-11-10): 10 requests a minute, 100 a day.
"""
import argparse, base64, json, os, sys, urllib.error, urllib.request
from pathlib import Path

p = argparse.ArgumentParser()
p.add_argument("--voice", required=True)
p.add_argument("--style", default="")
p.add_argument("--text", required=True)
p.add_argument("--out", required=True)
p.add_argument("--model", default="gemini-3.8-flash-tts")
a = p.parse_args()

key = os.environ.get("GEMINI_API_KEY")
if not key:
    for line in (Path.home() / ".env").read_text().splitlines():
        if line.startswith("GEMINI_API_KEY="):
            key = line.split("=", 1)[1].strip().strip("\"'")
item = {"type": "text", "text": a.text}
if a.style:
    item["annotations"] = [{"type": "speech_metadata", "style": a.style}]
body = {"model": a.model, "input": [{"type": "user_input", "content": [item]}], "response_format": {"type": "audio"},
        "generation_config": {"speech_config": [{"voice": a.voice}]}}
req = urllib.request.Request("https://generativelanguage.googleapis.com/v1beta/interactions", json.dumps(body).encode(),
                             {"Content-Type": "application/json", "x-goog-api-key": key})
try:
    with urllib.request.urlopen(req, timeout=150) as r:
        d = json.load(r)
except urllib.error.HTTPError as e:
    msg = e.read().decode(errors="replace")[:400]
    print(f"tts {e.code}: {msg}", file=sys.stderr)
    sys.exit(3 if e.code == 429 and "per day" in msg else 2)
except Exception as e:
    print(f"tts failed: {type(e).__name__}: {e}", file=sys.stderr)
    sys.exit(2)
if "error" in d:
    msg = json.dumps(d["error"])[:400]
    print(f"tts error: {msg}", file=sys.stderr)
    sys.exit(3 if "per day" in msg else 2)
audio = next((c.get("data") for st in d.get("steps", []) for c in st.get("content", []) if c.get("type") == "audio"), None)
if not audio:
    print(f"tts: no audio in response keys {list(d)}", file=sys.stderr)
    sys.exit(2)
out = Path(a.out)
out.parent.mkdir(parents=True, exist_ok=True)
tmp = out.with_suffix(".part")
tmp.write_bytes(base64.b64decode(audio))
tmp.rename(out)
