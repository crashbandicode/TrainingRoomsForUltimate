#!/usr/bin/env python3
"""COMBO-HELPER decision mockups: the Recording page's Combos section and two in-game helper layouts, drawn over real
Eden frames.

Usage: render_combo_helper.py <menu_background.png> <game_background.png> <out_dir>

Both backgrounds are Eden captures (1280x674 with side bars); render_input_viz.background() crops the 16:9 game area
and scales it to 1280x720. The menu frame is the real Recording page of 0.2.0-rc.1 (run tr-p23-docs-capture-03); only
the content area is redrawn, in the menu's own colours and layout. The game frame is a real Training frame (run
p4zc-rc9: P1 Steve, CPU Pikachu, the only Eden frame with Pikachu on screen). Every overlay element is drawn here from
rectangles, lines and text; the strip parts reuse render_input_viz.py (the frame strip, INPUT-VIZ option A), so the
helper reads as the same system. No game, fork or other-game artwork is used, and the game's own keyboard is not
drawn. All numbers are illustrative: a plausible P1 Pikachu short-hop nair loop of five nairs (four links).
"""
import os
import sys

from PIL import Image, ImageDraw

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import render_input_viz as iv  # noqa: E402  (the frame strip's primitives and palette)

W, H = iv.W, iv.H
font, text, tag, chip, panel, hatch = iv.font, iv.text, iv.tag, iv.chip, iv.panel, iv.hatch
GOOD, BAD, WARN, TEXT, MUTED, STATE = iv.GOOD, iv.BAD, iv.WARN, iv.TEXT, iv.MUTED, iv.STATE

# ---- The real menu's colours and grid (sampled from the rc.1 Recording page frame) ---------------------------------
M_PANEL = (13, 13, 13, 255)
M_FOCUS = (39, 42, 56, 255)
M_TEXT = (241, 242, 246, 255)
M_GROUP = (160, 166, 178, 255)
M_ON = (118, 226, 172, 255)
M_OFF = (113, 120, 129, 255)
M_RULE = (68, 75, 88, 255)
M_DIVIDER = (38, 41, 47, 255)
NEW = (255, 196, 64, 255)          # mockup annotation only: marks what this proposal adds
MF = font(15)
ROW_Y0, ROW_STEP = 153, 36         # row centres
GROUP_STEP = 37.5
LABEL_X, VALUE_X = 346, 1227

# ---- The illustrative combo: P1 Pikachu, five short-hop nairs, four links -----------------------------------------
# Per link: (verdict link, reference link, jump press vs A1, reference, nair press after the jump, reference).
LINKS = [
    (-2, -3, 1, 0, 5, 5),
    (-1, -2, 0, 0, 6, 5),
    (+1, -1, 2, 0, 5, 5),
    (-1, -1, 0, 0, 5, 5),
]
TOLERANCE = 1
HISTORY = [(3, 4, None), (4, 4, None), (4, 4, None), (2, 4, 3), (3, 4, None), (4, 4, None), (1, 4, 2), (3, 4, None)]
# History entries: (true links, links, stopped at link or None), newest first.

# The strip's last 60 frames (frame 0 = Pikachu's first actionable frame after nair 3's last hit is at 7).
P1_RUNS = [(-7, -5, "active"), (-4, -2, "hitlag"), (-1, 0, "active"), (1, 6, "recovery"), (7, 8, "idle"),
           (9, 11, "recovery"), (12, 13, "idle"), (14, 16, "startup"), (17, 19, "hitlag"), (20, 24, "active"),
           (25, 30, "recovery"), (31, 33, "recovery"), (34, 35, "idle"), (36, 38, "startup"), (39, 41, "hitlag"),
           (42, 46, "active"), (47, 52, "recovery")]
CPU_RUNS = [(-7, -5, "hitstun"), (-4, -2, "hitlag"), (-1, 15, "hitstun"), (16, 16, "idle"), (17, 19, "hitlag"),
            (20, 38, "hitstun"), (39, 41, "hitlag"), (42, 52, "hitstun")]
INPUTS = [(9, None, "X"), (14, None, "A"), (31, None, "X"), (36, None, "A")]


def sign(n):
    return ("−" if n < 0 else "+" if n > 0 else "±") + str(abs(n))


