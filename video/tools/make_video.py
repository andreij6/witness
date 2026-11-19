import json, math, os, subprocess, sys
from functools import lru_cache
from PIL import Image, ImageDraw, ImageFilter, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
VID = os.path.dirname(HERE)
DES = os.path.join(VID, "..", "design")
ASSETS = os.path.join(VID, "assets")
OUT = os.path.join(VID, "out")
WV, HV, FPS = 1920, 1080, 30
LEAD, GAP, TAIL, XF = 0.8, 0.7, 3.6, 0.5

INK = (14, 16, 20)
INK2 = (24, 28, 34)
LINE = (44, 50, 60)
WHITE = (240, 242, 245)
GREY = (163, 168, 176)
MUTED = (110, 117, 128)
GREEN = (52, 211, 153)
GREEN_D = (10, 135, 84)
TEAL = (34, 160, 190)
VIOLET = (167, 139, 250)
RED = (239, 99, 90)
AMBER = (245, 165, 36)
BLUE = (99, 118, 255)


def clamp(v, a=0.0, b=1.0):
    return max(a, min(b, v))


def ease(t):
    t = clamp(t)
    return 1 - (1 - t) ** 3


def ease_io(t):
    t = clamp(t)
    return 3 * t * t - 2 * t * t * t


def back(t):
    t = clamp(t)
    c = 1.70158
    return 1 + (c + 1) * (t - 1) ** 3 + c * (t - 1) ** 2


def appear(t, t0, d=0.5):
    return ease((t - t0) / d)


@lru_cache(maxsize=None)
def font(size, weight=400, mono=False):
    f = ImageFont.truetype(os.path.join(ASSETS, "jetbrains_mono.ttf" if mono else "inter.ttf"), size)
    f.set_variation_by_axes([weight] if mono else [min(32, max(14, size)), weight])
    return f


@lru_cache(maxsize=None)
def text_img(s, size, weight=400, color=WHITE, max_w=0, lh=1.22, mono=False, ls=0):
    f = font(size, weight, mono)
    words, lines, cur = s.split(" "), [], ""
    for w in words:
        trial = (cur + " " + w).strip()
        if max_w and f.getlength(trial) > max_w and cur:
            lines.append(cur)
            cur = w
        else:
            cur = trial
    lines.append(cur)
    wmax = int(max(f.getlength(l) + ls * len(l) for l in lines)) + 4
    hl = int(size * lh)
    im = Image.new("RGBA", (wmax, hl * len(lines) + int(size * 0.3)), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    for i, l in enumerate(lines):
        if ls:
            x = 0
            for ch in l:
                d.text((x, i * hl), ch, font=f, fill=color + (255,))
                x += f.getlength(ch) + ls
        else:
            d.text((0, i * hl), l, font=f, fill=color + (255,))
    return im


def fade(im, op):
    if op >= 0.999:
        return im
    a = im.getchannel("A").point(lambda v: int(v * op))
    out = im.copy()
    out.putalpha(a)
    return out


def put(base, im, x, y, op=1.0, scale=1.0):
    if op <= 0.003:
        return
    if abs(scale - 1) > 0.002:
        im = im.resize((max(1, int(im.width * scale)), max(1, int(im.height * scale))), Image.LANCZOS)
    base.alpha_composite(fade(im, op), (int(x), int(y)))


def rrect(w, h, r, fill, outline=None, width=2):
    im = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    ImageDraw.Draw(im).rounded_rectangle([0, 0, w - 1, h - 1], r, fill=fill, outline=outline, width=width)
    return im


@lru_cache(maxsize=None)
def shadow(w, h, r, blur=40, alpha=170):
    pad = blur * 2
    im = Image.new("RGBA", (w + pad * 2, h + pad * 2), (0, 0, 0, 0))
    ImageDraw.Draw(im).rounded_rectangle([pad, pad + 18, pad + w, pad + h + 18], r, fill=(0, 0, 0, alpha))
    return im.filter(ImageFilter.GaussianBlur(blur)), pad


@lru_cache(maxsize=None)
def render(name):
    return Image.open(os.path.join(DES, "renders", name + ".png")).convert("RGBA")


@lru_cache(maxsize=None)
def photo(name):
    return Image.open(os.path.join(DES, "images", name + ".png")).convert("RGBA")


@lru_cache(maxsize=None)
def phone(name, h):
    src = render(name)
    w = int(src.width * h / src.height)
    scr = src.resize((w, h), Image.LANCZOS)
    r = int(w * 0.075)
    mask = Image.new("L", (w, h), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, w - 1, h - 1], r, fill=255)
    b = int(w * 0.028)
    body = rrect(w + 2 * b, h + 2 * b, r + b, (8, 9, 11, 255), outline=(58, 64, 74, 255), width=2)
    body.paste(scr, (b, b), mask)
    sh, pad = shadow(body.width, body.height, r + b)
    out = Image.new("RGBA", (sh.width, sh.height), (0, 0, 0, 0))
    out.alpha_composite(sh)
    out.alpha_composite(body, (pad, pad))
    return out, pad, b


