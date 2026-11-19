from wlib import *

f, th, sh = camera(S_CAMPOST, 1, "hail", "Post", "rainbow", "4:12:09 PM · 14 satellites · spoof check passed")
goto(th, S_ITEM)
goto(sh, S_COMPOSER)
for (nx, ny, lab, bw, bh) in ((HOUSE[0], HOUSE[1], "House number", 26, 44), (PLATE[0], PLATE[1], "Plate", 40, 26)):
    px, py = img_pt(nx, ny, 0, 0, W, VF)
    D.rect(f, px - bw / 2, py - bh / 2, bw, bh, None, 6, stroke="#FFD43B", sw=1.6)
    chip(f, px + bw / 2, py - bh / 2 - 28, lab + " · auto-blur", INK, bg="#FFD43B", h=22, size=10.5, right=True)
D.rect(f, M, 626, W - 2 * M, 72, "#101216", 20, a=0.88, stroke=AI, sw=1.2, sa=0.6, name="Post hint")
D.rect(f, M, 644, 3, 36, AI)
D.icon(f, "auto_awesome", M + 16, 640, 18, "#B794FF")
D.text(f, M + 42, 638, "2 things will be blurred when you post", 14, "Semi Bold", WHITE)
D.text(f, M + 42, 659, "Location will show as Denver area · precise stays sealed", 12, "Regular", "#A9ADB5")

f = screen(S_COMPOSER)
app_bar(f, "New post", "Sealed 4:12:09 PM · anchoring…", right_icon=None, lead="close")
PW, PH = W - 2 * M, 290
D.image(f, M, 100, PW, PH, "hail", 16)
redact_hail(f, M, 100, PW, PH)
chip(f, M + 12, 100 + PH - 38, "2 declared blurs", WHITE, bg=INK, h=26, size=11.5, icon="edit")
ed_ = D.rect(f, W - M - 78, 112, 66, 32, "#000000", 16, a=0.5)
goto(ed_, S["B4"])
D.icon(f, "tune", W - M - 68, 119, 18, WHITE)
D.text(f, W - M - 46, 119, "Edit", 12.5, "Semi Bold", WHITE)

gy = 402
generated_surface(f, M, gy, W - 2 * M, 212)
ai_header(f, M + 18, gy + 16, "Before you post", W - 2 * M - 18)
D.ellipse(f, M + 32, gy + 62, 15, AI, a=0.1)
D.icon(f, "face", M + 23, gy + 53, 18, AI)
D.text(f, M + 56, gy + 46, "Blurs · declared edits", 13.5, "Semi Bold", INK)
x = M + 56
for s in ("House number", "Plate"):
    r_, w_ = chip(f, x, gy + 68, s + "  ×", EDIT, h=26, size=11.5)
    x += w_ + 6
for k, (ic, t, s_, act) in enumerate((("near_me", "Location: Denver area", "Precise location hidden, still sealed", "Change"),
                                      ("closed_caption", "Alt text added", "Hailstones piled on a lawn and a dark car…", "Edit"))):
    ry = gy + 108 + k * 50
    D.path(f, [(M + 18, ry - 7), (W - M - 18, ry - 7)], LINE, 1)
    D.ellipse(f, M + 32, ry + 18, 15, AI, a=0.1)
    D.icon(f, ic, M + 23, ry + 9, 18, AI)
    D.text(f, M + 56, ry + 2, t, 13.5, "Semi Bold", INK)
    D.text(f, M + 56, ry + 21, s_, 12, "Regular", TX2)
    at_ = D.text(f, M, ry + 10, act, 12.5, "Semi Bold", PRIMARY, w=W - 3 * M - 6, align="right")
    if act == "Change":
        goto(at_, S["B5"])

cy_ = 626
D.rect(f, M, cy_, W - 2 * M, 112, SURF, 16, stroke=AI, sw=1.2, sa=0.35)
overline(f, M + 16, cy_ + 12, "Caption")
generated_tag(f, W - M - 12, cy_ + 8, "Suggested", right=True)
D.rect(f, M + 14, cy_ + 30, W - 2 * M - 28, 30, SURF2, 8)
D.text(f, M + 24, cy_ + 37, "Golf-ball hail out of nowhere. Shot it, sealed it.", 13.5, "Regular", INK)
D.rect(f, M + 14, cy_ + 68, 240, 32, PRIMARY, 16, a=0.08)
D.icon(f, "link", M + 26, cy_ + 75, 18, PRIMARY)
D.text(f, M + 50, cy_ + 76, f"{DOMAIN}/p/H4IL-7QK", 12.5, "Regular", PRIMARY, font=MONO)
D.text(f, M + 16, 752, "Add proof link to caption", 14, "Medium", INK)
toggle(f, W - M - 46, 748, True)
D.text(f, M + 16, 790, "Anyone can check this post, even as a screenshot.", 12, "Regular", MUTED)
D.rect(f, 0, 834, W, 81, SURF)
D.path(f, [(0, 834), (W, 834)], LINE, 1)
button(f, M, 848, 140, 48, "Copy link", "tonal", icon="content_copy")
sp_ = button(f, M + 152, 848, W - 2 * M - 152, 48, "Share post", "filled", icon="share")
goto(sp_, S["B6"])
gesture(f)