def note(d, box, title, lines, recommended=None):
    """The mockup annotation (not part of the HUD)."""
    outline = (40, 170, 80, 255) if recommended else (90, 96, 110, 255)
    panel(d, box, radius=8, fill=(250, 250, 252, 238), outline=outline, width=4 if recommended else 2)
    text(d, (box[0] + 14, box[1] + 10), title, font(17, bold=True), fill=(20, 24, 32, 255))
    y = box[1] + 36
    for line in lines:
        text(d, (box[0] + 14, y), line, font(13), fill=(46, 52, 64, 255))
        y += 18


def new_tag(d, x, y):
    tag(d, (x, y - 9, x + 40, y + 9), "NEW", NEW, font(10, bold=True))


# ---- (a) The Recording page: a Combos group -----------------------------------------------------------------------
GROUPS = [("Recording", 4), ("Playback", 6), ("Named macros", 5), ("Combos", 7)]
ROWS = [
    ("Name (optional)", "Pika nair loop", "action"),
    ("Save last combo...", "Pikachu · 4 links · 1.6 s", "action"),
    ("Saved combos", "3 saved", "action"),
    ("Combo helper", "[ON]", "on"),
    ("Target combo", "Pika nair loop", "pick"),
    ("On-time tolerance", "±1 frame", "pick"),
    ("After an attempt", "Load save state", "pick"),
]
FOCUS = 0


def clear_content(d, list_view=False):
    """Paint the content area (both columns) and the help lines with the panel colour; keep tabs and hints."""
    d.rectangle((30, 120 if list_view else 128, 1246, 566), fill=M_PANEL)
    d.rectangle((44, 590, 1246, 656), fill=M_PANEL)
    if not list_view:
        d.line([(319, 130), (319, 562)], fill=M_DIVIDER, width=2)


def help_lines(d, first, second):
    text(d, (53, 608), first, MF, fill=M_TEXT, anchor="lm")
    text(d, (53, 640), second, MF, fill=M_TEXT, anchor="lm")


def menu_page(bg, annotate=True):
    img = bg.copy()
    d = ImageDraw.Draw(img)
    clear_content(d)
    for i, (name, count) in enumerate(GROUPS):
        y = ROW_Y0 + i * GROUP_STEP
        if i == 3:
            d.rectangle((33, y - 18, 307, y + 18), fill=M_FOCUS)
        text(d, (40, y), f"{name}  ({count})", MF, fill=M_TEXT if i == 3 else M_GROUP, anchor="lm")
    new_tag(d, 160, ROW_Y0 + 3 * GROUP_STEP)
    for i, (label, value, kind) in enumerate(ROWS):
        y = ROW_Y0 + i * ROW_STEP
        if i == FOCUS:
            d.rectangle((337, y - 17, 1236, y + 17), fill=M_FOCUS)
        text(d, (LABEL_X, y), label, MF, fill=M_TEXT, anchor="lm")
        if kind == "on":
            text(d, (VALUE_X, y), value, MF, fill=M_ON, anchor="rm")
        elif kind == "pick":
            text(d, (VALUE_X, y), f"<  {value}  >", MF, fill=M_TEXT, anchor="rm")
        else:
            w = d.textlength(">", font=MF)
            text(d, (VALUE_X, y), ">", MF, fill=M_TEXT, anchor="rm")
            text(d, (VALUE_X - w - 16, y), value, MF, fill=M_GROUP, anchor="rm")
    help_lines(d, "A name for the next combo you save. A opens the Switch keyboard.",
               "Optional: left empty, the combo is saved as \"Combo 4\".")
    if not annotate:
        return img
    o = iv.overlay()
    od = ImageDraw.Draw(o)
    note(od, (560, 400, 1236, 562), "Mockup · Recording page, new Combos group", [
        "A real rc.1 menu frame; only the content area is redrawn, in the menu's",
        "own colours. Name (optional): A opens the Switch keyboard; the name is",
        "used by the next Save last combo, which keeps the hits since both",
        "fighters could last act (the frame strip's exchange). Combo helper On:",
        "each attempt is measured against the Target combo. After an attempt:",
        "Keep playing, or Load save state (retry from the same spot).",
        "Values are illustrative.",
    ])
    img.alpha_composite(o)
    return img


