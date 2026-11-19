from wlib import *

f = D.frame("00 · Foundations", 0, FOUND_Y, FOUND_W, 760, PAPER)
D.rect(f, 40, 40, 132, 132, INK, 30, name="App icon")
mark(f, 106, 108, 104, ring=WHITE, check="#34D399")
D.text(f, 200, 52, "Witness — Foundations", 32, "Bold", INK, ls=-1.5)
D.text(f, 200, 98, "Proof, not just pictures. A calm, paper-toned evidence app: ink for trust, green only when something is proven, teal for declared\n"
                   "changes, violet only where the on-device AI generated the interface. The camera is always dark so the scene is the hero.",
       15, "Regular", TX2, lh=1.5)


def swatches(x, y, title, items):
    overline(f, x, y, title)
    for k, (n, h) in enumerate(items):
        sx = x + k * 118
        D.rect(f, sx, y + 24, 104, 66, h, 14, stroke=LINE)
        D.text(f, sx, y + 98, n, 13, "Semi Bold", INK)
        D.text(f, sx, y + 117, h, 11.5, "Regular", MUTED, font=MONO)


swatches(40, 216, "Surfaces & ink", (("paper", PAPER), ("surface", SURF), ("well", SURF2), ("outline", LINE), ("ink", INK), ("ink-2", TX2),
                                     ("muted · AA", MUTED), ("camera", CAM)))
swatches(1020, 216, "Meaning", (("action", PRIMARY), ("ai · generated", AI), ("proven", OK), ("proven text", OK_T), ("proven · dark", PROVEN_DARK),
                                ("changes", EDIT), ("queued", WARN), ("altered", CRIT)))

