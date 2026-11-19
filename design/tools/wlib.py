import hashlib, json, math, os, struct, sys, zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
ICONS = json.load(open(os.path.join(HERE, "..", "icons", "icons.json")))
IMG_DIR = os.path.join(HERE, "..", "images")


def load_image(name):
    data = open(os.path.join(IMG_DIR, name + ".png"), "rb").read()
    w, h = struct.unpack(">II", data[16:24])
    return {"data": data, "hash": hashlib.sha256(data).hexdigest(), "w": w, "h": h}


IMAGES = {n: load_image(n) for n in ("crash", "bumper", "plate", "wide", "water", "rental", "hail", "rainbow", "moose", "random")}

PAPER = "#F4F3EF"
SURF = "#FFFFFF"
SURF2 = "#EDEBE5"
SURF3 = "#E3E0D8"
LINE = "#E0DCD2"
INK = "#15171B"
TX2 = "#575D67"
MUTED = "#666B75"
PRIMARY = "#2D3BE6"
ON_PRIMARY = "#FFFFFF"
AI = "#7A3FF2"
OK = "#0A8754"
WARN = "#B4690E"
CRIT = "#C93A32"
EDIT = "#0E7490"
OK_T = "#076E44"
WARN_T = "#925309"
PROVEN_DARK = "#4ADE80"
TINT_TEXT = {"#0A8754": "#076E44", "#B4690E": "#925309"}
CAM = "#0B0C0E"
WHITE = "#FFFFFF"

W, H = 412, 915
M = 16
NAV_Y = 835
SANS, MONO = "Inter", "Menlo"
GAP = 64


def col(h, a=1.0):
    h = h.lstrip("#")
    r, g, b = (int(h[i:i + 2], 16) / 255 for i in (0, 2, 4))
    return {"r": r, "g": g, "b": b, "a": a}


def solid(h, a=1.0):
    return [{"type": "solid", "color": col(h, a), "opacity": 1.0, "visible": True, "blend": "normal"}]


def stroke_spec(sw, dashes=None, align="inside", cap="none"):
    return {"weight": float(sw), "align": align, "sides": None, "cap_start": cap, "cap_end": cap, "join": "round", "miter_limit": 4.0,
            "dashes": [float(d) for d in (dashes or [])]}


_NARROW = set("ilI.,:;!|'`\"· ")
_SEMI = set("fjrt()[]-–/")
_WIDE = set("mwABCDFGHKNOQRSTUVXYZ+=→←✓×≥≤")
_XWIDE = set("MW%@—●◐")
_WMULT = {"Regular": 0.98, "Medium": 1.01, "Semi Bold": 1.04, "Bold": 1.07}


def est_width(s, size, style, font=SANS, ls=0.0):
    def units(line):
        if font == MONO:
            return len(line) * 0.602
        u = 0.0
        for ch in line:
            u += 0.28 if ch in _NARROW else 0.36 if ch in _SEMI else 0.65 if ch in _WIDE else 0.82 if ch in _XWIDE else 0.53
        return u
    lines = s.split("\n")
    mult = 1.0 if font == MONO else _WMULT.get(style, 1.0)
    return max(units(l) * size * mult + len(l) * size * ls / 100 for l in lines) + 2.0


