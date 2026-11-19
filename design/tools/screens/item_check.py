from wlib import *

f = screen(S_ITEM)
D.image(f, 0, 0, W, 330, "plate")
scrim(f, 0, 130, 0.42)
status_bar(f, WHITE)
bk = D.ellipse(f, M + 20, 64, 20, "#000000", a=0.42)
D.icon(f, "arrow_back", M + 8, 52, 24, WHITE)
goto(bk, S_CASE)
mv = D.ellipse(f, W - M - 20, 64, 20, "#000000", a=0.42)
D.icon(f, "more_vert", W - M - 32, 52, 24, WHITE)
goto(mv, S["D2"])
chip(f, W / 2 - 40, 272, "Photo 3 of 4", WHITE, bg="#000000", h=26, size=12)
SY = 306
D.rect(f, 0, SY, W, H - SY, PAPER, radii=[26, 26, 0, 0], name="Proof sheet")
D.rect(f, W / 2 - 18, SY + 8, 36, 4, MUTED, 2, a=0.5)
D.ellipse(f, M + 28, SY + 50, 28, OK, a=0.12)
D.icon(f, "verified", M + 12, SY + 34, 32, OK)
D.text(f, M + 68, SY + 26, "Sealed & anchored", 20, "Semi Bold", INK)
D.text(f, M + 68, SY + 53, "Unaltered since capture", 13, "Regular", TX2)
chip(f, M + 68, SY + 76, "In case: Fender bender", TX2, h=24, size=11, icon="folder", outline=True)

D.text(f, M, SY + 118, "Checks & chain of custody", 15.5, "Semi Bold", INK)
card(f, M, SY + 146, W - 2 * M, 330)
EV = (("photo_camera", "Captured", "2:31:52 PM · Pixel 9 Pro · Witness 1.0", None),
      ("key", "Key attested", "StrongBox · Google root · not revoked", "Strong"),
      ("satellite_alt", "Location", "Raw satellites agree · ±6 m", "Strong"),
      ("schedule", "Time", "GNSS-signed · clock within 1 s", "Strong"),
      ("verified", "Anchored", "2:36:10 PM · Internet Computer", None))
D.path(f, [(M + 34, SY + 180), (M + 34, SY + 430)], LINE, 2)
for k, (ic, t, sub, strength) in enumerate(EV):
    ey = SY + 162 + k * 62
    D.ellipse(f, M + 34, ey + 18, 18, OK if strength else SURF2, a=0.12 if strength else 1.0)
    D.icon(f, ic, M + 24, ey + 8, 20, OK if strength else INK)
    D.text(f, M + 64, ey + 2, t, 14.5, "Semi Bold", INK)
    D.text(f, M + 64, ey + 23, sub, 12, "Regular", TX2)
    if strength:
        chip(f, W - M - 16, ey + 2, strength, OK, h=24, size=11, right=True)
tr = button(f, M, 846, 168, 50, "Technical record", "outlined", icon="description")
goto(tr, S["D3"])
sp = button(f, M + 180, 846, W - 2 * M - 180, 50, "Share", "filled", icon="share")
goto(sp, S_SHARE)
gesture(f)

