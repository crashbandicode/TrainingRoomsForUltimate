#!/usr/bin/python3 -I
"""Render the proposed TrainingRooms menu (MENU-DRAFT.md §2, proposal v3) as PNG and SVG screens in the
in-game overlay look, 1280x720 (Switch handheld).

v3 adds the fork's grid pickers (portrait roster grid 13x7, stage thumbnail pages 4x4, item grid 4x8),
a 3x3 direction grid for DI / SDI / Airdodge / Shield Tilt, and three pictogram chips (Kirby hats,
Solo Popo, character items) drawn from primitives. Every screen is drawn once through `Canvas`, which
records the primitives and writes both the PNG (Pillow) and a text SVG, so the two never drift.

Output: docs/menu-mockup/NN-<screen>.png and .svg plus index.md. Pillow only (no external converter on
this host); DejaVu Sans Mono for the modpack-style monospace text. No game asset, no third-party
artwork: portraits, stage thumbnails and item icons are stand-ins for the images the fork's own
builders derive from the player's game files at build time.
"""
import math
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

W, H = 1280, 720
OUT = Path(__file__).resolve().parent.parent / "docs" / "menu-mockup"
FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
FONT_B = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf"

# colours (the modpack overlay: near-black translucent panel, white text, red selection)
BG0, BG1 = (18, 22, 30), (40, 46, 58)
PANEL = (10, 12, 16, 226)
LINE = (70, 76, 90)
FG = (236, 236, 232)
MUTED = (150, 156, 170)
SEL = (176, 52, 42)
OURS = (111, 195, 154)
UP = (168, 155, 240)
ADV = (210, 170, 80)
HILITE = (38, 42, 52)  # selected row: a solid lift over the panel (Pillow's RGBA draw does not blend alpha fills)
TILE = (28, 32, 42)
TILE_HI = (46, 52, 66)
P1 = (232, 72, 72)      # the fork's red Player order
CPU = (72, 150, 232)    # the fork's blue CPU order
INK = (14, 16, 20)      # pictogram fills on the dark panel

FONTS = {"tab": (FONT_B, 22), "h": (FONT_B, 20), "t": (FONT, 20), "s": (FONT, 16), "xs": (FONT, 14),
         "xxs": (FONT, 12), "b": (FONT_B, 13), "hh": (FONT_B, 24), "tiny": (FONT, 10)}


# ---------------------------------------------------------------------------------------------------
# Canvas: one drawing API, two outputs
# ---------------------------------------------------------------------------------------------------
def rgb(c):
    return f"rgb({c[0]},{c[1]},{c[2]})"


def opacity(c):
    return f' fill-opacity="{c[3] / 255:.2f}"' if len(c) == 4 else ""


def esc(s):
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


class Canvas:
    def __init__(self):
        self.img = Image.new("RGB", (W, H), BG0)
        px = self.img.load()
        for y in range(H):  # vertical gradient, a hint of the stage behind the overlay
            t = y / H
            c = tuple(int(BG0[i] * (1 - t) + BG1[i] * t) for i in range(3))
            for x in range(W):
                px[x, y] = c
        self.img = self.img.convert("RGBA")
        self.d = ImageDraw.Draw(self.img, "RGBA")
        self.fonts = {k: ImageFont.truetype(p, s) for k, (p, s) in FONTS.items()}
        self.svg = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}" '
                    f'font-family="DejaVu Sans Mono, monospace">',
                    f'<defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{rgb(BG0)}"/>'
                    f'<stop offset="1" stop-color="{rgb(BG1)}"/></linearGradient></defs>',
                    f'<rect width="{W}" height="{H}" fill="url(#g)"/>']
        self.rect(0, 560, W, 600, fill=(52, 58, 72))  # a platform line, dimmed

    # -- primitives -------------------------------------------------------------------------------
    def rect(self, x0, y0, x1, y1, fill=None, outline=None, width=1, radius=0):
        if radius:
            self.d.rounded_rectangle([x0, y0, x1, y1], radius=radius, fill=fill, outline=outline, width=width)
        else:
            self.d.rectangle([x0, y0, x1, y1], fill=fill, outline=outline, width=width)
        f = f'fill="{rgb(fill)}"{opacity(fill)}' if fill else 'fill="none"'
        o = f' stroke="{rgb(outline)}" stroke-width="{width}"' if outline else ""
        r = f' rx="{radius}"' if radius else ""
        self.svg.append(f'<rect x="{x0:.1f}" y="{y0:.1f}" width="{x1 - x0:.1f}" height="{y1 - y0:.1f}"{r} {f}{o}/>')

    def ellipse(self, x0, y0, x1, y1, fill=None, outline=None, width=1):
        self.d.ellipse([x0, y0, x1, y1], fill=fill, outline=outline, width=width)
        f = f'fill="{rgb(fill)}"' if fill else 'fill="none"'
        o = f' stroke="{rgb(outline)}" stroke-width="{width}"' if outline else ""
        self.svg.append(f'<ellipse cx="{(x0 + x1) / 2:.1f}" cy="{(y0 + y1) / 2:.1f}" rx="{(x1 - x0) / 2:.1f}" '
                        f'ry="{(y1 - y0) / 2:.1f}" {f}{o}/>')

    def circle(self, cx, cy, r, fill=None, outline=None, width=1):
        self.ellipse(cx - r, cy - r, cx + r, cy + r, fill, outline, width)

    def line(self, pts, fill, width=1, dash=None):
        if dash:
            for a, b in dashed(pts, *dash):
                self.d.line([a, b], fill=fill, width=width)
        else:
            self.d.line(pts, fill=fill, width=width, joint="curve")
        d = f' stroke-dasharray="{dash[0]} {dash[1]}"' if dash else ""
        p = " ".join(f"{x:.1f},{y:.1f}" for x, y in pts)
        self.svg.append(f'<polyline points="{p}" fill="none" stroke="{rgb(fill)}" stroke-width="{width}" '
                        f'stroke-linecap="round" stroke-linejoin="round"{d}/>')

    def polygon(self, pts, fill=None, outline=None, width=1):
        self.d.polygon(pts, fill=fill, outline=outline, width=width if outline else 0)
        f = f'fill="{rgb(fill)}"' if fill else 'fill="none"'
        o = f' stroke="{rgb(outline)}" stroke-width="{width}" stroke-linejoin="round"' if outline else ""
        p = " ".join(f"{x:.1f},{y:.1f}" for x, y in pts)
        self.svg.append(f'<polygon points="{p}" {f}{o}/>')

    def arc(self, cx, cy, rx, ry, start, end, fill, width=2, dash=None):
        """Elliptical arc, angles in degrees clockwise from +x (screen coordinates), as a polyline."""
        n = max(8, int(abs(end - start) / 6))
        pts = [(cx + rx * math.cos(math.radians(start + (end - start) * i / n)),
                cy + ry * math.sin(math.radians(start + (end - start) * i / n))) for i in range(n + 1)]
        self.line(pts, fill, width, dash)

    def text(self, x, y, s, font="t", fill=FG, anchor="l"):
        """(x, y) is the top of the text line; anchor l / m / r."""
        self.d.text((x, y), s, font=self.fonts[font], fill=fill, anchor=anchor + "a")
        size = FONTS[font][1]
        ta = {"l": "start", "m": "middle", "r": "end"}[anchor]
        bold = ' font-weight="bold"' if FONTS[font][0] == FONT_B else ""
        self.svg.append(f'<text x="{x:.1f}" y="{y + size * 0.8:.1f}" font-size="{size}" fill="{rgb(fill)}" '
                        f'text-anchor="{ta}"{bold}>{esc(s)}</text>')

    def textlength(self, s, font="t"):
        return self.d.textlength(s, font=self.fonts[font])

    def save(self, slug):
        self.img.convert("RGB").save(OUT / f"{slug}.png")
        (OUT / f"{slug}.svg").write_text("\n".join(self.svg + ["</svg>"]) + "\n", encoding="utf-8")


def dashed(pts, on, off):
    """Split a polyline into dash segments of length `on` separated by `off`."""
    out, carry, drawing = [], 0.0, True
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        seg = math.hypot(x1 - x0, y1 - y0)
        if seg == 0:
            continue
        t = 0.0
        while t < seg:
            run = (on if drawing else off) - carry
            step = min(run, seg - t)
            if drawing:
                a = (x0 + (x1 - x0) * t / seg, y0 + (y1 - y0) * t / seg)
                b = (x0 + (x1 - x0) * (t + step) / seg, y0 + (y1 - y0) * (t + step) / seg)
                out.append((a, b))
            t += step
            carry += step
            if carry >= (on if drawing else off) - 1e-6:
                carry, drawing = 0.0, not drawing
    return out