class Doc:
    def __init__(self):
        self.nodes = {1: {"id": 1, "name": "Witness · Android", "type": "page", "background": col("#DCD9D1"), "flows": [], "guides": [],
                          "prototype": {"device": "None", "background": col("#DCD9D1"), "rotation": "portrait"}, "children": [], "width": 0.0, "height": 0.0}}
        self.next = 2
        self.frames = []

    def add(self, parent, node):
        i = self.next
        self.next += 1
        node["id"] = i
        node["parent"] = parent
        self.nodes[i] = node
        self.nodes[parent]["children"].append(i)
        return i

    def frame(self, name, x, y, w=W, h=H, fill=PAPER):
        f = self.add(1, {"name": name, "type": "frame", "children": [], "transform": [1, 0, 0, 1, float(x), float(y)], "width": float(w), "height": float(h),
                         "fills": solid(fill), "clip": True})
        self.frames.append((name, f))
        return f

    def clip(self, p, x, y, w, h, fill=None, r=0, stroke=None, name="Clip", radii=None):
        n = {"name": name, "type": "frame", "children": [], "transform": [1, 0, 0, 1, float(x), float(y)], "width": float(w), "height": float(h),
             "corners": {"radii": [float(v) for v in (radii or [r] * 4)], "smoothing": 0.6}, "clip": True}
        if fill:
            n["fills"] = solid(fill)
        if stroke:
            n["strokes"] = solid(stroke)
            n["stroke"] = stroke_spec(1)
        return self.add(p, n)

    def rect(self, p, x, y, w, h, fill=None, r=0, stroke=None, sw=1.0, a=1.0, sa=1.0, dashes=None, radii=None, name="Rectangle"):
        n = {"name": name, "type": "rectangle", "blend": "normal", "transform": [1, 0, 0, 1, float(x), float(y)], "width": float(w), "height": float(h),
             "corners": {"radii": [float(v) for v in (radii or [r] * 4)], "smoothing": 0.6}}
        if fill:
            n["fills"] = solid(fill, a)
        if stroke:
            n["strokes"] = solid(stroke, sa)
            n["stroke"] = stroke_spec(sw, dashes)
        return self.add(p, n)

    def ellipse(self, p, cx, cy, r, fill=None, stroke=None, sw=1.0, a=1.0, dashes=None, sa=1.0, name="Ellipse"):
        n = {"name": name, "type": "ellipse", "start": 0.0, "sweep": 2 * math.pi, "inner": 0.0, "blend": "normal",
             "transform": [1, 0, 0, 1, float(cx - r), float(cy - r)], "width": float(2 * r), "height": float(2 * r)}
        if fill:
            n["fills"] = solid(fill, a)
        if stroke:
            n["strokes"] = solid(stroke, sa)
            n["stroke"] = stroke_spec(sw, dashes, "center")
        return self.add(p, n)

    def path(self, p, pts, color, sw=1.5, dashes=None, closed=False, a=1.0, name="Path"):
        mx, my = min(x for x, _ in pts), min(y for _, y in pts)
        verts = [{"x": float(x - mx), "y": float(y - my), "radius": 0.0} for x, y in pts]
        n_seg = len(pts) if closed else len(pts) - 1
        segs = [{"a": k, "b": (k + 1) % len(pts), "ta": [0.0, 0.0], "tb": [0.0, 0.0]} for k in range(n_seg)]
        return self.add(p, {"name": name, "type": "vector", "blend": "normal", "transform": [1, 0, 0, 1, float(mx), float(my)],
                            "width": float(max(1, max(x for x, _ in pts) - mx)), "height": float(max(1, max(y for _, y in pts) - my)),
                            "network": {"vertices": verts, "segments": segs, "regions": []},
                            "strokes": solid(color, a), "stroke": stroke_spec(sw, dashes, "center", "round")})

    def text(self, p, x, y, s, size=14, style="Regular", color=INK, w=None, align=None, lh=1.3, font=SANS, ls=0.0, name=None):
        multi = "\n" in s or (align is not None and w is not None)
        n = {"name": name or (s.split("\n")[0][:40] or "Text"), "type": "text", "characters": s,
             "style": {"font_family": font, "font_style": style, "font_size": float(size),
                       "line_height": {"unit": "percent", "value": lh * 100} if multi else {"unit": "auto"},
                       "letter_spacing": {"unit": "percent", "value": float(ls)}, "decoration": "none", "case": "original", "fills": None, "link": None},
             "blend": "normal", "transform": [1, 0, 0, 1, float(x), float(y)], "width": float(w if align else est_width(s, size, style, font, ls)),
             "height": float(len(s.split("\n")) * size * lh), "fills": solid(color)}
        if align:
            n["align_h"] = align
            n["auto_resize"] = "height"
        return self.add(p, n)

    def icon(self, p, name, x, y, size=24, color=INK):
        k = size / ICONS[name]["size"]
        for part in ICONS[name]["parts"]:
            net = part["network"]
            verts = [dict(v, x=v["x"] * k, y=v["y"] * k) for v in net["vertices"]]
            segs = [dict(s, ta=[t * k for t in s["ta"]], tb=[t * k for t in s["tb"]]) for s in net["segments"]]
            self.add(p, {"name": "icon/" + name, "type": "vector", "blend": "normal",
                         "transform": [1, 0, 0, 1, float(x + part["x"] * k), float(y + part["y"] * k)],
                         "width": float(max(v["x"] for v in verts)), "height": float(max(v["y"] for v in verts)),
                         "network": {"vertices": verts, "segments": segs, "regions": net["regions"]}, "fills": solid(color)})

    def image(self, p, x, y, w, h, key, r=0, radii=None, name=None):
        return self.add(p, {"name": name or key, "type": "rectangle", "blend": "normal", "transform": [1, 0, 0, 1, float(x), float(y)],
                            "width": float(w), "height": float(h), "corners": {"radii": [float(v) for v in (radii or [r] * 4)], "smoothing": 0.6},
                            "fills": [{"type": "image", "image": IMAGES[key]["hash"], "scale_mode": "fill", "scale": 1.0, "rotation": 0, "crop": None,
                                       "opacity": 1.0, "visible": True, "blend": "normal",
                                       "filters": {"contrast": 0.0, "exposure": 0.0, "highlights": 0.0, "saturation": 0.0, "shadows": 0.0, "temperature": 0.0, "tint": 0.0}}]})

    def label(self, p, x, y, w, h, s, size=14, style="Semi Bold", color=INK, font=SANS, ls=0.0):
        return self.text(p, x, y + (h - size * 1.21) / 2, s, size, style, color, w=w, align="center", font=font, ls=ls)

    def nav(self, src_ids, dest):
        for i in src_ids:
            self.nodes[i]["interactions"] = [{"trigger": {"type": "onClick"}, "actions": [{"type": "navigate", "dest": dest}],
                                              "transition": {"type": "dissolve"}, "duration_ms": 200, "easing": "easeOut"}]

    def write(self, path, start):
        by = {n: i for n, i in self.frames}
        self.nodes[1]["flows"] = [{"name": "Capture → Case → Proof → Share → Verify", "start": by[start],
                                   "description": "A fender bender, from the evidence locker through sealed capture, the AI-built report, proof, a share link and the recipient's verdict."}]
        doc = {"format": "figcraft", "version": 1, "imageIndex": [{"hash": v["hash"], "width": v["w"], "height": v["h"]} for v in IMAGES.values()],
               "name": "witness", "pages": [1], "nodes": {str(k): v for k, v in self.nodes.items()},
               "next_id": self.next, "styles": [], "variables": {"collections": [], "variables": []}, "current_page": 1, "document_colors": [], "comments": []}
        with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as z:
            z.writestr("document.json", json.dumps(doc))
            for v in IMAGES.values():
                z.writestr(f"images/{v['hash']}", v["data"])