f = screen(S_CHECKIN)
app_bar(f, "Check", "Is this photo real?", back=False, right_icon=None)
D.rect(f, M, 104, W - 2 * M, 150, SURF, 20, stroke=MUTED, sw=1.2, sa=0.6, dashes=[6, 5])
D.icon(f, "photo_library", W / 2 - 16, 126, 32, TX2)
D.text(f, 0, 168, "Choose a photo or screenshot", 15, "Semi Bold", INK, w=W, align="center")
D.text(f, 0, 192, "Checked on your phone — nothing is uploaded", 12.5, "Regular", TX2, w=W, align="center")
chp = button(f, W / 2 - 80, 214, 160, 30, "Choose photo", "ink", size=13)
goto(chp, S["D6"])
D.rect(f, M, 268, W - 2 * M, 48, SURF2, 24)
D.icon(f, "link", M + 16, 280, 22, TX2)
D.text(f, M + 48, 282, "Paste a post or proof link", 14, "Regular", MUTED)
D.text(f, M, 283, "Paste", 13, "Semi Bold", PRIMARY, w=W - 2 * M - 18, align="right")
D.icon(f, "share", M, 330, 16, MUTED)
D.text(f, M + 22, 329, "Tip: in any app, use Share → Witness", 12, "Regular", MUTED)
D.text(f, M, 370, "Recent checks", 16, "Semi Bold", INK)
for k, (img, t, meta, st) in enumerate((("hail", "@stormwatch.den screenshot", "2 min ago · cropped, recompressed", "changes"),
                                        ("random", "Lighthouse wave", "Yesterday · from a group chat", "norecord"),
                                        ("plate", "Other driver's plate", "Nov 18 · case link", "original"))):
    ry = 400 + k * 84
    rc = card(f, M, ry, W - 2 * M, 74, r=16)
    D.image(f, M + 10, ry + 10, 54, 54, img, 10)
    if img == "hail":
        redact_hail(f, M + 10, ry + 10, 54, 54)
    D.text(f, M + 78, ry + 12, t, 14, "Semi Bold", INK)
    D.text(f, M + 78, ry + 33, meta, 12, "Regular", TX2)
    s__, c_, ic_ = STATES[st]
    short = {"changes": "With changes", "norecord": "No record", "original": "Original"}[st]
    chip(f, M + 78, ry + 50 - 2, short, c_, h=22, size=10.5, icon=ic_)
    goto(rc, {"changes": S_CHECK, "norecord": S_NORECORD, "original": S_CASELINK}[st])
nav_bar(f, "Check")

f = screen(S_CHECK)
app_bar(f, "Check", "Shared from another app", right_icon=None, lead="close")
VY_ = 100
D.rect(f, M, VY_, W - 2 * M, 112, EDIT, 20, a=0.07, stroke=EDIT, sw=1.2, sa=0.4, name="Verdict")
D.icon(f, "compare", M + 16, VY_ + 16, 30, EDIT)
D.text(f, M + 56, VY_ + 14, "Matches an original", 21, "Bold", EDIT, ls=-0.5)
D.text(f, M + 56, VY_ + 41, "with changes", 14, "Semi Bold", EDIT)
D.text(f, M + 16, VY_ + 70, "This screenshot is a copy of a sealed capture.\nNo local edits found in the regions we could compare.", 13, "Regular", INK, lh=1.35)

cy_ = 224
card(f, M, cy_, W - 2 * M, 342)
tw_ = (W - 2 * M - 44) / 2
lx, rx = M + 16, M + 28 + tw_
overline(f, lx, cy_ + 14, "Your screenshot")
overline(f, rx, cy_ + 14, "Sealed reference")
sy0 = cy_ + 38
D.rect(f, lx, sy0, tw_, 274, SURF, 12, stroke=LINE)
D.ellipse(f, lx + 16, sy0 + 16, 8, SURF3)
D.text(f, lx + 30, sy0 + 9, "@stormwatch.den", 10.5, "Semi Bold", INK)
D.icon(f, "more_vert", lx + tw_ - 20, sy0 + 8, 16, TX2)
D.image(f, lx, sy0 + 32, tw_, tw_, "hail")
redact_hail(f, lx, sy0 + 32, tw_, tw_)
for k, ic in enumerate(("favorite", "chat_bubble", "send")):
    D.icon(f, ic, lx + 8 + k * 24, sy0 + 38 + tw_, 16, INK)
D.text(f, lx + 8, sy0 + 60 + tw_, "1.9k likes", 10.5, "Semi Bold", INK)
D.text(f, lx + 8, sy0 + 76 + tw_, "is this real??", 10.5, "Regular", TX2)
oh = 274
D.image(f, rx, sy0, tw_, oh, "hail", 12)
redact_hail(f, rx, sy0, tw_, oh)
cy0 = sy0 + (oh - tw_) / 2
D.rect(f, rx, cy0, tw_, tw_, None, 2, stroke=EDIT, sw=2, dashes=[6, 4])
chip(f, rx + 6, cy0 + tw_ - 28, "Crop", WHITE, bg=EDIT, h=22, size=10.5, icon="compare")
D.text(f, rx, sy0 + oh + 8, "With 2 declared blurs", 11.5, "Medium", TX2)
D.text(f, lx, sy0 + oh + 8, "Tap either to compare", 11.5, "Medium", PRIMARY)