def star(cx, cy, r, points=5, inner=0.45):
    pts = []
    for i in range(points * 2):
        a = -math.pi / 2 + i * math.pi / points
        rr = r if i % 2 == 0 else r * inner
        pts.append((cx + rr * math.cos(a), cy + rr * math.sin(a)))
    return pts


# ---------------------------------------------------------------------------------------------------
# Pictogram chips (our own vector art; the operator's sketches are the brief, not the source)
# ---------------------------------------------------------------------------------------------------
def chip_kirby(c, cx, cy, s, simple=None):
    """Kirby in a star nightcap, two hat silhouettes floating above, a dashed arc between them.
    Below 60 px (a menu row) the floating hats are dropped and Kirby fills the chip."""
    simple = s < 60 if simple is None else simple
    w = max(1, round(s / 28))
    # body
    bx, by, br = (cx, cy + s * 0.14, s * 0.26) if not simple else (cx - s * 0.06, cy + s * 0.08, s * 0.34)
    c.ellipse(bx - br * 0.95, by + br * 0.55, bx - br * 0.15, by + br * 1.05, fill=INK, outline=FG, width=w)  # feet
    c.ellipse(bx + br * 0.15, by + br * 0.55, bx + br * 0.95, by + br * 1.05, fill=INK, outline=FG, width=w)
    c.circle(bx, by, br, fill=INK, outline=FG, width=w)
    c.circle(bx - br * 1.0, by, br * 0.32, fill=INK, outline=FG, width=w)  # arms
    c.circle(bx + br * 1.0, by, br * 0.32, fill=INK, outline=FG, width=w)
    # face
    c.ellipse(bx - br * 0.36, by - br * 0.42, bx - br * 0.16, by + br * 0.1, fill=FG)
    c.ellipse(bx + br * 0.16, by - br * 0.42, bx + br * 0.36, by + br * 0.1, fill=FG)
    c.arc(bx, by + br * 0.18, br * 0.26, br * 0.2, 20, 160, FG, w)
    # nightcap: band + cone drooping right + pompom
    c.polygon([(bx - br * 0.98, by - br * 0.52), (bx + br * 0.98, by - br * 0.52), (bx + br * 1.55, by + br * 0.05),
               (bx + br * 1.1, by - br * 0.9), (bx + br * 0.3, by - br * 1.3), (bx - br * 0.6, by - br * 1.1)],
              fill=INK, outline=FG, width=w)
    c.circle(bx + br * 1.6, by + br * 0.1, br * 0.22, fill=FG)
    c.polygon(star(bx - br * 0.2, by - br * 0.92, br * 0.17), fill=FG)
    if simple:
        return
    c.polygon(star(bx + br * 0.45, by - br * 0.78, br * 0.14), fill=FG)
    # floating hats: a winged star cap (left) and a flaming sailor cap (right)
    hy = cy - s * 0.3
    lx, rx = cx - s * 0.27, cx + s * 0.27
    hr = s * 0.11
    # winged star cap: a filled dome on a brim, a star, two wings
    dome = [(lx + hr * 1.1 * math.cos(math.radians(a)), hy + hr * 0.9 * math.sin(math.radians(a))) for a in range(180, 361, 15)]
    c.polygon(dome, fill=INK, outline=FG, width=w)
    c.rect(lx - hr * 1.3, hy - hr * 0.08, lx + hr * 1.3, hy + hr * 0.25, fill=INK, outline=FG, width=w, radius=hr * 0.1)
    c.polygon([(lx - hr * 1.25, hy - hr * 0.35), (lx - hr * 2.2, hy - hr * 1.0), (lx - hr * 1.35, hy + hr * 0.05)], fill=FG)
    c.polygon([(lx + hr * 1.25, hy - hr * 0.35), (lx + hr * 2.2, hy - hr * 1.0), (lx + hr * 1.35, hy + hr * 0.05)], fill=FG)
    c.polygon(star(lx, hy - hr * 0.4, hr * 0.36), fill=FG)
    # flaming sailor cap: a flat crown with a brim, a flame on top
    c.rect(rx - hr * 1.0, hy - hr * 0.55, rx + hr * 1.0, hy + hr * 0.05, fill=INK, outline=FG, width=w, radius=hr * 0.15)
    c.rect(rx - hr * 1.3, hy - hr * 0.05, rx + hr * 1.3, hy + hr * 0.25, fill=INK, outline=FG, width=w, radius=hr * 0.1)
    c.circle(rx, hy - hr * 0.25, hr * 0.18, fill=FG)
    c.polygon([(rx - hr * 0.7, hy - hr * 0.6), (rx - hr * 0.45, hy - hr * 1.5), (rx - hr * 0.1, hy - hr * 1.0),
               (rx + hr * 0.1, hy - hr * 2.0), (rx + hr * 0.45, hy - hr * 1.15), (rx + hr * 0.7, hy - hr * 0.6)],
              fill=INK, outline=FG, width=w)
    # dashed arc from one hat to the other: the copy ability changes
    c.arc(cx, hy + hr * 0.6, s * 0.13, s * 0.09, 205, 335, FG, w + 1, dash=(max(3, s / 26), max(3, s / 30)))


def chip_popo(c, cx, cy, s, simple=None):
    """Popo in his parka raising the hammer, a dotted looping path trailing to the right.
    Below 60 px the loop becomes three dots and the motion marks are dropped."""
    simple = s < 60 if simple is None else simple
    w = max(1, round(s / 28))
    px, py = (cx - s * 0.12, cy + s * 0.05) if not simple else (cx - s * 0.04, cy + s * 0.1)
    hr = s * 0.2 if not simple else s * 0.24
    # body (parka) and feet
    c.ellipse(px - hr * 0.9, py + hr * 1.9, px - hr * 0.1, py + hr * 2.3, fill=INK, outline=FG, width=w)
    c.ellipse(px + hr * 0.1, py + hr * 1.9, px + hr * 0.9, py + hr * 2.3, fill=INK, outline=FG, width=w)
    c.rect(px - hr * 0.95, py + hr * 0.6, px + hr * 0.95, py + hr * 2.05, fill=INK, outline=FG, width=w, radius=hr * 0.4)
    c.line([(px - hr * 0.95, py + hr * 1.85), (px + hr * 0.95, py + hr * 1.85)], FG, w)  # fur hem
    # hood + face
    c.circle(px, py, hr, fill=INK, outline=FG, width=w)
    c.circle(px, py + hr * 0.08, hr * 0.68, fill=INK, outline=FG, width=w)
    c.ellipse(px - hr * 0.3, py - hr * 0.22, px - hr * 0.14, py + hr * 0.14, fill=FG)
    c.ellipse(px + hr * 0.14, py - hr * 0.22, px + hr * 0.3, py + hr * 0.14, fill=FG)
    c.arc(px, py + hr * 0.26, hr * 0.2, hr * 0.14, 20, 160, FG, w)
    # raised arm and hammer
    ax, ay = px - hr * 1.5, py - hr * 1.1
    c.line([(px - hr * 0.7, py + hr * 0.9), (ax, ay)], FG, w * 2)
    c.circle(ax, ay, hr * 0.25, fill=INK, outline=FG, width=w)  # mitten
    c.line([(ax, ay), (ax + hr * 0.9, ay - hr * 1.6)], FG, w * 2)  # handle
    hx, hy = ax + hr * 0.9, ay - hr * 1.6
    c.polygon([(hx - hr * 0.95, hy - hr * 0.1), (hx + hr * 0.15, hy - hr * 0.85), (hx + hr * 0.55, hy - hr * 0.25),
               (hx - hr * 0.55, hy + hr * 0.5)], fill=INK, outline=FG, width=w)
    if simple:
        for i in range(3):  # a short dotted trail: Popo walks alone
            c.circle(cx + s * 0.3, cy - s * 0.1 + i * s * 0.17, max(1.5, s * 0.035), fill=FG)
        return
    for a in (-100, -70, -40):  # motion marks
        c.line([(hx + hr * 0.9 + hr * 0.3 * math.cos(math.radians(a)), hy - hr * 0.2 + hr * 0.3 * math.sin(math.radians(a))),
                (hx + hr * 0.9 + hr * 0.8 * math.cos(math.radians(a)), hy - hr * 0.2 + hr * 0.8 * math.sin(math.radians(a)))], FG, w)
    # dotted looping path (Solo Popo walks alone)
    ox, oy = cx + s * 0.24, cy + s * 0.08
    pts = [(ox + s * 0.17 * math.sin(t) * 0.9, oy - s * 0.26 + s * 0.42 * (t / (4 * math.pi)) + s * 0.08 * math.cos(2 * t))
           for t in [i * 4 * math.pi / 90 for i in range(91)]]
    c.line(pts, FG, w, dash=(max(2, s / 36), max(2, s / 36)))


