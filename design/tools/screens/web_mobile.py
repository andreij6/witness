from wlib import *

f = screen(S_PROOFPAGE)
browser(f, f"{DOMAIN}/p/H4IL-7QK")
D.image(f, M, 146, W - 2 * M, 214, "hail", 18)
redact_hail(f, M, 146, W - 2 * M, 214)
chip(f, M + 12, 158, "Matches an original", WHITE, bg=EDIT, h=28, size=12, icon="compare")
chip(f, M + 12, 360 - 40, "2 declared blurs", WHITE, bg=INK, h=26, size=11.5, icon="edit")
D.text(f, M, 372, "Matches an original", 24, "Bold", EDIT, ls=-1)
D.text(f, M, 406, "with 2 declared blurs", 14, "Semi Bold", EDIT)
D.text(f, M + est_width("with 2 declared blurs", 14, "Semi Bold") + 8, 407, "· the photographer's own edits", 13.5, "Regular", TX2)
card(f, M, 446, W - 2 * M, 176)
for k, (ic, t, s_) in enumerate((("schedule", "Captured", "Nov 18, 2026 · 4:12 PM · GNSS time"), ("near_me", "Location", "Denver area · precise sealed"),
                                  ("key", "Device", "Android phone · StrongBox · attested"), ("verified", "Anchored", "4:13 PM · Internet Computer"))):
    ry = 458 + k * 40
    if k:
        D.path(f, [(M + 16, ry - 6), (W - M - 16, ry - 6)], LINE, 1)
    D.icon(f, ic, M + 16, ry + 4, 20, TX2)
    D.text(f, M + 46, ry + 5, t, 13.5, "Semi Bold", INK)
    D.text(f, M + 128, ry + 6, s_, 12, "Regular", TX2)
    D.icon(f, "chevron_right", W - M - 34, ry + 3, 22, MUTED)
D.icon(f, "trending_up", M, 634, 18, OK)
D.text(f, M + 24, 634, "First seen here · checked 1,284 times · anonymous", 12.5, "Semi Bold", OK_T)
proves_card(f, M, 664, W - 2 * M)
web_footer(f, 812)
goto(D.rect(f, M - 4, 818, est_width("Report this page", 12, "Semi Bold") + 28, 30, PAPER, 8, a=0.01, name="Hotspot Report"), S["W10"])
gesture(f)

f = screen(S_ALTERED)
browser(f, f"{DOMAIN}/check")
VY = 148
D.rect(f, M, VY, W - 2 * M, 136, CRIT, 20, a=0.07, stroke=CRIT, sw=1.2, sa=0.45, name="Verdict")
D.icon(f, "error", M + 18, VY + 18, 34, CRIT)
D.text(f, M + 62, VY + 18, "Altered", 26, "Bold", CRIT, ls=-1)
D.text(f, M + 18, VY + 62, "This copy matches a sealed capture, but one region was changed. The sealed reference is authentic — this copy is not.",
       13.5, "Regular", INK, w=W - 2 * M - 36, align="left", lh=1.4)
card(f, M, 296, W - 2 * M, 300)
overline(f, M + 16, 310, "Sealed reference vs. this file")
tw_ = (W - 2 * M - 44) / 2
th_ = tw_ * 4 / 3
for k, (lab, c) in enumerate((("Reference · Nov 18", OK), ("This file", CRIT))):
    tx = M + 16 + k * (tw_ + 12)
    D.image(f, tx, 334, tw_, th_, "plate", 12)
    px, py, pw, ph = tx + tw_ * 0.44, 334 + th_ * 0.55, tw_ * 0.18, th_ * 0.065
    if k:
        D.rect(f, px - 2, py, pw + 4, ph, "#F6F6F2", 1)
        D.label(f, px - 4, py, pw + 8, ph, "7MNV862", 6.4, "Bold", "#2B3A55", MONO)
        D.rect(f, px - 6, py - 6, pw + 12, ph + 12, None, 4, stroke=CRIT, sw=2)
    D.ellipse(f, tx + 14, 334 + th_ + 18, 4, c)
    D.text(f, tx + 24, 334 + th_ + 10, lab, 12.5, "Semi Bold", INK)
