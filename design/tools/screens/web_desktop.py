from wlib import *


def desk_chrome(f, url):
    D.rect(f, 0, 0, DW, 48, SURF2)
    for k, c in enumerate(("#E5534B", "#E5A53B", "#4CB050")):
        D.ellipse(f, 22 + k * 20, 24, 6, c)
    D.rect(f, 120, 10, 640, 28, SURF, 14)
    D.icon(f, "lock", 134, 16, 16, TX2)
    D.text(f, 156, 16, url, 13, "Regular", INK)
    D.path(f, [(0, 48), (DW, 48)], LINE, 1)
    wordmark(f, 64, 76, 22)


def desk_nav(f):
    return D.text(f, DW - 64 - 300, 82, "Check a copy        About        Report", 13.5, "Semi Bold", TX2, w=300, align="right")


def desk_footer(f):
    D.path(f, [(64, 770), (DW - 64, 770)], LINE, 1)
    D.text(f, 64, 786, "Report this page · Contact · Terms", 12.5, "Semi Bold", TX2)
    D.text(f, DW - 64 - 420, 787, "Served from an ICP canister · certified response", 11.5, "Regular", MUTED, w=420, align="right", font=MONO)


f = screen(S_DESKTOP)
desk_chrome(f, f"{DOMAIN}/p/H4IL-7QK")
goto(desk_nav(f), S["X2"])
D.image(f, 64, 136, 600, 600, "hail", 20)
redact_hail(f, 64, 136, 600, 600)
chip(f, 80, 152, "Matches an original", WHITE, bg=EDIT, h=30, size=13, icon="compare")
chip(f, 80, 136 + 600 - 46, "2 declared blurs", WHITE, bg=EDIT, h=28, size=12, icon="edit")
RX = 712
D.text(f, RX, 136, "Matches an original", 34, "Bold", EDIT, ls=-1.5)
D.text(f, RX, 184, "with 2 declared blurs", 18, "Semi Bold", EDIT)
D.text(f, RX, 216, "Captured on a genuine phone. Only the 2 declared blurs differ.", 15, "Regular", INK, w=504, align="left")
D.icon(f, "trending_up", RX, 248, 20, OK)
D.text(f, RX + 28, 248, "First seen here · checked 1,284 times · anonymous", 14, "Semi Bold", OK_T)
card(f, RX, 280, 504, 210)
for k, (ic, t, s_) in enumerate((("schedule", "Captured", "Nov 18, 2026 · 4:12:09 PM · GNSS time"), ("near_me", "Location", "Denver area · precise location sealed"),
                                  ("key", "Device", "Android phone · StrongBox · Key Attestation"), ("verified", "Anchored", "4:13 PM · Internet Computer · certified"))):
    ry = 296 + k * 48
    if k:
        D.path(f, [(RX + 20, ry - 8), (RX + 484, ry - 8)], LINE, 1)
    D.icon(f, ic, RX + 20, ry + 4, 22, TX2)
    D.text(f, RX + 56, ry + 6, t, 14.5, "Semi Bold", INK)
    D.text(f, RX + 160, ry + 7, s_, 13.5, "Regular", TX2)
    D.icon(f, "chevron_right", RX + 470, ry + 4, 22, MUTED)
proves_card(f, RX, 506, 504)
dz = D.rect(f, RX, 650, 504, 86, SURF, 18, stroke=MUTED, sw=1.2, sa=0.7, dashes=[6, 5], name="Drop zone")
D.icon(f, "photo_library", RX + 22, 680, 26, TX2)
D.text(f, RX + 62, 672, "Check a copy", 15, "Semi Bold", INK)
D.text(f, RX + 62, 694, "Drop a screenshot here or paste a link — checked in your browser", 13, "Regular", TX2)
goto(dz, S["X2"])
desk_footer(f)