dy = 578
card(f, M, dy, W - 2 * M, 176)
overline(f, M + 16, dy + 14, "What changed")
for k, (ic, c, t, s_) in enumerate((("compare", EDIT, "Cropped to a square", "About 25% of the frame removed"),
                                    ("image", EDIT, "Recompressed by the platform", "62 of 64 regions match · 2 outside the crop"),
                                    ("edit", MUTED, "2 blurs declared by the photographer", "House number and a plate"))):
    ry = dy + 38 + k * 44
    D.icon(f, ic, M + 16, ry + 4, 20, c)
    D.text(f, M + 46, ry, t, 13.5, "Semi Bold", INK)
    D.text(f, M + 46, ry + 19, s_, 12, "Regular", TX2)
D.text(f, M + 4, 766, "Captured Nov 18, 4:12 PM · Denver area · genuine Android phone", 12, "Regular", TX2)
D.icon(f, "trending_up", M + 4, 788, 16, OK)
D.text(f, M + 26, 788, "First seen here · checked 1,284 times", 12, "Semi Bold", OK_T)
D.rect(f, 0, 834, W, 81, SURF)
D.path(f, [(0, 834), (W, 834)], LINE, 1)
button(f, M, 848, 140, 48, "Share result", "tonal", icon="share")
op = button(f, M + 152, 848, W - 2 * M - 152, 48, "Open proof page", "filled", icon="open_in_new")
goto(op, S_PROOFPAGE)
gesture(f)

f = screen(S["D2"])
D.image(f, 0, 0, W, 330, "plate")
scrim(f, 0, 130, 0.42)
status_bar(f, WHITE)
D.ellipse(f, M + 20, 64, 20, "#000000", a=0.42)
D.icon(f, "arrow_back", M + 8, 52, 24, WHITE)
D.ellipse(f, W - M - 20, 64, 20, "#000000", a=0.42)
D.icon(f, "more_vert", W - M - 32, 52, 24, WHITE)
D.rect(f, 0, 306, W, H - 306, PAPER, radii=[26, 26, 0, 0])
D.ellipse(f, M + 28, 356, 28, OK, a=0.12)
D.icon(f, "verified", M + 12, 340, 32, OK)
D.text(f, M + 68, 332, "Sealed & anchored", 20, "Semi Bold", INK)
D.text(f, M + 68, 359, "Unaltered since capture", 13, "Regular", TX2)
card(f, M, 452, W - 2 * M, 330)
dim = D.rect(f, 0, 0, W, H, "#000000", a=0.45, name="Dim")
goto(dim, S_ITEM)
TOP = 395
D.rect(f, 0, TOP, W, H - TOP, SURF, radii=[26, 26, 0, 0], name="Sheet")
D.rect(f, W / 2 - 18, TOP + 8, 36, 4, MUTED, 2, a=0.5)
D.text(f, M + 8, TOP + 28, "Photo 3 of 4", 16, "Semi Bold", INK)
D.text(f, M + 8, TOP + 50, "Fender bender · captured 2:31 PM", 12.5, "Regular", TX2)
D.path(f, [(M, TOP + 80), (W - M, TOP + 80)], LINE, 1)
ACTIONS = (("folder", "Move to case", "Now in Fender bender · 4 items", INK, S["C2"]),
           ("public", "Create post", "Share a redacted copy with a public proof page", INK, S_COMPOSER),
           ("pin_drop", "Prove exact location", "A private file that shows where you stood", INK, S["D4"]),
           ("data_object", "Technical record", "Hashes, key, certificate and anchor", INK, S["D3"]),
           ("delete", "Delete from this phone", "Anchored proof stays — it holds no image", CRIT, None),
           ("hide_image", "Erase public copies", "Posted images come down; links show “Removed”", CRIT, None))
for k, (ic, t, sub, c, dest) in enumerate(ACTIONS):
    ry = TOP + 88 + k * 64 + (16 if k >= 4 else 0)
    hit = D.rect(f, 0, ry, W, 64, None, name="Row " + t)
    D.icon(f, ic, M + 8, ry + 20, 24, c)
    D.text(f, M + 52, ry + 12, t, 15, "Semi Bold", c)
    D.text(f, M + 52, ry + 34, sub, 12.5, "Regular", TX2)
    if dest:
        D.icon(f, "chevron_right", W - M - 24, ry + 20, 24, MUTED)
        if dest in {n for n, _ in D.frames} or not dest.startswith("C"):
            goto(hit, dest)
