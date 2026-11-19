import os
from pathlib import Path
from google import genai

NAME = "Witness Narrator (deep, warm baritone)"
PROMPT = (
    "An original voice: a mature American man in his early sixties with a deep, warm, velvety bass-baritone, "
    "a little gravel at the bottom of the range. Calm, unhurried, measured documentary-narrator cadence with "
    "reassuring gravitas, like a trusted spokesman you instinctively believe. Gentle warmth and quiet authority, "
    "never salesy. General American accent, crisp consonants, natural pauses."
)
if not os.environ.get("GEMINI_API_KEY"):
    for line in (Path.home() / ".env").read_text().splitlines():
        if line.startswith("GEMINI_API_KEY="):
            os.environ["GEMINI_API_KEY"] = line.split("=", 1)[1].strip().strip("\"'")
client = genai.Client()
for v in client.voices.list(type_=["prompted"], language_code=["en-US"]).voices or []:
    if v.display_name == NAME:
        print(v.id)
        raise SystemExit
v = client.voices.create(store=True, voice={
    "model": "gemini-3.8-flash-tts", "type": "prompted", "display_name": NAME,
    "gender": "male", "language_code": "en-US", "prompted": {"input": PROMPT},
})
print(v.id)