card(f, M, 608, W - 2 * M, 176)
for k, (ok_, t, s_) in enumerate(((False, "Content", "Differs in 1 region · plate area"), (True, "Match", "Watermark ID + fingerprint"),
                                  (True, "Device", "Genuine Android phone · attested"), (True, "Anchored", "Nov 18, 2:36 PM"))):
    ry = 622 + k * 40
    if k:
        D.path(f, [(M + 16, ry - 8), (W - M - 16, ry - 8)], LINE, 1)
    D.icon(f, "check_circle" if ok_ else "error", M + 16, ry + 4, 20, OK if ok_ else CRIT)
    D.text(f, M + 46, ry + 5, t, 13.5, "Semi Bold", INK)
    D.text(f, M + 122, ry + 6, s_, 12.5, "Regular", CRIT if not ok_ else TX2)
web_footer(f, 812)
gesture(f)

f = screen(S_NORECORD)
browser(f, f"{DOMAIN}/check")
D.image(f, M, 146, W - 2 * M, 236, "random", 18)
VY = 396
D.rect(f, M, VY, W - 2 * M, 128, SURF2, 20, stroke=MUTED, sw=1.2, sa=0.4, name="Verdict")
D.icon(f, "help", M + 16, VY + 16, 30, TX2)
D.text(f, M + 56, VY + 14, "No Witness record", 22, "Bold", INK, ls=-1)
D.text(f, M + 16, VY + 56, "We found no sealed capture for this image. That doesn't make it fake — it just wasn't proven with Witness, or its proof was removed.",
       13.5, "Regular", INK, w=W - 2 * M - 32, align="left", lh=1.4)
card(f, M, 536, W - 2 * M, 160)
overline(f, M + 16, 550, "What we checked")
for k, (t, s_) in enumerate((("Watermark", "None found"), ("Fingerprints", "No match in 2.1M captures"), ("Credentials", "No C2PA data in this file"))):
    ry = 574 + k * 38
    D.icon(f, "search", M + 16, ry + 2, 18, TX2)
    D.text(f, M + 44, ry + 2, t, 13.5, "Semi Bold", INK)
    D.text(f, M + 160, ry + 3, s_, 12.5, "Regular", TX2)
D.rect(f, M, 708, W - 2 * M, 84, SURF, 18, stroke=LINE)
D.text(f, M + 16, 722, "Saw this online?", 14, "Semi Bold", INK)
D.text(f, M + 16, 744, "Ask the poster for a Witness proof link — originals captured with Witness always have one.", 12.5, "Regular", TX2,
       w=W - 2 * M - 32, align="left", lh=1.4)
web_footer(f, 812)
gesture(f)

f = screen(S_CASELINK)
browser(f, f"case.{DOMAIN}/v/8F2K-QM7D#k=••••", "Private case")
D.text(f, M, 146, "Fender bender", 20, "Semi Bold", INK)
D.text(f, M, 172, "Shared Nov 18 · 4 items · expires Nov 25", 12.5, "Regular", TX2)
VY = 200
D.rect(f, M, VY, W - 2 * M, 96, OK, 20, a=0.08, stroke=OK, sw=1.2, sa=0.45, name="Verdict")
D.icon(f, "check_circle", M + 16, VY + 16, 30, OK)
D.text(f, M + 56, VY + 14, "4 of 4 verified", 22, "Bold", OK_T, ls=-1)
D.text(f, M + 16, VY + 56, "3 original · 1 with a declared edit (face blurred).", 13.5, "Regular", INK)
sy_ = 308
generated_surface(f, M, sy_, W - 2 * M, 96)
D.icon(f, "auto_awesome", M + 18, sy_ + 14, 16, AI)
overline(f, M + 40, sy_ + 16, "Sender's summary · AI-drafted", AI)
D.text(f, M + 18, sy_ + 38, "At 2:31 PM a silver sedan struck the rear of a stopped hatchback at the Oak St stop sign. No injuries.", 12.5,
       "Regular", TX2, w=W - 2 * M - 34, align="left", lh=1.4)