D.path(f, [(M, TOP + 352), (W - M, TOP + 352)], LINE, 1)
gesture(f)

f = screen(S["D3"])
app_bar(f, "Technical record", "Photo 3 · Fender bender", right_icon=None)
bk = D.rect(f, M - 8, 42, 40, 40, None, name="Back")
goto(bk, S_ITEM)
FIELDS = (("record_id", "H4IL7QK2M9XRT3VA"), ("capture_id", "0x1F3A…9C2"), ("asset_hash", "sha256 9f3a…c21e"),
          ("pixel_hash", "sha256 41b0…77de"), ("key_id", "7c1e…a90f · StrongBox · RKP"), ("Witness cert serial", "03:A1…"),
          ("anchored", "2026-11-18 14:36:10 UTC"), ("MMR leaf", "2,108,733 of 2,114,002"))
RH = 64
card(f, M, 104, W - 2 * M, len(FIELDS) * RH + 8)
for k, (key, val) in enumerate(FIELDS):
    ry = 108 + k * RH
    if k:
        D.path(f, [(M + 16, ry), (W - M - 16, ry)], LINE, 1)
    D.text(f, M + 16, ry + 13, key, 11.5, "Regular", MUTED, font=MONO)
    D.text(f, M + 16, ry + 33, val, 12.5, "Medium", INK, w=W - 2 * M - 72, align="left", font=MONO)
    D.icon(f, "content_copy", W - M - 34, ry + 23, 18, MUTED)
vy = 104 + len(FIELDS) * RH + 8 + 16
D.rect(f, M, vy, W - 2 * M, 64, OK, 16, a=0.08, stroke=OK, sw=1.0, sa=0.35, name="Certificate verified")
D.icon(f, "verified", M + 16, vy + 20, 24, OK)
D.text(f, M + 52, vy + 12, "Certificate verified", 14.5, "Semi Bold", OK_T)
D.text(f, M + 52, vy + 33, "Witness CA chain · MMR proof matches the root", 12, "Regular", TX2)
overline(f, M + 4, vy + 88, "Check it yourself")
D.text(f, M + 4, vy + 110, "Anyone can recompute asset_hash from the file and check the MMR proof against the root the registry certifies. The web page does this in the browser.",
       12.5, "Regular", TX2, w=W - 2 * M - 8, align="left", lh=1.4)
button(f, M, 846, 150, 50, "Copy JSON", "tonal", icon="content_copy")
ow = button(f, M + 162, 846, W - 2 * M - 162, 50, "Open on the web", "outlined", icon="open_in_new")
goto(ow, S_PROOFPAGE)
gesture(f)

f = screen(S["D4"])
app_bar(f, "Prove exact location", "Photo 3 · Fender bender", right_icon=None)
bk = D.rect(f, M - 8, 42, 40, 40, None, name="Back")
goto(bk, S_ITEM)
MW_, MH_ = W - 2 * M, 306
mp = D.clip(f, M, 104, MW_, MH_, fill="#ECE9E2", r=18, stroke=LINE, name="Map")
for gx in range(0, int(MW_), 34):
    D.path(mp, [(gx, 0), (gx, MH_)], LINE, 1)
for gy in range(0, MH_, 34):
    D.path(mp, [(0, gy), (MW_, gy)], LINE, 1)
for bx, by, bw, bh in ((14, 18, 96, 64), (14, 150, 96, MH_ - 168), (MW_ - 128, 18, 112, 64), (MW_ - 128, 150, 112, MH_ - 168)):
    D.rect(mp, bx, by, bw, bh, SURF3, 6)
