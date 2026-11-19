from wlib import *

f = screen(S_SETTINGS)
app_bar(f, "Settings", None, right_icon=None)
acct = card(f, M, 96, W - 2 * M, 72)
D.ellipse(f, M + 36, 132, 20, SURF2)
D.icon(f, "account_circle", M + 24, 120, 24, TX2)
D.text(f, M + 68, 112, "No account", 15, "Semi Bold", INK)
D.text(f, M + 68, 134, "Optional: sign in with Internet Identity to sync", 12, "Regular", TX2)
D.icon(f, "chevron_right", W - M - 34, 120, 24, MUTED)
PY_ = 184
D.rect(f, M, PY_, W - 2 * M, 236, INK, 20, name="Plus card")
D.icon(f, "workspace_premium", M + 18, PY_ + 18, 26, "#34D399")
D.text(f, M + 52, PY_ + 20, "Witness Plus", 18, "Bold", WHITE)
D.text(f, W - M - 18 - 120, PY_ + 22, "$2.99 / month", 14, "Semi Bold", "#C9CDD3", w=120, align="right")
for k, s_ in enumerate(("Full-resolution proof pages · 25 GB included", "Case reports as PDF", "Backup and sync across devices",
                        "Creator profile (opt-in)")):
    ry = PY_ + 64 + k * 28
    D.icon(f, "check", M + 18, ry, 18, "#34D399")
    D.text(f, M + 44, ry + 1, s_, 13, "Regular", "#E7EAEE")
pl = button(f, M + 18, PY_ + 182, W - 2 * M - 36, 40, "Subscribe with Google Play", "filled", size=14)
D.text(f, M, PY_ + 248, "Proof records are permanent on every plan. Hosted media stays while you're subscribed, plus 30 days to export.",
       11.5, "Regular", MUTED, w=W - 2 * M, align="left", lh=1.4)
overline(f, M, 478, "Privacy")
pd = D.text(f, W - M - 18 - 140, 476, "Privacy & data", 12.5, "Semi Bold", PRIMARY, w=140, align="right")
D.icon(f, "chevron_right", W - M - 16, 476, 16, PRIMARY)
card(f, M, 500, W - 2 * M, 176)
for k, (ic, t, s_, ctl) in enumerate((("near_me", "Location on public posts", "City or area", "chevron"),
                                      ("auto_awesome", "On-device AI suggestions", "Gemini Nano available", "on"),
                                      ("privacy_tip", "Safety mode", "App lock · coming later", "soon"))):
    ry = 512 + k * 56
    if k:
        D.path(f, [(M + 16, ry - 6), (W - M - 16, ry - 6)], LINE, 1)
    D.icon(f, ic, M + 16, ry + 10, 22, TX2)
    D.text(f, M + 52, ry + 4, t, 14, "Medium", INK)
    D.text(f, M + 52, ry + 24, s_, 12, "Regular", TX2)
    if ctl == "on":
        toggle(f, W - M - 62, ry + 10, True)
    elif ctl == "chevron":
        D.icon(f, "chevron_right", W - M - 34, ry + 10, 24, MUTED)
    else:
        chip(f, W - M - 16, ry + 12, "Later", MUTED, h=22, size=10.5, right=True, outline=True)
card(f, M, 690, W - 2 * M, 112)
for k, (ic, t, c) in enumerate((("description", "Terms, privacy and open-source licenses", INK), ("delete", "Delete account and hosted media", CRIT))):
    ry = 702 + k * 50
    if k:
        D.path(f, [(M + 16, ry - 6), (W - M - 16, ry - 6)], LINE, 1)
    D.icon(f, ic, M + 16, ry + 8, 22, c)
    D.text(f, M + 52, ry + 10, t, 14, "Medium", c)
    hit = D.rect(f, M, ry - 6, W - 2 * M, 50, None, name="Hit " + t)
