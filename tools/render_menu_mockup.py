#!/usr/bin/python3 -I
"""Render the proposed TrainingRooms menu (MENU-DRAFT.md §2, proposal v8) as PNG and SVG screens in the
in-game overlay look, 1280x720 (Switch handheld).

v3 added the fork's grid pickers (portrait roster grid 13x7, stage thumbnail pages 4x4, item grid 4x8)
and a 3x3 direction grid for DI / SDI / Airdodge / Shield Tilt. v4 draws the portrait grid and the
stage grid with the real fighter icons and stage thumbnails, cropped from captures of the fork's own
pickers by tools/crop_menu_captures.py into docs/menu-mockup/captures/ (review material only, never
shipped), and replaces the Kirby Hats, Solo Popo and character-items pictograms with one neutral
placeholder: their final icons are open until the functionality is settled. v5 draws the item grid with
the fork's own 38 item icons (byte copies in docs/menu-mockup/captures/items/) and labels the stage grid
as a mock of run-time thumbnails: the clean room reads each stage's own game UI texture when the grid
opens, so a stage mod's picture shows; the stage crops stand in for that. v6 removes the small origin tags
("upstream" / "ours") from the page rows: origin is internal information and lives in PROVENANCE.md and
the provenance table of docs/MENU-PROPOSAL.md; the fourth field of each row below is kept as that
internal record and is never drawn. v8 draws the DI Direction chips with the game's own DI streak (the
system effect `sys_vector`), a crop of one frame of a real Eden capture (tools/crop_menu_captures.py,
docs/menu-mockup/captures/di/), rotated per direction as the game rotates that effect to the launch angle
(SPEC-core-07 §4.6); Airdodge Direction and Shield Tilt keep the clean room's arrows. v9 draws the DI and
SDI Direction chips from game-derived renders of the two effects (c1162 W9: the effect file's own textures,
meshes and colours; docs/menu-mockup/captures/renders/), one still per effect turned per direction; adds the
SDI Direction grid (screen 17); and puts crops of the real captures (the DI streak, the SDI spark Up and In,
docs/menu-mockup/captures/di/ and sdi/) beside the renders on the chip sheet as the visual check. v10 lengthens the
SDI burst x2.0 along the SDI vector (tools/render_sdi_stretch.py, the burst emitter's scale-Y; x1.17, x1.4, x1.7 compared on
sheet 21) and adds the screens for the
decided items (DECISIONS N1-N7): 02, 03 and 05 updated, 11 as one docked page, 18 handheld item grid, 19 Aerial Delay popup,
20 Developer page.

Every screen is drawn once through `Canvas`, which records the primitives and writes both the PNG
(Pillow) and a text SVG (images referenced by relative path), so the two never drift.

Output: docs/menu-mockup/NN-<screen>.png and .svg plus index.md. Pillow only; DejaVu Sans Mono for the
modpack-style monospace text.
"""
import json
import math
import textwrap
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

W, H = 1280, 720
OUT = Path(__file__).resolve().parent.parent / "docs" / "menu-mockup"
CAPS = OUT / "captures"  # per-cell crops of the fork's grid pickers (tools/crop_menu_captures.py)
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
ADV = (210, 170, 80)
HILITE = (38, 42, 52)  # selected row: a solid lift over the panel (Pillow's RGBA draw does not blend alpha fills)
TILE = (28, 32, 42)
TILE_HI = (46, 52, 66)
P1 = (232, 72, 72)      # the fork's red Player order
CPU = (72, 150, 232)    # the fork's blue CPU order

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


def slug(name):
    """File name of a grid cell crop: 'Mr. Game & Watch' -> 'mr-game-watch', 'Pokémon Trainer' -> 'pokemon-trainer'."""
    s = name.lower().replace("é", "e").replace("&", " ")
    return "-".join("".join(ch if ch.isalnum() else " " for ch in s).split())


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

    def image(self, rel, x0, y0, x1, y1):
        """A crop from docs/menu-mockup/<rel>, aspect kept and centred in the box; SVG refers to the file."""
        im = Image.open(OUT / rel).convert("RGBA")
        k = min((x1 - x0) / im.width, (y1 - y0) / im.height)
        w, h = max(1, round(im.width * k)), max(1, round(im.height * k))
        ix, iy = round(x0 + (x1 - x0 - w) / 2), round(y0 + (y1 - y0 - h) / 2)
        self.img.alpha_composite(im.resize((w, h), Image.LANCZOS), (ix, iy))
        self.svg.append(f'<image href="{rel}" x="{ix}" y="{iy}" width="{w}" height="{h}" preserveAspectRatio="none"/>')

    def image_rotated(self, rel, px, py, scale, deg, pivot, alpha=1.0):
        """A crop scaled by `scale`, its `pivot` (crop pixels) placed on (px, py) and rotated `deg` clockwise
        (screen coordinates) about it; `alpha` dims it. SVG refers to the file with a rotate transform."""
        im = Image.open(OUT / rel).convert("RGBA")
        w, h = max(1, round(im.width * scale)), max(1, round(im.height * scale))
        im = im.resize((w, h), Image.LANCZOS)
        pvx, pvy = pivot[0] * scale, pivot[1] * scale
        r = math.ceil(math.hypot(max(pvx, w - pvx), max(pvy, h - pvy))) + 2
        big = Image.new("RGBA", (2 * r, 2 * r), (0, 0, 0, 0))
        ox, oy = round(r - pvx), round(r - pvy)
        big.alpha_composite(im, (ox, oy))
        big = big.rotate(-deg, resample=Image.BICUBIC, center=(r, r))
        if alpha < 1:
            big.putalpha(big.getchannel("A").point(lambda v: round(v * alpha)))
        bx, by = round(px - r), round(py - r)
        self.img.alpha_composite(big, (bx, by))
        op = f' opacity="{alpha:.2f}"' if alpha < 1 else ""
        self.svg.append(f'<image href="{rel}" x="{bx + ox}" y="{by + oy}" width="{w}" height="{h}" preserveAspectRatio="none" '
                        f'transform="rotate({deg:.1f} {bx + r} {by + r})"{op}/>')

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


# ---------------------------------------------------------------------------------------------------
# Placeholder chip (Kirby Hats, Solo Popo, character items): the final icons are open and will be
# designed once the functionality is settled; until then one neutral, clearly marked placeholder.
# ---------------------------------------------------------------------------------------------------
def rounded_path(x0, y0, x1, y1, r, step=15):
    pts = []
    for cx, cy, a0 in ((x1 - r, y0 + r, 270), (x1 - r, y1 - r, 0), (x0 + r, y1 - r, 90), (x0 + r, y0 + r, 180)):
        for a in range(a0, a0 + 91, step):
            pts.append((cx + r * math.cos(math.radians(a)), cy + r * math.sin(math.radians(a))))
    return pts + [pts[0]]


def chip_placeholder(c, cx, cy, s, label, note="icon TBD"):
    """A dashed rounded square with a short label; below 60 px (a menu row) the label is dropped."""
    half = s / 2
    c.rect(cx - half, cy - half, cx + half, cy + half, fill=TILE, radius=s * 0.16)
    dash = (max(3, s / 12), max(2, s / 18))
    c.line(rounded_path(cx - half, cy - half, cx + half, cy + half, s * 0.16), MUTED, max(1, round(s / 40)) + 1, dash=dash)
    if s >= 60:
        c.text(cx, cy - s * 0.16, label, "b" if s < 120 else "h", FG, "m")
        c.text(cx, cy + s * 0.08, note, "tiny" if s < 120 else "xs", MUTED, "m")
    else:
        c.text(cx, cy - 6, "?", "b", MUTED, "m")


DIRS = {  # name: (dx, dy) in screen space; In is toward the opponent (drawn left), Out away (right)
    "Up In": (-1, -1), "Up": (0, -1), "Up Out": (1, -1),
    "In": (-1, 0), "Neutral": (0, 0), "Out": (1, 0),
    "Down In": (-1, 1), "Down": (0, 1), "Down Out": (1, 1),
    "Left": (-1, 0), "Right": (1, 0),
}


