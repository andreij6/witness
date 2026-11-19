from wlib import *


def hit(p, x, y, w, h, dest, name="Hit"):
    r = D.rect(p, x, y, w, h, None, name=name)
    goto(r, dest)
    return r


def dots(p, y, n, k):
    tw_ = 22 + (n - 1) * 8 + (n - 1) * 6
    x = (W - tw_) / 2
    for i in range(n):
        w_ = 22 if i == k else 8
        D.rect(p, x, y, w_, 8, INK if i == k else SURF3, 4)
        x += w_ + 6


def arc(p, cx, cy, r, a0, a1, color, sw):
    n = 18
    pts = [(cx + r * math.cos(math.radians(a0 + (a1 - a0) * i / n)), cy + r * math.sin(math.radians(a0 + (a1 - a0) * i / n))) for i in range(n + 1)]
    D.path(p, pts, color, sw)


def icon_disc(p, cx, cy, r, ic, color=INK, fill=SURF2, a=1.0, size=None):
    size = size or round(r * 1.1)
    D.ellipse(p, cx, cy, r, fill, a=a)
    D.icon(p, ic, cx - size / 2, cy - size / 2, size, color)


def home_header(f, trust):
    status_bar(f)
    wordmark(f, M, 44, 23)
    D.icon(f, "search", W - M - 72, 48, 24, INK)
    hit(f, W - M - 82, 40, 44, 40, S["A9"], "Search")
    st_ = D.ellipse(f, W - M - 18, 60, 18, SURF3)
    D.icon(f, "settings", W - M - 29, 49, 22, TX2)
    goto(st_, S_SETTINGS)
    D.icon(f, "verified", M, 88, 17, OK)
    D.text(f, M + 23, 88, trust, 12.5, "Medium", TX2)
    hit(f, M, 82, est_width(trust, 12.5, "Medium") + 30, 30, S["A8"], "Anchoring status")


def highlight(p, x, y, s, q, size, style, color=INK):
    i = s.index(q)
    k = 1.1 if style == "Regular" else 1.0
    D.rect(p, x + (est_width(s[:i], size, style) - 2) * k - 3, y - 1, (est_width(q, size, style) - 2) * k + 6, size * 1.3 + 2, PRIMARY, 4, a=0.14)
    D.text(p, x, y, s, size, style, color)


def case_thumb(p, bx, y):
    D.image(p, bx, y, 46, 56, "crash", radii=[10, 2, 2, 10])
    D.image(p, bx + 48, y, 28, 27, "bumper", radii=[2, 10, 2, 2])
    D.image(p, bx + 48, y + 29, 28, 27, "plate", radii=[2, 2, 10, 2])


f = screen(S["A1"], fill=INK)
status_bar(f, WHITE)
mark(f, W / 2, 388, 120, ring=WHITE, check="#34D399")
D.text(f, 0, 468, "Witness", 36, "Bold", WHITE, w=W, align="center", ls=-1.5)
D.text(f, 0, 518, "Proof, not just pictures", 15, "Regular", "#A3A8B0", w=W, align="center")
D.text(f, 0, 852, "Sealed on your phone · anchored on the Internet Computer", 11, "Regular", "#6F747C", w=W, align="center", font=MONO)
gesture(f, WHITE, 0.5)
hit(f, 0, 0, W, H, S["A2"], "Tap to continue")

f = screen(S["A2"])
status_bar(f)
HY, HH = 64, 360
D.image(f, M, HY, W - 2 * M, HH, "hail", 24)
redact_hail(f, M, HY, W - 2 * M, HH)
chip(f, M + 14, HY + 14, "Sealed", WHITE, bg=INK, h=28, size=12.5, icon="lock")
glass_pill(f, M + 14, HY + HH - 44, "Denver area · Nov 18, 4:12 PM", "location_on")
seal_badge(f, W - M - 30, HY + HH - 30, "anchored", 16, ring=SURF)
D.text(f, M, 448, "Proof, not just pictures", 28, "Bold", INK, ls=-1)
D.text(f, M, 490, "Witness seals each photo on your phone the moment you take it, so anyone can check it later.",
       14, "Regular", TX2, w=W - 2 * M, align="left", lh=1.45)
for k, (ic, s_) in enumerate((("lock", "Sealed by your phone's security chip"), ("trending_up", "Proof survives screenshots and reposts"),
                              ("visibility_off", "Your photos stay on your phone"))):
    y = 560 + k * 52
    icon_disc(f, M + 20, y + 20, 20, ic, INK, SURF3)
    D.text(f, M + 56, y + 11, s_, 14.5, "Medium", INK)
