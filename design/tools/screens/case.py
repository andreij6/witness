from wlib import *


def dim_sheet(f, y, title, sub=None):
    D.rect(f, 0, 0, W, H, "#000000", a=0.45, name="Dim")
    D.rect(f, 0, y, W, H - y, SURF, radii=[26, 26, 0, 0], name="Sheet")
    D.rect(f, W / 2 - 16, y + 10, 32, 4, INK, 2, a=0.2, name="Handle")
    D.text(f, M + 8, y + 32, title, 20, "Semi Bold", INK)
    if sub:
        D.text(f, M + 8, y + 60, sub, 12.5, "Regular", TX2)


def radio(p, x, y, on):
    D.icon(p, "radio_button_checked" if on else "radio_button_unchecked", x, y, 24, PRIMARY if on else MUTED)


def option_row(f, y, ic, title, sub, on=False, muted=False, trailing="radio"):
    hit = D.rect(f, M, y, W - 2 * M, 56, PRIMARY if on else None, 14, a=0.06, name="Row " + title)
    D.rect(f, M + 8, y + 8, 40, 40, PRIMARY if on else SURF2, 12, a=0.12 if on else 1.0)
    D.icon(f, ic, M + 16, y + 16, 24, PRIMARY if on else MUTED if muted else TX2)
    D.text(f, M + 60, y + 9, title, 14.5, "Semi Bold", MUTED if muted else INK)
    D.text(f, M + 60, y + 31, sub, 12, "Regular", MUTED if muted else TX2)
    if trailing == "radio":
        radio(f, W - M - 36, y + 16, on)
    return hit


def case_under(f):
    app_bar(f, "Fender bender", "Oak St & 5th Ave · Today 2:31 PM")
    D.rect(f, M, 100, W - 2 * M, 40, AI, 12, a=0.08)
    D.icon(f, "auto_awesome", M + 12, 110, 20, AI)
    D.text(f, M + 40, 111, "Report built on-device · Car accident", 13, "Semi Bold", AI)
    overline(f, M, 156, "Evidence · 4 sealed")
    D.text(f, M, 155, "3 anchored · 1 queued", 12, "Medium", MUTED, w=W - 2 * M, align="right")
    for k, (img, st) in enumerate((("crash", "anchored"), ("bumper", "anchored"), ("plate", "anchored"), ("wide", "queued"))):
        D.image(f, M + k * 92, 178, 84, 108, img, 12)
        seal_badge(f, M + k * 92 + 72, 274, st, 9, ring=PAPER)
    generated_surface(f, M, 302, W - 2 * M, 150)
    ai_header(f, M + 18, 318, "Still recommended", W - 2 * M - 18)
    for k, (done, s_) in enumerate(((True, "Damage close-up"), (True, "Both license plates"), (True, "Wide shot of both cars"),
                                    (False, "Other driver's insurance card"))):
        D.icon(f, "check_circle" if done else "add", M + 18, 344 + k * 26, 20, OK if done else PRIMARY)
        D.text(f, M + 46, 345 + k * 26, s_, 13.5, "Medium" if done else "Semi Bold", TX2 if done else INK)
    D.rect(f, 0, 834, W, 81, SURF)