D.rect(mp, 0, 100, MW_, 32, SURF)
D.rect(mp, MW_ / 2 - 70, 0, 32, MH_, SURF)
D.path(mp, [(0, MH_ - 10), (MW_, 30)], SURF, 10, a=0.9)
D.text(mp, MW_ - 112, 108, "Oak St", 11, "Medium", MUTED)
D.text(mp, MW_ / 2 - 64, 30, "5th Ave", 11, "Medium", MUTED)
px_, py_ = MW_ / 2 - 54, 116
D.ellipse(mp, px_, py_, 30, INK, a=0.08, stroke=INK, sw=1.2, sa=0.35)
D.ellipse(mp, px_, py_ + 2, 5, INK, a=0.25)
D.icon(mp, "location_on", px_ - 18, py_ - 34, 36, INK)
coord = "39.7392° N, 104.9903° W · ±6 m"
cw_ = est_width(coord, 12, "Medium", MONO) + 24
D.rect(mp, (MW_ - cw_) / 2, MH_ - 44, cw_, 30, SURF, 15, stroke=LINE)
D.label(mp, (MW_ - cw_) / 2, MH_ - 44, cw_, 30, coord, 12, "Medium", INK, MONO)
wy = 104 + MH_ + 16
D.rect(f, M, wy, W - 2 * M, 64, WARN, 16, a=0.08, stroke=WARN, sw=1.0, sa=0.35, name="Warning")
D.icon(f, "warning", M + 16, wy + 20, 24, WARN)
D.text(f, M + 52, wy + 13, "Anyone with this file can see exactly where you stood.", 13.5, "Semi Bold", WARN_T,
       w=W - 2 * M - 68, align="left", lh=1.35)
ey = wy + 80
card(f, M, ey, W - 2 * M, 236)
for k, (ic, t, sub) in enumerate((("description", "What's created", "A signed opening of the sealed location — a small file you send to one person."),
                                  ("verified_user", "Who can verify", "Anyone you give it to, on witness.app. It must match what was sealed at capture."),
                                  ("visibility_off", "Still never public", "Posts and proof pages keep showing only the area."))):
    ry = ey + 20 + k * 80
    D.icon(f, ic, M + 16, ry + 2, 22, INK)
    D.text(f, M + 52, ry, t, 14, "Semi Bold", INK)
    D.text(f, M + 52, ry + 21, sub, 12.5, "Regular", TX2, w=W - 2 * M - 68, align="left", lh=1.35)
cp = button(f, M, 794, W - 2 * M, 50, "Create location proof", "filled", icon="pin_drop")
goto(cp, S_ITEM)
cn = button(f, M, 852, W - 2 * M, 40, "Cancel", "outlined")
goto(cn, S_ITEM)
gesture(f)

f = screen(S["D6"])
app_bar(f, "Check", "Shared from another app", right_icon=None, lead="close")
cl = D.rect(f, M - 8, 42, 40, 40, None, name="Close")
goto(cl, S_CHECKIN)
TS = 272
D.image(f, (W - TS) / 2, 104, TS, TS, "hail", 16)
redact_hail(f, (W - TS) / 2, 104, TS, TS)
D.text(f, 0, 104 + TS + 12, "Screenshot · 1080 × 1080 · 214 KB", 12, "Regular", MUTED, w=W, align="center", font=MONO)
py0 = 104 + TS + 44
pc = card(f, M, py0, W - 2 * M, 5 * 64 + 16)
goto(pc, S_CHECK)
STEPS = (("done", "Reading the file", "JPEG · re-encoded by the platform"),
         ("muted", "Content Credentials · none found", "Stripped on upload — expected for screenshots"),
         ("run", "Searching fingerprints", "3 index shards · 2 answered"),
         ("wait", "Reading the watermark", None),
         ("wait", "Comparing with the reference", None))
for k, (st, t, sub) in enumerate(STEPS):
    ry = py0 + 8 + k * 64
    ic, c, tc = {"done": ("check_circle", INK, INK), "muted": ("check_circle", MUTED, MUTED), "run": ("sync", PRIMARY, INK),
                 "wait": ("radio_button_unchecked", MUTED, MUTED)}[st]
    D.icon(f, ic, M + 16, ry + 20, 22, c)
    D.text(f, M + 52, ry + (12 if sub else 21), t, 14, "Semi Bold" if st in ("done", "run") else "Medium", tc)
    if sub:
        D.text(f, M + 52, ry + 33, sub, 12, "Regular", PRIMARY if st == "run" else TX2)
    if st == "run":
        D.rect(f, W - M - 76, ry + 29, 60, 4, SURF2, 2)
        D.rect(f, W - M - 76, ry + 29, 40, 4, PRIMARY, 2)