def chip_items(c, cx, cy, s, simple=None):
    """A rounded badge: a hand drops a lit bomb over a banana peel, a turnip and a spinning top.
    Below 60 px only the hand and the bomb remain."""
    simple = s < 60 if simple is None else simple
    w = max(1, round(s / 28))
    half = s * 0.46
    c.rect(cx - half, cy - half, cx + half, cy + half, fill=INK, outline=FG, width=w, radius=s * 0.12)
    gy = cy + s * 0.3
    if simple:
        hx, hy = cx + s * 0.1, cy - s * 0.18
        c.ellipse(hx - s * 0.2, hy - s * 0.12, hx + s * 0.24, hy + s * 0.14, fill=INK, outline=FG, width=w)
        c.line([(hx + s * 0.24, hy), (cx + half, hy - s * 0.06)], FG, w)
        bx, by, br = cx - s * 0.06, cy + s * 0.14, s * 0.19
        c.circle(bx, by, br, fill=FG)
        c.arc(bx - br * 0.5, by - br * 1.2, br * 0.5, br * 0.4, 200, 330, FG, w)
        c.polygon(star(bx - br * 1.2, by - br * 1.5, br * 0.4, 6, 0.5), fill=INK, outline=FG, width=w)
        return
    c.line([(cx - s * 0.36, gy), (cx + s * 0.36, gy)], FG, w)  # ground
    # hand (palm + fingers) at the top right
    hx, hy = cx + s * 0.14, cy - s * 0.2
    c.ellipse(hx - s * 0.17, hy - s * 0.1, hx + s * 0.19, hy + s * 0.12, fill=INK, outline=FG, width=w)
    for i, (dx, dy) in enumerate([(-0.17, 0.02), (-0.1, 0.08), (0.0, 0.1)]):
        c.ellipse(hx + s * dx - s * 0.045, hy + s * dy, hx + s * dx + s * 0.045, hy + s * dy + s * 0.13, fill=INK, outline=FG, width=w)
    c.line([(hx + s * 0.19, hy - s * 0.04), (cx + half, hy - s * 0.12)], FG, w)  # wrist to the badge edge
    c.line([(hx + s * 0.19, hy + s * 0.1), (cx + half, hy + s * 0.06)], FG, w)
    # bomb with a fuse and a spark
    bx, by, br = cx - s * 0.02, cy + s * 0.07, s * 0.12
    c.circle(bx, by, br, fill=FG)
    c.ellipse(bx - br * 0.6, by - br * 0.55, bx - br * 0.2, by - br * 0.1, fill=INK)  # highlight
    c.arc(bx - br * 0.6, by - br * 1.2, br * 0.5, br * 0.4, 200, 330, FG, w)
    c.polygon(star(bx - br * 1.25, by - br * 1.55, br * 0.35, 6, 0.5), fill=INK, outline=FG, width=w)
    # banana peel, turnip, spinning top on the ground
    bax = cx - s * 0.27
    c.polygon([(bax - s * 0.08, gy), (bax - s * 0.1, gy - s * 0.1), (bax - s * 0.02, gy - s * 0.16), (bax + s * 0.02, gy - s * 0.1),
               (bax + s * 0.08, gy - s * 0.02), (bax + s * 0.1, gy)], fill=INK, outline=FG, width=w)
    tx = cx + s * 0.02
    c.circle(tx, gy - s * 0.07, s * 0.07, fill=INK, outline=FG, width=w)
    c.polygon([(tx - s * 0.02, gy - s * 0.13), (tx - s * 0.07, gy - s * 0.22), (tx, gy - s * 0.15)], fill=FG)
    c.polygon([(tx + s * 0.02, gy - s * 0.13), (tx + s * 0.07, gy - s * 0.22), (tx, gy - s * 0.15)], fill=FG)
    c.circle(tx - s * 0.025, gy - s * 0.08, s * 0.012, fill=FG)
    c.circle(tx + s * 0.025, gy - s * 0.08, s * 0.012, fill=FG)
    sx = cx + s * 0.27
    c.polygon([(sx - s * 0.08, gy - s * 0.12), (sx + s * 0.08, gy - s * 0.12), (sx, gy)], fill=INK, outline=FG, width=w)
    c.rect(sx - s * 0.02, gy - s * 0.2, sx + s * 0.02, gy - s * 0.12, fill=FG)


DIRS = {  # name: (dx, dy) in screen space; In is toward the opponent (drawn left), Out away (right)
    "Up In": (-1, -1), "Up": (0, -1), "Up Out": (1, -1),
    "In": (-1, 0), "Neutral": (0, 0), "Out": (1, 0),
    "Down In": (-1, 1), "Down": (0, 1), "Down Out": (1, 1),
    "Left": (-1, 0), "Right": (1, 0),
}


def chip_direction(c, cx, cy, s, name, selected=False, weight=0, absolute=False, cursor=False):
    """One direction chip: a bold stroke from the centre with an arrowhead; Neutral is a ring and a dot."""
    half = s / 2
    c.rect(cx - half, cy - half, cx + half, cy + half, fill=TILE_HI if selected else TILE,
           outline=SEL if cursor else (FG if selected else LINE), width=3 if cursor else 2, radius=s * 0.14)
    dx, dy = DIRS[name]
    col = FG if selected else MUTED
    if (dx, dy) == (0, 0):
        c.circle(cx, cy, s * 0.2, outline=col, width=max(2, round(s / 22)))
        c.circle(cx, cy, s * 0.07, fill=col)
    else:
        n = math.hypot(dx, dy)
        ux, uy = dx / n, dy / n
        tip = (cx + ux * s * 0.34, cy + uy * s * 0.34)
        c.line([(cx, cy), (cx + ux * s * 0.22, cy + uy * s * 0.22)], col, max(3, round(s / 11)))
        c.circle(cx, cy, s * 0.06, fill=col)
        ang = math.atan2(uy, ux)
        base = (cx + ux * s * 0.19, cy + uy * s * 0.19)
        px, py = -uy, ux
        c.polygon([tip, (base[0] + px * s * 0.11, base[1] + py * s * 0.11), (base[0] - px * s * 0.11, base[1] - py * s * 0.11)], fill=col)
    if absolute:
        c.text(cx, cy + half - s * 0.27, "abs", "tiny" if s < 70 else "xxs", MUTED, "m")
    if selected and weight:
        c.circle(cx + half - s * 0.14, cy - half + s * 0.14, s * 0.11, fill=SEL)
        c.text(cx + half - s * 0.14, cy - half + s * 0.14 - 7, str(weight), "b", FG, "m")


# ---------------------------------------------------------------------------------------------------
# Grid tiles (stand-ins for the fork's generated portraits / thumbnails / item icons)
# ---------------------------------------------------------------------------------------------------
def hue(name):
    h = sum(ord(ch) * (i + 7) for i, ch in enumerate(name)) % 360
    r, g, b = [int(120 + 70 * math.cos(math.radians(h + o))) for o in (0, 120, 240)]
    return (r, g, b)


def tile_portrait(c, x, y, w, h, name, rank_p=None, rank_c=None, solo=None, cursor=False, kind="fighter"):
    c.rect(x, y, x + w, y + h, fill=TILE_HI if cursor else TILE, outline=SEL if cursor else LINE, width=3 if cursor else 1, radius=6)
    if kind == "all":
        for i in range(3):
            for j in range(3):
                c.rect(x + w / 2 - 15 + j * 11, y + 8 + i * 11, x + w / 2 - 7 + j * 11, y + 16 + i * 11, fill=MUTED)
        c.text(x + w / 2, y + h - 15, "ALL", "xxs", FG, "m")
        return
    col = hue(name) if kind != "unsupported" else (70, 74, 84)
    cx, top = x + w / 2, y + 5
    # bust silhouette: head and shoulders, the stand-in for the stock portrait
    c.circle(cx, top + 12, 9, fill=col)
    c.polygon([(cx - 19, top + 36), (cx - 12, top + 23), (cx + 12, top + 23), (cx + 19, top + 36)], fill=col)
    label = name if len(name) <= 11 else name[:10] + "…"
    c.text(cx, y + h - 15, label, "tiny", FG if kind != "unsupported" else MUTED, "m")
    if rank_p is not None:
        c.circle(x + 9, y + 9, 8, fill=P1)
        c.text(x + 9, y + 2, str(rank_p), "tiny", FG, "m")
    if rank_c is not None:
        c.circle(x + w - 9, y + 9, 8, fill=CPU)
        c.text(x + w - 9, y + 2, str(rank_c), "tiny", FG, "m")
    if solo:
        c.text(x + 5, y + h - 27, "S", "b", P1 if solo == "p" else CPU, "l")