dots(f, 724, 3, 0)
goto(button(f, M, 752, W - 2 * M, 52, "Get started", "filled", size=15), S["A3"])
D.text(f, M, 820, "I just want to check a photo", 14, "Semi Bold", PRIMARY, w=W - 2 * M, align="center")
hit(f, M, 812, W - 2 * M, 34, S_CHECKIN, "Check a photo")
gesture(f)

f = screen(S["A3"])
status_bar(f)
D.icon(f, "arrow_back", M, 50, 24, INK)
hit(f, M - 8, 42, 40, 40, S["A2"], "Back")
D.text(f, M, 100, "Two permissions", 28, "Bold", INK, ls=-1)
D.text(f, M, 142, "Witness asks only for what makes a photo provable, and only while the camera is open.",
       14, "Regular", TX2, w=W - 2 * M, align="left", lh=1.45)
PY_ = 208
card(f, M, PY_, W - 2 * M, 216)
for k, (ic, t, s_) in enumerate((("photo_camera", "Camera", "Required to take sealed photos"),
                                 ("near_me", "Location while using", "Optional — makes location provable; never in the background"))):
    ry = PY_ + k * 100
    if k:
        D.path(f, [(M + 16, ry), (W - M - 16, ry)], LINE, 1)
    icon_disc(f, M + 38, ry + 46, 22, ic, INK, SURF2, size=24)
    D.text(f, M + 76, ry + 26, t, 15.5, "Semi Bold", INK)
    D.text(f, M + 76, ry + 51, s_, 13, "Regular", TX2, w=W - 2 * M - 96, align="left", lh=1.4)
NY = 440
card(f, M, NY, W - 2 * M, 136)
overline(f, M + 16, NY + 16, "Witness never asks for")
for k, s_ in enumerate(("Access to your photo library", "Location in the background", "Contacts, microphone or your accounts")):
    ry = NY + 44 + k * 28
    D.icon(f, "block", M + 16, ry, 18, MUTED)
    D.text(f, M + 44, ry + 1, s_, 13.5, "Regular", INK)
D.rect(f, M, 592, W - 2 * M, 64, SURF2, 16)
D.icon(f, "photo_library", M + 16, 612, 22, TX2)
D.text(f, M + 52, 604, "Gallery imports use the system picker — no access to your library", 12.5, "Regular", TX2,
       w=W - 2 * M - 68, align="left", lh=1.4)
dots(f, 728, 3, 1)
goto(button(f, M, 756, W - 2 * M, 52, "Continue", "filled", size=15), S["A4"])
D.text(f, M, 826, "You can change these anytime in Android settings", 12, "Regular", MUTED, w=W - 2 * M, align="center")
gesture(f)

f = screen(S["A4"])
status_bar(f)
D.text(f, M, 100, "Setting up your phone", 28, "Bold", INK, ls=-1)
D.text(f, M, 142, "Witness creates a signing key inside your phone's security chip. It can never be copied out, not even by us.",
       14, "Regular", TX2, w=W - 2 * M, align="left", lh=1.45)
SY = 216
card(f, M, SY, W - 2 * M, 304)
steps = (("Hardware key created · StrongBox", "P-256 key · can't be exported", "done"),
         ("Verified by Google key attestation", "Locked bootloader · verified OS · real app", "done"),
         ("Witness certificate issued", "Waiting for the registry…", "busy"),
         ("Ready to capture", "Your first photo can be sealed right after", "todo"))
for k, (t, s_, st) in enumerate(steps):
    y = SY + 16 + k * 72
    cx, cy = M + 36, y + 26
    if k < len(steps) - 1:
        D.path(f, [(cx, cy + 17), (cx, cy + 55)], OK if st == "done" and steps[k + 1][2] != "todo" else LINE, 2)
    if st == "done":
        D.ellipse(f, cx, cy, 13, OK)
        D.icon(f, "check", cx - 8, cy - 8, 16, WHITE)
    elif st == "busy":
        D.ellipse(f, cx, cy, 12, None, stroke=PRIMARY, sw=3, sa=0.18)
        arc(f, cx, cy, 12, -90, 150, PRIMARY, 3)
    else:
        D.ellipse(f, cx, cy, 12, None, stroke=MUTED, sw=1.6, sa=0.6)
    D.text(f, M + 64, y + 10, t, 14.5, "Semi Bold", MUTED if st == "todo" else INK)
    D.text(f, M + 64, y + 32, s_, 12.5, "Regular", MUTED if st == "todo" else TX2)