card(f, M, 416, W - 2 * M, 236)
for k, (img, t, meta, st) in enumerate((("crash", "Photo 1 · Intersection", "2:31:40 PM · location high", "original"),
                                        ("bumper", "Photo 2 · Rear damage", "2:31:47 PM · location high", "original"),
                                        ("plate", "Photo 3 · Other plate", "2:31:52 PM · location high", "original"),
                                        ("wide", "Photo 4 · Wide shot", "2:34:05 PM · face blurred", "edited"))):
    ry = 426 + k * 56
    if k:
        D.path(f, [(M + 16, ry - 4), (W - M - 16, ry - 4)], LINE, 1)
    if img == "wide":
        wide_thumb(f, M + 14, ry + 2, 40, 44, r=8, redacted=True)
    else:
        D.image(f, M + 14, ry + 2, 40, 44, img, 8)
    D.text(f, M + 66, ry + 6, t, 13.5, "Semi Bold", INK)
    D.text(f, M + 66, ry + 26, meta, 11.5, "Regular", TX2)
    s__, c_, ic_ = STATES[st]
    D.icon(f, ic_, W - M - 62, ry + 13, 20, c_)
    D.icon(f, "chevron_right", W - M - 38, ry + 12, 22, MUTED)
proves_card(f, M, 664, W - 2 * M, ((True, "Genuine phone, real Witness app"), (True, "Unaltered, except declared edits"),
                                     (False, "Doesn't prove the scene wasn't staged")))
web_footer(f, 812)
gesture(f)

f = screen(S["W2"])
browser(f, f"{DOMAIN}/check", "Check a photo")
D.text(f, M, 146, "Is this photo real?", 24, "Bold", INK, ls=-1)
D.text(f, M, 180, "Compare any photo with 2.1M sealed Witness captures.", 13.5, "Regular", TX2)
D.rect(f, M, 208, W - 2 * M, 216, SURF, 20, stroke=MUTED, sw=1.2, sa=0.6, dashes=[6, 5], name="Drop zone")
D.icon(f, "photo_library", W / 2 - 18, 232, 36, TX2)
D.text(f, 0, 284, "Choose a photo or screenshot", 16, "Semi Bold", INK, w=W, align="center")
D.text(f, 0, 308, "Checked in your browser — nothing is uploaded", 12.5, "Regular", TX2, w=W, align="center")
goto(button(f, W / 2 - 88, 344, 176, 48, "Choose photo", "ink", icon="upload"), S["W3"])
D.rect(f, M, 440, W - 2 * M, 48, SURF2, 24)
D.icon(f, "link", M + 16, 453, 22, TX2)
D.text(f, M + 48, 455, "Paste a post or proof link", 14, "Regular", MUTED)
D.text(f, M, 456, "Paste", 13, "Semi Bold", PRIMARY, w=W - 2 * M - 18, align="right")
card(f, M, 504, W - 2 * M, 204)
overline(f, M + 16, 520, "How it works")
for k, (ic, t, s_) in enumerate((("fingerprint", "Fingerprint", "Survives screenshots and recompression"),
                                  ("tag", "Watermark", "Reads the invisible ID in Witness photos"),
                                  ("compare", "Compare", "Region by region against the sealed original"))):
    ry = 548 + k * 52
    D.ellipse(f, M + 34, ry + 18, 18, SURF2)
    D.icon(f, ic, M + 24, ry + 8, 20, INK)
    D.text(f, M + 64, ry + 1, f"{k + 1}. {t}", 13.5, "Semi Bold", INK)
    D.text(f, M + 64, ry + 20, s_, 12, "Regular", TX2)