def put_phone(base, name, h, x, y, op=1.0, scale=1.0):
    im, pad, b = phone(name, h)
    put(base, im, x - pad * scale, y - pad * scale, op, scale)


def phone_point(name, h, x, y, px, py):
    src = render(name)
    k = h / src.height
    _, pad, b = phone(name, h)
    return x + b + px * 2 * k, y + b + py * 2 * k


@lru_cache(maxsize=None)
def cover(name, w, h, blur=0, dark=0.0):
    im = photo(name)
    k = max(w / im.width, h / im.height)
    im = im.resize((int(im.width * k) + 1, int(im.height * k) + 1), Image.LANCZOS)
    x, y = (im.width - w) // 2, (im.height - h) // 2
    im = im.crop((x, y, x + w, y + h))
    if blur:
        im = im.filter(ImageFilter.GaussianBlur(blur))
    if dark:
        im = Image.blend(im, Image.new("RGBA", im.size, INK + (255,)), dark)
    return im


@lru_cache(maxsize=None)
def backdrop(tint=(40, 60, 110), cx=0.5, cy=0.35, strength=0.55):
    small = Image.new("RGB", (192, 108), INK)
    px = small.load()
    for y in range(108):
        for x in range(192):
            d = math.hypot((x / 192 - cx) * 1.6, y / 108 - cy)
            k = max(0.0, 1 - d / 0.9) ** 2 * strength
            px[x, y] = tuple(int(INK[i] + (tint[i] - INK[i]) * k) for i in range(3))
    im = small.resize((WV, HV), Image.BICUBIC).convert("RGBA")
    grid = Image.new("RGBA", (WV, HV), (0, 0, 0, 0))
    d = ImageDraw.Draw(grid)
    for gx in range(0, WV, 64):
        d.line([(gx, 0), (gx, HV)], fill=(255, 255, 255, 9))
    for gy in range(0, HV, 64):
        d.line([(0, gy), (WV, gy)], fill=(255, 255, 255, 9))
    im.alpha_composite(grid)
    return im


@lru_cache(maxsize=None)
def app_icon(size):
    im = Image.open(os.path.join(ASSETS, "icon.png")).convert("RGBA").resize((size, size), Image.LANCZOS)
    m = Image.new("L", (size, size), 0)
    ImageDraw.Draw(m).rounded_rectangle([0, 0, size - 1, size - 1], int(size * 0.2296), fill=255)
    im.putalpha(m)
    glow = Image.new("RGBA", (size + 160, size + 160), (0, 0, 0, 0))
    ImageDraw.Draw(glow).rounded_rectangle([80, 80, 80 + size, 80 + size], int(size * 0.23), fill=(52, 211, 153, 70))
    glow = glow.filter(ImageFilter.GaussianBlur(40))
    glow.alpha_composite(im, (80, 80))
    return glow


def kicker(base, s, x, y, op, color=GREEN):
    put(base, text_img(s.upper(), 22, 600, color, mono=True, ls=2), x, y, op)


def headline(base, s, x, y, op, size=64, max_w=860, color=WHITE, rise=24):
    put(base, text_img(s, size, 700, color, max_w=max_w, lh=1.12), x, y + (1 - op) * rise, op)


def body(base, s, x, y, op, size=34, max_w=820, color=GREY, weight=400):
    put(base, text_img(s, size, weight, color, max_w=max_w, lh=1.3), x, y + (1 - op) * 16, op)