D = Doc()
NAV = {}


def goto(ids, frame):
    NAV.setdefault(frame, []).extend(ids if isinstance(ids, (list, tuple)) else [ids])


ROWS = {"A": "Start & home", "B": "Capture & post", "C": "Cases", "D": "Item & check", "E": "Settings & account",
        "W": "Website · mobile", "X": "Website · desktop"}
SCREEN_LIST = ['A1 Splash', 'A2 Welcome', 'A3 Permissions', 'A4 Device setup', 'A5 Unsupported device', 'A6 Home · empty', 'A7 Home', 'A8 Anchoring status', 'A9 Search', 'B1 Camera · Post', 'B2 Camera · sealed offline', 'B3 Post composer', 'B4 Edit photo', 'B5 Location precision', 'B6 Post shared', 'C1 Camera · Case', 'C2 Case switcher', 'C3 New case', 'C4 Cases', 'C5 Case', 'C6 Report template', 'C7 Flag AI output', 'C8 Export', 'C9 Share case', 'D1 Item proof', 'D2 Item actions', 'D3 Technical record', 'D4 Prove location', 'D5 Check', 'D6 Checking', 'D7 Check result', 'D8 Check · not yet anchored', 'E1 Settings', 'E2 Witness Plus', 'E3 Internet Identity', 'E4 Delete account', 'E5 Privacy & data', 'W1 Proof page', 'W2 Web check', 'W3 Web checking', 'W4 Web · with changes', 'W5 Altered', 'W6 No record', 'W7 Case link', 'W8 Case link expired', 'W9 Removed page', 'W10 Report', 'X1 Desktop · proof page', 'X2 Desktop · check', 'X3 Desktop · case link']
S = {}
for _n in SCREEN_LIST:
    _code, _title = _n.split(" ", 1)
    S[_code] = f"{_code} · {_title}"
