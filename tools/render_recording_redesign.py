#!/usr/bin/env python3
"""Revisions 3 and 4 of the recording-redesign mockups.

Menu screens are redrawn on the real rc.1 Recording page (the COMBO-HELPER method).
The pop-up, the chain, and combo-from-macro sit on a real Eden Training frame from
artifacts/phase23. The Switch keyboard is a labelled schematic, not a custom keyboard.
Revision 4 draws the loop, the pin, the link and the numbered-list marks from the
Material Symbols SVGs in docs/decisions/icons (Apache-2.0; see SOURCE.txt there).

Run from the repo root:
  /usr/bin/python3 -I tools/render_recording_redesign.py          (revision 4, RECORDING-REDESIGN-r4-*.png)
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

GROUPS = [("Macros", 8), ("Combos", 4), ("Conversions", 3), ("Save states", 5), ("Replays", 2), ("Options", 5)]
RED = (232, 72, 72, 255)


def thumbtack(d, x, y, on_green=False):
    """A tilted pushpin: a flat head and a needle, not a round map pin."""
    import math
    ang = math.radians(-36)
    cx, cy = x + 9, y + 6
    hw, hh = 8.5, 2.8
    pts = []
    for dx, dy in ((-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)):
        pts.append((
            cx + dx * math.cos(ang) - dy * math.sin(ang),
            cy + dx * math.sin(ang) + dy * math.cos(ang),
        ))
    nx = cx + math.cos(math.radians(62)) * 13
    ny = cy + math.sin(math.radians(62)) * 13
    d.line((cx + 1, cy + 2, nx, ny), fill=DARK, width=3)
    d.line((cx + 1, cy + 2, nx, ny), fill=GOLD, width=2)
    d.polygon(pts, fill=GOLD, outline=DARK)


def repeat_icon(d, x, y, mode, size=36):
    """A loop with the arrowhead at the lower left. White when off, green with a dot when it repeats,
    and the count drawn in the middle the way a 'repeat one' mark does."""
    col = WHITE if mode == "off" else GREEN
    w = max(2, size // 16)
    left, top, right, bot = x + 6, y + 4, x + size - 6, y + size - 12
    rad = max(5, size // 6)
    d.rounded_rectangle((left, top, right, bot), radius=rad, outline=col, width=w)
    # Open the lower-left corner so the arrowhead can finish the loop.
    d.rectangle((left - 2, bot - rad, left + rad, bot + 3), fill=(13, 13, 13, 255))
    d.polygon([(left - 1, bot), (left + size // 5, bot - size // 7), (left + size // 5, bot + size // 7)], fill=col)
    if mode == "on":
        cx = (left + right) / 2
        d.ellipse((cx - 3, bot + 5, cx + 3, bot + 11), fill=col)
    elif mode not in ("off", "on"):
        iv.text(d, ((left + right) / 2, (top + bot) / 2), str(mode),
                iv.font(max(11, size // 4), bold=True), fill=col, anchor="mm")


def save(img, name):
    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, name + ".png")
    img.convert("RGB").save(path, optimize=True)
    print(path)


def hints(d, line):
    d.rectangle((44, 660, 1246, 698), fill=ch.M_PANEL)
    iv.text(d, (53, 678), line, iv.font(14), fill=ch.M_GROUP, anchor="lm")


def draw_groups(d, selected):
    for i, (name, count) in enumerate(GROUPS):
        y = ch.ROW_Y0 + i * ch.GROUP_STEP
        if i == selected:
            d.rectangle((33, y - 18, 307, y + 18), fill=ch.M_FOCUS)
        iv.text(d, (48, y), "%s  (%d)" % (name, count), ch.MF,
                fill=ch.M_TEXT if i == selected else ch.M_GROUP, anchor="lm")


def page_base(bg, selected, help1, help2, hint):
    img = bg.copy()
    d = ImageDraw.Draw(img)
    ch.clear_content(d)
    draw_groups(d, selected)
    ch.help_lines(d, help1, help2)
    hints(d, hint)
    return img, d


def library_rows(d, rows, c2, c3, y0=168, c2x=760, c3x=1040):
    iv.text(d, (386, 140), "Name", SMALL, fill=ch.M_GROUP, anchor="lm")
    iv.text(d, (c2x, 140), c2, SMALL, fill=ch.M_GROUP, anchor="lm")
    iv.text(d, (c3x, 140), c3, SMALL, fill=ch.M_GROUP, anchor="lm")
    hot = 0
    for row in rows:
        if row["hot"]:
            hot += 1
        else:
            break
    centers = []
    y = y0
    for i, row in enumerate(rows):
        if hot and i == hot:
            y += 16
        top, bot = y - 16, y + 16
        on = row.get("active")
        if on:
            d.rectangle((330, top, 1236, bot), fill=GREEN_ROW)
        elif row.get("cursor"):
            d.rectangle((330, top, 1236, bot), fill=ch.M_FOCUS)
        if row.get("cursor"):
            d.rectangle((330, top, 334, bot), fill=WHITE)
        if row.get("pin"):
            thumbtack(d, 346, y - 12, on_green=bool(on))
        name_col = ch.M_TEXT if not row.get("dim") else ch.M_GROUP
        iv.text(d, (370, y), row["name"], ch.MF, fill=name_col, anchor="lm")
        iv.text(d, (c2x, y), row.get("c2", ""), ch.MF, fill=ch.M_GROUP, anchor="lm")
        iv.text(d, (c3x, y), row.get("c3", ""), ch.MF, fill=GREEN if row.get("c3_on") else ch.M_GROUP, anchor="lm")
        centers.append(y)
        y += 36
    if hot:
        d.rounded_rectangle((328, centers[0] - 20, 1238, centers[hot - 1] + 20),
                            outline=GREEN, width=2, radius=8)
    return y


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


def chip(d, x, y, label, on):
    f = iv.font(15, bold=True)
    w = d.textlength(label, font=f) + 28
    if on:
        d.rounded_rectangle((x, y, x + w, y + 28), radius=4, fill=ch.M_FOCUS, outline=GREEN, width=2)
        iv.text(d, (x + 14, y + 14), label, f, fill=ch.M_TEXT, anchor="lm")
    else:
        d.rounded_rectangle((x, y, x + w, y + 28), radius=4, outline=(120, 126, 140, 255), width=1)
        iv.text(d, (x + 14, y + 14), label, f, fill=ch.M_GROUP, anchor="lm")
    return x + w + 10


def popup(game, role, section, sections, rows, loop, c2, c3, c2x=620, c3x=860):
    img = game.copy()
    d = ImageDraw.Draw(img)
    last = 196 + (len(rows) - 1) * 34
    box = (210, 48, 1070, last + 78)
    d.rounded_rectangle(box, radius=12, fill=PANEL, outline=(80, 86, 100, 255), width=2)
    x = 232
    x = chip(d, x, 68, "Player", role == "player")
    chip(d, x, 68, "CPU", role == "cpu")
    x = 232
    y = 112
    for name in sections:
        x = chip(d, x, y, name, name == section)
    repeat_icon(d, 900, 104, loop, 46)
    iv.text(d, (888, 126), "Loop", TINY, fill=ch.M_GROUP, anchor="rm")
    # Shift the library into the panel by drawing rows with popup coordinates.
    iv.text(d, (268, 162), "Name", SMALL, fill=ch.M_GROUP, anchor="lm")
    iv.text(d, (c2x, 162), c2, SMALL, fill=ch.M_GROUP, anchor="lm")
    iv.text(d, (c3x, 162), c3, SMALL, fill=ch.M_GROUP, anchor="lm")
    hot = 0
    for row in rows:
        if row["hot"]:
            hot += 1
        else:
            break
    y = 196
    centers = []
    for i, row in enumerate(rows):
        if hot and i == hot:
            y += 16
        top, bot = y - 15, y + 15
        on = row.get("active")
        if on:
            d.rectangle((232, top, 1048, bot), fill=GREEN_ROW)
        elif row.get("cursor"):
            d.rectangle((232, top, 1048, bot), fill=ch.M_FOCUS)
        if row.get("cursor"):
            d.rectangle((232, top, 236, bot), fill=WHITE)
        if row.get("pin"):
            thumbtack(d, 248, y - 12, on_green=bool(on))
        iv.text(d, (272, y), row["name"], ch.MF, fill=ch.M_TEXT, anchor="lm")
        iv.text(d, (c2x, y), row.get("c2", ""), SMALL, fill=ch.M_GROUP, anchor="lm")
        iv.text(d, (c3x, y), row.get("c3", ""), SMALL, fill=GREEN if row.get("c3_on") else ch.M_GROUP, anchor="lm")
        centers.append(y)
        y += 34
        if y > 560:
            break
    if hot:
        d.rounded_rectangle((228, centers[0] - 18, 1052, centers[hot - 1] + 18),
                            outline=GREEN, width=2, radius=8)
    iv.text(d, (232, last + 46), "A select    Y pin    X loop    L role    R section    ZL page    B close",
            TINY, fill=ch.M_GROUP, anchor="lm")
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


def keyboard(bg):
    img = bg.copy()
    d = ImageDraw.Draw(img)
    ch.clear_content(d)
    draw_groups(d, 0)
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


def choices():
    """The questions revision 3 still leaves open. Green is the recommendation."""
    blocks = [
        ("How you number them",
         "A  Name tiles. A adds the next number. A again removes it, and the later numbers close up.",
         "Same gesture as Character Cycles. A macro or a combo is a name, so the tile is the name, not a portrait.",
         "B  A list, and Y moves that file one place later."),
        ("Which role a chained macro plays",
         "A  The role stored in the macro file. A Player macro plays on Player 1. A CPU macro plays on the CPU.",
         "They play one after another. Starting both roles yourself is unchanged.",
         "B  Every macro in the chain plays on the role tab that was open when the save loaded."),
        ("What loop does with a chain",
         "A  The chain is one unit. Each macro plays once, then the whole list repeats.",
         "A chord Load uses the Player tab's count. A load from the CPU tab uses that tab's count.",
         "B  Each macro repeats by its own role's count before the next number starts."),
        ("A macro recorded on another fighter",
         "A  Switch or Keep is asked once, when the chain starts, and the answer covers the whole chain.",
         "The prompt names every mismatched macro. It is the prompt that already exists.",
         "B  Ask again at each macro whose fighter does not match."),
        ("Where a conversion lives",
         "A  Its own section, Conversions, on the Recording page and on the Player pop-up.",
         "R reaches it. The CPU tab skips it, the same way it skips Combos. The drill is the player's.",
         "B  A sub-list inside Combos, with no section of its own."),
        ("Hot conversions",
         "A  Five hot conversions, same pin and newest rules as macros and saves.",
         "Drilling one puts it in the hot list. A new one takes a slot even when all five are pinned.",
         "B  No hot list. Only the conversion you are drilling is marked."),
        ("What the conversion file stores",
         "A  The combo files' ids, in order. The same kind of link as a macro's start save.",
         "Renaming a combo keeps it. Editing the combo updates the conversion. A missing combo is skipped.",
         "B  A copy of each combo's links, frozen when the conversion was saved."),
        ("A drop, and what completes the chain",
         "A  A drop restarts the conversion at combo 1. Complete means every combo, in order, while the CPU is still in hitstun between them.",
         "The conversion streak and best count a finished conversion only. One combo drilled alone keeps that combo's own streak.",
         "B  After a drop, continue from that combo. Anything landed back to back completes the chain, even if the CPU could act in between."),
        ("One drill at a time",
         "A  Drilling a conversion replaces the drilled combo, and drilling a combo replaces the conversion.",
         "The helper has one bar. A on a numbered combo inside the conversion drills that combo alone.",
         "B  A combo and a conversion can both be active, and a button chooses which bar you see."),
    ]
    height = 36 + len(blocks) * 168 + 24
    img = Image.new("RGBA", (W, height), (13, 13, 13, 255))
    d = ImageDraw.Draw(img)
    iv.text(d, (48, 16), "Still open. Green is the recommendation.", iv.font(18, bold=True), fill=ch.M_TEXT, anchor="lm")
    y = 48
    for title, a, amid, b in blocks:
        d.rounded_rectangle((40, y, 1240, y + 152), radius=8, outline=GREEN, width=2)
        iv.text(d, (56, y + 10), title, iv.font(16, bold=True), fill=ch.M_TEXT, anchor="lm")
        d.rectangle((56, y + 40, 1224, y + 72), fill=GREEN_ROW)
        iv.text(d, (68, y + 56), a, SMALL, fill=ch.M_TEXT, anchor="lm")
        iv.text(d, (68, y + 88), amid, TINY, fill=ch.M_GROUP, anchor="lm")
        iv.text(d, (68, y + 116), b, TINY, fill=ch.M_GROUP, anchor="lm")
        y += 168
    return img


def order_grid(bg, crumb, tiles, help1, help2, hint):
    """Name tiles with Character Cycles numbers. A adds the next number; A again removes it."""
    img = bg.copy()
    d = ImageDraw.Draw(img)
    ch.clear_content(d, list_view=True)
    iv.text(d, (53, 118), crumb, ch.MF, fill=ch.M_TEXT, anchor="lm")
    cols, tw, th, gapx, gapy = 4, 280, 72, 16, 14
    x0, y0 = 56, 150
    for i, tile in enumerate(tiles):
        col, row = i % cols, i // cols
        x = x0 + col * (tw + gapx)
        y = y0 + row * (th + gapy)
        cursor = tile.get("cursor")
        d.rounded_rectangle((x, y, x + tw, y + th), radius=6,
                            fill=ch.M_FOCUS if cursor else (28, 30, 38, 255),
                            outline=WHITE if cursor else ch.M_RULE, width=3 if cursor else 1)
        num = tile.get("number")
        if num is not None:
            d.ellipse((x + 10, y + 18, x + 36, y + 44), fill=RED)
            iv.text(d, (x + 23, y + 31), str(num), iv.font(14, bold=True), fill=WHITE, anchor="mm")
            nx = x + 48
        else:
            nx = x + 16
        iv.text(d, (nx, y + 22), tile["name"], ch.MF, fill=ch.M_TEXT, anchor="lm")
        iv.text(d, (nx, y + 46), tile.get("sub", ""), SMALL, fill=ch.M_GROUP, anchor="lm")
    ch.help_lines(d, help1, help2)
    hints(d, hint)
    return img


def hide_legacy_notice(img):
    """The captured Training frame still prints the old Input Playback notice. Paint that panel out with the sky above it, text included."""
    px = img.load()
    for y in range(30, 150):
        for x in range(300, 980):
            px[x, y] = px[x, 6]
    return img


def conversion_hud(game, title, subtitle, links, result, streak):
    img = hide_legacy_notice(game.copy())
    d = ImageDraw.Draw(img)
    x0, y0, x1, y1 = 40, 500, 1240, 692
    d.rounded_rectangle((x0, y0, x1, y1), radius=10, fill=(14, 18, 27, 232), outline=(124, 92, 214, 255), width=2)
    d.rounded_rectangle((x0 + 14, y0 + 12, x0 + 148, y0 + 36), radius=4, fill=(124, 92, 214, 255))
    iv.text(d, (x0 + 81, y0 + 24), "CONVERSION", iv.font(12, bold=True), fill=WHITE, anchor="mm")
    iv.text(d, (x0 + 162, y0 + 24), title, iv.font(16, bold=True), fill=WHITE, anchor="lm")
    iv.text(d, (x1 - 18, y0 + 24), streak, iv.font(14, bold=True), fill=GREEN, anchor="rm")
    iv.text(d, (x0 + 16, y0 + 52), subtitle, SMALL, fill=ch.M_GROUP, anchor="lm")
    n = len(links)
    gap = 8
    cw = (x1 - x0 - 32 - (n - 1) * gap) / n
    for i, (label, state) in enumerate(links):
        cx = x0 + 16 + i * (cw + gap)
        cy = y0 + 78
        outline = {"true": GREEN, "drop": RED, "now": WHITE}.get(state, ch.M_RULE)
        d.rounded_rectangle((cx, cy, cx + cw, cy + 52), radius=6, fill=(32, 38, 52, 255), outline=outline, width=2)
        iv.text(d, (cx + 10, cy + 16), label, SMALL, fill=WHITE, anchor="lm")
        word = {"true": "TRUE", "drop": "DROP", "now": "now", "wait": "—"}.get(state, "—")
        col = GREEN if state == "true" else RED if state == "drop" else WHITE if state == "now" else ch.M_GROUP
        iv.text(d, (cx + cw - 10, cy + 16), word, SMALL, fill=col, anchor="rm")
    iv.text(d, (x0 + 16, y1 - 28), result, iv.font(15, bold=True), fill=WHITE, anchor="lm")
    return img


MACROS = [
    {"name": "Nair loop", "c2": "Pikachu", "c3": "2.4 s", "hot": True, "pin": True, "active": True},
    {"name": "Dash grab", "c2": "Mario", "c3": "1.1 s", "hot": True, "pin": True, "cursor": True},
    {"name": "Replay 86f", "c2": "Pikachu", "c3": "1.4 s", "hot": True},
    {"name": "Slot 2 (2026-11-05)", "c2": "Steve", "c3": "4.0 s", "hot": True},
    {"name": "Macro 6", "c2": "Lucina", "c3": "0.8 s", "hot": False},
    {"name": "Slot 5 (2026-11-05)", "c2": "Villager", "c3": "3.2 s", "hot": False, "dim": True},
]
COMBOS = [
    {"name": "Ryu BnB", "c2": "Ryu → Mario", "c3": "6 links", "hot": True, "pin": True},
    {"name": "Nair loop", "c2": "Pikachu → Mario", "c3": "4 links", "hot": True, "active": True, "cursor": True},
    {"name": "Combo 2", "c2": "Pikachu → Mario", "c3": "2 links", "hot": True},
    {"name": "Combo 4", "c2": "Steve → Pikachu", "c3": "3 links", "hot": False},
]
SAVES = [
    {"name": "Corner pressure", "c2": "Steve · Villager", "c3": "1 Nair · 2 Grab · 3 Replay", "c3_on": True,
     "hot": True, "pin": True, "active": True},
    {"name": "Platform", "c2": "Pikachu · Mario", "c3": "chain off", "hot": True, "pin": True, "cursor": True},
    {"name": "Slot 1 (2026-11-05)", "c2": "Steve · Villager", "c3": "chain off", "hot": True},
    {"name": "Nair loop · start", "c2": "Pikachu · Mario", "c3": "linked", "hot": False},
    {"name": "CPU throw · start", "c2": "Mario · Pikachu", "c3": "linked", "hot": False},
    {"name": "Slot 3 (2026-11-05)", "c2": "Lucina · Cloud", "c3": "chain off", "hot": False, "dim": True},
]
CONVERSIONS = [
    {"name": "Corner to kill", "c2": "Ryu → Mario", "c3": "1 Ryu · 2 Nair · 3 Combo 2", "c3_on": True,
     "hot": True, "pin": True, "active": True},
    {"name": "Platform strings", "c2": "Pikachu → Mario", "c3": "1 Nair · 2 Combo 4", "hot": True, "cursor": True},
    {"name": "Conversion 3", "c2": "Steve → Pikachu", "c3": "1 Combo 4", "hot": False},
]
CPU_MACROS = [
    {"name": "CPU throw", "c2": "Mario", "c3": "1.8 s", "hot": True, "pin": True, "active": True},
    {"name": "CPU tech", "c2": "Mario", "c3": "0.6 s", "hot": True, "cursor": True},
    {"name": "Slot 1 (2026-11-05)", "c2": "Mario", "c3": "2.0 s", "hot": True},
    {"name": "Macro 6", "c2": "Lucina", "c3": "0.8 s", "hot": False},
]


def main_r3():
    """Revision 3's pictures (kept so they can be rebuilt; `--r3`)."""
    menu = iv.background(MENU)
    game = iv.background(GAME)
    sections_p = ("Macros", "Combos", "Conversions", "Save states")
    sections_c = ("Macros", "Save states")

    save(popup(game, "player", "Macros", sections_p, MACROS, "on", "Fighter", "Length"),
         "RECORDING-REDESIGN-popup-player")
    save(popup(game, "player", "Combos", sections_p, COMBOS, "on", "Fighters", "Links"),
         "RECORDING-REDESIGN-popup-combos")
    save(popup(game, "player", "Conversions", sections_p, CONVERSIONS, "on", "Fighters", "Combos", c2x=520, c3x=760),
         "RECORDING-REDESIGN-popup-conversions")
    save(popup(game, "player", "Save states", sections_p, SAVES, "on", "Fighters", "Chain", c2x=520, c3x=740),
         "RECORDING-REDESIGN-popup-saves")
    save(popup(game, "cpu", "Macros", sections_c, CPU_MACROS, "1", "Fighter", "Length"),
         "RECORDING-REDESIGN-popup-cpu")

    loop = Image.new("RGBA", (W, 220), (13, 13, 13, 255))
    ld = ImageDraw.Draw(loop)
    iv.text(ld, (40, 24), "Loop indicator. X cycles ∞, 1, 2, 3, 5, 10.", iv.font(18, bold=True), fill=ch.M_TEXT)
    for i, (mode, label) in enumerate((("off", "off"), ("on", "∞  repeats"), ("1", "1"), ("2", "2"),
                                       ("3", "3"), ("5", "5"), ("10", "10"))):
        x = 48 + i * 170
        repeat_icon(ld, x, 80, mode, 52)
        iv.text(ld, (x + 26, 170), label, ch.MF, fill=ch.M_GROUP, anchor="mm")
    save(loop, "RECORDING-REDESIGN-loop")

    img, d = page_base(menu, 0,
                       "Nair loop is the one that plays on Player 1. The green row is that macro.",
                       "The blue-gray row is the cursor. Y pins. R moves to Combos.",
                       "Up/Down move    A actions    Y pin    R section    ZL+Up/Down page    B close")
    library_rows(d, MACROS, "Fighter", "Length")
    save(img, "RECORDING-REDESIGN-page")

    img, d = page_base(menu, 1,
                       "Ryu BnB is pinned, so it sits above the other hot combos.",
                       "The green row is the combo you are drilling.",
                       "Up/Down move    A actions    Y pin    R section    B close")
    library_rows(d, COMBOS, "Fighters", "Links")
    save(img, "RECORDING-REDESIGN-page-combos")

    img, d = page_base(menu, 2,
                       "Corner to kill is the conversion you are drilling. The numbers are its combos.",
                       "A on a combo inside it drills that combo alone. R moves to Save states.",
                       "Up/Down move    A actions    Y pin    R section    B close")
    library_rows(d, CONVERSIONS, "Fighters", "Combos", c2x=640, c3x=900)
    save(img, "RECORDING-REDESIGN-page-conversions")

    img, d = page_base(menu, 3,
                       "The green row is the save the Save and Load chords use.",
                       "Corner pressure plays 1, then 2, then 3. Nair loop · start stays under the outline.",
                       "Up/Down move    A actions    Y pin    R section    B close")
    library_rows(d, SAVES, "Fighters", "Chain", c2x=680, c3x=920)
    save(img, "RECORDING-REDESIGN-page-saves")

    img, d = page_base(menu, 4,
                       "Replay capture and the Replay window save into the same macro library.",
                       "The window's own crop is the macro. There is no second crop command there.",
                       "Up/Down move    A open    R section    B close")
    setting_rows(d, [
        ("Replay capture", "[ON]", "on", True),
        ("Replay window", "", "action", False),
    ])
    save(img, "RECORDING-REDESIGN-page-replays")

    img, d = page_base(menu, 5,
                       "Loop counts are the list X cycles on the pop-up.",
                       "Button chords is where Record, Play and the pop-up are edited.",
                       "Up/Down move    Left/Right change    A open    R section    B close")
    setting_rows(d, [
        ("Crop trailing neutral", "[ON]", "on", False),
        ("Hitstun timing", "After hitstun", "pick", False),
        ("Mash interrupts playback", "[ON]", "on", False),
        ("Loop counts", "∞  1  2  3  5  10", "pick", True),
        ("Button chords", "", "action", False),
    ])
    save(img, "RECORDING-REDESIGN-page-options")

    img = action_screen(menu, "Recording  >  Macros  >  Nair loop", [
        ("Start save", False),
        ("Set active for Player 1", True),
        ("Set active for CPU", False),
        ("Rename", False),
        ("Crop", False),
        ("Make combo", False),
        ("Pin", False),
        ("Delete", False),
    ], "Record wrote Nair loop · start and this macro stores that file's id.",
       "Renaming the save keeps the link. Make combo loads it, then plays on Player 1.")
    d = ImageDraw.Draw(img)
    iv.text(d, (1180, 168), "Nair loop · start", ch.MF, fill=ch.M_GROUP, anchor="rm")
    save(img, "RECORDING-REDESIGN-macro-actions")

    save(action_screen(menu, "Recording  >  Combos  >  Nair loop", [
        ("Drill", True),
        ("Rename", False),
        ("Crop", False),
        ("Pin", False),
        ("Delete", False),
    ], "Drill makes this the one combo the pop-up marks.",
       "Crop asks for the first and last link and saves a copy."),
         "RECORDING-REDESIGN-combo-actions")

    img = action_screen(menu, "Recording  >  Save states  >  Corner pressure", [
        ("Bind to Save / Load", False),
        ("Chain", False),
        ("Chain macros", True),
        ("Duplicate", False),
        ("Rename", False),
        ("Pin", False),
        ("Delete", False),
    ], "Chain on: Load plays 1, then 2, then 3. Chain off: Load only restores the save.",
       "A on Chain macros opens the numbered grid. The numbers stay when chain is off.")
    d = ImageDraw.Draw(img)
    iv.text(d, (1180, 168 + 36), "[ON]", ch.MF, fill=GREEN, anchor="rm")
    iv.text(d, (1180, 168 + 72), "1 Nair · 2 Grab · 3 Replay  >", ch.MF, fill=ch.M_TEXT, anchor="rm")
    save(img, "RECORDING-REDESIGN-save-actions")

    save(keyboard(menu), "RECORDING-REDESIGN-keyboard")

    save(status(game, "Make combo · step 1", [
        "Loaded the linked save, Nair loop · start.",
        "Then Nair loop plays on Player 1.",
        "The macro file is kept.",
    ]), "RECORDING-REDESIGN-combo-from-macro")

    save(status(game, "Loaded Corner pressure", [
        "Chain is on. Playing 1, then 2, then 3.",
        "1 Nair loop on Player 1.",
        "Then 2 Dash grab, then 3 Replay 86f.",
    ]), "RECORDING-REDESIGN-chain")

    img, d = page_base(menu, 1,
                       "Links 1 through 3 stay. Link 4 is left out.",
                       "A saves a new combo. The original file stays.",
                       "Up/Down move    A save a copy    B back")
    # Replace the combo library with the crop list by painting the right pane first.
    d.rectangle((328, 130, 1240, 560), fill=ch.M_PANEL)
    crop_rows = [
        {"name": "1  aerial → aerial", "c2": "kept", "c3": "first", "hot": True},
        {"name": "2  aerial → tilt", "c2": "kept", "c3": "", "hot": True},
        {"name": "3  tilt → smash", "c2": "kept", "c3": "last", "hot": True, "cursor": True},
        {"name": "4  smash → special", "c2": "left out", "c3": "", "hot": False, "dim": True},
    ]
    library_rows(d, crop_rows, "Range", "")
    save(img, "RECORDING-REDESIGN-crop")

    img, d = page_base(menu, 0,
                       "The same In and Out as the Replay window. L and R set them.",
                       "A saves a new macro. Nair loop itself stays.",
                       "L start    R end    Left/Right move    A save a copy    B back")
    d.rectangle((328, 130, 1240, 560), fill=ch.M_PANEL)
    setting_rows(d, [
        ("In", "40", "pick", True),
        ("Out", "210", "pick", False),
    ], y0=180)
    iv.text(d, (360, 280), "Pikachu · the frames between In and Out · a new file", SMALL, fill=ch.M_GROUP, anchor="lm")
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

    save(choices(), "RECORDING-REDESIGN-choices")

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

    img = menu.copy()
    d = ImageDraw.Draw(img)
    ch.clear_content(d, list_view=True)
    iv.text(d, (53, 112), "Where a save sits. The other pictures use the green one.", ch.MF, fill=ch.M_TEXT, anchor="lm")
    d.rounded_rectangle((48, 150, 1232, 310), radius=8, outline=GREEN, width=2)
    iv.text(d, (68, 166), "A  A section on both roles. Recommended.", iv.font(16, bold=True), fill=ch.M_TEXT, anchor="lm")
    iv.text(d, (68, 206), "Player and CPU stay the role tabs. R opens Save states on either tab.", SMALL, fill=ch.M_TEXT, anchor="lm")
    iv.text(d, (68, 236), "A save is both fighters, so it is not a third role. A still binds the row to Save and Load.", SMALL, fill=ch.M_GROUP, anchor="lm")
    d.rounded_rectangle((48, 330, 1232, 470), radius=8, outline=ch.M_RULE, width=2)
    iv.text(d, (68, 346), "B  A third top tab, Saves. Rejected.", iv.font(16, bold=True), fill=ch.M_TEXT, anchor="lm")
    iv.text(d, (68, 386), "Saves would sit beside Player and CPU. R would not be needed for it.", SMALL, fill=ch.M_GROUP, anchor="lm")
    iv.text(d, (68, 416), "Binding would be the same. The tab would read as a third role, which a save is not.", SMALL, fill=ch.M_GROUP, anchor="lm")
    save(img, "RECORDING-REDESIGN-save-where")

    chain_tiles = [
        {"name": "Nair loop", "sub": "Player · Pikachu", "number": 1},
        {"name": "Dash grab", "sub": "Player · Mario", "number": 2, "cursor": True},
        {"name": "Replay 86f", "sub": "Player · Pikachu", "number": 3},
        {"name": "CPU throw", "sub": "CPU · Mario"},
        {"name": "Slot 2", "sub": "Player · Steve"},
        {"name": "Macro 6", "sub": "Player · Lucina"},
        {"name": "CPU tech", "sub": "CPU · Mario"},
        {"name": "Slot 5", "sub": "Player · Villager"},
    ]
    save(order_grid(menu, "Recording  >  Save states  >  Corner pressure  >  Chain", chain_tiles,
                    "Red numbers are the order. Load plays 1, then 2, then 3.",
                    "A on a numbered tile removes it and the later numbers close up.",
                    "A add or remove    Y clear    B back"),
         "RECORDING-REDESIGN-chain-picker")

    combo_tiles = [
        {"name": "Ryu BnB", "sub": "Ryu → Mario · 6 links", "number": 1},
        {"name": "Nair loop", "sub": "Pikachu → Mario · 4 links", "number": 2, "cursor": True},
        {"name": "Combo 2", "sub": "Pikachu → Mario · 2 links", "number": 3},
        {"name": "Combo 4", "sub": "Steve → Pikachu · 3 links"},
    ]
    save(order_grid(menu, "Recording  >  Conversions  >  Corner to kill", combo_tiles,
                    "A conversion is numbered combos. Corner to kill is 1, then 2, then 3.",
                    "Combo 4 has no number, so it is not in this conversion.",
                    "A add or remove    Y clear    B back"),
         "RECORDING-REDESIGN-conversion-picker")

    save(action_screen(menu, "Recording  >  Conversions  >  Corner to kill", [
        ("Drill conversion", True),
        ("1  Ryu BnB", False),
        ("2  Nair loop", False),
        ("3  Combo 2", False),
        ("Rename", False),
        ("Pin", False),
        ("Delete", False),
    ], "Drill conversion is the whole chain. The helper then grades that chain.",
       "A on a numbered combo drills that combo alone and leaves the conversion file as it is."),
         "RECORDING-REDESIGN-conversion-actions")

    save(conversion_hud(
        game, "Corner to kill",
        "combo 2 of 3  ·  Ryu BnB  ·  6 links  ·  tolerance ±1f",
        [("L1", "true"), ("L2", "true"), ("L3", "now"), ("L4", "wait"), ("L5", "wait"), ("L6", "wait")],
        "2 of 3 combos  ·  link 2 of 6 true",
        "streak 4  ·  best 9",
    ), "RECORDING-REDESIGN-conversion-bar")

    dropped = conversion_hud(
        game, "Corner to kill",
        "combo 2 of 3  ·  Ryu BnB  ·  link 3 dropped",
        [("L1", "true"), ("L2", "true"), ("L3", "drop"), ("L4", "wait"), ("L5", "wait"), ("L6", "wait")],
        "Dropped. The conversion restarts at combo 1.",
        "streak 0  ·  best 9",
    )
    dd = ImageDraw.Draw(dropped)
    dd.rounded_rectangle((40, 28, 760, 168), radius=10, fill=(14, 18, 27, 232), outline=GREEN, width=2)
    iv.text(dd, (60, 48), "Completed, the other outcome", iv.font(16, bold=True), fill=WHITE, anchor="lm")
    iv.text(dd, (60, 84), "Every combo landed, in order, while the CPU was still in hitstun.", SMALL, fill=ch.M_GROUP, anchor="lm")
    iv.text(dd, (60, 112), "Conversion complete. Streak 5. Best 9.", iv.font(15, bold=True), fill=GREEN, anchor="lm")
    iv.text(dd, (60, 140), "The streak counts a finished conversion. A single combo keeps its own.", SMALL, fill=ch.M_GROUP, anchor="lm")
    save(dropped, "RECORDING-REDESIGN-conversion-drop")