D.icon(f, "privacy_tip", M, 726, 18, MUTED)
D.text(f, M + 26, 725, "Your photo stays on this device. Only a short fingerprint is sent to search.", 12.5, "Regular", MUTED,
       w=W - 2 * M - 26, align="left", lh=1.4)
web_footer(f, 812)
gesture(f)

f = screen(S["W3"])
browser(f, f"{DOMAIN}/check", "Check a photo")
card(f, M, 146, W - 2 * M, 104)
D.image(f, M + 12, 158, 80, 80, "hail", 12)
redact_hail(f, M + 12, 158, 80, 80)
D.text(f, M + 108, 164, "Screenshot_1119-0942.png", 14, "Semi Bold", INK)
D.text(f, M + 108, 186, "1.6 MB · 1080 × 1080 · PNG", 12, "Regular", TX2)
chip(f, M + 108, 208, "Stays in your browser", TX2, h=22, size=10.5, icon="lock")
D.text(f, M, 274, "Checking…", 22, "Bold", INK, ls=-0.5)
D.text(f, M, 304, "Usually under 5 seconds", 13, "Regular", TX2)
stg = card(f, M, 336, W - 2 * M, 296)
for k, (st, t, s_) in enumerate((("done", "Reading file", "PNG · 1080 × 1080"),
                                 ("done", "Content Credentials", "None in this file"),
                                 ("active", "Fingerprints", "Searching 3 shards…"),
                                 ("pending", "Watermark", "Loads 12 MB only if needed"),
                                 ("pending", "Comparing", "Waiting"))):
    ry = 352 + k * 56
    if k:
        D.path(f, [(M + 16, ry - 8), (W - M - 16, ry - 8)], LINE, 1)
    ic, c = {"done": ("check_circle", INK), "active": ("sync", PRIMARY), "pending": ("radio_button_unchecked", MUTED)}[st]
    D.icon(f, ic, M + 16, ry + 6, 22, c)
    D.text(f, M + 50, ry + 2, t, 14, "Semi Bold", MUTED if st == "pending" else INK)
    D.text(f, M + 50, ry + 22, s_, 12, "Regular", PRIMARY if st == "active" else MUTED if st == "pending" else TX2)
    if st == "active":
        D.text(f, M, ry + 4, "2 of 3", 12, "Semi Bold", PRIMARY, w=W - 2 * M - 16, align="right")
        D.rect(f, W - M - 112, ry + 26, 96, 4, SURF3, 2)
        D.rect(f, W - M - 112, ry + 26, 64, 4, PRIMARY, 2)
goto(stg, S["W4"])
D.rect(f, M, 648, W - 2 * M, 68, SURF2, 18)
D.icon(f, "fingerprint", M + 16, 664, 20, TX2)
D.text(f, M + 46, 663, "Only a short fingerprint leaves your browser. The photo itself is never sent.", 12.5, "Regular", TX2,
       w=W - 2 * M - 62, align="left", lh=1.4)
button(f, M, 732, W - 2 * M, 48, "Cancel", "outlined", size=14)
web_footer(f, 812)
gesture(f)