S_HOME, S_CAMPOST, S_COMPOSER, S_CHECK, S_PROOFPAGE = S["A7"], S["B1"], S["B3"], S["D7"], S["W1"]
S_CAMCASE, S_CASE, S_ITEM, S_SHARE, S_CASELINK = S["C1"], S["C5"], S["D1"], S["C9"], S["W7"]
S_ALTERED, S_CHECKIN, S_NORECORD, S_SETTINGS, S_DESKTOP = S["W5"], S["D5"], S["W6"], S["E1"], S["X1"]
ROW_H = H + 220
DW, DH = 1280, 820
ROW_Y = {r: i * ROW_H for i, r in enumerate(ROWS)}
FOUND_Y = len(ROWS) * ROW_H + 40
FOUND_W = 10 * (W + GAP) - GAP


def screen(name, fill=PAPER, w=None, h=None):
    code = name.split(" ")[0]
    row, col = code[0], int(code[1:]) - 1
    if row == "X":
        w, h = w or DW, h or DH
        x = col * (DW + GAP)
    else:
        w, h = w or W, h or H
        x = col * (W + GAP)
    return D.frame(name, x, ROW_Y[row], w, h, fill)


def row_labels():
    for r, title in ROWS.items():
        D.text(1, 0, ROW_Y[r] - 64, f"{r} · {title}", 34, "Bold", INK, ls=-1, name="Row " + r)


DOMAIN = "witness.app"
HOUSE, PLATE = (0.72, 0.405), (0.567, 0.575)


def img_pt(nx, ny, x, y, w, h, iw=896, ih=1200):
    k = max(w / iw, h / ih)
    return x + nx * iw * k - (iw * k - w) / 2, y + ny * ih * k - (ih * k - h) / 2


def fx(i):
    return i * (W + GAP)


def overline(p, x, y, s, color=MUTED):
    return D.text(p, x, y, s.upper(), 11, "Semi Bold", color, ls=8)


def card(p, x, y, w, h, fill=SURF, r=18, stroke=LINE):
    return D.rect(p, x, y, w, h, fill, r, stroke=stroke, name="Card")


def chip(p, x, y, s, fg=TX2, bg=None, a=0.12, h=26, size=12, icon=None, right=False, mono=False, style="Semi Bold", dot=False, outline=False):
    font = MONO if mono else SANS
    lead = 21 if icon else 12 if dot else 0
    w = est_width(s, size, style, font) + 20 + lead
    if right:
        x -= w
    if outline:
        r = D.rect(p, x, y, w, h, None, h / 2, stroke=LINE, name="Chip " + s)
    else:
        r = D.rect(p, x, y, w, h, bg or fg, h / 2, a=a if bg is None else 1.0, name="Chip " + s)
    if icon:
        D.icon(p, icon, x + 9, y + (h - 15) / 2, 15, fg)
    if dot:
        D.ellipse(p, x + 13, y + h / 2, 3.5, fg)
    tc = TINT_TEXT.get(fg, fg) if bg is None and not outline else fg
    D.label(p, x + lead, y, w - lead, h, s, size, style, tc, font)
    return r, w