f = screen(S["X2"])
desk_chrome(f, f"{DOMAIN}/check")
desk_nav(f)
LX, LW = 64, 552
D.rect(f, LX, 136, LW, 600, SURF, 20, stroke=MUTED, sw=1.2, sa=0.7, dashes=[6, 5], name="Drop zone")
IX, IS = LX + 56, 440
D.image(f, IX, 160, IS, IS, "hail", 14, name="Dropped screenshot")
redact_hail(f, IX, 160, IS, IS)
D.rect(f, IX, 616, IS, 44, SURF2, 12, name="File")
D.icon(f, "image", IX + 14, 628, 20, TX2)
D.text(f, IX + 44, 629, "IMG_4021.jpg · 1.2 MB", 13.5, "Semi Bold", INK)
D.text(f, IX + 200, 631, "1080 × 1080 · JPEG", 11.5, "Regular", MUTED, w=IS - 248, align="right", font=MONO)
D.icon(f, "close", IX + IS - 34, 628, 20, TX2)
D.text(f, LX, 680, "Drop another screenshot, paste an image or a link", 14, "Semi Bold", INK, w=LW, align="center")
D.text(f, LX, 704, "Checked in your browser · the file is never uploaded", 12.5, "Regular", TX2, w=LW, align="center")
RX, RW = 664, 552
D.rect(f, RX, 136, RW, 144, EDIT, 20, a=0.07, stroke=EDIT, sw=1.2, sa=0.45, name="Verdict")
D.icon(f, "compare", RX + 20, 156, 34, EDIT)
D.text(f, RX + 66, 152, "Matches an original", 28, "Bold", EDIT, ls=-1)
D.text(f, RX + 66, 190, "with changes", 16, "Semi Bold", EDIT)
D.text(f, RX + 20, 224, "Traces back to a sealed capture from Nov 18. This copy was cropped and re-saved as a screenshot; the sender declared 2 blurs.",
       14, "Regular", INK, w=RW - 40, align="left", lh=1.4)
card(f, RX, 296, RW, 216)
overline(f, RX + 16, 310, "This screenshot vs. sealed reference")
TY, TH = 334, 140
D.image(f, RX + 16, TY, TH, TH, "hail", 10)
redact_hail(f, RX + 16, TY, TH, TH)
D.icon(f, "compare", RX + 168, TY + 58, 24, MUTED)
RW_ = TH * 3 / 4
D.image(f, RX + 204, TY, RW_, TH, "hail", 10)
redact_hail(f, RX + 204, TY, RW_, TH)
D.rect(f, RX + 204, TY + (TH - RW_) / 2, RW_, RW_, None, 2, stroke=WHITE, sw=1.5, dashes=[4, 3], name="Crop")
for x_, lab, c in ((RX + 16, "Screenshot", EDIT), (RX + 204, "Reference", OK)):
    D.ellipse(f, x_ + 4, TY + TH + 20, 4, c)
    D.text(f, x_ + 14, TY + TH + 12, lab, 12.5, "Semi Bold", INK)
D.path(f, [(RX + 332, TY), (RX + 332, TY + TH + 28)], LINE, 1)
for k, (t, v, mono_) in enumerate((("Matched by", "Watermark + fingerprint", False), ("Record", "H4IL-7QK…", True), ("Aligned", "92% of reference frame", False))):
    ry = TY + 2 + k * 56
    D.text(f, RX + 352, ry, t, 11.5, "Medium", MUTED)
    D.text(f, RX + 352, ry + 18, v, 13, "Semi Bold", INK, font=MONO if mono_ else SANS)
card(f, RX, 528, RW, 128)
overline(f, RX + 16, 542, "What changed")
for k, (ic, t, who, c) in enumerate((("crop", "Cropped to a square", "This copy", TX2), ("image", "Re-saved as a screenshot JPEG", "This copy", TX2),
                                     ("blur_on", "2 blurs · house number, plate", "Declared by sender", EDIT))):
    ry = 566 + k * 28
    D.icon(f, ic, RX + 16, ry, 18, EDIT if c == EDIT else TX2)
    D.text(f, RX + 44, ry + 1, t, 13.5, "Regular", INK)
    D.text(f, RX + RW - 216, ry + 2, who, 12.5, "Semi Bold", c, w=200, align="right")
D.icon(f, "schedule", RX, 668, 18, TX2)
D.text(f, RX + 26, 669, "Captured Nov 18, 2026 · 4:12 PM · Denver area · genuine Android phone", 13, "Regular", TX2)
goto(button(f, RX, 696, 204, 40, "Open proof page", icon="open_in_new"), S_DESKTOP)
button(f, RX + 216, 696, 160, 40, "Check another", "outlined")
desk_footer(f)