# =====================================================================================================================
# Revision 4: one big grouped pop-up with a Links panel, the link diagram, loop only on macros and chains,
# the conversion bar at the top. Icons come from Material Symbols SVGs (docs/decisions/icons, Apache-2.0).
# =====================================================================================================================
import math  # noqa: E402
import re  # noqa: E402

from PIL import ImageChops  # noqa: E402

ICON_DIR = os.path.join(OUT, "icons")
TYPES = {
    "macro": ((104, 160, 244, 255), "MACRO"),
    "save": ((242, 152, 72, 255), "SAVE STATE"),
    "combo": ((172, 124, 244, 255), "COMBO"),
    "conv": ((236, 108, 176, 255), "CONVERSION"),
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


def pin4(img, x, y, size=20):
    """Material 'push_pin' (filled), tilted, gold, with a thin shadow so it reads on green and on grey."""
    icon(img, "push_pin_fill1", x + 1, y + 1, size, (0, 0, 0, 190), angle=35)
    icon(img, "push_pin_fill1", x, y, size, GOLD, angle=35)


def badge(d, cx, cy, n, r=10):
    d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=RED)
    iv.text(d, (cx, cy + 0.5), str(n), fnt(int(r * 1.3), bold=True), fill=WHITE, anchor="mm")


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
        icon(img, "link", x, cy - 9, 18, col)
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
    """A hint line: the button in white, what it does in grey; 'PIN' draws the thumbtack."""
    d = ImageDraw.Draw(img)
    fk, fl = fnt(size, bold=True), fnt(size)
    for key, label in parts:
        iv.text(d, (x, y), key, fk, fill=ch.M_TEXT, anchor="lm")
        x += d.textlength(key, font=fk) + 6
        if label == "PIN":
            pin4(img, x - 2, y - 11, 20)
            d = ImageDraw.Draw(img)
            x += 18
        else:
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
M4 = [
    dict(key="nair", name="Nair loop", fighter="Pikachu", role="P1", length="2.4 s", start="Nair loop · start",
         hot=True, pin=True),
    dict(key="dash", name="Dash grab", fighter="Pikachu", role="P1", length="1.1 s", start="Dash grab · start",
         hot=True, pin=True),
    dict(key="cputhrow", name="CPU throw", fighter="Mario", role="CPU", length="1.8 s", start="CPU throw · start",
         hot=True, pin=True),
    dict(key="replay", name="Replay 86f", fighter="Pikachu", role="P1", length="1.4 s", start=None, hot=True),
    dict(key="slot2", name="Slot 2 (2026-11-05)", fighter="Steve", role="P1", length="4.0 s", start=None, hot=True),
    dict(key="macro6", name="Macro 6", fighter="Lucina", role="P1", length="0.8 s", start="Macro 6 · start", hot=False),
    dict(key="slot5", name="Slot 5 (2026-11-05)", fighter="Villager", role="P1", length="3.2 s", start=None, hot=False),
]
C4 = [
    dict(key="ryu", name="Ryu BnB", fighters="Ryu → Mario", links="6 links", src=None, hot=True, pin=True),
    dict(key="nairc", name="Nair loop combo", fighters="Pikachu → Mario", links="4 links", src="Nair loop", hot=True),
    dict(key="combo2", name="Combo 2", fighters="Pikachu → Mario", links="2 links", src=None, hot=True),
    dict(key="combo4", name="Combo 4", fighters="Steve → Pikachu", links="3 links", src=None, hot=False),
]
V4 = [
    dict(key="corner", name="Corner to kill", seq=["Ryu BnB", "Nair loop combo"], hot=True, pin=True),
    dict(key="platstr", name="Platform strings", seq=["Combo 2", "Combo 4"], hot=True),
    dict(key="conv3", name="Conversion 3", seq=["Combo 4"], hot=False),
]
S4 = [
    dict(key="pressure", name="Corner pressure", fighters="Pikachu · Mario", chain=["nair", "dash", "cputhrow"],
         chain_on=True, hot=True, pin=True),
    dict(key="platform", name="Platform", fighters="Pikachu · Mario", chain=["dash", "replay"], chain_on=False,
         hot=True, pin=True),
    dict(key="slot1", name="Slot 1 (2026-11-05)", fighters="Steve · Villager", chain=[], hot=True),
    dict(key="nairstart", name="Nair loop · start", fighters="Pikachu · Mario", chain=[], hot=False),
    dict(key="dashstart", name="Dash grab · start", fighters="Pikachu · Mario", chain=[], hot=False),
    dict(key="throwstart", name="CPU throw · start", fighters="Pikachu · Mario", chain=[], hot=False),
    dict(key="slot3", name="Slot 3 (2026-11-05)", fighters="Lucina · Cloud", chain=[], hot=False),
]
MNAME = {m["key"]: m for m in M4}
DRILL = "corner"
BOUND = "pressure"
ACTIVE = {"player": "nair", "cpu": "cputhrow"}
LOOP = {"player": "on", "cpu": "1"}