del_row = hit
D.text(f, M, 816, "Anonymous proofs stay on-chain after deletion — they contain no personal data.", 11.5, "Regular", MUTED, w=W - 2 * M, align="left", lh=1.4)
gesture(f)
goto(acct, S["E3"])
goto(pl, S["E2"])
goto(pd, S["E5"])
goto(del_row, S["E4"])


def dot_row(p, x, y, s, c=INK, size=13.5, dot=TX2, w=None):
    D.ellipse(p, x + 3, y + size * 0.62, 2.5, dot)
    if w:
        return D.text(p, x + 14, y, s, size, "Regular", c, w=w, align="left", lh=1.35)
    return D.text(p, x + 14, y, s, size, "Regular", c)


def icon_disc(p, cx, cy, r, ic, fg, bg, a=1.0):
    D.ellipse(p, cx, cy, r, bg, a=a)
    D.icon(p, ic, cx - r * 0.6, cy - r * 0.6, r * 1.2, fg)


f = screen(S["E2"])
status_bar(f)
D.icon(f, "close", M, 50, 24, INK)
HY = 96
D.rect(f, M, HY, W - 2 * M, 264, INK, 24, name="Plus hero")
D.ellipse(f, W - M - 56, HY + 52, 30, "#34D399", a=0.12)
D.icon(f, "workspace_premium", W - M - 76, HY + 32, 40, "#34D399")
D.text(f, M + 20, HY + 24, "Witness Plus", 26, "Bold", WHITE, ls=-1)
D.text(f, M + 20, HY + 62, "Keep full-quality proof online and\nshare it your way.", 13.5, "Regular", "#C9CDD3", lh=1.35)
for k, s_ in enumerate(("Full-resolution proof pages · 25 GB", "Case reports as PDF", "Backup & sync across devices", "Creator profile (opt-in)")):
    ry = HY + 124 + k * 30
    D.icon(f, "check", M + 20, ry, 18, "#34D399")
    D.text(f, M + 48, ry + 1, s_, 14, "Regular", "#E7EAEE")
D.icon(f, "storage", M, 382, 18, TX2)
D.text(f, M + 28, 382, "25 GB covers hosted full-resolution media. Previews and proof records never count toward it.",
       12, "Regular", TX2, w=W - 2 * M - 28, align="left", lh=1.4)
overline(f, M, 440, "Choose a plan")
PW = (W - 2 * M - 12) / 2
for k, (name, price, per, sel) in enumerate((("Monthly", "$2.99", "per month", False), ("Yearly", "$24.99", "per year · $2.08 a month", True))):
    x = M + k * (PW + 12)
    D.rect(f, x, 464, PW, 112, SURF, 18, stroke=PRIMARY if sel else LINE, sw=2 if sel else 1, name="Plan " + name)
    D.icon(f, "radio_button_checked" if sel else "radio_button_unchecked", x + 14, 480, 22, PRIMARY if sel else MUTED)
    D.text(f, x + 42, 482, name, 14, "Semi Bold", INK)
    if sel:
        chip(f, x + PW - 12, 480, "Save 30%", PRIMARY, h=22, size=10.5, right=True)
    D.text(f, x + 16, 514, price, 24, "Bold", INK, ls=-1)
    D.text(f, x + 16, 548, per, 12, "Regular", TX2)
button(f, M, 600, W - 2 * M, 52, "Subscribe with Google Play", "filled", size=15)
lk = ("Restore purchase", "·", "Terms")
lw = [est_width(s_, 13, "Medium") for s_ in lk]
lx = (W - sum(lw) - 16) / 2
for s_, w_ in zip(lk, lw):
    D.text(f, lx, 668, s_, 13, "Medium", MUTED if s_ == "·" else PRIMARY)
    lx += w_ + 8