def menu_keyboard(bg):
    """The Name row's A: the Switch's own keyboard opens full screen over the game (ShowKeyboard). It is the
    system's UI, so it is drawn here only as a labelled schematic: what TrainingRooms passes in (the header text,
    the current name, the 48-character limit) and what comes back."""
    img = menu_page(bg, annotate=False)
    o = iv.overlay()
    d = ImageDraw.Draw(o)
    d.rectangle((0, 0, W, H), fill=(6, 8, 12, 214))
    sheet = (150, 40, 1130, 575)
    d.rounded_rectangle(sheet, radius=14, fill=(226, 229, 235, 255), outline=(255, 196, 64, 255), width=3)
    text(d, (sheet[0] + 24, sheet[1] + 22), "SCHEMATIC · the Switch's own keyboard (system UI, not drawn from it)",
         font(13, bold=True), fill=(150, 104, 0, 255))
    text(d, ((sheet[0] + sheet[2]) / 2, sheet[1] + 72), "Name combo", font(22, bold=True), fill=(24, 28, 36, 255),
         anchor="mm")
    field = (sheet[0] + 120, sheet[1] + 102, sheet[2] - 120, sheet[1] + 152)
    d.rounded_rectangle(field, radius=6, fill=(255, 255, 255, 255), outline=(120, 128, 140, 255), width=2)
    typed = "Pika nair loop"
    text(d, (field[0] + 16, (field[1] + field[3]) / 2), typed, font(20), fill=(24, 28, 36, 255), anchor="lm")
    cx = field[0] + 16 + d.textlength(typed, font=font(20)) + 3
    d.line([(cx, field[1] + 12), (cx, field[3] - 12)], fill=(40, 120, 220, 255), width=2)
    text(d, (field[2] - 12, (field[1] + field[3]) / 2), f"{len(typed)} / 48", font(13), fill=(110, 118, 130, 255),
         anchor="rm")
    # A generic key grid: plain rounded squares, no system artwork.
    kx0, ky0, key, gap = sheet[0] + 120, sheet[1] + 190, 62, 8
    for row, count in enumerate((10, 10, 9, 7)):
        indent = (10 - count) * (key + gap) / 2
        for k in range(count):
            x = kx0 + indent + k * (key + gap)
            y = ky0 + row * (key + gap)
            d.rounded_rectangle((x, y, x + key, y + key), radius=8, fill=(248, 249, 251, 255),
                                outline=(170, 176, 188, 255), width=1)
    sy = ky0 + 4 * (key + gap)
    d.rounded_rectangle((kx0 + 2 * (key + gap), sy, kx0 + 7 * (key + gap) - gap, sy + key - 14), radius=8,
                        fill=(248, 249, 251, 255), outline=(170, 176, 188, 255), width=1)
    ok = (kx0 + 8 * (key + gap), sy, kx0 + 10 * (key + gap) - gap, sy + key - 14)
    d.rounded_rectangle(ok, radius=8, fill=(64, 150, 230, 255))
    text(d, ((ok[0] + ok[2]) / 2, (ok[1] + ok[3]) / 2), "OK", font(16, bold=True), fill=(255, 255, 255, 255),
         anchor="mm")
    img.alpha_composite(o)
    lo = iv.overlay()
    ld = ImageDraw.Draw(lo)
    note(ld, (700, 588, 1268, 712), "Mockup · Name (optional) → the Switch keyboard", [
        "Training: ShowKeyboard (preset default), header \"Name combo\", initial text",
        "= the current name, 48 characters at most. Waiting room: the inline keyboard",
        "Mii naming already uses. OK stores the name; B (cancel) keeps the old one;",
        "OK on an empty field saves as \"Combo N\". The name appears in Saved combos.",
    ])
    img.alpha_composite(lo)
    return img


SAVED = [
    ("Pika nair loop", "Pikachu → Mario", 4, "1.6 s", "30%", "11-05 13:02", True),
    ("Uair ladder", "Pikachu → Mario", 3, "1.1 s", "60%", "11-05 13:10", False),
    ("Combo 3", "Steve → Pikachu", 2, "0.6 s", "0%", "11-05 13:21", False),
]