f = screen(S["W4"])
browser(f, f"{DOMAIN}/check", "Check a photo")
VY = 146
D.rect(f, M, VY, W - 2 * M, 112, EDIT, 20, a=0.07, stroke=EDIT, sw=1.2, sa=0.4, name="Verdict")
D.icon(f, "compare", M + 16, VY + 16, 30, EDIT)
D.text(f, M + 56, VY + 14, "Matches an original", 21, "Bold", EDIT, ls=-0.5)
D.text(f, M + 56, VY + 41, "with changes", 14, "Semi Bold", EDIT)
D.text(f, M + 16, VY + 70, "This screenshot is a copy of a sealed capture.\nNo local edits found in the regions we could compare.", 13, "Regular", INK, lh=1.35)
cy_ = 270
card(f, M, cy_, W - 2 * M, 294)
tw_ = (W - 2 * M - 44) / 2
lx, rx = M + 16, M + 28 + tw_
oh = tw_ * 4 / 3
overline(f, lx, cy_ + 14, "Your screenshot")
overline(f, rx, cy_ + 14, "Sealed reference")
sy0 = cy_ + 38
D.image(f, lx, sy0 + (oh - tw_) / 2, tw_, tw_, "hail", 12)
redact_hail(f, lx, sy0 + (oh - tw_) / 2, tw_, tw_)
D.image(f, rx, sy0, tw_, oh, "hail", 12)
redact_hail(f, rx, sy0, tw_, oh)
cy0 = sy0 + (oh - tw_) / 2
D.rect(f, rx, cy0, tw_, tw_, None, 2, stroke=EDIT, sw=2, dashes=[6, 4])
chip(f, rx + 6, cy0 + tw_ - 28, "Crop", WHITE, bg=EDIT, h=22, size=10.5, icon="crop")
D.text(f, lx, sy0 + oh + 10, "Square crop · PNG", 11.5, "Medium", TX2)
D.text(f, rx, sy0 + oh + 10, "Nov 18 · 4:12 PM", 11.5, "Medium", TX2)
dy = 576
card(f, M, dy, W - 2 * M, 176)
overline(f, M + 16, dy + 14, "What changed")
for k, (ic, c, t, s_) in enumerate((("crop", EDIT, "Cropped to a square", "About 25% of the frame removed"),
                                    ("image", EDIT, "Recompressed", "62 of 64 regions match · 2 outside the crop"),
                                    ("edit", EDIT, "2 blurs declared", "By the photographer · house number and a plate"))):
    ry = dy + 38 + k * 44
    D.icon(f, ic, M + 16, ry + 4, 20, c)
    D.text(f, M + 46, ry, t, 13.5, "Semi Bold", INK)
    D.text(f, M + 46, ry + 19, s_, 12, "Regular", TX2)
lw_ = est_width("Open proof page", 14, "Semi Bold")
op = D.text(f, M, 768, "Open proof page", 14, "Semi Bold", PRIMARY)
D.icon(f, "open_in_new", M + lw_ + 10, 768, 18, PRIMARY)
goto(op, S_PROOFPAGE)
web_footer(f, 812)
gesture(f)

def gone_page(code, url, label, ic, title, body, note, lines):
    f = screen(S[code])
    browser(f, url, label)
    D.ellipse(f, W / 2, 260, 44, SURF2)
    D.icon(f, ic, W / 2 - 22, 238, 44, TX2)
    D.text(f, M, 328, title, 22, "Bold", INK, w=W - 2 * M, align="center", ls=-0.5)
    D.text(f, M + 16, 368, body, 14, "Regular", TX2, w=W - 2 * M - 32, align="center", lh=1.45)
    ny = round(368 + lines * 14 * 1.45 + 14)
    D.text(f, M + 16, ny, note, 12.5, "Medium", MUTED, w=W - 2 * M - 32, align="center", lh=1.4)
    return f, ny + 50


f, y = gone_page("W8", f"case.{DOMAIN}/v/8F2K-QM7D#k=••••", "Private case", "timer_off", "This case link has expired",
              "The sender set it to expire on Nov 25. Expired cases are deleted — ask the sender for a new link.",
              "Any copies opened before expiry may still exist.", 2)
card(f, M, y, W - 2 * M, 176)
overline(f, M + 16, y + 16, "Why links expire")
for k, (ic, t, s_) in enumerate((("vpn_key", "The key is in the link", "Witness never sees it or the case"),
                                  ("delete", "Deleted on expiry", "Encrypted files are removed from storage"),
                                  ("link", "New link, new key", "The sender can share again anytime"))):
    ry = y + 42 + k * 42
    D.icon(f, ic, M + 16, ry + 4, 20, TX2)
    D.text(f, M + 46, ry, t, 13.5, "Semi Bold", INK)
    D.text(f, M + 46, ry + 19, s_, 12, "Regular", TX2)
