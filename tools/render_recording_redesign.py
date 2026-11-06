#!/usr/bin/env python3
"""Revision 3 of the recording-redesign mockups.

Menu screens are redrawn on the real rc.1 Recording page (the COMBO-HELPER method).
The pop-up, the chain, and combo-from-macro sit on a real Eden Training frame from
artifacts/phase23. The Switch keyboard is a labelled schematic, not a custom keyboard.

Run from the repo root:
  /usr/bin/python3 -I tools/render_recording_redesign.py
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


def main():
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


if __name__ == "__main__":
    main()