def menu_list(bg):
    img = bg.copy()
    d = ImageDraw.Draw(img)
    clear_content(d, list_view=True)
    text(d, (53, 118), "Recording  >  Combos  >  Saved combos", MF, fill=M_TEXT, anchor="lm")
    text(d, (400, 118), "3 of 3", MF, fill=M_GROUP, anchor="lm")
    cols = [(86, "Name"), (360, "Fighters (P1 → CPU)"), (640, "Links"), (730, "Length"), (840, "CPU from"),
            (980, "Recorded")]
    small = font(13)
    for x, name in cols:
        text(d, (x, 158), name, small, fill=M_GROUP, anchor="lm")
    d.line([(53, 174), (1227, 174)], fill=M_RULE, width=1)
    for i, (name, fighters, links, length, start, when, target) in enumerate(SAVED):
        y = 200 + i * ROW_STEP
        if i == 0:
            d.rectangle((47, y - 17, 1236, y + 17), fill=M_FOCUS)
        if target:
            d.polygon([(62, y), (70, y - 8), (78, y), (70, y + 8)], fill=M_ON)
        for (x, _), value in zip(cols, (name, fighters, str(links), length, start, when)):
            text(d, (x, y), value, MF, fill=M_TEXT, anchor="lm")
        if target:
            tag(d, (1140, y - 10, 1224, y + 10), "target", M_ON, font(11, bold=True))
    help_lines(d, "Pika nair loop: Pikachu on Mario, 4 links, recorded with Mario at 30%.",
               "A makes it the target; X renames it with the keyboard; Y deletes it.")
    # The hints line of the list (the page's line, rewritten for this screen).
    d.rectangle((44, 660, 1246, 690), fill=M_PANEL)
    text(d, (53, 675), "Up/Down move   A set target   X rename   Y delete   B back", font(14), fill=M_GROUP,
         anchor="lm")
    o = iv.overlay()
    od = ImageDraw.Draw(o)
    # Where the game's own keyboard opens (a placeholder: the keyboard is the game's and is not drawn).
    kb = (53, 330, 540, 524)
    od.rounded_rectangle(kb, radius=8, fill=(255, 196, 64, 28))
    for x in range(kb[0], kb[2], 14):
        od.line([(x, kb[1]), (min(x + 7, kb[2]), kb[1])], fill=NEW, width=2)
        od.line([(x, kb[3]), (min(x + 7, kb[2]), kb[3])], fill=NEW, width=2)
    for y in range(kb[1], kb[3], 14):
        od.line([(kb[0], y), (kb[0], min(y + 7, kb[3]))], fill=NEW, width=2)
        od.line([(kb[2], y), (kb[2], min(y + 7, kb[3]))], fill=NEW, width=2)
    text(od, ((kb[0] + kb[2]) / 2, kb[1] + 62), "X rename: the Switch keyboard opens", font(16, bold=True),
         fill=NEW, anchor="mm")
    text(od, ((kb[0] + kb[2]) / 2, kb[1] + 90), "header \"Name combo\", the current name filled in", font(13),
         fill=TEXT, anchor="mm")
    text(od, ((kb[0] + kb[2]) / 2, kb[1] + 112), "48 characters at most · B keeps the old name", font(13),
         fill=TEXT, anchor="mm")
    text(od, ((kb[0] + kb[2]) / 2, kb[1] + 134), "OK on an empty field: \"Combo N\"", font(13), fill=TEXT,
         anchor="mm")
    note(od, (560, 330, 1236, 524), "Mockup · Saved combos", [
        "One file per combo in sd:/ultimate/TrainingRooms/combos/<id>.json, on the",
        "same Store as the named-macro and setup libraries (atomic writes, ids).",
        "Each holds P1's inputs, the per-link reference the frame strip measured,",
        "and the context: both fighters, the CPU's damage and DI settings.",
        "The names are the ones typed on the Switch keyboard (Name (optional)",
        "before saving, or X rename here); the name lives in the combo's file.",
        "The diamond and \"target\" mark the Target combo. Values illustrative.",
    ])
    img.alpha_composite(o)
    return img