goto(button(f, M, y + 200, W - 2 * M, 48, "Check a saved copy", "outlined", icon="image_search"), S["W2"])
web_footer(f, 812)
gesture(f)

f, y = gone_page("W9", f"{DOMAIN}/p/K9PD-3WLT", "Verification", "hide_image", "This proof page was removed",
              "It was taken down after a report on Nov 20. The anonymous anchored proof still exists, but its image and page are gone.",
              "Removed pages can't be restored by visitors.", 3)
card(f, M, y, W - 2 * M, 176)
overline(f, M + 16, y + 16, "What still exists")
D.icon(f, "verified", M + 16, y + 44, 20, TX2)
D.text(f, M + 46, y + 40, "An anonymous proof record", 13.5, "Semi Bold", INK)
D.text(f, M + 46, y + 59, "No image, no location · anchored on Nov 18", 12, "Regular", TX2)
D.path(f, [(M + 16, y + 90), (W - M - 16, y + 90)], LINE, 1)
overline(f, M + 16, y + 102, "What's gone")
D.icon(f, "hide_image", M + 16, y + 130, 20, TX2)
D.text(f, M + 46, y + 126, "The image and this page", 13.5, "Semi Bold", INK)
D.text(f, M + 46, y + 145, "Copies elsewhere can still be checked", 12, "Regular", TX2)
goto(button(f, M, y + 200, W - 2 * M, 48, "Check a copy", "outlined", icon="image_search"), S["W2"])
web_footer(f, 812)
gesture(f)

f = screen(S["W10"])
browser(f, f"{DOMAIN}/p/H4IL-7QK", "Report")
D.text(f, M, 146, "Report this page", 22, "Bold", INK, ls=-0.5)
D.text(f, M, 176, "Tell us what's wrong. No account needed.", 13, "Regular", TX2)
card(f, M, 204, W - 2 * M, 260)
for k, t in enumerate(("Someone's private information", "Nudity or sexual content", "Violence or abuse", "Copyright",
                       "Misleading or fake", "Other")):
    ry = 208 + k * 42
    if k:
        D.path(f, [(M + 16, ry), (W - M - 16, ry)], LINE, 1)
    on = k == 0
    D.icon(f, "radio_button_checked" if on else "radio_button_unchecked", M + 16, ry + 10, 22, PRIMARY if on else MUTED)
    D.text(f, M + 50, ry + 12, t, 14, "Semi Bold" if on else "Regular", INK)
D.text(f, M, 480, "Details", 13, "Semi Bold", INK)
D.rect(f, M, 500, W - 2 * M, 72, SURF, 14, stroke=LINE)
D.text(f, M + 14, 512, "The car's plate is readable in the reflection on the left.", 13.5, "Regular", INK, w=W - 2 * M - 28, align="left", lh=1.4)
D.text(f, M, 588, "Email (optional)", 13, "Semi Bold", INK)
D.text(f, M, 589, "Only to tell you the outcome", 11.5, "Regular", MUTED, w=W - 2 * M, align="right")
D.rect(f, M, 608, W - 2 * M, 48, SURF, 14, stroke=LINE)
D.text(f, M + 14, 623, "you@example.com", 14, "Regular", MUTED)
button(f, M, 672, W - 2 * M, 48, "Send report", "filled", icon="send")
D.icon(f, "info", M, 738, 16, MUTED)
D.text(f, M + 24, 736, "Reports are reviewed by people. We remove pages, never anchored proofs.", 12.5, "Regular", MUTED,
       w=W - 2 * M - 24, align="left", lh=1.4)
web_footer(f, 812)
gesture(f)