ii = card(f, M, 708, W - 2 * M, 72)
icon_disc(f, M + 36, 744, 20, "fingerprint", PRIMARY, PRIMARY, a=0.1)
D.text(f, M + 68, 724, "Plus is linked to your Internet Identity", 13.5, "Medium", INK)
D.text(f, M + 68, 746, "Link Internet Identity", 13, "Semi Bold", PRIMARY)
D.icon(f, "chevron_right", W - M - 34, 732, 24, MUTED)
D.text(f, M, 804, "Proof records are permanent on every plan. Hosted media stays while you're subscribed, plus 30 days to export.",
       11.5, "Regular", MUTED, w=W - 2 * M, align="center", lh=1.4)
gesture(f)
goto(ii, S["E3"])

f = screen(S["E3"])
app_bar(f, "Link Internet Identity", right_icon=None)
CX = W / 2
D.path(f, [(CX - 96, 188), (CX + 96, 188)], LINE, 2, dashes=[4, 6])
icon_disc(f, CX - 104, 188, 30, "key", TX2, SURF2)
icon_disc(f, CX + 104, 188, 30, "devices", TX2, SURF2)
D.ellipse(f, CX, 184, 60, PAPER)
D.ellipse(f, CX, 184, 56, PRIMARY, a=0.1)
D.ellipse(f, CX, 184, 40, PRIMARY)
D.icon(f, "fingerprint", CX - 24, 160, 48, WHITE)
D.text(f, M, 272, "One identity for all your phones", 22, "Bold", INK, w=W - 2 * M, align="center", ls=-0.5)
D.text(f, M + 24, 310, "Internet Identity uses your phone's passkey. No email, no password.", 14, "Regular", TX2,
       w=W - 2 * M - 48, align="center", lh=1.4)
card(f, M, 384, W - 2 * M, 204)
for k, (ic, t) in enumerate((("delete", "Delete your posts from any device or the web"), ("sync", "Sync your captures across phones"),
                             ("workspace_premium", "Required for Witness Plus"))):
    ry = 390 + k * 64
    if k:
        D.path(f, [(M + 16, ry), (W - M - 16, ry)], LINE, 1)
    icon_disc(f, M + 36, ry + 32, 18, ic, PRIMARY, PRIMARY, a=0.1)
    D.text(f, M + 66, ry + 22, t, 13.5, "Medium", INK)
D.rect(f, M, 608, W - 2 * M, 72, SURF2, 18, name="Privacy note")
D.icon(f, "visibility_off", M + 16, 632, 22, TX2)
D.text(f, M + 50, 624, "Your proofs stay anonymous — the link is never public.", 13, "Regular", INK, w=W - 2 * M - 66, align="left", lh=1.4)
D.text(f, M, 704, "Optional. Witness works fully without an account.", 12, "Regular", MUTED, w=W - 2 * M, align="center")
button(f, M, 768, W - 2 * M, 52, "Continue with Internet Identity", "filled", size=15, icon="login")
nn = button(f, M, 832, W - 2 * M, 48, "Not now", "outlined")
gesture(f)
goto(nn, S["E1"])

f = screen(S["E4"])
app_bar(f, "Settings", None, right_icon=None)
card(f, M, 96, W - 2 * M, 72)
D.ellipse(f, M + 36, 132, 20, SURF2)
D.text(f, M + 68, 122, "No account", 15, "Semi Bold", INK)
D.rect(f, M, 184, W - 2 * M, 236, INK, 20)
D.text(f, M + 20, 204, "Witness Plus", 18, "Bold", WHITE)
overline(f, M, 478, "Privacy")
card(f, M, 500, W - 2 * M, 176)
card(f, M, 690, W - 2 * M, 112)
D.text(f, M + 52, 762, "Delete account and hosted media", 14, "Medium", CRIT)
gesture(f)
D.rect(f, 0, 0, W, H, "#000000", a=0.45, name="Dim")
DX, DW_, DHH = 24, W - 48, 550
DY = round((H - DHH) / 2)
D.rect(f, DX, DY, DW_, DHH, SURF, 24, name="Dialog")
icon_disc(f, CX, DY + 48, 24, "delete", CRIT, CRIT, a=0.12)
D.text(f, DX + 24, DY + 88, "Delete account and hosted media?", 20, "Semi Bold", INK, w=DW_ - 48, align="center", lh=1.3)
IX = DX + 24
D.icon(f, "delete", IX, DY + 156, 18, CRIT)
overline(f, IX + 26, DY + 159, "Deleted", CRIT)
for k, s_ in enumerate(("Previews and proof pages", "Encrypted case links", "Fingerprints", "Your Internet Identity link")):
    dot_row(f, IX + 4, DY + 184 + k * 24, s_)