# ---- (b) The in-game helper ---------------------------------------------------------------------------------------
def strip(d, x0=196, y0=8):
    """The frame strip (INPUT-VIZ A layout) with the nair-loop frames: links 3 and 4 on screen."""
    pitch, cw = 9, 8
    gx = x0 + 56
    col = lambda f: gx + (f - iv.FIRST) * pitch
    nx = col(iv.LAST) + cw + 3
    rx = nx + 18
    x1, y1 = rx + 248, y0 + 144
    panel(d, (x0, y0, x1, y1))
    lane_in, lane_p1, lane_cpu = y0 + 10, y0 + 48, y0 + 72
    lh = 19
    small, tiny = font(11, bold=True), font(10)
    tag(d, (x0 + 8, lane_in + 3, x0 + 48, lane_in + 21), "IN", (60, 68, 84, 255), small, fg=TEXT)
    tag(d, (x0 + 8, lane_p1 + 1, x0 + 48, lane_p1 + 18), "P1", iv.P1_TINT, small, fg=(255, 255, 255, 255))
    tag(d, (x0 + 8, lane_cpu + 1, x0 + 48, lane_cpu + 18), "CPU", iv.CPU_TINT, small, fg=(20, 22, 28, 255))
    # SPEC-input-06's window for the latest follow-up: A1 (31) to S2 - link (36 + 1 = 37), for the nair press.
    d.rectangle((col(31), lane_p1 - 8, col(37) + cw, lane_p1 - 3), fill=GOOD)
    text(d, (col(37) + cw + 5, lane_p1 - 6), "combo window 7f", tiny, fill=GOOD, anchor="lm")
    for runs, top in ((P1_RUNS, lane_p1), (CPU_RUNS, lane_cpu)):
        for a, b, s in runs:
            for f in range(a, b + 1):
                box = (col(f), top, col(f) + cw, top + lh)
                if s == "hitlag":
                    hatch(d, box, STATE[s])
                else:
                    d.rectangle(box, fill=STATE[s])
    for a, b, s in CPU_RUNS:
        if s == "hitstun" and b - a >= 6:
            text(d, ((col(a) + col(b) + cw) / 2, lane_cpu + lh / 2), str(b - a + 1), small,
                 fill=(255, 255, 255, 255), anchor="mm")
    for f, stick, button in INPUTS:
        cx = col(f) + cw / 2
        d.line([(cx, lane_in + 22), (cx, lane_p1)], fill=(255, 255, 255, 170), width=1)
        chip(d, cx - 10, lane_in + 3, stick, button, colour=iv.ATTACK_CHIP)
    by = lane_cpu + lh + 4
    for a, b, label, colour in ((col(16), col(17), "link +1", BAD), (col(39), col(40), "link −1", GOOD)):
        d.line([(a, by), (a, by + 6), (b + cw, by + 6), (b + cw, by)], fill=colour, width=2)
        text(d, ((a + b + cw) / 2, by + 8), label, small, fill=colour, anchor="ma")
    d.line([(nx, lane_in), (nx, lane_cpu + lh)], fill=(255, 255, 255, 120), width=1)
    text(d, (nx - 3, lane_in), "now", tiny, fill=MUTED, anchor="ra")
    d.line([(rx - 9, y0 + 10), (rx - 9, y0 + 108)], fill=iv.EDGE, width=1)
    big, lab = font(21, bold=True, mono=True), font(11, bold=True)
    for i, (name, value, colour, sub) in enumerate([("ADVANTAGE", "—", MUTED, "exchange running"),
                                                     ("LINK", "−1", GOOD, "TRUE combo ✓"),
                                                     ("INPUT", "+5", WARN, "5f late · 1 spare")]):
        yy = y0 + 6 + i * 37
        text(d, (rx, yy), name, lab, fill=MUTED)
        text(d, (rx, yy + 12), value, big, fill=colour)
        text(d, (rx + 50, yy + 17), sub, font(12, bold=True), fill=TEXT)
    hy = y0 + 122
    d.line([(x0 + 10, hy - 3), (x1 - 10, hy - 3)], fill=iv.EDGE, width=1)
    text(d, (x0 + 12, hy + 8), "LAST", small, fill=MUTED, anchor="lm")
    hx = gx
    for label, ok in (("TRUE −1", True), ("DROP +1", False), ("TRUE −1", True), ("TRUE −2", True)):
        tag(d, (hx, hy, hx + 64, hy + 16), label, GOOD if ok else BAD, font(10, bold=True))
        text(d, (hx + 70, hy + 8), "Aerial → Aerial", tiny, fill=TEXT, anchor="lm")
        hx += 70 + d.textlength("Aerial → Aerial", font=tiny) + 18
    return x0, x1, y1