overline(f, 2020, 216, "Verdicts · what anyone sees when they check")
for k, (st, desc) in enumerate((("original", "Matches the sealed capture exactly"), ("changes", "Global changes only · reference shown"),
                                ("altered", "Matched, but a region was changed"), ("norecord", "Not proven — which isn't proven fake"))):
    vx, vy = 2020 + (k % 2) * 380, 244 + (k // 2) * 92
    s_, c_, ic_ = STATES[st]
    D.rect(f, vx, vy, 364, 80, c_, 16, a=0.07, stroke=c_, sw=1.2, sa=0.4)
    D.icon(f, ic_, vx + 16, vy + 16, 24, c_)
    D.text(f, vx + 50, vy + 16, s_, 15, "Bold", TINT_TEXT.get(c_, c_))
    D.text(f, vx + 50, vy + 42, desc, 12, "Regular", TX2)

overline(f, 2820, 216, "Evidence states")
for k, st in enumerate(("sealed", "queued", "anchored", "edited", "unverified", "unanchored")):
    state_chip(f, 2820 + (k % 2) * 170, 244 + (k // 2) * 40, st, h=30, size=12.5)
for k, st in enumerate(("sealed", "queued", "anchored", "edited", "altered", "unverified")):
    D.image(f, 2820 + k * 66, 370, 54, 54, ("crash", "bumper", "plate", "wide", "water", "moose")[k], 12)
    seal_badge(f, 2820 + k * 66 + 44, 414, st, 9, ring=PAPER)

overline(f, 3260, 216, "Type · Inter for language, JetBrains Mono for proof")
for k, (n, sz, st, fnt, sample, ls_) in enumerate((("Verdict", 26, "Bold", SANS, "Original", -1), ("Title", 20, "Semi Bold", SANS, "Fender bender", 0),
                                                   ("Card title", 15.5, "Semi Bold", SANS, "Chain of custody", 0), ("Body", 14, "Regular", SANS, "Unaltered since capture", 0),
                                                   ("Label", 12.5, "Medium", SANS, "3 anchored · 1 queued", 0), ("Overline", 11, "Semi Bold", SANS, "STILL RECOMMENDED", 8),
                                                   ("Proof", 12, "Regular", MONO, "sha256 9f3a…c21e", 0))):
    yy = 244 + k * 36
    D.text(f, 3260, yy + 6, f"{n} · {sz:g}", 11, "Regular", MUTED, font=MONO)
    D.text(f, 3410, yy, sample, sz, st, OK_T if n == "Verdict" else INK, font=fnt, ls=ls_)
D.text(f, 3260, 506, "Mockups render proof text in Menlo; the app and website bundle JetBrains Mono and Inter (self-hosted).", 11.5, "Regular", MUTED)

overline(f, 40, 420, "Buttons & chips")
x = 40
for s_, kind in (("Share case", "filled"), ("Capture", "ink"), ("Export", "tonal"), ("Technical record", "outlined")):
    w_ = est_width(s_, 14.5, "Semi Bold") + 48
    button(f, x, 448, w_, 48, s_, kind)
    x += w_ + 12
x += 20
for s_, on in (("All", True), ("Posts", False)):
    if on:
        _, w_ = chip(f, x, 455, s_, WHITE, bg=INK, h=34, size=13)
    else:
        _, w_ = chip(f, x, 455, s_, TX2, h=34, size=13, style="Medium", outline=True)
    x += w_ + 8
x += 20
_, w_ = chip(f, x, 455, "Blur faces · 1", EDIT, h=34, size=12.5, icon="face")
x += w_ + 28
toggle(f, x, 459, True)
toggle(f, x + 58, 459, False)

overline(f, 40, 536, "Generated surfaces (A2UI)")
generated_surface(f, 40, 560, 460, 150)
ai_header(f, 58, 576, "Still recommended", 442)
D.icon(f, "check_circle", 58, 602, 20, OK)
D.text(f, 86, 603, "Damage close-up", 13.5, "Medium", TX2)
D.icon(f, "add", 58, 630, 20, PRIMARY)
D.text(f, 86, 631, "Other driver's insurance card", 13.5, "Semi Bold", INK)
generated_tag(f, 58, 668, "Read from photo 3")
D.text(f, 530, 560, "Anything the on-device model composes through A2UI sits on a generated surface: a hairline violet border, a violet\n"
                   "rail, an ✦ tag and a ⚑ flag. Users always know which parts the AI built, and can edit, remove or flag them.\n\n"
                   "Report screens are A2UI documents built from Witness's own Compose catalog — fields, pickers, the car diagram,\n"
                   "the checklist — so new situations ship as data, generated screens look native, and nothing can run arbitrary code.",
       13.5, "Regular", TX2, lh=1.55)

overline(f, 2020, 470, "Icons · Material Symbols")
for k, n in enumerate(("photo_camera", "lock", "verified", "satellite_alt", "auto_awesome", "key", "verified_user", "share", "link", "face",
                       "compare", "flag", "report", "workspace_premium", "help", "error")):
    D.icon(f, n, 2020 + (k % 8) * 48, 498 + (k // 8) * 44, 26, TX2)

overline(f, 2460, 470, "App icon")
for k, sz in enumerate((96, 64, 48)):
    ix = 2460 + sum(v + 28 for v in (96, 64, 48)[:k])
    D.rect(f, ix, 494 + (96 - sz), sz, sz, INK, sz * 0.23)
    mark(f, ix + sz / 2, 494 + (96 - sz) + sz / 2, sz * 0.8, ring=WHITE, check="#34D399")
ix = 2460 + 96 + 64 + 48 + 3 * 28
D.ellipse(f, ix + 32, 494 + 64, 32, "#DDE3FF")
mark(f, ix + 32, 494 + 64, 52, ring="#1E2A8A", check="#1E2A8A")
D.text(f, 2460, 602, "Adaptive · legacy · themed", 12, "Regular", MUTED)

overline(f, 2820, 470, "Camera modes")
D.rect(f, 2820, 494, 300, 64, CAM, 16)
mw = 148
for k, s_ in enumerate(("Post", "Case")):
    on = k == 0
    if on:
        D.rect(f, 2820 + 76 + 3, 494 + 17, mw / 2 - 3, 30, WHITE, 15)
    D.label(f, 2820 + 76 + k * mw / 2, 508, mw / 2, 36, s_, 13.5, "Semi Bold", INK if on else WHITE)
D.text(f, 2820, 572, "Post is the default: seal, blur, caption, share.\nCase captures into the active case, shown and switchable on the camera.", 12.5, "Regular", TX2, lh=1.5)

D.text(f, 3260, 560, "Principles", 15.5, "Semi Bold", INK)
for k, s_ in enumerate(("Green means proven — never decoration.", "Every claim shows its evidence one tap away.",
                        "Say what we don't prove as clearly as what we do.", "Camera first: two taps from launch to a sealed photo.",
                        "AI suggests, the person decides — and can see which is which.")):
    D.text(f, 3260, 590 + k * 26, f"{k + 1}.  {s_}", 13.5, "Regular", TX2)