def button(p, x, y, w, h, s, kind="filled", size=14.5, icon=None):
    fill, a, stroke, color = {
        "filled": (PRIMARY, 1.0, None, ON_PRIMARY),
        "ink": (INK, 1.0, None, WHITE),
        "tonal": (SURF3, 1.0, None, INK),
        "outlined": (None, 1.0, LINE, INK),
        "glass": ("#000000", 0.42, None, WHITE),
    }[kind]
    r = D.rect(p, x, y, w, h, fill, h / 2, a=a, stroke=stroke, name="Button " + s)
    if icon:
        tw = est_width(s, size, "Semi Bold") + 26
        ix = x + (w - tw) / 2
        D.icon(p, icon, ix, y + (h - 20) / 2, 20, color)
        D.text(p, ix + 26, y + (h - size * 1.21) / 2, s, size, "Semi Bold", color)
    else:
        D.label(p, x, y, w, h, s, size, "Semi Bold", color)
    return r


def status_bar(p, color=INK):
    D.text(p, 22, 10, "2:32", 13.5, "Semi Bold", color)
    D.icon(p, "wifi", W - 86, 9, 17, color)
    D.icon(p, "cell_tower", W - 66, 9, 17, color)
    D.rect(p, W - 44, 12, 22, 11, None, 3.5, stroke=color, sw=1.2, sa=0.7)
    D.rect(p, W - 42, 14, 14, 7, color, 1.5)


def mark(p, cx, cy, s, ring=INK, check=OK, sw=None):
    sw = sw or s * 0.11
    r = s * 0.36
    D.ellipse(p, cx, cy, r, None, stroke=ring, sw=sw, name="Mark ring")
    D.rect(p, cx + r * 0.42, cy - r - sw * 1.9, r * 0.62, sw * 1.05, ring, sw / 2, name="Mark shutter")
    D.path(p, [(cx - r * 0.42, cy + r * 0.02), (cx - r * 0.1, cy + r * 0.34), (cx + r * 0.46, cy - r * 0.3)], check, sw, name="Mark check")


def wordmark(p, x, y, size=22, color=INK):
    mark(p, x + size * 0.55, y + size * 0.62, size * 1.15)
    D.text(p, x + size * 1.35, y, "Witness", size, "Bold", color, ls=-1.5)


def seal_badge(p, cx, cy, state, r=11, ring=SURF):
    c, ic = {"anchored": (OK, "verified"), "queued": (WARN, "hourglass_top"), "sealed": (INK, "lock"), "edited": (EDIT, "edit"),
             "altered": (CRIT, "error"), "unverified": (MUTED, "help")}[state]
    D.ellipse(p, cx, cy, r + 2, ring)
    D.ellipse(p, cx, cy, r, c)
    D.icon(p, ic, cx - r * 0.66, cy - r * 0.66, r * 1.32, WHITE)


STATES = {"original": ("Original", OK, "check_circle"), "changes": ("Matches an original, with changes", EDIT, "compare"),
          "altered": ("Altered", CRIT, "error"), "norecord": ("No Witness record", MUTED, "help"), "anchored": ("Anchored", OK, "verified"),
          "queued": ("Queued", WARN, "hourglass_top"), "sealed": ("Sealed", INK, "lock"), "edited": ("Declared edit", EDIT, "edit"),
          "unverified": ("Unverified import", MUTED, "help"), "unanchored": ("Sealed · not yet anchored", INK, "lock")}


def state_chip(p, x, y, state, right=False, h=26, size=12, text=None):
    s, c, ic = STATES[state]
    return chip(p, x, y, text or s, c, h=h, size=size, icon=ic, right=right)


def generated_tag(p, x, y, s="Generated on-device", right=False):
    return chip(p, x, y, s, AI, h=24, size=11, icon="auto_awesome", right=right)