def verdict_pill(d, box, link):
    tag(d, box, ("TRUE " if link <= 0 else "DROP ") + sign(link).replace("±", ""), GOOD if link <= 0 else BAD,
        font(11, bold=True))


def timing_ruler(d, x, y, lo, hi, step, window, ref, you, h=10):
    """Frames lo..hi from P1's first actionable frame: the true-combo window green, the reference as a hollow tick,
    the attempt as a filled triangle (green within the tolerance, amber outside it but still true, red when it
    dropped)."""
    for f in range(lo, hi + 1):
        fill = GOOD if window[0] <= f <= window[1] else (70, 78, 94, 255)
        d.rectangle((x + (f - lo) * step, y, x + (f - lo) * step + step - 2, y + h), fill=fill)
        if f == 0:
            text(d, (x + (f - lo) * step + (step - 2) / 2, y + h + 2), "0", font(9), fill=MUTED, anchor="ma")
    rx = x + (ref - lo) * step + (step - 2) / 2
    d.rectangle((rx - 3, y - 3, rx + 3, y + h + 3), outline=(255, 255, 255, 255), width=2)
    yx = x + (you - lo) * step + (step - 2) / 2
    off = you - ref
    colour = GOOD if abs(off) <= TOLERANCE and you <= window[1] else (BAD if you > window[1] else WARN)
    d.polygon([(yx - 6, y - 9), (yx + 6, y - 9), (yx, y - 1)], fill=colour)


def link_window(ref_link, ref_jump):
    """The jump presses that still make a true combo, from the reference: up to ref_jump − ref_link."""
    return (0, ref_jump - ref_link)


def what_was_off(link, jump, ref_jump, nair, ref_nair):
    """The per-link verdict text and its colour: as recorded, within the tolerance, or off (amber; red if it
    dropped)."""
    parts = []
    if jump != ref_jump:
        parts.append(f"jump {abs(jump - ref_jump)}f {'late' if jump > ref_jump else 'early'}")
    if nair != ref_nair:
        parts.append(f"nair {abs(nair - ref_nair)}f {'late' if nair > ref_nair else 'early'}")
    if not parts:
        return "as recorded", MUTED
    if max(abs(jump - ref_jump), abs(nair - ref_nair)) <= TOLERANCE and link <= 0:
        return " · ".join(parts) + f" (within ±{TOLERANCE})", MUTED
    return " · ".join(parts), BAD if link > 0 else WARN


def history_pills(d, x, y, f, label=True):
    if label:
        text(d, (x, y + 8), "LAST 8", font(10, bold=True), fill=MUTED, anchor="lm")
        x += 48
    for i, (true, total, stop) in enumerate(HISTORY):
        colour = GOOD if true == total else (BAD if stop else WARN)
        box = (x, y, x + 34, y + 16)
        tag(d, box, f"{true}/{total}", colour, f)
        if i == 0:
            d.rounded_rectangle((box[0] - 2, box[1] - 2, box[2] + 2, box[3] + 2), radius=10,
                                outline=(255, 255, 255, 230), width=2)
        x += 40
    return x


def result_line(d, x, y):
    big = font(14, bold=True)
    text(d, (x, y), "3 of 4 links true", big, fill=WARN, anchor="lm")
    w = d.textlength("3 of 4 links true", font=big)
    text(d, (x + w + 10, y), "·  link 3 dropped by 1f: jump 2f late (ref ±0)", font(12, bold=True), fill=TEXT,
         anchor="lm")