f, th, sh = camera(S["B2"], 2, "rainbow", "Post", "hail", "4:31:52 PM · no network · sealed on-device")
seal_badge(f, M + 68, VF + 48, "queued", 9, ring=CAM)
goto(th, S_ITEM)
goto(sh, S_COMPOSER)
AMBER = "#F2B35B"
ow_ = est_width("Offline · will anchor later", 12, "Semi Bold") + 40
D.rect(f, M, 138, ow_, 30, "#000000", 15, a=0.5)
D.rect(f, M, 138, ow_, 30, WARN, 15, a=0.4, stroke=AMBER, sw=1, sa=0.5)
D.icon(f, "cloud_off", M + 10, 145, 16, AMBER)
D.text(f, M + 31, 145, "Offline · will anchor later", 12, "Semi Bold", WHITE)
tt_ = D.rect(f, M, 626, W - 2 * M, 72, "#101216", 20, a=0.9, stroke=OK, sw=1.2, sa=0.5, name="Sealed toast")
goto(tt_, S_ITEM)
D.image(f, M + 12, 638, 48, 48, "hail", 10)
D.ellipse(f, M + 58, 640, 11, "#101216")
D.ellipse(f, M + 58, 640, 9, OK)
D.icon(f, "lock", M + 52, 634, 12, WHITE)
D.text(f, M + 76, 640, "Sealed · StrongBox", 14, "Semi Bold", WHITE)
D.icon(f, "hourglass_top", M + 76, 662, 14, AMBER)
D.text(f, M + 94, 660, "Queued for anchoring", 12, "Regular", AMBER)
D.icon(f, "chevron_right", W - M - 34, 652, 20, "#A3A8B0")

f = screen(S["B4"], fill=CAM)
status_bar(f, WHITE)
cl_ = D.rect(f, M - 8, 42, 40, 40, None, name="Close")
D.icon(f, "close", M, 50, 24, WHITE)
goto(cl_, S_COMPOSER)
D.text(f, 0, 51, "Edit photo", 17, "Semi Bold", WHITE, w=W, align="center")
goto(button(f, W - M - 84, 44, 84, 36, "Done", "filled", size=14), S_COMPOSER)
IX, IY, IW, IH = M, 104, W - 2 * M, 508
D.image(f, IX, IY, IW, IH, "hail")
redact_hail(f, IX, IY, IW, IH)
CW, CH = 324, 405
CX, CY = (W - CW) / 2, IY + 52
for (dx, dy, dw, dh) in ((IX, IY, IW, CY - IY), (IX, CY + CH, IW, IY + IH - CY - CH), (IX, CY, CX - IX, CH), (CX + CW, CY, IX + IW - CX - CW, CH)):
    D.rect(f, dx, dy, dw, dh, "#000000", a=0.7, name="Crop dim")
D.rect(f, CX, CY, CW, CH, None, stroke=WHITE, sw=1, sa=0.9, name="Crop 4:5")
for k in (1, 2):
    D.path(f, [(CX + CW * k / 3, CY), (CX + CW * k / 3, CY + CH)], WHITE, 1, a=0.35)
    D.path(f, [(CX, CY + CH * k / 3), (CX + CW, CY + CH * k / 3)], WHITE, 1, a=0.35)
for (bx, by_, sx, sy) in ((CX, CY, 1, 1), (CX + CW, CY, -1, 1), (CX, CY + CH, 1, -1), (CX + CW, CY + CH, -1, -1)):
    D.path(f, [(bx, by_ + 24 * sy), (bx, by_), (bx + 24 * sx, by_)], WHITE, 4)
for (a_, b_) in (((CX + CW / 2 - 12, CY), (CX + CW / 2 + 12, CY)), ((CX + CW / 2 - 12, CY + CH), (CX + CW / 2 + 12, CY + CH)),
                 ((CX, CY + CH / 2 - 12), (CX, CY + CH / 2 + 12)), ((CX + CW, CY + CH / 2 - 12), (CX + CW, CY + CH / 2 + 12))):
    D.path(f, [a_, b_], WHITE, 4)
asp = ("Free", "1:1", "4:5", "9:16")
x = (W - sum(est_width(s, 12.5, "Semi Bold") + 20 for s in asp) - 8 * (len(asp) - 1)) / 2
for s in asp:
    on = s == "4:5"
    x += chip(f, x, 636, s, INK if on else WHITE, bg=WHITE if on else None, h=32, size=12.5)[1] + 8
D.path(f, [(0, 696), (W, 696)], "#23262C", 1)
for k, (s, ic) in enumerate((("Crop", "crop"), ("Straighten", "straighten"), ("Tone", "tonality"), ("Blur", "blur_on"))):
    cx = W / 8 * (2 * k + 1)
    on = k == 0
    c = WHITE if on else "#A3A8B0"
    if on:
        D.rect(f, cx - 30, 712, 60, 32, WHITE, 16, a=0.14)
    D.icon(f, ic, cx - 12, 716, 24, c)
    D.label(f, cx - 51, 750, 102, 16, s, 12, "Semi Bold" if on else "Medium", c)