def generated_surface(p, x, y, w, h, name="Generated surface"):
    D.rect(p, x, y, w, h, SURF, 18, stroke=AI, sw=1.2, sa=0.35, name=name)
    D.rect(p, x, y + 18, 3, h - 36, AI, 1.5, a=0.9)


def app_bar(p, title, sub=None, back=True, right_icon="more_vert", lead="arrow_back"):
    status_bar(p)
    x = M
    if back:
        D.icon(p, lead, M, 50, 24, INK)
        x = 56
    D.text(p, x, 46 if sub else 50, title, 20, "Semi Bold", INK)
    if sub:
        D.text(p, x, 72, sub, 12.5, "Regular", TX2)
    if right_icon:
        D.icon(p, right_icon, W - M - 24, 50, 24, INK)


def nav_bar(p, active):
    D.rect(p, 0, NAV_Y, W, H - NAV_Y, SURF, name="Nav bar")
    D.path(p, [(0, NAV_Y), (W, NAV_Y)], LINE, 1)
    for i, (name, ic, dest) in enumerate((("Home", "home", S_HOME), ("Check", "verified_user", S_CHECKIN))):
        cx = W * (0.18 if i == 0 else 0.82)
        on = name == active
        hit = D.rect(p, cx - 56, NAV_Y, 112, 70, None, name="Tab " + name)
        if on:
            D.rect(p, cx - 30, NAV_Y + 10, 60, 30, INK, 15, a=0.08)
        D.icon(p, ic, cx - 12, NAV_Y + 13, 24, INK if on else TX2)
        D.label(p, cx - 56, NAV_Y + 44, 112, 16, name, 12, "Semi Bold" if on else "Medium", INK if on else TX2)
        goto(hit, dest)
    cx = W / 2
    D.ellipse(p, cx, NAV_Y + 8, 36, PAPER)
    shutter = D.ellipse(p, cx, NAV_Y + 8, 30, INK, name="Capture")
    D.icon(p, "photo_camera", cx - 13, NAV_Y - 5, 26, WHITE)
    D.label(p, cx - 50, NAV_Y + 44, 100, 16, "Capture", 12, "Medium", TX2)
    goto(shutter, S_CAMPOST)
    D.rect(p, W / 2 - 54, H - 10, 108, 4, INK, 2, a=0.35, name="Gesture handle")


def scrim(p, y0, h, a_max, steps=10):
    for k in range(steps):
        D.rect(p, 0, y0, W, h * (k + 1) / steps, "#000000", a=a_max / steps, name="Scrim")


def gesture(p, color=INK, a=0.35):
    D.rect(p, W / 2 - 54, H - 10, 108, 4, color, 2, a=a, name="Gesture handle")


def mosaic(p, x, y, w, h, seed, palette):
    n = 4
    cw, ch = w / n, h / n
    for i in range(n):
        for j in range(n):
            k = int(hashlib.md5(f"{seed}{i}{j}".encode()).hexdigest()[:2], 16) % len(palette)
            D.rect(p, x + i * cw, y + j * ch, cw + 0.4, ch + 0.4, palette[k])


FACE = (0.868, 0.41)


def wide_thumb(p, x, y, w, h, r=12, redacted=False):
    D.image(p, x, y, w, h, "wide", r)
    if redacted:
        s = w * 0.07
        cx, cy = x + w * FACE[0], y + h * FACE[1]
        mosaic(p, cx - s / 2, cy - s / 2, s, s, "face", ["#8E8B86", "#A6A29B", "#7A7772", "#B5B0A8", "#94908A"])