f = screen(S["X3"])
desk_chrome(f, "case.witness.app/v/8F2K-QM7D#k=••••")
_, cw_ = chip(f, DW - 64, 78, "Private case", TX2, h=30, size=13, icon="lock", right=True)
D.text(f, DW - 64 - cw_ - 312, 85, "End-to-end encrypted · expires Nov 25", 13, "Regular", TX2, w=300, align="right")
LX, LW = 64, 440
D.text(f, LX, 136, "Fender bender", 30, "Bold", INK, ls=-1)
D.text(f, LX, 180, "Shared Nov 18 · 4 items · expires Nov 25", 14, "Regular", TX2)
D.rect(f, LX, 216, LW, 112, OK, 20, a=0.08, stroke=OK, sw=1.2, sa=0.45, name="Verdict")
D.icon(f, "check_circle", LX + 20, 236, 34, OK)
D.text(f, LX + 66, 234, "4 of 4 verified", 28, "Bold", OK_T, ls=-1)
D.text(f, LX + 20, 284, "3 original · 1 with a declared edit (face blurred).", 14.5, "Regular", INK)
generated_surface(f, LX, 344, LW, 176)
D.icon(f, "auto_awesome", LX + 18, 360, 16, AI)
overline(f, LX + 40, 362, "Sender's summary · AI-drafted", AI)
D.text(f, LX + 18, 386, "At 2:31 PM on Nov 18 a silver sedan struck the rear of a stopped hatchback at the Oak St & 5th Ave stop sign. "
       "No injuries. Both plates, 8KTR214 and 7MNV562, are photographed.", 13.5, "Regular", TX2, w=LW - 36, align="left", lh=1.45)
D.text(f, LX + 18, 490, "Drafted by the sender's on-device model · not part of the proof", 11.5, "Regular", MUTED, w=LW - 36, align="left")
card(f, LX, 536, LW, 200)
overline(f, LX + 16, 552, "About this link")
for k, (ic, t, s_) in enumerate((("lock", "End-to-end encrypted", "The key lives in the link, never on Witness servers"),
                                  ("schedule", "Expires Nov 25, 2026", "Then the photos are deleted from Witness"),
                                  ("verified_user", "Checked in your browser", "Each item is verified against its anchor"))):
    ry = 580 + k * 52
    if k:
        D.path(f, [(LX + 16, ry - 10), (LX + LW - 16, ry - 10)], LINE, 1)
    D.icon(f, ic, LX + 16, ry + 2, 20, TX2)
    D.text(f, LX + 48, ry, t, 13.5, "Semi Bold", INK)
    D.text(f, LX + 48, ry + 20, s_, 12.5, "Regular", TX2)
RX, RW = 544, 672
CW, CH = (RW - 16) / 2, 228
for k, (img, t, meta, st) in enumerate((("crash", "Intersection", "Photo 1 · 2:31:40 PM", "original"), ("bumper", "Rear damage", "Photo 2 · 2:31:47 PM", "original"),
                                        ("plate", "Other plate", "Photo 3 · 2:31:52 PM", "original"), ("wide", "Wide shot", "Photo 4 · 2:34:05 PM", "edited"))):
    cx, cy = RX + (k % 2) * (CW + 16), 136 + (k // 2) * (CH + 16)
    card(f, cx, cy, CW, CH)
    if img == "wide":
        wide_thumb(f, cx + 8, cy + 8, CW - 16, 148, r=12, redacted=True)
    else:
        D.image(f, cx + 8, cy + 8, CW - 16, 148, img, 12)
    D.text(f, cx + 16, cy + 170, t, 14.5, "Semi Bold", INK)
    D.text(f, cx + 16, cy + 192, meta, 12.5, "Regular", TX2)
    state_chip(f, cx + CW - 16, cy + 176, st, right=True, text="Face blurred" if st == "edited" else None)
proves_card(f, RX, 624, RW, ((True, "Genuine phone, real Witness app"), (True, "Unaltered, except declared edits"),
                             (False, "Doesn't prove the scene wasn't staged")))
desk_footer(f)
