#!/usr/bin/python3 -I
"""Render one illustration per open operator decision for docs/DECISIONS.md (docs/decisions/*.png).

Same look as tools/render_menu_mockup.py (its Canvas, colours, row controls and chips are reused); each
image shows the options side by side, the recommended option outlined in green. Images that show menu
screens draw them the way the proposal's mockups do; the few pictures inside them (fighter portraits,
item icons, the DI / SDI chip renders) are the review copies already in docs/menu-mockup/captures/, drawn
the same way the mockup screens draw them. No new game-derived file is created.

Output: docs/decisions/<id>.png. Pillow only.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import render_menu_mockup as m  # noqa: E402
from PIL import Image, ImageDraw, ImageFont  # noqa: E402

OUT = Path(__file__).resolve().parent.parent / "docs" / "decisions"
FG, MUTED, SEL, OURS, ADV, LINE = m.FG, m.MUTED, m.SEL, m.OURS, m.ADV, m.LINE
TILE, TILE_HI, HILITE, P1, CPU = m.TILE, m.TILE_HI, m.HILITE, m.P1, m.CPU
PANEL = (12, 14, 19)
BAD = (214, 120, 96)
DARK = (14, 16, 20)
WIDE = 1200


class DCanvas(m.Canvas):
    """The mockup canvas at any size, plain gradient, PNG only."""

    def __init__(self, w, h):
        img = Image.new("RGB", (w, h), m.BG0)
        px = img.load()
        for y in range(h):
            t = y / h
            col = tuple(int(m.BG0[i] * (1 - t) + m.BG1[i] * t) for i in range(3))
            for x in range(w):
                px[x, y] = col
        self.img = img.convert("RGBA")
        self.d = ImageDraw.Draw(self.img, "RGBA")
        self.fonts = {k: ImageFont.truetype(p, s) for k, (p, s) in m.FONTS.items()}
        self.svg = []

    def save(self, name):
        OUT.mkdir(parents=True, exist_ok=True)
        self.img.convert("RGB").save(OUT / f"{name}.png", optimize=True)


# ---------------------------------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------------------------------
def panel(c, x0, y0, x1, y1, title, rec=False, note=None, tag_below=False):
    c.rect(x0, y0, x1, y1, fill=PANEL, outline=OURS if rec else LINE, width=3 if rec else 1, radius=8)
    c.text(x0 + 14, y0 + 10, title, "h", FG)
    if rec:
        w = c.textlength("recommended", "b") + 16
        tx, ty = (x0 + 14 + w, y0 + 38) if tag_below else (x1 - 10, y0 + 9)
        c.rect(tx - w, ty, tx, ty + 22, fill=OURS, radius=5)
        c.text(tx - w / 2, ty + 3, "recommended", "b", DARK, "m")
    if tag_below:
        y0 += 28
    if note:
        c.text(x0 + 14, y1 - 26, note, "xs", MUTED)
    return y0 + 46


def row(c, x0, x1, y, label, kind, value, focus=False, dim=False):
    if focus:
        c.rect(x0, y - 6, x1, y + 26, fill=HILITE)
        c.rect(x0, y - 6, x0 + 4, y + 26, fill=SEL)
    c.text(x0 + 12, y, label, "s", MUTED if dim else FG)
    if kind in ("toggle", "drop", "multi", "number", "info", "editor"):
        text, col = m.control_text(kind, value)
        c.text(x1 - 10, y, text, "s", MUTED if dim else col, "r")
    else:
        m.draw_control(c, kind, value, y, x1 - 10)


def tabs(c, x, y, names, active, font="s"):
    for i, name in enumerate(names):
        w = c.textlength(name, font) + 20
        if i == active:
            c.rect(x, y - 4, x + w, y + 24, fill=SEL, radius=3)
            c.text(x + 10, y, name, font, FG)
        else:
            c.rect(x, y - 4, x + w, y + 24, outline=LINE, radius=3)
            c.text(x + 10, y, name, font, MUTED)
        x += w + 8
    return x


def arrow(c, p0, p1, col=MUTED, width=2, both=False):
    import math
    c.line([p0, p1], col, width)
    for a, b in ((p0, p1), (p1, p0)) if both else ((p0, p1),):
        ang = math.atan2(b[1] - a[1], b[0] - a[0])
        hx, hy = b
        c.polygon([(hx, hy), (hx - 11 * math.cos(ang - 0.45), hy - 11 * math.sin(ang - 0.45)),
                   (hx - 11 * math.cos(ang + 0.45), hy - 11 * math.sin(ang + 0.45))], fill=col)


def box(c, x0, y0, x1, y1, lines, col=LINE, fill=TILE, font="xs", tcol=FG):
    c.rect(x0, y0, x1, y1, fill=fill, outline=col, width=2, radius=6)
    lh = m.FONTS[font][1] + 5
    y = (y0 + y1) / 2 - lh * len(lines) / 2 + 2
    for ln in lines:
        c.text((x0 + x1) / 2, y, ln, font, tcol, "m")
        y += lh


def tag(c, x, y, text, col):
    w = c.textlength(text, "b") + 12
    c.rect(x, y, x + w, y + 20, fill=col, radius=4)
    c.text(x + w / 2, y + 3, text, "b", DARK, "m")
    return x + w


def lines(c, x, y, items, font="xs", col=MUTED, step=None):
    step = step or m.FONTS[font][1] + 6
    for it in items:
        if isinstance(it, tuple):
            c.text(x, y, it[0], font, it[1])
        else:
            c.text(x, y, it, font, col)
        y += step
    return y


def two(c, h, ta, tb, rec="a", na=None, nb=None, y0=12):
    """Two option panels side by side; returns their inner boxes."""
    mid = WIDE // 2
    ya = panel(c, 12, y0, mid - 6, h - 12, ta, rec == "a", na)
    yb = panel(c, mid + 6, y0, WIDE - 12, h - 12, tb, rec == "b", nb)
    return (24, ya, mid - 18), (mid + 18, yb, WIDE - 24)


def three(c, h, titles, rec=0, notes=(None, None, None)):
    w = (WIDE - 24 - 24) / 3
    out = []
    for i, t in enumerate(titles):
        x0 = 12 + i * (w + 12)
        yi = panel(c, x0, 12, x0 + w, h - 12, t, rec == i, notes[i], tag_below=True)
        out.append((x0 + 12, yi, x0 + w - 12))
    return out


# ---------------------------------------------------------------------------------------------------
# menu decisions (MENU-DRAFT.md §4)
# ---------------------------------------------------------------------------------------------------
def d_pages():
    c = DCanvas(WIDE, 360)
    opts = [("A · five pages (as drawn)", ["CPU", "Saves", "Recording", "Fighters & Stages", "Display & Controls"], True,
             "each page one theme; Display & Controls is the short one"),
            ("B · four pages", ["CPU", "Room & Display", "Recording", "Fighters & Stages"], False,
             "Saves + Display & Controls merged: fewer tabs, one long mixed page"),
            ("C · six pages", ["CPU", "Saves", "Recording", "Fighters", "Stages & Lobby", "Display & Controls"], False,
             "stage and lobby rows get their own tab; worth it only if lobby practice grows")]
    y = 12
    for title, names, rec, note in opts:
        panel(c, 12, y, WIDE - 12, y + 104, title, rec)
        tabs(c, 28, y + 50, names, 0)
        c.text(WIDE - 28, y + 80, note, "xs", MUTED, "r")
        y += 114
    c.save("01-pages")


def d_dropdown():
    c = DCanvas(WIDE, 300)
    (ax, ay, ax1), (bx, by, bx1) = two(c, 300, "A · Left / Right cycle in place", "B · A must open the list",
                                       na="fast; a stray Left / Right changes the value",
                                       nb="one more press per change; no accidental change")
    row(c, ax, ax1, ay + 10, "SDI Strength", "drop", "Normal", focus=True)
    arrow(c, (ax + 260, ay + 44), (ax + 260, ay + 84))
    c.text(ax + 276, ay + 54, "D-pad Right, once", "xs", MUTED)
    row(c, ax, ax1, ay + 100, "SDI Strength", "drop", "Medium", focus=True)
    c.text(ax + 12, ay + 150, "A still opens the full list when wanted.", "xs", MUTED)
    row(c, bx, bx1, by + 10, "SDI Strength", "drop", "Normal", focus=True)
    lx = bx1 - 200
    c.rect(lx, by + 40, bx1 - 10, by + 40 + 4 * 30 + 10, fill=TILE, outline=LINE, radius=6)
    for i, v in enumerate(["None", "Normal", "Medium", "High"]):
        yy = by + 48 + i * 30
        if v == "Medium":
            c.rect(lx + 4, yy - 3, bx1 - 14, yy + 23, fill=SEL, radius=4)
        c.text(lx + 16, yy, v, "s", FG)
    c.text(bx + 12, by + 56, "A opens the list,", "xs", MUTED)
    c.text(bx + 12, by + 76, "Up / Down picks, A confirms.", "xs", MUTED)
    c.text(bx + 12, by + 96, "Left / Right do nothing here.", "xs", MUTED)
    c.save("02-dropdown")


def d_delays():
    c = DCanvas(WIDE, 330)
    (ax, ay, ax1), (bx, by, bx1) = two(c, 330, "A · checklist (today's meaning)", "B · min - max range slider", rec="a",
                                       na="saved files load unchanged; any set of values",
                                       nb="simpler to read; changes the meaning of saved sets")
    c.text(ax + 12, ay, "Aerial Delay  (frames; tick any)", "s", FG)
    picked = {0, 5, 10}
    for v in range(31):
        col, r = v % 8, v // 8
        x, y = ax + 14 + col * 66, ay + 30 + r * 34
        c.rect(x, y + 2, x + 18, y + 20, outline=FG, width=2)
        if v in picked:
            c.rect(x + 4, y + 6, x + 14, y + 16, fill=SEL)
        c.text(x + 24, y + 2, str(v), "xs", FG if v in picked else MUTED)
    c.text(ax + 12, ay + 172, "CPU waits 0, 5 or 10 frames: exactly these, at random", "xs", OURS)
    c.text(bx + 12, by, "Aerial Delay", "s", FG)
    sx0, sx1, sy = bx + 30, bx1 - 30, by + 70
    c.line([(sx0, sy), (sx1, sy)], LINE, 6)
    k = (sx1 - sx0) / 30
    c.line([(sx0, sy), (sx0 + 10 * k, sy)], SEL, 6)
    for v in (0, 10):
        c.circle(sx0 + v * k, sy, 11, fill=FG)
    for v in (0, 10, 20, 30):
        c.text(sx0 + v * k, sy + 18, str(v), "xs", MUTED, "m")
    c.text(bx + 12, by + 130, "CPU waits any of 0 - 10 frames at random", "xs", ADV)
    c.text(bx + 12, by + 150, "(a set like {0, 5, 10} can no longer be said)", "xs", MUTED)
    c.save("03-delays")


def d_subscreens():
    c = DCanvas(WIDE, 420)
    (ax, ay, ax1), (bx, by, bx1) = two(c, 420, "A · one editor row each (as drawn)", "B · every row on the page", rec="a",
                                       na="the pages stay short; an editor opens on A",
                                       nb="+12 rows on CPU, +6 on Fighters, +5 on Display")
    for i, (lab, kind, val) in enumerate([("Mash", "multi", "2 of 13 selected"), ("Followup", "multi", "0 of 13 selected"),
                                          ("Mash Triggers", "multi", "4 of 17 selected"),
                                          ("Situation overrides", "editor", "3 of 13 set")]):
        row(c, ax, ax1, ay + 6 + i * 36, lab, kind, val, focus=(i == 3))
    c.text(ax + 12, ay + 160, "Button chords and Swap groups: same, one row each", "xs", MUTED)
    sits = ["Ledge Neutral Getup", "Ledge Roll", "Ledge Jump", "Ledge Attack", "Tech Action", "Clatter", "Tumble",
            "Hitstun", "...  5 more"]
    for i, s in enumerate(sits):
        row(c, bx, bx1, by + 6 + i * 32, s, "multi" if i < 8 else "info", "0 of 13 selected" if i < 8 else "", dim=(i == 8))
    c.save("04-subscreens")


def d_firstpage():
    c = DCanvas(WIDE, 250)
    names = ["CPU", "Saves", "Recording", "Fighters & Stages", "Display & Controls"]
    (ax, ay, ax1), (bx, by, bx1) = two(c, 250, "A · the last page used (today)", "B · always CPU", rec="a",
                                       na="a drill loop returns to the same page",
                                       nb="predictable; one L/R press more for other pages")
    for (x, y), act, t in (((ax, ay), 2, "you closed the menu on Recording ..."), ((bx, by), 2, "you closed the menu on Recording ...")):
        c.text(x + 4, y, t, "xs", MUTED)
    tabs(c, ax + 4, ay + 30, names, 2, "xxs")
    tabs(c, bx + 4, by + 30, names, 2, "xxs")
    c.text(ax + 4, ay + 72, "... and B + D-pad Up reopens it on:", "xs", MUTED)
    c.text(bx + 4, by + 72, "... and B + D-pad Up reopens it on:", "xs", MUTED)
    tabs(c, ax + 4, ay + 102, names, 2, "xxs")
    tabs(c, bx + 4, by + 102, names, 0, "xxs")
    c.save("05-first-page")


def d_itemonload():
    c = DCanvas(WIDE, 300)
    (ax, ay, ax1), (bx, by, bx1) = two(c, 300, "A · keep the row (renamed)", "B · migrate once, drop the row", rec="a",
                                       na="zero risk; one rarely used row stays on Saves",
                                       nb="one row fewer; the import must map a variation to an item")
    for i, (lab, kind, val) in enumerate([("Buffs on load", "multi", "0 of 12 selected"),
                                          ("Item on load (older saves)", "drop", "None")]):
        row(c, ax, ax1, ay + 6 + i * 36, lab, kind, val, focus=(i == 1))
    lines(c, ax + 12, ay + 90, ["used only when a save has no item snapshot",
                                "(saves made before the fork's snapshot);",
                                "new saves carry the item themselves"])
    box(c, bx, by + 10, bx + 200, by + 80, ["old save", "no item snapshot", "+ Player 3rd Var."])
    arrow(c, (bx + 206, by + 45), (bx + 270, by + 45))
    box(c, bx + 276, by + 10, bx1 - 4, by + 80, ["first-boot import writes", "the snapshot once"], col=ADV)
    lines(c, bx + 12, by + 100, ["the row disappears from Saves;", "a variation the import cannot map loads as None"])
    c.save("06-item-on-load")


def d_replaycapture():
    c = DCanvas(WIDE, 290)
    (ax, ay, ax1), (bx, by, bx1) = two(c, 290, "A · toggle in the menu (as drawn)", "B · INI / Deluxe page only (today)",
                                       rec="a", na="settable in-game; all three always agree",
                                       nb="no new row; change it outside Training")
    row(c, ax, ax1, ay + 6, "Replay capture", "toggle", "ON", focus=True)
    box(c, ax + 10, ay + 70, ax + 250, ay + 130, ["replay_capture.ini", "enabled=on"])
    box(c, ax1 - 250, ay + 70, ax1 - 10, ay + 130, ["Deluxe Plugins page", "Replay capture  ON"])
    arrow(c, (ax + 130, ay + 34), (ax + 130, ay + 66), OURS, both=True)
    arrow(c, (ax + 254, ay + 100), (ax1 - 254, ay + 100), OURS, both=True)
    c.text(bx + 12, by + 10, "Recording page: no Replay capture row", "xs", MUTED)
    box(c, bx + 10, by + 70, bx + 250, by + 130, ["replay_capture.ini", "edit by hand"])
    box(c, bx1 - 250, by + 70, bx1 - 10, by + 130, ["Deluxe Plugins page", "Replay capture  ON"])
    arrow(c, (bx + 254, by + 100), (bx1 - 254, by + 100), MUTED, both=True)
    c.save("07-replay-capture")


def d_preloadsamples():
    c = DCanvas(WIDE, 330)
    (ax, ay, ax1), (bx, by, bx1) = two(c, 330, "A · shipped, Loading (advanced)", "B · diag build only", rec="b",
                                       na="measurable on any card; a diagnostics row every player sees",
                                       nb="shipped menu one row shorter; measuring needs the diag build")
    rows_ = [("Cycle Preload", "drop", "5"), ("Preload", "drop", "Both"), ("Ready Fighters", "number", "8"),
             ("Group Swap", "toggle", "ON"), ("Preload Samples", "toggle", "OFF")]
    for i, (lab, kind, val) in enumerate(rows_):
        row(c, ax, ax1, ay + 6 + i * 34, lab, kind, val, focus=(i == 4))
    for i, (lab, kind, val) in enumerate(rows_[:4]):
        row(c, bx, bx1, by + 6 + i * 34, lab, kind, val)
    c.text(bx + 12, by + 150, "diag build adds:  Preload Samples  [OFF]", "xs", ADV)
    c.text(bx + 12, by + 172, "(writes sparse timing / memory samples to the diagnostics file)", "xs", MUTED)
    c.save("08-preload-samples")


def d_shortcuts():
    c = DCanvas(WIDE, 300)
    (ax, ay, ax1), (bx, by, bx1) = two(c, 300, "A · read-only, INI is the editor", "B · editable in the menu", rec="a",
                                       na="no new editor; restart to apply, as the fork",
                                       nb="change in-game; menu writes the INI")
    for i, (lab, val) in enumerate([("Menu shortcut 1", "Minus"), ("Menu shortcut 2", "disabled"),
                                    ("Permanent shortcut", "B + D-pad Up")]):
        row(c, ax, ax1, ay + 6 + i * 34, lab, "info", val)
    box(c, ax + 10, ay + 120, ax + 300, ay + 172, ["button_config.ini", "shortcut_1=Minus"])
    arrow(c, (ax + 304, ay + 146), (ax + 380, ay + 146))
    c.text(ax + 388, ay + 138, "shown, next boot", "xs", MUTED)
    for i, (lab, val) in enumerate([("Menu shortcut 1", "[ Minus ]  A: re-bind"), ("Menu shortcut 2", "[ disabled ]  A: re-bind")]):
        row(c, bx, bx1, by + 6 + i * 34, lab, "info", val, focus=(i == 0))
    row(c, bx, bx1, by + 74, "Permanent shortcut", "info", "B + D-pad Up")
    box(c, bx + 10, by + 120, bx + 300, by + 172, ["button_config.ini", "written on re-bind"], col=OURS)
    c.save("09-menu-shortcuts")


def d_helptext():
    c = DCanvas(WIDE, 290)
    text = ("Ice Climbers Pair Groups: a pair window may own groups. Needs Ice Climbers Pair Bank and "
            "Group Swap; greyed while either is off.")
    (ax, ay, ax1), (bx, by, bx1) = two(c, 290, "A · one line (today)", "B · two-line panel", rec="b",
                                       na="one more row of list; long texts cut or rewritten",
                                       nb="the fork's longer texts fit; the list loses one row")
    for (x, y, x1), n in (((ax, ay, ax1), 1), ((bx, by, bx1), 2)):
        row(c, x, x1, y + 6, "Ice Climbers Pair Groups", "toggle", "ON", focus=True)
        row(c, x, x1, y + 40, "PT/Aegis Groups", "toggle", "ON")
        top = y + 150 - 26 * (n - 1)
        c.line([(x, top - 8), (x1, top - 8)], LINE)
        per = int((x1 - x - 16) / 8.45)
        if n == 1:
            c.text(x + 8, top, text[:per - 1] + "…", "xs", FG)
        else:
            cut = text.rfind(" ", 0, per)
            c.text(x + 8, top, text[:cut], "xs", FG)
            c.text(x + 8, top + 22, text[cut + 1:], "xs", FG)
    c.save("10-help-text")


def d_p1fallback():
    c = DCanvas(WIDE, 330)
    (a, b, cc) = three(c, 330, ["A · fallback rows (as drawn)", "B · INI only", "C · pilot required"], rec=0,
                       notes=("works without the pilot", "no rows; a file to edit", "no TM-side P1 control"))
    x, y, x1 = a
    c.text(x, y, "pilot on the card:", "xs", MUTED)
    row(c, x - 4, x1, y + 26, "P1 control", "info", "CPU pilot plugin")
    c.text(x, y + 70, "pilot absent:", "xs", MUTED)
    row(c, x - 4, x1, y + 96, "P1 CPU Control", "toggle", "OFF")
    row(c, x - 4, x1, y + 130, "P1 CPU Level", "drop", "3")
    x, y, x1 = b
    c.text(x, y, "pilot absent:", "xs", MUTED)
    row(c, x - 4, x1, y + 26, "P1 control", "info", "see the INI")
    box(c, x, y + 80, x1, y + 140, ["an INI key turns the", "fallback on (no menu row)"])
    x, y, x1 = cc
    c.text(x, y, "pilot absent:", "xs", MUTED)
    row(c, x - 4, x1, y + 26, "P1 control", "info", "install the pilot")
    box(c, x, y + 80, x1, y + 140, ["the clean room only hands", "P1 to the pilot"])
    c.save("11-p1-fallback")


def d_weights():
    c = DCanvas(WIDE, 400)
    (ax, ay, ax1), (bx, by, bx1) = two(c, 400, "A · weight digit (today)", "B · plain on / off",
                                       rec="a", na="Up In is picked 2 times in 4", nb="each lit chip 1 time in 3")
    sel = {"Up In": 2, "Up": 1, "Up Out": 1}
    for (x0, y0, x1), weights in (((ax, ay, ax1), True), ((bx, by, bx1), False)):
        gx, gy, s, p = x0 + 150, y0 + 140, 76, 92
        for name, (dx, dy) in list(m.DIRS.items())[:9]:
            m.chip_direction(c, gx + (dx + 1) * p - p, gy + dy * p, s, name, name in sel,
                             sel.get(name, 0) if weights else 0)
        lx = x0 + 330
        c.text(lx, y0 + 40, "picked at random:", "xs", MUTED)
        tot = sum(sel.values()) if weights else len(sel)
        for i, (n, w) in enumerate(sel.items()):
            c.text(lx, y0 + 70 + i * 26, f"{n:<7} {(w if weights else 1)} in {tot}", "s", FG)
    c.save("12-direction-weights")


def d_paging():
    c = DCanvas(WIDE, 440)
    (ax, ay, ax1), (bx, by, bx1) = two(c, 440, "A · 4 x 8, two pages (the fork)", "B · denser, one page", rec="a",
                                       na="names readable; L / R for items 33 - 38",
                                       nb="all 38 at once; names cut to a few letters")
    for (x0, y0, x1), cols, rws, n, lab in (((ax, ay, ax1), 4, 8, 32, "page 1 of 2"), ((bx, by, bx1), 5, 8, 38, "page 1 of 1")):
        tw, th = (x1 - x0 - 10) / cols, 34
        for i in range(n):
            cx, cy = x0 + 4 + (i % cols) * tw, y0 + 4 + (i // cols) * (th + 4)
            c.rect(cx, cy, cx + tw - 6, cy + th, fill=TILE, outline=LINE, radius=4)
            c.image(f"captures/items/icon-{i:02}.png", cx + 3, cy + 2, cx + 33, cy + th - 2)
            nm = m.ITEMS[i].split(": ", 1)[1]
            room = int((tw - 46) / 7.3)
            c.text(cx + 38, cy + 10, nm if len(nm) <= room else nm[:max(1, room - 1)] + "…", "xxs", FG)
        c.text(x1 - 6, y0 + 8 * 38 + 10, lab, "xs", MUTED, "r")
    c.save("14-grid-paging")


def mii_bust(c, x, y, s):
    c.circle(x + s / 2, y + s * 0.36, s * 0.22, fill=(170, 176, 190))
    c.ellipse(x + s * 0.18, y + s * 0.62, x + s * 0.82, y + s * 1.05, fill=(120, 126, 140))


def d_miirows():
    c = DCanvas(WIDE, 300)
    (ax, ay, ax1), (bx, by, bx1) = two(c, 300, "A · list with the bust beside it", "B · text only", rec="a",
                                       na="the fork's bust where one exists; else text", nb="smallest; names only")
    entries = ["Saved Mii 1", "Saved Mii 2", "From a replay: Mii 3", "Add New..."]
    for i, e in enumerate(entries):
        y = ay + 4 + i * 44
        if i == 0:
            c.rect(ax, y - 4, ax1, y + 36, fill=HILITE)
            c.rect(ax, y - 4, ax + 4, y + 36, fill=SEL)
        if i < 3:
            c.rect(ax + 12, y - 2, ax + 46, y + 32, fill=TILE, outline=LINE, radius=4)
            mii_bust(c, ax + 12, y - 2, 34)
        c.text(ax + 58, y + 6, e, "s", FG if i < 3 else MUTED)
        y = by + 4 + i * 44
        if i == 0:
            c.rect(bx, y - 4, bx1, y + 36, fill=HILITE)
            c.rect(bx, y - 4, bx + 4, y + 36, fill=SEL)
        c.text(bx + 14, y + 6, e, "s", FG if i < 3 else MUTED)
    c.text(ax1 - 4, ay + 184, "(busts drawn as a neutral silhouette here)", "xxs", MUTED, "r")
    c.save("15-mii-rows")


def d_portraits():
    c = DCanvas(WIDE, 330)
    (ax, ay, ax1), (bx, by, bx1) = two(c, 330, "A · read at run time (the stage rule)", "B · fixed stock icons (the fork)",
                                       rec="a", na="costume / character mods show; nothing baked",
                                       nb="always the stock picture; 89 files in the layout archive")
    tw, th = 110, 120
    for i, (title, kind) in enumerate([("stock card", "stock"), ("with a costume mod", "mod"), ("unreadable", "text")]):
        x, y = ax + 10 + i * (tw + 70), ay + 30
        c.text(x + tw / 2, y - 24, title, "xs", MUTED, "m")
        c.rect(x, y, x + tw, y + th, fill=TILE, outline=LINE, radius=6)
        if kind == "stock":
            c.image("captures/portraits/mario.png", x + 15, y + 6, x + tw - 15, y + th - 26)
        elif kind == "mod":
            c.line(m.rounded_path(x + 12, y + 6, x + tw - 12, y + th - 26, 8), ADV, 2, dash=(6, 4))
            c.text(x + tw / 2, y + 30, "the mod's", "xxs", ADV, "m")
            c.text(x + tw / 2, y + 46, "own picture", "xxs", ADV, "m")
        else:
            c.text(x + tw / 2, y + 40, "MARIO", "b", FG, "m")
        c.text(x + tw / 2, y + th - 20, "Mario", "xxs", FG, "m")
    c.text(ax + 10, ay + 170, "read from the game's files through the mod-aware view", "xs", MUTED)
    for i, title in enumerate(["stock card", "with a costume mod", "unreadable"]):
        x, y = bx + 10 + i * (tw + 70), by + 30
        c.text(x + tw / 2, y - 24, title, "xs", MUTED, "m")
        c.rect(x, y, x + tw, y + th, fill=TILE, outline=LINE, radius=6)
        c.image("captures/portraits/mario.png", x + 15, y + 6, x + tw - 15, y + th - 26)
        c.text(x + tw / 2, y + th - 20, "Mario", "xxs", FG, "m")
    c.text(bx + 10, by + 170, "the same baked picture in every case", "xs", MUTED)
    c.save("16-fighter-portraits")


def d_dichips():
    c = DCanvas(WIDE, 330)
    (ax, ay, ax1), (bx, by, bx1) = two(c, 330, "A · game visual on DI / SDI only", "B · game visual on all four", rec="a",
                                       na="each chip says what the option does",
                                       nb="air dodge and shield tilt would look like launch influence")
    for (x0, y0, x1), vis in (((ax, ay, ax1), ["di", "sdi", "arrow", "arrow"]), ((bx, by, bx1), ["di", "sdi", "di", "di"])):
        for i, (lab, v) in enumerate(zip(["DI", "SDI", "Airdodge", "Shield Tilt"], vis)):
            cx = x0 + 70 + i * 135
            m.chip_direction(c, cx, y0 + 90, 104, "Up In", True, visual=v)
            c.text(cx, y0 + 150, lab, "s", FG, "m")
            if v == "di" and i >= 2:
                c.text(cx, y0 + 176, "misleading", "xs", BAD, "m")
    c.save("17-di-sdi-chips")


def d_applychange():
    c = DCanvas(WIDE, 450)
    c.rect(12, 12, WIDE - 12, 438, fill=PANEL, outline=LINE, radius=8)
    x0, x1 = 300, WIDE - 40
    ev = {"change": 0.18, "hit1": 0.33, "hit2": 0.46, "reset": 0.62, "hit3": 0.78, "hit4": 0.9}
    X = {k: x0 + v * (x1 - x0) for k, v in ev.items()}
    c.text(28, 24, "SDI Direction changed from Neutral to {Up, In} in the menu, then four hits (example rolls)", "h", FG)
    ty = 68
    c.line([(x0, ty + 14), (x1, ty + 14)], LINE, 2)
    for k, lab in (("change", "menu change"), ("hit1", "hit 1"), ("hit2", "hit 2"), ("reset", "L+R+A reset"),
                   ("hit3", "hit 3"), ("hit4", "hit 4")):
        col = ADV if k in ("change", "reset") else FG
        c.line([(X[k], ty + 6), (X[k], 420)], (60, 66, 80), 1, dash=(4, 4))
        c.text(X[k], ty - 12, lab, "xs", col, "m")
    lanes = [("Fork before M5o", None, ["Neutral", "Neutral", "Up", "Up"], "old direction until the next reset"),
             ("A · fork, stage M5o", "recommended", ["Up", "Up", "In", "In"],
              "applies at the next CPU frame; one roll per reset"),
             ("B · fork + roll per hit", "alternative", ["Up", "In", "In", "Up"],
              "applies at the change; a new roll every hit")]
    y = 120
    for name, badge, picks, note in lanes:
        c.rect(24, y - 8, WIDE - 24, y + 84, fill=TILE, outline=OURS if badge == "recommended" else LINE,
               width=3 if badge == "recommended" else 1, radius=6)
        c.text(36, y, name, "s", FG)
        if badge:
            tag(c, 36, y + 26, badge, OURS if badge == "recommended" else ADV)
        c.text(36, y + 56, note, "xxs", MUTED)
        for k, p in zip(("hit1", "hit2", "hit3", "hit4"), picks):
            m.chip_direction(c, X[k], y + 38, 64, p if p != "Neutral" else "Neutral", True, visual="sdi")
            c.text(X[k], y + 72, p, "tiny", FG, "m")
        y += 104
    c.save("18-apply-on-change")


# ---------------------------------------------------------------------------------------------------
# storage and files
# ---------------------------------------------------------------------------------------------------
def d_import():
    c = DCanvas(WIDE, 550)
    c.rect(12, 12, WIDE - 12, 538, fill=PANEL, outline=LINE, radius=8)
    c.text(28, 24, "sd:/ultimate/TrainingModpack/", "h", FG)
    c.text(700, 24, "sd:/ultimate/TrainingRooms/", "h", FG)
    rows_ = [("training_modpack_menu.conf", "settings.json", "1", OURS, "copy + rename"),
             ("save_states.json", "save_states.json", None, OURS, "copy"),
             ("setups/", "setups/", None, OURS, "copy"),
             ("mii-presets.json", "mii-presets.json", None, OURS, "copy"),
             ("macros/", "macros/", None, OURS, "copy"),
             ("button_config.ini", "button_config.ini", "2", OURS, "copy"),
             ("replay_capture.ini", "replay_capture.ini", "2", OURS, "copy"),
             ("replay_captures/", "replay_captures/", "3", OURS, "copy, all (background)"),
             ("*.toml (old formats)", "(not converted)", "4", MUTED, "use port_save_states.py"),
             ("cycle_preload.ini", "cycle_preload.ini", None, MUTED, "regenerated"),
             ("logs, dev.json, layout.arc", "-", None, MUTED, "not copied"),
             (None, "migration.json", "5", ADV, "marker: import done")]
    y = 64
    for src, dst, q, col, how in rows_:
        if src:
            c.text(48, y, "├─ " + src, "s", FG)
        c.text(720, y, "├─ " + dst, "s", FG if col != MUTED else MUTED)
        if src:
            arrow(c, (380, y + 9), (700, y + 9), col)
        c.text(540, y - 14 if src else y, how, "xxs", col, "m")
        if q:
            tag(c, WIDE - 70, y - 1, q, ADV)
        y += 36
    c.text(28, y + 6, "originals never touched; a destination that exists is never overwritten", "xs", MUTED)
    c.save("S1-import-details")


def d_corrupt():
    c = DCanvas(WIDE, 300)
    (ax, ay, ax1), (bx, by, bx1) = two(c, 300, "A · delete it (upstream)", "B · keep a copy, default per field", rec="b",
                                       na="every setting back to default, file gone",
                                       nb="good fields survive; the bad file stays for a look")
    for (x0, y0, x1) in ((ax, ay, ax1), (bx, by, bx1)):
        box(c, x0 + 10, y0 + 10, x0 + 220, y0 + 70, ["settings file", "one field unreadable"], col=BAD)
        arrow(c, (x0 + 226, y0 + 40), (x0 + 286, y0 + 40))
    box(c, ax + 292, ay + 10, ax1 - 10, ay + 70, ["file deleted", "all settings default"], col=BAD)
    box(c, bx + 292, by + 10, bx1 - 10, by + 70, ["copy kept, renamed", "loaded field by field"], col=OURS)
    lines(c, ax + 12, ay + 96, ["Mash, DI, chords, stage order ... all lost"])
    lines(c, bx + 12, by + 96, ["only the unreadable field takes its default;", "the next save writes a clean file"])
    c.save("S2-corrupt-settings")


# ---------------------------------------------------------------------------------------------------
# behaviour and product
# ---------------------------------------------------------------------------------------------------
def d_name():
    c = DCanvas(WIDE, 280)
    (ax, ay, ax1), (bx, by, bx1) = two(c, 280, "A · own name: training_rooms", "B · keep training_modpack", rec="a",
                                       na="tells the two apart on a card and in logs",
                                       nb="indistinguishable from the fork's file")
    p = ".../romfs/skyline/plugins/"
    for (x0, y0, x1), mine in (((ax, ay, ax1), "libtraining_rooms.nro"), ((bx, by, bx1), "libtraining_modpack.nro")):
        c.text(x0 + 8, y0 + 4, p, "s", MUTED)
        c.text(x0 + 24, y0 + 36, "├─ libtraining_modpack.nro   (the fork)", "s", FG)
        c.text(x0 + 24, y0 + 66, "└─ " + mine + "   (clean room)", "s", OURS if mine.endswith("rooms.nro") else BAD)
        c.text(x0 + 8, y0 + 110, "never install both: they hook the same sites", "xs", ADV)
    c.text(bx + 8, by + 134, "same file name: one silently replaces the other", "xs", BAD)
    c.save("B1-plugin-name")


def d_panic():
    c = DCanvas(WIDE, 260)
    (ax, ay, ax1), (bx, by, bx1) = two(c, 260, "A · error pop-up (upstream)", "B · log it, no pop-up", rec="b",
                                       na="message on screen; game unusable until closed",
                                       nb="as the playbook's plugins; the reason is in the logs")
    for (x0, y0, x1), out, col in (((ax, ay, ax1), ["error applet", "on screen"], BAD),
                                   ((bx, by, bx1), ["diagnostics file", "+ Skyline log"], OURS)):
        box(c, x0 + 10, y0 + 10, x0 + 200, y0 + 70, ["plugin panic"], col=BAD)
        arrow(c, (x0 + 206, y0 + 40), (x0 + 276, y0 + 40))
        box(c, x0 + 282, y0 + 10, x1 - 10, y0 + 70, out, col=col)
    c.save("B2-panic-policy")


def d_layoutmods():
    c = DCanvas(WIDE, 280)
    (ax, ay, ax1), (bx, by, bx1) = two(c, 280, "A · inject anyway, document it", "B · detect it, skip injection", rec="a",
                                       na="our menu works; the other mod's training HUD is replaced",
                                       nb="their HUD stays; our menu and overlays are off")
    for (x0, y0, x1), res, col in (((ax, ay, ax1), ["our layout wins", "install line notes the mod"], OURS),
                                   ((bx, by, bx1), ["their layout stays", "menu unavailable"], ADV)):
        box(c, x0 + 10, y0 + 10, x0 + 230, y0 + 80, ["another mod ships", "info_training layout"])
        arrow(c, (x0 + 236, y0 + 45), (x0 + 296, y0 + 45))
        box(c, x0 + 302, y0 + 10, x1 - 10, y0 + 80, res, col=col)
    c.save("B3-layout-mods")


def d_p1ai():
    c = DCanvas(WIDE, 330)
    (ax, ay, ax1), (bx, by, bx1) = two(c, 330, "CPU settings when P1 is AI-driven", "Recording when P1 is AI-driven",
                                       rec=None)
    box(c, ax + 10, ay + 10, ax + 200, ay + 70, ["P1", "driven by the pilot"], col=P1)
    box(c, ax1 - 200, ay + 10, ax1 - 10, ay + 70, ["CPU"], col=CPU)
    box(c, ax + 60, ay + 120, ax1 - 60, ay + 170, ["Mash, DI, SDI, ledge, tech, shield ..."])
    arrow(c, ((ax + ax1) / 2 + 60, ay + 116), (ax1 - 110, ay + 74), OURS)
    arrow(c, ((ax + ax1) / 2 - 60, ay + 116), (ax + 110, ay + 74), BAD)
    c.text(ax1 - 10, ay + 186, "A (recommended): the CPU role only", "xs", OURS, "r")
    c.text(ax + 10, ay + 206, "B: P1 too (two drivers fight over P1)", "xs", BAD)
    box(c, bx + 10, by + 10, bx + 220, by + 70, ["P1 AI-driven", "Record pressed"], col=P1)
    arrow(c, (bx + 226, by + 40), (bx + 296, by + 20), OURS)
    arrow(c, (bx + 226, by + 40), (bx + 296, by + 100), ADV)
    box(c, bx + 302, by + 0, bx1 - 4, by + 46, ["A: refused, with a notice"], col=OURS)
    box(c, bx + 302, by + 78, bx1 - 4, by + 124, ["B: record the AI's inputs"], col=ADV)
    lines(c, bx + 12, by + 150, ["A is recommended: no new game site;", "B reads P1's controller slot (+0x4178)"])
    c.save("B4-p1-ai")


def d_playback():
    c = DCanvas(WIDE, 300)
    (ax, ay, ax1), (bx, by, bx1) = two(c, 300, "A · settings override the recording", "B · the recording wins", rec="b",
                                       na="the recorded air dodge is changed", nb="what you recorded is what plays")
    for (x0, y0, x1), out, col in (((ax, ay, ax1), "Up", BAD), ((bx, by, bx1), "Down In", OURS)):
        c.text(x0 + 8, y0 + 4, "recording: air dodge Down In", "s", FG)
        c.text(x0 + 8, y0 + 30, "Airdodge Direction setting: Up", "s", MUTED)
        m.chip_direction(c, x0 + 80, y0 + 120, 90, out, True)
        c.text(x0 + 150, y0 + 104, f"CPU air-dodges {out}", "s", col)
        c.text(x0 + 150, y0 + 130, "(same rule for DI, SDI, tech, throws)", "xs", MUTED)
    c.save("B5-playback-vs-settings")


def d_perfighter():
    c = DCanvas(WIDE, 290)
    (ax, ay, ax1), (bx, by, bx1) = two(c, 290, "A · one shared state (upstream)", "B · one per fighter", rec="b",
                                       na="Popo's SDI count is Nana's too", nb="a few bytes per CPU fighter")
    box(c, ax + 20, ay + 10, ax + 200, ay + 60, ["Popo"], col=CPU)
    box(c, ax1 - 200, ay + 10, ax1 - 20, ay + 60, ["Nana"], col=CPU)
    box(c, ax + 150, ay + 110, ax1 - 150, ay + 170, ["SDI / clatter counters,", "DI case, ledge delay, shield"])
    arrow(c, (ax + 110, ay + 64), (ax + 200, ay + 106))
    arrow(c, (ax1 - 110, ay + 64), (ax1 - 200, ay + 106))
    for i, nm in enumerate(["Popo", "Nana"]):
        x = bx + 20 + i * 290
        box(c, x, by + 10, x + 240, by + 60, [nm], col=CPU)
        box(c, x, by + 110, x + 240, by + 170, ["own counters,", "own DI case, own delays"], col=OURS)
        arrow(c, (x + 120, by + 64), (x + 120, by + 106))
    c.save("B6-per-fighter-state")


def scene(c, x, y, w, player_left, labels):
    """A stage line with the CPU in the middle and the player on one side; labels: (In, Out) arrow texts."""
    c.line([(x, y + 60), (x + w, y + 60)], LINE, 3)
    cx = x + w / 2
    px = x + 40 if player_left else x + w - 40
    c.circle(px, y + 40, 16, fill=P1)
    c.text(px, y + 32, "P", "b", FG, "m")
    c.circle(cx, y + 40, 16, fill=CPU)
    c.text(cx, y + 32, "C", "b", FG, "m")
    toward = -1 if player_left else 1
    inl, outl = labels
    if inl:
        arrow(c, (cx + toward * 22, y + 20), (cx + toward * 80, y + 20), OURS)
        c.text(cx + toward * 86, y + 12, inl, "xs", OURS, "r" if toward < 0 else "l")
    if outl:
        arrow(c, (cx - toward * 22, y + 20), (cx - toward * 80, y + 20), ADV)
        c.text(cx - toward * 86, y + 12, outl, "xs", ADV, "l" if toward < 0 else "r")


def d_inout():
    c = DCanvas(WIDE, 380)
    (a, b, cc) = three(c, 380, ["A · toward the opponent", "B · upstream air dodge", "C · upstream shield tilt"], rec=0,
                       notes=("In / Out mean the same on both sides", "flips depending on the side",
                              "Out is always to the right"))
    for (x, y, x1), (l1, l2) in ((a, (("In", "Out"), ("In", "Out"))),
                                 (b, (("Out", "In"), ("In", "Out"))),
                                 (cc, (("In", "Out"), ("Out", "In")))):
        c.text(x, y, "player on the left", "xs", MUTED)
        scene(c, x, y + 20, x1 - x, True, l1)
        c.text(x, y + 110, "player on the right", "xs", MUTED)
        scene(c, x, y + 130, x1 - x, False, l2)
    x, y, x1 = cc
    c.text(x, y + 204, "absolute: Out = right, In = left", "xxs", MUTED)
    x, y, x1 = b
    c.text(x, y + 204, "mirrors by launch speed only when", "xxs", MUTED)
    c.text(x, y + 220, "the player is on the right", "xxs", MUTED)
    c.save("B7-in-out-convention")


def d_fixes():
    changes = [
        ("Shield = None", "CPU shield never shrinks", "native shield decay", "cpu-05 Q1"),
        ("Tech Options empty", "Mistech Options silently off", "Mistech works on its own", "cpu-04 Q1"),
        ("SDI not set", "CPU never SDIs", "native CPU SDI", "cpu-02 Q3"),
        ("one-button chord held", "fires every frame", "fires once per press", "input-04 Q1"),
        ("recording of L frames", "stores and plays L - 1", "stores and plays all L", "input-01 Q1"),
        ("play an empty slot", "60 neutral frames", "refused, with a notice", "input-01 Q11"),
        ("L+R+A while recording", "recording continues", "recording stops", "input-01 Q13"),
        ("Playback while recording", "recording dropped silently", "playback refused", "input-01 Q17"),
        ("save-state load / reset", "input-delay queue kept", "queue cleared", "input-03 Q2"),
        ("notices under the menu", "expire unseen", "wait until it closes", "input-05 Q3"),
        ("Plus while our menu open", "pause menu on top", "Plus ignored", "core-03 Q3"),
        ("Mash situation lasts", "re-rolled every frame", "one roll per situation", "cpu-01 Q1"),
        ("Attack Angle after a mash", "stays on the CPU's attacks", "cleared when done", "cpu-06 Q4"),
    ]
    kept = [
        ("natural deaths in Training", "instant respawn, no invincibility", "state-01 Q3"),
        ("Full Hop default", "Yes", "cpu-06 Q8"),
        ("Ledge option Wait", "never times out", "cpu-03 Q3"),
        ("Mash after a throw", "ignores Mash Triggers; help text says so", "cpu-06 Q3"),
        ("HUD Off", "the plugin's notices stay visible", "input-05 Q5"),
        ("HUD Off", "item cursor hidden too", "core-05 Q6"),
    ]
    h = 70 + 28 * len(changes) + 60 + 28 * len(kept) + 20
    c = DCanvas(WIDE, h)
    c.rect(12, 12, WIDE - 12, h - 12, fill=PANEL, outline=OURS, width=3, radius=8)
    tag(c, WIDE - 160, 20, "recommended", OURS)
    c.text(28, 24, "Change from the fork (accept as a batch)", "h", FG)
    cols = (28, 330, 640, 1040)
    y = 58
    for t, x in zip(("when", "fork today", "clean room", "spec"), cols):
        c.text(x, y, t, "xs", MUTED)
    y += 24
    for w, old, new, ref in changes:
        c.text(cols[0], y, w, "xs", FG)
        c.text(cols[1], y, old, "xs", BAD)
        c.text(cols[2] - 34, y, "→", "xs", MUTED)
        c.text(cols[2], y, new, "xs", OURS)
        c.text(cols[3], y, ref, "xxs", MUTED)
        y += 28
    y += 16
    c.text(28, y, "Kept as the fork does it (confirm)", "h", FG)
    y += 34
    for w, beh, ref in kept:
        c.text(cols[0], y, w, "xs", FG)
        c.text(cols[1], y, beh, "xs", FG)
        c.text(cols[3], y, ref, "xxs", MUTED)
        y += 28
    c.save("B8-behaviour-changes")


def d_devcode():
    c = DCanvas(WIDE, 300)
    c.rect(12, 12, WIDE - 12, 288, fill=PANEL, outline=OURS, width=3, radius=8)
    tag(c, WIDE - 160, 20, "recommended", OURS)
    c.text(28, 24, "Upstream developer-only code", "h", FG)
    cols = (28, 420, 640, 860)
    y = 64
    for t, x in zip(("code", "shipped NRO", "diag NRO", "why"), cols):
        c.text(x, y, t, "xs", MUTED)
    y += 26
    for code, ship, diag, why in [("dev_config (file read per frame", "dropped", "only if asked", "a file read every"),
                                  ("  while L+R+A is held)", "", "", "  frame for developers"),
                                  ("debug hooks (debug_assertions)", "dropped", "only if asked", "never in a release"),
                                  ("raygun_printer (screen text)", "dropped", "only if asked", "debug output"),
                                  ("training_mod_sync (36 lines)", "parking_lot", "parking_lot", "the helper adds nothing")]:
        c.text(cols[0], y, code, "xs", FG)
        c.text(cols[1], y, ship, "xs", OURS if ship else FG)
        c.text(cols[2], y, diag, "xs", ADV if diag == "only if asked" else OURS)
        c.text(cols[3], y, why, "xs", MUTED)
        y += 28
    c.text(28, y + 14, "Alternative: keep each as diag-only behaviour now (more clean-room spec work, no player gain).",
           "xs", MUTED)
    c.save("B9-dev-only-code")


def d_readme():
    c = DCanvas(WIDE, 300)
    (a, b, cc) = three(c, 300, ["A · reuse what has no menu", "B · reuse every GIF now", "C · no media until built"],
                       rec=0, notes=("re-record menu GIFs after the build", "shows the old menu", "a text-only README"))
    for (x, y, x1), gifs in ((a, [("hot swap", OURS), ("practice stage", OURS), ("menu (later)", MUTED)]),
                             (b, [("hot swap", OURS), ("practice stage", OURS), ("old menu", BAD)]),
                             (cc, [("(none)", MUTED), ("", None), ("", None)])):
        c.text(x, y, "README", "s", FG)
        for i, (g, col) in enumerate(gifs):
            if not col:
                continue
            gx, gy = x + (i % 2) * ((x1 - x) / 2), y + 32 + (i // 2) * 70
            c.rect(gx, gy, gx + (x1 - x) / 2 - 10, gy + 60, fill=TILE, outline=col, width=2, radius=6)
            c.text(gx + ((x1 - x) / 2 - 10) / 2, gy + 22, g, "xs", col, "m")
    c.save("B10-readme-media")


def d_ledgechange():
    c = DCanvas(WIDE, 380)
    c.rect(12, 12, WIDE - 12, 368, fill=PANEL, outline=LINE, radius=8)
    x0, x1 = 300, WIDE - 40
    ev = {"grab": 0.04, "change": 0.30, "leave": 0.66, "regrab": 0.82}
    X = {k: x0 + v * (x1 - x0) for k, v in ev.items()}
    c.text(28, 24, "Ledge Options changed from Wait to Jump while the CPU is already hanging on the ledge", "h", FG)
    tx = tag(c, 28, 52, "applies to the fork", ADV)
    tag(c, tx + 8, 52, "and the clean room", ADV)
    ty = 112
    c.line([(x0, ty + 14), (x1, ty + 14)], LINE, 2)
    for k, lab in (("grab", "CPU grabs the ledge"), ("change", "menu change"), ("regrab", "CPU grabs again")):
        col = ADV if k == "change" else FG
        c.line([(X[k], ty + 6), (X[k], 350)], (60, 66, 80), 1, dash=(4, 4))
        c.text(X[k], ty - 12, lab, "xs", col, "m")
    jump_end = X["change"] + 215          # lane A: the CPU jumps off during the same hang
    lanes = [("A · re-roll on menu change", "recommended", "like SDI Direction", jump_end,
              [(X["grab"], X["change"], "hangs · holds Wait", LINE),
               (X["change"], jump_end, "re-rolled: Jump", OURS),
               (X["regrab"], x1, "rolls: Jump", LINE)],
              "the CPU acts on the new option during this hang", MUTED),
             ("B · apply on the next grab", "current", "the held roll stays", X["leave"],
              [(X["grab"], X["leave"], "holds Wait · the new setting is ignored", BAD),
               (X["regrab"], x1, "rolls: Jump", LINE)],
              "nothing changes until the CPU leaves the ledge and grabs again", BAD)]
    y = 150
    for name, badge, sub, off_from, segs, result, rcol in lanes:
        rec = badge == "recommended"
        c.rect(24, y - 8, WIDE - 24, y + 92, fill=TILE, outline=OURS if rec else LINE, width=3 if rec else 1, radius=6)
        c.text(36, y, name, "s", FG)
        tag(c, 36, y + 26, badge, OURS if rec else ADV)
        c.text(36, y + 56, sub, "xxs", MUTED)
        for xa, xb, lab, col in segs:
            box(c, xa, y + 4, xb, y + 48, [lab], col=col, fill=TILE_HI, font="xs")
        c.text((off_from + X["regrab"]) / 2, y + 20, "off the ledge", "xxs", MUTED, "m")
        c.text(x0 + 12, y + 64, result, "xxs", rcol)
        y += 112
    c.save("B11-ledge-change")


def d_pausehelper():
    """PAUSE-HELPER (2026-11-04): the game's pause loops against a created helper entry's phases."""
    h = 720
    c = DCanvas(WIDE, h)
    c.rect(12, 12, WIDE - 12, h - 12, fill=PANEL, outline=LINE, radius=8)
    c.text(28, 24, "Pause or quit while a helper entry is still loading: the game's two loops", "h", FG)
    tx = tag(c, 28, 54, "TrainingRooms p23", ADV)
    tag(c, tx + 8, 54, "the fork: same path by source, not console-confirmed", ADV)
    cols = [(330, 604), (616, 890), (902, 1176)]
    phases = [["1 · created", "entry_summon_fighter", "no actor: +0x4178 = 0"],
              ["2 · loading", "still no actor;", "no progress seen in the pause"],
              ["3 · ready", "actor + AI controller", "(actor +0x58)"]]
    c.text(28, 104, "Helper entry (slot 5-7)", "s", FG)
    c.text(28, 128, "FighterManager entry", "xs", MUTED)
    for i, ((x0, x1), lab) in enumerate(zip(cols, phases)):
        box(c, x0, 92, x1, 160, lab, col=OURS if i == 2 else LINE, fill=TILE_HI)
        if i:
            arrow(c, (cols[i - 1][1] + 1, 126), (x0 - 1, 126))
    # the enter loop: needs phase 6, so it never touches a helper
    y = 186
    lines(c, 28, y, [("Pause opens", FG), "enter loop main+0x1558540", "needs phase == 6", "reads [[e+0x4178]+0x58]"])
    for x0, x1 in cols[:2]:
        box(c, x0, y - 4, x1, y + 70, ["skipped", "(not phase 6: no access", "logged at pause open)"], col=LINE,
            fill=TILE, tcol=MUTED)
    box(c, cols[2][0], y - 4, cols[2][1], y + 70, ["only if phase 6", "(not traced)"], col=LINE, fill=TILE, tcol=MUTED)
    # the exit loop: skips only phase 3
    y = 280
    lines(c, 28, y, [("Pause closes / quit", FG), "exit loop main+0x1558f4c", "skips only phase 3",
                     "calls 0x6b82b0([e+0x4178], 1)"])
    for x0, x1 in cols[:2]:
        box(c, x0, y - 4, x1, y + 92, ["0x6b82b0(actor = 0, 1)", "Read64 @0x58 (null actor),",
                                       "then writes @0xc08 ...", "console: Data Abort"], col=BAD, fill=TILE)
    box(c, cols[2][0], y - 4, cols[2][1], y + 92, ["actor present: no fault", "(session 1 quit)",
                                                   "if walked: control type 1", "into its AI (to check)"],
        col=ADV, fill=TILE)
    c.text((cols[0][0] + cols[1][1]) / 2, y + 100, "b3 run, session 2 quit: slot 6 created 60 ms before the pause",
           "xxs", BAD, "m")
    c.text(28, 408, "Item 4 (does a plain resume also run the exit loop?): trace result pending", "xs", ADV)
    # the options
    opts = [("(a) Cancel at pause-open", True,
             ["retire the unfinished entry", "through finalize's own path;", "recreate it after the pause"],
             ["cost: one more helper load", "after the pause; a mid-scene", "retire must be proved safe"]),
            ("(b) Hold creation", False,
             ["create only when no pause", "can open in the load window"],
             ["cost: not enforceable; the", "player can pause on any frame"]),
            ("(c) Write phase 3", False,
             ["mark the actor-less entry", "phase 3 so the loop skips it"],
             ["cost: rejected; phase 3", "drives the Reload loader"]),
            ("(d) Skip in the exit loop", False,
             ["hook the game's loop to skip", "entries without an actor"],
             ["cost: a guard (needs approval);", "a hook on game code; the", "half-built state stays"])]
    w = (WIDE - 48 - 3 * 12) / 4
    for i, (title, rec, what, cost) in enumerate(opts):
        x0 = 24 + i * (w + 12)
        c.rect(x0, 440, x0 + w, h - 28, fill=TILE, outline=OURS if rec else LINE, width=3 if rec else 1, radius=6)
        c.text(x0 + 12, 452, title, "s", FG)
        if rec:
            tag(c, x0 + 12, 478, "recommended", OURS)
        lines(c, x0 + 12, 512, what, "xs", FG)
        lines(c, x0 + 12, 600, cost, "xs", BAD if not rec else MUTED)
    c.save("PAUSE-HELPER-null-access")


ALL = [d_pages, d_dropdown, d_delays, d_subscreens, d_firstpage, d_itemonload, d_replaycapture, d_preloadsamples,
       d_shortcuts, d_helptext, d_p1fallback, d_weights, d_paging, d_miirows, d_portraits, d_dichips, d_applychange,
       d_import, d_corrupt, d_name, d_panic, d_layoutmods, d_p1ai, d_playback, d_perfighter, d_inout, d_fixes,
       d_devcode, d_readme, d_ledgechange, d_pausehelper]


def main():
    only = set(sys.argv[1:])
    for fn in ALL:
        if not only or fn.__name__ in only:
            fn()
    print(len(list(OUT.glob("*.png"))), "images in", OUT)


if __name__ == "__main__":
    main()