D.rect(f, M, 536, W - 2 * M, 68, WARN, 16, a=0.1)
D.icon(f, "cloud_off", M + 16, 559, 22, WARN)
D.text(f, M + 52, 550, "Offline? You can capture right away — setup finishes when you're back online.", 13, "Regular", WARN_T,
       w=W - 2 * M - 68, align="left", lh=1.4)
card(f, M, 620, W - 2 * M, 76, r=16)
D.icon(f, "privacy_tip", M + 16, 646, 22, TX2)
D.text(f, M + 52, 634, "Only your key's attestation is sent to be verified. Photos, accounts and your phone number stay out of it.",
       12.5, "Regular", TX2, w=W - 2 * M - 68, align="left", lh=1.4)
r_ = D.rect(f, M, 772, W - 2 * M, 52, SURF3, 26, name="Button Open camera (disabled)")
D.label(f, M, 772, W - 2 * M, 52, "Open camera", 15, "Semi Bold", MUTED)
goto(r_, S_CAMPOST)
D.text(f, M, 840, "Usually takes a few seconds", 12, "Regular", MUTED, w=W - 2 * M, align="center")
gesture(f)

f = screen(S["A5"])
status_bar(f)
icon_disc(f, W / 2, 192, 48, "gpp_bad", CRIT, CRIT, a=0.12, size=48)
D.text(f, M, 268, "This phone can't seal captures", 22, "Bold", INK, w=W - 2 * M, align="center", ls=-0.5)
D.text(f, M + 8, 308, "Witness couldn't confirm this phone's security. Usually the bootloader is unlocked or the device can't prove it, so photos taken here can't be proven original.",
       14, "Regular", TX2, w=W - 2 * M - 16, align="center", lh=1.45)
RY = 400
card(f, M, RY, W - 2 * M, 160)
overline(f, M + 16, RY + 16, "Common reasons")
for k, s_ in enumerate(("Bootloader is unlocked", "Rooted or custom system software", "No hardware key store on this model",
                        "Security patch is too old")):
    ry = RY + 46 + k * 28
    D.path(f, [(M + 18, ry + 9), (M + 28, ry + 9)], MUTED, 2)
    D.text(f, M + 42, ry, s_, 13.5, "Regular", INK)
CY_ = 576
card(f, M, CY_, W - 2 * M, 168)
icon_disc(f, M + 38, CY_ + 38, 22, "verified_user", PRIMARY, PRIMARY, a=0.1, size=24)
D.text(f, M + 76, CY_ + 18, "You can still check any photo", 15.5, "Semi Bold", INK)
D.text(f, M + 76, CY_ + 42, "See if a photo is an original, matches one with changes, or has no Witness record.", 13, "Regular", TX2,
       w=W - 2 * M - 92, align="left", lh=1.4)
goto(button(f, M + 16, CY_ + 104, W - 2 * M - 32, 48, "Check a photo", "filled", icon="verified_user"), S_CHECKIN)
lw = est_width("Learn why", 14, "Semi Bold") + 24
lx = (W - lw) / 2
D.text(f, lx, 776, "Learn why", 14, "Semi Bold", PRIMARY)
D.icon(f, "open_in_new", lx + lw - 18, 777, 16, PRIMARY)
gesture(f)

f = screen(S["A6"])
home_header(f, "Ready · hardware key active")
cy = 300
D.ellipse(f, W / 2, cy, 104, SURF2)
D.ellipse(f, W / 2, cy, 72, SURF)
mark(f, W / 2, cy + 6, 84)
for dx, dy, r in ((-96, -64, 6), (88, -84, 4), (104, 46, 7), (-84, 78, 4)):
    D.ellipse(f, W / 2 + dx, cy + dy, r, SURF3)
D.text(f, M, 436, "Take your first sealed photo", 24, "Bold", INK, w=W - 2 * M, align="center", ls=-1)
D.text(f, 40, 476, "Sealed by your phone the moment you press the shutter, then anchored so anyone can check it.",
       14, "Regular", TX2, w=W - 80, align="center", lh=1.45)