nw_ = est_width("Every edit is declared in the proof", 12, "Regular") + 22
D.icon(f, "edit", (W - nw_) / 2, 808, 16, "#A3A8B0")
D.text(f, (W - nw_) / 2 + 22, 808, "Every edit is declared in the proof", 12, "Regular", "#A3A8B0")
gesture(f, WHITE, 0.5)

f = screen(S["B5"])
app_bar(f, "New post", "Sealed 4:12:09 PM · anchoring…", right_icon=None, lead="close")
D.image(f, M, 100, W - 2 * M, 290, "hail", 16)
redact_hail(f, M, 100, W - 2 * M, 290)
D.rect(f, 0, 0, W, H, "#000000", a=0.45, name="Dim")
SY = 416
D.rect(f, 0, SY, W, H - SY, SURF, radii=[26, 26, 0, 0], name="Location sheet")
D.rect(f, W / 2 - 16, SY + 10, 32, 4, MUTED, 2, a=0.4, name="Handle")
D.text(f, M, SY + 32, "Location on this post", 20, "Semi Bold", INK)
D.text(f, M, SY + 62, "Only changes what people see on the post.", 13, "Regular", TX2)
for k, (t, s_, c, ic, on) in enumerate((("Hidden", "No location on the post", TX2, None, False),
                                         ("City or area · Denver area", "A broad area, not your street", TX2, None, True),
                                         ("Neighborhood", "Narrower area, easier to pin down", WARN_T, "info", False),
                                         ("Exact spot", "Shows where you were standing", CRIT, "warning", False))):
    ry = SY + 96 + k * 64
    if on:
        D.rect(f, M, ry, W - 2 * M, 60, PRIMARY, 14, a=0.07)
    D.icon(f, "radio_button_checked" if on else "radio_button_unchecked", M + 12, ry + 18, 24, PRIMARY if on else TX2)
    D.text(f, M + 48, ry + 10, t, 15, "Semi Bold" if on else "Medium", INK)
    sx = M + 48
    if ic:
        D.icon(f, ic, sx, ry + 33, 14, c)
        sx += 18
    D.text(f, sx, ry + 32, s_, 12.5, "Regular", c)
D.icon(f, "lock", M, 786, 16, TX2)
D.text(f, M + 24, 784, "The precise location stays sealed either way — you can prove it later if you choose.", 12.5, "Regular", TX2,
       w=W - 2 * M - 24, align="left", lh=1.35)
goto(button(f, M, 848, W - 2 * M, 48, "Done", "filled"), S_COMPOSER)
gesture(f)

f = screen(S["B6"])
status_bar(f)
cl_ = D.rect(f, M - 8, 42, 40, 40, None, name="Close")
D.icon(f, "close", M, 50, 24, INK)
goto(cl_, S_HOME)
D.ellipse(f, W / 2, 200, 56, OK, a=0.1)
D.ellipse(f, W / 2, 200, 40, OK)
D.icon(f, "verified", W / 2 - 22, 178, 44, WHITE)
D.text(f, M, 280, "Your post has a proof link", 24, "Bold", INK, w=W - 2 * M, align="center")
D.text(f, M, 316, "Sealed 4:12:09 PM · anchored 4:12:41 PM", 13, "Regular", TX2, w=W - 2 * M, align="center")
lk_ = f"{DOMAIN}/p/H4IL-7QK"
lw_ = 16 + 18 + 8 + est_width(lk_, 13.5, "Regular", MONO) + 14 + 18 + 16
lx_ = (W - lw_) / 2
D.rect(f, lx_, 352, lw_, 40, PRIMARY, 20, a=0.08, name="Proof link")
D.icon(f, "link", lx_ + 16, 363, 18, PRIMARY)
D.text(f, lx_ + 42, 363, lk_, 13.5, "Regular", PRIMARY, font=MONO)
D.icon(f, "content_copy", lx_ + lw_ - 34, 363, 18, PRIMARY)
qr(f, (W - 176) / 2, 428, 176, seed="H4IL-7QK")
D.text(f, M, 640, "Anyone can check this post — even a screenshot of it.", 13, "Regular", TX2, w=W - 2 * M, align="center")
card(f, M, 680, W - 2 * M, 72)
cw_ = (W - 2 * M) / 3
for k, (lab, val) in enumerate((("Checked", "0"), ("Shared", "just now"), ("Shows", "Denver area"))):
    x = M + k * cw_
    if k:
        D.path(f, [(x, 696), (x, 736)], LINE, 1)
    overline(f, x + 16, 694, lab)
    D.text(f, x + 16, 714, val, 15, "Semi Bold", INK)
D.rect(f, 0, 834, W, 81, SURF)
D.path(f, [(0, 834), (W, 834)], LINE, 1)
button(f, M, 848, 140, 48, "Copy link", "tonal", icon="content_copy")
goto(button(f, M + 152, 848, W - 2 * M - 152, 48, "Done", "filled"), S_HOME)
gesture(f)