def qr(p, x, y, size, seed="8F2K-QM7D", fg=INK):
    n = 25
    m = size / n
    D.rect(p, x - 8, y - 8, size + 16, size + 16, SURF, 10)

    def finder(i, j):
        D.rect(p, x + i * m, y + j * m, 7 * m, 7 * m, fg, 1.5)
        D.rect(p, x + (i + 1) * m, y + (j + 1) * m, 5 * m, 5 * m, SURF, 1)
        D.rect(p, x + (i + 2) * m, y + (j + 2) * m, 3 * m, 3 * m, fg, 1)
    for (i, j) in ((0, 0), (n - 7, 0), (0, n - 7)):
        finder(i, j)
    bits = hashlib.sha256(seed.encode()).digest() * 4
    k = 0
    for j in range(n):
        for i in range(n):
            if (i < 8 and j < 8) or (i >= n - 8 and j < 8) or (i < 8 and j >= n - 8):
                continue
            if bits[k // 8] >> (k % 8) & 1:
                D.rect(p, x + i * m, y + j * m, m + 0.3, m + 0.3, fg)
            k += 1


def redact_hail(p, x, y, w, h):
    k = max(w / 896, h / 1200)
    for (nx, ny), (sw_, sh_) in ((HOUSE, (36, 64)), (PLATE, (54, 30))):
        cx, cy = img_pt(nx, ny, x, y, w, h)
        bw, bh = sw_ * k, sh_ * k
        if x <= cx <= x + w and y <= cy <= y + h:
            mosaic(p, cx - bw / 2, cy - bh / 2, bw, bh, f"{nx}{ny}", ["#5E6064", "#A7A9AC", "#47494D", "#C4C5C7", "#7F8185"])


def toggle(p, x, y, on=True):
    D.rect(p, x, y, 46, 26, PRIMARY if on else SURF3, 13)
    D.ellipse(p, x + (33 if on else 13), y + 13, 9.5, WHITE)


def flag(p, x, y, color=MUTED):
    return D.icon(p, "flag", x, y, 18, color)


def ai_header(p, x, y, label, w, flag_it=True):
    D.icon(p, "auto_awesome", x, y - 1, 16, AI)
    overline(p, x + 22, y + 1, label, AI)
    if flag_it:
        flag(p, x + w - 22, y - 2)


def browser(p, url, label="Verification", w=W):
    status_bar(p)
    D.rect(p, 0, 34, w, 58, SURF)
    D.rect(p, M, 42, w - 2 * M - 44, 40, SURF2, 20)
    D.icon(p, "lock", M + 14, 53, 16, TX2)
    D.text(p, M + 36, 53, url, 13, "Regular", INK)
    D.rect(p, w - M - 30, 49, 26, 26, None, 6, stroke=INK, sw=1.6)
    D.label(p, w - M - 30, 49, 26, 26, "2", 12, "Bold", INK)
    D.path(p, [(0, 92), (w, 92)], LINE, 1)
    wordmark(p, M, 108, 19)
    D.text(p, M, 112, label, 13, "Medium", MUTED, w=w - 2 * M, align="right")


def web_footer(p, y, w=W):
    D.path(p, [(M, y), (w - M, y)], LINE, 1)
    x = M
    for s, ic in (("Report this page", "report"), ("Contact", None), ("Check a copy", None)):
        if ic:
            D.icon(p, ic, x, y + 13, 16, TX2)
            x += 20
        D.text(p, x, y + 13, s, 12, "Semi Bold", TX2)
        x += est_width(s, 12, "Semi Bold") + 18
    D.text(p, M, y + 38, "Served from an ICP canister · certified response", 11, "Regular", MUTED, font=MONO)


def proves_card(p, x, y, w, rows=None):
    rows = rows or ((True, "Genuine phone running the real Witness app"), (True, "Existed no later than its anchor time"),
                    (True, "Unaltered, except declared edits"), (False, "Doesn't prove the scene wasn't staged"))
    card(p, x, y, w, 40 + 24 * len(rows))
    overline(p, x + 16, y + 14, "What this proves")
    for k, (good, s) in enumerate(rows):
        ry = y + 38 + k * 24
        if good:
            D.icon(p, "check", x + 16, ry, 18, OK)
        else:
            D.path(p, [(x + 20, ry + 9), (x + 30, ry + 9)], MUTED, 2)
        D.text(p, x + 42, ry + 1, s, 12.5, "Regular", INK if good else MUTED)
    return y + 40 + 24 * len(rows)


def fade_right(p, x, y, w, h, color=PAPER, steps=8):
    for k in range(steps):
        D.rect(p, x + w * k / steps, y, w * (steps - k) / steps, h, color, a=1.0 / steps, name="Fade")


def mode_switch(p, mode):
    w, h = 148, 36
    x = (W - w) / 2
    D.rect(p, x, 48, w, h, "#000000", h / 2, a=0.5)
    for k, s in enumerate(("Post", "Case")):
        on = s == mode
        if on:
            D.rect(p, x + 3 + k * (w / 2 - 3), 51, w / 2 - 3, h - 6, WHITE, (h - 6) / 2)
        D.label(p, x + k * w / 2 + (3 if k == 0 else 0), 48, w / 2 - 3, h, s, 13.5, "Semi Bold", INK if on else WHITE)


def glass_pill(p, x, y, s, ic, c=WHITE, trailing=None):
    tw_ = est_width(s, 12, "Semi Bold") + 40 + (18 if trailing else 0)
    D.rect(p, x, y, tw_, 30, "#000000", 15, a=0.5)
    D.icon(p, ic, x + 10, y + 7, 16, c)
    D.text(p, x + 31, y + 7, s, 12, "Semi Bold", WHITE)
    if trailing:
        D.icon(p, trailing, x + tw_ - 24, y + 6, 18, WHITE)
    return tw_


VF = 724


def camera(name, idx, img, mode, thumb, meta):
    f = screen(name, fill=CAM)
    D.image(f, 0, 0, W, VF, img)
    scrim(f, 0, 170, 0.5)
    status_bar(f, WHITE)
    cl = D.ellipse(f, M + 20, 66, 20, "#000000", a=0.42)
    D.icon(f, "close", M + 8, 54, 24, WHITE)
    goto(cl, S_HOME)
    mode_switch(f, mode)
    D.ellipse(f, W - M - 20, 66, 20, "#000000", a=0.42)
    D.icon(f, "flash_off", W - M - 32, 54, 24, WHITE)
    x = M
    for s, ic in (("StrongBox seal", "lock"), ("Location · High", "satellite_alt")):
        x += glass_pill(f, x, 100, s, ic, PROVEN_DARK) + 6
    for (bx, by_, sx, sy) in ((56, 190, 1, 1), (W - 56, 190, -1, 1), (56, 560, 1, -1), (W - 56, 560, -1, -1)):
        D.path(f, [(bx, by_ + 18 * sy), (bx, by_), (bx + 18 * sx, by_)], WHITE, 2, a=0.55)
    th = D.image(f, M + 16, VF + 44, 56, 56, thumb, 14)
    seal_badge(f, M + 68, VF + 48, "anchored", 9, ring=CAM)
    D.ellipse(f, W / 2, VF + 72, 38, None, stroke=WHITE, sw=4)
    sh = D.ellipse(f, W / 2, VF + 72, 30, WHITE, name="Shutter")
    D.ellipse(f, W - M - 44, VF + 72, 26, "#1D2026")
    D.icon(f, "cameraswitch", W - M - 57, VF + 59, 26, WHITE)
    D.text(f, 0, VF + 136, meta, 11, "Regular", "#A3A8B0", w=W, align="center", font=MONO)
    D.text(f, 0, VF + 154, "Photo · video and audio coming later", 11, "Regular", "#6F747C", w=W, align="center")
    gesture(f, WHITE, 0.5)
    return f, th, sh


def finish(path):
    row_labels()
    by_name = {n: i for n, i in D.frames}
    for frame, ids in NAV.items():
        D.nav(ids, by_name[frame])
    D.write(path, S["A1"] if S["A1"] in by_name else S_HOME)
    print("wrote", path, "nodes:", len(D.nodes), "frames:", len(D.frames))