goto(button(f, (W - 220) / 2, 560, 220, 52, "Open camera", "filled", size=15, icon="photo_camera"), S_CAMPOST)
gc = card(f, M, 672, W - 2 * M, 76, r=16)
icon_disc(f, M + 38, 710, 22, "verified_user", INK, SURF2, size=22)
D.text(f, M + 76, 692, "Got a photo to check?", 14.5, "Semi Bold", INK)
D.text(f, M + 76, 714, "Use the Check tab", 12.5, "Regular", TX2)
D.icon(f, "chevron_right", W - M - 36, 698, 24, MUTED)
goto(gc, S_CHECKIN)
nav_bar(f, "Home")

f = screen(S_HOME)
home_header(f, "31 sealed · 30 anchored · 1 queued")
x = M
for s, on in (("All", True), ("Posts", False), ("Cases", False)):
    if on:
        _, w_ = chip(f, x, 116, s, WHITE, bg=INK, h=34, size=13)
    else:
        _, w_ = chip(f, x, 116, s, TX2, h=34, size=13, style="Medium", outline=True)
    x += w_ + 8

HY = 160
hc = card(f, M, HY, W - 2 * M, 276)
D.image(f, M + 8, HY + 8, W - 2 * M - 16, 180, "hail", 12)
redact_hail(f, M + 8, HY + 8, W - 2 * M - 16, 180)
chip(f, M + 18, HY + 18, "Post", WHITE, bg="#000000", h=24, size=11.5, icon="public")
D.text(f, M + 16, HY + 198, "Golf-ball hail out of nowhere.", 16.5, "Semi Bold", INK)
D.text(f, M + 16, HY + 221, "Today 4:12 PM · Denver area · posted with proof link", 12.5, "Regular", TX2)
x = M + 16
_, w_ = state_chip(f, x, HY + 242, "anchored", h=24, size=11)
x += w_ + 6
chip(f, x, HY + 242, "Checked 1,284×", OK, h=24, size=11, icon="trending_up")
goto(hc, S_COMPOSER)

TY = 448
tw_ = (W - 2 * M - 10) / 2
for k, (img, cap, meta, st) in enumerate((("rainbow", "Double rainbow", "Nov 16 · checked 86×", "anchored"),
                                          ("moose", "Moose on Route 9", "Imported from gallery", "unverified"))):
    tx = M + k * (tw_ + 10)
    D.image(f, tx, TY, tw_, 112, img, 14)
    seal_badge(f, tx + tw_ - 14, TY + 98, st, 9, ring=PAPER)
    D.text(f, tx + 2, TY + 120, cap, 13, "Semi Bold", INK)
    D.text(f, tx + 2, TY + 139, meta, 11.5, "Regular", TX2)

CY2 = 612
D.text(f, M, CY2, "Cases", 16, "Semi Bold", INK)
nc = D.text(f, M, CY2 + 2, "+ New case", 13, "Semi Bold", PRIMARY, w=W - 2 * M, align="right")
for k, (imgs, title, meta, chips_) in enumerate(((("crash", "bumper", "plate"), "Fender bender", "Today · 4 items", (("anchored", "3"), ("queued", "1"))),
                                                 (("water",), "Kitchen ceiling leak", "Nov 12 · 6 items", (("anchored", "6"),)))):
    y = CY2 + 28 + k * 80
    rc = card(f, M, y, W - 2 * M, 72, r=16)
    bx, bw = M + 10, 76
    D.rect(f, bx, y + 8, bw, 56, SURF2, 10)
    if len(imgs) == 1:
        D.image(f, bx, y + 8, bw, 56, imgs[0], 10)
    else:
        D.image(f, bx, y + 8, 46, 56, imgs[0], radii=[10, 2, 2, 10])
        D.image(f, bx + 48, y + 8, 28, 27, imgs[1], radii=[2, 10, 2, 2])
        D.image(f, bx + 48, y + 37, 28, 27, imgs[2], radii=[2, 2, 10, 2])
    D.text(f, M + 100, y + 14, title, 14.5, "Semi Bold", INK)
    D.text(f, M + 100, y + 38, meta, 12, "Regular", TX2)
    xx = W - M - 14
    for st, n_ in reversed(chips_):
        _, w_ = state_chip(f, xx, y + 24, st, right=True, h=24, size=11, text=n_)
        xx -= w_ + 6
    if k == 0:
        goto(rc, S_CASE)
nav_bar(f, "Home")