def option_a(bg):
    img = bg.copy()
    o = iv.overlay()
    d = ImageDraw.Draw(o)
    x0, x1, _ = strip(d)
    y0, y1 = 158, 290
    panel(d, (x0, y0, x1, y1))
    small = font(11, bold=True)
    tag(d, (x0 + 8, y0 + 8, x0 + 70, y0 + 26), "COMBO", (124, 92, 214, 255), small, fg=(255, 255, 255, 255))
    text(d, (x0 + 80, y0 + 17), "Pika nair loop", font(14, bold=True), fill=TEXT, anchor="lm")
    text(d, (x0 + 200, y0 + 17), "target · 4 links · tolerance ±1f · CPU at 30% (as recorded)", font(11),
         fill=MUTED, anchor="lm")
    text(d, (x1 - 12, y0 + 17), "attempt 12", font(12, bold=True), fill=TEXT, anchor="rm")
    cw = (x1 - x0 - 16 - 3 * 6) / 4
    for k, (link, ref_link, jump, ref_jump, nair, ref_nair) in enumerate(LINKS):
        cx = x0 + 8 + k * (cw + 6)
        cy = y0 + 32
        d.rounded_rectangle((cx, cy, cx + cw, cy + 70), radius=6, fill=(32, 38, 52, 255),
                            outline=(GOOD if link <= 0 else BAD)[:3] + (190,), width=2)
        text(d, (cx + 8, cy + 11), f"L{k + 1}", font(12, bold=True), fill=MUTED, anchor="lm")
        x = cx + 30
        x += chip(d, x, cy + 2, None, "X", size=18) + 3
        chip(d, x, cy + 2, None, "A", size=18)
        text(d, (cx + 88, cy + 11), "hit ✓", font(11, bold=True), fill=GOOD, anchor="lm")
        verdict_pill(d, (cx + cw - 70, cy + 3, cx + cw - 6, cy + 20), link)
        timing_ruler(d, cx + 10, cy + 33, -1, 6, 13, link_window(ref_link, ref_jump), ref_jump, jump)
        text(d, (cx + 10 + 8 * 13 + 6, cy + 31), f"jump {sign(jump)}", font(11, bold=True),
             fill=TEXT, anchor="la")
        text(d, (cx + 10 + 8 * 13 + 6, cy + 45), f"ref {sign(ref_jump)}", font(10), fill=MUTED, anchor="la")
        off, colour = what_was_off(link, jump, ref_jump, nair, ref_nair)
        text(d, (cx + 8, cy + 61), off, font(10, bold=True), fill=colour, anchor="lm")
    result_line(d, x0 + 12, y1 - 16)
    history_pills(d, x1 - 12 - 48 - 8 * 40 + 6, y1 - 24, font(10, bold=True))
    img.alpha_composite(o)
    lo = iv.overlay()
    ld = ImageDraw.Draw(lo)
    legend_box = (842, 548, 1268, 584)
    panel(ld, legend_box, radius=6, fill=(14, 18, 27, 225), outline=(0, 0, 0, 0))
    iv.legend(ld, legend_box[0] + 8, legend_box[1] + 3, ["startup", "active", "recovery", "idle"], font(11))
    iv.legend(ld, legend_box[0] + 8, legend_box[1] + 19, ["hitlag", "hitstun", "shieldstun"], font(11))
    note(ld, (700, 588, 1268, 710), "Option A · Combo bar under the strip (recommended)", [
        "Real Eden frame (P1 Steve, CPU Pikachu: the only Pikachu frame); the numbers",
        "illustrate a P1 Pikachu nair loop. One cell per link: hit, TRUE/DROP, your jump",
        "(▼) against the recorded one (□) and the jumps that still combo (green); the",
        "attempt result and the last 8 attempts underneath. The strip above is INPUT-VIZ A.",
    ], True)
    img.alpha_composite(lo)
    return img