def tile_stage(c, x, y, w, h, name, rank=None, form=None, cursor=False, all_tile=False):
    c.rect(x, y, x + w, y + h, fill=TILE, outline=SEL if cursor else LINE, width=3 if cursor else 1, radius=6)
    th = h - 24
    if all_tile:
        for i in range(2):
            for j in range(4):
                c.rect(x + w / 2 - 44 + j * 23, y + 10 + i * 23, x + w / 2 - 26 + j * 23, y + 28 + i * 23, fill=MUTED)
        c.text(x + w / 2, y + h - 19, "All (listed order)", "xxs", FG, "m")
        return
    # thumbnail stand-in: sky band + main stage + platforms derived from the name
    col = hue(name)
    c.rect(x + 6, y + 6, x + w - 6, y + th, fill=(col[0] // 4 + 10, col[1] // 4 + 12, col[2] // 4 + 18), radius=4)
    seed = sum(map(ord, name))
    mw = w * (0.45 + (seed % 5) * 0.06)
    c.polygon([(x + w / 2 - mw / 2, y + th - 14), (x + w / 2 + mw / 2, y + th - 14), (x + w / 2 + mw / 2 - 12, y + th - 4),
               (x + w / 2 - mw / 2 + 12, y + th - 4)], fill=col)
    for k in range((seed % 3)):
        px = x + w / 2 - mw / 3 + k * mw / 3
        c.rect(px - 20, y + th - 30 - k * 6, px + 20, y + th - 27 - k * 6, fill=col)
    label = name if len(name) <= 24 else name[:23] + "…"
    c.text(x + w / 2, y + h - 19, label, "xxs", FG, "m")
    if rank is not None:
        c.circle(x + 14, y + 14, 10, fill=P1)
        c.text(x + 14, y + 7, str(rank), "xxs", FG, "m")
    if form:
        c.rect(x + w - 40, y + 6, x + w - 6, y + 24, fill=HILITE, radius=4)
        c.text(x + w - 23, y + 8, form, "xxs", OURS, "m")


ITEM_GLYPHS = {
    "bomb": "bomb", "grenade": "grenade", "egg": "egg", "mechakoopa": "koopa", "turnip": "turnip", "saturn": "saturn",
    "banana": "banana", "peanut": "peanut", "crown": "crown", "arrow": "arrow", "blade": "blade", "cherry": "fruit",
    "strawberry": "fruit", "orange": "fruit", "apple": "fruit", "melon": "fruit", "galaxian": "ship", "bell": "bell",
    "key": "key", "water": "flask", "gyro": "gyro", "tome": "book", "sword": "sword", "chip": "chip",
}


def item_glyph(c, cx, cy, s, name):
    kind = next((g for k, g in ITEM_GLYPHS.items() if k in name.lower()), "box")
    w = 2
    if kind == "bomb":
        c.circle(cx, cy + s * 0.08, s * 0.3, fill=FG)
        c.arc(cx + s * 0.1, cy - s * 0.3, s * 0.18, s * 0.14, 180, 330, FG, w)
    elif kind == "grenade":
        c.ellipse(cx - s * 0.24, cy - s * 0.18, cx + s * 0.24, cy + s * 0.36, fill=INK, outline=FG, width=w)
        c.rect(cx - s * 0.1, cy - s * 0.34, cx + s * 0.1, cy - s * 0.16, fill=FG)
    elif kind == "egg":
        c.ellipse(cx - s * 0.22, cy - s * 0.32, cx + s * 0.22, cy + s * 0.32, fill=INK, outline=FG, width=w)
        c.circle(cx - s * 0.06, cy - s * 0.05, s * 0.05, fill=FG)
        c.circle(cx + s * 0.08, cy + s * 0.12, s * 0.05, fill=FG)
    elif kind == "koopa":
        c.ellipse(cx - s * 0.3, cy - s * 0.1, cx + s * 0.3, cy + s * 0.3, fill=INK, outline=FG, width=w)
        c.rect(cx - s * 0.06, cy - s * 0.32, cx + s * 0.06, cy - s * 0.08, fill=FG)
    elif kind == "turnip":
        c.circle(cx, cy + s * 0.1, s * 0.24, fill=INK, outline=FG, width=w)
        c.polygon([(cx - s * 0.04, cy - s * 0.1), (cx - s * 0.22, cy - s * 0.36), (cx, cy - s * 0.14)], fill=FG)
        c.polygon([(cx + s * 0.04, cy - s * 0.1), (cx + s * 0.22, cy - s * 0.36), (cx, cy - s * 0.14)], fill=FG)
        c.circle(cx - s * 0.08, cy + s * 0.06, s * 0.035, fill=FG)
        c.circle(cx + s * 0.08, cy + s * 0.06, s * 0.035, fill=FG)
    elif kind == "saturn":
        c.circle(cx, cy + s * 0.05, s * 0.26, fill=INK, outline=FG, width=w)
        c.line([(cx, cy - s * 0.2), (cx, cy - s * 0.36)], FG, w)
        c.circle(cx, cy - s * 0.38, s * 0.04, fill=FG)
    elif kind == "banana":
        c.polygon([(cx - s * 0.32, cy + s * 0.1), (cx - s * 0.2, cy - s * 0.2), (cx + s * 0.05, cy - s * 0.3), (cx + s * 0.3, cy - s * 0.1),
                   (cx + s * 0.1, cy - s * 0.02), (cx - s * 0.12, cy + s * 0.14)], fill=INK, outline=FG, width=w)
    elif kind == "peanut":
        c.ellipse(cx - s * 0.16, cy - s * 0.3, cx + s * 0.16, cy + s * 0.02, fill=INK, outline=FG, width=w)
        c.ellipse(cx - s * 0.18, cy - s * 0.04, cx + s * 0.18, cy + s * 0.32, fill=INK, outline=FG, width=w)
    elif kind == "crown":
        c.polygon([(cx - s * 0.3, cy + s * 0.22), (cx - s * 0.3, cy - s * 0.2), (cx - s * 0.12, cy), (cx, cy - s * 0.3),
                   (cx + s * 0.12, cy), (cx + s * 0.3, cy - s * 0.2), (cx + s * 0.3, cy + s * 0.22)], fill=INK, outline=FG, width=w)
    elif kind == "arrow":
        c.line([(cx - s * 0.3, cy + s * 0.3), (cx + s * 0.22, cy - s * 0.22)], FG, w)
        c.polygon([(cx + s * 0.3, cy - s * 0.3), (cx + s * 0.08, cy - s * 0.24), (cx + s * 0.24, cy - s * 0.08)], fill=FG)
    elif kind == "blade":
        c.polygon(star(cx, cy, s * 0.32, 4, 0.5), fill=INK, outline=FG, width=w)
        c.circle(cx, cy, s * 0.08, fill=FG)
    elif kind == "fruit":
        c.circle(cx, cy + s * 0.06, s * 0.26, fill=INK, outline=FG, width=w)
        c.line([(cx, cy - s * 0.2), (cx + s * 0.1, cy - s * 0.36)], FG, w)
    elif kind == "ship":
        c.polygon([(cx, cy - s * 0.3), (cx + s * 0.3, cy + s * 0.2), (cx, cy + s * 0.05), (cx - s * 0.3, cy + s * 0.2)], fill=INK, outline=FG, width=w)
    elif kind == "bell":
        c.polygon([(cx - s * 0.26, cy + s * 0.18), (cx - s * 0.16, cy - s * 0.1), (cx, cy - s * 0.3), (cx + s * 0.16, cy - s * 0.1),
                   (cx + s * 0.26, cy + s * 0.18)], fill=INK, outline=FG, width=w)
        c.circle(cx, cy + s * 0.26, s * 0.06, fill=FG)
    elif kind == "key":
        c.circle(cx - s * 0.12, cy - s * 0.1, s * 0.16, fill=INK, outline=FG, width=w)
        c.line([(cx, cy + s * 0.02), (cx + s * 0.3, cy + s * 0.3)], FG, w + 1)
        c.line([(cx + s * 0.18, cy + s * 0.18), (cx + s * 0.08, cy + s * 0.28)], FG, w)
    elif kind == "flask":
        c.polygon([(cx - s * 0.1, cy - s * 0.32), (cx + s * 0.1, cy - s * 0.32), (cx + s * 0.1, cy - s * 0.1), (cx + s * 0.26, cy + s * 0.3),
                   (cx - s * 0.26, cy + s * 0.3), (cx - s * 0.1, cy - s * 0.1)], fill=INK, outline=FG, width=w)
    elif kind == "gyro":
        c.polygon([(cx - s * 0.3, cy - s * 0.1), (cx + s * 0.3, cy - s * 0.1), (cx, cy + s * 0.3)], fill=INK, outline=FG, width=w)
        c.rect(cx - s * 0.04, cy - s * 0.34, cx + s * 0.04, cy - s * 0.1, fill=FG)
    elif kind == "book":
        c.rect(cx - s * 0.24, cy - s * 0.3, cx + s * 0.24, cy + s * 0.3, fill=INK, outline=FG, width=w, radius=2)
        c.line([(cx - s * 0.1, cy - s * 0.3), (cx - s * 0.1, cy + s * 0.3)], FG, w)
    elif kind == "sword":
        c.polygon([(cx - s * 0.26, cy + s * 0.26), (cx + s * 0.2, cy - s * 0.28), (cx + s * 0.28, cy - s * 0.2), (cx - s * 0.18, cy + s * 0.34)], fill=FG)
        c.line([(cx - s * 0.3, cy + s * 0.12), (cx - s * 0.12, cy + s * 0.3)], FG, w + 1)
    elif kind == "chip":
        c.polygon([(cx - s * 0.3, cy + s * 0.1), (cx - s * 0.05, cy - s * 0.3), (cx + s * 0.3, cy - s * 0.12), (cx + s * 0.1, cy + s * 0.3)], fill=INK, outline=FG, width=w)
    else:
        c.rect(cx - s * 0.26, cy - s * 0.26, cx + s * 0.26, cy + s * 0.26, fill=INK, outline=FG, width=w)


def tile_item(c, x, y, w, h, name, cursor=False):
    c.rect(x, y, x + w, y + h, fill=TILE_HI if cursor else TILE, outline=SEL if cursor else LINE, width=3 if cursor else 1, radius=6)
    item_glyph(c, x + 26, y + h / 2, 34, name)
    who, what = name.split(": ", 1)
    c.text(x + 50, y + 7, what if len(what) <= 24 else what[:23] + "…", "xxs", FG)
    c.text(x + 50, y + 25, who, "tiny", MUTED)


# ---------------------------------------------------------------------------------------------------
# Data (MENU-DRAFT.md §2 and the fork's own tables)
# ---------------------------------------------------------------------------------------------------
# kind: toggle | drop | multi | slider | number | editor | list | info | dirgrid | dirsingle | portraits | cycles
#       | hats | items | stages | popo ; value = text shown in the control
PAGES = [
    ("CPU", [
        ("Reactions", False, [
            ("Mash", "multi", "0 of 13 selected", "up"),
            ("Followup", "multi", "0 of 13 selected", "up"),
            ("Mash Triggers", "multi", "6 of 17 selected", "up"),
            ("Situation overrides", "editor", "0 of 13 set", "up"),
        ]),
        ("Defence", False, [
            ("Airdodge Direction", "dirgrid", "0 selected", "up"),
            ("DI Direction", "dirgrid", "3 selected", "up"),
            ("SDI Direction", "dirgrid", "0 selected", "up"),
            ("SDI Strength", "drop", "None", "up"),
            ("Clatter Strength", "drop", "None", "up"),
            ("Dodge Staling", "toggle", "ON", "up"),
        ]),
        ("Ledge & Tech", False, [
            ("Ledge Options", "multi", "4 of 10 selected", "up"),
            ("Ledge Delay", "multi", "0 of 31 selected  (0-300 by 10)", "up"),
            ("Tech Options", "multi", "4 of 4 selected", "up"),
            ("Mistech Options", "multi", "4 of 4 selected", "up"),
            ("Hide Tech Animations", "toggle", "OFF", "up"),
        ]),
        ("Shield", False, [
            ("Shield", "drop", "None", "up"),
            ("Shield Tilt", "dirsingle", "Neutral", "up"),
            ("OoS Offset", "multi", "0 of 31 selected  (0-30 hits)", "up"),
            ("Reaction Time", "multi", "0 of 31 selected  (0-30 frames)", "up"),
        ]),
        ("Attacks & Movement", False, [
            ("Attack Angle", "multi", "0 of 3 selected", "up"),
            ("Throw", "multi", "0 of 5 selected", "up"),
            ("Throw Delay", "multi", "0 of 31 selected  (0-150 by 5)", "up"),
            ("Pummel Delay", "multi", "0 of 31 selected  (0-150 by 5)", "up"),
            ("Falling Aerials", "drop", "No", "up"),
            ("Aerial Delay", "multi", "0 of 31 selected  (0-30)", "up"),
            ("Full Hop", "drop", "Yes", "up"),
            ("Fast Fall", "drop", "No", "up"),
            ("Fast Fall Delay", "multi", "0 of 31 selected  (0-30)", "up"),
            ("Crouch", "toggle", "OFF", "up"),
        ]),
    ]),
    ("Saves", [
        ("Save states", False, [
            ("Save States", "toggle", "ON", "up"),
            ("Slot", "drop", "1", "up"),
            ("Randomize Slots", "multi", "0 of 5 selected", "up"),
            ("Mirroring", "drop", "None", "up"),
            ("Auto-load on death", "toggle", "OFF", "up"),
            ("Player damage on load", "drop", "Default", "up"),
            ("Player damage range", "slider", "0 - 150 %", "up"),
            ("CPU damage on load", "drop", "Default", "up"),
            ("CPU damage range", "slider", "0 - 150 %", "up"),
            ("Buffs on load", "multi", "0 of 12 selected", "up"),
            ("Item on load (older saves)", "drop", "None", "up"),
        ]),
        ("Setup library", False, [
            ("Save setup to library...", "editor", "name entry", "ours"),
            ("Setup library", "list", "saved setups", "ours"),
        ]),
    ]),
    ("Recording", [
        ("Recording", False, [
            ("Record Slot", "drop", "1", "up"),
            ("Trigger", "drop", "Button Combination", "up"),
            ("Duration", "drop", "150 frames", "up"),
            ("Crop trailing neutral", "toggle", "ON", "up"),
        ]),
        ("Playback", False, [
            ("Button slots", "multi", "1 of 5 selected", "up"),
            ("Hitstun timing", "drop", "As Hitstun Ends", "up"),
            ("Save-state slots", "multi", "0 of 5 selected", "up"),
            ("Mash interrupts playback", "toggle", "ON", "up"),
            ("Loop", "toggle", "OFF", "up"),
        ]),
        ("Named macros", False, [
            ("Source", "drop", "Numbered Slots", "ours"),
            ("Ask before switching fighter", "toggle", "ON", "ours"),
            ("Named library", "list", "library", "ours"),
            ("Save slot to library...", "editor", "name entry", "ours"),
            ("Edit placement", "editor", "placement editor", "ours"),
            ("Replay window", "editor", "captures", "ours"),
            ("Replay capture", "toggle", "ON", "ours"),
        ]),
    ]),
    ("Fighters & Stages", [
        ("Player 1 and CPU", False, [
            ("Player Swapping", "toggle", "OFF", "ours"),
            ("Player Character", "portraits", "Mr. Game & Watch", "ours"),
            ("CPU Character", "portraits", "Pikachu", "ours"),
            ("Character Cycles", "cycles", "P1 3 / CPU 4", "ours"),
            ("Solo Popo (original Ice Climbers)", "popo", "OFF", "ours"),
        ]),
        ("Extras", False, [
            ("Kirby Hats", "hats", "P1 6 / CPU 6 hats", "ours"),
            ("Give item", "items", "none", "ours"),
            ("Mii Brawler", "list", "role's saved pick", "ours"),
            ("Mii Swordfighter", "list", "role's saved pick", "ours"),
            ("Mii Gunner", "list", "role's saved pick", "ours"),
        ]),
        ("Stages", False, [
            ("Lobby Practice", "toggle", "OFF", "ours"),
            ("Lobby Stages", "stages", "12 ordered + All", "ours"),
            ("Training Stage Rotation", "toggle", "OFF", "ours"),
            ("Stage Hazards", "toggle", "OFF", "up"),
        ]),
        ("Loading", True, [
            ("Cycle Preload", "drop", "5 (built-in max)", "ours"),
            ("Preload", "drop", "Both", "ours"),
            ("Ready Fighters", "number", "8", "ours"),
            ("Ready Fighter Bodies", "drop", "3", "ours"),
            ("Group Swap", "toggle", "ON", "ours"),
            ("Swap groups", "multi", "7 of 7 selected", "ours"),
            ("Preload Samples", "toggle", "OFF", "ours"),
        ]),
    ]),
    ("Display & Controls", [
        ("On screen", False, [
            ("HUD", "toggle", "ON", "up"),
            ("Hitbox Visualization", "toggle", "OFF", "up"),
            ("Frame Advantage", "toggle", "OFF", "up"),
            ("Input Display", "drop", "Smash Inputs", "up"),
            ("Group inputs by status", "toggle", "OFF", "up"),
            ("Input Delay", "number", "0 frames", "up"),
        ]),
        ("Controls", False, [
            ("Button chords", "editor", "6 chords", "mixed"),
            ("L+R+A Reset", "toggle", "ON", "up"),
            ("Menu shortcut 1", "info", "Minus  (button_config.ini)", "ours"),
            ("Menu shortcut 2", "info", "disabled  (button_config.ini)", "ours"),
            ("Permanent shortcut", "info", "B + D-pad Up", "ours"),
        ]),
    ]),
]

HELP = {
    "Mash": "Actions the CPU mashes out of hitstun, shieldstun, tumble and landing. Random among the selected.",
    "Airdodge Direction": "Direction grid: pick the chips the CPU may air-dodge toward (random by weight). In/Out follow the opponent.",
    "DI Direction": "Direction grid: chips the CPU may DI toward, random by weight. In = toward the opponent, Out = away.",
    "Situation overrides": "Per-situation action sets (ledge getups, tech, clatter, hitstun...). Opens a sub-screen.",
    "Save States": "Save and load positions and damage with the chord set under Display & Controls.",
    "Record Slot": "Slot the next recording is written to.",
    "Player Swapping": "Swap Player 1's fighter in Training without reloading (cycle chord: ZL + D-pad Right).",
    "Kirby Hats": "Hat order per role on the portrait grid (ZR switches role). Kirby takes the next hat on each hat cycle.",
    "Lobby Practice": "Practise in the waiting room; the stage rows below decide where.",
    "HUD": "Show the Training HUD overlay (damage, state, frame data).",
}

SUBSCREENS = [
    ("Situation overrides", "CPU > Reactions", [
        ("Situation", "drop", "Ledge roll", None),
        ("Use Mash set", "toggle", "OFF", None),
        ("Airdodge", "check", "x", None), ("Jump", "check", "x", None), ("Shield", "check", " ", None),
        ("Spotdodge", "check", " ", None), ("Roll In", "check", " ", None), ("Roll Out", "check", " ", None),
        ("Aerial", "check", "x", None), ("Jab", "check", " ", None), ("Tilts", "check", " ", None),
        ("Smash Attacks", "check", " ", None), ("Grab", "check", " ", None), ("Dash", "check", " ", None),
        ("Dash Attack", "check", " ", None),
    ], "Pick the situation with the dropdown, then tick the actions. The page row shows how many situations are set."),
    ("Button chords", "Display & Controls > Controls", [
        ("Save State Save", "chord", "ZL + D-pad Down", None),
        ("Save State Load", "chord", "ZL + D-pad Up", None),
        ("Input Record", "chord", "ZR + D-pad Down", None),
        ("Input Playback", "chord", "ZR + D-pad Up", None),
        ("Cycle Player Character", "chord", "ZL + D-pad Right", None),
        ("Cycle CPU Character", "chord", "ZL + D-pad Left", None),
    ], "Hold one button and press the others to set a chord (16 buttons). A chord of one button is allowed."),
    ("Swap groups", "Fighters & Stages > Loading (advanced)", [
        ("Ice Climbers pair bank", "check", "x", None),
        ("Original fighter bank", "check", "x", None),
        ("Solo Popo groups", "check", "x", None),
        ("Ice Climbers pair groups", "check", "x", None),
        ("PT/Aegis groups", "check", "x", None),
        ("Ice Climbers pair group", "check", "x", None),
        ("Mii groups", "check", "x", None),
    ], "Which special groups may be prefetched and swapped warm. An entry greys out, with the reason, when a dependency is off."),
]

# The fork's physical roster grid (training_mod_tui/src/roster_grid.rs GRID): six rows of 13, then a centred row of 9.
ROSTER = [
    ["Mario", "Donkey Kong", "Link", "Samus", "Dark Samus", "Yoshi", "Kirby", "Fox", "Pikachu", "Luigi", "Ness", "Captain Falcon", "Jigglypuff"],
    ["Peach", "Daisy", "Bowser", "Ice Climbers", "Sheik", "Zelda", "Dr. Mario", "Pichu", "Falco", "Marth", "Lucina", "Young Link", "Ganondorf"],
    ["Mewtwo", "Roy", "Chrom", "Mr. Game & Watch", "Meta Knight", "Pit", "Dark Pit", "Zero Suit Samus", "Wario", "Snake", "Ike", "Pokémon Trainer", "Diddy Kong"],
    ["Lucas", "Sonic", "King Dedede", "Olimar", "Lucario", "R.O.B.", "Toon Link", "Wolf", "Villager", "Mega Man", "Wii Fit Trainer", "Rosalina & Luma", "Little Mac"],
    ["Greninja", "Palutena", "PAC-MAN", "Robin", "Shulk", "Bowser Jr.", "Duck Hunt", "Ryu", "Ken", "Cloud", "Corrin", "Bayonetta", "Inkling"],
    ["Ridley", "Simon", "Richter", "King K. Rool", "Isabelle", "Incineroar", "Piranha Plant", "Joker", "Hero", "Banjo & Kazooie", "Terry", "Byleth", "Min Min"],
    ["Steve", "Sephiroth", "Pyra / Mythra", "Kazuya", "Sora", "Mii Brawler", "Mii Swordfighter", "Mii Gunner", "ALL"],
]
P1_CYCLE = ["Mr. Game & Watch", "Palutena", "Pikachu"]                 # the fork's default P1 order
CPU_CYCLE = ["Pikachu", "Kirby", "Bowser", "Bayonetta"]                 # the fork's default CPU order
P1_HATS = ["Mario", "Kirby", "Fox", "Pikachu", "Link", "Samus"]         # illustrative hat orders
CPU_HATS = ["Donkey Kong", "Luigi", "Ness", "Yoshi", "Peach", "Bowser"]

STAGES = ["Waiting Room", "Battlefield", "Small Battlefield", "Big Battlefield", "Final Destination", "Peach's Castle",
          "Kongo Jungle", "Hyrule Castle", "Super Happy Tree", "Dream Land", "Saffron City", "Mushroom Kingdom",
          "Princess Peach's Castle", "Rainbow Cruise", "Kongo Falls"]  # the first page of 118 cells (All + 117)
STAGE_ORDER = {"Waiting Room": (1, "N"), "Battlefield": (2, "N"), "Small Battlefield": (3, "N"), "Final Destination": (4, "N"),
               "Dream Land": (5, "N"), "Kongo Falls": (6, "Ω"), "Hyrule Castle": (7, "BF")}

ITEMS = [  # training_mod_consts/src/character_items.rs ITEM_NAMES (38); page 1 shows 32
    "Banjo & Kazooie: Grenade Egg", "Bowser Jr.: Mechakoopa", "Daisy: Smile Turnip", "Daisy: Winky Turnip",
    "Daisy: Dot-Eyes Turnip", "Daisy: Stitch-Face Turnip", "Daisy: Mr. Saturn", "Daisy: Bob-omb", "Diddy Kong: Banana Peel",
    "Diddy Kong: Peanut", "King K. Rool: Crown", "Link: Remote Bomb", "Link: Arrow", "Mega Man: Metal Blade", "Pac-Man: Cherry",
    "Pac-Man: Strawberry", "Pac-Man: Orange", "Pac-Man: Apple", "Pac-Man: Melon", "Pac-Man: Galaxian", "Pac-Man: Bell",
    "Pac-Man: Key", "Peach: Smile Turnip", "Peach: Winky Turnip", "Peach: Dot-Eyes Turnip", "Peach: Stitch-Face Turnip",
    "Peach: Mr. Saturn", "Peach: Bob-omb", "Richter: Holy Water", "R.O.B.: Gyro", "Robin: Tome", "Robin: Levin Sword",
    "Sheik: Burst Grenade", "Simon: Holy Water", "Snake: Hand Grenade", "Toon Link: Bomb", "Villager: Wood Chip", "Young Link: Bomb",
]

DI_SELECTED = {"Up In": 2, "Up": 1, "Up Out": 1}
KEYS = "L/R page   D-pad move   A change / open   B back   X reset page"


# ---------------------------------------------------------------------------------------------------
# Row controls (right-hand side of a page row)
# ---------------------------------------------------------------------------------------------------
def control_text(kind, value):
    if kind == "toggle":
        return f"[{value}]", (OURS if value == "ON" else MUTED)
    if kind in ("drop", "number"):
        return f"<  {value}  >", FG
    if kind == "multi":
        return f"{value}  >", FG
    if kind == "slider":
        return f"|=========---|  {value}", FG
    if kind in ("editor", "list"):
        return f"{value}  >", FG
    if kind == "info":
        return value, MUTED
    return f"{value}  >", FG


def mini_dirgrid(c, right, y, selected):
    """3x3 dots, the selected directions lit, plus the two absolute dots."""
    x0 = right - 44
    for name, (dx, dy) in list(DIRS.items())[:9]:
        cx, cy = x0 + 12 + (dx + 1) * 11, y + 4 + (dy + 1) * 11
        lit = name in selected
        c.circle(cx, cy, 4 if lit else 2.5, fill=SEL if lit else MUTED)
    c.circle(x0 - 10, y + 15, 2.5, fill=MUTED)
    c.circle(x0 + 46, y + 15, 2.5, fill=MUTED)
    return x0 - 24


def mini_portraits(c, right, y, names, color=None, ranks=False):
    x = right
    for i, name in reversed(list(enumerate(names))):
        x -= 32
        c.rect(x, y, x + 28, y + 30, fill=TILE, outline=LINE, radius=4)
        col = hue(name)
        c.circle(x + 14, y + 10, 5, fill=col)
        c.polygon([(x + 4, y + 24), (x + 9, y + 15), (x + 19, y + 15), (x + 24, y + 24)], fill=col)
        if ranks:
            c.circle(x + 5, y + 5, 5, fill=color)
            c.text(x + 5, y - 1, str(i + 1), "tiny", FG, "m")
    return x


def mini_stages(c, right, y, names):
    x = right
    for i, name in reversed(list(enumerate(names))):
        x -= 46
        col = hue(name)
        c.rect(x, y, x + 42, y + 30, fill=TILE, outline=LINE, radius=4)
        c.polygon([(x + 8, y + 22), (x + 34, y + 22), (x + 30, y + 27), (x + 12, y + 27)], fill=col)
        c.rect(x + 14, y + 12, x + 28, y + 14, fill=col)
        c.circle(x + 6, y + 6, 5, fill=P1)
        c.text(x + 6, y, str(i + 1), "tiny", FG, "m")
    return x


def draw_control(c, kind, value, y, right):
    """Draw the control at the row's right edge; returns nothing. Grid rows get a mini preview."""
    if kind == "dirgrid":
        sel = DI_SELECTED if value.startswith("3") else {}
        x = mini_dirgrid(c, right, y, sel)
        c.text(x - 10, y + 4, f"{value}  >", "s", FG, "r")
    elif kind == "dirsingle":
        x = mini_dirgrid(c, right, y, {value} if value != "Neutral" else {"Neutral"})
        c.text(x - 10, y + 4, f"<  {value}  >", "s", FG, "r")
    elif kind == "portraits":
        x = mini_portraits(c, right, y - 4, [value])
        c.text(x - 10, y + 2, f"{value}  >", "s", FG, "r")
    elif kind == "cycles":
        x = mini_portraits(c, right, y - 4, CPU_CYCLE, CPU, True)
        x = mini_portraits(c, x - 10, y - 4, P1_CYCLE, P1, True)
        c.text(x - 10, y + 2, f"{value}  >", "s", FG, "r")
    elif kind == "hats":
        chip_kirby(c, right - 18, y + 12, 36)
        c.text(right - 44, y + 2, f"{value}  >", "s", FG, "r")
    elif kind == "items":
        chip_items(c, right - 18, y + 12, 36)
        c.text(right - 44, y + 2, f"{value}  >", "s", FG, "r")
    elif kind == "popo":
        chip_popo(c, right - 18, y + 12, 38)
        t, col = control_text("toggle", value)
        c.text(right - 44, y, t, "t", col, "r")
    elif kind == "stages":
        x = mini_stages(c, right, y - 4, STAGES[:3])
        c.text(x - 10, y + 2, f"{value}  >", "s", FG, "r")
    else:
        t, col = control_text(kind, value)
        c.text(right, y, t, "t", col, "r")


# ---------------------------------------------------------------------------------------------------
# Screens
# ---------------------------------------------------------------------------------------------------
def frame(c, tabs, active, footer_help, crumb=None):
    c.rect(28, 24, W - 28, H - 24, fill=PANEL, outline=LINE)
    x = 48
    for i, name in enumerate(tabs):
        w = c.textlength(name, "tab") + 28
        if i == active:
            c.rect(x, 40, x + w, 76, fill=SEL)
            c.text(x + 14, 47, name, "tab", FG)
        else:
            c.text(x + 14, 47, name, "tab", MUTED)
        x += w + 10
    c.text(W - 60, 52, "TrainingRooms", "s", MUTED, "r")
    c.line([(48, 88), (W - 48, 88)], LINE)
    c.line([(48, H - 92), (W - 48, H - 92)], LINE)
    c.text(56, H - 82, footer_help, "s", FG)
    c.text(56, H - 56, KEYS, "xs", MUTED)
    if crumb:
        c.text(48, 100, crumb, "h", FG)


def render_page(pi, selected_group=0):
    name, groups = PAGES[pi]
    c = Canvas()
    gname, adv, items = groups[selected_group]
    first = items[0]
    frame(c, [p[0] for p in PAGES], pi, HELP.get(first[0], f"{first[0]}: {first[2]}"))
    y = 104
    for gi, (g, a, its) in enumerate(groups):
        if gi == selected_group:
            c.rect(48, y - 6, 372, y + 26, fill=HILITE)
            c.rect(48, y - 6, 52, y + 26, fill=SEL)
        c.text(62, y, f"{g}  ({len(its)})", "h" if gi == selected_group else "t", FG if gi == selected_group else MUTED)
        if a:
            c.text(62, y + 24, "advanced - collapsed by default", "xs", ADV)
            y += 20
        y += 40
    c.line([(388, 96), (388, H - 100)], LINE)
    y = 104
    for ii, (label, kind, value, origin) in enumerate(items):
        if ii == 0:
            c.rect(404, y - 6, W - 48, y + 26, fill=HILITE)
            c.rect(404, y - 6, 408, y + 26, fill=SEL)
        c.text(418, y, label, "t", FG)
        tag = {"ours": ("ours", OURS), "up": ("upstream", UP), "mixed": ("upstream+ours", MUTED)}[origin]
        c.text(418 + c.textlength(label, "t") + 14, y + 4, tag[0], "xs", tag[1])
        draw_control(c, kind, value, y, W - 60)
        y += 38
        if y > H - 120:
            c.text(418, y, "...", "t", MUTED)
            break
    return c


def render_subscreen(si):
    title, crumb, rows, help_text = SUBSCREENS[si]
    c = Canvas()
    frame(c, [crumb.split(" > ")[0]], 0, help_text, f"{crumb}  >  {title}")
    y = 146
    cols = 2 if sum(1 for r in rows if r[1] == "check") > 8 else 1
    colw = (W - 96) // cols
    for i, (label, kind, value, _) in enumerate(rows):
        if kind == "check":
            cx = 48 + (i % cols) * colw
            c.rect(cx + 10, y + 2, cx + 30, y + 22, outline=FG, width=2)
            if value == "x":
                c.rect(cx + 14, y + 6, cx + 26, y + 18, fill=SEL)
            c.text(cx + 44, y, label, "t", FG)
            if cols == 1 or i % cols == cols - 1:
                y += 36
        else:
            c.text(58, y, label, "t", FG)
            t, col = (f"[ {value} ]   press A to re-bind", FG) if kind == "chord" else control_text(kind, value)
            c.text(W - 60, y, t, "t", col, "r")
            y += 40
    return c


def render_direction_grid():
    c = Canvas()
    frame(c, ["CPU"], 0, "A adds one weight to the chip (random pick is by weight), Y clears it, ZL all / none, B back.",
          "CPU  >  Defence  >  DI Direction")
    s, pitch = 108, 132
    gx, gy = 150, 338  # centre column x, centre row y
    c.text(gx, 128, "In (toward the opponent)", "xs", MUTED, "m")
    c.text(gx + 2 * pitch, 128, "Out (away)", "xs", MUTED, "m")
    for name, (dx, dy) in list(DIRS.items())[:9]:
        cx, cy = gx + (dx + 1) * pitch, gy + dy * pitch
        chip_direction(c, cx, cy, s, name, name in DI_SELECTED, DI_SELECTED.get(name, 0), cursor=(name == "Up Out"))
        c.text(cx, cy + s / 2 + 5, name, "xs", FG if name in DI_SELECTED else MUTED, "m")
    ax = gx + 3 * pitch + 80
    c.text(ax, 128, "absolute", "xs", MUTED, "m")
    for name, cy in (("Left", gy - 72), ("Right", gy + 72)):
        chip_direction(c, ax, cy, s, name, absolute=True)
        c.text(ax, cy + s / 2 + 5, name, "xs", MUTED, "m")
    # legend
    lx = ax + 100
    c.rect(lx, 120, W - 48, H - 112, fill=HILITE, outline=LINE, radius=6)
    c.text(lx + 16, 132, "Selected (weight)", "h", FG)
    y = 168
    for name, wgt in DI_SELECTED.items():
        c.text(lx + 16, y, f"{name}", "t", FG)
        c.text(W - 64, y, f"x{wgt}", "t", OURS, "r")
        y += 30
    y += 10
    for line in ["In / Out mirror with the CPU's facing:", "the stick angle flips when the player", "stands on the other side.",
                 "", "Left / Right never mirror.", "", "Nothing selected: the CPU does not", "influence its launch.",
                 "", "Same grid for SDI Direction,", "Airdodge Direction and Shield Tilt", "(Shield Tilt: one chip only)."]:
        c.text(lx + 16, y, line, "xs", MUTED if line else FG)
        y += 22
    return c


def render_roster_grid(hats=False):
    c = Canvas()
    title = "Kirby Hats" if hats else "Character Cycles"
    frame(c, ["Fighters & Stages"], 0,
          "A adds the fighter to the P1 order, ZR switches to the CPU order (blue), Y marks Solo Popo on Ice Climbers, X previews a form." if not hats
          else "A adds the hat to the P1 order, ZR switches to the CPU order (blue). Kirby takes the next hat in this order on each hat cycle.",
          f"Fighters & Stages  >  {'Extras' if hats else 'Player 1 and CPU'}  >  {title}")
    c.text(W - 60, 100, "editing: P1 order (red)   ZR: CPU order", "s", P1, "r")
    if hats:
        chip_kirby(c, 620, 112, 46)
    p1 = P1_HATS if hats else P1_CYCLE
    cpu = CPU_HATS if hats else CPU_CYCLE
    tw, th, px, py = 82, 56, 88, 64
    x0 = 48 + (W - 96 - 13 * px + 6) / 2
    y0 = 136
    for r, row in enumerate(ROSTER):
        start = 2 if r == 6 else 0
        for i, name in enumerate(row):
            x = x0 + (start + i) * px
            y = y0 + r * py
            kind = "all" if name == "ALL" else "fighter"
            tile_portrait(c, x, y, tw, th, name,
                          rank_p=p1.index(name) + 1 if name in p1 else None,
                          rank_c=cpu.index(name) + 1 if name in cpu else None,
                          solo="p" if (name == "Ice Climbers" and not hats) else None,
                          cursor=(name == ("Fox" if hats else "Palutena")), kind=kind)
    c.text(x0, y0 + 7 * py + 2, "Cursor: " + ("Fox — P1 hat 3" if hats else "Palutena — P1 2nd, not in the CPU order"), "xs", FG)
    return c


def render_item_grid():
    c = Canvas()
    frame(c, ["Fighters & Stages"], 0, "A gives Player 1 the item now (one at a time). The icons are the game's own item pictures.",
          "Fighters & Stages  >  Extras  >  Give item")
    chip_items(c, 620, 112, 46)
    c.text(W - 60, 100, "page 1 / 2   (38 items)", "s", MUTED, "r")
    tw, th, px, py = 280, 48, 292, 54
    x0, y0 = 48 + (W - 96 - 4 * px + 12) / 2, 136
    for i, name in enumerate(ITEMS[:32]):
        col, row = i % 4, i // 4
        tile_item(c, x0 + col * px, y0 + row * py, tw, th, name, cursor=(i == 8))
    return c


def render_stage_grid():
    c = Canvas()
    frame(c, ["Fighters & Stages"], 0, "A adds the stage to the rotation order, Y cycles its form (Normal / Omega / Battlefield), L/R pages.",
          "Fighters & Stages  >  Stages  >  Lobby Stages")
    c.text(W - 60, 100, "page 1 / 8   (All + 117 stages)", "s", MUTED, "r")
    tw, th, px, py = 272, 100, 290, 108
    x0, y0 = 48 + (W - 96 - 4 * px + 18) / 2, 136
    cells = ["ALL"] + STAGES
    for i, name in enumerate(cells[:16]):
        col, row = i % 4, i // 4
        if name == "ALL":
            tile_stage(c, x0 + col * px, y0 + row * py, tw, th, name, all_tile=True)
        else:
            rank, form = STAGE_ORDER.get(name, (None, None))
            tile_stage(c, x0 + col * px, y0 + row * py, tw, th, name, rank, form, cursor=(name == "Small Battlefield"))
    c.text(x0, y0 + 4 * py + 2, "Cursor: Small Battlefield — 3rd in the rotation, Normal form   (Ω omega, BF battlefield form)", "xs", FG)
    return c


def render_chip_sheet():
    """The three pictogram chips at review size, their row size, and the direction chip set."""
    c = Canvas()
    frame(c, ["Chips"], 0, "Our own vector art (MIT): a brief from the operator's sketches, redrawn from primitives; no asset is traced or embedded.",
          "Pictogram chips  >  review sheet")
    for i, (name, fn, where) in enumerate([("Kirby Hats", chip_kirby, "Fighters & Stages > Extras > Kirby Hats row, and the hat grid header"),
                                            ("Solo Popo", chip_popo, "Fighters & Stages > Player 1 and CPU > Solo Popo row"),
                                            ("Character items", chip_items, "Fighters & Stages > Extras > Give item row, and the item grid header")]):
        x = 150 + i * 390
        c.rect(x - 110, 140, x + 110, 360, fill=TILE, outline=LINE, radius=10)
        fn(c, x, 250, 190)
        c.rect(x + 130, 190, x + 190, 250, fill=TILE, outline=LINE, radius=6)
        fn(c, x + 160, 220, 38)
        c.text(x + 160, 258, "row size", "tiny", MUTED, "m")
        c.text(x, 372, name, "h", FG, "m")
        c.text(x, 398, where, "tiny", MUTED, "m")
    c.text(48, 440, "Direction chips (DI, SDI, Airdodge, Shield Tilt): a bold stroke from the centre with an arrowhead; Neutral is a ring; Left/Right carry 'abs'.", "xs", FG)
    for i, name in enumerate(["Up In", "Up", "Up Out", "In", "Neutral", "Out", "Down In", "Down", "Down Out", "Left", "Right"]):
        x = 90 + i * 100
        chip_direction(c, x, 520, 72, name, selected=(name in DI_SELECTED), weight=DI_SELECTED.get(name, 0), absolute=name in ("Left", "Right"))
        c.text(x, 562, name, "tiny", MUTED, "m")
    return c


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for old in OUT.glob("*.png"):
        old.unlink()
    for old in OUT.glob("*.svg"):
        old.unlink()
    index = ["# Menu mockup screens (proposal v3, in-game look, 1280x720)", "",
             "Rendered by `tools/render_menu_mockup.py` from MENU-DRAFT.md §2. Pages show the group that best shows the new controls; "
             "the help bar shows the focused item's text. Portraits, stage thumbnails and item icons are stand-ins for the images the "
             "fork's builders derive from the player's own game files at build time; the chips and direction glyphs are the clean room's own vector art.", ""]
    screens = [
        ("01-cpu", lambda: render_page(0, 1), "page CPU (Defence group: direction grids)"),
        ("02-saves", lambda: render_page(1), "page Saves"),
        ("03-recording", lambda: render_page(2), "page Recording"),
        ("04-fighters-stages", lambda: render_page(3), "page Fighters & Stages (Player 1 and CPU: portrait pickers, cycle preview, Solo Popo chip)"),
        ("05-display-controls", lambda: render_page(4), "page Display & Controls"),
        ("06-situation-overrides", lambda: render_subscreen(0), "sub-screen Situation overrides (CPU > Reactions)"),
        ("07-button-chords", lambda: render_subscreen(1), "sub-screen Button chords (Display & Controls > Controls)"),
        ("08-swap-groups", lambda: render_subscreen(2), "sub-screen Swap groups (Fighters & Stages > Loading)"),
        ("09-direction-grid", render_direction_grid, "sub-screen DI Direction: the 3x3 direction grid with absolute Left/Right and weights"),
        ("10-character-grid", lambda: render_roster_grid(False), "sub-screen Character Cycles: the fork's 13x7 portrait grid with P1 (red) and CPU (blue) order badges"),
        ("11-item-grid", render_item_grid, "sub-screen Give item: 4x8 item grid, page 1 of 2"),
        ("12-stage-grid", render_stage_grid, "sub-screen Lobby Stages: 4x4 stage thumbnail page with All, order badges and form tags"),
        ("13-kirby-hats-grid", lambda: render_roster_grid(True), "sub-screen Kirby Hats: the same portrait grid as a two-role hat order"),
        ("14-fighters-extras", lambda: render_page(3, 1), "page Fighters & Stages (Extras group: Kirby hats chip, items chip, Mii lists)"),
        ("15-fighters-stage-rows", lambda: render_page(3, 2), "page Fighters & Stages (Stages group: stage order preview)"),
        ("16-chip-sheet", render_chip_sheet, "review sheet: the three pictogram chips at full and row size, and the direction chip set"),
    ]
    for slug, fn, what in screens:
        fn().save(slug)
        index.append(f"- `{slug}.png` / `.svg` — {what}")
    (OUT / "index.md").write_text("\n".join(index) + "\n", encoding="utf-8")
    print(f"wrote {2 * len(screens)} files to {OUT}")


if __name__ == "__main__":
    main()