def di_streak():
    """The DI streak crop (tools/crop_menu_captures.py): file, the anchor that sits on the chip centre (the
    strip's tail end, so the streak runs from the centre toward the direction) and the strip length."""
    rec = json.loads((CAPS / "crops.json").read_text(encoding="utf-8"))["di_streak"]
    w, h = rec["size"]
    return f"captures/{rec['file']}", (rec["feather"][0] // 2, h / 2), w


def sdi_spark(which="sdi_spark"):
    """An SDI spark crop (tools/crop_menu_captures.py): file, the anchor that sits on the chip centre (the
    spark's ring centre, i.e. the CPU's head) and its reach toward the direction (crop pixels)."""
    rec = json.loads((CAPS / "crops.json").read_text(encoding="utf-8"))[which]
    return f"captures/{rec['file']}", tuple(rec["pivot_in_crop"]), rec["along"][1]


def w9_render(visual):
    """The game-derived chip render (c1162 W9, copied by tools/crop_menu_captures.py): file, its centre (the
effect origin, which sits on the chip centre) and its size; drawn pointing Up, reach 0.46 x chip size. Since v10
    the SDI chip draws the taller-burst still (tools/render_sdi_stretch.py); the DI chip draws W9's file unchanged."""
    crops = json.loads((CAPS / "crops.json").read_text(encoding="utf-8"))
    if visual.startswith("sdi@"):  # a compared variant (chip sheet 21): "sdi@<factor>"
        f = next(v for v in crops["sdi_stretch"]["variants"] if v["factor"] == float(visual[4:]))
    elif visual == "sdi":
        f = crops["sdi_stretch"]
    else:  # "di", or "sdi-v9": W9's own file, unchanged
        f = next(r for r in crops["w9_renders"]["files"] if r["file"].endswith(f"/{visual.replace('-v9', '')}-up-4x-layer.png"))
    return f"captures/{f['file']}", (f["size"][0] / 2, f["size"][1] / 2), f["size"][0]


def chip_direction(c, cx, cy, s, name, selected=False, weight=0, absolute=False, cursor=False, visual="arrow"):
    """One direction chip. visual="arrow": a bold stroke from the centre with an arrowhead (Airdodge, Shield
    Tilt). visual="di" / "sdi": the game-derived render of the DI / SDI effect (c1162 W9), its origin on the chip
    centre, turned from Up to the direction as the game turns the one effect to the launch angle / SDI vector
    (SPEC-core-07 §4.6). The captures, for the chip sheet's check column: visual="di-capture" (the DI streak
    crop), "sdi-capture" (the SDI Up spark crop, ring centre on the chip centre), "sdi-check" (the SDI In spark
    crop). Neutral is a ring and a dot in all."""
    half = s / 2
    c.rect(cx - half, cy - half, cx + half, cy + half, fill=TILE_HI if selected else TILE,
           outline=SEL if cursor else (FG if selected else LINE), width=3 if cursor else 2, radius=s * 0.14)
    dx, dy = DIRS[name]
    col = FG if selected else MUTED
    if (visual in ("di", "sdi", "sdi-v9") or visual.startswith("sdi@")) and (dx, dy) != (0, 0):
        rel, anchor, size = w9_render(visual)
        c.image_rotated(rel, cx, cy, s / size, math.degrees(math.atan2(dy, dx)) + 90, anchor, 1.0 if selected else 0.5)
        c.circle(cx, cy, max(2, s * 0.05), fill=col)
    elif visual == "di-capture" and (dx, dy) != (0, 0):
        rel, anchor, length = di_streak()
        c.image_rotated(rel, cx, cy, 0.46 * s / length, math.degrees(math.atan2(dy, dx)), anchor, 1.0 if selected else 0.5)
        c.circle(cx, cy, max(2, s * 0.05), fill=col)
    elif visual in ("sdi-capture", "sdi-check") and (dx, dy) != (0, 0):
        rel, anchor, reach = sdi_spark("sdi_spark" if visual == "sdi-capture" else "sdi_spark_check")
        c.image_rotated(rel, cx, cy, 0.40 * s / reach, math.degrees(math.atan2(dy, dx)), anchor, 1.0 if selected else 0.5)
        c.circle(cx, cy, max(2, s * 0.05), fill=col)
    elif (dx, dy) == (0, 0):
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
# Grid tiles (portraits and stage thumbnails: crops of the fork's grids; item icons: the fork's own files)
# ---------------------------------------------------------------------------------------------------
def tile_portrait(c, x, y, w, h, name, rank_p=None, rank_c=None, solo=None, cursor=False, kind="fighter"):
    c.rect(x, y, x + w, y + h, fill=TILE_HI if cursor else TILE, outline=SEL if cursor else LINE, width=3 if cursor else 1, radius=6)
    cx = x + w / 2
    # the fighter's real icon as the fork's portrait grid draws it (a crop of a capture of that grid)
    c.image(f"captures/portraits/{slug(name)}.png", cx - 20, y + 3, cx + 20, y + h - 15)
    label = "ALL" if kind == "all" else (name if len(name) <= 11 else name[:10] + "…")
    c.text(cx, y + h - 14, label, "tiny", FG, "m")
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
        c.image("captures/stages/all.png", x + w / 2 - 36, y + 5, x + w / 2 + 36, y + th)
        c.text(x + w / 2, y + h - 19, "All (listed order)", "xxs", FG, "m")
        return
    # a capture of the stock thumbnail as the fork's grid draws it; the clean room reads each stage's own game
    # UI texture at run time instead (so a stage mod's picture shows). Waiting Room is a text tile in both.
    if name != "Waiting Room":
        c.image(f"captures/stages/{slug(name)}.png", x + 6, y + 5, x + w - 6, y + th)
    label = name if len(name) <= 24 else name[:23] + "…"
    c.text(x + w / 2, y + h - 19, label, "xxs", FG, "m")
    if rank is not None:
        c.circle(x + 14, y + 14, 10, fill=P1)
        c.text(x + 14, y + 7, str(rank), "xxs", FG, "m")
    if form:
        c.rect(x + w - 40, y + 6, x + w - 6, y + 24, fill=HILITE, radius=4)
        c.text(x + w - 23, y + 8, form, "xxs", OURS, "m")


def tile_item(c, x, y, w, h, index, name, cursor=False):
    c.rect(x, y, x + w, y + h, fill=TILE_HI if cursor else TILE, outline=SEL if cursor else LINE, width=3 if cursor else 1, radius=6)
    # the fork's own item-grid picture (scripts/item-icons/assets/icon-NN.png, 64x64), copied unchanged
    c.image(f"captures/items/icon-{index:02}.png", x + 6, y + 4, x + 46, y + h - 4)
    who, what = name.split(": ", 1)
    c.text(x + 50, y + 7, what if len(what) <= 24 else what[:23] + "…", "xxs", FG)
    c.text(x + 50, y + 25, who, "tiny", MUTED)


# ---------------------------------------------------------------------------------------------------
# Data (MENU-DRAFT.md §2 and the fork's own tables)
# ---------------------------------------------------------------------------------------------------
# kind: toggle | drop | multi | slider | number | editor | list | info | dirgrid | digrid | sdigrid | dirsingle
#       | portraits | cycles | hats | items | stages | popo ; value = text shown in the control
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
            ("DI Direction", "digrid", "3 selected", "up"),
            ("SDI Direction", "sdigrid", "2 selected", "up"),
            ("SDI Strength", "drop", "High", "up"),
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
            ("Respawn invincibility", "toggle", "OFF", "ours"),   # N2; greyed while Auto-load on death is On
            ("Player damage on load", "drop", "Default", "up"),
            ("Player damage range", "slider", "0 - 150 %", "up"),
            ("CPU damage on load", "drop", "Default", "up"),
            ("CPU damage range", "slider", "0 - 150 %", "up"),
            ("Buffs on load", "multi", "0 of 12 selected", "up"),
        ]),   # "Item on load (older saves)" is gone (N6): the first-boot import converts older saves once
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
            ("Settings override playback", "toggle", "OFF", "ours"),   # N4, advanced row
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
        ]),   # Preload Samples moved to the Developer page (Q8 / N1)
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
            ("Menu shortcut 1", "editor", "Minus", "ours"),        # N7: opens the chord editor
            ("Menu shortcut 2", "editor", "disabled", "ours"),
            ("Permanent shortcut", "info", "B + D-pad Up", "ours"),
        ]),
    ]),
]