f = screen(S["A8"])
app_bar(f, "Anchoring", right_icon=None)
hit(f, M - 8, 42, 40, 40, S_HOME, "Back")
SY = 100
card(f, M, SY, W - 2 * M, 168)
for k, (n_, lab, ic, c) in enumerate((("30", "Anchored", "verified", OK), ("1", "Queued", "hourglass_top", WARN))):
    x = M + 20 + k * 172
    D.text(f, x, SY + 16, n_, 32, "Bold", INK, ls=-1)
    D.icon(f, ic, x, SY + 62, 16, c)
    D.text(f, x + 22, SY + 62, lab, 13, "Medium", TX2)
D.path(f, [(M + 172, SY + 24), (M + 172, SY + 82)], LINE, 1)
D.path(f, [(M + 16, SY + 100), (W - M - 16, SY + 100)], LINE, 1)
for k, (ic, s_) in enumerate((("schedule", "Last anchored 2 min ago"), ("verified_user", "Certified by the Internet Computer"))):
    D.icon(f, ic, M + 20, SY + 114 + k * 24, 16, TX2)
    D.text(f, M + 44, SY + 114 + k * 24, s_, 12.5, "Regular", TX2)
overline(f, M, 296, "Queue")
D.text(f, M, 293, "Retry now", 13, "Semi Bold", PRIMARY, w=W - 2 * M, align="right")
QY = 320
qc = card(f, M, QY, W - 2 * M, 80, r=16)
wide_thumb(f, M + 12, QY + 12, 56, 56, r=10, redacted=True)
D.text(f, M + 82, QY + 18, "Wide shot · Fender bender", 14, "Semi Bold", INK)
D.icon(f, "wifi_off", M + 82, QY + 43, 14, WARN)
D.text(f, M + 101, QY + 42, "Waiting for network", 12.5, "Regular", WARN_T)
state_chip(f, W - M - 14, QY + 28, "queued", right=True, h=24, size=11)
goto(qc, S_ITEM)
overline(f, M, 428, "Recently anchored")
RY = 452
card(f, M, RY, W - 2 * M, 216)
for k, (img, t, s_, dest) in enumerate((("hail", "Golf-ball hail out of nowhere", "Post · 2 min ago", S_COMPOSER),
                                        ("plate", "Plate 7MNV562", "Fender bender · Photo 3 · 9 min ago", S_ITEM),
                                        ("bumper", "Rear bumper", "Fender bender · Photo 2 · 11 min ago", S_ITEM))):
    ry = RY + k * 72
    if k:
        D.path(f, [(M + 16, ry), (W - M - 16, ry)], LINE, 1)
    D.image(f, M + 12, ry + 10, 52, 52, img, 10)
    if img == "hail":
        redact_hail(f, M + 12, ry + 10, 52, 52)
    D.text(f, M + 78, ry + 16, t, 14, "Semi Bold", INK)
    D.text(f, M + 78, ry + 38, s_, 12.5, "Regular", TX2)
    D.icon(f, "verified", W - M - 36, ry + 25, 20, OK)
    hit(f, M, ry, W - 2 * M, 72, dest, "Row " + t)
overline(f, M, 696, "Your key")
KY = 720
card(f, M, KY, W - 2 * M, 96)
icon_disc(f, M + 36, KY + 36, 20, "vpn_key", INK, SURF2, size=22)
D.text(f, M + 68, KY + 16, "Active · StrongBox · rotates in 12 days", 14, "Semi Bold", INK)
D.text(f, M + 68, KY + 40, "Previous keys keep signing edits of the photos they sealed", 12.5, "Regular", TX2, w=W - 2 * M - 84, align="left", lh=1.4)
gesture(f)

f = screen(S["A9"])
status_bar(f)
D.icon(f, "arrow_back", M, 52, 24, INK)
hit(f, M - 8, 44, 40, 40, S_HOME, "Back")
D.rect(f, M + 40, 42, W - 2 * M - 40, 44, SURF, 22, stroke=PRIMARY, sw=1.5)
D.text(f, M + 60, 54, "7MNV", 15, "Regular", INK)
D.rect(f, M + 61 + (est_width("7MNV", 15, "Regular") - 2) * 1.15, 53, 1.5, 22, PRIMARY)
D.icon(f, "close", W - M - 34, 54, 20, TX2)
x = M
for s, on in (("All", True), ("Posts", False), ("Cases", False), ("Items", False)):
    if on:
        _, w_ = chip(f, x, 102, s, WHITE, bg=INK, h=34, size=13)
    else:
        _, w_ = chip(f, x, 102, s, TX2, h=34, size=13, style="Medium", outline=True)
    x += w_ + 8
