#!/usr/bin/python3 -I
"""Render the proposed TrainingRooms menu (MENU-DRAFT.md §2, proposal v6) as PNG and SVG screens in the
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
internal record and is never drawn.

Every screen is drawn once through `Canvas`, which records the primitives and writes both the PNG
(Pillow) and a text SVG (images referenced by relative path), so the two never drift.

Output: docs/menu-mockup/NN-<screen>.png and .svg plus index.md. Pillow only; DejaVu Sans Mono for the
modpack-style monospace text.
"""
import json
import math
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


def chip_placeholder(c, cx, cy, s, label):
    """A dashed rounded square with a short label; below 60 px (a menu row) the label is dropped."""
    half = s / 2
    c.rect(cx - half, cy - half, cx + half, cy + half, fill=TILE, radius=s * 0.16)
    dash = (max(3, s / 12), max(2, s / 18))
    c.line(rounded_path(cx - half, cy - half, cx + half, cy + half, s * 0.16), MUTED, max(1, round(s / 40)) + 1, dash=dash)
    if s >= 60:
        c.text(cx, cy - s * 0.16, label, "b" if s < 120 else "h", FG, "m")
        c.text(cx, cy + s * 0.08, "icon TBD", "tiny" if s < 120 else "xs", MUTED, "m")
    else:
        c.text(cx, cy - 6, "?", "b", MUTED, "m")


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
    for ii, (label, kind, value, _origin) in enumerate(items):  # _origin: internal only, never drawn (v6)
        if ii == 0:
            c.rect(404, y - 6, W - 48, y + 26, fill=HILITE)
            c.rect(404, y - 6, 408, y + 26, fill=SEL)
        c.text(418, y, label, "t", FG)
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


def render_item_grid():
    c = Canvas()
    frame(c, ["Fighters & Stages"], 0, "A gives Player 1 the item now (one at a time). L/R pages.",
          "Fighters & Stages  >  Extras  >  Give item")
    chip_placeholder(c, 620, 112, 40, "ITEMS")
    c.text(W - 60, 100, "page 1 / 2   (38 items)", "s", MUTED, "r")
    tw, th, px, py = 280, 48, 292, 54
    x0, y0 = 48 + (W - 96 - 4 * px + 12) / 2, 136
    for i, name in enumerate(ITEMS[:32]):
        col, row = i % 4, i // 4
        tile_item(c, x0 + col * px, y0 + row * py, tw, th, i, name, cursor=(i == 8))
    c.text(x0, y0 + 8 * py + 2, "Icons: the fork's 38 item pictures (scripts/item-icons), made from the game's own item files.",
           "xs", MUTED)
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
        c.rect(x - 110, 140, x + 110, 360, fill=TILE, outline=LINE, radius=10)
        chip_placeholder(c, x, 250, 190, label)
        c.rect(x + 130, 190, x + 190, 250, fill=TILE, outline=LINE, radius=6)
        chip_placeholder(c, x + 160, 220, 34, label)
        c.text(x + 160, 258, "row size", "tiny", MUTED, "m")
        c.text(x, 372, name, "h", FG, "m")
        c.text(x, 398, where, "tiny", MUTED, "m")
    c.text(48, 440, "Direction chips (DI, SDI, Airdodge, Shield Tilt): a bold stroke from the centre with an arrowhead; Neutral is a ring; Left/Right carry 'abs'.", "xs", FG)
    for i, name in enumerate(["Up In", "Up", "Up Out", "In", "Neutral", "Out", "Down In", "Down", "Down Out", "Left", "Right"]):
        x = 90 + i * 100
        chip_direction(c, x, 520, 72, name, selected=(name in DI_SELECTED), weight=DI_SELECTED.get(name, 0), absolute=name in ("Left", "Right"))
        c.text(x, 562, name, "tiny", MUTED, "m")
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
    return out


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for old in OUT.glob("*.png"):
        old.unlink()
    for old in OUT.glob("*.svg"):
        old.unlink()
    index = ["# Menu mockup screens (proposal v6, in-game look, 1280x720)", "",
             "Rendered by `tools/render_menu_mockup.py` from MENU-DRAFT.md §2. Pages show the group that best shows the new controls; "
             "the help bar shows the focused item's text. The portrait grid uses the real fighter icons, cropped from captures of the "
             "fork's own grid pickers. The item grid uses the fork's own 38 item icons, copied unchanged. The stage grid shows "
             "captures of the stock thumbnails; the clean room does not bake them but reads each stage's own game UI texture at run "
             "time, so a stage mod's picture shows (SPEC-core-07 §4.5). Sources are in the tables below. The Kirby Hats, Solo Popo and "
             "character-item chips are a neutral placeholder: their final icons are open and will be designed once the functionality "
             "is settled. The direction glyphs are the clean room's own vector art. The screens carry no origin tags "
             "(upstream / ours, proposal v6): origin is internal, recorded in `PROVENANCE.md` and the provenance table "
             "in `docs/MENU-PROPOSAL.md`.", ""]
    screens = [
        ("01-cpu", lambda: render_page(0, 1), "page CPU (Defence group: direction grids)"),
        ("02-saves", lambda: render_page(1), "page Saves"),
        ("03-recording", lambda: render_page(2), "page Recording"),
        ("04-fighters-stages", lambda: render_page(3), "page Fighters & Stages (Player 1 and CPU: portrait pickers, cycle preview, Solo Popo placeholder)"),
        ("05-display-controls", lambda: render_page(4), "page Display & Controls"),
        ("06-situation-overrides", lambda: render_subscreen(0), "sub-screen Situation overrides (CPU > Reactions)"),
        ("07-button-chords", lambda: render_subscreen(1), "sub-screen Button chords (Display & Controls > Controls)"),
        ("08-swap-groups", lambda: render_subscreen(2), "sub-screen Swap groups (Fighters & Stages > Loading)"),
        ("09-direction-grid", render_direction_grid, "sub-screen DI Direction: the 3x3 direction grid with absolute Left/Right and weights"),
        ("10-character-grid", lambda: render_roster_grid(False), "sub-screen Character Cycles: the fork's 13x7 portrait grid with P1 (red) and CPU (blue) order badges"),
        ("11-item-grid", render_item_grid, "sub-screen Give item: 4x8 item grid with the fork's own item icons, page 1 of 2"),
        ("12-stage-grid", render_stage_grid, "sub-screen Lobby Stages: 4x4 stage page with All, order badges and form tags; at run time "
         "each thumbnail is the stage's own game UI texture (a stage mod's picture shows), here stock captures"),
        ("13-kirby-hats-grid", lambda: render_roster_grid(True), "sub-screen Kirby Hats: the same portrait grid as a two-role hat order"),
        ("14-fighters-extras", lambda: render_page(3, 1), "page Fighters & Stages (Extras group: Kirby Hats and Give item placeholders, Mii lists)"),
        ("15-fighters-stage-rows", lambda: render_page(3, 2), "page Fighters & Stages (Stages group: stage order preview)"),
        ("16-chip-sheet", render_chip_sheet, "review sheet: the placeholder chip at full and row size for its three uses, and the direction chip set"),
    ]
    for name, fn, what in screens:
        fn().save(name)
        index.append(f"- `{name}.png` / `.svg` — {what}")
    index += image_sources()
    (OUT / "index.md").write_text("\n".join(index) + "\n", encoding="utf-8")
    print(f"wrote {2 * len(screens)} files to {OUT}")


if __name__ == "__main__":
    main()