# The sixth page (N1, SPEC-core-01 §2 and §6): hidden unless developer.ini has developer_page=on. Rows are the three
# the specs describe, all toggles, default Off; the raygun printer has no row because the specs do not describe it.
DEV_PAGE = ("Developer", [
    ("Developer", False, [
        ("Preload Samples", "toggle", "OFF", "ours"),
        ("Reload developer config", "toggle", "OFF", "ours"),
        ("Debug hooks", "toggle", "OFF", "ours"),
    ]),
])
DEV_NOTES = [  # what each row does, from SPEC-core-01 §2 (drawn under the rows, muted)
    ("Preload Samples", "sparse timing and memory samples of the cycle preload go to the diagnostics file."),
    ("Reload developer config", "while On and P1 holds L+R+A, the developer config file is re-read every frame."),
    ("Debug hooks", "a debug-only watch on a game float write (Kirby backshield); installed at boot only when this page is on."),
]
ADV_ROWS = {"Settings override playback"}  # a single advanced row (the group is not): tagged next to its label

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
    "Respawn invincibility": "On: the game's respawn invincibility and, where possible, the respawn platform. Greyed while Auto-load on death is On.",
    "Settings override playback": "Off: a playing recording wins over DI, SDI, air-dodge, tech and throw settings. On: those settings override it.",
    "Menu shortcut 1": "Opens the chord editor. A change applies at once and is written to button_config.ini.",
    "Menu shortcut 2": "Opens the chord editor. A change applies at once and is written to button_config.ini.",
    "Preload Samples": "Timing and memory samples of the cycle preload, written to the diagnostics file. Inert unless the page is enabled.",
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
SDI_SELECTED = {"Up": 1, "In": 1}  # the two directions captured in W8 (with Strength High)
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
        c.image(f"captures/portraits/{slug(name)}.png", x + 1, y + 2, x + 27, y + 28)
        if ranks:
            c.circle(x + 5, y + 5, 5, fill=color)
            c.text(x + 5, y - 1, str(i + 1), "tiny", FG, "m")
    return x


def mini_stages(c, right, y, names):
    x = right
    for i, name in reversed(list(enumerate(names))):
        x -= 46
        c.rect(x, y, x + 42, y + 30, fill=TILE, outline=LINE, radius=4)
        if name == "Waiting Room":
            c.text(x + 23, y + 10, "WR", "tiny", FG, "m")  # a text tile in the fork's grid as well
        else:
            c.image(f"captures/stages/{slug(name)}.png", x + 2, y + 2, x + 40, y + 28)
        c.circle(x + 6, y + 6, 5, fill=P1)
        c.text(x + 6, y, str(i + 1), "tiny", FG, "m")
    return x


def draw_control(c, kind, value, y, right):
    """Draw the control at the row's right edge; returns nothing. Grid rows get a mini preview."""
    if kind == "dirgrid":
        x = mini_dirgrid(c, right, y, {})
        c.text(x - 10, y + 4, f"{value}  >", "s", FG, "r")
    elif kind == "digrid":  # the selected DI chips, each the game's DI streak at that direction
        x = right
        for name in reversed(list(DI_SELECTED)):
            x -= 34
            chip_direction(c, x + 15, y + 11, 30, name, selected=True, visual="di")
        c.text(x - 10, y + 4, f"{value}  >", "s", FG, "r")
    elif kind == "sdigrid":  # the selected SDI chips, each the game's SDI spark at that direction
        x = right
        for name in reversed(list(SDI_SELECTED)):
            x -= 34
            chip_direction(c, x + 15, y + 11, 30, name, selected=True, visual="sdi")
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
        chip_placeholder(c, right - 18, y + 12, 34, "HATS")
        c.text(right - 44, y + 2, f"{value}  >", "s", FG, "r")
    elif kind == "items":
        chip_placeholder(c, right - 18, y + 12, 34, "ITEMS")
        c.text(right - 44, y + 2, f"{value}  >", "s", FG, "r")
    elif kind == "popo":
        chip_placeholder(c, right - 18, y + 12, 34, "POPO")
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


def render_page(pi, selected_group=0, focus=0, dev=False):
    """One page, one group selected, row `focus` of that group highlighted (its help text in the footer). `dev`: the
    sixth, hidden Developer page (pi is then ignored): six tabs, a header note, and one line per row under the rows."""
    pages = PAGES + [DEV_PAGE] if dev else PAGES
    name, groups = pages[len(PAGES) if dev else pi]
    c = Canvas()
    gname, adv, items = groups[selected_group]
    first = items[focus]
    frame(c, [p[0] for p in pages], len(PAGES) if dev else pi, HELP.get(first[0], f"{first[0]}: {first[2]}"))
    top = 104
    if dev:  # the note sits between the tab strip and the panes, so the panes start lower
        c.rect(48, 98, W - 48, 128, fill=HILITE, outline=ADV, radius=6)
        c.text(62, 105, "Hidden page: shown only when developer.ini has developer_page=on (read once at boot); rows do nothing otherwise.",
               "xs", ADV)
        top = 148
    y = top
    for gi, (g, a, its) in enumerate(groups):
        if gi == selected_group:
            c.rect(48, y - 6, 372, y + 26, fill=HILITE)
            c.rect(48, y - 6, 52, y + 26, fill=SEL)
        c.text(62, y, f"{g}  ({len(its)})", "h" if gi == selected_group else "t", FG if gi == selected_group else MUTED)
        if a:
            c.text(62, y + 24, "advanced - collapsed by default", "xs", ADV)
            y += 20
        y += 40
    c.line([(388, top - 8), (388, H - 100)], LINE)
    y = top
    for ii, (label, kind, value, _origin) in enumerate(items):  # _origin: internal only, never drawn (v6)
        if ii == focus:
            c.rect(404, y - 6, W - 48, y + 26, fill=HILITE)
            c.rect(404, y - 6, 408, y + 26, fill=SEL)
        c.text(418, y, label, "t", FG)
        if label in ADV_ROWS:
            c.text(418 + c.textlength(label, "t") + 14, y + 4, "advanced", "xxs", ADV)
        draw_control(c, kind, value, y, W - 60)
        y += 38
        if y > H - 120:
            c.text(418, y, "...", "t", MUTED)
            break
    if dev:
        y += 14
        for label, text in DEV_NOTES:
            c.text(418, y, label, "s", FG)
            lines = textwrap.wrap(text, 92)
            for k, line in enumerate(lines):
                c.text(418, y + 20 + 18 * k, line, "xs", MUTED)
            y += 30 + 18 * len(lines)
        c.text(418, y + 6, "No row for the raygun printer: the specs do not describe it yet.", "xs", ADV)
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