ny = py0 + 5 * 64 + 16 + 20
D.icon(f, "phonelink_lock", M + 4, ny, 18, TX2)
D.text(f, M + 30, ny + 1, "Checked on your phone — nothing is uploaded", 12.5, "Regular", TX2)
cx_ = button(f, W / 2 - 60, 846, 120, 44, "Cancel", "outlined")
goto(cx_, S_CHECKIN)
gesture(f)

f = screen(S["D8"])
app_bar(f, "Check", "Shared from another app", right_icon=None, lead="close")
cl = D.rect(f, M - 8, 42, 40, 40, None, name="Close")
goto(cl, S_CHECKIN)
VY_ = 100
D.rect(f, M, VY_, W - 2 * M, 136, INK, 20, a=0.05, stroke=INK, sw=1.2, sa=0.25, name="Verdict")
D.icon(f, "lock", M + 16, VY_ + 16, 30, INK)
D.text(f, M + 56, VY_ + 18, "Sealed · not yet anchored", 21, "Bold", INK, ls=-0.5)
D.text(f, M + 16, VY_ + 58, "This file is validly sealed by an enrolled phone, but its proof hasn't reached the registry yet — usually it was shared straight from a phone that's offline.",
       13, "Regular", INK, w=W - 2 * M - 32, align="left", lh=1.35)
fy = VY_ + 152
card(f, M, fy, W - 2 * M, 88)
D.image(f, M + 12, fy + 12, 64, 64, "water", 10)
D.text(f, M + 90, fy + 16, "IMG_2041.jpg", 14, "Semi Bold", INK)
D.text(f, M + 90, fy + 37, "Received in Messages · 3.1 MB", 12, "Regular", TX2)
D.text(f, M + 90, fy + 55, "Captured Nov 19, 9:48 AM", 12, "Regular", TX2)
ky = fy + 104
card(f, M, ky, W - 2 * M, 38 + 3 * 64 + 8)
overline(f, M + 16, ky + 14, "What we could check")
for k, (ic, t, sub, ok) in enumerate((("lock", "Seal", "Valid · unchanged since sealing", True),
                                      ("phone_android", "Device", "Genuine Android phone · attested", True),
                                      ("verified", "Anchor", "Not in the registry yet", False))):
    ry = ky + 38 + k * 64
    if k:
        D.path(f, [(M + 16, ry), (W - M - 16, ry)], LINE, 1)
    D.icon(f, ic, M + 16, ry + 20, 22, INK)
    D.text(f, M + 52, ry + 12, t, 14, "Semi Bold", INK)
    D.text(f, M + 52, ry + 33, sub, 12, "Regular", TX2)
    if ok:
        chip(f, W - M - 16, ry + 19, "Valid" if k == 0 else "Genuine", OK, h=26, size=11.5, icon="check", right=True)
    else:
        chip(f, W - M - 16, ry + 19, "Pending", WARN, h=26, size=11.5, icon="hourglass_top", right=True)
ty = ky + 38 + 3 * 64 + 8 + 24
D.icon(f, "info", M + 4, ty, 18, TX2)
D.text(f, M + 30, ty + 1, "Check again later for the full verdict.", 13, "Semi Bold", INK)
D.text(f, M + 30, ty + 23, "Until it's anchored we can't say when it was first registered or whether its key was revoked.",
       12.5, "Regular", TX2, w=W - 2 * M - 34, align="left", lh=1.35)
card(f, M, ty + 88, W - 2 * M, 64, r=16)
D.icon(f, "notifications", M + 16, ty + 108, 22, INK)
D.text(f, M + 52, ty + 100, "Tell me when it's anchored", 14, "Semi Bold", INK)
D.text(f, M + 52, ty + 121, "We'll recheck quietly in the background", 12, "Regular", TX2)
toggle(f, W - M - 62, ty + 107)
D.rect(f, 0, 834, W, 81, SURF)
D.path(f, [(0, 834), (W, 834)], LINE, 1)
button(f, M, 848, W - 2 * M, 48, "Check again later", "tonal", icon="schedule")
gesture(f)
