#!/usr/bin/env python3
"""The recording-redesign mockups, revisions 3 to 8.

Menu screens are redrawn on the real rc.1 Recording page (the COMBO-HELPER method).
The pop-up, the chain, and combo-from-macro sit on a real Eden Training frame from
artifacts/phase23. The Switch keyboard is a labelled schematic, not a custom keyboard.
The loop, the floppy (save), the link and the numbered-list marks are drawn from the
Material Symbols SVGs in docs/decisions/icons (Apache-2.0; see SOURCE.txt there).

Run from the repo root:
  /usr/bin/python3 -I tools/render_recording_redesign.py          (every picture the doc uses, except --r3's)
  /usr/bin/python3 -I tools/render_recording_redesign.py --q52    (only the question pictures, RECORDING-REDESIGN-qN-*.png)
  /usr/bin/python3 -I tools/render_recording_redesign.py --r3     (revision 3's pictures)
"""
import os
import sys

from PIL import Image, ImageDraw

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import render_combo_helper as ch  # noqa: E402
import render_input_viz as iv  # noqa: E402

W, H = iv.W, iv.H
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "docs", "decisions")
MENU = "/home/intpa/eden-runs/tr-p23-docs-capture-03/out/c15-q7/b-021632Z-877073-q7-row.png"
GAME = "/home/intpa/smash-training-lobby/artifacts/phase23/overlay-investigation/frames/fix-tr-training.png"

GREEN = (118, 226, 172, 255)
GREEN_ROW = (16, 78, 52, 255)
GOLD = (255, 206, 84, 255)
WHITE = (255, 255, 255, 255)
DARK = (16, 18, 22, 255)
PANEL = (13, 13, 13, 236)
SMALL = iv.font(13)
TINY = iv.font(12)

RED = (232, 72, 72, 255)


def save(img, name):
    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, name + ".png")
    img.convert("RGB").save(path, optimize=True)
    print(path)


def hints(d, line):
    d.rectangle((44, 660, 1246, 698), fill=ch.M_PANEL)
    iv.text(d, (53, 678), line, iv.font(14), fill=ch.M_GROUP, anchor="lm")


def file_screen(bg, crumb, help1, help2, hint):
    """A file's own screen: full width, a breadcrumb, no group column (as the r8 action screens). Crop and Rename
    open from here."""
    img = bg.copy()
    d = ImageDraw.Draw(img)
    ch.clear_content(d, list_view=True)
    iv.text(d, (53, 118), crumb, ch.MF, fill=ch.M_TEXT, anchor="lm")
    ch.help_lines(d, help1, help2)
    hints(d, hint)
    return img, d


def crop_links(d, rows, kept, cursor, y0=192):
    """A combo's crop screen, by links. The kept range is a bracket at the left labelled "kept" (the thin green
    outline means the hot five everywhere else), and the links left out are dimmed. rows: (name, range, end)."""
    x0, x1, nx, c2x, c3x = 110, 1236, 140, 760, 1040
    iv.text(d, (nx, 156), "Link", SMALL, fill=ch.M_GROUP, anchor="lm")
    iv.text(d, (c2x, 156), "Range", SMALL, fill=ch.M_GROUP, anchor="lm")
    centres = []
    for i, (name, rng, end) in enumerate(rows):
        y = y0 + i * 36
        top, bot = y - 16, y + 16
        if i == cursor:
            d.rectangle((x0, top, x1, bot), fill=ch.M_FOCUS)
            d.rectangle((x0, top, x1, bot), outline=WHITE, width=2)
            d.rectangle((x0, top, x0 + 5, bot), fill=WHITE)
        ink = ch.M_TEXT if i < kept else DIM
        iv.text(d, (nx, y), name, ch.MF, fill=ink, anchor="lm")
        iv.text(d, (c2x, y), rng, ch.MF, fill=ch.M_GROUP if i < kept else DIM, anchor="lm")
        iv.text(d, (c3x, y), end, ch.MF, fill=ch.M_GROUP, anchor="lm")
        centres.append(y)
    top, bot = centres[0] - 14, centres[kept - 1] + 14
    bx = x0 - 14
    d.line([(bx + 8, top), (bx, top), (bx, bot), (bx + 8, bot)], fill=ch.M_TEXT, width=2)
    iv.text(d, (bx - 10, (top + bot) / 2), "kept", iv.font(15, bold=True), fill=ch.M_TEXT, anchor="rm")


def setting_rows(d, rows, y0=160):
    y = y0
    for label, value, kind, focus in rows:
        if focus:
            d.rectangle((330, y - 16, 1236, y + 16), fill=ch.M_FOCUS)
        iv.text(d, (360, y), label, ch.MF, fill=ch.M_TEXT, anchor="lm")
        if kind == "on":
            iv.text(d, (1220, y), value, ch.MF, fill=GREEN, anchor="rm")
        elif kind == "off":
            iv.text(d, (1220, y), value, ch.MF, fill=ch.M_GROUP, anchor="rm")
        elif kind == "pick":
            iv.text(d, (1220, y), "<  %s  >" % value, ch.MF, fill=ch.M_TEXT, anchor="rm")
        else:
            iv.text(d, (1220, y), ">", ch.MF, fill=ch.M_TEXT, anchor="rm")
            iv.text(d, (1196, y), value, ch.MF, fill=ch.M_GROUP, anchor="rm")
        y += 36
    return y


def action_screen(bg, crumb, rows, help1, help2):
    img = bg.copy()
    d = ImageDraw.Draw(img)
    ch.clear_content(d, list_view=True)
    iv.text(d, (53, 118), crumb, ch.MF, fill=ch.M_TEXT, anchor="lm")
    y = 168
    for label, focus in rows:
        if focus:
            d.rectangle((47, y - 16, 1236, y + 16), fill=ch.M_FOCUS)
            d.rectangle((47, y - 16, 51, y + 16), fill=WHITE)
        iv.text(d, (70, y), label, ch.MF, fill=ch.M_TEXT, anchor="lm")
        y += 36
    ch.help_lines(d, help1, help2)
    hints(d, "Up/Down move    A do it    B back")
    return img


def status(game, title, lines):
    img = game.copy()
    d = ImageDraw.Draw(img)
    d.rounded_rectangle((330, 28, 950, 168), radius=10, fill=(250, 250, 252, 236), outline=GREEN, width=3)
    iv.text(d, (360, 48), title, iv.font(20, bold=True), fill=DARK, anchor="lm")
    y = 84
    for line in lines:
        iv.text(d, (360, y), line, iv.font(16), fill=(40, 46, 58, 255), anchor="lm")
        y += 24
    return img


def keyboard(base):
    """The Switch keyboard over the screen it opens from (base), dimmed."""
    img = base.copy()
    o = iv.overlay()
    k = ImageDraw.Draw(o)
    k.rectangle((0, 0, W, H), fill=(6, 8, 12, 214))
    sheet = (150, 36, 1130, 600)
    k.rounded_rectangle(sheet, radius=14, fill=(226, 229, 235, 255), outline=GOLD, width=3)
    iv.text(k, (sheet[0] + 24, sheet[1] + 18),
            "SCHEMATIC · the Switch's own keyboard (system UI, not drawn from it)",
            iv.font(13, bold=True), fill=(150, 104, 0, 255))
    iv.text(k, ((sheet[0] + sheet[2]) / 2, sheet[1] + 64), "Name macro", iv.font(22, bold=True),
            fill=(24, 28, 36, 255), anchor="mm")
    field = (sheet[0] + 120, sheet[1] + 96, sheet[2] - 120, sheet[1] + 146)
    k.rounded_rectangle(field, radius=6, fill=WHITE, outline=(120, 128, 140, 255), width=2)
    typed = "Nair loop"
    iv.text(k, (field[0] + 16, (field[1] + field[3]) / 2), typed, iv.font(20), fill=(24, 28, 36, 255), anchor="lm")
    cx = field[0] + 16 + k.textlength(typed, font=iv.font(20)) + 3
    k.line([(cx, field[1] + 12), (cx, field[3] - 12)], fill=(40, 120, 220, 255), width=2)
    iv.text(k, (field[2] - 12, (field[1] + field[3]) / 2), "%d / 48" % len(typed), iv.font(13),
            fill=(110, 118, 130, 255), anchor="rm")
    kx0, ky0, key, gap = sheet[0] + 120, sheet[1] + 180, 62, 8
    for row, count in enumerate((10, 10, 9, 7)):
        indent = (10 - count) * (key + gap) / 2
        for n in range(count):
            x = kx0 + indent + n * (key + gap)
            y = ky0 + row * (key + gap)
            k.rounded_rectangle((x, y, x + key, y + key), radius=8, fill=(248, 249, 251, 255),
                                outline=(170, 176, 188, 255), width=1)
    sy = ky0 + 4 * (key + gap)
    k.rounded_rectangle((kx0 + 2 * (key + gap), sy, kx0 + 7 * (key + gap) - gap, sy + key - 14), radius=8,
                        fill=(248, 249, 251, 255), outline=(170, 176, 188, 255), width=1)
    ok = (kx0 + 8 * (key + gap), sy, kx0 + 10 * (key + gap) - gap, sy + key - 14)
    k.rounded_rectangle(ok, radius=8, fill=(64, 150, 230, 255))
    iv.text(k, ((ok[0] + ok[2]) / 2, (ok[1] + ok[3]) / 2), "OK", iv.font(16, bold=True), fill=WHITE, anchor="mm")
    img.alpha_composite(o)
    return img


def main_r3():
    """Revision 3's pictures that the repo still keeps (`--r3`): the keyboard, crop, shortcuts and respawn
    screens the doc uses, and combo-from-macro."""
    menu = iv.background(MENU)
    game = iv.background(GAME)
    # Rename opens the keyboard from the macro's own screen: that screen, dimmed, with the cursor on Rename and the
    # help and footer area blank.
    base = action_screen4(menu, "Recording (Player view)  >  Save states & Macros  >  Nair loop", [
        ("Start save", False), ("Set active", False), ("Rename", True), ("Crop", False), ("Make combo", False),
        ("Delete", False),
    ], "", "", {"Start save": (("Nair loop · start", "On"), "switch"), "Set active": ("On · Player 1", "on")})
    bd = ImageDraw.Draw(base)
    bd.rectangle((44, 590, 1246, 656), fill=ch.M_PANEL)
    bd.rectangle((44, 660, 1246, 698), fill=ch.M_PANEL)
    save(keyboard(base), "RECORDING-REDESIGN-keyboard")

    save(status(game, "Make combo · step 1", [
        "Loaded the connected save, Nair loop · start.",
        "Then Nair loop plays on Player 1.",
        "The macro file is kept.",
    ]), "RECORDING-REDESIGN-combo-from-macro")

    img, d = file_screen(menu, "Recording (Player view)  >  Combos & Sequences  >  Nair loop combo  >  Crop",
                         "Links 1 through 3 stay. Link 4 is left out.",
                         "A saves a new combo. The original file stays.",
                         "Up/Down move    A save a copy    B back")
    crop_links(d, [("1  aerial → aerial", "kept", "first"), ("2  aerial → aerial", "kept", ""),
                   ("3  aerial → aerial", "kept", "last"), ("4  aerial → aerial", "left out", "")], kept=3, cursor=2)
    save(img, "RECORDING-REDESIGN-crop")

    img, d = file_screen(menu, "Recording (Player view)  >  Save states & Macros  >  Nair loop  >  Crop",
                         "The same In and Out as the Replay window. L and R set them.",
                         "A saves a new macro. Nair loop itself stays.",
                         "L start    R end    Left/Right move    A save a copy    B back")
    for i, (label, value) in enumerate((("In", "40"), ("Out", "210"))):
        y = 168 + i * 36
        if i == 0:
            d.rectangle((47, y - 16, 1236, y + 16), fill=ch.M_FOCUS)
            d.rectangle((47, y - 16, 1236, y + 16), outline=WHITE, width=2)
            d.rectangle((47, y - 16, 52, y + 16), fill=WHITE)
        iv.text(d, (70, y), label, ch.MF, fill=ch.M_TEXT, anchor="lm")
        iv.text(d, (1210, y), "<  %s  >" % value, ch.MF, fill=ch.M_TEXT, anchor="rm")
    iv.text(d, (70, 262), "Pikachu · the frames between In and Out · a new file", SMALL, fill=ch.M_GROUP, anchor="lm")
    save(img, "RECORDING-REDESIGN-crop-macro")

    img = action_screen(menu, "Recording  >  Options  >  Button chords", [
        ("Save State Save", False),
        ("Save State Load", False),
        ("Record", False),
        ("Play on Player 1", True),
        ("Play on CPU", False),
        ("Recording pop-up", False),
        ("Cycle Player", False),
        ("Cycle CPU", False),
    ], "Save and Load use the bound save state. There is no slot number.",
       "Play on Player 1 is ZR + Left. The pop-up is ZR + Right. Both can be changed here.")
    d = ImageDraw.Draw(img)
    chords = ["ZL + Down", "ZL + Up", "ZR + Down", "ZR + Left", "ZR + Up", "ZR + Right", "ZL + Right", "ZL + Left"]
    for i, chord in enumerate(chords):
        iv.text(d, (1180, 168 + i * 36), chord, ch.MF, fill=ch.M_GROUP, anchor="rm")
    save(img, "RECORDING-REDESIGN-shortcuts")

    img = menu.copy()
    d = ImageDraw.Draw(img)
    ch.clear_content(d, list_view=True)
    d.rectangle((36, 36, 1244, 108), fill=ch.M_PANEL)
    x = 56
    for i, name in enumerate(("CPU", "Saves", "Recording", "Fighters & Stages", "Display & Controls")):
        f = iv.font(16, bold=True)
        w = d.textlength(name, font=f) + 28
        if i == 3:
            d.rounded_rectangle((x, 52, x + w, 84), radius=3, fill=(176, 52, 42, 255))
            iv.text(d, (x + 14, 68), name, f, fill=ch.M_TEXT, anchor="lm")
        else:
            iv.text(d, (x + 14, 68), name, f, fill=ch.M_GROUP, anchor="lm")
        x += w + 16
    iv.text(d, (53, 128), "Fighters & Stages  >  Stages", ch.MF, fill=ch.M_TEXT, anchor="lm")
    setting_rows(d, [
        ("Change Stage", "", "action", False),
        ("Lobby Practice", "[ON]", "on", False),
        ("Lobby Stages", "", "action", False),
        ("Training Stage Rotation", "[OFF]", "off", False),
        ("Respawn on platform", "[OFF]", "off", True),
    ], y0=176)
    ch.help_lines(d, "Respawn on platform moved here from Saves. It is not a save state.",
                  "The row id stays respawn_invincibility. p23/respawn-label implements the move.")
    hints(d, "Up/Down move    Left/Right change    B back")
    save(img, "RECORDING-REDESIGN-respawn")


# =====================================================================================================================
# Revision 4: one big grouped pop-up with a Links panel, the link diagram, loop only on macros and chains,
# the sequence bar at the top. Icons come from Material Symbols SVGs (docs/decisions/icons, Apache-2.0).
# =====================================================================================================================
import math  # noqa: E402
import re  # noqa: E402

from PIL import ImageChops  # noqa: E402

ICON_DIR = os.path.join(OUT, "icons")
TYPES = {
    "macro": ((104, 160, 244, 255), "MACRO"),
    "save": ((242, 152, 72, 255), "SAVE STATE"),
    "combo": ((172, 124, 244, 255), "COMBO"),
    "conv": ((236, 108, 176, 255), "SEQUENCE"),
}
TAB_RED = (176, 52, 42, 255)
DIM = (126, 132, 144, 255)
HEAD_BG = (25, 27, 34, 255)
HEAD_FOCUS = (36, 39, 50, 255)
NODE_BG = (30, 33, 42, 255)
WIN = (13, 13, 13, 255)
INK = (14, 16, 22, 255)


def fnt(size, bold=False):
    return iv.font(size, bold=bold)


# ---- SVG icons: parse the path data and fill it at 8x, then downsample (no system rasteriser is installed) ----------
def _svg_subpaths(d_attr):
    toks = re.findall(r"[A-Za-z]|-?(?:\d+\.\d*|\.\d+|\d+)(?:[eE][-+]?\d+)?", d_attr)
    subs, cur = [], []
    state = {"i": 0}
    x = y = sx = sy = 0.0
    last_ctrl = None
    cmd = None

    def take(n):
        i = state["i"]
        vals = [float(t) for t in toks[i:i + n]]
        state["i"] = i + n
        return vals

    def curve(points, steps=14):
        # points: start, controls..., end (quadratic or cubic Bezier), flattened into the current subpath.
        n = len(points) - 1
        for k in range(1, steps + 1):
            t = k / steps
            px = py = 0.0
            for j, (qx, qy) in enumerate(points):
                coef = math.comb(n, j) * (1 - t) ** (n - j) * t ** j
                px += coef * qx
                py += coef * qy
            cur.append((px, py))

    while state["i"] < len(toks):
        tok = toks[state["i"]]
        if tok.isalpha():
            cmd = tok
            state["i"] += 1
            if cmd in "Zz":
                if cur:
                    subs.append(cur)
                cur = []
                x, y = sx, sy
                last_ctrl = None
                continue
        rel = cmd.islower()
        c = cmd.upper()
        ox, oy = (x, y) if rel else (0.0, 0.0)
        ctrl = None
        if c == "M":
            px, py = take(2)
            x, y = px + ox, py + oy
            if cur:
                subs.append(cur)
            cur = [(x, y)]
            sx, sy = x, y
            cmd = "l" if rel else "L"
        elif c == "L":
            px, py = take(2)
            x, y = px + ox, py + oy
            cur.append((x, y))
        elif c == "H":
            (px,) = take(1)
            x = px + ox
            cur.append((x, y))
        elif c == "V":
            (py,) = take(1)
            y = py + oy
            cur.append((x, y))
        elif c == "Q":
            cx, cy, px, py = take(4)
            ctrl = (cx + ox, cy + oy)
            curve([(x, y), ctrl, (px + ox, py + oy)])
            x, y = px + ox, py + oy
        elif c == "T":
            px, py = take(2)
            ctrl = (2 * x - last_ctrl[0], 2 * y - last_ctrl[1]) if last_ctrl else (x, y)
            curve([(x, y), ctrl, (px + ox, py + oy)])
            x, y = px + ox, py + oy
        elif c == "C":
            c1x, c1y, c2x, c2y, px, py = take(6)
            curve([(x, y), (c1x + ox, c1y + oy), (c2x + ox, c2y + oy), (px + ox, py + oy)])
            x, y = px + ox, py + oy
        else:
            raise ValueError("unsupported SVG path command " + cmd)
        last_ctrl = ctrl if c in "QT" else None
    if cur:
        subs.append(cur)
    return subs


_MASKS = {}


def svg_mask(name, size, angle=0):
    key = (name, size, angle)
    if key in _MASKS:
        return _MASKS[key]
    with open(os.path.join(ICON_DIR, name + ".svg"), encoding="utf-8") as f:
        src = f.read()
    vb = re.search(r'viewBox="([^"]+)"', src)
    if vb:
        minx, miny, vw, vh = (float(v) for v in vb.group(1).split())
    else:
        minx = miny = 0.0
        vw = float(re.search(r'width="([\d.]+)"', src).group(1))
        vh = float(re.search(r'height="([\d.]+)"', src).group(1))
    big = size * 8
    mask = Image.new("L", (big, big), 0)
    for d_attr in re.findall(r'<path[^>]*\sd="([^"]+)"', src):
        for sub in _svg_subpaths(d_attr):
            pts = [((px - minx) / vw * big, (py - miny) / vh * big) for px, py in sub]
            layer = Image.new("L", (big, big), 0)
            ImageDraw.Draw(layer).polygon(pts, fill=255)
            mask = ImageChops.difference(mask, layer)   # even-odd fill
    if angle:
        mask = mask.rotate(angle, resample=Image.BICUBIC, center=(big / 2, big / 2))
    mask = mask.resize((size, size), Image.LANCZOS)
    _MASKS[key] = mask
    return mask