@lru_cache(maxsize=None)
def check_dot(color, r=22, glyph="check"):
    im = Image.new("RGBA", (r * 2, r * 2), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    d.ellipse([0, 0, r * 2 - 1, r * 2 - 1], fill=color + (40,), outline=color + (255,), width=2)
    if glyph == "check":
        d.line([(r * 0.55, r * 1.02), (r * 0.88, r * 1.35), (r * 1.48, r * 0.7)], fill=color + (255,), width=4, joint="curve")
    elif glyph == "x":
        d.line([(r * 0.65, r * 0.65), (r * 1.35, r * 1.35)], fill=color + (255,), width=4)
        d.line([(r * 1.35, r * 0.65), (r * 0.65, r * 1.35)], fill=color + (255,), width=4)
    elif glyph == "q":
        d.text((r * 0.62, r * 0.28), "?", font=font(int(r * 1.2), 700), fill=color + (255,))
    elif glyph == "eq":
        d.line([(r * 0.6, r * 0.82), (r * 1.4, r * 0.82)], fill=color + (255,), width=4)
        d.line([(r * 0.6, r * 1.18), (r * 1.4, r * 1.18)], fill=color + (255,), width=4)
    return im


def bullet(base, s, x, y, op, color=GREEN, glyph="check", size=36):
    put(base, check_dot(color, 22, glyph), x, y + 2 + (1 - op) * 12, op)
    put(base, text_img(s, size, 500, WHITE, max_w=760), x + 64, y + (1 - op) * 12, op)


@lru_cache(maxsize=None)
def pill(s, color, solid=False, size=30, pad=26, h=60):
    f = font(size, 600)
    w = int(f.getlength(s)) + pad * 2
    im = rrect(w, h, h // 2, color + ((255,) if solid else (36,)), outline=color + (255,), width=2)
    ImageDraw.Draw(im).text((pad, (h - size * 1.2) / 2), s, font=f, fill=(WHITE if solid else color) + (255,))
    return im


def frac(text, phrase):
    i = text.find(phrase)
    return 0.0 if i < 0 else i / max(1, len(text))


class Scene:
    def __init__(self, sid, text, dur, a0):
        self.sid, self.text, self.adur, self.a0 = sid, text, dur, a0

    def at(self, phrase, extra=0.0):
        return self.a0 + frac(self.text, phrase) * self.adur + extra


def s01(sc, t):
    im = cover("hail", WV, HV, 26, 0.62).copy()
    push = 1.0 + 0.04 * clamp(t / sc.len)
    cw, phh = 560, 560
    card = rrect(cw, 800, 26, (250, 250, 248, 255))
    d = ImageDraw.Draw(card)
    d.ellipse([22, 20, 66, 64], fill=(214, 210, 200, 255))
    d.text((80, 18), "@stormwatch.den", font=font(26, 650), fill=(20, 22, 26, 255))
    d.text((80, 50), "Denver · 4:12 PM", font=font(20, 400), fill=(110, 116, 126, 255))
    card.paste(cover("hail", cw, phh), (0, 84))
    for k in range(3):
        d.ellipse([24 + k * 52, 668, 56 + k * 52, 700], outline=(20, 22, 26, 255), width=3)
    d.text((24, 716), "1,904 likes", font=font(24, 650), fill=(20, 22, 26, 255))
    d.text((24, 752), "Hail out of nowhere in Denver.", font=font(23, 400), fill=(60, 64, 72, 255))
    sh, pad = shadow(cw, 800, 26, 50, 200)
    op = appear(t, 0.1, 0.7)
    cx, cy = 470, 140
    put(im, sh, cx - pad, cy - pad, op, 1.0)
    put(im, card, cx + (1 - push) * cw / 2, cy + (1 - push) * 400, op, push)
    replies = (("AI.", "@kev_92", sc.at('"AI."')), ("Fake.", "@truthseeker", sc.at('"Fake."')), ("Photoshopped.", "@mara.k", sc.at('"Photoshopped."')))
    for k, (s, who, t0) in enumerate(replies):
        o = appear(t, t0 - 0.15, 0.4)
        bw = int(font(52, 750).getlength(s)) + 140
        b = rrect(max(bw, 340), 150, 30, (22, 25, 30, 235), outline=(70, 78, 90, 255))
        db = ImageDraw.Draw(b)
        db.ellipse([28, 26, 64, 62], fill=((239, 99, 90, 255), (245, 165, 36, 255), (167, 139, 250, 255))[k])
        db.text((78, 28), who, font=font(24, 600), fill=GREY + (255,))
        db.text((30, 76), s, font=font(52, 750), fill=WHITE + (255,))
        put(im, b, 1150 + (1 - o) * 80 + k * 40, 210 + k * 200, o)
    return im


def s02(sc, t):
    im = backdrop((90, 30, 40), 0.75, 0.4, 0.5).copy()
    headline(im, "Anything can be faked in seconds.", 140, 250, appear(t, sc.at("Anything") - 0.1), 76, 860)
    body(im, "Real moments get dismissed.", 140, 520, appear(t, sc.at("real moments")), 42, 860, WHITE, 500)
    body(im, "Fakes get believed.", 140, 586, appear(t, sc.at("fakes get")), 42, 860, RED, 600)
    x0, y0, w = 1130, 250, 640
    o = appear(t, sc.at("metadata") - 0.3, 0.5)
    card = rrect(w, 470, 26, (22, 25, 30, 245), outline=(58, 64, 74, 255))
    put(im, card, x0, y0, o)
    kicker(im, "Photo metadata", x0 + 36, y0 + 34, o, GREY)
    rows = (("Taken", "Nov 18 · 4:12 PM"), ("Place", "39.7392, −104.9903"), ("Device", "Android phone"), ("Edited", "unknown"))
    ts = sc.at("Every platform")
    for k, (a, b) in enumerate(rows):
        ry = y0 + 100 + k * 76
        strike = appear(t, ts + k * 0.32, 0.35)
        rop = o * (1 - 0.72 * appear(t, ts + k * 0.32 + 0.3, 0.4))
        put(im, text_img(a, 30, 500, GREY), x0 + 36, ry, rop)
        put(im, text_img(b, 30, 500, WHITE, mono=True), x0 + 200, ry, rop)
        if strike > 0:
            ImageDraw.Draw(im).line([(x0 + 30, ry + 20), (x0 + 30 + (w - 60) * strike, ry + 20)], fill=RED + (230,), width=4)
    so = appear(t, ts + 1.5, 0.4)
    put(im, pill("Stripped on upload", RED, True, 30), x0 + 36, y0 + 400, so)
    return im


def s03(sc, t):
    im = backdrop((20, 90, 70), 0.5, 0.42, 0.6).copy()
    icon = app_icon(300)
    s = 0.55 + 0.45 * back((t - 0.15) / 0.8)
    o = appear(t, 0.1, 0.5)
    put(im, icon, WV / 2 - icon.width / 2 * s, 170 + icon.height / 2 * (1 - s), o, s)
    put(im, text_img("Witness", 110, 760, WHITE), WV / 2 - text_img("Witness", 110, 760, WHITE).width / 2, 590, appear(t, sc.at("Witness"), 0.5))
    tag = text_img("Proof, not just pictures.", 50, 500, GREEN)
    put(im, tag, WV / 2 - tag.width / 2, 740 + (1 - appear(t, sc.at("Proof"), 0.6)) * 20, appear(t, sc.at("Proof"), 0.6))
    return im


def s04(sc, t):
    im = backdrop((30, 50, 110), 0.25, 0.45, 0.55).copy()
    o = appear(t, 0.0, 0.7)
    sw = appear(t, sc.at("Even offline") - 0.2, 0.6)
    py = 70 + (1 - o) * 60
    put_phone(im, "b1-camera-post", 940, 280, py, o * (1 - sw))
    put_phone(im, "b2-camera-sealed-offline", 940, 280, py, o * sw)
    flash_t = sc.at("seals the photo")
    fl = max(0.0, 1 - abs(t - flash_t - 0.1) / 0.25)
    if fl > 0:
        _, pad, b = phone("b1-camera-post", 940)
        w = int(824 * 940 / 1830)
        put(im, rrect(w, 940, int(w * 0.075), (255, 255, 255, 255)), 280 + b, py + b, fl * 0.7)
    kicker(im, "Sealed at the shutter", 900, 210, appear(t, 0.3))
    headline(im, "Your phone's security chip signs every photo.", 900, 256, appear(t, 0.45), 62, 900)
    items = (("The time", sc.at("the time")), ("The place", sc.at("the place")), ("A genuine device", sc.at("genuine device")),
             ("Even offline", sc.at("Even offline")))
    for k, (s, t0) in enumerate(items):
        bullet(im, s, 900, 520 + k * 86, appear(t, t0 - 0.1, 0.4), AMBER if k == 3 else GREEN)
    return im


def s05(sc, t):
    im = backdrop((80, 50, 140), 0.7, 0.4, 0.55).copy()
    o = appear(t, 0.0, 0.6)
    put_phone(im, "b3-post-composer", 940, 170, 70 + (1 - o) * 50, o)
    kicker(im, "Before you share", 820, 150, appear(t, 0.3), VIOLET)
    headline(im, "Private by default.", 820, 196, appear(t, 0.4), 66, 980)
    zo = appear(t, sc.at("faces") - 0.3, 0.6)
    src = render("b3-post-composer")
    crop = src.crop((32, 402 * 2, 32 + 380 * 2, 614 * 2))
    k = 900 / crop.width
    crop = crop.resize((900, int(crop.height * k)), Image.LANCZOS)
    m = Image.new("L", crop.size, 0)
    ImageDraw.Draw(m).rounded_rectangle([0, 0, crop.width - 1, crop.height - 1], 34, fill=255)
    cc = Image.new("RGBA", crop.size, (0, 0, 0, 0))
    cc.paste(crop, (0, 0), m)
    sh, pad = shadow(crop.width, crop.height, 34, 40, 160)
    zs = 0.92 + 0.08 * zo
    put(im, sh, 820 - pad, 330 - pad, zo)
    put(im, cc, 820, 330 + (1 - zo) * 30, zo, zs)
    items = (("Faces, plates & house numbers blurred", sc.at("faces")), ("Exact location kept private", sc.at("exact location")),
             ("On-device — nothing leaves your phone", sc.at("None of it")))
    for kk, (s, t0) in enumerate(items):
        bullet(im, s, 820, 845 + kk * 64, appear(t, t0, 0.4), VIOLET if kk < 2 else GREEN, size=32)
    return im


def s06(sc, t):
    im = backdrop((20, 110, 140), 0.3, 0.4, 0.55).copy()
    kicker(im, "Survives screenshots & reposts", 120, 120, appear(t, 0.2), TEAL)
    headline(im, "Even a cropped screenshot traces back.", 120, 166, appear(t, 0.35), 60, 700)
    src = render("d7-check-result")
    shot = src.crop((64, 524, 400, 1076))
    hh = 560
    shot = shot.resize((int(shot.width * hh / shot.height), hh), Image.LANCZOS)
    so = appear(t, 0.4, 0.6)
    sh, pad = shadow(shot.width, shot.height, 22, 40, 160)
    put(im, sh, 170 - pad, 400 - pad, so)
    put(im, shot, 170, 400 + (1 - so) * 30, so)
    put(im, pill("Invisible watermark", TEAL, False, 26, 22, 52), 120 + 520, 430, appear(t, sc.at("invisible watermark"), 0.4))
    put(im, pill("Visual fingerprints", TEAL, False, 26, 22, 52), 120 + 520, 500, appear(t, sc.at("visual fingerprints"), 0.4))
    ao = appear(t, sc.at("traces back") - 0.2, 0.7)
    if ao > 0:
        d = ImageDraw.Draw(im)
        x1, x2, y = 660, 660 + 440 * ao, 640
        d.line([(x1, y), (x2, y)], fill=TEAL + (255,), width=6)
        if ao > 0.95:
            d.polygon([(x2 + 18, y), (x2 - 6, y - 16), (x2 - 6, y + 16)], fill=TEAL + (255,))
    po = appear(t, sc.at("traces back") - 0.1, 0.7)
    put_phone(im, "d7-check-result", 940, 1180, 70 + (1 - po) * 60, po)
    ho = appear(t, sc.at("every change listed") - 0.2, 0.4)
    if ho > 0:
        x, y = phone_point("d7-check-result", 940, 1180, 70, 16, 578)
        x2, y2 = phone_point("d7-check-result", 940, 1180, 70, 396, 754)
        lay = Image.new("RGBA", (WV, HV), (0, 0, 0, 0))
        ImageDraw.Draw(lay).rounded_rectangle([x - 6, y - 6, x2 + 6, y2 + 6], 22, outline=TEAL + (int(255 * ho),), width=5)
        im.alpha_composite(lay)
    return im


def s07(sc, t):
    im = backdrop((110, 40, 50), 0.35, 0.4, 0.45).copy()
    o1 = appear(t, 0.0, 0.6)
    o2 = appear(t, sc.at("No proof") - 0.3, 0.6)
    put_phone(im, "w5-altered", 820, 110, 120 + (1 - o1) * 50, o1)
    put_phone(im, "w6-no-record", 820, 520, 120 + (1 - o2) * 50, o2)
    kicker(im, "Honest verdicts", 1010, 150, appear(t, 0.2), GREY)
    headline(im, "Every check ends in one clear answer.", 1010, 196, appear(t, 0.3), 54, 820)
    rows = (("Original", GREEN_D, "check"), ("Matches an original · with changes", TEAL, "eq"), ("Altered", RED, "x"), ("No Witness record", MUTED, "q"))
    hi = {2: sc.at("Someone edits"), 3: sc.at("No proof")}
    for k, (s, c, g) in enumerate(rows):
        ro = appear(t, 0.5 + k * 0.25, 0.4)
        lit = appear(t, hi.get(k, 99), 0.4)
        y = 380 + k * 92
        put(im, rrect(820, 74, 37, c + (int(30 + 60 * lit),), outline=c + (255,), width=2 + int(2 * lit)), 1010, y, ro)
        put(im, check_dot(c, 20, g), 1030, y + 17, ro)
        put(im, text_img(s, 32, 600 if lit > 0.5 else 500, WHITE), 1090, y + 16, ro)
    qo = appear(t, sc.at("Not proven") - 0.1, 0.6)
    put(im, text_img("“Not proven” isn’t the same as fake.", 46, 700, WHITE, max_w=820), 1010, 790 + (1 - qo) * 20, qo)
    return im


def s08(sc, t):
    im = backdrop((40, 70, 130), 0.5, 0.25, 0.55).copy()
    headline(im, "Anyone can check — in any browser.", 120, 70, appear(t, 0.1), 64, 1400)
    src = render("x1-desktop-proof-page")
    dw = 1280
    desk = src.resize((dw, int(src.height * dw / src.width)), Image.LANCZOS)
    m = Image.new("L", desk.size, 0)
    ImageDraw.Draw(m).rounded_rectangle([0, 0, desk.width - 1, desk.height - 1], 18, fill=255)
    dd = Image.new("RGBA", desk.size, (0, 0, 0, 0))
    dd.paste(desk, (0, 0), m)
    sh, pad = shadow(desk.width, desk.height, 18, 50, 180)
    do = appear(t, 0.2, 0.7)
    put(im, sh, 120 - pad, 210 - pad + (1 - do) * 40, do)
    put(im, dd, 120, 210 + (1 - do) * 40, do)
    po = appear(t, sc.at("on any phone") - 0.2, 0.6)
    put_phone(im, "w1-proof-page", 800, 1420, 230 + (1 - po) * 60, po)
    x = 120
    for k, (s, t0) in enumerate((("No app", sc.at("No app")), ("No account", sc.at("No account")))):
        p = pill(s, GREEN, True, 30, 28, 62)
        put(im, p, x, 1000 - 10 * (1 - appear(t, t0, 0.4)) - 10, appear(t, t0 - 0.05, 0.35))
        x += p.width + 16
    return im


def s09(sc, t):
    im = backdrop((90, 60, 150), 0.72, 0.45, 0.5).copy()
    o1 = appear(t, 0.0, 0.6)
    o2 = appear(t, sc.at("shares it") - 0.4, 0.6)
    put_phone(im, "c5-case", 900, 960, 90 + (1 - o1) * 60, o1)
    put_phone(im, "c9-share-case", 900, 1400, 90 + (1 - o2) * 60, o2)
    kicker(im, "Case mode", 120, 220, appear(t, 0.2), VIOLET)
    headline(im, "For the moments that really matter.", 120, 266, appear(t, 0.3), 62, 760)
    items = (("The report builds itself, on-device", sc.at("builds the report"), VIOLET),
             ("Every photo sealed and anchored", sc.at("builds the report") + 0.9, GREEN),
             ("Shared with an end-to-end encrypted link", sc.at("end-to-end"), GREEN))
    for k, (s, t0, c) in enumerate(items):
        bullet(im, s, 120, 560 + k * 92, appear(t, t0, 0.4), c, size=34)
    return im


def s10(sc, t):
    split = sc.at("Witness.") - 0.25
    if t < split + 0.6:
        im = backdrop((20, 110, 80), 0.5, 0.5, 0.5).copy()
        a = 1 - appear(t, split, 0.6)
        kicker(im, "Anchored on the Internet Computer", 140, 170, appear(t, 0.2) * a, GREEN)
        headline(im, "A neutral record no one can quietly rewrite.", 140, 216, appear(t, sc.at("a neutral") - 0.2) * a, 66, 1300)
        lay = Image.new("RGBA", (WV, HV), (0, 0, 0, 0))
        d = ImageDraw.Draw(lay)
        n, bw, gap = 9, 168, 30
        shift = (t * 70) % (bw + gap)
        y = 560
        for k in range(-1, n + 1):
            x = 140 + k * (bw + gap) - shift
            mid = k == 4
            col = GREEN if mid else (70, 78, 90)
            alpha = int(255 * a * appear(t, 0.3 + abs(k - 4) * 0.08, 0.4))
            if alpha <= 0:
                continue
            d.rounded_rectangle([x, y, x + bw, y + 150], 22, fill=(GREEN + (int(55 * a),)) if mid else (26, 30, 36, alpha), outline=col + (alpha,), width=3)
            d.text((x + 20, y + 22), f"#{2108729 + k}", font=font(22, 500, True), fill=GREY + (alpha,))
            d.text((x + 20, y + 62), ("H4IL-7QK" if mid else ["9f3a", "41b0", "7c1e", "c21e", "77de", "a90f"][k % 6] + "…"), font=font(28, 700, True), fill=(WHITE if mid else GREY) + (alpha,))
            d.text((x + 20, y + 104), "sealed" if mid else "anchored", font=font(20, 500), fill=(GREEN if mid else MUTED) + (alpha,))
            if k < n:
                d.line([(x + bw, y + 75), (x + bw + gap, y + 75)], fill=LINE + (alpha,), width=3)
        im.alpha_composite(lay)
        return im if t < split else Image.alpha_composite(im, fade(endcard(sc, t), appear(t, split, 0.6)))
    return endcard(sc, t)


def endcard(sc, t):
    im = backdrop((20, 90, 70), 0.5, 0.42, 0.6).copy()
    icon = app_icon(220)
    t0 = sc.at("Witness.") - 0.25
    put(im, icon, WV / 2 - icon.width / 2, 70, appear(t, t0, 0.5))
    wm = text_img("Witness", 96, 760, WHITE)
    put(im, wm, WV / 2 - wm.width / 2, 400, appear(t, sc.at("Witness."), 0.5))
    beats = (("Capture it.", sc.at("Capture it."), WHITE), ("Seal it.", sc.at("Seal it."), WHITE), ("Prove it.", sc.at("Prove it."), GREEN))
    ims = [text_img(s, 58, 700, c) for s, _, c in beats]
    total = sum(i.width for i in ims) + 2 * 60
    x = WV / 2 - total / 2
    for (s, tb, c), img in zip(beats, ims):
        o = appear(t, tb - 0.05, 0.4)
        put(im, img, x, 560 + (1 - o) * 18, o)
        x += img.width + 60
    fo = appear(t, sc.at("Prove it.") + 1.0, 0.6)
    foot = text_img("Coming to Android   ·   Check any photo at witness.app", 34, 500, GREY)
    put(im, foot, WV / 2 - foot.width / 2, 760, fo)
    return im


RENDER = [s01, s02, s03, s04, s05, s06, s07, s08, s09, s10]


def build():
    scenes = json.load(open(os.path.join(HERE, "scenes.json")))
    durs = [float(subprocess.check_output(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0",
                                           os.path.join(VID, "audio", "trim", s["id"] + ".wav")])) for s in scenes]
    starts, t = [], LEAD
    for d in durs:
        starts.append(t)
        t += d + GAP
    total = starts[-1] + durs[-1] + TAIL
    objs = []
    for i, s in enumerate(scenes):
        vs = 0.0 if i == 0 else starts[i] - 0.45
        ve = total if i == len(scenes) - 1 else starts[i + 1] - 0.45
        sc = Scene(s["id"], s["text"], durs[i], starts[i] - vs)
        sc.vs, sc.ve, sc.len = vs, ve, ve - vs
        objs.append(sc)
    return scenes, durs, starts, total, objs


def frame(objs, total, t):
    for i, sc in enumerate(objs):
        if sc.vs <= t < sc.ve or i == len(objs) - 1:
            im = RENDER[i](sc, t - sc.vs)
            if i + 1 < len(objs) and t > sc.ve - XF / 2:
                nx = objs[i + 1]
                k = ease_io((t - (sc.ve - XF / 2)) / XF)
                im = Image.blend(im, RENDER[i + 1](nx, t - nx.vs), k).convert("RGBA")
            if i > 0 and t < sc.vs + XF / 2:
                pv = objs[i - 1]
                k = ease_io((t - (sc.vs - XF / 2)) / XF)
                im = Image.blend(RENDER[i - 1](pv, t - pv.vs), im, k).convert("RGBA")
            if t > total - 0.7:
                im = Image.blend(im, Image.new("RGBA", im.size, (0, 0, 0, 255)), clamp((t - (total - 0.7)) / 0.7))
            if t < 0.4:
                im = Image.blend(Image.new("RGBA", im.size, (0, 0, 0, 255)), im, clamp(t / 0.4))
            return im.convert("RGB")


def srt(scenes, durs, starts, path):
    def ts(x):
        ms = int(round(x * 1000))
        return f"{ms // 3600000:02d}:{ms // 60000 % 60:02d}:{ms // 1000 % 60:02d},{ms % 1000:03d}"
    out, n = [], 1
    for s, d, st in zip(scenes, durs, starts):
        parts = [p.strip() for p in s["text"].replace("? ", "?|").replace(". ", ".|").replace(": ", ":|").split("|") if p.strip()]
        lens = [len(p) for p in parts]
        t = st
        for p, l in zip(parts, lens):
            pd = d * l / sum(lens)
            out.append(f"{n}\n{ts(t)} --> {ts(t + pd)}\n{p}\n")
            n += 1
            t += pd
    open(path, "w").write("\n".join(out))


def audio(scenes, starts, total, path):
    args = ["ffmpeg", "-v", "error", "-y"]
    for s in scenes:
        args += ["-i", os.path.join(VID, "audio", "trim", s["id"] + ".wav")]
    pad = ("aevalsrc=exprs='0.05*sin(2*PI*110*t)*(0.7+0.3*sin(2*PI*0.07*t))+0.035*sin(2*PI*164.81*t)*(0.7+0.3*sin(2*PI*0.05*t+1))"
           "+0.03*sin(2*PI*220*t)*(0.6+0.4*sin(2*PI*0.09*t+2))+0.02*sin(2*PI*277.18*t)*(0.6+0.4*sin(2*PI*0.06*t+3))'"
           f":s=48000:d={total:.3f}")
    fc = [f"{pad},lowpass=f=900,afade=t=in:d=2.5,afade=t=out:st={total - 3:.3f}:d=3,volume=0.55[pad]"]
    for i, st in enumerate(starts):
        ms = int(st * 1000)
        fc.append(f"[{i}:a]aresample=48000,adelay={ms}|{ms},apad=whole_dur={total:.3f}[v{i}]")
    fc.append("".join(f"[v{i}]" for i in range(len(starts))) + f"amix=inputs={len(starts)}:normalize=0[voice]")
    fc.append("[voice][pad]amix=inputs=2:normalize=0:weights=1 0.35,atrim=0:" + f"{total:.3f}" + ",loudnorm=I=-16:TP=-1.5:LRA=11,aresample=48000[out]")
    args += ["-filter_complex", ";".join(fc), "-map", "[out]", "-ac", "2", path]
    subprocess.run(args, check=True)


def main():
    os.makedirs(OUT, exist_ok=True)
    scenes, durs, starts, total, objs = build()
    mode = sys.argv[1] if len(sys.argv) > 1 else "all"
    if mode == "stills":
        for i, sc in enumerate(objs):
            for f in (0.35, 0.75, 0.97):
                frame(objs, total, sc.vs + sc.len * f).save(os.path.join(OUT, f"still-{i + 1:02d}-{int(f * 100)}.jpg"), quality=88)
        print("total", round(total, 2))
        return
    wav, sub, mp4 = os.path.join(OUT, "narration.wav"), os.path.join(OUT, "witness-pitch.srt"), os.path.join(OUT, "witness-pitch.mp4")
    audio(scenes, starts, total, wav)
    srt(scenes, durs, starts, sub)
    n = int(total * FPS)
    enc = subprocess.Popen(["ffmpeg", "-v", "error", "-y", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", f"{WV}x{HV}", "-r", str(FPS), "-i", "-",
                            "-i", wav, "-i", sub, "-map", "0:v", "-map", "1:a", "-map", "2:s", "-c:v", "libx264", "-preset", "slow", "-crf", "18",
                            "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "192k", "-c:s", "mov_text", "-metadata:s:s:0", "language=eng",
                            "-movflags", "+faststart", "-t", f"{total:.3f}", mp4], stdin=subprocess.PIPE)
    for k in range(n):
        enc.stdin.write(frame(objs, total, k / FPS).tobytes())
        if k % 300 == 0:
            print(f"{k}/{n}", flush=True)
    enc.stdin.close()
    enc.wait()
    print("wrote", mp4, round(total, 2), "s")


if __name__ == "__main__":
    main()