def option_b(bg):
    img = bg.copy()
    o = iv.overlay()
    d = ImageDraw.Draw(o)
    x0, y0, x1 = 12, 12, 404
    rh = 86
    y1 = y0 + 52 + rh * len(LINKS) + 66
    panel(d, (x0, y0, x1, y1))
    small = font(11, bold=True)
    tag(d, (x0 + 10, y0 + 10, x0 + 72, y0 + 28), "COMBO", (124, 92, 214, 255), small, fg=(255, 255, 255, 255))
    text(d, (x0 + 82, y0 + 19), "Pika nair loop", font(14, bold=True), fill=TEXT, anchor="lm")
    text(d, (x1 - 12, y0 + 19), "attempt 12", font(12, bold=True), fill=TEXT, anchor="rm")
    text(d, (x0 + 12, y0 + 40), "frames after you can act · □ recorded · ▼ you · green: jumps that combo",
         font(10), fill=MUTED, anchor="lm")
    for k, (link, ref_link, jump, ref_jump, nair, ref_nair) in enumerate(LINKS):
        cy = y0 + 52 + k * rh
        d.rounded_rectangle((x0 + 8, cy, x1 - 8, cy + rh - 8), radius=7, fill=(32, 38, 52, 255),
                            outline=(GOOD if link <= 0 else BAD)[:3] + (190,), width=2)
        text(d, (x0 + 18, cy + 13), f"L{k + 1}", font(13, bold=True), fill=MUTED, anchor="lm")
        text(d, (x0 + 46, cy + 13), "Aerial → Aerial", font(12, bold=True), fill=TEXT, anchor="lm")
        text(d, (x0 + 162, cy + 13), "hit ✓", font(11, bold=True), fill=GOOD, anchor="lm")
        text(d, (x0 + 212, cy + 13), f"ref {sign(ref_link)}", font(10), fill=MUTED, anchor="lm")
        verdict_pill(d, (x1 - 88, cy + 4, x1 - 16, cy + 22), link)
        # One timeline per link: frames −1..+12 from A1; the jump and the nair presses, recorded and yours.
        lo, hi, step = -1, 12, 18
        tx, ty = x0 + 46, cy + 40
        win = link_window(ref_link, ref_jump)
        for f in range(lo, hi + 1):
            fill = GOOD if win[0] <= f <= win[1] else (70, 78, 94, 255)
            d.rectangle((tx + (f - lo) * step, ty, tx + (f - lo) * step + step - 2, ty + 8), fill=fill)
        for label, ref_at, you_at in (("jump", ref_jump, jump), ("nair", ref_jump + ref_nair, jump + nair)):
            rx = tx + (ref_at - lo) * step + (step - 2) / 2
            yx = tx + (you_at - lo) * step + (step - 2) / 2
            d.rectangle((rx - 4, ty - 4, rx + 4, ty + 12), outline=(255, 255, 255, 255), width=2)
            off = you_at - ref_at
            colour = GOOD if abs(off) <= TOLERANCE and not (label == "jump" and link > 0) else (
                BAD if link > 0 and label == "jump" else WARN)
            d.polygon([(yx - 6, ty - 12), (yx + 6, ty - 12), (yx, ty - 3)], fill=colour)
            text(d, (yx, ty + 14), label, font(9), fill=MUTED, anchor="ma")
        off, colour = what_was_off(link, jump, ref_jump, nair, ref_nair)
        text(d, (x1 - 16, cy + rh - 18), off, font(10, bold=True), fill=colour, anchor="rm")
    fy = y0 + 52 + rh * len(LINKS) + 4
    text(d, (x0 + 14, fy + 10), "3 of 4 links true", font(14, bold=True), fill=WARN, anchor="lm")
    text(d, (x0 + 14, fy + 30), "link 3 dropped by 1f: jump 2f late (ref ±0)", font(12, bold=True), fill=TEXT,
         anchor="lm")
    history_pills(d, x0 + 14, fy + 42, font(10, bold=True))
    img.alpha_composite(o)
    lo = iv.overlay()
    ld = ImageDraw.Draw(lo)
    note(ld, (700, 588, 1268, 710), "Option B · Link ladder (works with the strip Off)", [
        "Real Eden frame (P1 Steve, CPU Pikachu: the only Pikachu frame); the numbers",
        "illustrate a P1 Pikachu nair loop. One row per link: hit, TRUE/DROP, and both",
        "presses of the follow-up (jump, nair) as you (▼) against the recording (□) on",
        "one timeline; the attempt result and the last 8 attempts at the bottom.",
    ], False)
    img.alpha_composite(lo)
    return img


def main():
    menu_bg = iv.background(sys.argv[1])
    game_bg = iv.background(sys.argv[2])
    out = sys.argv[3]
    os.makedirs(out, exist_ok=True)
    for name, fn, bg in (("COMBO-HELPER-menu-recording-page", menu_page, menu_bg),
                         ("COMBO-HELPER-menu-name-keyboard", menu_keyboard, menu_bg),
                         ("COMBO-HELPER-menu-saved-combos", menu_list, menu_bg),
                         ("COMBO-HELPER-A-combo-bar", option_a, game_bg),
                         ("COMBO-HELPER-B-link-ladder", option_b, game_bg)):
        path = os.path.join(out, name + ".png")
        fn(bg).convert("RGB").save(path, optimize=True)
        print(path)


if __name__ == "__main__":
    main()