DIR_GRIDS = {  # breadcrumb title, selection, chip visual, cursor chip, legend lines
    "di": ("DI Direction", DI_SELECTED, "di", "Up Out",
           ["In / Out mirror with the CPU's facing;", "Left / Right never mirror. Nothing", "selected: the CPU does not DI.",
            "", "Chip: the game's own DI streak, turned", "to each direction as the game turns it", "to the launch angle. Read from the",
            "game's files when the grid opens.", "(This mock: a render from the game's", "effect file; captures on the sheet.)",
            "", "SDI Direction: same grid with the", "game's SDI spark (screen 17).", "Airdodge and Shield Tilt: arrows",
            "(Shield Tilt: one chip only)."]),
    "sdi": ("SDI Direction", SDI_SELECTED, "sdi", "In",
            ["In / Out mirror with the CPU's facing;", "Left / Right never mirror. Nothing", "selected, or SDI Strength None:",
             "the CPU does not SDI.",
             "", "Chip: the game's own SDI spark on", "the CPU's head, turned to each", "direction as the game turns it to",
             "the SDI vector. Read from the game's", "files when the grid opens.", "(This mock: a render from the game's",
             "effect file; captures on the sheet.)"]),
}


def render_direction_grid(which="di"):
    title, selected, visual, cursor, legend = DIR_GRIDS[which]
    c = Canvas()
    frame(c, ["CPU"], 0, "A adds one weight to the chip (random pick is by weight), Y clears it, ZL all / none, B back.",
          f"CPU  >  Defence  >  {title}")
    s, pitch = 108, 132
    gx, gy = 150, 338  # centre column x, centre row y
    c.text(gx, 128, "In (toward the opponent)", "xs", MUTED, "m")
    c.text(gx + 2 * pitch, 128, "Out (away)", "xs", MUTED, "m")
    for name, (dx, dy) in list(DIRS.items())[:9]:
        cx, cy = gx + (dx + 1) * pitch, gy + dy * pitch
        chip_direction(c, cx, cy, s, name, name in selected, selected.get(name, 0), cursor=(name == cursor), visual=visual)
        c.text(cx, cy + s / 2 + 5, name, "xs", FG if name in selected else MUTED, "m")
    ax = gx + 3 * pitch + 80
    c.text(ax, 128, "absolute", "xs", MUTED, "m")
    for name, cy in (("Left", gy - 72), ("Right", gy + 72)):
        chip_direction(c, ax, cy, s, name, absolute=True, visual=visual)
        c.text(ax, cy + s / 2 + 5, name, "xs", MUTED, "m")
    # legend
    lx = ax + 100
    c.rect(lx, 120, W - 48, H - 112, fill=HILITE, outline=LINE, radius=6)
    c.text(lx + 16, 132, "Selected (weight)", "h", FG)
    y = 168
    for name, wgt in selected.items():
        c.text(lx + 16, y, f"{name}", "t", FG)
        c.text(W - 64, y, f"x{wgt}", "t", OURS, "r")
        y += 30
    y += 10
    for line in legend:
        c.text(lx + 16, y, line, "xs", MUTED if line else FG)
        y += 22
    return c


def render_roster_grid(hats=False):
    c = Canvas()
    title = "Kirby Hats" if hats else "Character Cycles"
    frame(c, ["Fighters & Stages"], 0,
          "A adds to the P1 order, ZR: CPU order (blue), Y marks Solo Popo on Ice Climbers, X previews a form." if not hats
          else "A adds the hat to the P1 order, ZR: CPU order (blue). Kirby takes the next hat in this order each cycle.",
          f"Fighters & Stages  >  {'Extras' if hats else 'Player 1 and CPU'}  >  {title}")
    c.text(W - 60, 100, "editing: P1 order (red)   ZR: CPU order", "s", P1, "r")
    if hats:
        chip_placeholder(c, 620, 112, 40, "HATS")
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


def tile_item_full(c, x, y, w, h, index, name, cursor=False):
    """Handheld tile: the fork's 64 px icon and the full name on two lines (owner, then item)."""
    c.rect(x, y, x + w, y + h, fill=TILE_HI if cursor else TILE, outline=SEL if cursor else LINE, width=3 if cursor else 1, radius=6)
    c.image(f"captures/items/icon-{index:02}.png", x + 8, y + (h - 64) / 2, x + 72, y + (h + 64) / 2)
    who, what = name.split(": ", 1)
    c.text(x + 82, y + h / 2 - 20, who + ":", "xs", MUTED)
    c.text(x + 82, y + h / 2 + 2, what, "s", FG)


def render_item_grid(handheld=False):
    """Give item grid (N5). Docked: one page, 5 x 8 tiles (38 used), short names. Handheld: three pages of 4 x 4
    (16, 16, 6), full names; this is page 1."""
    c = Canvas()
    frame(c, ["Fighters & Stages"], 0,
          "A gives Player 1 the item now (one at a time)." + (" L/R pages." if handheld else ""),
          "Fighters & Stages  >  Extras  >  Give item")
    chip_placeholder(c, 620, 112, 40, "ITEMS")
    label = "page 1 of 3   (38 items)" if handheld else "page 1 of 1   (38 items)"
    c.text(W - 60, 100, label, "s", MUTED, "r")
    if handheld:
        tx = W - 60 - c.textlength(label, "s") - 90
        c.rect(tx, 98, tx + 78, 120, fill=HILITE, outline=ADV, radius=5)
        c.text(tx + 39, 101, "handheld", "xs", ADV, "m")
        tw, th, px, py = 280, 100, 292, 110
        x0, y0 = 48 + (W - 96 - 4 * px + 12) / 2, 136
        for i, name in enumerate(ITEMS[:16]):
            col, row = i % 4, i // 4
            tile_item_full(c, x0 + col * px, y0 + row * py, tw, th, i, name, cursor=(i == 8))
        c.text(x0, y0 + 4 * py + 2, "Handheld mode: 4 x 4 tiles with full names, three pages (16, 16, 6). Docked: one page, 5 x 8 (screen 11).",
               "xs", MUTED)
        return c
    tw, th, px, py = 228, 48, 236, 54
    x0, y0 = 48 + (W - 96 - 5 * px + 8) / 2, 136
    for i, name in enumerate(ITEMS):
        col, row = i % 5, i // 5
        tile_item(c, x0 + col * px, y0 + row * py, tw, th, i, name, cursor=(i == 8))
    c.text(x0, y0 + 8 * py + 2, "Docked: one page, 5 x 8 tiles (38 used), short names. Icons: the fork's item pictures, "
           "made from the game's own item files.", "xs", MUTED)
    return c