f, th, sh = camera(S_CAMCASE, 5, "wide", "Case", "plate", "2:32:07 PM · 14 satellites · spoof check passed")
goto(th, S_ITEM)
goto(sh, S_CASE)
cs = glass_pill(f, M, 138, "Case: Fender bender", "folder", WHITE, trailing="expand_more")
goto(D.rect(f, M, 138, cs, 30, None, 15, name="Case pill"), S["C2"])
goto(D.rect(f, M + cs + 6, 138, 104, 30, "#000000", 15, a=0.5, name="New case pill"), S["C3"])
D.icon(f, "add", M + cs + 14, 144, 18, WHITE)
D.text(f, M + cs + 36, 145, "New case", 12, "Semi Bold", WHITE)
gy = 586
D.rect(f, M, gy, W - 2 * M, 112, "#101216", 20, a=0.9, stroke=AI, sw=1.2, sa=0.6, name="Evidence coach")
D.rect(f, M, gy + 18, 3, 76, AI)
D.icon(f, "auto_awesome", M + 16, gy + 14, 18, "#B794FF")
D.text(f, M + 40, gy + 14, "EVIDENCE COACH", 11, "Semi Bold", "#B794FF", ls=8)
flag(f, W - M - 34, gy + 12, "#8E939C")
D.text(f, M + 16, gy + 38, "Now get a wide shot showing both cars\nand the intersection.", 15, "Semi Bold", WHITE, lh=1.35)
for k in range(5):
    D.rect(f, M + 16 + k * 22, gy + 92, 16, 4, "#B794FF" if k < 3 else "#3A3D45", 2)
D.text(f, M + 134, gy + 86, "3 of 5", 11.5, "Medium", "#A9ADB5")
D.text(f, M, gy + 86, "Skip", 12.5, "Semi Bold", "#D0D3D8", w=W - 3 * M, align="right")

f = screen(S["C2"], fill=CAM)
D.image(f, 0, 0, W, VF, "wide")
scrim(f, 0, 170, 0.5)
status_bar(f, WHITE)
mode_switch(f, "Case")
glass_pill(f, M, 138, "Case: Fender bender", "folder", WHITE, trailing="expand_more")
D.ellipse(f, W / 2, VF + 72, 38, None, stroke=WHITE, sw=4)
D.ellipse(f, W / 2, VF + 72, 30, WHITE)
SY = 496
dim_sheet(f, SY, "Capture into")
for k, (title, sub, on) in enumerate((("Fender bender", "Car accident · 4 items", True), ("Kitchen ceiling leak", "Property damage · 6 items", False),
                                      ("Move-out walkthrough", "Rental condition · 12 items", False))):
    goto(option_row(f, SY + 72 + k * 64, "folder", title, sub, on), S_CAMCASE)
D.path(f, [(M, SY + 268), (W - M, SY + 268)], LINE, 1)
nc = D.rect(f, M, SY + 276, W - 2 * M, 48, None, 14, name="Row New case")
D.rect(f, M + 8, SY + 280, 40, 40, PRIMARY, 12, a=0.1)
D.icon(f, "add", M + 16, SY + 288, 24, PRIMARY)
D.text(f, M + 60, SY + 290, "New case", 14.5, "Semi Bold", PRIMARY)
goto(nc, S["C3"])
D.icon(f, "info", M + 8, SY + 344, 16, MUTED)
D.text(f, M + 30, SY + 344, "Post mode captures don't go into a case.", 12, "Regular", MUTED)
gesture(f)

f = screen(S["C3"])
status_bar(f)
wordmark(f, M, 44, 23)
D.icon(f, "search", W - M - 72, 48, 24, INK)
D.ellipse(f, W - M - 18, 60, 18, SURF3)
D.icon(f, "settings", W - M - 29, 49, 22, TX2)
D.icon(f, "verified", M, 88, 17, OK)
D.text(f, M + 23, 88, "31 sealed · 30 anchored · 1 queued", 12.5, "Medium", TX2)
x = M
for s_, on in (("All", True), ("Posts", False), ("Cases", False)):
    if on:
        _, w_ = chip(f, x, 116, s_, WHITE, bg=INK, h=34, size=13)
    else:
        _, w_ = chip(f, x, 116, s_, TX2, h=34, size=13, style="Medium", outline=True)
    x += w_ + 8