# Column layouts: (field, x, caption). Pop-up list is 592 wide, the page list 530, a flow fragment 361.
COLS = {
    "pop": {
        "macro": [("fighter", 232, "Fighter"), ("length", 338, "Length"), ("start", 398, "Start save")],
        "combo": [("fighters", 232, "Fighters"), ("links", 366, "Links"), ("src", 434, "Made from")],
        "conv": [("seq", 232, "Contains, in order")],
        "save": [("fighters", 232, "Fighters"), ("chain", 398, "Chain")],
    },
    "page": {
        "macro": [("fighter", 214, "Fighter"), ("length", 306, "Length"), ("start", 362, "Start save")],
        "combo": [("fighters", 214, "Fighters"), ("links", 340, "Links"), ("src", 404, "Made from")],
        "conv": [("seq", 214, "Contains, in order")],
        "save": [("fighters", 214, "Fighters"), ("chain", 370, "Chain")],
    },
    "frag": {
        "macro": [("start", 196, "Start save")],
        "combo": [("src", 196, "Made from")],
        "conv": [("seq", 162, "Contains, in order")],
        "save": [("chain", 236, "Chain")],
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
    combos = {"title": "COMBOS & CONVERSIONS", "typ": "combo", "count": len(C4) + len(V4), "loop": None,
              "sections": [("Combos", "combo", rows(C4, "combo")), ("Conversions", "conv", rows(V4, "conv"))]}
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
        col = TYPES[g["typ"]][0]
        d.rounded_rectangle((0, y + 2, width - 1, y + HEAD_H - 2), radius=5, fill=HEAD_FOCUS if has_focus else HEAD_BG)
        cy = y + HEAD_H / 2
        d.rectangle((10, cy - 6, 22, cy + 6), fill=col)
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
    if r.get("pin"):
        pin4(img, 12, cy - 11, 20)
        d = ImageDraw.Draw(img)
    name_col = ch.M_TEXT if hot else ch.M_GROUP
    fn = fnt(15)
    iv.text(d, (34, cy), r["name"], fn, fill=name_col, anchor="lm")
    if r.get("tag"):
        tx = 34 + d.textlength(r["name"], font=fn) + 10
        ft = fnt(11, bold=True)
        tw = d.textlength(r["tag"], font=ft) + 14
        d.rounded_rectangle((tx, cy - 9, tx + tw, cy + 9), radius=9, fill=(196, 240, 214, 255))
        iv.text(d, (tx + tw / 2, cy + 0.5), r["tag"], ft, fill=INK, anchor="mm")
    cell = ch.M_GROUP if hot else DIM
    fc = fnt(14)
    for field, x, _ in cols:
        if field in ("fighter", "length", "fighters", "links"):
            v = r.get(field, "")
            if field == "fighter" and r.get("role") == "CPU":
                v += " · CPU"
            iv.text(d, (x, cy), v, fc, fill=cell, anchor="lm")
        elif field == "start":
            if r.get("start"):
                icon(img, "link", x, cy - 9, 18, TYPES["save"][0] if hot else DIM)
                d = ImageDraw.Draw(img)
                iv.text(d, (x + 22, cy), r["start"], fc, fill=cell, anchor="lm")
            else:
                iv.text(d, (x + 4, cy), "—  none", fnt(13), fill=DIM, anchor="lm")
        elif field == "src":
            if r.get("src"):
                iv.text(d, (x, cy), "from " + r["src"], fnt(13), fill=cell, anchor="lm")
        elif field == "seq":
            iv.text(d, (x, cy), " → ".join(r["seq"]), fc if width > 400 else fnt(13), fill=cell, anchor="lm")
        elif field == "chain":
            if r.get("chain"):
                icon(img, "format_list_numbered", x, cy - 9, 18, ch.M_TEXT if hot else DIM)
                d = ImageDraw.Draw(img)
                fb = fnt(14, bold=True)
                iv.text(d, (x + 24, cy), str(len(r["chain"])), fb, fill=ch.M_TEXT if hot else DIM, anchor="lm")
                on = r.get("chain_on")
                iv.text(d, (x + 40, cy), "On" if on else "Off", fb, fill=GREEN if on else DIM, anchor="lm")
            else:
                iv.text(d, (x + 4, cy), "—", fnt(13), fill=DIM, anchor="lm")


# ---- The Links panel: the focused row's links, as a small diagram --------------------------------------------------
def links_spec(key, role="player"):
    """What the Links panel shows for one row (the example data)."""
    if key in MNAME:
        m = MNAME[key]
        played = [(s["name"], s["chain"].index(key) + 1) for s in S4 if key in s["chain"]]
        made = [c["name"] for c in C4 if c.get("src") == m["name"]]
        return {
            "self": ("macro", m["name"]),
            "branches": [
                {"label": "starts from", "out": True,
                 "nodes": [("save", m["start"], None, None, True)] if m["start"] else [],
                 "none": "no start save"},
                {"label": "played by chains", "out": False,
                 "nodes": [("save", n, None, "#%d" % i, False) for n, i in played], "none": "none"},
                {"label": "combos made from it", "out": False,
                 "nodes": [("combo", n, None, None, False) for n in made],
                 "none": "none (Make combo plays on Player 1)" if m["role"] == "CPU" else "none"},
            ],
            "lines": [("Recorded on", "%s%s · %s" % (m["fighter"], " · CPU" if m["role"] == "CPU" else "", m["length"]),
                       ch.M_TEXT)],
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
                {"label": "in conversions", "out": False,
                 "nodes": [("conv", n, None, "#%d" % i, False) for n, i in convs], "none": "none"},
            ],
            "lines": [("Its links", "", None), ("CHIPS", ["L1", "L2", "L3", "L4"], None),
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
    on = s.get("chain_on")
    nodes = [("macro", MNAME[k]["name"], i + 1, "· " + MNAME[k]["role"], False) for i, k in enumerate(s["chain"])]
    lines = []
    if s["chain"] and on:
        lines.append(("LOOP", LOOP.get(role, "on"), None))
    elif s["chain"]:
        lines.append(("Chain Off", "Load only restores. The numbers stay.", ch.M_GROUP))
    lines.append(("Bound to Save / Load", "yes" if key == BOUND else "no", GREEN if key == BOUND else ch.M_TEXT))
    return {
        "self": ("save", s["name"]),
        "branches": [
            {"label": "plays on load, in order · Chain %s" % ("On" if on else "Off"), "out": True, "ordered": True,
             "nodes": nodes, "none": "no chain: Load only restores", "dim": not on},
        ],
        "lines": lines,
    }


def links_panel(img, box, spec, header=True):
    d = ImageDraw.Draw(img)
    x0, y0, x1, y1 = box
    y = y0
    if header:
        iv.text(d, (x0, y + 8), "LINKS", fnt(12, bold=True), fill=ch.M_GROUP, anchor="lm")
        iv.text(d, (x0 + 46, y + 8), "of the row under the cursor", fnt(12), fill=DIM, anchor="lm")
        y += 22
    typ, name = spec["self"]
    node4(img, (x0, y, x1, y + 38), typ, name, fsize=15, width=3)
    d = ImageDraw.Draw(img)
    spine_x = x0 + 16
    spine_top = y + 38
    spine_bot = spine_top
    y += 38 + 6
    for br in spec["branches"]:
        iv.text(d, (x0 + 34, y + 10), br["label"], fnt(13, bold=True), fill=ch.M_GROUP, anchor="lm")
        y += 22
        if not br["nodes"]:
            iv.text(d, (x0 + 34, y + 9), br["none"], fnt(13), fill=DIM, anchor="lm")
            y += 26
            continue
        for k, (ntyp, nname, nb, suffix, link) in enumerate(br["nodes"]):
            cy = y + 16
            col = TYPES[ntyp][0] if not br.get("dim") else DIM
            if br["out"]:
                arrow4(d, [(spine_x, cy), (x0 + 32, cy)], col, width=2, head=7)
            else:
                arrow4(d, [(x0 + 32, cy), (spine_x + 1, cy)], col, width=2, head=7)
            node4(img, (x0 + 34, y, x1, y + 32), ntyp, nname, badge_n=nb, suffix=suffix, link=link, fsize=14)
            d = ImageDraw.Draw(img)
            spine_bot = cy
            y += 32
            if br.get("ordered") and k < len(br["nodes"]) - 1:
                iv.text(d, (x0 + 60, y + 8), "then", fnt(11), fill=DIM, anchor="lm")
                y += 16
            else:
                y += 6
        y += 4
    d.line((spine_x, spine_top, spine_x, spine_bot), fill=(150, 156, 170, 255), width=2)
    y += 4
    for label, value, colour in spec["lines"]:
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
            y += 26
    return y


FOOT = {
    "macro": [("A", "set active"), ("Y", "PIN"), ("X", "loop"), ("ZL/ZR", "group"), ("L/R", "role"),
              ("ZL+Up/Down", "page"), ("B", "close")],
    "combo": [("A", "drill"), ("Y", "PIN"), ("ZL/ZR", "group"), ("L/R", "role"), ("ZL+Up/Down", "page"),
              ("B", "close")],
    "save": [("A", "bind"), ("Y", "PIN"), ("+", "edit chain"), ("ZL/ZR", "group"), ("L/R", "role"),
             ("ZL+Up/Down", "page"), ("B", "close")],
}
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
    """A scrolled list keeps the current group's header pinned at the top, so the group is always named."""
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


def auto_scroll(lst, pos, focus, view):
    """Top of the list if the cursor fits; else the cursor's group header at the top if that still shows the cursor;
    else the cursor a little above the middle."""
    if focus not in pos or pos[focus][1] + 12 <= view:
        return 0
    g_top = max(v[0] for k, v in pos.items() if k.startswith("__group") and v[0] <= pos[focus][0])
    if pos[focus][1] - g_top + 12 <= view:
        return int(min(lst.height - view, g_top))
    return int(min(lst.height - view, pos[focus][0] - view * 0.42))


def paste_list(img, lst, box, pos, focus, scroll=None):
    x0, y0, x1, y1 = box
    view = y1 - y0
    if scroll is None:
        scroll = auto_scroll(lst, pos, focus, view)
    scroll = max(0, min(scroll, max(0, lst.height - view)))
    img.alpha_composite(lst.crop((0, scroll, lst.width, scroll + view)), (x0, y0))
    sticky_header(img, lst, pos, scroll, x0, y0)
    if lst.height > view:
        d = ImageDraw.Draw(img)
        d.rounded_rectangle((x1 + 4, y0, x1 + 8, y1), radius=2, fill=(40, 43, 52, 255))
        th = view * view / lst.height
        ty = y0 + (view - th) * scroll / (lst.height - view)
        d.rounded_rectangle((x1 + 4, ty, x1 + 8, ty + th), radius=2, fill=(150, 156, 170, 255))
    return scroll


def popup4(game, role, focus, scroll=None, overrides=None):
    img = hide_notice4(game.copy())
    d = ImageDraw.Draw(img)
    d.rounded_rectangle(POP, radius=12, fill=WIN, outline=(86, 92, 108, 255), width=2)
    iv.text(d, (152, 102), "Recording", fnt(20, bold=True), fill=ch.M_TEXT, anchor="lm")
    role_toggle(d, 640, 86, role)
    d.line((144, 126, 1136, 126), fill=ch.M_RULE, width=1)
    groups = groups_for(role, focus, overrides)
    lst, pos, fg = render_list(groups, POP_LIST[2] - POP_LIST[0], COLS["pop"], focus)
    paste_list(img, lst, POP_LIST, pos, focus, scroll)
    d = ImageDraw.Draw(img)
    d.line((750, 136, 750, 596), fill=ch.M_DIVIDER, width=2)
    links_panel(img, POP_LINKS, links_spec(focus, role))
    d = ImageDraw.Draw(img)
    d.line((144, 604, 1136, 604), fill=ch.M_RULE, width=1)
    foot = list(FOOT[fg])
    if fg == "save" and {x["key"]: x for x in S4}[focus].get("chain_on"):
        foot.insert(2, ("X", "loop"))
    keys_line(img, 152, 626, foot, 14)
    return img


# ---- The Recording page in the menu -------------------------------------------------------------------------------
GROUPS4 = [("Macros", len(M4)), ("Combos & Conversions", len(C4) + len(V4)), ("Save states", len(S4)),
           ("Replays", 2), ("Options", 5)]
PAGE_LIST = (330, 132, 856, 562)
PAGE_LINKS = (874, 136, 1236, 562)


def page_base4(bg, selected, help1, help2, keys):
    img = bg.copy()
    d = ImageDraw.Draw(img)
    ch.clear_content(d)
    for i, (name, count) in enumerate(GROUPS4):
        y = ch.ROW_Y0 + i * ch.GROUP_STEP
        if i == selected:
            d.rectangle((33, y - 18, 307, y + 18), fill=ch.M_FOCUS)
        iv.text(d, (48, y), "%s  (%d)" % (name, count), ch.MF,
                fill=ch.M_TEXT if i == selected else ch.M_GROUP, anchor="lm")
    ch.help_lines(d, help1, help2)
    d.rectangle((44, 660, 1246, 698), fill=ch.M_PANEL)
    keys_line(img, 53, 678, keys, 14)
    return img


def page4(menu, selected, focus, help1, help2, keys):
    img = page_base4(menu, selected, help1, help2, keys)
    groups = groups_for("page", focus)
    want = {0: "macro", 1: "combo", 2: "save"}[selected]
    groups = [g for g in groups if g["typ"] == want]
    lst, pos, _ = render_list(groups, PAGE_LIST[2] - PAGE_LIST[0], COLS["page"], focus)
    paste_list(img, lst, PAGE_LIST, pos, focus)
    d = ImageDraw.Draw(img)
    d.line((866, 132, 866, 562), fill=ch.M_DIVIDER, width=2)
    links_panel(img, PAGE_LINKS, links_spec(focus, "player"))
    return img


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
    iv.text(d, (40, 30), "What links to what", fnt(24, bold=True), fill=ch.M_TEXT, anchor="lm")
    iv.text(d, (40, 60), "Four kinds of file. Four kinds of link. Each arrow says what the link means; the grey line "
            "says how you make it.", fnt(14), fill=ch.M_GROUP, anchor="lm")
    save_b, macro_b = (60, 150, 360, 250), (840, 150, 1140, 250)
    conv_b, combo_b = (60, 380, 360, 480), (840, 380, 1140, 480)
    big_node(img, save_b, "save", "Save state", "a named file: both fighters")
    big_node(img, macro_b, "macro", "Macro", "recorded inputs, one role")
    big_node(img, conv_b, "conv", "Conversion", "combos drilled as one chain")
    big_node(img, combo_b, "combo", "Combo", "the hits the helper grades")
    d = ImageDraw.Draw(img)
    sc, mc, vc, cc = (TYPES[k][0] for k in ("save", "macro", "conv", "combo"))
    # save -> macro (chain), above; macro -> save (start save), below
    arrow4(d, [(366, 180), (832, 180)], sc, width=3, head=13)
    link_label(d, 600, 164, "plays on load, in order", "made with the save's Edit chain (+), numbered like Character "
               "Cycles.\nChain On or Off is set on the save.", sc, how_first=True, width=470)
    arrow4(d, [(834, 222), (368, 222)], mc, width=3, head=13)
    link_label(d, 600, 240, "starts from", "automatic: Record writes \"<macro name> · start\" and links it", mc)
    # conversion -> combo
    arrow4(d, [(366, 430), (832, 430)], vc, width=3, head=13)
    link_label(d, 600, 414, "contains, in order", "made with Combos & Conversions › New conversion:\nthe same "
               "numbered picker, then a name on the Switch keyboard", vc, how_first=True)
    # combo -> macro
    arrow4(d, [(990, 374), (990, 258)], cc, width=3, head=13)
    link_label(d, 1006, 306, "made from", "a macro's Make combo", cc, anchor="lm", width=230)
    # The loop note under the two save/macro arrows.
    loop4(img, 60, 296, "on", 26)
    d = ImageDraw.Draw(img)
    iv.text(d, (96, 310), "Loop belongs to macros. On a chained save it repeats the whole chain.", fnt(13),
            fill=ch.M_GROUP, anchor="lm")

    # ---- the example ----
    d.line((40, 520, 1240, 520), fill=ch.M_RULE, width=1)
    iv.text(d, (40, 548), "Example", fnt(20, bold=True), fill=ch.M_TEXT, anchor="lm")
    iv.text(d, (150, 548), "the files every picture in this document uses", fnt(14), fill=ch.M_GROUP, anchor="lm")
    start_b = (60, 580, 400, 622)
    press_b = (60, 690, 400, 732)
    mac = [(760, 590, 1100, 632), (760, 654, 1100, 696), (760, 718, 1100, 760)]
    corner_b = (60, 836, 400, 878)
    combos = [(760, 806, 1100, 848), (760, 868, 1100, 910)]
    node4(img, start_b, "save", "Nair loop · start", link=True)
    node4(img, press_b, "save", "Corner pressure", suffix="Chain On")
    for i, (b, k) in enumerate(zip(mac, ["nair", "dash", "cputhrow"])):
        node4(img, b, "macro", MNAME[k]["name"], badge_n=i + 1, suffix="· " + MNAME[k]["role"])
    node4(img, corner_b, "conv", "Corner to kill")
    node4(img, combos[0], "combo", "Ryu BnB", badge_n=1)
    node4(img, combos[1], "combo", "Nair loop combo", badge_n=2)
    d = ImageDraw.Draw(img)
    # starts from: Nair loop -> Nair loop · start
    arrow4(d, [(754, 600), (408, 600)], mc, width=2, head=10)
    iv.text(d, (580, 588), "starts from", fnt(13, bold=True), fill=mc, anchor="mm")
    # plays on load, in order: Corner pressure -> 1, 2, 3
    for k, b in enumerate(mac):
        cy = (b[1] + b[3]) / 2 + (8 if k == 0 else 0)
        arrow4(d, [(406, 711), (630, 711), (670, cy), (752, cy)], sc, width=2, head=10)
    iv.text(d, (516, 696), "plays on load, in order", fnt(13, bold=True), fill=sc, anchor="mm")
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
    img = Image.new("RGBA", (W, 350), WIN)
    d = ImageDraw.Draw(img)
    iv.text(d, (40, 30), "How to read a list", fnt(22, bold=True), fill=ch.M_TEXT, anchor="lm")
    width = 560
    rows = [
        dict(key="a", name="Nair loop", fighter="Pikachu", role="P1", length="2.4 s", start="Nair loop · start",
             hot=True, pin=True, active=True),
        dict(key="b", name="Dash grab", fighter="Pikachu", role="P1", length="1.1 s", start="Dash grab · start",
             hot=True, pin=True, cursor=True),
        dict(key="c", name="Replay 86f", fighter="Pikachu", role="P1", length="1.4 s", start=None, hot=True),
        dict(key="d", name="Macro 6", fighter="Lucina", role="P1", length="0.8 s", start="Macro 6 · start", hot=False),
    ]
    groups = [{"title": "MACROS", "typ": "macro", "count": 7, "loop": [("", "on")],
               "sections": [(None, "macro", rows)]}]
    lst, pos, _ = render_list(groups, width, {"macro": COLS["pop"]["macro"]})
    img.alpha_composite(lst, (40, 64))
    both = Image.new("RGBA", (width, 40), WIN)
    draw_row(both, dict(name="Corner pressure", fighters="Pikachu · Mario", chain=["a", "b", "c"], chain_on=True,
                        hot=True, pin=True, active=True, cursor=True), "save", 4, 34, width, COLS["pop"]["save"])
    img.alpha_composite(both, (40, 64 + lst.height + 6))
    d = ImageDraw.Draw(img)
    notes = [
        (pos["a"], "Green fill: active. The macro that plays, the drill, or the bound save."),
        (pos["b"], "White border and bar: the cursor. A acts on this row."),
        (pos["c"], "Thin green outline: the hot five. Pins first, then the newest."),
        (pos["d"], "Under the hairline, dimmer: the rest of the library, on the card."),
        ((lst.height + 10, lst.height + 40, None), "Both: the cursor sits on the active row."),
    ]
    for (top, bot, _), text in notes:
        cy = 64 + (top + bot) / 2
        d.line((40 + width + 8, cy, 640, cy), fill=ch.M_RULE, width=1)
        iv.text(d, (650, cy), text, fnt(14), fill=ch.M_TEXT, anchor="lm")
    y = 64 + lst.height + 64
    x = 40
    items = [("pin", "pinned"), ("link", "start save (a link)"), ("list", "chain: macros, count, On/Off"),
             ("badge", "order, as Character Cycles"), ("loop", "loop (macros only)")]
    for kind, label in items:
        if kind == "pin":
            pin4(img, x, y - 11, 22)
        elif kind == "link":
            icon(img, "link", x, y - 11, 22, TYPES["save"][0])
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
    iv.text(d, (40, 228), "the MACROS group header, for the role on screen (Player and CPU each keep their own count);",
            fnt(15), fill=ch.M_GROUP, anchor="lm")
    iv.text(d, (40, 254), "a chained save's Links panel, where it repeats the whole chain. Never in COMBOS & "
            "CONVERSIONS or SAVE STATES headers.", fnt(15), fill=ch.M_GROUP, anchor="lm")
    return img


# ---- Fragments for the step-by-step strips -------------------------------------------------------------------------
PANEL_W, PANEL_H = 381, 330


def frag_list(img, box, groups, focus=None, scroll=None, extra_rows=None):
    width = box[2] - box[0]
    lst, pos, _ = render_list(groups, width, COLS["frag"], focus, extra_rows)
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
    img.alpha_composite(lst.crop((0, scroll, width, scroll + view)), (box[0], box[1]))
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
            v, is_link = values[label]
            if is_link:
                tw = d.textlength(v, font=fnt(13))
                icon(img, "link", x1 - 10 - tw - 22, y - 9, 18, TYPES["save"][0])
                d = ImageDraw.Draw(img)
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
    iv.text(d, (24, 28), "How a link is made · %d · %s" % (number, title), fnt(20, bold=True), fill=ch.M_TEXT,
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


def group_only(role, typ, focus, overrides=None, keep=None):
    g = [g for g in groups_for(role, focus, overrides) if g["typ"] == typ][0]
    if keep:
        g = dict(g, sections=[(s, t, [r for r in rows if r["key"] in keep]) for s, t, rows in g["sections"]
                              if any(r["key"] in keep for r in rows)])
    return g


def flow_record(game):
    img = strip_canvas(1, "Record → start save", "A recording always writes its own start save and links to it.")
    b = strip_panel(img, 0, "Record", "In the match, ZR + Down records. The pop-up's role, Player, says whose "
                    "inputs are recorded. ZR + Down again stops.")
    bottom = frag_game(img, b, hide_notice4(game.copy()))
    notice_plate(img, (b[0] + 40, b[1] + 14, b[2] - 40, b[1] + 66), "Recording · Player", "ZR + D-pad Down stops")
    d = ImageDraw.Draw(img)
    iv.text(d, (b[0], bottom + 18), "Record also saves the match as it is now:", fnt(13), fill=ch.M_GROUP, anchor="lm")
    iv.text(d, (b[0], bottom + 38), "that file becomes \"Macro 7 · start\".", fnt(13), fill=ch.M_GROUP, anchor="lm")

    b = strip_panel(img, 1, "Macro 7 is hot", "It stops. Macro 7 joins the hot five after the pins, with its start "
                    "save linked, and gets the cursor. The oldest unpinned macro, Slot 2, leaves the outline.")
    m7 = dict(key="m7", name="Macro 7", fighter="Pikachu", role="P1", length="3.0 s", start="Macro 7 · start",
              hot=True, new=True)
    g = group_only("player", "macro", "m7")
    rows = g["sections"][0][2]
    rows = rows[:3] + [dict(m7, cursor=True, active=False)] + [dict(rows[3])] + [dict(r, hot=False) for r in rows[4:5]]
    g = dict(g, sections=[(None, "macro", rows)], loop=[("", "on")], count=len(M4) + 1)
    frag_list(img, b, [g])

    b = strip_panel(img, 2, "The link, both ends", "The Links panel shows starts from Macro 7 · start. The new save "
                    "(gold outline, a mockup mark for new) is in SAVE STATES under the outline.")
    y = links_panel(img, (b[0], b[1], b[2], b[1] + 150), {
        "self": ("macro", "Macro 7"),
        "branches": [{"label": "starts from", "out": True, "nodes": [("save", "Macro 7 · start", None, None, True)],
                      "none": ""}],
        "lines": []}, header=False)
    sg = group_only("player", "save", None)
    srows = sg["sections"][0][2]
    new_save = dict(key="m7start", name="Macro 7 · start", fighters="Pikachu · Mario", chain=[], hot=False, new=True,
                    active=False, cursor=False)
    sg = dict(sg, sections=[(None, "save", [dict(srows[0]), new_save, dict(srows[3])])], count=len(S4) + 1)
    frag_list(img, (b[0], y + 8, b[2], b[3]), [sg])
    return img


def flow_chain():
    img = strip_canvas(2, "Chain a save", "A save's chain is a numbered list of macros. Load plays them in order "
                       "while Chain is On.")
    b = strip_panel(img, 0, "Focus a save, press +", "Corner pressure is the bound save (green) and the cursor. "
                    "+ opens Edit chain.")
    g = group_only("player", "save", "pressure", overrides={"pressure": {"chain": [], "chain_on": False}},
                   keep={"pressure", "platform", "slot1"})
    frag_list(img, (b[0], b[1], b[2], b[3] - 34), [g], "pressure")
    keys_line(img, b[0], b[3] - 12, [("A", "bind"), ("+", "edit chain"), ("B", "close")], 14)

    b = strip_panel(img, 1, "Number the macros", "A adds the next number. A again removes it and the later numbers "
                    "close up. Y clears. The same gesture as Character Cycles.")
    frag_tiles(img, b, "Edit chain · Corner pressure", [
        {"name": "Nair loop", "sub": "Player · Pikachu", "number": 1},
        {"name": "Dash grab", "sub": "Player · Pikachu", "number": 2},
        {"name": "CPU throw", "sub": "CPU · Mario", "number": 3, "cursor": True},
        {"name": "Replay 86f", "sub": "Player · Pikachu"},
        {"name": "Slot 2", "sub": "Player · Steve"},
        {"name": "Macro 6", "sub": "Player · Lucina"},
    ], [("A", "add / remove"), ("Y", "clear"), ("B", "done")])

    b = strip_panel(img, 2, "Linked", "Back in the list the row reads 3 On. The Links panel shows 1 → 2 → 3. "
                    "Chain On/Off is on the save's own screen.")
    row = [r for r in group_only("player", "save", "pressure")["sections"][0][2] if r["key"] == "pressure"][0]
    one_row(img, b[0], b[1], b[2] - b[0], row, "save")
    spec = dict(links_spec("pressure"), lines=[])
    links_panel(img, (b[0], b[1] + 44, b[2], b[3]), spec, header=False)
    return img


def flow_conversion():
    img = strip_canvas(3, "Make a conversion", "A conversion is a numbered list of combos, drilled as one chain.")
    b = strip_panel(img, 0, "New conversion", "Recording › Combos & Conversions. New conversion is the last row "
                    "under the conversions.")
    g = group_only("page", "combo", "newconv")
    secs = g["sections"]
    newrow = dict(key="newconv", name="+ New conversion", seq=[], hot=False, cursor=True)
    g = dict(g, sections=[secs[0], ("Conversions", "conv", secs[1][2] + [newrow])])
    frag_list(img, b, [g], "newconv")

    b = strip_panel(img, 1, "Number the combos", "The same numbered picker, with combos. Ryu BnB is 1, Nair loop "
                    "combo is 2.")
    frag_tiles(img, b, "New conversion", [
        {"name": "Ryu BnB", "sub": "Ryu → Mario", "number": 1},
        {"name": "Nair loop combo", "sub": "Pikachu → Mario", "number": 2, "cursor": True},
        {"name": "Combo 2", "sub": "Pikachu → Mario"},
        {"name": "Combo 4", "sub": "Steve → Pikachu"},
    ], [("A", "add / remove"), ("Y", "clear"), ("B", "done, then name it")])

    b = strip_panel(img, 2, "Name it", "OK on the Switch's own keyboard names it. The new row (gold: new) shows its "
                    "sequence, Ryu BnB → Nair loop combo.")
    frag_keyboard(img, (b[0] + 20, b[1], b[2] - 20, b[1] + 168), "Name conversion", "Corner to kill")
    d = ImageDraw.Draw(img)
    arrow4(d, [((b[0] + b[2]) / 2, b[1] + 174), ((b[0] + b[2]) / 2, b[1] + 196)], GREEN, width=3, head=9)
    row = [dict(V4[0], active=False, cursor=False, pin=False, hot=True)]
    row[0]["new"] = True
    lst = Image.new("RGBA", (b[2] - b[0], 40), WIN)
    draw_row(lst, row[0], "conv", 4, 34, b[2] - b[0], COLS["frag"]["conv"])
    img.alpha_composite(lst, (b[0], b[1] + 204))
    return img


def flow_make_combo(game):
    img = strip_canvas(4, "Make combo from a macro", "A combo made from a macro remembers which macro it came from.")
    b = strip_panel(img, 0, "Make combo", "Recording › Macros › Nair loop. A on Make combo. The first row is the "
                    "start save it will load.")
    frag_actions(img, b, "Macros  ›  Nair loop", ["Start save", "Set active for Player 1", "Set active for CPU",
                                                   "Rename", "Crop", "Make combo", "Pin", "Delete"], 5,
                 {"Start save": ("Nair loop · start", True)})

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

    b = strip_panel(img, 2, "The new combo", "Nair loop combo (gold: new) is hot and is the drill, one drill at a "
                    "time. Its row says from Nair loop; its Links panel says made from.")
    g = group_only("player", "combo", None, overrides={"_drill": "nairc"}, keep={"ryu", "nairc", "combo2"})
    g["sections"][0][2][1]["new"] = True
    frag_list(img, b, [g])
    links_panel(img, (b[0], b[1] + 160, b[2], b[3]), {
        "self": ("combo", "Nair loop combo"),
        "branches": [{"label": "made from", "out": True, "nodes": [("macro", "Nair loop", None, None, False)],
                      "none": ""}],
        "lines": []}, header=False)
    return img


def flow_drill(game, bar_picture):
    img = strip_canvas(5, "Drill a conversion", "One drill at a time: drilling a conversion replaces a drilled combo.")
    b = strip_panel(img, 0, "Pick it", "In the pop-up, the cursor on Corner to kill. A drills it.")
    g = group_only("player", "combo", "corner", overrides={"_drill": "nairc"})
    frag_list(img, (b[0], b[1], b[2], b[3] - 34), [g], "corner")
    keys_line(img, b[0], b[3] - 12, [("A", "drill"), ("ZL/ZR", "group"), ("B", "close")], 14)

    b = strip_panel(img, 1, "It is the drill", "The row turns green. Its Links panel lists the combos in order, "
                    "with the conversion's streak and best.")
    row = [r for r in group_only("player", "combo", "corner")["sections"][1][2] if r["key"] == "corner"][0]
    one_row(img, b[0], b[1], b[2] - b[0], row, "conv")
    links_panel(img, (b[0], b[1] + 44, b[2], b[3]), links_spec("corner"), header=False)

    b = strip_panel(img, 2, "The bar, at the top", "The helper bar sits under the frame strip, where the combo bar "
                    "is today. Full size below.")
    frag_game(img, b, bar_picture)
    return img


def conversion_bar4(game):
    img = hide_notice4(game.copy())
    o = iv.overlay()
    d = ImageDraw.Draw(o)
    x0, x1, _ = ch.strip(d)
    y0, y1 = 158, 300
    iv.panel(d, (x0, y0, x1, y1))
    cc = TYPES["conv"][0]
    iv.tag(d, (x0 + 8, y0 + 8, x0 + 112, y0 + 26), "CONVERSION", cc, fnt(11, bold=True), fg=INK)
    iv.text(d, (x0 + 122, y0 + 17), "Corner to kill", fnt(14, bold=True), fill=iv.TEXT, anchor="lm")
    iv.text(d, (x0 + 240, y0 + 17), "combo 2 of 2 · Nair loop combo · 4 links · tolerance ±1f", fnt(11),
            fill=iv.MUTED, anchor="lm")
    iv.text(d, (x1 - 12, y0 + 17), "streak 4 · best 9", fnt(12, bold=True), fill=iv.GOOD, anchor="rm")
    # The chain: combo 1 landed, combo 2 now.
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


# ---- Open choices --------------------------------------------------------------------------------------------------
CHOICES4 = [
    ("How you number them",
     "A  Name tiles. A adds the next number. A again removes it, and the later numbers close up.",
     "Same gesture as Character Cycles. A macro or a combo is a name, so the tile is the name, not a portrait.",
     "B  A list, and Y moves that file one place later."),
    ("Which role a chained macro plays",
     "A  The role stored in the macro file. A Player macro plays on Player 1. A CPU macro plays on the CPU.",
     "They play one after another. Starting both roles yourself is unchanged.",
     "B  Every macro in the chain plays on the role tab that was open when the save loaded."),
    ("What loop does with a chain",
     "A  The chain is one unit. Each macro plays once, then the whole list repeats.",
     "A chord Load uses the Player loop. A load from the CPU tab uses the CPU loop. The chained save's Links panel "
     "shows it.",
     "B  Each macro repeats by its own role's count before the next number starts."),
    ("A macro recorded on another fighter",
     "A  Switch or Keep is asked once, when the chain starts, and the answer covers the whole chain.",
     "The prompt names every mismatched macro. It is the prompt that already exists.",
     "B  Ask again at each macro whose fighter does not match."),
    ("Where a conversion lives",
     "A  Inside COMBOS & CONVERSIONS, after the combos: revision 4's layout, one group to jump to.",
     "Revision 3's A was a separate Conversions section. Revision 4 folds it into the combos' group.",
     "B  Its own group, CONVERSIONS, between COMBOS and SAVE STATES."),
    ("Hot conversions",
     "A  Five hot conversions, same pin and newest rules as macros and saves.",
     "Drilling one puts it in the hot list. A new one takes a slot even when all five are pinned.",
     "B  No hot list. Only the conversion you are drilling is marked."),
    ("What the conversion file stores",
     "A  The combo files' ids, in order. The same kind of link as a macro's start save.",
     "Renaming a combo keeps it. Editing the combo updates the conversion. A missing combo is skipped.",
     "B  A copy of each combo's links, frozen when the conversion was saved."),
    ("A drop, and what completes the chain",
     "A  A drop restarts the conversion at combo 1. Complete means every combo, in order, while the CPU is still "
     "in hitstun.",
     "The conversion streak and best count a finished conversion only. One combo drilled alone keeps its own streak.",
     "B  After a drop, continue from that combo. Back to back completes it, even if the CPU could act between."),
    ("One drill at a time",
     "A  Drilling a conversion replaces the drilled combo, and drilling a combo replaces the conversion.",
     "The helper has one bar. A on a numbered combo inside the conversion drills that combo alone.",
     "B  A combo and a conversion can both be active, and a button chooses which bar you see."),
]


def choices4():
    height = 56 + len(CHOICES4) * 150 + 16
    img = Image.new("RGBA", (W, height), WIN)
    d = ImageDraw.Draw(img)
    iv.text(d, (40, 26), "Still open. A is the recommendation and is what the pictures use.", fnt(20, bold=True),
            fill=ch.M_TEXT, anchor="lm")
    y = 56
    for n, (title, a, amid, b) in enumerate(CHOICES4, 1):
        d.rounded_rectangle((40, y, 1240, y + 138), radius=8, outline=ch.M_RULE, width=1)
        iv.text(d, (56, y + 18), "%d · %s" % (n, title), fnt(16, bold=True), fill=ch.M_TEXT, anchor="lm")
        tag_w = 128
        d.rounded_rectangle((1224 - tag_w, y + 8, 1224, y + 30), radius=11, fill=GREEN)
        iv.text(d, (1224 - tag_w / 2, y + 19.5), "recommend A", fnt(12, bold=True), fill=INK, anchor="mm")
        d.rectangle((56, y + 38, 1224, y + 70), fill=GREEN_ROW)
        iv.text(d, (68, y + 54), a, fnt(14), fill=ch.M_TEXT, anchor="lm")
        iv.text(d, (68, y + 88), amid, fnt(13), fill=ch.M_GROUP, anchor="lm")
        iv.text(d, (68, y + 116), b, fnt(14), fill=ch.M_GROUP, anchor="lm")
        y += 150
    return img


def seq_boxes(img, x, y, items, h=30):
    d = ImageDraw.Draw(img)
    for label, typ, state in items:
        if typ == "arrow":
            arrow4(d, [(x, y + h / 2), (x + 18, y + h / 2)], ch.M_GROUP, width=2, head=7)
            x += 26
            continue
        if typ == "text":
            iv.text(d, (x, y + h / 2), label, fnt(13), fill=ch.M_GROUP, anchor="lm")
            x += d.textlength(label, font=fnt(13)) + 10
            continue
        col = {"ok": GREEN, "drop": RED, "ask": GOLD}.get(state, TYPES[typ][0])
        f = fnt(13, bold=True)
        w = d.textlength(label, font=f) + 20
        d.rounded_rectangle((x, y, x + w, y + h), radius=6, fill=NODE_BG, outline=col, width=2)
        iv.text(d, (x + 10, y + h / 2), label, f, fill=ch.M_TEXT if state != "ask" else GOLD, anchor="lm")
        x += w + 8
    return x


def choices_ab():
    img = Image.new("RGBA", (W, 560), WIN)
    d = ImageDraw.Draw(img)
    iv.text(d, (40, 26), "Three choices drawn as A and B", fnt(20, bold=True), fill=ch.M_TEXT, anchor="lm")
    blocks = [
        ("What loop does with a chain (loop 2, chain 1 → 2 → 3)",
         [("1", "macro", ""), ("", "arrow", ""), ("2", "macro", ""), ("", "arrow", ""), ("3", "macro", ""),
          ("", "arrow", ""), ("1", "macro", ""), ("", "arrow", ""), ("2", "macro", ""), ("", "arrow", ""),
          ("3", "macro", ""), ("the whole chain twice", "text", "")],
         [("1", "macro", ""), ("", "arrow", ""), ("1", "macro", ""), ("", "arrow", ""), ("2", "macro", ""),
          ("", "arrow", ""), ("2", "macro", ""), ("", "arrow", ""), ("3", "macro", ""), ("", "arrow", ""),
          ("3", "macro", ""), ("each macro twice", "text", "")]),
        ("A drop in Corner to kill (combo 2 drops)",
         [("1 Ryu BnB", "combo", "ok"), ("", "arrow", ""), ("2 Nair loop combo", "combo", "drop"), ("", "arrow", ""),
          ("restart at 1 Ryu BnB", "combo", ""), ("streak 0, best stays", "text", "")],
         [("1 Ryu BnB", "combo", "ok"), ("", "arrow", ""), ("2 Nair loop combo", "combo", "drop"), ("", "arrow", ""),
          ("retry 2 Nair loop combo", "combo", ""), ("", "text", "")]),
        ("A chained macro recorded on another fighter (say 2 and 3 were)",
         [("Switch or Keep? names 2 and 3", "save", "ask"), ("", "arrow", ""), ("1", "macro", ""),
          ("", "arrow", ""), ("2", "macro", ""), ("", "arrow", ""), ("3", "macro", ""),
          ("asked once, the answer covers the chain", "text", "")],
         [("1", "macro", ""), ("", "arrow", ""), ("Switch or Keep? 2", "save", "ask"), ("", "arrow", ""),
          ("2", "macro", ""), ("", "arrow", ""), ("Switch or Keep? 3", "save", "ask"), ("", "arrow", ""),
          ("3", "macro", ""), ("asked at each one", "text", "")]),
    ]
    y = 62
    for title, a, b in blocks:
        iv.text(d, (40, y + 12), title, fnt(16, bold=True), fill=ch.M_TEXT, anchor="lm")
        d.rounded_rectangle((40, y + 30, 1240, y + 76), radius=8, fill=GREEN_ROW)
        iv.text(d, (56, y + 53), "A", fnt(16, bold=True), fill=GREEN, anchor="lm")
        seq_boxes(img, 84, y + 38, a)
        d = ImageDraw.Draw(img)
        iv.text(d, (56, y + 107), "B", fnt(16, bold=True), fill=ch.M_GROUP, anchor="lm")
        seq_boxes(img, 84, y + 92, b)
        d = ImageDraw.Draw(img)
        y += 160
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
            else:
                iv.text(d, (1210, y), v, ch.MF, fill=ch.M_GROUP, anchor="rm")
                if kind == "link":
                    icon(img, "link", 1210 - d.textlength(v, font=ch.MF) - 26, y - 10, 20, TYPES["save"][0])
                    d = ImageDraw.Draw(img)
        y += 36
    ch.help_lines(d, help1, help2)
    d.rectangle((44, 660, 1246, 698), fill=ch.M_PANEL)
    keys_line(img, 53, 678, keys or [("Up/Down", "move"), ("A", "do it"), ("B", "back")], 14)
    return img


def main():
    menu = iv.background(MENU)
    game = iv.background(GAME)
    P = "RECORDING-REDESIGN-r4-"

    save(diagram(), P + "links")
    save(legend(), P + "legend")
    save(loop_picture(), P + "loop")

    save(popup4(game, "player", "nairc"), P + "popup-player")
    save(popup4(game, "player", "nair"), P + "popup-macro")
    save(popup4(game, "player", "corner"), P + "popup-conversion")
    save(popup4(game, "player", "pressure"), P + "popup-save")
    save(popup4(game, "cpu", "cputhrow"), P + "popup-cpu")

    save(page4(menu, 0, "dash",
               "Two green rows: Nair loop plays on Player 1, CPU throw on the CPU. The cursor is on Dash grab.",
               "The Links panel is Dash grab's: its start save, and the two chains that number it.",
               [("A", "actions"), ("Y", "PIN"), ("ZL/ZR", "group"), ("L/R", "page"), ("B", "close")]),
         P + "page-macros")
    save(page4(menu, 2, "platform",
               "Corner pressure is the bound save (green). The cursor is on Platform, whose chain is Off.",
               "Off keeps the numbers; Load only restores. No loop here: it only applies to a chain that is On.",
               [("A", "actions"), ("Y", "PIN"), ("+", "edit chain"), ("ZL/ZR", "group"), ("L/R", "page"),
                ("B", "close")]),
         P + "page-saves")

    bar = conversion_bar4(game)
    save(flow_record(game), P + "flow-record")
    save(flow_chain(), P + "flow-chain")
    save(flow_conversion(), P + "flow-conversion")
    save(flow_make_combo(game), P + "flow-make-combo")
    save(flow_drill(game, bar), P + "flow-drill")
    save(bar, P + "conversion-bar")

    save(choices4(), P + "choices")
    save(choices_ab(), P + "choices-ab")

    save(action_screen4(menu, "Recording  >  Save states  >  Corner pressure", [
        ("Bind to Save / Load", False), ("Chain", False), ("Edit chain", True), ("Duplicate", False),
        ("Rename", False), ("Pin", False), ("Delete", False),
    ], "Chain On: Load plays 1, then 2, then 3. Chain Off: Load only restores the save; the numbers stay.",
        "Edit chain is the numbered picker. + opens the same picker from the list and the pop-up.",
        {"Bind to Save / Load": ("bound", "on"), "Chain": ("[ON]", "on"),
         "Edit chain": ("1 Nair loop  ·  2 Dash grab  ·  3 CPU throw  >", "text")}), P + "save-actions")
    save(action_screen4(menu, "Recording  >  Combos & Conversions  >  Corner to kill", [
        ("Drill conversion", True), ("1  Ryu BnB", False), ("2  Nair loop combo", False), ("Edit combos", False),
        ("Rename", False), ("Pin", False), ("Delete", False),
    ], "Drill conversion grades the whole chain. A on a numbered combo drills that combo alone.",
        "Edit combos is the numbered picker. Deleting the conversion keeps the combo files."),
        P + "conversion-actions")
    save(action_screen4(menu, "Recording  >  Combos & Conversions  >  Nair loop combo", [
        ("Drill", True), ("Made from", False), ("Rename", False), ("Crop", False), ("Pin", False),
        ("Delete", False),
    ], "Drill makes this the one drill and clears a drilled conversion.",
        "Made from names the macro it came from. Crop asks for the first and last link and saves a copy.",
        {"Made from": ("Nair loop", "text")}), P + "combo-actions")


if __name__ == "__main__":
    if "--r3" in sys.argv[1:]:
        main_r3()
    else:
        main()