overline(f, M, 160, "Items · 2")
IY = 182
card(f, M, IY, W - 2 * M, 160)
for k, (img, t, s_, ht) in enumerate((("plate", "Plate 7MNV562 · Photo 3", "Fender bender · Today 2:09 PM", True),
                                      ("bumper", "Rear damage · Photo 2", "Note: hit by 7MNV562, silver sedan", False))):
    ry = IY + k * 80
    if k:
        D.path(f, [(M + 16, ry), (W - M - 16, ry)], LINE, 1)
    D.image(f, M + 12, ry + 12, 56, 56, img, 10)
    seal_badge(f, M + 62, ry + 62, "anchored", 8)
    if ht:
        highlight(f, M + 82, ry + 18, t, "7MNV", 14, "Semi Bold")
        D.text(f, M + 82, ry + 42, s_, 12.5, "Regular", TX2)
    else:
        D.text(f, M + 82, ry + 18, t, 14, "Semi Bold", INK)
        highlight(f, M + 82, ry + 42, s_, "7MNV", 12.5, "Regular", TX2)
    D.icon(f, "chevron_right", W - M - 36, ry + 28, 24, MUTED)
    hit(f, M, ry, W - 2 * M, 80, S_ITEM, "Row " + t)
overline(f, M, 366, "Cases · 1")
cc = card(f, M, 388, W - 2 * M, 80)
case_thumb(f, M + 12, 400)
D.text(f, M + 102, 406, "Fender bender", 14, "Semi Bold", INK)
highlight(f, M + 102, 430, "Oak St & 5th Ave · plate 7MNV562", "7MNV", 12.5, "Regular", TX2)
D.icon(f, "chevron_right", W - M - 36, 416, 24, MUTED)
goto(cc, S_CASE)
nw = est_width("Search runs on your phone", 12, "Regular") + 20
D.icon(f, "lock", (W - nw) / 2, 520, 14, MUTED)
D.text(f, (W - nw) / 2 + 20, 519, "Search runs on your phone", 12, "Regular", MUTED)
KB = 600
D.rect(f, 0, KB, W, H - KB, SURF2, name="Keyboard")
for k, s_ in enumerate(("7MNV562", "7MNV", "7MNV5")):
    D.label(f, k * W / 3, KB + 6, W / 3, 32, s_, 14, "Medium" if k else "Semi Bold", INK)
    if k:
        D.path(f, [(k * W / 3, KB + 14), (k * W / 3, KB + 30)], LINE, 1)
kw = (W - 12 - 54) / 10
for r_i, row in enumerate(("QWERTYUIOP", "ASDFGHJKL", "ZXCVBNM")):
    ky = KB + 52 + r_i * 56
    x0 = 6 + (kw + 6) * (0.5 if r_i == 1 else 1.5 if r_i == 2 else 0)
    for i, ch in enumerate(row):
        kx = x0 + i * (kw + 6)
        D.rect(f, kx, ky, kw, 46, SURF, 6)
        D.label(f, kx, ky, kw, 46, ch, 18, "Regular", INK)
ky = KB + 52 + 2 * 56
sw_ = 1.5 * kw
D.rect(f, 6, ky, sw_, 46, SURF3, 6)
D.path(f, [(6 + sw_ / 2 - 8, ky + 24), (6 + sw_ / 2, ky + 15), (6 + sw_ / 2 + 8, ky + 24)], INK, 2)
D.path(f, [(6 + sw_ / 2, ky + 15), (6 + sw_ / 2, ky + 32)], INK, 2)
D.rect(f, W - 6 - sw_, ky, sw_, 46, SURF3, 6)
D.icon(f, "arrow_back", W - 6 - sw_ / 2 - 11, ky + 12, 22, INK)
ky += 56
x = 6
for s_, w_k, fill in (("?123", 1.5, SURF3), (",", 1, SURF3), ("", 4, SURF), (".", 1, SURF3)):
    ww = w_k * kw + (w_k - 1) * 6
    D.rect(f, x, ky, ww, 46, fill, 6)
    if s_:
        D.label(f, x, ky, ww, 46, s_, 15 if len(s_) > 1 else 18, "Medium", INK)
    x += ww + 6
D.rect(f, x, ky, W - 6 - x, 46, PRIMARY, 6)
D.icon(f, "search", x + (W - 6 - x) / 2 - 11, ky + 12, 22, WHITE)
gesture(f)