def delay_popup(c, x0, y0, w, active, ticked=(), rng=(0, 10)):
    """The Aerial Delay popup (N3) in one of its two states: active = "list" (the checklist holds ticks, the range is
    greyed) or "range" (a min-max range is set, the checklist is greyed). 31 values, 0-30 frames."""
    on_list = active == "list"
    c.rect(x0, y0, x0 + w, y0 + 410, fill=TILE, outline=LINE, radius=8)
    c.text(x0 + 16, y0 + 12, "Aerial Delay", "h", FG)
    c.text(x0 + w - 16, y0 + 15, "frames 0 - 30", "xs", MUTED, "r")
    # checklist
    ink = FG if on_list else MUTED
    c.text(x0 + 16, y0 + 50, "Checklist", "t", ink)
    c.text(x0 + w - 16, y0 + 53, f"{len(ticked)} ticked" if on_list else "none", "xs", ink, "r")
    cw = (w - 32) / 8
    for v in range(31):
        cx, cy = x0 + 16 + (v % 8) * cw, y0 + 84 + (v // 8) * 32
        c.rect(cx, cy, cx + 18, cy + 18, outline=ink if on_list else LINE, width=2)
        if v in ticked:
            c.rect(cx + 4, cy + 4, cx + 14, cy + 14, fill=SEL)
        c.text(cx + 26, cy, str(v), "s", ink)
    if not on_list:
        c.text(x0 + 16, y0 + 218, "clear the range to use the list", "xs", ADV)
    c.line([(x0 + 16, y0 + 246), (x0 + w - 16, y0 + 246)], LINE)
    # range
    ink = FG if not on_list else MUTED
    c.text(x0 + 16, y0 + 262, "Range (min - max)", "t", ink)
    c.text(x0 + w - 16, y0 + 265, f"{rng[0]} - {rng[1]} frames" if not on_list else "not set", "xs", ink, "r")
    sx0, sx1, sy = x0 + 40, x0 + w - 40, y0 + 330

    def at(v):
        return sx0 + (sx1 - sx0) * v / 30

    c.rect(sx0, sy - 3, sx1, sy + 3, fill=HILITE if on_list else (60, 66, 80), radius=3)
    if not on_list:
        c.rect(at(rng[0]), sy - 3, at(rng[1]), sy + 3, fill=SEL, radius=3)
        for v, tag in ((rng[0], "min"), (rng[1], "max")):
            c.circle(at(v), sy, 10, fill=FG, outline=SEL, width=2)
            c.text(at(v), sy - 34, f"{tag} {v}", "xs", FG, "m")
    else:
        for v, tag in ((0, "min"), (30, "max")):
            c.circle(at(v), sy, 10, fill=TILE, outline=LINE, width=2)
            c.text(at(v), sy - 34, tag, "xs", MUTED, "m")
        c.text(x0 + 16, y0 + 354, "clear the list to use a range", "xs", ADV)
    if not on_list:
        c.text(x0 + 16, y0 + 354, "any value from min to max, picked uniformly", "xs", MUTED)
    c.text(x0 + 16, y0 + 384, "A tick / move handle    Y clears the active one", "xxs", MUTED)


def render_delay_popup():
    c = Canvas()
    frame(c, ["CPU"], 0, "Tick values for a random set, or set a min-max range; Y clears the active one so the other can be used.",
          "CPU  >  Attacks & Movement  >  Aerial Delay")
    w = 570
    xa, xb = 48, W - 48 - w
    c.text(xa, 132, "(a) checklist active: 0, 5 and 10 ticked", "t", FG)
    c.text(xb, 132, "(b) range active: 0 - 10", "t", FG)
    delay_popup(c, xa, 164, w, "list", ticked=(0, 5, 10))
    delay_popup(c, xb, 164, w, "range", rng=(0, 10))
    return c


def tile_stage_dense(c, x, y, w, h, name, rank=None, form=None, cursor=False, n=None):
    """Docked tile (same geometry as the docked item grid): thumbnail left, short name, rotation badge, form tag. `n` is
    the tile number for a text placeholder (no capture exists for that stage in this mock)."""
    c.rect(x, y, x + w, y + h, fill=TILE_HI if cursor else TILE, outline=SEL if cursor else LINE, width=3 if cursor else 1, radius=6)
    if name == "ALL":
        c.image("captures/stages/all.png", x + 6, y + 4, x + 66, y + h - 4)
        c.text(x + 74, y + h / 2 - 8, "All", "xs", FG)
        return
    if n is not None:  # text placeholder: dashed picture box and a muted label
        c.rect(x + 6, y + 4, x + 66, y + h - 4, outline=LINE, radius=4)
        c.text(x + 36, y + h / 2 - 7, "text", "tiny", MUTED, "m")
        c.text(x + 74, y + h / 2 - 8, f"Stage {n}", "xs", MUTED)
        return
    if name == "Waiting Room":  # a text tile in the fork's grid as well
        c.rect(x + 6, y + 4, x + 66, y + h - 4, outline=LINE, radius=4)
        c.text(x + 36, y + h / 2 - 7, "WR", "tiny", FG, "m")
    else:
        c.image(f"captures/stages/{slug(name)}.png", x + 6, y + 4, x + 66, y + h - 4)
    room = 13 if form else 17  # short names: the form tag takes the right-hand end of the tile
    c.text(x + 74, y + h / 2 - 8, name if len(name) <= room else name[:room - 1] + "…", "xs", FG)
    if rank is not None:
        c.circle(x + 14, y + 12, 8, fill=P1)
        c.text(x + 14, y + 6, str(rank), "tiny", FG, "m")
    if form:
        c.rect(x + w - 36, y + 6, x + w - 6, y + 22, fill=HILITE, radius=4)
        c.text(x + w - 21, y + 7, form, "xxs", OURS, "m")


def render_stage_grid(handheld=False):
    """Lobby Stages grid (N5 extended to stages). Docked: 5 x 8 tiles per page, short names, three pages (40, 40, 38 of
    All + 117 = 118). Handheld: 4 x 4 per page as the fork, full names, eight pages. This is page 1 in both."""
    c = Canvas()
    frame(c, ["Fighters & Stages"], 0, "A adds the stage to the rotation order, Y cycles its form (Normal / Omega / Battlefield), L/R pages.",
          "Fighters & Stages  >  Stages  >  Lobby Stages")
    label = "page 1 of 8   (All + 117 stages)" if handheld else "page 1 of 3   (All + 117 stages)"
    c.text(W - 60, 100, label, "s", MUTED, "r")
    if handheld:
        tx = W - 60 - c.textlength(label, "s") - 90
        c.rect(tx, 98, tx + 78, 120, fill=HILITE, outline=ADV, radius=5)
        c.text(tx + 39, 101, "handheld", "xs", ADV, "m")
    else:
        tw, th, px, py = 228, 48, 236, 54
        x0, y0 = 48 + (W - 96 - 5 * px + 8) / 2, 136
        cells = ["ALL"] + STAGES  # 16 tiles with a capture; tiles 17-40 are text placeholders
        for i in range(40):
            col, row = i % 5, i // 5
            if i < len(cells):
                rank, form = STAGE_ORDER.get(cells[i], (None, None))
                tile_stage_dense(c, x0 + col * px, y0 + row * py, tw, th, cells[i], rank, form, cursor=(cells[i] == "Small Battlefield"))
            else:
                tile_stage_dense(c, x0 + col * px, y0 + row * py, tw, th, "", n=i + 1)
        c.text(x0, y0 + 8 * py + 2, "Cursor: Small Battlefield — 3rd in the rotation, Normal form   (Ω omega, BF battlefield form)", "xs", FG)
        c.text(x0, y0 + 8 * py + 22, "Docked: 5 x 8 per page, short names, pages of 40, 40 and 38. Tiles 17-40 are text placeholders: "
               "the mock has captures for the first 16 only.", "xs", MUTED)
        return c
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
    c.text(x0, y0 + 4 * py + 24, "Thumbnails: read at run time from each stage's own game UI texture, so a stage mod's "
           "picture shows; this mock shows stock captures.", "xs", MUTED)
    return c


def render_chip_sheet():
    """The placeholder chip at review size and row size (three uses), and the direction chip set."""
    c = Canvas()
    frame(c, ["Chips"], 0, "Placeholders: the final Kirby Hats, Solo Popo and item icons are open until the functionality is settled.",
          "Chips  >  review sheet")
    for i, (name, label, where) in enumerate([("Kirby Hats", "HATS", "Extras > Kirby Hats row, hat grid header"),
                                               ("Solo Popo", "POPO", "Player 1 and CPU > Solo Popo row"),
                                               ("Character items", "ITEMS", "Extras > Give item row, item grid header")]):
        x = 150 + i * 390
        c.rect(x - 90, 132, x + 90, 300, fill=TILE, outline=LINE, radius=10)
        chip_placeholder(c, x, 216, 150, label)
        c.rect(x + 110, 176, x + 170, 236, fill=TILE, outline=LINE, radius=6)
        chip_placeholder(c, x + 140, 206, 34, label)
        c.text(x + 140, 244, "row size", "tiny", MUTED, "m")
        c.text(x, 308, name, "h", FG, "m")
        c.text(x, 332, where, "tiny", MUTED, "m")
    names = ["Up In", "Up", "Up Out", "In", "Neutral", "Out", "Down In", "Down", "Down Out", "Left", "Right"]
    c.text(48, 352, "DI Direction: render of the game's DI effect, turned as the game turns it to the launch angle.   Check: capture", "xs", FG)
    for i, name in enumerate(names):
        x = 84 + i * 88
        chip_direction(c, x, 406, 60, name, selected=(name in DI_SELECTED), weight=DI_SELECTED.get(name, 0),
                       absolute=name in ("Left", "Right"), visual="di")
        c.text(x, 440, name, "tiny", MUTED, "m")
    chip_direction(c, 1150, 406, 60, "Up Out", selected=True, visual="di-capture")  # the captured streak, launch ~45°
    c.text(1150, 440, "captured (launch ~45°)", "tiny", FG, "m")
    c.text(48, 458, "SDI Direction: render of the game's SDI spark on the head, turned as the game turns it to the SDI vector.", "xs", FG)
    for i, name in enumerate(names):
        x = 84 + i * 88
        chip_direction(c, x, 512, 60, name, selected=(name in SDI_SELECTED), weight=SDI_SELECTED.get(name, 0),
                       absolute=name in ("Left", "Right"), visual="sdi")
        c.text(x, 546, name, "tiny", MUTED, "m")
    chip_direction(c, 1080, 512, 60, "Up", selected=True, visual="sdi-capture")  # the game's own Up, for comparison
    c.text(1080, 546, "Up, captured", "tiny", FG, "m")
    chip_direction(c, 1170, 512, 60, "In", selected=True, visual="sdi-check")  # the game's own In, for comparison
    c.text(1170, 546, "In, captured", "tiny", FG, "m")
    c.text(48, 564, "Airdodge Direction and Shield Tilt: the clean room's arrows; Neutral is a ring; Left/Right carry 'abs'.", "xs", FG)
    for i, name in enumerate(names):
        x = 84 + i * 88
        chip_direction(c, x, 600, 40, name, absolute=name in ("Left", "Right"))
    return c


def render_sdi_stretch_compare():
    """Review sheet (proposal v10): the SDI Up and In chips at the two real chip sizes (60 and 30 px) for the v9 render and
    four stretches along the SDI vector, beside the captured Up and In spark crops at the same sizes."""
    c = Canvas()
    frame(c, ["Chips"], 0, "Pick the stretch whose chip-size look is closest to the captured spark (right column).",
          "Chips  >  SDI spark: stretch along the SDI vector")
    cols = [("v9  x1.0", "sdi-v9", "sdi-v9"), ("x1.17", "sdi@1.17", "sdi@1.17"), ("x1.4", "sdi@1.4", "sdi@1.4"),
            ("x1.7", "sdi@1.7", "sdi@1.7"), ("x2.0", "sdi@2.0", "sdi@2.0"), ("captured", "sdi-capture", "sdi-check")]
    pitch = (W - 96) / len(cols)
    chosen = json.loads((CAPS / "crops.json").read_text(encoding="utf-8"))["sdi_stretch"]["factor"]
    c.text(40, 222, "60 px", "xs", MUTED)
    c.text(40, 330, "30 px", "xs", MUTED)
    for i, (label, up_v, in_v) in enumerate(cols):
        cx = 48 + pitch * (i + 0.5) + 20
        if label == f"x{chosen}":  # the stretch the SDI chips draw
            c.rect(cx - 92, 140, cx + 92, 372, outline=OURS, width=2, radius=8)
            c.text(cx, 354, "drawn on 01, 16, 17", "xs", OURS, "m")
        c.text(cx, 150, label, "h", FG, "m")
        for row, (name, vis) in enumerate((("Up", up_v), ("In", in_v))):
            x = cx - 46 + row * 92
            chip_direction(c, x, 240, 60, name, selected=True, visual=vis)
            chip_direction(c, x, 322, 30, name, selected=True, visual=vis)
            c.text(x, 276, name, "tiny", MUTED, "m")
            c.text(x, 342, name, "tiny", MUTED, "m")
    notes = ["How: the burst is stretched along the SDI vector, its width unchanged. x1.17 is the emitter's own scale-key table at life 0.5;",
             "x1.4, x1.7 and x2.0 multiply the life-0.3 scale (calibrated against the captures; the file's keys end at x1.42 with the burst faded).",
             "Captured: crops of real frames, SDI Up (sdiup f+12) and SDI In (sdiin f+12), drawn at the same chip size; same chip scale in every column.",
             "Burst tip from head, share of chip: captured Up 0.36, captured In 0.31 (its detached streaks go past 0.4); render 0.19 x factor."]
    for k, t in enumerate(notes):
        c.text(48, 410 + 22 * k, t, "xs", MUTED)
    return c


def image_sources():
    """index.md section: every source capture (path, sha256) and every crop box, from captures/crops.json."""
    m = json.loads((CAPS / "crops.json").read_text(encoding="utf-8"))
    out = ["", "## Image sources (review mockup only, not shipped artwork)", "",
           "The crops in `captures/` are game-asset captures (stock fighter icons, stock stage thumbnails) as the fork's own grid "
           "pickers show them in Training; `captures/items/` holds byte copies of the fork's item icons. They exist for this "
           "review mockup only; the plugin never ships these files. Written by "
           "`tools/crop_menu_captures.py` (pins each source by sha256); the manifest is `captures/crops.json`. Eden captures are "
           "1920x1080 window grabs with the game frame at (361,181) scaled 1198x674; the two `docs/feature-walkthroughs/` images "
           "are the fork's own 1280x720 frames (byte-identical in the fork worktree). Upstream's menu icons are not used (PROVENANCE §C).", "",
           "| source | file | sha256 |", "| --- | --- | --- |"]
    for k, s in m["sources"].items():
        out.append(f"| {k} | `{s['path']}` | `{s['sha256']}` |")
    out += ["", "Crop boxes (x0, y0, x1, y1 in source pixels). Portraits come from `change-player` except cells that carry the "
            "cursor or an order badge there (Fox, Sora: `change-cpu`) and the ALL cell (`kirby-grid`); Waiting Room has no "
            "picture in the fork's stage grid and none here.", "", "| cell | crop | source | box |", "| --- | --- | --- | --- |"]
    for cell in m["cells"]:
        out.append(f"| {cell['cell']} | `captures/{cell['file']}` | {cell['source']} | {tuple(cell['box'])} |")
    it = m["item_icons"]
    out += ["", "Item icons (proposal v5): byte copies of the fork's own item-grid pictures, not crops. Source directory "
            f"`{it['path']}` (the frozen M5k stage; byte-identical to the fork worktree's `scripts/item-icons/assets/`), "
            f"manifest `sources.json` sha256 `{it['manifest_sha256']}`. Each icon's sha256 is checked against that manifest "
            "before the copy. The manifest records how the fork made each one: native `item_0` UI icons for Mr. Saturn, "
            "Bob-omb and Banana Peel, renders of the item's own game model and textures for the rest.", "",
            "| # | item | file | sha256 |", "| --- | --- | --- | --- |"]
    for item in m["items"]:
        out.append(f"| {item['index']} | {item['cell']} | `captures/{item['file']}` | `{item['sha256']}` |")
    d = m["di_streak"]
    r9 = m["w9_renders"]
    out += ["", "DI and SDI chip renders (proposal v9): the DI and SDI chips on 01, 09, 16 and 17 draw game-derived renders made "
            "by the c1162 W9 extraction (`c1162/di-sdi-extract/RESULT.md` §4): each effect's own emitter textures, meshes and "
            "colours read from the stock 13.0.5 `effect/system/common/ef_common.eff` (DI `P_CmnVector`, SDI `P_CmnHitStopSpark`), "
            "composited as one still at particle life t = 0.3 with the effect origin on the chip centre and its reach at 0.46 x "
            "the chip size. Only the two Up layers (432 px RGBA light, alpha = max channel) are byte copies here; the renderer "
            "turns each to every direction, as the game turns its one effect to the launch angle / SDI vector. Field meanings "
            "behind the render are W9 inference, and one SDI emitter (`spark1`) is placed by calibration against the capture "
            "(RESULT.md §4.8, §7). The clean room renders the same from the player's own file at run time; nothing from this "
            "folder ships. No `.eff`, texture, mesh or parameter file is in the repository. The captures below stay as the "
            f"visual check (chip sheet 16). W9 manifest `{r9['path']}/manifest.json` sha256 `{r9['manifest_sha256']}`.", "",
            "| render | file | source | sha256 |", "| --- | --- | --- | --- |"]
    for f in r9["files"]:
        out.append(f"| {'DI' if '/di-' in f['file'] else 'SDI'} Up ({f['size'][0]}x{f['size'][1]}) | `captures/{f['file']}` | "
                   f"`{f['source']}` | `{f['sha256']}` |")
    st = m["sdi_stretch"]
    st = m["sdi_stretch"]
    out += ["", "SDI burst, longer (proposal v10): the SDI chips (01, 16, 17 and the decision pictures 17 and 18) draw `captures/" + st["file"] + "`"
            f" (the burst stretched x{st['factor']} along the SDI vector), not W9's own `sdi-up-4x-layer.png` (kept above, unused). "
            "Measured on the SDI Up captures (`c1162/captures-ally-sdi/sdi-up`, frames +10 to +15; the largest connected pixels with "
            "min(R,G,B) >= 240, bounding box): height / width 1.76, 1.47, 1.78, 1.84, 1.65, 1.57, median 1.71 (frames +16 and +17 are "
            "merged with the fighters and left out). W9's render, the solid burst (alpha >= 230, crown and legs, the thin line excluded): "
            "58 x 86 px, height / width 1.48. The first try (x1.17, height / width 1.72, the captured median) was not visible at chip "
            "size, because the capture's burst is bolder and reaches much farther from the head in chip terms: measured against the "
            "captured crops at the chip scale of sheet 16 (reach 0.40 of the chip), the captured Up burst tip is 0.36 of the chip from "
            "the head and the captured In burst tip 0.31 with detached streaks going on past 0.40, while W9's burst tip is 0.19. Sheet 21 "
            "shows the Up and In chips at 60 and 30 px for x1.0 (v9), x1.17, x1.4, x1.7 and x2.0 beside the captured crops; x2.0 (tip 0.39) "
            "is the closest in reach and in weight, x1.7 (tip 0.32) falls short of the Up capture and of the In streaks, so x2.0 is drawn. "
            "Remaining difference: the render's burst keeps its width (about 13 % of the chip against about 29 % in the capture), so it "
            "is a taller, thinner flame than the captured arch. The thin line (`spark1`, W9's calibrated quad) is unchanged. "
            "Method, the root route as far as the file goes: W9's own renderer (`di-sdi-extract/tools/render_chips.py`, imported "
            "read-only) is run by `tools/render_sdi_stretch.py`, which changes only the burst emitter `impactring1`'s scale-Y. x1.17 reads "
            "its own scale-key table at particle life 0.5 instead of 0.3 (y key 1.356 -> 1.584, alpha still 1.0). That table ends at "
            "y = 1.93 (x1.42) at life 1.0, where the burst has faded to alpha 0.14, so x1.4, x1.7 and x2.0 are not in the file: they "
            "multiply the life-0.3 scale-Y by the factor, CALIBRATED against the captures, like W9's `spark1` quad. The scale-X, the "
            "origin, the colours, the gain, the framing and the other emitter are unchanged. The tool self-checks that factor 1 "
            "reproduces W9's file pixel for pixel. It needs numpy, so it runs under the ssbu-recomp venv Python "
            "(`tools/crop_menu_captures.py` calls it); the inputs are pinned by sha256 below. No `.eff`, texture, mesh or parameter "
            "file is in the repository.", "",
            "| field | value |", "| --- | --- |",
            f"| file drawn | `captures/{st['file']}` ({st['size'][0]}x{st['size'][1]}, RGBA light layer), factor {st['factor']} ({st['how']}); same pixels as `captures/{st['same_pixels_as']}` |",
            f"| sha256 | `{st['sha256']}` |",
            f"| tool | `{st['tool']}`; W9 `tools/render_chips.py` sha256 `{st['tool_sha256']}` |",
            f"| inputs | stock 13.0.5 `ef_common.eff` sha256 `{st['eff_sha256']}`; W9 `sdi-up-4x-layer.png` sha256 `{st['base_render_sha256']}` |"]
    out += ["", "Compared stretches (sheet 21, review material; the file drawn is the x" + str(st["factor"]) + " one):", "",
            "| factor | file | source | sha256 |", "| --- | --- | --- | --- |"]
    for v in st["variants"]:
        out.append(f"| x{v['factor']} | `captures/{v['file']}` | {v['source']} | `{v['sha256']}` |")
    out += ["", "DI streak (proposal v8; the visual check since v9): the game's own DI visual (system effect `sys_vector`), cut "
            "from one frame of a real capture. v8 drew the DI chips from this crop; since v9 the chip sheet (16) shows it once, as "
            "captured, beside the rendered DI chips. The game draws one effect and turns it about Z to the DI-corrected launch "
            "angle (SPEC-core-07 §4.6, from the static "
            "trace `c1162/di-sdi-trace/RESULT.md` §3.1). The clean room reads the effect from the player's game files at run time "
            "and never ships a picture of it. Capture: `c1162/captures-ally` run A (Ally Eden, fork M5l, Training on Final "
            "Destination, Steve forward smash on CPU Sora, DI Direction = Up, Hitbox Visualization off), Eden's GIF recorder, "
            "1280x720 game frame, 256-colour palette; frame +13 after first contact (the streak shows from about +5 to +15). The "
            "strip: rotate the frame about the centre by the streak's measured axis so the streak points along +x, cut along and "
            "across it, feather the strip's edges in alpha (smoothstep; pixels unchanged). The frame's background and the CPU's "
            "keyblade show through.", "",
            "| field | value |", "| --- | --- |",
            f"| crop | `captures/{d['file']}` ({d['size'][0]}x{d['size'][1]}) |",
            f"| source | `{d['source']}` |",
            f"| sha256 | `{d['sha256']}` |",
            f"| centre, axis | {tuple(d['centre'])}, {d['angle_deg']}° counter-clockwise from +x |",
            f"| along, across | {tuple(d['along'])} px along the streak, ±{d['half_width']} px across; feather {tuple(d['feather'])} |",
            f"| crop box in the source | corners {', '.join(str(tuple(p)) for p in d['corners_in_source'])}; bounding box {tuple(d['bbox_in_source'])} |",
            f"| rotation centre in the crop | {tuple(d['pivot_in_crop'])} px; on a chip the strip's tail end sits on the chip "
            "centre and the streak runs toward the direction |"]
    out += ["", "SDI spark (proposal v9, the visual check): the game's own SDI visual (system effect `sys_hitstop_spark`, a white "
            "spiky burst with a ring outline drawn on the CPU's head and pointing along the SDI vector), cut from one frame of a "
            "real capture. The chip sheet (16) shows it as captured, with its ring centre (the head) on the chip centre, beside "
            "the rendered SDI chips. The clean room reads the effect from the player's game files at run time and never ships a "
            "picture of it. Capture: `c1162/captures-ally-sdi` (W8; Ally Eden, fork M5l, "
            "Training on stock Final Destination, Steve forward smash on CPU Fox at 100 %, SDI Direction Up, Strength High, DI "
            "none, Hitbox Visualization off), the same recorder and frame as the DI capture; offset 0 is the first frame of the "
            "hit flash; the spark shows from about +6 to +17 and is absent in the no-SDI control (`sdi-none/`). The strip is cut as "
            "the DI streak; its alpha also carries a whiteness key (smoothstep of min(R,G,B) between the two key values), because "
            "the spark is pure white and the stage, the smoke and both fighters sit behind it; pixels are unchanged. A second crop "
            "from the SDI In run (the CPU stood right of the player, so In points left) is shown once on the chip sheet, as "
            "captured, beside the rendered In chip: a visual check of the rotation.", "",
            "| field | SDI Up (16, check) | SDI In (16, check) |", "| --- | --- | --- |"]
    s, k = m["sdi_spark"], m["sdi_spark_check"]
    rows = [("crop", lambda r: f"`captures/{r['file']}` ({r['size'][0]}x{r['size'][1]})"),
            ("source", lambda r: f"`{r['source']}`"),
            ("sha256", lambda r: f"`{r['sha256']}`"),
            ("centre, axis", lambda r: f"{tuple(r['centre'])}, {r['angle_deg']}° counter-clockwise from +x"),
            ("along, across", lambda r: f"{tuple(r['along'])} px along, ±{r['half_width']} px across; feather {tuple(r['feather'])}"),
            ("whiteness key", lambda r: f"{tuple(r['whiteness_key'])} on min(R,G,B)"),
            ("crop box in the source", lambda r: f"corners {', '.join(str(tuple(p)) for p in r['corners_in_source'])}; "
                                                 f"bounding box {tuple(r['bbox_in_source'])}"),
            ("rotation centre in the crop", lambda r: f"{tuple(r['pivot_in_crop'])} px (the ring centre, on the chip centre)")]
    out += [f"| {label} | {fn(s)} | {fn(k)} |" for label, fn in rows]
    return out


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for old in OUT.glob("*.png"):
        old.unlink()
    for old in OUT.glob("*.svg"):
        old.unlink()
    index = ["# Menu mockup screens (proposal v10, in-game look, 1280x720)", "",
             "Rendered by `tools/render_menu_mockup.py` from MENU-DRAFT.md §2. Pages show the group that best shows the new controls; "
             "the help bar shows the focused item's text. The portrait grid uses the real fighter icons, cropped from captures of the "
             "fork's own grid pickers. The item grid uses the fork's own 38 item icons, copied unchanged. The stage grid shows "
             "captures of the stock thumbnails; the clean room does not bake them but reads each stage's own game UI texture at run "
             "time, so a stage mod's picture shows (SPEC-core-07 §4.5). Sources are in the tables below. The Kirby Hats, Solo Popo and "
             "character-item chips are a neutral placeholder: their final icons are open and will be designed once the functionality "
             "is settled. The DI and SDI Direction chips show the game's own DI streak and SDI spark as renders made from the "
             "game's effect file (one still per effect, turned to each direction as the game turns that effect to the launch "
             "angle / SDI vector; SPEC-core-07 §4.6); the chip sheet shows crops of real captures beside them as the visual "
             "check. The Airdodge Direction and Shield Tilt "
             "arrows are the clean room's own vector art. The screens carry no origin tags "
             "(upstream / ours, proposal v6): origin is internal, recorded in `PROVENANCE.md` and the provenance table "
             "in `docs/MENU-PROPOSAL.md`.", "",
             "Version 10 (2026-11-01): the SDI chips draw the spark's burst x2.0 longer along the SDI vector (screens 01, 16, 17; the stretches are compared on 21; method "
             "and measurements below, `DECISIONS.md` N10); the decided items are drawn (`DECISIONS.md` N1-N7): 02 Respawn "
             "invincibility added and Item on load gone, 03 the advanced row Settings override playback, 05 editable menu shortcuts, "
             "11 and 18 the item grid docked (5x8, one page) and handheld (4x4, three pages), 19 the delay popup with checklist and "
             "range, 20 the hidden Developer page. Preload Samples left the Loading group (04, 14 and 15 now count 6 rows there).", ""]
    screens = [
        ("01-cpu", lambda: render_page(0, 1), "page CPU (Defence group: direction grids; the DI and SDI rows preview their "
         "selected chips)"),
        ("02-saves", lambda: render_page(1, 0, 5), "page Saves (Save states group; Respawn invincibility focused, Item on load gone)"),
        ("03-recording", lambda: render_page(2, 1, 5), "page Recording (Playback group; the advanced row Settings override playback focused)"),
        ("04-fighters-stages", lambda: render_page(3), "page Fighters & Stages (Player 1 and CPU: portrait pickers, cycle preview, Solo Popo placeholder)"),
        ("05-display-controls", lambda: render_page(4, 1, 2), "page Display & Controls (Controls group; Menu shortcut 1 / 2 are editor rows, Permanent shortcut stays info)"),
        ("06-situation-overrides", lambda: render_subscreen(0), "sub-screen Situation overrides (CPU > Reactions)"),
        ("07-button-chords", lambda: render_subscreen(1), "sub-screen Button chords (Display & Controls > Controls)"),
        ("08-swap-groups", lambda: render_subscreen(2), "sub-screen Swap groups (Fighters & Stages > Loading)"),
        ("09-direction-grid", render_direction_grid, "sub-screen DI Direction: the 3x3 direction grid of DI streak chips with "
         "absolute Left/Right and weights"),
        ("10-character-grid", lambda: render_roster_grid(False), "sub-screen Character Cycles: the fork's 13x7 portrait grid with P1 (red) and CPU (blue) order badges"),
        ("11-item-grid", render_item_grid, "sub-screen Give item, docked: one page of 5x8 tiles (38 used) with short names and the fork's own item icons, page 1 of 1"),
        ("12-stage-grid", render_stage_grid, "sub-screen Lobby Stages, docked: 5x8 tiles per page (the docked item grid's geometry), All first, "
         "order badges and form tags, short names, page 1 of 3 (40, 40, 38 of All + 117); tiles 17-40 are text placeholders (the mock has "
         "captures for the first 16 only); at run time each thumbnail is the stage's own game UI texture (a stage mod's picture shows), here stock captures"),
        ("13-kirby-hats-grid", lambda: render_roster_grid(True), "sub-screen Kirby Hats: the same portrait grid as a two-role hat order"),
        ("14-fighters-extras", lambda: render_page(3, 1), "page Fighters & Stages (Extras group: Kirby Hats and Give item placeholders, Mii lists)"),
        ("15-fighters-stage-rows", lambda: render_page(3, 2), "page Fighters & Stages (Stages group: stage order preview)"),
        ("16-chip-sheet", render_chip_sheet, "review sheet: the placeholder chip at full and row size for its three uses, the rendered DI and "
         "SDI chips with crops of the captures beside them as the check, and the Airdodge / Shield Tilt arrows"),
        ("17-sdi-direction-grid", lambda: render_direction_grid("sdi"), "sub-screen SDI Direction: the 3x3 direction grid of SDI "
         "spark chips with absolute Left/Right and weights (same layout as 09)"),
        ("18-item-grid-handheld", lambda: render_item_grid(True), "sub-screen Give item, handheld: 4x4 tiles with full names, page 1 of 3, "
         "with a small handheld tag"),
        ("19-delay-popup", render_delay_popup, "popup Aerial Delay with both controls, two states: (a) checklist active (0, 5, 10 ticked), "
         "range greyed; (b) range 0 - 10 active, checklist greyed; Y clears the active one"),
        ("20-developer", lambda: render_page(0, dev=True), "sixth page Developer, hidden unless developer.ini has developer_page=on: "
         "Preload Samples (moved from Loading), Reload developer config, Debug hooks"),
        ("21-sdi-stretch-compare", render_sdi_stretch_compare, "review sheet: SDI Up and In chips at 60 and 30 px for the v9 render "
         "and four stretches along the SDI vector (x1.17, x1.4, x1.7, x2.0), beside the captured Up and In crops at the same size"),
        ("22-stage-grid-handheld", lambda: render_stage_grid(True), "sub-screen Lobby Stages, handheld: 4x4 tiles per page as the fork, full "
         "names, All, order badges and form tags, page 1 of 8, with a small handheld tag"),
    ]
    for name, fn, what in screens:
        fn().save(name)
        index.append(f"- `{name}.png` / `.svg` — {what}")
    index += image_sources()
    (OUT / "index.md").write_text("\n".join(index) + "\n", encoding="utf-8")
    print(f"wrote {2 * len(screens)} files to {OUT}")


if __name__ == "__main__":
    main()