D.icon(f, "lock", IX, DY + 294, 18, INK)
overline(f, IX + 26, DY + 297, "Stays", INK)
dot_row(f, IX + 4, DY + 322, "Anonymous anchored proofs — no images or personal data", w=DW_ - 66)
D.rect(f, IX, DY + 386, DW_ - 48, 60, SURF2, 14, name="Subscription note")
D.icon(f, "info", IX + 12, DY + 406, 20, TX2)
D.text(f, IX + 42, DY + 398, "Cancel Witness Plus in Google Play separately.", 12.5, "Regular", INK, w=DW_ - 48 - 56, align="left", lh=1.4)
BW = (DW_ - 48 - 12) / 2
cn = button(f, IX, DY + 474, BW, 48, "Cancel", "outlined")
D.rect(f, IX + BW + 12, DY + 474, BW, 48, CRIT, 24, name="Button Delete")
D.label(f, IX + BW + 12, DY + 474, BW, 48, "Delete", 14.5, "Semi Bold", WHITE)
goto(cn, S["E1"])

f = screen(S["E5"])
app_bar(f, "Privacy & data", right_icon=None)
SECTIONS = (("On the Internet Computer · permanent", "lock", INK, (("tag", "Hashes and commitments"), ("key", "Key IDs · rotated, unlinkable"), ("schedule", "Capture and anchor times"))),
            ("On the Internet Computer · erasable", "delete", TX2, (("fingerprint", "Fingerprints and region grids"), ("image", "Previews and proof pages"), ("link", "Encrypted case links"))),
            ("Only on this phone", "smartphone", TX2, (("photo_library", "Original photos"), ("location_on", "Exact locations"), ("vpn_key", "Signing keys · hardware-backed"))))
for i, (title, hic, hc, rows) in enumerate(SECTIONS):
    sy = 100 + i * 180
    D.icon(f, hic, M, sy - 2, 16, hc)
    overline(f, M + 22, sy, title, hc)
    card(f, M, sy + 24, W - 2 * M, 140)
    for k, (ic, t) in enumerate(rows):
        ry = sy + 28 + k * 44
        if k:
            D.path(f, [(M + 52, ry), (W - M - 16, ry)], LINE, 1)
        D.icon(f, ic, M + 16, ry + 11, 22, TX2)
        D.text(f, M + 52, ry + 12, t, 14, "Regular", INK)
card(f, M, 652, W - 2 * M, 136)
for k, (ic, t, s_, c) in enumerate((("download", "Export my data", "Proof records, case files and a JSON summary", PRIMARY),
                                    ("delete", "Erase all public copies", "Fingerprints, previews, pages and links. Proofs stay.", CRIT))):
    ry = 656 + k * 64
    if k:
        D.path(f, [(M + 16, ry), (W - M - 16, ry)], LINE, 1)
    D.icon(f, ic, M + 16, ry + 20, 22, c)
    D.text(f, M + 52, ry + 12, t, 14, "Semi Bold", c)
    D.text(f, M + 52, ry + 34, s_, 12, "Regular", TX2)
D.text(f, M, 808, "Erasing is signed by this phone's key, or from any device with a linked Internet Identity.", 11.5, "Regular", MUTED,
       w=W - 2 * M, align="left", lh=1.4)
gesture(f)