card(f, M, 160, W - 2 * M, 244)
D.image(f, M + 8, 168, W - 2 * M - 16, 180, "hail", 12)
redact_hail(f, M + 8, 168, W - 2 * M - 16, 180)
D.text(f, M + 16, 358, "Golf-ball hail out of nowhere.", 16.5, "Semi Bold", INK)
D.text(f, M + 16, 381, "Today 4:12 PM · Denver area · posted with proof link", 12.5, "Regular", TX2)
SY = 448
dim_sheet(f, SY, "New case")
D.rect(f, M, SY + 72, W - 2 * M, 56, SURF, 12, stroke=PRIMARY, sw=1.5)
D.text(f, M + 16, SY + 80, "Name", 11.5, "Medium", PRIMARY)
D.text(f, M + 16, SY + 98, "Fender bender — Oak & 5th", 15, "Regular", INK)
D.rect(f, M + 16 + 198, SY + 98, 1.5, 19, PRIMARY)
overline(f, M + 8, SY + 152, "What happened?")
cw = (W - 2 * M - 8) / 2
for k, (s_, ic) in enumerate((("Car accident", "directions_car"), ("Property damage", "water_drop"), ("Rental home", "home"), ("Other", "edit_note"))):
    cx_, cy_ = M + (k % 2) * (cw + 8), SY + 176 + (k // 2) * 48
    on = k == 0
    D.rect(f, cx_, cy_, cw, 40, INK if on else None, 20, stroke=None if on else LINE, name="Chip " + s_)
    D.icon(f, ic, cx_ + 14, cy_ + 10, 20, WHITE if on else TX2)
    D.text(f, cx_ + 42, cy_ + 11, s_, 13.5, "Semi Bold" if on else "Medium", WHITE if on else INK)
generated_tag(f, M + 8, SY + 280, "Report suggested on-device")
D.text(f, M + 8, SY + 312, "Witness picks a report template from the name and your first photos. You can change it anytime.", 12.5, "Regular", TX2,
       w=W - 2 * M - 16, align="left", lh=1.4)
goto(button(f, M, SY + 368, W - 2 * M, 48, "Create case"), S_CAMCASE)
gesture(f)

f = screen(S["C4"])
app_bar(f, "Cases", right_icon="search")
x = M
for s_, on in (("All", True), ("Open", False), ("Shared", False)):
    if on:
        _, w_ = chip(f, x, 100, s_, WHITE, bg=INK, h=34, size=13)
    else:
        _, w_ = chip(f, x, 100, s_, TX2, h=34, size=13, style="Medium", outline=True)
    x += w_ + 8
for k, (imgs, title, meta, chips_) in enumerate(((("crash", "bumper", "plate"), "Fender bender", "Today · Oak St & 5th Ave · 4 items", (("anchored", "3"), ("queued", "1"))),
                                                 (("water",), "Kitchen ceiling leak", "Nov 12 · 6 items", (("anchored", "6"),)),
                                                 (("rental",), "Move-out walkthrough", "Nov 2 · 12 items · Expires Nov 25", (("anchored", "12"),)))):
    y = 152 + k * 212
    rc = card(f, M, y, W - 2 * M, 200)
    bx, bw = M + 8, W - 2 * M - 16
    if len(imgs) == 3:
        D.image(f, bx, y + 8, 240, 128, imgs[0], radii=[12, 2, 2, 12])
        D.image(f, bx + 244, y + 8, bw - 244, 62, imgs[1], radii=[2, 12, 2, 2])
        D.image(f, bx + 244, y + 74, bw - 244, 62, imgs[2], radii=[2, 2, 12, 2])
    else:
        D.image(f, bx, y + 8, bw, 128, imgs[0], 12)
    D.text(f, M + 16, y + 146, title, 15.5, "Semi Bold", INK)
    D.text(f, M + 16, y + 170, meta, 12.5, "Regular", TX2)
    xx = W - M - 16
    if k == 2:
        chip(f, bx + 8, y + 16, "Shared · 1 link", PRIMARY, bg=SURF, h=26, size=11.5, icon="link")
    for st, n_ in reversed(chips_):
        _, w_ = state_chip(f, xx, y + 145, st, right=True, h=24, size=11, text=n_)
        xx -= w_ + 6
    if k == 0:
        goto(rc, S_CASE)
fab = D.rect(f, W - M - 148, 824, 148, 56, PRIMARY, 16, name="New case FAB")
D.icon(f, "add", W - M - 130, 840, 24, ON_PRIMARY)
D.text(f, W - M - 98, 842, "New case", 14.5, "Semi Bold", ON_PRIMARY)
goto(fab, S["C3"])
gesture(f)

f = screen(S_CASE)
app_bar(f, "Fender bender", "Oak St & 5th Ave · Today 2:31 PM")
D.rect(f, M, 100, W - 2 * M, 40, AI, 12, a=0.08)
D.icon(f, "auto_awesome", M + 12, 110, 20, AI)
D.text(f, M + 40, 111, "Report built on-device · Car accident", 13, "Semi Bold", AI)
goto(D.text(f, M, 111, "Change", 13, "Semi Bold", AI, w=W - 3 * M + 4, align="right"), S["C6"])
overline(f, M, 156, "Evidence · 4 sealed")
D.text(f, M, 155, "3 anchored · 1 queued", 12, "Medium", MUTED, w=W - 2 * M, align="right")
for k, (img, st) in enumerate((("crash", "anchored"), ("bumper", "anchored"), ("plate", "anchored"), ("wide", "queued"))):
    tx = M + k * 92
    t = D.image(f, tx, 178, 84, 108, img, 12)
    seal_badge(f, tx + 72, 274, st, 9, ring=PAPER)
    goto(t, S_ITEM)
D.rect(f, M + 4 * 92, 178, 84, 108, AI, 12, a=0.06, stroke=AI, sw=1.2, sa=0.6, dashes=[5, 4])
D.icon(f, "add", M + 4 * 92 + 12, 220, 22, AI)
fade_right(f, W - 44, 176, 44, 112)

ck = 302
generated_surface(f, M, ck, W - 2 * M, 150)
ai_header(f, M + 18, ck + 16, "Still recommended", W - 2 * M - 18)
for k, (done, s_, meta) in enumerate(((True, "Damage close-up", "Photo 2"), (True, "Both license plates", "Photos 2 · 3"),
                                      (True, "Wide shot of both cars", "Photo 4"), (False, "Other driver's insurance card", None))):
    ry = ck + 42 + k * 26
    D.icon(f, "check_circle" if done else "add", M + 18, ry, 20, OK if done else PRIMARY)
    D.text(f, M + 46, ry + 1, s_, 13.5, "Medium" if done else "Semi Bold", TX2 if done else INK)
    if meta:
        D.text(f, M, ry + 2, meta, 12, "Regular", MUTED, w=W - 3 * M - 4, align="right")
    else:
        cap = D.text(f, M, ry + 1, "Capture", 13, "Semi Bold", PRIMARY, w=W - 3 * M - 4, align="right")
        goto(cap, S_CAMCASE)

fy = 466
D.text(f, M, fy, "Accident details", 16, "Semi Bold", INK)
generated_tag(f, W - M, fy - 2, "Generated form", right=True)
generated_surface(f, M, fy + 30, W - 2 * M, 236)
overline(f, M + 18, fy + 46, "Impact point")
D.text(f, M + 18, fy + 64, "Rear, driver side", 15.5, "Semi Bold", INK)
D.icon(f, "auto_awesome", M + 18, fy + 89, 14, AI)
D.text(f, M + 36, fy + 88, "Detected from photo 2", 12, "Medium", AI)
cxr, cyr = W - M - 96, fy + 40
D.rect(f, cxr, cyr, 44, 72, SURF2, 14, stroke=LINE, name="Car diagram")
D.rect(f, cxr + 7, cyr + 12, 30, 14, SURF3, 5)
D.rect(f, cxr + 7, cyr + 46, 30, 10, SURF3, 4)
D.rect(f, cxr, cyr + 54, 22, 18, CRIT, radii=[0, 0, 0, 14], a=0.28)
D.ellipse(f, cxr + 10, cyr + 64, 4, CRIT)
flag(f, W - M - 38, fy + 44)
goto(D.rect(f, W - M - 44, fy + 38, 30, 30, None, name="Flag impact point"), S["C7"])
D.path(f, [(M + 18, fy + 118), (W - M - 18, fy + 118)], LINE, 1)
overline(f, M + 18, fy + 130, "Other driver's plate")
D.rect(f, M + 18, fy + 148, 180, 40, SURF2, 10)
D.text(f, M + 30, fy + 159, "7MNV562", 15, "Bold", INK, font=MONO)
chip(f, M + 206, fy + 155, "Read from photo 3", AI, h=26, size=11, icon="auto_awesome")
overline(f, M + 18, fy + 202, "Anyone injured?")
sw_ = (W - 2 * M - 36) / 3
for k, (s_, on) in enumerate((("No", True), ("Yes", False), ("Not sure", False))):
    sx = M + 18 + k * sw_
    D.rect(f, sx + 2, fy + 220, sw_ - 4, 36, INK if on else None, 18, stroke=None if on else LINE)
    D.label(f, sx + 2, fy + 220, sw_ - 4, 36, s_, 13, "Semi Bold" if on else "Medium", WHITE if on else TX2)

sy = 744
generated_surface(f, M, sy, W - 2 * M, 80)
ai_header(f, M + 18, sy + 13, "Draft summary", W - 2 * M - 70)
D.text(f, M, sy + 11, "Edit", 12.5, "Semi Bold", PRIMARY, w=W - 2 * M - 16, align="right")
D.text(f, M + 18, sy + 34, "At 2:31 PM a silver sedan struck the rear of my stopped hatchback at the Oak St stop sign. No injuries.", 12.5,
       "Regular", TX2, w=W - 2 * M - 34, align="left", lh=1.4)
D.rect(f, 0, 834, W, 81, SURF)
D.path(f, [(0, 834), (W, 834)], LINE, 1)
goto(button(f, M, 848, 128, 48, "Export ZIP", "tonal"), S["C8"])
shb = button(f, M + 140, 848, W - 2 * M - 140, 48, "Share case", "filled", icon="share")
goto(shb, S_SHARE)
gesture(f)

f = screen(S["C6"])
case_under(f)
SY = 424
dim_sheet(f, SY, "Change report type")
generated_tag(f, M + 8, SY + 68, "AI suggested: Car accident")
for k, (ic, title, sub, on, muted) in enumerate((("directions_car", "Car accident", "Plates, damage, other driver, injuries", True, False),
                                                  ("water_drop", "Property damage", "Source, affected areas, repair notes", False, False),
                                                  ("home", "Rental condition", "Room by room, move-in vs move-out", False, False),
                                                  ("badge", "Workplace incident", "Coming in a later update", False, True),
                                                  ("check_box", "Other", "Plain checklist, no form", False, False))):
    ry = SY + 108 + k * 60
    row = option_row(f, ry, ic, title, sub, on, muted, trailing=None if muted else "radio")
    if muted:
        chip(f, W - M - 12, ry + 15, "Later", TX2, h=26, size=11.5, outline=True, right=True)
    else:
        goto(row, S_CASE)
D.icon(f, "info", M + 8, SY + 422, 16, MUTED)
D.text(f, M + 30, SY + 421, "Changing type keeps your photos and moves\nmatching details.", 12.5, "Regular", MUTED,
       w=W - 2 * M - 38, align="left", lh=1.4)
gesture(f)

f = screen(S["C7"])
case_under(f)
SY = 320
dim_sheet(f, SY, "What's wrong with this?")
generated_surface(f, M, SY + 72, W - 2 * M, 72, name="Flagged item")
D.icon(f, "auto_awesome", M + 18, SY + 87, 16, AI)
overline(f, M + 40, SY + 89, "Impact point", AI)
D.text(f, M + 18, SY + 110, "Rear, driver side", 15.5, "Semi Bold", INK)
D.icon(f, "flag", W - M - 36, SY + 98, 20, INK)
overline(f, M + 8, SY + 164, "Reason")
for k, (s_, hint) in enumerate((("Wrong", "Doesn't match the photos"), ("Misleading", "Right facts, wrong impression"),
                                ("Offensive", None), ("Other", None))):
    ry = SY + 184 + k * 44
    on = k == 0
    radio(f, M + 4, ry + 10, on)
    D.text(f, M + 40, ry + 12, s_, 14.5, "Semi Bold" if on else "Medium", INK)
    if hint:
        D.text(f, M + 48 + est_width(s_, 14.5, "Semi Bold" if on else "Medium"), ry + 14, hint, 12.5, "Regular", MUTED)
D.rect(f, M, SY + 368, W - 2 * M, 88, SURF, 12, stroke=LINE)
D.text(f, M + 16, SY + 384, "Tell us more (optional)", 14, "Regular", MUTED)
D.icon(f, "lock", M + 8, SY + 472, 16, MUTED)
D.text(f, M + 30, SY + 472, "Flags stay on your phone unless you export feedback.", 12, "Regular", MUTED)
bw = (W - 2 * M - 12) / 2
rm = D.rect(f, M, SY + 504, bw, 48, None, 24, stroke=LINE, name="Button Remove it")
tw_ = est_width("Remove it", 14.5, "Semi Bold") + 26
D.icon(f, "delete", M + (bw - tw_) / 2, SY + 518, 20, CRIT)
D.text(f, M + (bw - tw_) / 2 + 26, SY + 519, "Remove it", 14.5, "Semi Bold", CRIT)
goto(rm, S_CASE)
goto(button(f, M + bw + 12, SY + 504, bw, 48, "Send flag", icon="send"), S_CASE)
gesture(f)

f = screen(S["C8"])
case_under(f)
SY = 456
dim_sheet(f, SY, "Export case", "Fender bender · 4 items")
for k, (ic, title, tag, tag_ic, sub, on) in enumerate((("folder_zip", "ZIP of sealed files + proof records", "Free", None, "Anyone can check them", True),
                                                       ("picture_as_pdf", "PDF report", "Plus", "workspace_premium", "Formatted report with photos", False))):
    oy = SY + 92 + k * 92
    oc = D.rect(f, M, oy, W - 2 * M, 80, PRIMARY if on else None, 16, a=0.04, stroke=PRIMARY if on else LINE, sw=1.5 if on else 1.0, name="Option " + title)
    D.rect(f, M + 12, oy + 20, 40, 40, PRIMARY if on else SURF2, 12, a=0.12 if on else 1.0)
    D.icon(f, ic, M + 20, oy + 28, 24, PRIMARY if on else MUTED)
    D.text(f, M + 64, oy + 16, title, 14, "Semi Bold", INK if on else TX2)
    _, cw_ = chip(f, M + 64, oy + 40, tag, TX2 if on else INK, h=22, size=11, icon=tag_ic, a=0.1)
    D.text(f, M + 72 + cw_, oy + 43, sub, 12, "Regular", TX2 if on else MUTED)
    if on:
        radio(f, W - M - 40, oy + 28, True)
    else:
        D.icon(f, "lock", W - M - 38, oy + 30, 20, MUTED)
        goto(oc, S["E2"])
D.text(f, M + 8, SY + 284, "Include exact location", 14.5, "Medium", INK)
toggle(f, W - M - 54, SY + 280, False)
D.text(f, M + 8, SY + 310, "Off: location is rounded to the neighborhood.", 12, "Regular", MUTED)
goto(button(f, M, SY + 360, W - 2 * M, 48, "Export ZIP", icon="folder_zip"), S_CASE)
gesture(f)

f = screen(S_SHARE)
app_bar(f, "Share case", "Fender bender · 4 items", right_icon=None, lead="close")
overline(f, M, 104, "Items")
D.text(f, M, 103, "4 of 4 selected", 12, "Medium", MUTED, w=W - 2 * M, align="right")
for k, img in enumerate(("crash", "bumper", "plate", "wide")):
    tx = M + k * 96
    if img == "wide":
        wide_thumb(f, tx, 126, 88, 116, redacted=True)
        chip(f, tx + 6, 212, "Edited", WHITE, bg=EDIT, h=22, size=10.5, icon="edit")
    else:
        D.image(f, tx, 126, 88, 116, img, 12)
    D.ellipse(f, tx + 74, 140, 11, INK, stroke=WHITE, sw=2)
    D.icon(f, "check", tx + 66, 132, 16, WHITE)
overline(f, M, 262, "Privacy")
x = M
for s_, on, ic in (("Blur faces · 1", True, "face"), ("Blur plates", False, "directions_car"), ("Hide location", False, "location_on")):
    if on:
        _, w_ = chip(f, x, 284, s_, EDIT, h=34, size=12.5, icon=ic, a=0.12)
    else:
        _, w_ = chip(f, x, 284, s_, TX2, h=34, size=12.5, icon=ic, style="Medium", outline=True)
    x += w_ + 8
D.icon(f, "info", M, 330, 16, MUTED)
D.text(f, M + 22, 329, "Edits are declared — recipients see exactly what changed.", 12, "Regular", MUTED)

card(f, M, 362, W - 2 * M, 132)
overline(f, M + 16, 378, "Link expires")
sw_ = (W - 2 * M - 32) / 4
D.rect(f, M + 16, 398, W - 2 * M - 32, 38, SURF2, 19)
for k, (s_, on) in enumerate((("24 h", False), ("7 days", True), ("30 days", False), ("Never", False))):
    sx = M + 16 + k * sw_
    if on:
        D.rect(f, sx + 3, 401, sw_ - 6, 32, SURF, 16, stroke=LINE)
    D.label(f, sx, 398, sw_, 38, s_, 13, "Semi Bold" if on else "Medium", INK if on else TX2)
D.path(f, [(M + 16, 450), (W - M - 16, 450)], LINE, 1)
D.text(f, M + 16, 462, "Include AI summary", 14, "Medium", INK)
toggle(f, W - M - 62, 458, True)

card(f, M, 508, W - 2 * M, 312)
qr(f, M + 24, 532, 112)
D.text(f, M + 160, 530, "Private case link", 15, "Semi Bold", INK)
D.text(f, M + 160, 554, f"case.{DOMAIN}/v/\n8F2K-QM7D#k=••••", 12.5, "Regular", PRIMARY, font=MONO, lh=1.35)
D.text(f, M + 160, 594, "Expires Nov 25 · 4 items", 12, "Regular", MUTED)
D.rect(f, M + 160, 620, 108, 30, SURF2, 15)
D.icon(f, "content_copy", M + 172, 626, 18, INK)
D.text(f, M + 196, 626, "Copy link", 12.5, "Semi Bold", INK)
D.rect(f, M + 16, 664, W - 2 * M - 32, 38, OK, 12, a=0.08)
D.icon(f, "lock", M + 28, 674, 18, OK)
D.text(f, M + 54, 674, "End-to-end encrypted · the key stays in the link", 12.5, "Semi Bold", OK_T)
D.path(f, [(M + 16, 716), (W - M - 16, 716)], LINE, 1)
overline(f, M + 16, 728, "Recipients will see")
D.icon(f, "check_circle", M + 16, 752, 20, OK)
D.text(f, M + 44, 752, "4 of 4 verified · 1 declared edit", 13.5, "Semi Bold", OK_T)
pv = D.text(f, M, 784, "Preview  →", 12.5, "Semi Bold", PRIMARY, w=W - 2 * M - 16, align="right")
goto(pv, S_CASELINK)
D.rect(f, 0, 834, W, 81, SURF)
D.path(f, [(0, 834), (W, 834)], LINE, 1)
button(f, M, 848, 150, 48, "Export file", "tonal", icon="description")
sl = button(f, M + 162, 848, W - 2 * M - 162, 48, "Share link", "filled", icon="link")
goto(sl, S_CASELINK)
gesture(f)