def icon(img, name, x, y, size, colour, angle=0):
    m = svg_mask(name, size, angle)
    layer = Image.new("RGBA", m.size, colour[:3] + (0,))
    layer.putalpha(m.point(lambda v: v * colour[3] // 255))
    img.alpha_composite(layer, (int(round(x)), int(round(y))))


def loop4(img, x, y, mode, size=28):
    """Material 'repeat'. Off: white. On (infinity): green with a dot under it. A count (1, 2, 3, 5, 10): green with a
    number badge on its upper right corner, readable at list size."""
    col = WHITE if mode == "off" else GREEN
    icon(img, "repeat", x, y, size, col)
    d = ImageDraw.Draw(img)
    if mode == "on":
        cx = x + size / 2
        r = max(2.0, size / 13)
        d.ellipse((cx - r, y + size + 1, cx + r, y + size + 1 + 2 * r), fill=col)
    elif mode != "off":
        r = max(8.0, size * 0.2)
        cx, cy = x + size * 0.92, y + size * 0.22
        f = fnt(int(r * 1.4), bold=True)
        half = max(r, d.textlength(mode, font=f) / 2 + 3)
        d.rounded_rectangle((cx - half - 1, cy - r - 1, cx + half + 1, cy + r + 1), radius=r + 1, fill=WIN)
        d.rounded_rectangle((cx - half, cy - r, cx + half, cy + r), radius=r, fill=col)
        iv.text(d, (cx, cy + 0.5), mode, f, fill=INK, anchor="mm")


def badge(d, cx, cy, n, r=10):
    d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=RED)
    iv.text(d, (cx, cy + 0.5), str(n), fnt(int(r * 1.3), bold=True), fill=WHITE, anchor="mm")


def head_mark(img, typ, cx, cy, size):
    """A section header's kind mark, centred on (cx, cy): the orange floppy for SAVE STATES (revision 8), a square for
    the other kinds."""
    col = TYPES[typ][0]
    if typ == "save":
        s = int(round(size * 1.5))
        icon(img, "save", cx - s / 2, cy - s / 2, s, col)
    else:
        ImageDraw.Draw(img).rectangle((cx - size / 2, cy - size / 2, cx + size / 2, cy + size / 2), fill=col)


def pill(d, x, cy, typ, size=10):
    col, label = TYPES[typ]
    f = fnt(size, bold=True)
    w = d.textlength(label, font=f) + 12
    d.rounded_rectangle((x, cy - size + 1, x + w, cy + size - 1), radius=size - 1, fill=col)
    iv.text(d, (x + w / 2, cy + 0.5), label, f, fill=INK, anchor="mm")
    return x + w


def node4(img, box, typ, name, badge_n=None, suffix=None, link=False, fsize=14, psize=10, width=2):
    d = ImageDraw.Draw(img)
    col = TYPES[typ][0]
    x0, y0, x1, y1 = box
    d.rounded_rectangle(box, radius=6, fill=NODE_BG, outline=col, width=width)
    cy = (y0 + y1) / 2
    x = x0 + 8
    if badge_n is not None:
        badge(d, x + 10, cy, badge_n)
        x += 26
    x = pill(d, x, cy, typ, psize) + 8
    if link:
        icon(img, "save", x, cy - 9, 18, col)
        x += 22
    f = fnt(fsize)
    iv.text(d, (x, cy), name, f, fill=ch.M_TEXT, anchor="lm")
    if suffix:
        x += d.textlength(name, font=f) + 8
        iv.text(d, (x, cy), suffix, fnt(fsize - 1), fill=ch.M_GROUP, anchor="lm")


def arrow4(d, pts, col, width=2, head=10):
    d.line(pts, fill=col, width=width, joint="curve")
    (ax, ay), (bx, by) = pts[-2], pts[-1]
    ang = math.atan2(by - ay, bx - ax)
    a, b = ang + math.radians(152), ang - math.radians(152)
    d.polygon([(bx, by), (bx + head * math.cos(a), by + head * math.sin(a)),
               (bx + head * math.cos(b), by + head * math.sin(b))], fill=col)


def wrap(d, s, f, width):
    words, lines, line = s.split(" "), [], ""
    for w in words:
        trial = (line + " " + w).strip()
        if d.textlength(trial, font=f) <= width or not line:
            line = trial
        else:
            lines.append(line)
            line = w
    if line:
        lines.append(line)
    return lines


def keys_line(img, x, y, parts, size=14):
    """A hint line: the button in white, what it does in grey."""
    d = ImageDraw.Draw(img)
    fk, fl = fnt(size, bold=True), fnt(size)
    for key, label in parts:
        iv.text(d, (x, y), key, fk, fill=ch.M_TEXT, anchor="lm")
        x += d.textlength(key, font=fk) + 6
        iv.text(d, (x, y), label, fl, fill=ch.M_GROUP, anchor="lm")
        x += d.textlength(label, font=fl)
        x += 22
    return x


def hide_notice4(img):
    """The Training frame still shows the old Input Playback notice. Fill that box row by row with the sky colours
    just left and right of it, so the gradient continues."""
    px = img.load()
    for yy in range(36, 144):
        left, right = px[302, yy], px[978, yy]
        for xx in range(303, 978):
            t = (xx - 302) / (978 - 302)
            px[xx, yy] = tuple(int(left[k] * (1 - t) + right[k] * t) for k in range(4))
    return img


# ---- The example library (one coherent set used by every picture) -----------------------------------------------
# The hot rule, in every list (revision 7): the hot five are the five most recently used of a kind, the most recent
# first. Used means selected (bound or set active), played, drilled or recorded. A file sits under the hairline only
# when five are already hot, so a kind with five files or fewer is all hot and has no hairline. Nothing is held at
# the top by hand.
# Macros, most recently used first: Nair loop was just set active for Player 1; before that the chain Corner pressure
# was set active for the CPU and played. Then CPU throw and Dash grab, then Macro 6. Replay 86f and the migrated slots
# are the least recent. Revision 8: a chain is a compound macro, a file in MACROS, with its macros in order. Under Q12 A
# its start save is its macro 1's.
M4 = [
    dict(key="nair", name="Nair loop", fighter="Pikachu", role="P1", length="2.4 s", start="Nair loop · start",
         hot=True),
    dict(key="cpchain", name="Corner pressure", fighter="Pikachu · Mario", role=None, length="5.3 s",
         start="Nair loop · start", chain=["nair", "dash", "cputhrow"], hot=True),
    dict(key="cputhrow", name="CPU throw", fighter="Mario", role="CPU", length="1.8 s", start="CPU throw · start",
         hot=True),
    dict(key="dash", name="Dash grab", fighter="Pikachu", role="P1", length="1.1 s", start="Dash grab · start",
         hot=True),
    dict(key="macro6", name="Macro 6", fighter="Lucina", role="P1", length="0.8 s", start="Macro 6 · start", hot=True),
    dict(key="replay", name="Replay 86f", fighter="Pikachu", role="P1", length="1.4 s", start=None, hot=False),
    dict(key="slot2", name="Slot 2 (2026-11-05)", fighter="Steve", role="P1", length="4.0 s", start=None, hot=False),
    dict(key="slot5", name="Slot 5 (2026-11-05)", fighter="Villager", role="P1", length="3.2 s", start=None, hot=False),
]
# The recordings alone, as the library was before strip 2 made the chain.
RECORDINGS = [m for m in M4 if not m.get("chain")]
C4 = [
    dict(key="ryu", name="Ryu BnB", fighters="Ryu → Mario", links="6 links", src=None, hot=True),
    dict(key="nairc", name="Nair loop combo", fighters="Pikachu → Mario", links="4 links", src="Nair loop", hot=True),
    dict(key="combo4", name="Combo 4", fighters="Steve → Pikachu", links="3 links", src=None, hot=True),
    dict(key="combo2", name="Combo 2", fighters="Pikachu → Mario", links="2 links", src=None, hot=True),
]
V4 = [
    dict(key="corner", name="Corner to kill", seq=["Ryu BnB", "Nair loop combo"], hot=True),
    dict(key="platstr", name="Platform strings", seq=["Combo 2", "Combo 4"], hot=True),
    dict(key="conv3", name="Sequence 3", seq=["Combo 4"], hot=True),
]
# Saves, most recently used first: Corner is bound and loaded; the three "· start" saves were written by Record after
# the 2026-11-05 migration, so the migrated slots are the least recent. Revision 8: a save only warps; it has no chain
# and no role.
S4 = [
    dict(key="pressure", name="Corner", fighters="Pikachu · Mario", hot=True),
    dict(key="platform", name="Platform", fighters="Pikachu · Mario", hot=True),
    dict(key="nairstart", name="Nair loop · start", fighters="Pikachu · Mario", hot=True),
    dict(key="dashstart", name="Dash grab · start", fighters="Pikachu · Mario", hot=True),
    dict(key="throwstart", name="CPU throw · start", fighters="Pikachu · Mario", hot=True),
    dict(key="slot1", name="Slot 1 (2026-11-05)", fighters="Steve · Villager", hot=False),
    dict(key="slot3", name="Slot 3 (2026-11-05)", fighters="Lucina · Cloud", hot=False),
]
MNAME = {m["key"]: m for m in M4}
DRILL = "corner"
BOUND = "pressure"
# One bound save at a time, green in both role views; it only warps. One active macro per role: Nair loop for Player 1,
# the chain Corner pressure for the CPU, which loops it (∞) while you practise against it.
ROLE_NAME = {"player": "Player 1", "cpu": "the CPU"}
ACTIVE = {"player": "nair", "cpu": "cpchain"}
LOOP = {"player": "1", "cpu": "on"}

# Column layouts: (field, x, caption). Pop-up list is 592 wide, the page list 530, a flow fragment 361.
COLS = {
    "pop": {
        "macro": [("fighter", 222, "Fighter"), ("length", 340, "Length"), ("start", 400, "Start save")],
        "combo": [("fighters", 232, "Fighters"), ("links", 366, "Links"), ("src", 434, "Made from")],
        "conv": [("seq", 232, "Contains, in order")],
        "save": [("fighters", 222, "Fighters")],
    },
    "page": {
        "macro": [("fighter", 214, "Fighter"), ("length", 306, "Length"), ("start", 362, "Start save")],
        "combo": [("fighters", 214, "Fighters"), ("links", 340, "Links"), ("src", 404, "Made from")],
        "conv": [("seq", 214, "Contains, in order")],
        "save": [("fighters", 214, "Fighters")],
    },
    "frag": {
        "macro": [("start", 196, "Start save")],
        "combo": [("src", 196, "Made from")],
        "conv": [("seq", 162, "Contains, in order")],
        "save": [],
    },
}
ROW_H, HEAD_H, SUB_H = 30, 36, 26


def groups_for(role, focus=None, overrides=None):
    """The stacked groups of the pop-up for one role. CPU: MACROS and SAVE STATES only."""
    overrides = overrides or {}

    def rows(data, typ):
        out = []
        for r in data:
            r = dict(r, **overrides.get(r["key"], {}))
            if typ == "macro":
                r["active"] = r["key"] == ACTIVE[role] if role in ACTIVE else r["key"] in ACTIVE.values()
                if role == "page":
                    r["tag"] = "P1" if r["key"] == ACTIVE["player"] else "CPU" if r["key"] == ACTIVE["cpu"] else None
            elif typ in ("combo", "conv"):
                r["active"] = r["key"] == overrides.get("_drill", DRILL)
            else:
                r["active"] = r["key"] == BOUND
            r["cursor"] = r["key"] == focus
            out.append(r)
        return out

    macros = {"title": "MACROS", "typ": "macro", "count": len(M4),
              "loop": [("", LOOP[role])] if role in LOOP else [("P1", LOOP["player"]), ("CPU", LOOP["cpu"])],
              "sections": [(None, "macro", rows(M4, "macro"))]}
    combos = {"title": "COMBOS & SEQUENCES", "typ": "combo", "count": len(C4) + len(V4), "loop": None,
              "sections": [("Combos", "combo", rows(C4, "combo")), ("Sequences", "conv", rows(V4, "conv"))]}
    saves = {"title": "SAVE STATES", "typ": "save", "count": len(S4), "loop": None,
             "sections": [(None, "save", rows(S4, "save"))]}
    if role == "cpu":
        return [macros, saves]
    return [macros, combos, saves]


def render_list(groups, width, cols, focus=None, extra_rows=None):
    """Draw the stacked groups on one tall image; return it with each row's (top, bottom, typ) and the focused group."""
    extra_rows = extra_rows or {}
    height = 0
    for g in groups:
        height += HEAD_H + SUB_H + 16
        for sub, typ, rws in g["sections"]:
            height += SUB_H + ROW_H * (len(rws) + len(extra_rows.get(typ, []))) + 24
    img = Image.new("RGBA", (width, height), WIN)
    d = ImageDraw.Draw(img)
    pos, focus_group, y = {}, None, 0
    for gi, g in enumerate(groups):
        pos["__group%d" % gi] = (y, y + HEAD_H, g["typ"])
        has_focus = any(r.get("cursor") for _, _, rws in g["sections"] for r in rws)
        if has_focus:
            focus_group = g["typ"]
        d.rounded_rectangle((0, y + 2, width - 1, y + HEAD_H - 2), radius=5, fill=HEAD_FOCUS if has_focus else HEAD_BG)
        cy = y + HEAD_H / 2
        head_mark(img, g["typ"], 16, cy, 12)
        d = ImageDraw.Draw(img)
        ft = fnt(14, bold=True)
        iv.text(d, (32, cy), g["title"], ft, fill=ch.M_TEXT if has_focus else ch.M_GROUP, anchor="lm")
        tx = 32 + d.textlength(g["title"], font=ft) + 8
        iv.text(d, (tx, cy), "(%d)" % g["count"], fnt(13), fill=DIM, anchor="lm")
        single = len(g["sections"]) == 1
        inline = single and g["sections"][0][0] is None and (not g["loop"] or len(g["loop"]) == 1)
        if inline:
            for field, x, cap in cols[g["sections"][0][1]]:
                iv.text(d, (x, cy), cap, fnt(12), fill=DIM, anchor="lm")
        if g["loop"]:
            lx = width - 10
            for label, mode in reversed(g["loop"]):
                lx -= 30
                loop4(img, lx, y + 3, mode, 26)
                d = ImageDraw.Draw(img)
                if label:
                    lx -= 6
                    iv.text(d, (lx, cy), label, fnt(12, bold=True), fill=ch.M_GROUP, anchor="rm")
                    lx -= d.textlength(label, font=fnt(12, bold=True)) + 10
            iv.text(d, (lx - 6, cy), "Loop", fnt(12), fill=ch.M_GROUP, anchor="rm")
        y += HEAD_H
        if single and not inline and g["sections"][0][0] is None:
            for field, x, cap in cols[g["sections"][0][1]]:
                iv.text(d, (x, y + SUB_H / 2), cap, fnt(12), fill=DIM, anchor="lm")
            y += SUB_H
        for sub, typ, rws in g["sections"]:
            rws = rws + extra_rows.get(typ, [])
            if sub:
                iv.text(d, (34, y + SUB_H / 2 + 1), sub, fnt(13, bold=True), fill=ch.M_GROUP, anchor="lm")
                for field, x, cap in cols[typ]:
                    iv.text(d, (x, y + SUB_H / 2 + 1), cap, fnt(12), fill=DIM, anchor="lm")
                y += SUB_H
            hot = [r for r in rws if r.get("hot")]
            first_top = y
            for i, r in enumerate(rws):
                if hot and i == len(hot):
                    if not r.get("action"):   # an action row (+ New sequence) is not a file: no hairline
                        d.line((10, y + 6, width - 10, y + 6), fill=ch.M_RULE, width=1)
                    y += 12
                top, bot = y, y + ROW_H
                draw_row(img, r, typ, top, bot, width, cols[typ])
                d = ImageDraw.Draw(img)
                pos[r["key"]] = (top, bot, typ)
                y = bot
                if hot and i == len(hot) - 1:
                    d.rounded_rectangle((3, first_top - 2, width - 4, bot + 2), radius=7, outline=GREEN, width=2)
            y += 8
        y += 8
    return img.crop((0, 0, width, y)), pos, focus_group


def draw_row(img, r, typ, top, bot, width, cols):
    d = ImageDraw.Draw(img)
    box = (8, top + 2, width - 9, bot - 2)
    if r.get("active"):
        d.rectangle(box, fill=GREEN_ROW)
    elif r.get("cursor"):
        d.rectangle(box, fill=ch.M_FOCUS)
    if r.get("new"):
        d.rectangle(box, outline=GOLD, width=2)
    if r.get("cursor"):
        d.rectangle(box, outline=WHITE, width=2)
        d.rectangle((box[0], box[1], box[0] + 5, box[3]), fill=WHITE)
    cy = (top + bot) / 2
    hot = r.get("hot")
    # Revision 6: the order badge sits at the row's left edge.
    lx = 0
    if r.get("order"):
        badge(d, 22, cy, r["order"], 9)
        lx = 22
    name_col = ch.M_TEXT if hot else ch.M_GROUP
    fn = fnt(15)
    iv.text(d, (34 + lx, cy), r["name"], fn, fill=name_col, anchor="lm")
    name_end = 34 + lx + d.textlength(r["name"], font=fn)
    if r.get("action"):
        return
    if typ == "macro" and r.get("chain"):     # a chain: the chain mark and how many macros it plays
        cx = name_end + 10
        icon(img, "format_list_numbered", cx, cy - 9, 18, ch.M_TEXT if hot else DIM)
        d = ImageDraw.Draw(img)
        iv.text(d, (cx + 22, cy), str(len(r["chain"])), fnt(14, bold=True), fill=ch.M_TEXT if hot else DIM,
                anchor="lm")
        name_end = cx + 22 + d.textlength(str(len(r["chain"])), font=fnt(14, bold=True))
    cell = ch.M_GROUP if hot else DIM
    fc = fnt(14)
    for field, x, _ in cols:
        if field == "start":     # a chain's mark may reach past the column: the start save follows it
            x = max(x, name_end + 12)
        if field in ("fighter", "length", "fighters", "links"):
            v = r.get(field, "")
            if field == "fighter" and r.get("role") == "CPU":
                v += " · CPU"
            iv.text(d, (x, cy), v, fc, fill=cell, anchor="lm")
        elif field == "start":
            if r.get("start"):
                # The floppy chip: orange while the Start save switch is On; grey when it is Off (the link is kept).
                on = hot and r.get("start_on", True)
                icon(img, "save", x, cy - 9, 18, TYPES["save"][0] if on else DIM)
                d = ImageDraw.Draw(img)
                iv.text(d, (x + 22, cy), r["start"], fc, fill=cell, anchor="lm")
            else:
                iv.text(d, (x + 4, cy), "—  none", fnt(13), fill=DIM, anchor="lm")
        elif field == "src":
            if r.get("src"):
                iv.text(d, (x, cy), "from " + r["src"], fnt(13), fill=cell, anchor="lm")
        elif field == "seq":
            iv.text(d, (x, cy), " → ".join(r["seq"]), fc if width > 400 else fnt(13), fill=cell, anchor="lm")


# ---- The Links panel: the focused row's links, as a small diagram --------------------------------------------------
def links_spec(key, role="player"):
    """What the Links panel shows for one row (the example data)."""
    if key in MNAME:
        m = MNAME[key]
        switch = "· On" if m.get("start_on", True) else "· Off"
        start = {"label": "starts from (warps there once, first)", "out": True,
                 "nodes": [("save", m["start"], None, switch, True)] if m["start"] else [],
                 "none": "no start save · plays from where the fighters are"}
        active = [role] if ACTIVE.get(role) == key else []
        if m.get("chain"):
            lines = []
            if active:
                lines.append(("LOOP", LOOP[active[0]], None))
                lines.append(("Active for", ROLE_NAME[active[0]] + ", whose Play chord plays it", GREEN))
            lines.append(("Recorded on", m["fighter"] + " · information only", ch.M_TEXT))
            return {
                "self": ("macro", m["name"]),
                "branches": [
                    start,
                    {"label": "plays, in order", "out": True, "ordered": True,
                     "nodes": [("macro", MNAME[k]["name"], i + 1, None, False) for i, k in enumerate(m["chain"])],
                     "none": "empty: plays nothing"},
                ],
                "lines": lines,
            }
        chains = [(c["name"], c["chain"].index(key) + 1) for c in M4 if key in c.get("chain", [])]
        made = [c["name"] for c in C4 if c.get("src") == m["name"]]
        return {
            "self": ("macro", m["name"]),
            "branches": [
                start,
                {"label": "in chains", "out": False,
                 "nodes": [("macro", n, None, "as %d" % i, False) for n, i in chains], "none": "none"},
                {"label": "combos made from it", "out": False,
                 "nodes": [("combo", n, None, None, False) for n in made],
                 "none": "none (Make combo plays on Player 1)" if m["role"] == "CPU" else "none"},
            ],
            # The recorded role, like the recorded fighter, is information only.
            "lines": [("Recorded on", "%s%s · %s · information only" % (
                m["fighter"], ", as CPU" if m["role"] == "CPU" else "", m["length"]), ch.M_TEXT)],
        }
    combo = {c["key"]: c for c in C4}.get(key)
    if combo:
        convs = [(v["name"], v["seq"].index(combo["name"]) + 1) for v in V4 if combo["name"] in v["seq"]]
        return {
            "self": ("combo", combo["name"]),
            "branches": [
                {"label": "made from", "out": True,
                 "nodes": [("macro", combo["src"], None, None, False)] if combo.get("src") else [],
                 "none": "recorded in a match, not from a macro"},
                {"label": "in sequences", "out": False,
                 "nodes": [("conv", n, None, "#%d" % i, False) for n, i in convs], "none": "none"},
            ],
            "lines": [("Its links (4)", "", None),
                      ("L1  aerial → aerial", "TRUE", GREEN, "jump · release · ↓ flick · A", "drift →"),
                      ("L2  aerial → aerial", "TRUE", GREEN, "jump · release · ↓ flick · A", "drift →"),
                      ("L3  aerial → aerial", "TRUE", GREEN, "jump · release · ↓ flick · A", "drift →"),
                      ("L4  aerial → aerial", "TRUE", GREEN, "jump · release · ↓ flick · A", "drift →"),
                      ("Fighters", combo["fighters"], ch.M_TEXT)],
        }
    conv = {v["key"]: v for v in V4}.get(key)
    if conv:
        return {
            "self": ("conv", conv["name"]),
            "branches": [
                {"label": "contains, in order", "out": True, "ordered": True,
                 "nodes": [("combo", n, i + 1, None, False) for i, n in enumerate(conv["seq"])], "none": "empty"},
            ],
            "lines": [("Streak", "4 · best 9", GREEN), ("Drilled", "yes, the one drill" if key == DRILL else "no",
                                                      GREEN if key == DRILL else ch.M_TEXT)],
        }
    s = {x["key"]: x for x in S4}[key]
    bound = key == BOUND
    # A save only warps: it starts nothing. Its links are the macros that start from it.
    users = [(m["name"], m) for m in M4 if m["start"] == s["name"]]
    lines = [("Bound to Save / Load", "yes · Load warps here" if bound else "no", GREEN if bound else ch.M_TEXT)]
    return {
        "self": ("save", s["name"]),
        "branches": [
            {"label": "start save of", "out": False,
             "nodes": [("macro", n, None, None, False) for n, _ in users], "none": "no macro starts here"},
        ],
        "lines": lines,
    }


def links_panel(img, box, spec, header=True):
    d = ImageDraw.Draw(img)
    x0, y0, x1, y1 = box
    y = y0
    if header:
        word = "CONNECTIONS"
        fw, fs = fnt(12, bold=True), fnt(12)
        iv.text(d, (x0, y + 8), word, fw, fill=ch.M_GROUP, anchor="lm")
        sub = "of the row under the cursor"
        sub_x = x0 + d.textlength(word, font=fw) + 8
        if sub_x + d.textlength(sub, font=fs) <= x1:
            iv.text(d, (sub_x, y + 8), sub, fs, fill=DIM, anchor="lm")
            y += 22
        else:
            y += 18
            iv.text(d, (x0, y + 8), sub, fs, fill=DIM, anchor="lm")
            y += 22
    typ, name = spec["self"]
    node4(img, (x0, y, x1, y + 38), typ, name, fsize=15, width=3)
    d = ImageDraw.Draw(img)
    stem_x = x0 + 16
    stem_top = y + 38
    stem_bot = stem_top
    y += 38 + 6
    for br in spec["branches"]:
        # A label wider than the panel wraps onto a second line, at its last " · " when it has one.
        lines = wrap(d, br["label"], fnt(13, bold=True), x1 - x0 - 34)
        if len(lines) > 1 and " · " in br["label"]:
            head, tail = br["label"].rsplit(" · ", 1)
            lines = [head, "· " + tail]
        for k, line in enumerate(lines):
            if k:
                y += 18
            iv.text(d, (x0 + 34, y + 10), line, fnt(13, bold=True), fill=ch.M_GROUP, anchor="lm")
        y += 22
        if not br["nodes"]:
            iv.text(d, (x0 + 34, y + 9), br["none"], fnt(13), fill=DIM, anchor="lm")
            y += 26
            continue
        for k, (ntyp, nname, nb, suffix, link) in enumerate(br["nodes"]):
            cy = y + 16
            col = TYPES[ntyp][0] if not br.get("dim") else DIM
            if br["out"]:
                arrow4(d, [(stem_x, cy), (x0 + 32, cy)], col, width=2, head=7)
            else:
                arrow4(d, [(x0 + 32, cy), (stem_x + 1, cy)], col, width=2, head=7)
            node4(img, (x0 + 34, y, x1, y + 32), ntyp, nname, badge_n=nb, suffix=suffix, link=link, fsize=14)
            d = ImageDraw.Draw(img)
            stem_bot = cy
            y += 32
            if br.get("ordered") and k < len(br["nodes"]) - 1:
                iv.text(d, (x0 + 60, y + 8), "then", fnt(11), fill=DIM, anchor="lm")
                y += 16
            else:
                y += 6
        y += 4
    d.line((stem_x, stem_top, stem_x, stem_bot), fill=(150, 156, 170, 255), width=2)
    y += 4
    for item in spec["lines"]:
        label, value, colour = item[:3]
        detail = item[3] if len(item) > 3 else None
        drift = item[4] if len(item) > 4 else None
        if label == "LOOP":
            iv.text(d, (x0, y + 14), "Loop", fnt(13, bold=True), fill=ch.M_GROUP, anchor="lm")
            loop4(img, x0 + 40, y, value, 28)
            d = ImageDraw.Draw(img)
            word = {"on": "∞", "off": "off"}.get(value, "× " + value)
            iv.text(d, (x0 + 76, y + 14), "%s · the whole chain repeats" % word, fnt(14), fill=ch.M_TEXT, anchor="lm")
            y += 38
        elif label == "CHIPS":
            x = x0
            for chip_label in value:
                d.rounded_rectangle((x, y, x + 52, y + 24), radius=5, fill=(32, 38, 52, 255), outline=ch.M_RULE)
                iv.text(d, (x + 26, y + 12), chip_label, fnt(12, bold=True), fill=ch.M_TEXT, anchor="mm")
                x += 60
            y += 32
        else:
            fl = fnt(13, bold=True)
            iv.text(d, (x0, y + 10), label, fl, fill=ch.M_GROUP, anchor="lm")
            if value:
                iv.text(d, (x0 + d.textlength(label, font=fl) + 10, y + 10), value, fnt(14),
                        fill=colour or ch.M_TEXT, anchor="lm")
            if detail:
                fe = fnt(11)
                ev_x = x0
                # Stay inside the panel. A line that would pass the right edge is not drawn.
                if ev_x + d.textlength(detail, font=fe) <= x1:
                    iv.text(d, (ev_x, y + 26), detail, fe, fill=ch.M_TEXT, anchor="lm")
                    if drift:
                        dx = ev_x + d.textlength(detail, font=fe) + 10
                        if dx + d.textlength(drift, font=fe) <= x1:
                            iv.text(d, (dx, y + 26), drift, fe, fill=DIM, anchor="lm")
                y += 40
            else:
                y += 26
    return y


POP = (128, 72, 1152, 648)
POP_LIST = (144, 136, 736, 596)
POP_LINKS = (762, 140, 1136, 596)


def role_toggle(d, cx, y, role, size=16):
    f = fnt(size, bold=True)
    segs = [("Player", role == "player"), ("CPU", role == "cpu")]
    w = 116
    x = cx - w
    d.rounded_rectangle((x - 4, y - 4, x + 2 * w + 4, y + 34), radius=6, outline=ch.M_RULE, width=1)
    for label, on in segs:
        if on:
            d.rectangle((x, y, x + w, y + 30), fill=TAB_RED)
        iv.text(d, (x + w / 2, y + 15), label, f, fill=ch.M_TEXT if on else ch.M_GROUP, anchor="mm")
        x += w
    fk = fnt(13, bold=True)
    for kx, k in ((cx - w - 26, "L"), (cx + w + 26, "R")):
        d.rounded_rectangle((kx - 11, y + 4, kx + 11, y + 26), radius=5, outline=ch.M_GROUP, width=1)
        iv.text(d, (kx, y + 15), k, fk, fill=ch.M_GROUP, anchor="mm")


def sticky_header(img, lst, pos, scroll, x0, y0):
    """A scrolled list keeps the current group's header at the top, so the group is always named."""
    if scroll <= 0:
        return
    tops = sorted(v[0] for k, v in pos.items() if k.startswith("__group") and v[0] < scroll + 4)
    if not tops:
        return
    top = tops[-1]
    nxt = [v[0] for k, v in pos.items() if k.startswith("__group") and v[0] > top]
    head = lst.crop((0, top, lst.width, top + HEAD_H))
    shift = 0
    if nxt and nxt[0] - scroll < HEAD_H:
        shift = nxt[0] - scroll - HEAD_H   # the next header pushes this one up
    if shift <= -HEAD_H:
        return
    img.alpha_composite(head.crop((0, -shift, lst.width, HEAD_H)), (x0, y0))
    ImageDraw.Draw(img).line((x0, y0 + HEAD_H + shift, x0 + lst.width, y0 + HEAD_H + shift), fill=(0, 0, 0, 255),
                             width=2)


# ---- The link diagram ----------------------------------------------------------------------------------------------
def big_node(img, box, typ, title, desc):
    d = ImageDraw.Draw(img)
    col = TYPES[typ][0]
    d.rounded_rectangle(box, radius=10, fill=NODE_BG, outline=col, width=3)
    x0, y0, x1, y1 = box
    pill(d, x0 + 14, y0 + 22, typ, 11)
    iv.text(d, (x0 + 14, y0 + 50), title, fnt(18, bold=True), fill=ch.M_TEXT, anchor="lm")
    iv.text(d, (x0 + 14, y0 + 74), desc, fnt(13), fill=ch.M_GROUP, anchor="lm")


def link_label(d, x, y, label, how, col, anchor="mm", how_first=False, width=440):
    fl, fh = fnt(16, bold=True), fnt(13)
    lines = [line for part in how.split("\n") for line in wrap(d, part, fh, width)]
    iv.text(d, (x, y), label, fl, fill=col, anchor=anchor)
    if how_first:
        for k, line in enumerate(reversed(lines)):
            iv.text(d, (x, y - 21 - 18 * k), line, fh, fill=ch.M_GROUP, anchor=anchor)
    else:
        for k, line in enumerate(lines):
            iv.text(d, (x, y + 21 + 18 * k), line, fh, fill=ch.M_GROUP, anchor=anchor)


def diagram():
    img = Image.new("RGBA", (W, 940), WIN)
    d = ImageDraw.Draw(img)
    iv.text(d, (40, 30), "What connects to what", fnt(24, bold=True), fill=ch.M_TEXT, anchor="lm")
    # Kept to the left of the chain note, which sits above the Macro box.
    sub = ("Four kinds of file. Four kinds of connection. Each arrow says what the connection means; the grey line "
           "says how you make it.")
    for k, line in enumerate(wrap(d, sub, fnt(14), 760)):
        iv.text(d, (40, 60 + 18 * k), line, fnt(14), fill=ch.M_GROUP, anchor="lm")
    save_b, macro_b = (60, 150, 360, 250), (840, 150, 1140, 250)
    conv_b, combo_b = (60, 380, 360, 480), (840, 380, 1140, 480)
    big_node(img, save_b, "save", "Save state", "both fighters; Load only warps here")
    big_node(img, macro_b, "macro", "Macro", "a recording, or a chain of macros")
    big_node(img, conv_b, "conv", "Sequence", "an ordered list of combos")
    big_node(img, combo_b, "combo", "Combo", "the hits the helper grades")
    d = ImageDraw.Draw(img)
    sc, mc, vc, cc = (TYPES[k][0] for k in ("save", "macro", "conv", "combo"))
    # macro -> save (start save), with the floppy chip
    arrow4(d, [(834, 200), (368, 200)], mc, width=3, head=13)
    icon(img, "save", 585, 162, 26, sc)
    d = ImageDraw.Draw(img)
    link_label(d, 600, 222, "starts from", "automatic: Record writes \"<macro name> · start\" and connects it.\nA Play "
               "chord warps there once, first, while the Start save switch is On.", mc, width=470)
    # chain -> macros: a loop on the Macro box
    arrow4(d, [(1100, 150), (1100, 112), (1180, 112), (1180, 200), (1146, 200)], mc, width=3, head=12)
    link_label(d, 1004, 100, "a chain: plays macros, in order", "MACROS › + New chain, numbered like Character "
               "Cycles", mc, anchor="mm", how_first=True, width=300)
    # sequence -> combo
    arrow4(d, [(366, 430), (832, 430)], vc, width=3, head=13)
    link_label(d, 600, 414, "contains, in order", "made with Combos & Sequences › New sequence:\nthe same "
               "numbered picker, then a name on the Switch keyboard", vc, how_first=True)
    # combo -> macro
    arrow4(d, [(990, 374), (990, 258)], cc, width=3, head=13)
    link_label(d, 1006, 306, "made from", "a macro's Make combo", cc, anchor="lm", width=230)
    # The loop note.
    loop4(img, 60, 296, "on", 26)
    d = ImageDraw.Draw(img)
    iv.text(d, (96, 310), "Loop belongs to a role. A chain repeats as one unit, by the loop of the role it is active "
            "for, and no pass warps back.", fnt(13), fill=ch.M_GROUP, anchor="lm")

    # ---- the example ----
    d.line((40, 520, 1240, 520), fill=ch.M_RULE, width=1)
    iv.text(d, (40, 548), "Example", fnt(20, bold=True), fill=ch.M_TEXT, anchor="lm")
    iv.text(d, (150, 548), "the files every picture in this document uses", fnt(14), fill=ch.M_GROUP, anchor="lm")
    start_b = (60, 580, 400, 622)
    chain_b = (60, 690, 500, 732)
    mac = [(760, 590, 1100, 632), (760, 654, 1100, 696), (760, 718, 1100, 760)]
    corner_b = (60, 836, 400, 878)
    combos = [(760, 806, 1100, 848), (760, 868, 1100, 910)]
    node4(img, start_b, "save", "Nair loop · start", link=True)
    node4(img, chain_b, "macro", "Corner pressure", suffix="a chain, the CPU's active macro")
    for i, (b, k) in enumerate(zip(mac, ["nair", "dash", "cputhrow"])):
        node4(img, b, "macro", MNAME[k]["name"], badge_n=i + 1)
    node4(img, corner_b, "conv", "Corner to kill")
    node4(img, combos[0], "combo", "Ryu BnB", badge_n=1)
    node4(img, combos[1], "combo", "Nair loop combo", badge_n=2)
    d = ImageDraw.Draw(img)
    # starts from: Nair loop -> Nair loop · start, and the chain -> the same file (its macro 1's start save)
    arrow4(d, [(754, 600), (408, 600)], mc, width=2, head=10)
    iv.text(d, (580, 588), "starts from", fnt(13, bold=True), fill=mc, anchor="mm")
    arrow4(d, [(230, 688), (230, 630)], mc, width=2, head=10)
    iv.text(d, (242, 659), "starts from", fnt(13, bold=True), fill=mc, anchor="lm")
    # a chain: Corner pressure -> 1, 2, 3
    for k, b in enumerate(mac):
        cy = (b[1] + b[3]) / 2 + (8 if k == 0 else 0)
        arrow4(d, [(506, 711), (680, 711), (716, cy), (752, cy)], mc, width=2, head=10)
    iv.text(d, (520, 696), "plays, in order", fnt(13, bold=True), fill=mc, anchor="lm")
    # contains, in order: Corner to kill -> 1, 2
    for b in combos:
        cy = (b[1] + b[3]) / 2
        arrow4(d, [(406, 857), (630, 857), (670, cy), (752, cy)], vc, width=2, head=10)
    iv.text(d, (516, 842), "contains, in order", fnt(13, bold=True), fill=vc, anchor="mm")
    # made from: Nair loop combo -> Nair loop
    arrow4(d, [(1106, 889), (1150, 889), (1150, 611), (1106, 611)], cc, width=2, head=10)
    iv.text(d, (1162, 750), "made from", fnt(13, bold=True), fill=cc, anchor="lm")
    return img


# ---- A legend for the list, and the loop mark ----------------------------------------------------------------------
def legend():
    width = 560
    # The whole MACROS list: the five most recently used, the most recent first, and three under the hairline.
    keys = {"nair": "a", "dash": "b", "macro6": "c", "replay": "d"}
    rows = [dict(m, key=keys.get(m["key"], m["key"]), active=m["key"] == "nair", cursor=m["key"] == "dash")
            for m in M4]
    groups = [{"title": "MACROS", "typ": "macro", "count": len(M4), "loop": [("", "1")],
               "sections": [(None, "macro", rows)]}]
    lst, pos, _ = render_list(groups, width, {"macro": COLS["pop"]["macro"]})
    img = Image.new("RGBA", (W, 64 + lst.height + 140), WIN)
    d = ImageDraw.Draw(img)
    iv.text(d, (40, 30), "How to read a list", fnt(22, bold=True), fill=ch.M_TEXT, anchor="lm")
    img.alpha_composite(lst, (40, 64))
    both = Image.new("RGBA", (width, 40), WIN)
    draw_row(both, dict(S4[0], hot=True, active=True, cursor=True), "save", 4, 34, width, COLS["pop"]["save"])
    img.alpha_composite(both, (40, 64 + lst.height + 6))
    # With the chain under the cursor, its macros wear their order at the left edge.
    numbered = Image.new("RGBA", (width, 40), WIN)
    draw_row(numbered, dict(M4[0], order=1), "macro", 4, 34, width, COLS["pop"]["macro"])
    img.alpha_composite(numbered, (40, 64 + lst.height + 46))
    d = ImageDraw.Draw(img)
    notes = [
        (pos["a"], "Green fill: active. The macro that plays, the drill, or the bound save."),
        (pos["b"], "White border and bar: the cursor. A acts on this row."),
        (pos["c"], "Thin green outline: the hot five, the most recently used first."),
        (pos["d"], "Under the hairline, dimmer: the rest of the library, on the card."),
        ((lst.height + 10, lst.height + 40, None), "Both: the cursor on the bound save, which is green in both views."),
        ((lst.height + 50, lst.height + 80, None), "Red number at the left edge: its place in the chain under the cursor."),
    ]
    for (top, bot, _), text in notes:
        cy = 64 + (top + bot) / 2
        d.line((40 + width + 8, cy, 640, cy), fill=ch.M_RULE, width=1)
        iv.text(d, (650, cy), text, fnt(14), fill=ch.M_TEXT, anchor="lm")
    rows_of_items = [
        [("floppy", "the save state it starts at, grey when Off"), ("list", "a chain, and how many macros it plays"),
         ("badge", "order, at the row's left edge"), ("loop", "loop (macros only)")],
    ]
    for line, items in enumerate(rows_of_items):
        y = 64 + lst.height + 118 + line * 36
        x = 40
        for kind, label in items:
            if kind == "floppy":
                icon(img, "save", x, y - 11, 22, TYPES["save"][0])
                icon(img, "save", x + 26, y - 11, 22, DIM)
                x += 26
            elif kind == "list":
                icon(img, "format_list_numbered", x, y - 11, 22, ch.M_TEXT)
            elif kind == "badge":
                badge(ImageDraw.Draw(img), x + 11, y, 2)
            else:
                loop4(img, x, y - 14, "on", 26)
            d = ImageDraw.Draw(img)
            iv.text(d, (x + 30, y), label, fnt(14), fill=ch.M_GROUP, anchor="lm")
            x += 34 + d.textlength(label, font=fnt(14)) + 30
    return img


def loop_picture():
    img = Image.new("RGBA", (W, 300), WIN)
    d = ImageDraw.Draw(img)
    iv.text(d, (40, 30), "The loop mark (Material Symbols \"repeat\"). X cycles ∞, 1, 2, 3, 5, 10.", fnt(20, bold=True),
            fill=ch.M_TEXT, anchor="lm")
    for i, (mode, label) in enumerate((("off", "off"), ("on", "∞"), ("1", "1"), ("2", "2"), ("3", "3"),
                                       ("5", "5"), ("10", "10"))):
        x = 60 + i * 168
        loop4(img, x, 70, mode, 56)
        d = ImageDraw.Draw(img)
        iv.text(d, (x + 28, 158), label, fnt(16, bold=True), fill=ch.M_GROUP, anchor="mm")
    iv.text(d, (40, 200), "Where it appears:", fnt(15, bold=True), fill=ch.M_TEXT, anchor="lm")
    iv.text(d, (40, 228), "the MACROS section header, for the role in view (Player and CPU each keep their own count);",
            fnt(15), fill=ch.M_GROUP, anchor="lm")
    iv.text(d, (40, 254), "a chain's Connections panel, the count of the role it is active for: the whole chain repeats. "
            "Never in the SAVE STATES, SEQUENCES or COMBOS headers.", fnt(15), fill=ch.M_GROUP, anchor="lm")
    return img


# ---- Fragments for the step-by-step strips -------------------------------------------------------------------------
PANEL_W, PANEL_H = 381, 330


def frag_list(img, box, groups, focus=None, scroll=None, extra_rows=None, cols=None):
    width = box[2] - box[0]
    lst, pos, _ = render_list(groups, width, cols or COLS["frag"], focus, extra_rows)
    view = box[3] - box[1]
    if lst.height <= view:
        img.alpha_composite(lst, (box[0], box[1]))
        return pos
    if scroll is None:
        scroll = 0
        if focus in pos and pos[focus][1] + 8 > view:
            scroll = min(lst.height - view, pos[focus][1] + 8 - view)
            if scroll + view < lst.height and pos[focus][1] + 8 > scroll + view - ROW_H:
                scroll = min(lst.height - view, scroll + ROW_H)
    # End on the last whole row (and its outline), so no row shows cut at the bottom.
    whole = [bot for k, (top, bot, _) in pos.items() if not k.startswith("__group") and bot <= scroll + view]
    end = min(scroll + view, max(whole) + 3) if whole else scroll + view
    img.alpha_composite(lst.crop((0, scroll, width, end)), (box[0], box[1]))
    sticky_header(img, lst, pos, scroll, box[0], box[1])
    return pos


def one_row(img, x, y, width, row, typ, cols_key="frag"):
    lst = Image.new("RGBA", (width, 36), WIN)
    draw_row(lst, row, typ, 3, 33, width, COLS[cols_key][typ])
    img.alpha_composite(lst, (int(x), int(y)))


def frag_game(img, box, game_img):
    w = box[2] - box[0]
    small = game_img.resize((w, int(w * 9 / 16)), Image.LANCZOS)
    img.alpha_composite(small, (box[0], box[1]))
    return box[1] + small.height


def notice_plate(img, box, title, line):
    """The game's own notice style (the white box the frame shows): a title and one line."""
    d = ImageDraw.Draw(img)
    d.rectangle(box, fill=(255, 255, 255, 250))
    cx = (box[0] + box[2]) / 2
    iv.text(d, (cx, box[1] + 16), title, fnt(13), fill=(40, 40, 40, 255), anchor="mm")
    iv.text(d, (cx, box[1] + 40), line, fnt(15), fill=(20, 20, 20, 255), anchor="mm")


def frag_tiles(img, box, title, tiles, hint):
    d = ImageDraw.Draw(img)
    x0, y0, x1, y1 = box
    iv.text(d, (x0, y0 + 10), title, fnt(14, bold=True), fill=ch.M_TEXT, anchor="lm")
    tw = (x1 - x0 - 10) / 2
    for i, t in enumerate(tiles):
        cx = x0 + (i % 2) * (tw + 10)
        cy = y0 + 28 + (i // 2) * 56
        cur = t.get("cursor")
        d.rounded_rectangle((cx, cy, cx + tw, cy + 48), radius=6, fill=ch.M_FOCUS if cur else (28, 30, 38, 255),
                            outline=WHITE if cur else ch.M_RULE, width=3 if cur else 1)
        nx = cx + 12
        if t.get("number"):
            badge(d, cx + 22, cy + 24, t["number"], 11)
            nx = cx + 40
        iv.text(d, (nx, cy + 16), t["name"], fnt(14), fill=ch.M_TEXT, anchor="lm")
        iv.text(d, (nx, cy + 34), t.get("sub", ""), fnt(12), fill=ch.M_GROUP, anchor="lm")
    keys_line(img, x0, y1 - 10, hint, 13)


def frag_actions(img, box, crumb, rows, focus_i, values=None):
    d = ImageDraw.Draw(img)
    x0, y0, x1, y1 = box
    iv.text(d, (x0, y0 + 10), crumb, fnt(13), fill=ch.M_GROUP, anchor="lm")
    y = y0 + 40
    for i, label in enumerate(rows):
        if i == focus_i:
            d.rectangle((x0, y - 13, x1, y + 13), fill=ch.M_FOCUS)
            d.rectangle((x0, y - 13, x1, y + 13), outline=WHITE, width=2)
            d.rectangle((x0, y - 13, x0 + 5, y + 13), fill=WHITE)
        iv.text(d, (x0 + 16, y), label, fnt(14), fill=ch.M_TEXT, anchor="lm")
        if values and label in values:
            v, kind = values[label]
            if kind == "on":          # a value in the menu's green
                iv.text(d, (x1 - 10, y), v, fnt(13), fill=GREEN, anchor="rm")
            elif kind == "switch":    # a linked file and its On / Off switch, as on the macro's own screen
                name, state = v
                fb = fnt(13, bold=True)
                iv.text(d, (x1 - 10, y), state, fb, fill=GREEN if state == "On" else DIM, anchor="rm")
                nx = x1 - 10 - d.textlength(state, font=fb) - 6
                iv.text(d, (nx, y), name + " ·", fnt(13), fill=ch.M_GROUP, anchor="rm")
                icon(img, "save", nx - d.textlength(name + " ·", font=fnt(13)) - 22, y - 9, 18,
                     TYPES["save"][0] if state == "On" else DIM)
                d = ImageDraw.Draw(img)
            else:
                iv.text(d, (x1 - 10, y), v, fnt(13), fill=ch.M_GROUP, anchor="rm")
        y += 28


def frag_keyboard(img, box, header, typed):
    d = ImageDraw.Draw(img)
    x0, y0, x1, y1 = box
    d.rounded_rectangle(box, radius=8, fill=(226, 229, 235, 255), outline=GOLD, width=2)
    iv.text(d, (x0 + 10, y0 + 12), "SCHEMATIC · the Switch's own keyboard", fnt(11, bold=True),
            fill=(150, 104, 0, 255), anchor="lm")
    iv.text(d, ((x0 + x1) / 2, y0 + 34), header, fnt(15, bold=True), fill=(24, 28, 36, 255), anchor="mm")
    field = (x0 + 20, y0 + 48, x1 - 20, y0 + 76)
    d.rounded_rectangle(field, radius=4, fill=WHITE, outline=(120, 128, 140, 255))
    iv.text(d, (field[0] + 8, (field[1] + field[3]) / 2), typed, fnt(14), fill=(24, 28, 36, 255), anchor="lm")
    key, gap = 20, 4
    for row, count in enumerate((10, 10, 9)):
        indent = (10 - count) * (key + gap) / 2
        for n in range(count):
            kx = x0 + 20 + indent + n * (key + gap)
            ky = y0 + 84 + row * (key + gap)
            d.rounded_rectangle((kx, ky, kx + key, ky + key), radius=3, fill=(248, 249, 251, 255),
                                outline=(170, 176, 188, 255))
    ok = (x1 - 70, y1 - 30, x1 - 14, y1 - 8)
    d.rounded_rectangle(ok, radius=4, fill=(64, 150, 230, 255))
    iv.text(d, ((ok[0] + ok[2]) / 2, (ok[1] + ok[3]) / 2), "OK", fnt(12, bold=True), fill=WHITE, anchor="mm")


def strip_canvas(number, title, subtitle):
    img = Image.new("RGBA", (W, 500), WIN)
    d = ImageDraw.Draw(img)
    iv.text(d, (24, 28), "How a connection is made · %d · %s" % (number, title), fnt(20, bold=True), fill=ch.M_TEXT,
            anchor="lm")
    iv.text(d, (24, 54), subtitle, fnt(14), fill=ch.M_GROUP, anchor="lm")
    return img


def strip_panel(img, i, step_title, caption):
    """Panel i (0..2): the frame, a step header, the caption under it. Returns the content box."""
    d = ImageDraw.Draw(img)
    x = 24 + i * (PANEL_W + 44)
    y = 76
    d.rounded_rectangle((x, y, x + PANEL_W, y + PANEL_H), radius=10, fill=WIN, outline=(70, 76, 92, 255), width=2)
    d.ellipse((x + 10, y + 8, x + 34, y + 32), fill=GREEN)
    iv.text(d, (x + 22, y + 20.5), str(i + 1), fnt(14, bold=True), fill=INK, anchor="mm")
    iv.text(d, (x + 44, y + 20), step_title, fnt(15, bold=True), fill=ch.M_TEXT, anchor="lm")
    d.line((x + 10, y + 40, x + PANEL_W - 10, y + 40), fill=ch.M_RULE, width=1)
    if i < 2:
        ax = x + PANEL_W + 10
        arrow4(d, [(ax, y + PANEL_H / 2), (ax + 26, y + PANEL_H / 2)], GREEN, width=4, head=12)
    cy = y + PANEL_H + 20
    for line in wrap(d, caption, fnt(14), PANEL_W):
        iv.text(d, (x, cy), line, fnt(14), fill=ch.M_TEXT, anchor="lm")
        cy += 20
    return (x + 10, y + 50, x + PANEL_W - 10, y + PANEL_H - 10)


def section_group(title, typ, rows, count=None):
    """One stacked section for a strip, as the revision 6 tabs have them: its own header and count."""
    return {"title": title, "typ": typ, "count": len(rows) if count is None else count, "loop": None,
            "sections": [(None, typ, rows)]}


# The strips' columns for stacked sections. A sequence's list of combos keeps its place, without a header caption
# that would run into "SEQUENCES (n)".
FRAG_STACK = dict(COLS["frag"], conv=[("seq", 162, "")])


def tab_rows(focus, drill):
    """The Combos & Sequences rows (the example data) with a given drill: (sequences, combos)."""
    g = group_only("player", "combo", focus, overrides={"_drill": drill})
    return g["sections"][1][2], g["sections"][0][2]


def group_only(role, typ, focus, overrides=None, keep=None):
    g = [g for g in groups_for(role, focus, overrides) if g["typ"] == typ][0]
    if keep:
        g = dict(g, sections=[(s, t, [r for r in rows if r["key"] in keep]) for s, t, rows in g["sections"]
                              if any(r["key"] in keep for r in rows)])
    return g


def flow_record(game):
    img = strip_canvas(1, "Record → start save", "A recording always writes its own start save and connects to it.")
    b = strip_panel(img, 0, "Record", "In the match, ZR + Down records. The pop-up's role, Player, says whose "
                    "inputs are recorded. ZR + Down again stops.")
    bottom = frag_game(img, b, hide_notice4(game.copy()))
    notice_plate(img, (b[0] + 40, b[1] + 14, b[2] - 40, b[1] + 66), "Recording · Player", "ZR + D-pad Down stops")
    d = ImageDraw.Draw(img)
    iv.text(d, (b[0], bottom + 18), "Record also saves the match as it is now:", fnt(13), fill=ch.M_GROUP, anchor="lm")
    iv.text(d, (b[0], bottom + 38), "that file becomes \"Macro 7 · start\".", fnt(13), fill=ch.M_GROUP, anchor="lm")

    b = strip_panel(img, 1, "Macro 7 is hot", "It stops. Macro 7 joins the hot five at the top, the most recently "
                    "used, with its start save connected, and gets the cursor. The least recently used hot macro, "
                    "Replay 86f, leaves the outline.")
    m7 = dict(key="m7", name="Macro 7", fighter="Pikachu", role="P1", length="3.0 s", start="Macro 7 · start",
              hot=True, new=True)
    g = group_only("player", "macro", "m7")
    rows = [r for r in g["sections"][0][2] if not r.get("chain")]     # strip 2 makes the chain, later
    # The most recently used first: Macro 7 goes to the top, and the least recently used hot macro drops out.
    # The older library below it (the migrated slots) is outside this view.
    hot = [r for r in rows if r.get("hot")] + [r for r in rows if r["key"] == "replay"]
    rows = [dict(m7, cursor=True, active=False)] + hot[:-1] + [dict(hot[-1], hot=False)]
    g = dict(g, sections=[(None, "macro", rows)], loop=[("", "1")], count=len(RECORDINGS) + 1)
    frag_list(img, b, [g])

    b = strip_panel(img, 2, "The connection, both ends", "The Connections panel shows starts from Macro 7 · start. The new save "
                    "(gold outline, a mockup mark for new) is hot in SAVE STATES, at the top.")
    y = links_panel(img, (b[0], b[1], b[2], b[1] + 150), {
        "self": ("macro", "Macro 7"),
        "branches": [{"label": "starts from", "out": True, "nodes": [("save", "Macro 7 · start", None, None, True)],
                      "none": ""}],
        "lines": []}, header=False)
    sg = group_only("player", "save", None)
    srows = sg["sections"][0][2]
    # The most recently used save, so it is first in the hot five (the rest is below this view).
    new_save = dict(key="m7start", name="Macro 7 · start", fighters="Pikachu · Mario", hot=True, new=True,
                    active=False, cursor=False)
    sg = dict(sg, sections=[(None, "save", [new_save, dict(srows[0]), dict(srows[1])])], count=len(S4) + 1)
    frag_list(img, (b[0], y + 8, b[2], b[3]), [sg])
    return img


def flow_chain():
    img = strip_canvas(2, "Make a chain", "A chain is a macro made of macros. A role's Play chord plays them one after "
                       "another.")
    b = strip_panel(img, 0, "New chain", "Recording › Save states & Macros. + New chain is the last row under the "
                    "macros.")
    # MACROS as strip 1 left it: eight recordings, Macro 7 the most recent; Nair loop is Player 1's active macro.
    m7 = dict(key="m7", name="Macro 7", fighter="Pikachu", role="P1", length="3.0 s", start="Macro 7 · start",
              hot=True)
    recs = [r for r in group_only("player", "macro", "newchain")["sections"][0][2] if not r.get("chain")]
    hot = [r for r in recs if r.get("hot")] + [r for r in recs if r["key"] == "replay"]
    rows = [m7] + hot[:-1] + [dict(hot[-1], hot=False)] + [r for r in recs if not r.get("hot") and
                                                              r["key"] != "replay"]
    newrow = dict(key="newchain", name="+ New chain", hot=False, cursor=True, action=True)
    frag_list(img, b, [dict(section_group("MACROS", "macro", rows + [newrow], count=len(rows)), loop=[("", "1")])],
              "newchain")

    b = strip_panel(img, 1, "Number the macros", "The same picker as Character Cycles: A adds the next number, R "
                    "removes one. B is done, then the Switch keyboard names it.")
    frag_tiles(img, b, "New chain", [
        {"name": "Nair loop", "sub": "Recorded on Pikachu", "number": 1},
        {"name": "Dash grab", "sub": "Recorded on Pikachu", "number": 2},
        {"name": "CPU throw", "sub": "Recorded on Mario", "number": 3, "cursor": True},
        {"name": "Macro 6", "sub": "Recorded on Lucina"},
        {"name": "Replay 86f", "sub": "Recorded on Pikachu"},
        {"name": "Macro 7", "sub": "Recorded on Pikachu"},
    ], [("A", "add number"), ("R", "remove"), ("B", "done, then name it")])

    b = strip_panel(img, 2, "The chain", "Named Corner pressure, it shows the chain mark 3 and starts at Nair loop · "
                    "start. Set active for the CPU, on its own screen or with A in the pop-up's CPU view, the CPU's Play "
                    "chord plays it.")
    row = dict(MNAME["cpchain"], new=True, active=False, cursor=False)
    one_row(img, b[0], b[1], b[2] - b[0], row, "macro")
    spec = links_spec("cpchain")
    spec = dict(spec, branches=spec["branches"][1:], lines=[])
    links_panel(img, (b[0], b[1] + 44, b[2], b[3]), spec, header=False)
    return img


def flow_sequence():
    img = strip_canvas(4, "Make a sequence", "A sequence is a numbered list of combos, drilled one after another.")
    b = strip_panel(img, 0, "New sequence", "Recording › Combos & Sequences. New sequence is the last row "
                    "under the sequences.")
    # Revision 6's stacked tab: SEQUENCES on top, COMBOS below. Corner to kill does not exist yet, so SEQUENCES
    # holds two files, then the + New sequence row with the cursor. COMBOS is as strip 3 left it: Nair loop combo,
    # the most recently used, at the top, and Ryu BnB still the drill.
    convs, combos = tab_rows("newconv", "ryu")
    convs = [r for r in convs if r["key"] != "corner"]
    combos = [r for r in combos if r["key"] == "nairc"] + [r for r in combos if r["key"] != "nairc"]
    newrow = dict(key="newconv", name="+ New sequence", seq=[], hot=False, cursor=True, action=True)
    frag_list(img, b, [section_group("SEQUENCES", "conv", convs + [newrow], count=len(convs)),
                       section_group("COMBOS", "combo", combos)], "newconv", cols=FRAG_STACK)

    b = strip_panel(img, 1, "Number the combos", "The same numbered picker, with combos. Ryu BnB is 1, Nair loop "
                    "combo is 2.")
    frag_tiles(img, b, "New sequence", [
        {"name": "Ryu BnB", "sub": "Ryu → Mario", "number": 1},
        {"name": "Nair loop combo", "sub": "Pikachu → Mario", "number": 2, "cursor": True},
        {"name": "Combo 2", "sub": "Pikachu → Mario"},
        {"name": "Combo 4", "sub": "Steve → Pikachu"},
    ], [("A", "add number"), ("R", "remove"), ("B", "done, then name it")])

    b = strip_panel(img, 2, "Name it", "OK on the Switch's own keyboard names it. The new row (gold: new) lists its "
                    "combos, Ryu BnB → Nair loop combo.")
    frag_keyboard(img, (b[0] + 20, b[1], b[2] - 20, b[1] + 168), "Name sequence", "Corner to kill")
    d = ImageDraw.Draw(img)
    arrow4(d, [((b[0] + b[2]) / 2, b[1] + 174), ((b[0] + b[2]) / 2, b[1] + 196)], GREEN, width=3, head=9)
    row = [dict(V4[0], active=False, cursor=False, hot=True)]
    row[0]["new"] = True
    lst = Image.new("RGBA", (b[2] - b[0], 40), WIN)
    draw_row(lst, row[0], "conv", 4, 34, b[2] - b[0], COLS["frag"]["conv"])
    img.alpha_composite(lst, (b[0], b[1] + 204))
    return img


def flow_make_combo(game):
    img = strip_canvas(3, "Make combo from a macro", "A combo made from a macro remembers which macro it came from.")
    b = strip_panel(img, 0, "Make combo", "Recording › Save states & Macros › Nair loop, in the Player view. A on "
                    "Make combo. The first row is the start save it will load.")
    # The macro's own screen (revision 7.2): one Set active row, for the role in view.
    frag_actions(img, b, "Player view  ›  Save states & Macros  ›  Nair loop",
                 ["Start save", "Set active", "Rename", "Crop", "Make combo", "Delete"], 4,
                 {"Start save": (("Nair loop · start", "On"), "switch"), "Set active": ("On · Player 1", "on")})

    b = strip_panel(img, 1, "It plays from the start save", "The menu closes. Nair loop · start loads, Nair loop "
                    "plays once on Player 1, and the strip records the hits.")
    g = hide_notice4(game.copy())
    o = iv.overlay()
    ch.strip(ImageDraw.Draw(o))
    g.alpha_composite(o)
    bottom = frag_game(img, b, g)
    notice_plate(img, (b[0] + 30, b[1] + 92, b[2] - 30, b[1] + 144), "Make combo", "Nair loop · start → Player 1")
    d = ImageDraw.Draw(img)
    iv.text(d, (b[0], bottom + 18), "The frame strip at the top is recording.", fnt(13), fill=ch.M_GROUP, anchor="lm")

    b = strip_panel(img, 2, "The new combo", "Nair loop combo (gold: new) joins the hot five at the top. The drill "
                    "does not change: Ryu BnB is still the drill. Its row says from Nair loop; its Connections panel says "
                    "made from.")
    # The stacked tab with SEQUENCES collapsed to its header. Ryu BnB is the drill in this strip, and the new combo,
    # the most recently used, is at the top of the hot five without becoming the drill.
    convs, combos = tab_rows(None, "ryu")
    combos = [dict(r, new=True) for r in combos if r["key"] == "nairc"] + [r for r in combos if r["key"] != "nairc"]
    made = [r for r in convs if r["key"] != "corner"]     # Corner to kill is made in strip 4, later
    frag_list(img, (b[0], b[1], b[2], b[1] + 152), [section_group("SEQUENCES", "conv", [], count=len(made)),
                                                    section_group("COMBOS", "combo", combos)], cols=FRAG_STACK)
    links_panel(img, (b[0], b[1] + 158, b[2], b[3]), {
        "self": ("combo", "Nair loop combo"),
        "branches": [{"label": "made from", "out": True, "nodes": [("macro", "Nair loop", None, None, False)],
                      "none": ""}],
        "lines": []}, header=False)
    return img


def flow_drill(game, bar_picture):
    img = strip_canvas(5, "Drill a sequence", "One drill at a time: drilling a sequence replaces a drilled combo.")
    b = strip_panel(img, 0, "Pick it", "In the pop-up, the cursor on Corner to kill. A drills it.")
    # The pop-up's stacked tab, as strip 4 left it: SEQUENCES on top, Corner to kill (the newest) first with the
    # cursor, not yet green; COMBOS below, Nair loop combo first and Ryu BnB, the current drill, green.
    convs, combos = tab_rows("corner", "ryu")
    combos = [r for r in combos if r["key"] == "nairc"] + [r for r in combos if r["key"] != "nairc"]
    frag_list(img, (b[0], b[1], b[2], b[1] + 102), [section_group("SEQUENCES", "conv", convs)], "corner",
              cols=FRAG_STACK)
    frag_list(img, (b[0], b[1] + 110, b[2], b[1] + 212), [section_group("COMBOS", "combo", combos)], cols=FRAG_STACK)
    foot = r5_foot("conv", "player")
    keys_line(img, b[0], b[3] - 36, foot[:3], 13)
    keys_line(img, b[0], b[3] - 14, foot[3:], 13)

    b = strip_panel(img, 1, "It is the drill", "The row turns green, and Ryu BnB is no longer the drill. Its Connections "
                    "panel lists the combos in order. The sequence is new, so its streak and best start at 0.")
    convs, combos = tab_rows("corner", "corner")
    one_row(img, b[0], b[1] - 4, b[2] - b[0], [r for r in convs if r["key"] == "corner"][0], "conv")
    one_row(img, b[0], b[1] + 27, b[2] - b[0], [r for r in combos if r["key"] == "ryu"][0], "combo")
    # The Links panel as in the pop-up, with its streak and its Drilled line; the two rows above sit close to fit it.
    # Strip 4 has just made Corner to kill, so it has no streak or best yet.
    spec = links_spec("corner")
    spec["lines"] = [("Streak", "0 · best 0, new", ch.M_TEXT) if line[0] == "Streak" else line
                     for line in spec["lines"]]
    links_panel(img, (b[0], b[1] + 64, b[2], b[3]), spec, header=False)

    b = strip_panel(img, 2, "The bar, at the top", "The helper bar sits under the frame strip, where the combo bar "
                    "is today. Full size below.")
    frag_game(img, b, bar_picture)
    return img


def sequence_bar4(game):
    img = hide_notice4(game.copy())
    o = iv.overlay()
    d = ImageDraw.Draw(o)
    x0, x1, _ = ch.strip(d)
    y0, y1 = 158, 300
    iv.panel(d, (x0, y0, x1, y1))
    cc = TYPES["conv"][0]
    iv.tag(d, (x0 + 8, y0 + 8, x0 + 112, y0 + 26), "SEQUENCE", cc, fnt(11, bold=True), fg=INK)
    iv.text(d, (x0 + 122, y0 + 17), "Corner to kill", fnt(14, bold=True), fill=iv.TEXT, anchor="lm")
    iv.text(d, (x0 + 240, y0 + 17), "combo 2 of 2 · Nair loop combo · 4 links · tolerance ±1f", fnt(11),
            fill=iv.MUTED, anchor="lm")
    iv.text(d, (x1 - 12, y0 + 17), "streak 4 · best 9", fnt(12, bold=True), fill=iv.GOOD, anchor="rm")
    # The sequence: combo 1 landed, combo 2 now.
    cy = y0 + 46
    x = x0 + 10
    for label, state in (("1  Ryu BnB  ✓", "done"), ("2  Nair loop combo", "now")):
        f = fnt(12, bold=True)
        w = d.textlength(label, font=f) + 20
        d.rounded_rectangle((x, cy - 11, x + w, cy + 11), radius=5, fill=(32, 38, 52, 255),
                            outline=iv.GOOD if state == "done" else WHITE, width=2)
        iv.text(d, (x + 10, cy), label, f, fill=iv.GOOD if state == "done" else WHITE, anchor="lm")
        x += w + 8
        if state == "done":
            arrow4(d, [(x, cy), (x + 18, cy)], iv.MUTED, width=2, head=7)
            x += 26
    # This combo's links.
    cw = (x1 - x0 - 16 - 3 * 6) / 4
    for k, (label, state) in enumerate((("L1", "true"), ("L2", "now"), ("L3", "wait"), ("L4", "wait"))):
        cx = x0 + 8 + k * (cw + 6)
        top = y0 + 66
        outline = {"true": iv.GOOD, "now": WHITE}.get(state, ch.M_RULE)
        d.rounded_rectangle((cx, top, cx + cw, top + 40), radius=6, fill=(32, 38, 52, 255), outline=outline, width=2)
        iv.text(d, (cx + 10, top + 20), label, fnt(12, bold=True), fill=iv.MUTED, anchor="lm")
        iv.text(d, (cx + 40, top + 20), "Aerial → Aerial", fnt(11), fill=iv.TEXT, anchor="lm")
        word = {"true": "TRUE", "now": "now"}.get(state, "—")
        col = iv.GOOD if state == "true" else WHITE if state == "now" else iv.MUTED
        iv.text(d, (cx + cw - 10, top + 20), word, fnt(12, bold=True), fill=col, anchor="rm")
    iv.text(d, (x0 + 12, y1 - 16), "Ryu BnB landed · Nair loop combo: link 1 of 4 true", fnt(13, bold=True),
            fill=iv.TEXT, anchor="lm")
    img.alpha_composite(o)
    return img


# ---- One file's own screen (the Recording page's A), with the revision 4 names -------------------------------------
def action_screen4(bg, crumb, rows, help1, help2, values=None, keys=None):
    img = bg.copy()
    d = ImageDraw.Draw(img)
    ch.clear_content(d, list_view=True)
    iv.text(d, (53, 118), crumb, ch.MF, fill=ch.M_TEXT, anchor="lm")
    y = 168
    for label, focus in rows:
        if focus:
            d.rectangle((47, y - 16, 1236, y + 16), fill=ch.M_FOCUS)
            d.rectangle((47, y - 16, 1236, y + 16), outline=WHITE, width=2)
            d.rectangle((47, y - 16, 52, y + 16), fill=WHITE)
        iv.text(d, (70, y), label, ch.MF, fill=ch.M_TEXT, anchor="lm")
        if values and label in values:
            v, kind = values[label]
            if kind == "on":
                iv.text(d, (1210, y), v, ch.MF, fill=GREEN, anchor="rm")
            elif kind == "switch":
                # A linked file and its On / Off switch: the link icon and name grey, the state green (On) or grey.
                name, state = v
                fb = fnt(15, bold=True)
                iv.text(d, (1210, y), state, fb, fill=GREEN if state == "On" else DIM, anchor="rm")
                nx = 1210 - d.textlength(state, font=fb) - 12
                iv.text(d, (nx, y), name + "  ·", ch.MF, fill=ch.M_GROUP, anchor="rm")
                icon(img, "save", nx - d.textlength(name + "  ·", font=ch.MF) - 26, y - 10, 20,
                     TYPES["save"][0] if state == "On" else DIM)
                d = ImageDraw.Draw(img)
            else:
                iv.text(d, (1210, y), v, ch.MF, fill=ch.M_GROUP, anchor="rm")
        y += 36
    ch.help_lines(d, help1, help2)
    d.rectangle((44, 660, 1246, 698), fill=ch.M_PANEL)
    keys_line(img, 53, 678, keys or [("Up/Down", "move"), ("A", "do it"), ("B", "back")], 14)
    return img


# =====================================================================================================================
# Revision 5: two tabs under Player | CPU. Each tab is two sections, side by side, scrolling on their own.
# The recorded fighter is information. Revision 4's drawing (icons, rows, hot outline) is reused.
# =====================================================================================================================
R5_TABS = (("saves", "Save states & Macros"), ("combos", "Combos & Sequences"))


def r5_tail(typ):
    """Extra library rows so one picture can scroll. They are the same kind of file, and only that picture draws them."""
    if typ == "combo":
        # Q5 only: two combos newer than Combo 4 and one older than Combo 2, so the one-list layout has to scroll to
        # reach the sequences. Seven combos: five hot, and the two oldest under the hairline.
        return [dict(key="combo6", name="Combo 6", fighters="Pikachu → Mario", links="3 links", src=None),
                dict(key="combo5", name="Combo 5", fighters="Lucina → Cloud", links="3 links", src=None),
                dict(key="combo1", name="Combo 1", fighters="Steve → Pikachu", links="2 links", src=None)]
    return []


def r5_rows(typ, role, focus, extend=False):
    data = {"macro": M4, "combo": C4, "conv": V4, "save": S4}[typ]
    if extend:
        tail = r5_tail(typ)
        if typ == "combo":
            # The most recently used first: Ryu BnB, Nair loop combo, Combo 6, Combo 5, Combo 4 | Combo 2, Combo 1.
            data = list(data[:2]) + tail[:2] + list(data[2:]) + tail[2:]
            data = [dict(r, hot=i < 5) for i, r in enumerate(data)]
        else:
            data = list(data) + tail      # the extra slots are the oldest files: under the hairline
    out = []
    for src in data:
        r = dict(src)
        if typ == "macro":
            if role == "page":
                r["active"] = r["key"] in (ACTIVE["player"], ACTIVE["cpu"])
                r["tag"] = "P1" if r["key"] == ACTIVE["player"] else "CPU" if r["key"] == ACTIVE["cpu"] else None
            else:
                r["active"] = r["key"] == ACTIVE.get(role)
        elif typ in ("combo", "conv"):
            r["active"] = r["key"] == DRILL
        else:
            r["active"] = r["key"] == BOUND
        r["cursor"] = r["key"] == focus
        out.append(r)
    return out


def r5_marks(focus):
    """Badges and start-save marks the sections draw for the focused row. A chain numbers its macros; a macro shows
    its number on every chain that plays it; a macro and the save it starts from mark each other."""
    order, link = {}, set()
    if focus in MNAME:
        m = MNAME[focus]
        for s in S4:
            if m["start"] and s["name"] == m["start"]:
                link.add(s["key"])
        if m.get("chain"):
            for i, key in enumerate(m["chain"]):
                order[key] = i + 1
        else:
            for c in M4:
                if focus in c.get("chain", []):
                    order[c["key"]] = c["chain"].index(focus) + 1
    elif any(c["key"] == focus for c in C4):
        name = next(c["name"] for c in C4 if c["key"] == focus)
        for v in V4:
            if name in v["seq"]:
                order[v["key"]] = v["seq"].index(name) + 1
    elif any(v["key"] == focus for v in V4):
        seq = next(v["seq"] for v in V4 if v["key"] == focus)
        for c in C4:
            if c["name"] in seq:
                order[c["key"]] = seq.index(c["name"]) + 1
    else:
        s = next(x for x in S4 if x["key"] == focus)
        for m in M4:
            if m["start"] and m["start"] == s["name"]:
                link.add(m["key"])
    return order, link


def r5_section(title, typ, rows, width, loop=None, section_on=False, order=None, link=None):
    """One section: header, hot outline, hairline, library. Returns the image and each row's top."""
    order = order or {}
    link = link or set()
    row_h = 40
    height = HEAD_H + 8 + row_h * len(rows) + 40
    img = Image.new("RGBA", (width, height), WIN)
    d = ImageDraw.Draw(img)
    d.rounded_rectangle((0, 2, width - 1, HEAD_H - 2), radius=5, fill=HEAD_FOCUS if section_on else HEAD_BG)
    cy = HEAD_H / 2
    head_mark(img, typ, 14, cy, 12)
    d = ImageDraw.Draw(img)
    ft = fnt(13, bold=True)
    iv.text(d, (28, cy), title, ft, fill=ch.M_TEXT if section_on else ch.M_GROUP, anchor="lm")
    iv.text(d, (28 + d.textlength(title, font=ft) + 8, cy), "(%d)" % len(rows), fnt(12), fill=DIM, anchor="lm")
    if loop:
        lx = width - 20
        for label, mode in reversed(loop):
            lx -= 26
            loop4(img, lx, 5, mode, 24)
            d = ImageDraw.Draw(img)
            if label:
                lx -= 4
                iv.text(d, (lx, cy), label, fnt(11, bold=True), fill=ch.M_GROUP, anchor="rm")
                lx -= d.textlength(label, font=fnt(11, bold=True)) + 8
    y = HEAD_H + 2
    pos = {}
    hot = [r for r in rows if r.get("hot")]
    first = y
    slot = any(r["key"] in order for r in rows)
    for i, r in enumerate(rows):
        if hot and i == len(hot):
            d.line((8, y + 4, width - 8, y + 4), fill=ch.M_RULE, width=1)
            y += 10
        top, bot = y, y + row_h
        r5_row(img, r, typ, top, bot, width, order.get(r["key"]), r["key"] in link, slot)
        d = ImageDraw.Draw(img)
        pos[r["key"]] = top
        y = bot
        if hot and i == len(hot) - 1:
            d.rounded_rectangle((2, first - 1, width - 3, bot + 1), radius=6, outline=GREEN, width=2)
    return img.crop((0, 0, width, y + 4)), pos


def r5_row(img, r, typ, top, bot, width, order_n, link_mark, slot=False):
    """One two-line row. Revision 6: the red order badge sits at the left edge (slot reserves that
    place on every row of a section that shows badges, so the names line up)."""
    d = ImageDraw.Draw(img)
    box = (4, top + 1, width - 5, bot - 1)
    if r.get("active"):
        d.rectangle(box, fill=GREEN_ROW)
    elif r.get("cursor"):
        d.rectangle(box, fill=ch.M_FOCUS)
    if r.get("cursor"):
        d.rectangle(box, outline=WHITE, width=2)
        d.rectangle((box[0], box[1], box[0] + 4, box[3]), fill=WHITE)
    name_y = top + 12
    sub_y = top + 28
    lx = 22 if slot else 0
    if order_n:
        badge(d, 18, (top + bot) / 2, order_n, 9)
    hot = r.get("hot")
    ink = ch.M_TEXT if hot else ch.M_GROUP
    muted = ch.M_GROUP if hot else DIM
    nx = 30 + lx
    iv.text(d, (nx, name_y), r["name"], fnt(13), fill=ink, anchor="lm")
    right = width - 12
    if typ == "macro":
        iv.text(d, (nx, sub_y), "Recorded on " + r["fighter"], fnt(11), fill=muted, anchor="lm")
        if r.get("chain"):     # a chain: the chain mark and how many macros it plays
            iv.text(d, (right, name_y), str(len(r["chain"])), fnt(12, bold=True), fill=ch.M_TEXT if hot else DIM,
                    anchor="rm")
            icon(img, "format_list_numbered", right - 30, name_y - 8, 16, ch.M_TEXT if hot else DIM)
            d = ImageDraw.Draw(img)
        if r.get("start"):
            # The floppy chip before the start save's name: orange while the Start save switch is On, grey when Off.
            on = (hot or link_mark) and r.get("start_on", True)
            iv.text(d, (right, sub_y), r["start"], fnt(11), fill=muted, anchor="rm")
            icon(img, "save", right - d.textlength(r["start"], font=fnt(11)) - 18, sub_y - 8, 15,
                 TYPES["save"][0] if on else DIM)
            d = ImageDraw.Draw(img)
    elif typ == "save":
        iv.text(d, (nx, sub_y), r["fighters"], fnt(11), fill=muted, anchor="lm")
        if link_mark:     # the macro under the cursor starts from this save
            iv.text(d, (right, sub_y), "start save", fnt(11), fill=TYPES["save"][0], anchor="rm")
            icon(img, "save", right - d.textlength("start save", font=fnt(11)) - 18, sub_y - 8, 15, TYPES["save"][0])
            d = ImageDraw.Draw(img)
    elif typ == "combo":
        iv.text(d, (nx, sub_y), "Recorded on " + r["fighters"], fnt(11), fill=muted, anchor="lm")
    elif typ == "conv":
        iv.text(d, (nx, sub_y), " → ".join(r["seq"]), fnt(11), fill=muted, anchor="lm")


def r5_paste_sticky(img, section, box, scroll, header, positions, to_end=False):
    """Header stays. Rows below it move by whole rows. A row that would be cut off is left out. With to_end, the
    section is scrolled to its end: the first row is the earliest one from which every later row still fits."""
    x0, y0, x1, y1 = box
    header = min(int(header), section.height)
    img.alpha_composite(section.crop((0, 0, section.width, header)), (x0, y0))
    view = max(0, y1 - (y0 + header))
    content = section.height - header
    max_scroll = max(0, content - view)
    tops = sorted(p - header for p in positions.values())
    if to_end:
        last_bot = tops[-1] + 40
        scroll = next(top for top in tops if last_bot - top <= view)
    else:
        snapped = 0
        for top in tops:
            if top <= scroll and top <= max_scroll:
                snapped = top
        scroll = snapped
    show = 0
    for top in tops:
        if top < scroll:
            continue
        bot = top + 40
        if bot <= scroll + view:
            show = bot - scroll
        else:
            break
    if content <= view:
        show = content   # the whole section fits: keep the hot outline's bottom edge too
    if show > 0:
        src = header + scroll
        img.alpha_composite(section.crop((0, src, section.width, src + show)), (x0, y0 + header))
    if content > view:
        d = ImageDraw.Draw(img)
        track_top = y0 + header + 2
        track_bot = y1 - 2
        track_h = max(1, track_bot - track_top)
        d.rounded_rectangle((x1 - 6, track_top, x1 - 2, track_bot), radius=2, fill=(40, 43, 52, 255))
        thumb = max(18, track_h * view / content)
        span = max(1, content - view)
        ty = track_top + (track_h - thumb) * (1 if to_end else scroll / span)
        d.rounded_rectangle((x1 - 6, ty, x1 - 2, ty + thumb), radius=2, fill=(150, 156, 170, 255))
    return scroll


def r5_tabs(d, x, y, tab, role):
    """The two tabs. CPU draws only Save states & Macros: no greyed second tab and no text."""
    labels = R5_TABS if role != "cpu" else R5_TABS[:1]
    for key, label in labels:
        f = fnt(14, bold=True)
        w = d.textlength(label, font=f) + 28
        on = key == tab
        if on:
            d.rounded_rectangle((x, y, x + w, y + 28), radius=5, fill=ch.M_FOCUS, outline=GREEN, width=2)
        else:
            d.rounded_rectangle((x, y, x + w, y + 28), radius=5, outline=ch.M_RULE, width=1)
        iv.text(d, (x + 14, y + 14), label, f, fill=ch.M_TEXT if on else ch.M_GROUP, anchor="lm")
        x += w + 10
    return x


def r5_pair(role, tab):
    """(left typ, right typ) for a tab. CPU has only the saves tab."""
    if role == "cpu" or tab == "saves":
        return ("save", "macro")
    return ("conv", "combo")


def r5_loop_for(typ, role):
    if typ != "macro":
        return None
    if role == "page":
        return [("P1", LOOP["player"]), ("CPU", LOOP["cpu"])]
    return [("", LOOP[role])]


def r5_columns(role, tab, focus, width=470, extend=False):
    left_t, right_t = r5_pair(role, tab)
    order, link = r5_marks(focus)
    sections = {}
    positions = {}
    for typ in (left_t, right_t):
        title = {"save": "SAVE STATES", "macro": "MACROS", "conv": "SEQUENCES", "combo": "COMBOS"}[typ]
        img, pos = r5_section(title, typ, r5_rows(typ, role, focus, extend), width, r5_loop_for(typ, role),
                              section_on=any(r["cursor"] for r in r5_rows(typ, role, focus)),
                              order=order, link=link)
        sections[typ] = img
        positions[typ] = pos
    return left_t, right_t, sections, positions


def r5_foot(typ, role="player"):
    common = [("Left/Right", "section"), ("L/R", "role"), ("ZL+Up/Down", "page"), ("B", "close")]
    if role != "cpu":
        common.insert(1, ("ZL/ZR", "tab"))
    if typ == "macro":
        return [("A", "set active"), ("X", "loop")] + common
    if typ == "save":
        return [("A", "bind")] + common
    return [("A", "drill")] + common


# ---- Revision 6: layout B. The two sections of a tab stacked, each scrolling on its own by whole rows with its header
# kept, and revision 4's Links panel on the right for the row under the cursor. --------------------------------------
UPPER_BOX = (148, 158, 700, 360)
LOWER_BOX = (148, 368, 700, 560)
LINKS_BOX = (732, 164, 1130, 548)


def popup_chrome6(game, role, tab):
    """The window: the title, L / Player | CPU / R, and the tabs (CPU has only Save states & Macros)."""
    img = hide_notice4(game.copy())
    d = ImageDraw.Draw(img)
    d.rounded_rectangle(POP, radius=12, fill=WIN, outline=(86, 92, 108, 255), width=2)
    iv.text(d, (152, 98), "Recording", fnt(18, bold=True), fill=ch.M_TEXT, anchor="lm")
    role_toggle(d, 820, 82, "cpu" if role == "cpu" else "player", 15)
    r5_tabs(d, 152, 122, "saves" if role == "cpu" else tab, role)
    return img


def links_and_foot6(img, focus, role, keys):
    d = ImageDraw.Draw(img)
    d.line((716, 158, 716, 560), fill=ch.M_DIVIDER, width=2)
    links_panel(img, LINKS_BOX, links_spec(focus, role))
    d = ImageDraw.Draw(img)
    d.line((144, 604, 1136, 604), fill=ch.M_RULE, width=1)
    keys_line(img, 152, 624, keys, 13)


def popup6(game, role, tab, focus, scrolls=(0, 0), extend=False):
    img = popup_chrome6(game, role, tab)
    upper, lower, sections, positions = r5_columns(role, tab, focus, 540, extend)
    for typ, box, scroll in ((upper, UPPER_BOX, scrolls[0]), (lower, LOWER_BOX, scrolls[1])):
        if scroll == "end":   # scrolled to the end of the section
            r5_paste_sticky(img, sections[typ], box, 0, HEAD_H + 2, positions[typ], to_end=True)
            continue
        if scroll < 0:   # scrolled to the top of the library, the first row under the hairline
            rows = r5_rows(typ, role, focus, extend)
            scroll = next(positions[typ][r["key"]] - (HEAD_H + 2) for r in rows if not r.get("hot"))
        r5_paste_sticky(img, sections[typ], box, scroll, HEAD_H + 2, positions[typ])
    focus_typ = upper if any(r["cursor"] for r in r5_rows(upper, role, focus)) else lower
    links_and_foot6(img, focus, role, r5_foot(focus_typ, role))
    return img


def prompt5(game):
    """The shared Fighter Mismatch screen (macro_panel::fighter). Named and clip origins use this footer."""
    img = hide_notice4(game.copy())
    d = ImageDraw.Draw(img)
    x0, y0, x1, y1 = 360, 168, 920, 392
    d.rounded_rectangle((x0, y0, x1, y1), radius=10, fill=WIN, outline=(86, 92, 108, 255), width=2)
    iv.text(d, (x0 + 24, y0 + 28), "Fighter Mismatch", fnt(18, bold=True), fill=ch.M_TEXT, anchor="lm")
    rows = ("Keep current fighter", "Switch to recorded fighter")
    for i, label in enumerate(rows):
        top = y0 + 58 + i * 44
        box = (x0 + 16, top, x1 - 16, top + 36)
        if i == 0:
            d.rectangle(box, fill=ch.M_FOCUS)
            d.rectangle(box, outline=WHITE, width=2)
            d.rectangle((box[0], box[1], box[0] + 4, box[3]), fill=WHITE)
        iv.text(d, (x0 + 36, top + 18), label, fnt(16), fill=ch.M_TEXT, anchor="lm")
    d.line((x0 + 16, y1 - 78, x1 - 16, y1 - 78), fill=ch.M_RULE, width=1)
    iv.text(d, (x0 + 24, y1 - 52), "Recorded Pikachu · current CPU Villager", fnt(14), fill=ch.M_TEXT, anchor="lm")
    iv.text(d, (x0 + 24, y1 - 28), "Inputs are the same either way. A chooses, B cancels.", fnt(14), fill=ch.M_GROUP, anchor="lm")
    return img


def page_role_toggle(d, x, y, role):
    """The Recording page's role toggle, drawn as the pop-up's: Player | CPU, the role in view in the menu's red. ZR
    switches it, as in Character Cycles, because L / R change menu pages here."""
    f = fnt(15, bold=True)
    w = 112
    d.rounded_rectangle((x - 4, y - 4, x + 2 * w + 4, y + 32), radius=6, outline=ch.M_RULE, width=1)
    for i, (label, key) in enumerate((("Player", "player"), ("CPU", "cpu"))):
        sx = x + i * w
        if key == role:
            d.rectangle((sx, y, sx + w, y + 28), fill=TAB_RED)
        iv.text(d, (sx + w / 2, y + 14), label, f, fill=ch.M_TEXT if key == role else ch.M_GROUP, anchor="mm")
    keycap(d, x + 2 * w + 16, y + 14, "ZR", 13)


def page6(menu, selected, tab, focus, help1, help2, role="player"):
    """The Recording page with layout B, one role per view (revision 7.1): the role toggle on top, the two sections
    stacked in the content area, and the Links panel beside them. The CPU view has only Save states & Macros."""
    img = menu.copy()
    d = ImageDraw.Draw(img)
    ch.clear_content(d)
    groups = [("Save states & Macros", len(S4) + len(M4))]
    if role != "cpu":
        groups.append(("Combos & Sequences", len(C4) + len(V4)))
    groups += [("Replays", 2), ("Options", 5)]
    for i, (name, count) in enumerate(groups):
        y = ch.ROW_Y0 + i * ch.GROUP_STEP
        if i == selected:
            d.rectangle((33, y - 18, 307, y + 18), fill=ch.M_FOCUS)
        iv.text(d, (44, y), "%s  (%d)" % (name, count), fnt(14),
                fill=ch.M_TEXT if i == selected else ch.M_GROUP, anchor="lm")
    page_role_toggle(d, 334, 138, role)
    upper, lower, sections, positions = r5_columns(role, tab, focus, 490)
    r5_paste_sticky(img, sections[upper], (330, 180, 820, 366), 0, HEAD_H + 2, positions[upper])
    r5_paste_sticky(img, sections[lower], (330, 374, 820, 560), 0, HEAD_H + 2, positions[lower])
    d = ImageDraw.Draw(img)
    d.line((834, 132, 834, 556), fill=ch.M_DIVIDER, width=2)
    links_panel(img, (848, 136, 1236, 556), links_spec(focus, role))
    d = ImageDraw.Draw(img)
    ch.help_lines(d, help1, help2)
    d.rectangle((44, 660, 1246, 698), fill=ch.M_PANEL)
    focus_typ = upper if any(r["key"] == focus for r in r5_rows(upper, role, focus)) else lower
    # On the menu page, L / R still change the page, ZR changes the role, and the left column is the only tab list.
    foot = [p for p in r5_foot(focus_typ, role) if p[0] not in ("ZL/ZR", "L/R")]
    at = [k for k, p in enumerate(foot) if p[0] == "ZL+Up/Down"][0]
    foot[at:at] = [("ZR", "role"), ("L/R", "page")]
    # On the page, A opens the file's own screen (revision 7.2); binding, set active and drill are done there.
    foot[0] = ("A", "open")
    keys_line(img, 53, 678, foot, 13)
    return img


def macro_actions6(menu):
    """A macro's own screen, with the Start save switch (On by default) and one Set active row, for the role in view
    (revision 7.2)."""
    return action_screen4(menu, "Recording (Player view)  >  Save states & Macros  >  Nair loop", [
        ("Start save", True), ("Set active", False), ("Rename", False), ("Crop", False), ("Make combo", False),
        ("Delete", False),
    ], "Start save On: its Play chord warps to Nair loop · start once, then plays. Off: it plays from where the "
        "fighters are.",
        "Set active is for the role in view, here Player 1. In a chain, only the chain's start save is used.",
        {"Start save": (("Nair loop · start", "On"), "switch"), "Set active": ("On · Player 1", "on")},
        [("Up/Down", "move"), ("Left/Right", "On / Off"), ("A", "do it"), ("B", "back")])


def chain_actions8(menu):
    """A chain's own screen, in the CPU view (revision 8): a chain is a macro made of macros, set active for a role
    like any macro. No Crop. The same picture answers Q2."""
    return action_screen4(menu, "Recording (CPU view)  >  Save states & Macros  >  Corner pressure", [
        ("Start save", False), ("Set active", True), ("Edit chain", False), ("Rename", False), ("Make combo", False),
        ("Delete", False),
    ], "Set active On · CPU: the CPU's Play chord plays this chain, 1, 2, 3. Start save On: it warps there once, first.",
        "The start save is macro 1's, Nair loop · start. Edit chain is the numbered picker. A chain has no Crop.",
        {"Start save": (("Nair loop · start", "On"), "switch"), "Set active": ("On · CPU", "on"),
         "Edit chain": ("1 Nair loop  ·  2 Dash grab  ·  3 CPU throw  >", "text")},
        [("Up/Down", "move"), ("Left/Right", "On / Off"), ("A", "do it"), ("B", "back")])


def main_r8(menu, game):
    P = "RECORDING-REDESIGN-r8-"
    # A save under the cursor, in the Player view: Nair loop · start, the start save of Nair loop and of the chain.
    # Corner, the bound save, is green.
    save(popup6(game, "player", "saves", "nairstart"), P + "popup-save")
    # Same window and the same files. MACROS has scrolled to its end; SAVE STATES has not. Headers stay, rows are whole.
    save(popup6(game, "player", "saves", "nairstart", scrolls=(0, "end")), P + "popup-scrolled")
    save(popup6(game, "player", "saves", "nair"), P + "popup-macro")
    save(popup6(game, "player", "combos", "corner"), P + "popup-sequence")
    save(popup6(game, "player", "combos", "nairc"), P + "popup-combo")
    # The CPU view, with the cursor on its active macro, the chain Corner pressure.
    save(popup6(game, "cpu", "saves", "cpchain"), P + "popup-cpu")
    save(prompt5(game), "RECORDING-REDESIGN-r5-switch-keep")
    # One role per view: the Player view of both groups, and the CPU view, which has only one.
    save(page6(menu, 0, "saves", "pressure",
               "Player view: save states on top, macros below. Nair loop, green, is Player 1's active macro.",
               "Corner is the bound save, green in both views. Load only warps there. ZR: CPU view."),
         P + "page-saves-macros")
    save(page6(menu, 1, "combos", "corner",
               "Player view: sequences on top, combos below. Corner to kill numbers its combos.",
               "Ryu and Pikachu in one sequence is allowed. Recorded on is information."),
         P + "page-combos")
    save(page6(menu, 0, "saves", "cputhrow",
               "CPU view: Save states & Macros only. The chain Corner pressure, green, is the CPU's active macro.",
               "CPU throw is its macro 3. Corner is green here too: one bound save. ZR switches back.",
               role="cpu"),
         P + "page-cpu")
    save(macro_actions6(menu), P + "macro-actions")
    save(chain_actions8(menu), P + "chain-actions")


# =====================================================================================================================
# Revision 5.2: one picture per open question, A and B side by side. Revisions 4 and 5's pieces, drawn at a size where
# no text is under 14 px. The layout questions (Q5, Q10, Q11) are 1920 wide, so the whole window fits.
# =====================================================================================================================
Q52 = {
    1: "How you number a chain or a sequence",
    2: "Which role plays a chain",
    3: "What loop does with a chain",
    4: "When Switch / Keep appears for a chain",
    5: "How the two lists sit in the Combos & Sequences tab",
    6: "Hot sequences",
    7: "What the sequence file stores",
    8: "A drop, and what completes the sequence",
    9: "One drill at a time",
    10: "Side by side, or stacked",
    11: "Which buttons move",
    12: "Where a chain starts",
    13: "Can a chain contain a chain",
}
ROLE_FILL = {"P1": RED, "CPU": (58, 118, 226, 255)}
EDGE52 = (70, 76, 92, 255)
CHIP_GREY = (88, 94, 108, 255)
WIN_EDGE = (86, 92, 108, 255)
WIDE = 1920
STACK_CROP = (128, 72, 1153, 649)      # the stacked pop-up's window (POP), its outline included


def q_canvas(n, width, content_h, cap_a, cap_b, pad=None, note=None, label_a="A · decided"):
    """The title bar 'QN · question' (and an example line on its right, when given), then two panels: A · decided
    (green chip) and B · not chosen (grey chip). Each panel ends in its caption. Returns the image and the two
    content boxes."""
    wide = width > W
    margin, gap, pad0 = (12, 12, 8) if wide else (24, 16, 16)
    pad = pad0 if pad is None else pad
    pw = (width - 2 * margin - gap) // 2
    probe = ImageDraw.Draw(Image.new("RGBA", (4, 4)))
    fcap = fnt(15)
    caps = [wrap(probe, c, fcap, pw - 2 * pad - 8) for c in (cap_a, cap_b)]
    cap_h = 16 + 21 * max(len(c) for c in caps)
    top, head = 70, 52
    ph = head + content_h + 14 + cap_h + 4
    img = Image.new("RGBA", (width, top + ph + 14), WIN)
    d = ImageDraw.Draw(img)
    d.rounded_rectangle((margin, 12, width - margin, 56), radius=8, fill=HEAD_BG)
    iv.text(d, (margin + 18, 34), "Q%d · %s" % (n, Q52[n]), fnt(22, bold=True), fill=ch.M_TEXT, anchor="lm")
    if note:
        iv.text(d, (width - margin - 18, 34), note, fnt(15), fill=ch.M_GROUP, anchor="rm")
    boxes = []
    for i, (label, rec) in enumerate(((label_a, True), ("B · not chosen", False))):
        x = margin + i * (pw + gap)
        d.rounded_rectangle((x, top, x + pw, top + ph), radius=10, fill=WIN, outline=EDGE52, width=2)
        f = fnt(15, bold=True)
        cw = d.textlength(label, font=f) + 28
        d.rounded_rectangle((x + 14, top + 12, x + 14 + cw, top + 40), radius=14, fill=GREEN if rec else CHIP_GREY)
        iv.text(d, (x + 14 + cw / 2, top + 26.5), label, f, fill=INK if rec else ch.M_TEXT, anchor="mm")
        rule = top + head + content_h + 14
        d.line((x + 12, rule, x + pw - 12, rule), fill=ch.M_RULE, width=1)
        y = rule + 18
        for line in caps[i]:
            iv.text(d, (x + pad + 2, y), line, fcap, fill=ch.M_TEXT, anchor="lm")
            y += 21
        boxes.append((x + pad, top + head, x + pw - pad, top + head + content_h))
    return img, boxes


def keycap(d, x, cy, key, size=15):
    """A button name in a key box, like the L and R boxes beside the role toggle."""
    f = fnt(size, bold=True)
    w = d.textlength(key, font=f) + 14
    d.rounded_rectangle((x, cy - size + 2, x + w, cy + size - 2), radius=5, outline=ch.M_GROUP, width=1)
    iv.text(d, (x + w / 2, cy), key, f, fill=ch.M_TEXT, anchor="mm")
    return x + w


def role_chip(d, x, cy, role):
    """P1 red, CPU blue."""
    f = fnt(15, bold=True)
    w = d.textlength(role, font=f) + 24
    d.rounded_rectangle((x, cy - 14, x + w, cy + 14), radius=14, fill=ROLE_FILL[role])
    iv.text(d, (x + w / 2, cy + 0.5), role, f, fill=WHITE, anchor="mm")
    return x + w


def qnode(img, box, typ, name, badge_n=None, suffix=None, link=False, fsize=16, width=2):
    """node4 with a larger badge and type pill, so nothing in it is under 14 px."""
    d = ImageDraw.Draw(img)
    col = TYPES[typ][0]
    x0, y0, x1, y1 = box
    d.rounded_rectangle(box, radius=6, fill=NODE_BG, outline=col, width=width)
    cy = (y0 + y1) / 2
    x = x0 + 8
    if badge_n is not None:
        badge(d, x + 12, cy, badge_n, 12)
        x += 30
    x = pill(d, x, cy, typ, 14) + 10
    if link:
        icon(img, "save", x, cy - 10, 20, col)
        d = ImageDraw.Draw(img)
        x += 26
    f = fnt(fsize)
    iv.text(d, (x, cy), name, f, fill=ch.M_TEXT, anchor="lm")
    if suffix:
        x += d.textlength(name, font=f) + 8
        iv.text(d, (x, cy), suffix, fnt(fsize - 1), fill=ch.M_GROUP, anchor="lm")


def seq52(img, x, y, items, h=48, size=17, pad=24, minw=None, arrow=30):
    """seq_boxes from revision 4, larger. An item is (label, typ, state); a label may have a second line."""
    d = ImageDraw.Draw(img)
    minw = h - 4 if minw is None else minw
    for label, typ, state in items:
        if typ == "arrow":
            arrow4(d, [(x + 2, y + h / 2), (x + arrow - 6, y + h / 2)], ch.M_GROUP, width=2, head=8)
            x += arrow
            continue
        col = {"ok": GREEN, "drop": RED, "ask": GOLD}.get(state, TYPES[typ][0])
        lines = label.split("\n")
        fonts = [fnt(size, bold=True)] + [fnt(max(14, size - 1))] * (len(lines) - 1)
        w = max(max(d.textlength(t, font=f) for t, f in zip(lines, fonts)) + pad, minw)
        d.rounded_rectangle((x, y, x + w, y + h), radius=6, fill=NODE_BG, outline=col, width=2)
        ink = GOLD if state == "ask" else ch.M_TEXT
        step = size + 4
        ty = y + h / 2 - step * (len(lines) - 1) / 2
        for t, f in zip(lines, fonts):
            iv.text(d, (x + w / 2, ty), t, f, fill=ink, anchor="mm")
            ty += step
        x += w + 6
    return x - 6


def darrow(d, a, b, col=GREEN, width=3, head=11):
    arrow4(d, [a, b], col, width=width, head=head)
    arrow4(d, [b, a], col, width=width, head=head)


def big_rows(img, x0, y0, width, title, typ, rows, hot_set=True):
    """r5_section and r5_row at about 1.3 times: the header, a name and a sub-line per row, the hot outline,
    and the library under the hairline. With hot_set False the list is one plain list."""
    d = ImageDraw.Draw(img)
    head, rh = 46, 54
    d.rounded_rectangle((x0, y0 + 2, x0 + width - 1, y0 + head - 2), radius=6, fill=HEAD_FOCUS)
    cy = y0 + head / 2
    head_mark(img, typ, x0 + 18, cy, 16)
    d = ImageDraw.Draw(img)
    ft = fnt(16, bold=True)
    iv.text(d, (x0 + 36, cy), title, ft, fill=ch.M_TEXT, anchor="lm")
    iv.text(d, (x0 + 46 + d.textlength(title, font=ft), cy), "(%d)" % len(rows), fnt(15), fill=DIM, anchor="lm")
    y = y0 + head + 4
    hot = [r for r in rows if r.get("hot")] if hot_set else []
    first = y
    for i, r in enumerate(rows):
        if hot and i == len(hot):
            d.line((x0 + 10, y + 7, x0 + width - 10, y + 7), fill=ch.M_RULE, width=1)
            y += 14
        top, bot = y, y + rh
        box = (x0 + 5, top + 2, x0 + width - 6, bot - 2)
        if r.get("active"):
            d.rectangle(box, fill=GREEN_ROW)
        elif r.get("cursor"):
            d.rectangle(box, fill=ch.M_FOCUS)
        if r.get("cursor"):
            d.rectangle(box, outline=WHITE, width=2)
            d.rectangle((box[0], box[1], box[0] + 5, box[3]), fill=WHITE)
        bright = r.get("hot") or not hot_set
        iv.text(d, (x0 + 40, top + 19), r["name"], fnt(17), fill=ch.M_TEXT if bright else ch.M_GROUP, anchor="lm")
        iv.text(d, (x0 + 40, top + 39), r["sub"], fnt(14), fill=ch.M_GROUP if bright else DIM, anchor="lm")
        y = bot
        if hot and i == len(hot) - 1:
            d.rounded_rectangle((x0 + 2, first - 1, x0 + width - 3, bot + 1), radius=7, outline=GREEN, width=2)
    return y


def rounded(img, radius):
    """The image with its corners cut round, so no game pixels show outside a window's rounded outline."""
    mask = Image.new("L", img.size, 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, img.width - 1, img.height - 1), radius=radius, fill=255)
    out = Image.new("RGBA", img.size, WIN)
    out.paste(img, (0, 0), mask)
    return out


def window_crop(img, crop, width):
    """A pop-up cropped to its window and scaled to the panel."""
    win = rounded(img.crop(crop), 12)
    return win.resize((int(width), int(round(win.height * width / win.width))), Image.LANCZOS)


# ---- Q1 ---------------------------------------------------------------------------------------------------------------
def q1_tiles(img, box):
    d = ImageDraw.Draw(img)
    x0, y0, x1, y1 = box
    iv.text(d, (x0, y0 + 12), "Edit chain · Corner pressure", fnt(16, bold=True), fill=ch.M_TEXT, anchor="lm")
    tiles = [("Nair loop", "Recorded on Pikachu", 1), ("Dash grab", "Recorded on Pikachu", 2), ("CPU throw", "Recorded on Mario", 3),
             ("Macro 6", "Recorded on Lucina", None), ("Macro 7", "Recorded on Pikachu", None),
             ("Replay 86f", "Recorded on Pikachu", None)]
    tw, th = (x1 - x0 - 12) / 2, 62
    for i, (name, sub, num) in enumerate(tiles):
        tx = x0 + (i % 2) * (tw + 12)
        ty = y0 + 36 + (i // 2) * (th + 10)
        cur = name == "Macro 6"
        d.rounded_rectangle((tx, ty, tx + tw, ty + th), radius=6, fill=ch.M_FOCUS if cur else (28, 30, 38, 255),
                            outline=WHITE if cur else ch.M_RULE, width=3 if cur else 1)
        nx = tx + 16
        if num:
            badge(d, tx + 28, ty + th / 2, num, 14)
            nx = tx + 52
        iv.text(d, (nx, ty + 21), name, fnt(17), fill=ch.M_TEXT, anchor="lm")
        iv.text(d, (nx, ty + 43), sub, fnt(14), fill=ch.M_GROUP, anchor="lm")
    keys_line(img, x0, y1 - 12, [("A", "add number"), ("R", "remove number"), ("B", "done")], 15)


def q1_list(img, box):
    d = ImageDraw.Draw(img)
    x0, y0, x1, y1 = box
    iv.text(d, (x0, y0 + 12), "Edit chain · Corner pressure", fnt(16, bold=True), fill=ch.M_TEXT, anchor="lm")
    rows = [("Nair loop", "Recorded on Pikachu"), ("Dash grab", "Recorded on Pikachu"), ("CPU throw", "Recorded on Mario")]
    right = x1 - 84
    y = y0 + 36
    mids = []
    for i, (name, sub) in enumerate(rows):
        top, bot = y, y + 52
        if i == 1:
            d.rectangle((x0, top, right, bot), fill=ch.M_FOCUS)
            d.rectangle((x0, top, right, bot), outline=WHITE, width=2)
            d.rectangle((x0, top, x0 + 5, bot), fill=WHITE)
        badge(d, x0 + 28, (top + bot) / 2, i + 1, 14)
        iv.text(d, (x0 + 52, top + 17), name, fnt(17), fill=ch.M_TEXT, anchor="lm")
        iv.text(d, (x0 + 52, top + 38), sub, fnt(14), fill=ch.M_GROUP, anchor="lm")
        mids.append((top + bot) / 2)
        y += 56
    target = y + 2                       # Y puts Dash grab here, under CPU throw
    y += 14
    d.line((x0 + 8, y, right, y), fill=ch.M_RULE, width=1)
    iv.text(d, (x0 + 52, y + 26), "Add a macro…", fnt(17), fill=ch.M_TEXT, anchor="lm")
    ghost = Image.new("RGBA", img.size, (0, 0, 0, 0))
    g = ImageDraw.Draw(ghost)
    col = (255, 255, 255, 150)
    for gx in range(int(x0 + 52), int(right) - 6, 14):
        g.line((gx, target, gx + 7, target), fill=col, width=2)
    ax = right + 30
    g.line([(right + 6, mids[1]), (ax, mids[1]), (ax, target)], fill=col, width=3, joint="curve")
    arrow4(g, [(ax, target), (right + 8, target)], col, width=3, head=10)
    img.alpha_composite(ghost)
    d = ImageDraw.Draw(img)
    keycap(d, ax + 12, (mids[1] + target) / 2, "Y")
    keys_line(img, x0, y1 - 12, [("A", "add / remove"), ("Y", "move later"), ("B", "done")], 15)


def q1():
    img, (a, b) = q_canvas(1, W, 292,
                           "A on Macro 6 gives it 4. R takes a number off, and the later numbers close up.",
                           "Y on Dash grab moves it below CPU throw. Add a macro… opens a picker for more.")
    q1_tiles(img, a)
    q1_list(img, b)
    return img


# ---- Q3 ---------------------------------------------------------------------------------------------------------------
def q3_panel(img, box, order, groups, label, warp=False):
    d = ImageDraw.Draw(img)
    x0, y0, x1, y1 = box
    # The loop chip, count 2, in the panel's corner.
    iv.text(d, (x1 - 62, y0 - 26), "Loop", fnt(15), fill=ch.M_GROUP, anchor="rm")
    loop4(img, x1 - 58, y0 - 50, "2", 50)
    d = ImageDraw.Draw(img)
    iv.text(d, (x0, y0 + 14), "Chain: 1 Nair loop · 2 Dash grab · 3 CPU throw", fnt(15), fill=ch.M_GROUP, anchor="lm")
    bw = 52
    step = 80 if warp else 90
    lead = bw + 34 if warp else 0
    sx = x0 + (x1 - x0 - (lead + 5 * step + bw)) / 2
    top = y0 + 44
    if warp:
        # Revision 8: the Play chord warps to the start save once, before the first pass.
        sc = TYPES["save"][0]
        d.rounded_rectangle((sx, top, sx + bw, top + bw), radius=7, fill=NODE_BG, outline=sc, width=2)
        icon(img, "save", sx + 11, top + 11, 30, sc)
        d = ImageDraw.Draw(img)
        arrow4(d, [(sx + bw + 7, top + bw / 2), (sx + lead - 7, top + bw / 2)], ch.M_GROUP, width=2, head=8)
        iv.text(d, (sx + bw / 2, top + bw + 36), "warps once", fnt(15), fill=sc, anchor="mm")
        sx += lead
    for i, n in enumerate(order):
        bx = sx + i * step
        d.rounded_rectangle((bx, top, bx + bw, top + bw), radius=7, fill=NODE_BG, outline=TYPES["macro"][0], width=2)
        iv.text(d, (bx + bw / 2, top + bw / 2), str(n), fnt(20, bold=True), fill=ch.M_TEXT, anchor="mm")
        if i < len(order) - 1:
            arrow4(d, [(bx + bw + 7, top + bw / 2), (bx + step - 7, top + bw / 2)], ch.M_GROUP, width=2, head=8)
    if warp:
        # Between the passes: the arrow from 3 back to 1 plays on from where the fighters are.
        mx = sx + 2 * step + bw + (step - bw) / 2
        iv.text(d, (mx, top - 12), "no warp back", fnt(14, bold=True), fill=GREEN, anchor="mm")
    by = top + bw + 12
    for i, j, text in groups:
        xa, xb = sx + i * step, sx + j * step + bw
        d.line([(xa, by), (xa, by + 8), (xb, by + 8), (xb, by)], fill=ch.M_GROUP, width=2)
        iv.text(d, ((xa + xb) / 2, by + 24), text, fnt(15), fill=ch.M_TEXT, anchor="mm")
    iv.text(d, ((x0 + x1) / 2, by + 70), label, fnt(19, bold=True), fill=ch.M_TEXT, anchor="mm")


def q3():
    img, (a, b) = q_canvas(3, W, 200,
                           "Loop 2: the Play chord warps to the start save once, then the whole chain plays twice. No "
                           "pass warps back.",
                           "Loop 2: each macro plays twice before the next number starts.")
    q3_panel(img, a, (1, 2, 3, 1, 2, 3), ((0, 2, "chain, 1st time"), (3, 5, "chain, 2nd time")),
             "the whole chain twice", warp=True)
    q3_panel(img, b, (1, 1, 2, 2, 3, 3), ((0, 1, "Nair loop ×2"), (2, 3, "Dash grab ×2"), (4, 5, "CPU throw ×2")),
             "each macro twice")
    return img


# ---- Q4 ---------------------------------------------------------------------------------------------------------------
def mismatch_prompt(img, box, info, footer, chosen=0, inset=24):
    """The Fighter Mismatch screen as prompt5 draws it, at the same type sizes: the title, Keep / Switch with the
    answer under the cursor, a rule, the recorded lines, then the footer. The wording is macro_panel/fighter.rs's."""
    d = ImageDraw.Draw(img)
    x0, y0, x1, y1 = box
    d.rounded_rectangle(box, radius=10, fill=WIN, outline=WIN_EDGE, width=2)
    iv.text(d, (x0 + inset, y0 + 28), "Fighter Mismatch", fnt(18, bold=True), fill=ch.M_TEXT, anchor="lm")
    row_in = 16 if inset >= 24 else 8
    for i, label in enumerate(("Keep current fighter", "Switch to recorded fighter")):
        top = y0 + 58 + i * 44
        b = (x0 + row_in, top, x1 - row_in, top + 36)
        if i == chosen:
            d.rectangle(b, fill=ch.M_FOCUS)
            d.rectangle(b, outline=WHITE, width=2)
            d.rectangle((b[0], b[1], b[0] + 4, b[3]), fill=WHITE)
        iv.text(d, (x0 + row_in + 20, top + 18), label, fnt(16), fill=ch.M_TEXT, anchor="lm")
    d.line((x0 + row_in, y0 + 146, x1 - row_in, y0 + 146), fill=ch.M_RULE, width=1)
    y = y0 + 172
    for line, colour in [(t, ch.M_TEXT) for t in info] + [(t, ch.M_GROUP) for t in footer]:
        iv.text(d, (x0 + inset, y), line, fnt(14), fill=colour, anchor="lm")
        y += 24


def q4():
    # Revision 8.2: one prompt, only because macro 1 mismatches. Macros 2 and 3 never ask. B stays the rejected
    # per-macro prompts.
    img, (a, b) = q_canvas(4, W, 456,
                           "Asked once, only because macro 1 mismatches. Macros 2 and 3 play on the same character and "
                           "never ask.",
                           "Asked before 1, before 2 and before 3. The chain stops at each prompt, and the answers can "
                           "differ.",
                           pad=12, note="Example: the CPU is Palutena; all three were recorded on other fighters.",
                           label_a="A · decided, amended")
    ask = lambda s: ("Switch or Keep?\n" + s, "save", "ask")
    arr = ("", "arrow", "")
    num = lambda n: (str(n), "macro", "")
    sq = dict(h=54, size=15, pad=20, minw=46, arrow=26)
    footer = "Inputs are the same either way. A chooses, B cancels."
    lines = ["Recorded Pikachu · current CPU Palutena", "Recorded Pikachu · current CPU Palutena",
             "Recorded Mario · current CPU Palutena"]
    x0, y0, x1, _ = a
    seq52(img, x0, y0, [("Play on CPU\nZR + Up", "macro", ""), arr, ask("macro 1"), arr, num(1), arr,
                        num(2), arr, num(3)], **sq)
    d = ImageDraw.Draw(img)
    iv.text(d, (x0, y0 + 84), "The existing screen, for macro 1 only. No list:", fnt(15),
            fill=ch.M_GROUP, anchor="lm")
    mismatch_prompt(img, (x0, y0 + 104, x1, y0 + 328),
                    ["Recorded Pikachu · current CPU Palutena"], [footer])
    iv.text(d, (x0, y0 + 348), "Macros 2 and 3 play on the same character; they never ask.", fnt(15),
            fill=ch.M_TEXT, anchor="lm")
    # B: a prompt before each macro, each with its own line; then the one screen, drawn once.
    x0, y0, x1, _ = b
    step = 70
    for i in range(3):
        y = y0 + i * step
        end = seq52(img, x0, y, [ask("names %d" % (i + 1)), arr, num(i + 1)], **sq)
        d = ImageDraw.Draw(img)
        iv.text(d, (end + 16, y + 27), lines[i], fnt(14), fill=ch.M_TEXT, anchor="lm")
        if i < 2:   # then the next prompt
            nx = end - 23
            ax = x0 + 80
            d.line([(nx, y + 54), (nx, y + 60), (ax, y + 60)], fill=ch.M_GROUP, width=2)
            arrow4(d, [(ax, y + 60), (ax, y + step)], ch.M_GROUP, width=2, head=7)
    ly = y0 + 2 * step + 74
    lead = "The same screen before each macro, "
    iv.text(d, (x0, ly), lead, fnt(15), fill=ch.M_GROUP, anchor="lm")
    iv.text(d, (x0 + d.textlength(lead, font=fnt(15)), ly), "×3, one per macro:", fnt(15, bold=True), fill=GOLD,
            anchor="lm")
    mismatch_prompt(img, (x0, ly + 18, x1, ly + 18 + 224), [lines[0]], [footer])
    return img


# ---- Q5 ---------------------------------------------------------------------------------------------------------------
def popup6_one_list(game, focus="corner"):
    """B. The Combos & Sequences tab as one list: COMBOS, then SEQUENCES, one scroll, with the Links panel. The
    list scrolls by whole rows until the cursor shows, and the header of the section at the top stays put."""
    img = popup_chrome6(game, "player", "combos")
    view_top, view_bottom = UPPER_BOX[1], LOWER_BOX[3]
    width = UPPER_BOX[2] - UPPER_BOX[0] - 12
    order, link = r5_marks(focus)
    sec_c, pos_c = r5_section("COMBOS", "combo", r5_rows("combo", "player", focus, extend=True), width, None, False,
                              order, link)
    sec_v, pos_v = r5_section("SEQUENCES", "conv", r5_rows("conv", "player", focus), width, None, True, order, link)
    col = Image.new("RGBA", (width, sec_c.height + sec_v.height), WIN)
    col.alpha_composite(sec_c, (0, 0))
    col.alpha_composite(sec_v, (0, sec_c.height))
    header = HEAD_H + 2
    view = view_bottom - view_top
    rows = sorted([(t, t + 40) for t in pos_c.values()] + [(sec_c.height + t, sec_c.height + t + 40)
                                                          for t in pos_v.values()])
    blocks = rows + [(sec_c.height, sec_c.height + header)]
    cursor = sec_c.height + pos_v[focus] if focus in pos_v else pos_c[focus]
    # The first whole-row scroll that shows the cursor; once scrolled, COMBOS stays at the top.
    starts = [0] + [t for t, _ in rows if t < sec_c.height]
    for start in starts:
        limit = start + (view if start == 0 else view - header)
        if cursor + 40 <= limit:
            break
    end = max(bt for t, bt in blocks if t >= start and bt <= limit)
    x = UPPER_BOX[0]
    if start == 0:
        img.alpha_composite(col.crop((0, 0, width, end)), (x, view_top))
    else:
        img.alpha_composite(col.crop((0, 0, width, header)), (x, view_top))
        img.alpha_composite(col.crop((0, start, width, end)), (x, view_top + header))
    d = ImageDraw.Draw(img)
    scroll = 0 if start == 0 else start - header
    sx = UPPER_BOX[2] - 6
    track_top, track_bot = view_top + 2, view_bottom - 2
    d.rounded_rectangle((sx, track_top, sx + 4, track_bot), radius=2, fill=(40, 43, 52, 255))
    thumb = max(18, (track_bot - track_top) * view / col.height)
    ty = track_top + (track_bot - track_top - thumb) * scroll / max(1, col.height - view)
    d.rounded_rectangle((sx, ty, sx + 4, ty + thumb), radius=2, fill=(150, 156, 170, 255))
    # One list: Up and Down move through it, so the footer has no Left / Right.
    links_and_foot6(img, focus, "player", [p for p in r5_foot("conv") if p[0] != "Left/Right"])
    return img


def q5(game):
    pw = (WIDE - 36) // 2 - 16
    a_img = window_crop(popup6(game, "player", "combos", "corner", extend=True), STACK_CROP, pw)
    b_img = window_crop(popup6_one_list(game), STACK_CROP, pw)
    img, (a, b) = q_canvas(5, WIDE, a_img.height,
                           "Sequences on top, combos below, each with its own scroll. The Connections panel lists Corner "
                           "to kill's combos in order.",
                           "One list, one scroll. The combos push Corner to kill down, so the list has scrolled and Ryu "
                           "BnB, its combo 1, is off the top.")
    img.alpha_composite(a_img, (a[0], a[1]))
    img.alpha_composite(b_img, (b[0], b[1]))
    return img


# ---- Q6 ---------------------------------------------------------------------------------------------------------------
def q6():
    img, (a, b) = q_canvas(6, W, 438,
                           "Up to five in the green outline, the most recently used first. The rest sit dimmer under "
                           "the hairline.",
                           "One plain list, newest first, with no outline. Only the drilled sequence is "
                           "green.")
    # Six sequences, newest first: Sequence 6, 5, 4, Corner to kill, Platform strings, Sequence 3.
    corner = dict(name="Corner to kill", sub="Ryu BnB → Nair loop combo", hot=True, active=True,
                  cursor=True)
    conv6 = dict(name="Sequence 6", sub="Combo 4 → Ryu BnB", hot=True)
    conv5 = dict(name="Sequence 5", sub="Ryu BnB → Combo 2", hot=True)
    conv4 = dict(name="Sequence 4", sub="Combo 2 → Nair loop combo", hot=True)
    plat = dict(name="Platform strings", sub="Combo 2 → Combo 4", hot=True)
    conv3 = dict(name="Sequence 3", sub="Combo 4", hot=False)
    big_rows(img, a[0], a[1], a[2] - a[0], "SEQUENCES", "conv", [corner, conv6, conv5, conv4, plat, conv3])
    keys_line(img, a[0], a[3] - 12, [("A", "drill"), ("B", "close")], 15)
    big_rows(img, b[0], b[1], b[2] - b[0], "SEQUENCES", "conv", [conv6, conv5, conv4, corner, plat, conv3],
             hot_set=False)
    keys_line(img, b[0], b[3] - 12, [("A", "drill"), ("B", "close")], 15)
    return img


# ---- Q7 ---------------------------------------------------------------------------------------------------------------
def q7_files(img, box, inner):
    """The sequence on the left with two inner rows, the combo files on the right. Returns the rows' centres."""
    d = ImageDraw.Draw(img)
    x0, y0, x1, y1 = box
    vc = TYPES["conv"][0]
    split = x0 + 330
    iv.text(d, (x0, y0 + 10), "Sequence file", fnt(15, bold=True), fill=ch.M_GROUP, anchor="lm")
    iv.text(d, (split, y0 + 10), "Combo files", fnt(15, bold=True), fill=ch.M_GROUP, anchor="lm")
    cb = (x0, y0 + 26, split - 34, y0 + 226)
    d.rounded_rectangle(cb, radius=8, fill=NODE_BG, outline=vc, width=3)
    pill(d, cb[0] + 12, cb[1] + 24, "conv", 14)
    ft = fnt(17, bold=True)
    px = cb[0] + 12 + d.textlength("SEQUENCE", font=fnt(14, bold=True)) + 24
    iv.text(d, (px, cb[1] + 24), "Corner to kill", ft, fill=ch.M_TEXT, anchor="lm")
    centres = []
    for i, (name, links) in enumerate((("Ryu BnB", 6), ("Nair loop combo", 4))):
        top = cb[1] + 50 + i * 72
        ib = (cb[0] + 12, top, cb[2] - 12, top + 64)
        inner(img, ib, i + 1, name, links)
        d = ImageDraw.Draw(img)
        cy = (ib[1] + ib[3]) / 2
        centres.append(cy)
        qnode(img, (split, cy - 22, x1, cy + 22), "combo", name)
        d = ImageDraw.Draw(img)
    return cb, split, centres


def q7_events(img, box, events):
    d = ImageDraw.Draw(img)
    x0, y0, x1, y1 = box
    y = y0 + 254
    for event, result in events:
        iv.text(d, (x0, y), event, fnt(15, bold=True), fill=ch.M_TEXT, anchor="lm")
        iv.text(d, (x0 + 18, y + 23), result, fnt(15), fill=ch.M_TEXT, anchor="lm")
        y += 56


def q7():
    img, (a, b) = q_canvas(7, W, 362,
                           "The sequence keeps a connection to each combo file. A rename or a delete of the combo reaches "
                           "it.",
                           "The sequence keeps a frozen copy of each combo's links. Renames and deletes of the combo "
                           "don't reach it.")
    cc = TYPES["combo"][0]

    def ref(img, ib, n, name, links):
        d = ImageDraw.Draw(img)
        d.rounded_rectangle(ib, radius=6, fill=WIN, outline=ch.M_RULE, width=1)
        cy = ib[1] + 20
        badge(d, ib[0] + 18, cy, n, 12)
        icon(img, "link", ib[0] + 36, cy - 10, 20, cc)
        d = ImageDraw.Draw(img)
        iv.text(d, (ib[0] + 62, cy), name, fnt(16), fill=ch.M_TEXT, anchor="lm")
        iv.text(d, (ib[0] + 62, cy + 24), "a connection to the file", fnt(14), fill=ch.M_GROUP, anchor="lm")

    def copy(img, ib, n, name, links):
        d = ImageDraw.Draw(img)
        d.rounded_rectangle(ib, radius=6, fill=WIN, outline=cc, width=2)
        cy = ib[1] + 18
        badge(d, ib[0] + 18, cy, n, 12)
        iv.text(d, (ib[0] + 38, cy), name, fnt(16), fill=ch.M_TEXT, anchor="lm")
        x = ib[0] + 10
        for k in range(links):
            d.rounded_rectangle((x, cy + 14, x + 34, cy + 38), radius=4, fill=(32, 38, 52, 255), outline=ch.M_RULE)
            iv.text(d, (x + 17, cy + 26), "L%d" % (k + 1), fnt(14, bold=True), fill=ch.M_TEXT, anchor="mm")
            x += 39

    cb, split, centres = q7_files(img, a, ref)
    d = ImageDraw.Draw(img)
    for cy in centres:
        arrow4(d, [(cb[2] - 10, cy), (split - 6, cy)], cc, width=3, head=10)
    q7_events(img, a, (("Rename Nair loop combo → Nair loop v2:", "Corner to kill shows Nair loop v2"),
                       ("Delete Ryu BnB:", "Corner to kill skips that step")))
    q7_files(img, b, copy)
    q7_events(img, b, (("Rename Nair loop combo → Nair loop v2:", "Corner to kill still says Nair loop combo"),
                       ("Delete Ryu BnB:", "Corner to kill still drills its copy")))
    return img


# ---- Q8 ---------------------------------------------------------------------------------------------------------------
def q8_decided():
    """Q8 as decided by the operator, neither A nor B: a drop restarts the sequence at combo 1, and neither the
    sequence nor a combo has to be true. One panel."""
    margin, pad, content_h = 24, 16, 270
    caption = ("A missed press drops combo 2, so the drill restarts at combo 1: streak 0, best kept. Both combos done in "
               "order complete it, even with a gap the CPU could act in. TRUE is shown, but not required.")
    probe = ImageDraw.Draw(Image.new("RGBA", (4, 4)))
    caps = wrap(probe, caption, fnt(15), W - 2 * margin - 2 * pad - 8)
    top, head = 70, 52
    ph = head + content_h + 14 + 16 + 21 * len(caps) + 4
    img = Image.new("RGBA", (W, top + ph + 14), WIN)
    d = ImageDraw.Draw(img)
    d.rounded_rectangle((margin, 12, W - margin, 56), radius=8, fill=HEAD_BG)
    iv.text(d, (margin + 18, 34), "Q8 · %s" % Q52[8], fnt(22, bold=True), fill=ch.M_TEXT, anchor="lm")
    x, x1 = margin, W - margin
    d.rounded_rectangle((x, top, x1, top + ph), radius=10, fill=WIN, outline=EDGE52, width=2)
    f = fnt(15, bold=True)
    cw = d.textlength("Decided", font=f) + 28
    d.rounded_rectangle((x + 14, top + 12, x + 14 + cw, top + 40), radius=14, fill=GREEN)
    iv.text(d, (x + 14 + cw / 2, top + 26.5), "Decided", f, fill=INK, anchor="mm")
    rule = top + head + content_h + 14
    d.line((x + 12, rule, x1 - 12, rule), fill=ch.M_RULE, width=1)
    for k, line in enumerate(caps):
        iv.text(d, (x + pad + 2, rule + 18 + 21 * k), line, fnt(15), fill=ch.M_TEXT, anchor="lm")
    x0, y0 = x + pad, top + head
    # Row 1: a drop, and the restart.
    arr = ("", "arrow", "")
    end = seq52(img, x0, y0, [("1 Ryu BnB ✓", "combo", "ok"), arr, ("2 Nair loop combo ✗\nmissed press", "combo", "drop"),
                              arr, ("restart at 1 Ryu BnB", "combo", "")], h=54, size=16, pad=24, arrow=30)
    d = ImageDraw.Draw(img)
    iv.text(d, (end + 20, y0 + 27), "streak 0, best stays", fnt(16), fill=ch.M_GROUP, anchor="lm")
    # Row 2: what completes it.
    ly = y0 + 104
    fb = fnt(16, bold=True)
    iv.text(d, (x0, ly), "complete =", fb, fill=ch.M_TEXT, anchor="lm")
    iv.text(d, (x0 + d.textlength("complete = ", font=fb), ly), "both combos done in order", fnt(16),
            fill=ch.M_TEXT, anchor="lm")
    lx = x0 + 70
    s1, s2 = (lx, lx + 230), (lx + 360, lx + 640)
    p1y, cpuy = ly + 32, ly + 82
    role_chip(d, x0, p1y + 17, "P1")
    role_chip(d, x0, cpuy + 15, "CPU")
    for (sa, sb), label in ((s1, "1 Ryu BnB ✓"), (s2, "2 Nair loop combo ✓")):
        d.rounded_rectangle((sa, p1y, sb, p1y + 34), radius=6, fill=NODE_BG, outline=GREEN, width=2)
        iv.text(d, ((sa + sb) / 2, p1y + 17), label, fnt(16, bold=True), fill=ch.M_TEXT, anchor="mm")
    stun, idle = iv.STATE["hitstun"], iv.STATE["idle"]
    for sa, sb in (s1, s2):
        d.rectangle((sa, cpuy, sb, cpuy + 30), fill=stun)
        iv.text(d, ((sa + sb) / 2, cpuy + 15), "hitstun", fnt(15, bold=True), fill=WHITE, anchor="mm")
    d.rectangle((s1[1] + 3, cpuy, s2[0] - 3, cpuy + 30), fill=idle)
    mid = (s1[1] + s2[0]) / 2
    d.line((mid, cpuy + 32, mid, cpuy + 42), fill=GREEN, width=2)
    iv.text(d, (mid, cpuy + 56), "CPU could act (up to 20 frames): still counts", fnt(16, bold=True), fill=GREEN, anchor="mm")
    # TRUE stays on the bar as information.
    tx = s2[1] + 40
    iv.tag(d, (tx, p1y + 4, tx + 64, p1y + 30), "TRUE", (110, 116, 130, 255), fnt(14, bold=True), fg=WHITE)
    iv.text(d, (tx + 76, p1y + 17), "shown, not required", fnt(16), fill=ch.M_GROUP, anchor="lm")
    return img


# ---- Q9 ---------------------------------------------------------------------------------------------------------------
def bar52(img, box, kind, name, sub, streak, links, chain=None, alpha=255):
    """sequence_bar4's look, at a size with no text under 14 px: the tag and name, the streak, then the combos of a
    sequence, then this combo's links."""
    o = Image.new("RGBA", img.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(o)
    x0, y0, x1, y1 = box
    iv.panel(d, box)
    ft = fnt(14, bold=True)
    tw = d.textlength(kind, font=ft) + 24
    if kind == "SEQUENCE":
        iv.tag(d, (x0 + 10, y0 + 10, x0 + 10 + tw, y0 + 36), kind, TYPES["conv"][0], ft, fg=INK)
    else:
        iv.tag(d, (x0 + 10, y0 + 10, x0 + 10 + tw, y0 + 36), kind, (124, 92, 214, 255), ft, fg=WHITE)
    iv.text(d, (x0 + 22 + tw, y0 + 23), name, fnt(17, bold=True), fill=iv.TEXT, anchor="lm")
    iv.text(d, (x1 - 12, y0 + 23), streak, fnt(15, bold=True), fill=iv.GOOD, anchor="rm")
    cy = y0 + 58
    iv.text(d, (x0 + 12, cy), sub, fnt(15), fill=iv.MUTED, anchor="lm")
    x = x0 + 22 + d.textlength(sub, font=fnt(15))
    for label, state in chain or ():
        f = fnt(14, bold=True)
        w = d.textlength(label, font=f) + 20
        d.rounded_rectangle((x, cy - 13, x + w, cy + 13), radius=5, fill=(32, 38, 52, 255),
                            outline=iv.GOOD if state == "done" else WHITE, width=2)
        iv.text(d, (x + 10, cy), label, f, fill=iv.GOOD if state == "done" else WHITE, anchor="lm")
        x += w + 6
        if state == "done":
            arrow4(d, [(x, cy), (x + 16, cy)], iv.MUTED, width=2, head=7)
            x += 24
    cw = (x1 - x0 - 20 - 3 * 6) / 4
    for k, state in enumerate(links):
        cx = x0 + 10 + k * (cw + 6)
        top = y0 + 80
        outline = {"true": iv.GOOD, "now": WHITE}.get(state, ch.M_RULE)
        d.rounded_rectangle((cx, top, cx + cw, top + 34), radius=6, fill=(32, 38, 52, 255), outline=outline, width=2)
        iv.text(d, (cx + 12, top + 17), "L%d" % (k + 1), fnt(14, bold=True), fill=iv.MUTED, anchor="lm")
        word = {"true": "TRUE", "now": "now"}.get(state, "—")
        colour = iv.GOOD if state == "true" else WHITE if state == "now" else iv.MUTED
        iv.text(d, (cx + cw - 12, top + 17), word, fnt(14, bold=True), fill=colour, anchor="rm")
    if alpha < 255:
        o.putalpha(o.getchannel("A").point(lambda v: v * alpha // 255))
    img.alpha_composite(o)


def q9():
    img, (a, b) = q_canvas(9, W, 312,
                           "One drill, one bar, one streak. The bar always shows what you are drilling.",
                           "Both drills stay active, each with its own streak. A button flips which bar you see.")
    conv = dict(kind="SEQUENCE", name="Corner to kill", sub="combo 2 of 2", streak="streak 4 · best 9",
                links=("true", "now", "wait", "wait"), chain=(("1 Ryu BnB ✓", "done"), ("2 Nair loop combo", "now")))
    x0, y0, x1, _ = a
    bar52(img, (x0, y0 + 36, x1, y0 + 162), **conv)
    d = ImageDraw.Draw(img)
    keys_line(img, x0 + 4, y0 + 194, [("A", "on Nair loop combo replaces it with that combo's own bar")], 15)
    x0, y0, x1, _ = b
    f, fb = fnt(14), fnt(14, bold=True)
    parts = (("button: ", f, ch.M_GROUP), ("sequence", fb, ch.M_TEXT), (" | ", f, ch.M_GROUP),
             ("combo", f, ch.M_GROUP))
    tw = sum(d.textlength(t, font=ff) for t, ff, _ in parts) + 24
    d.rounded_rectangle((x1 - tw, y0, x1, y0 + 28), radius=14, outline=ch.M_GROUP, width=1)
    tx = x1 - tw + 12
    for t, ff, colour in parts:
        iv.text(d, (tx, y0 + 14), t, ff, fill=colour, anchor="lm")
        tx += d.textlength(t, font=ff)
    bar52(img, (x0, y0 + 36, x1, y0 + 162), **conv)
    bar52(img, (x0, y0 + 178, x1, y0 + 304), "COMBO", "Nair loop combo", "4 links · tolerance ±1f",
          "streak 2 · best 6", ("true", "now", "wait", "wait"), alpha=150)
    return img


# ---- Q11 --------------------------------------------------------------------------------------------------------------
def q11_label(d, x, y, key, desc, anchor="l"):
    """A button and what it does: the button bold and white, the job grey, on one line."""
    fk, fd = fnt(19, bold=True), fnt(17)
    w = d.textlength(key, font=fk) + 10 + d.textlength(desc, font=fd)
    if anchor == "m":
        x -= w / 2
    elif anchor == "r":
        x -= w
    iv.text(d, (x, y), key, fk, fill=ch.M_TEXT, anchor="lm")
    iv.text(d, (x + d.textlength(key, font=fk) + 10, y), desc, fd, fill=ch.M_GROUP, anchor="lm")


def q11_window(img, box):
    """Layout B as a schematic: the title, Player | CPU, the tab row, the two sections stacked, and the Links panel's
    area on the right. Returns the tabs, the column, and the two sections' (header top, rows top, rows bottom)."""
    d = ImageDraw.Draw(img)
    x0, y0, x1, y1 = box
    d.rounded_rectangle((x0, y0, x1, y0 + 630), radius=12, fill=WIN, outline=WIN_EDGE, width=2)
    iv.text(d, (x0 + 22, y0 + 36), "Recording", fnt(20, bold=True), fill=ch.M_TEXT, anchor="lm")
    role_toggle(d, x1 - 190, y0 + 18, "player", 16)
    tabs, tx = [], x0 + 22
    for i, label in enumerate(("Save states & Macros", "Combos & Sequences")):
        f = fnt(16, bold=True)
        w = d.textlength(label, font=f) + 30
        tb = (tx, y0 + 88, tx + w, y0 + 122)
        if i == 0:
            d.rounded_rectangle(tb, radius=5, fill=ch.M_FOCUS, outline=GREEN, width=2)
        else:
            d.rounded_rectangle(tb, radius=5, outline=ch.M_RULE, width=1)
        iv.text(d, (tx + 15, y0 + 105), label, f, fill=ch.M_TEXT if i == 0 else ch.M_GROUP, anchor="lm")
        tabs.append(tb)
        tx += w + 12
    col = (x0 + 22, x0 + 322)
    data = (("SAVE STATES", "save", ("Corner", "Platform", "Nair loop · start"), y0 + 168),
            ("MACROS", "macro", ("Nair loop", "Dash grab", "CPU throw"), y0 + 414))
    sections = []
    for title, typ, names, head_y in data:
        d.rounded_rectangle((col[0], head_y, col[1], head_y + 38), radius=5, fill=HEAD_BG)
        head_mark(img, typ, col[0] + 17, head_y + 19, 14)
        d = ImageDraw.Draw(img)
        iv.text(d, (col[0] + 34, head_y + 19), title, fnt(16, bold=True), fill=ch.M_TEXT, anchor="lm")
        for k, name in enumerate(names):
            top = head_y + 44 + k * 44
            if typ == "save" and k == 0:
                d.rectangle((col[0] + 2, top, col[1] - 2, top + 40), fill=ch.M_FOCUS)
                d.rectangle((col[0] + 2, top, col[1] - 2, top + 40), outline=WHITE, width=2)
                d.rectangle((col[0] + 2, top, col[0] + 7, top + 40), fill=WHITE)
            iv.text(d, (col[0] + 20, top + 20), name, fnt(16), fill=ch.M_TEXT, anchor="lm")
        sections.append((head_y, head_y + 44, head_y + 44 + len(names) * 44 - 4))
    # The Links panel's area, marked.
    lb = (x0 + 690, y0 + 168, x1 - 22, sections[1][2])
    for yy in range(int(lb[1]), int(lb[3]), 12):
        d.line((lb[0], yy, lb[0], min(yy + 6, lb[3])), fill=ch.M_GROUP, width=2)
        d.line((lb[2], yy, lb[2], min(yy + 6, lb[3])), fill=ch.M_GROUP, width=2)
    for xx in range(int(lb[0]), int(lb[2]), 12):
        d.line((xx, lb[1], min(xx + 6, lb[2]), lb[1]), fill=ch.M_GROUP, width=2)
        d.line((xx, lb[3], min(xx + 6, lb[2]), lb[3]), fill=ch.M_GROUP, width=2)
    iv.text(d, (lb[0] + 16, lb[1] + 24), "Connections panel", fnt(17, bold=True), fill=ch.M_TEXT, anchor="lm")
    for k, line in enumerate(wrap(d, "the connections of the row under the cursor", fnt(15), lb[2] - lb[0] - 32)):
        iv.text(d, (lb[0] + 16, lb[1] + 52 + 20 * k), line, fnt(15), fill=ch.M_GROUP, anchor="lm")
    for k in range(4):
        top = lb[1] + 110 + k * 52
        d.rounded_rectangle((lb[0] + 16 + (24 if k else 0), top, lb[2] - 16, top + 36), radius=6,
                            outline=ch.M_RULE, width=1)
    return tabs, col, sections


def q11_panel(img, box, variant):
    tabs, col, sections = q11_window(img, box)
    d = ImageDraw.Draw(img)
    x0, y0 = box[0], box[1]
    t1 = (tabs[0][0] + tabs[0][2]) / 2
    t2 = (tabs[1][0] + tabs[1][2]) / 2
    ay = y0 + 70
    darrow(d, (t1, ay), (t2, ay))
    # Between the upper and the lower section: a vertical arrow in the gap between them.
    cx = (col[0] + col[1]) / 2
    gy0, gy1 = sections[0][2] + 6, sections[1][0] - 6
    darrow(d, (cx, gy0), (cx, gy1))
    gmid = (gy0 + gy1) / 2
    if variant == "a":
        q11_label(d, t2 + 22, ay, "ZL / ZR", "change the tab")
        q11_label(d, cx + 18, gmid, "Left / Right", "between the upper and the lower section")
        ux = col[1] + 22
        darrow(d, (ux, sections[0][1] + 2), (ux, sections[0][2] - 2))
        q11_label(d, ux + 16, (sections[0][1] + sections[0][2]) / 2, "Up / Down", "move inside a section")
        darrow(d, (ux, sections[1][1] + 2), (ux, sections[1][2] - 2), width=6, head=15)
        q11_label(d, ux + 16, (sections[1][1] + sections[1][2]) / 2, "ZL + Up / Down", "page a section")
    else:
        q11_label(d, t2 + 22, ay, "Left / Right", "on the tab row: change the tab")
        q11_label(d, cx + 18, gmid, "ZL / ZR", "between the upper and the lower section")
        ux = tabs[0][2] - 26
        arrow4(d, [(ux, sections[0][1]), (ux, tabs[0][3] + 4)], GREEN, width=3, head=11)
        q11_label(d, ux + 16, (tabs[0][3] + sections[0][0]) / 2, "Up",
                  "from the top row of the upper section reaches the tab row")


def q11():
    img, (a, b) = q_canvas(11, WIDE, 630,
                           "ZL / ZR change the tab. Left / Right move between the upper and the lower section. Up / "
                           "Down move inside a section, and ZL + Up / Down pages it.",
                           "ZL / ZR move between the upper and the lower section. Up from the top row of the upper "
                           "section reaches the tab row, where Left / Right change the tab.")
    q11_panel(img, a, "a")
    q11_panel(img, b, "b")
    return img


# ---- Q12 --------------------------------------------------------------------------------------------------------------
def q_macro_row(img, box, name, start, badge_n=None, chain_n=None, bright=True):
    """A macro or a chain at question size: the badge, the MACRO pill, the name, the chain mark with its count, and on
    the right the floppy chip with the start save's name (orange when bright, grey when it is not used)."""
    d = ImageDraw.Draw(img)
    x0, y0, x1, y1 = box
    mc = TYPES["macro"][0]
    d.rounded_rectangle(box, radius=6, fill=NODE_BG, outline=mc if bright else EDGE52, width=2)
    cy = (y0 + y1) / 2
    x = x0 + 8
    if badge_n is not None:
        badge(d, x + 12, cy, badge_n, 12)
        x += 30
    x = pill(d, x, cy, "macro", 14) + 10
    ink = ch.M_TEXT if bright else ch.M_GROUP
    iv.text(d, (x, cy), name, fnt(16), fill=ink, anchor="lm")
    x += d.textlength(name, font=fnt(16)) + 10
    if chain_n:
        icon(img, "format_list_numbered", x, cy - 10, 20, ink)
        d = ImageDraw.Draw(img)
        iv.text(d, (x + 24, cy), str(chain_n), fnt(16, bold=True), fill=ink, anchor="lm")
    if start:
        iv.text(d, (x1 - 12, cy), start, fnt(15), fill=ink if bright else DIM, anchor="rm")
        fx = x1 - 12 - d.textlength(start, font=fnt(15)) - 26
        icon(img, "save", fx, cy - 10, 20, TYPES["save"][0] if bright else DIM)
        return fx + 10
    return None


def q12():
    img, (a, b) = q_canvas(12, W, 316,
                           "The chain starts where its macro 1 starts: Nair loop · start, on the chain's row with the "
                           "floppy chip. To change it, reorder, or save over that file. No new screen.",
                           "The chain's own screen has a Start save row. A on it opens the SAVE STATES list, and any "
                           "save can be picked, here Corner. One new picker.")
    # A: the chain's row, then its macros in order. Macro 1's start save is the chain's.
    x0, y0, x1, y1 = a
    d = ImageDraw.Draw(img)
    iv.text(d, (x0, y0 + 10), "The chain's row", fnt(15), fill=ch.M_GROUP, anchor="lm")
    fx = q_macro_row(img, (x0, y0 + 24, x1, y0 + 70), "Corner pressure", "Nair loop · start", chain_n=3)
    d = ImageDraw.Draw(img)
    iv.text(d, (x0, y0 + 88), "Its macros, in order", fnt(15), fill=ch.M_GROUP, anchor="lm")
    rows = (("Nair loop", "Nair loop · start", True), ("Dash grab", "Dash grab · start", False),
            ("CPU throw", "CPU throw · start", False))
    for i, (name, start, used) in enumerate(rows):
        top = y0 + 102 + i * 52
        q_macro_row(img, (x0 + 24, top, x1, top + 44), name, start, badge_n=i + 1, bright=used)
    d = ImageDraw.Draw(img)
    darrow(d, (fx, y0 + 98), (fx, y0 + 76), col=TYPES["save"][0], width=3, head=9)
    iv.text(d, (fx + 14, y0 + 87), "the same file", fnt(15, bold=True), fill=TYPES["save"][0], anchor="lm")
    iv.text(d, (x0, y0 + 274), "Their own start saves are not used.", fnt(15), fill=ch.M_GROUP, anchor="lm")
    iv.text(d, (x0, y0 + 298), "Macro 1 with no start save: the chain plays from where the fighters are.", fnt(15),
            fill=ch.M_GROUP, anchor="lm")

    # B: the chain's own screen, with a Start save row, and the SAVE STATES picker it opens.
    x0, y0, x1, y1 = b
    d = ImageDraw.Draw(img)
    mid = x0 + 286
    iv.text(d, (x0, y0 + 10), "Corner pressure, its own screen", fnt(15), fill=ch.M_GROUP, anchor="lm")
    screen = (("Start save", "Nair loop · start  >"), ("Set active", "On · CPU"), ("Edit chain", ""), ("Rename", ""),
              ("Make combo", ""), ("Delete", ""))
    for i, (label, value) in enumerate(screen):
        y = y0 + 44 + i * 38
        if i == 0:
            d.rectangle((x0, y - 16, mid, y + 16), fill=ch.M_FOCUS)
            d.rectangle((x0, y - 16, mid, y + 16), outline=WHITE, width=2)
            d.rectangle((x0, y - 16, x0 + 5, y + 16), fill=WHITE)
        iv.text(d, (x0 + 14, y), label, fnt(16), fill=ch.M_TEXT, anchor="lm")
        if value:
            iv.text(d, (mid - 8, y), value, fnt(14), fill=GREEN if value.startswith("On") else ch.M_GROUP,
                    anchor="rm")
    px0 = mid + 40
    arrow4(d, [(mid + 6, y0 + 44), (px0 - 6, y0 + 44)], GREEN, width=3, head=10)
    iv.text(d, ((mid + px0) / 2, y0 + 24), "A", fnt(15, bold=True), fill=ch.M_TEXT, anchor="mm")
    d.rounded_rectangle((px0, y0 + 26, x1, y0 + 62), radius=5, fill=HEAD_FOCUS)
    head_mark(img, "save", px0 + 16, y0 + 44, 14)
    d = ImageDraw.Draw(img)
    iv.text(d, (px0 + 34, y0 + 44), "Pick a start save", fnt(15, bold=True), fill=ch.M_TEXT, anchor="lm")
    for i, name in enumerate(("Corner", "Platform", "Nair loop · start", "Dash grab · start", "CPU throw · start")):
        top = y0 + 68 + i * 38
        if i == 0:
            d.rectangle((px0, top, x1, top + 34), fill=ch.M_FOCUS)
            d.rectangle((px0, top, x1, top + 34), outline=WHITE, width=2)
            d.rectangle((px0, top, px0 + 5, top + 34), fill=WHITE)
        iv.text(d, (px0 + 16, top + 17), name, fnt(16), fill=ch.M_TEXT, anchor="lm")
    keys_line(img, px0, y0 + 274, [("A", "pick"), ("B", "back")], 15)
    d = ImageDraw.Draw(img)
    iv.text(d, (x0, y0 + 298), "A new screen, opened from the Start save row.", fnt(15), fill=ch.M_GROUP, anchor="lm")
    return img


# ---- Q13 --------------------------------------------------------------------------------------------------------------
def q_tiles(img, box, title, tiles, note):
    """The numbered picker at question size, as Character Cycles: name tiles, two columns, a red number on the
    numbered ones. A tile is (name, sub, number, cursor, chain count)."""
    d = ImageDraw.Draw(img)
    x0, y0, x1, y1 = box
    iv.text(d, (x0, y0 + 12), title, fnt(16, bold=True), fill=ch.M_TEXT, anchor="lm")
    tw, th = (x1 - x0 - 12) / 2, 62
    for i, (name, sub, num, cur, chain_n) in enumerate(tiles):
        tx = x0 + (i % 2) * (tw + 12)
        ty = y0 + 36 + (i // 2) * (th + 10)
        d.rounded_rectangle((tx, ty, tx + tw, ty + th), radius=6, fill=ch.M_FOCUS if cur else (28, 30, 38, 255),
                            outline=WHITE if cur else ch.M_RULE, width=3 if cur else 1)
        nx = tx + 16
        if num:
            badge(d, tx + 28, ty + th / 2, num, 14)
            nx = tx + 52
        iv.text(d, (nx, ty + 21), name, fnt(17), fill=ch.M_TEXT, anchor="lm")
        if chain_n:
            cx = nx + d.textlength(name, font=fnt(17)) + 10
            icon(img, "format_list_numbered", cx, ty + 11, 20, ch.M_TEXT)
            d = ImageDraw.Draw(img)
            iv.text(d, (cx + 24, ty + 21), str(chain_n), fnt(16, bold=True), fill=ch.M_TEXT, anchor="lm")
        iv.text(d, (nx, ty + 43), sub, fnt(14), fill=ch.M_GROUP, anchor="lm")
    iv.text(d, (x0, y0 + 274), note, fnt(15), fill=ch.M_GROUP, anchor="lm")
    keys_line(img, x0, y1 - 12, [("A", "add number"), ("R", "remove number"), ("B", "done")], 15)


def q13():
    img, (a, b) = q_canvas(13, W, 330,
                           "No. The chain picker lists recordings only, so a chain never contains itself and nothing "
                           "needs a loop check.",
                           "Yes. A chain can be a step of another chain. The picker leaves out any chain that already "
                           "contains this one, so no chain plays itself.")
    q_tiles(img, a, "Edit chain · Corner pressure", [
        ("Nair loop", "Recorded on Pikachu", 1, False, None), ("Dash grab", "Recorded on Pikachu", 2, False, None),
        ("CPU throw", "Recorded on Mario", 3, True, None), ("Macro 6", "Recorded on Lucina", None, False, None),
        ("Replay 86f", "Recorded on Pikachu", None, False, None),
        ("Slot 2 (2026-11-05)", "Recorded on Steve", None, False, None),
    ], "Recordings only: no chain is offered.")
    q_tiles(img, b, "New chain", [
        ("Corner pressure", "a chain · Pikachu · Mario", 1, False, 3), ("Macro 6", "Recorded on Lucina", 2, True, None),
        ("Nair loop", "Recorded on Pikachu", None, False, None), ("Dash grab", "Recorded on Pikachu", None, False, None),
        ("CPU throw", "Recorded on Mario", None, False, None), ("Replay 86f", "Recorded on Pikachu", None, False, None),
    ], "Left out: any chain that already contains this one.")
    return img


def main_q52(game):
    P = "RECORDING-REDESIGN-q%d-ab"
    save(q1(), P % 1)
    save(q3(), P % 3)
    save(q4(), P % 4)
    save(q5(game), P % 5)
    save(q6(), P % 6)
    save(q7(), P % 7)
    save(q8_decided(), "RECORDING-REDESIGN-q8-decided")
    save(q9(), P % 9)
    save(q11(), P % 11)
    save(q12(), P % 12)
    save(q13(), P % 13)


def main():
    menu = iv.background(MENU)
    game = iv.background(GAME)
    P = "RECORDING-REDESIGN-r8-"

    save(diagram(), P + "links")
    save(legend(), P + "legend")
    save(loop_picture(), P + "loop")

    bar = sequence_bar4(game)
    save(flow_record(game), P + "flow-record")
    save(flow_chain(), P + "flow-chain")
    save(flow_make_combo(game), P + "flow-make-combo")
    save(flow_sequence(), P + "flow-sequence")
    save(flow_drill(game, bar), P + "flow-drill")
    save(bar, P + "sequence-bar")

    # Revision 8: a save only warps. Bind to Save / Load is On or Off, for both roles; it has no chain and no role.
    save(action_screen4(menu, "Recording (Player view)  >  Save states & Macros  >  Corner", [
        ("Bind to Save / Load", True), ("Duplicate", False), ("Rename", False), ("Delete", False),
    ], "Bind to Save / Load: Left / Right turn it On or Off. One save is bound at a time, for both roles.",
        "Save (ZL + Down) writes this file. Load (ZL + Up) warps here and starts nothing.",
        {"Bind to Save / Load": ("On", "on")},
        [("Up/Down", "move"), ("Left/Right", "On / Off"), ("A", "do it"), ("B", "back")]), P + "save-actions")
    save(action_screen4(menu, "Recording (Player view)  >  Combos & Sequences  >  Corner to kill", [
        ("Drill sequence", True), ("1  Ryu BnB", False), ("2  Nair loop combo", False), ("Edit combos", False),
        ("Rename", False), ("Delete", False),
    ], "Drill sequence grades the whole sequence. A on a numbered combo drills that combo alone.",
        "Edit combos is the numbered picker. Deleting the sequence keeps the combo files."),
        P + "sequence-actions")
    save(action_screen4(menu, "Recording (Player view)  >  Combos & Sequences  >  Nair loop combo", [
        ("Drill", True), ("Made from", False), ("Rename", False), ("Crop", False),
        ("Delete", False),
    ], "Drill makes this the one drill and clears a drilled sequence.",
        "Made from names the macro it came from. Crop asks for the first and last link and saves a copy.",
        {"Made from": ("Nair loop", "text")}), P + "combo-actions")
    main_r8(menu, game)
    main_q52(game)


if __name__ == "__main__":
    if "--r3" in sys.argv[1:]:
        main_r3()
    elif "--q52" in sys.argv[1:]:
        main_q52(iv.background(GAME))
    else:
        main()
