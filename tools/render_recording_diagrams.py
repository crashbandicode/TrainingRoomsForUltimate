#!/usr/bin/python3 -I
"""Draw the diagrams in docs/diagrams/recording-*.png for docs/guide/recording.md and docs/menu/recording.md, and
docs/diagrams/setup-a-versus-y.png for docs/guide/save-states.md and docs/menu/saves.md.

Same look as tools/render_decisions.py (its canvas, colours and helpers are reused). Every picture is drawn from the
facts in the two Recording docs; no game-derived file is used, so these are drawings, not Eden footage (the GIFs in
docs/media are footage and are listed in docs/MEDIA.md).

usage: /home/intpa/smash-training-lobby/tmp-integration/venv-pillow/bin/python -I tools/render_recording_diagrams.py [name ...]
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import render_decisions as d  # noqa: E402

OUT = Path(__file__).resolve().parent.parent / "docs" / "diagrams"
FG, MUTED, OURS, ADV, BAD, LINE = d.FG, d.MUTED, d.OURS, d.ADV, d.BAD, d.LINE
TILE, TILE_HI, PANEL, DARK, P1, CPU, SEL = d.TILE, d.TILE_HI, d.PANEL, d.DARK, d.P1, d.CPU, d.SEL

# the marks the pop-up and the page use: an orange floppy for saves, blue, purple and pink squares for the rest
SAVE_C = (226, 150, 70)
MACRO_C = (72, 150, 232)
COMBO_C = (170, 120, 226)
SEQ_C = (230, 120, 170)
GREEN = OURS
W = 1200


class Canvas(d.DCanvas):
    def save(self, name):
        OUT.mkdir(parents=True, exist_ok=True)
        self.img.convert("RGB").save(OUT / f"{name}.png", optimize=True)


def title(c, text, sub=None):
    c.text(20, 14, text, "h", FG)
    if sub:
        c.text(34 + c.textlength(text, "h"), 18, sub, "xs", MUTED)


def kind_box(c, x0, y0, x1, y1, kind, col, name, lines, hi=False):
    """A file box: its kind pill and name on top, then short lines."""
    c.rect(x0, y0, x1, y1, fill=TILE, outline=GREEN if hi else col, width=3 if hi else 2, radius=8)
    end = d.tag(c, x0 + 12, y0 + 10, kind, col)
    c.text(end + 10, y0 + 10, name, "s", FG)
    y = y0 + 40
    for ln in lines:
        c.text(x0 + 14, y, ln[0] if isinstance(ln, tuple) else ln, "xs", ln[1] if isinstance(ln, tuple) else MUTED)
        y += 20


def label(c, x, y, lines, col=MUTED, anchor="m", font="xs"):
    for i, ln in enumerate(lines):
        c.text(x, y + i * 18, ln, font, col, anchor)


def bullets(c, x, y, items):
    """Lines; a line that starts with two spaces continues the bullet above it."""
    for it in items:
        c.text(x, y, "   " + it.strip() if it.startswith("  ") else "• " + it, "xs", FG)
        y += 22


def bracket(c, x0, x1, y, col, up=True):
    k = 8 if up else -8
    c.line([(x0, y + k), (x0, y), (x1, y), (x1, y + k)], col, 2)


# ---------------------------------------------------------------------------------------------------
def pieces():
    c = Canvas(W, 470)
    title(c, "The five kinds of file", "and what plays or grades them")
    # the active macro bracket, over MACRO and CHAIN
    bracket(c, 440, 1176, 78, GREEN)
    label(c, 808, 48, ["ACTIVE MACRO: one for Player 1, one for the CPU.  Its Play chord plays it."], GREEN)
    kind_box(c, 24, 96, 340, 204, "SAVE STATE", SAVE_C, "Save 1",
             ["Both fighters, where they stood.", "From: the Save chord; Record", "(as \"Macro N · start\")."])
    kind_box(c, 440, 96, 756, 204, "MACRO", MACRO_C, "Macro 1",
             ["A recording of inputs.", "From: Record (ZR + D-pad Down).", "Plays on Player 1 or the CPU."])
    kind_box(c, 860, 96, 1176, 204, "MACRO", MACRO_C, "Chain 1",
             ["Macros played in order (1, 2, 3).", "From: + New chain.", "A chain can't hold a chain."])
    kind_box(c, 440, 262, 756, 370, "COMBO", COMBO_C, "Combo 1",
             ["The links of a combo you drill.", "From: + Save last combo...,", "or Make combo on a macro."])
    kind_box(c, 860, 262, 1176, 370, "SEQUENCE", SEQ_C, "Sequence 1",
             ["Combos drilled one after another.", "From: + New sequence.", "Streak and best are kept."])
    kind_box(c, 24, 262, 340, 370, "FRAME STRIP", MUTED, "",
             ["The strip under the match", "sees your last hit and", "follow-up as you play."])
    # connections
    d.arrow(c, (440, 150), (340, 150), SAVE_C)
    label(c, 390, 106, ["starts from"], MUTED, "m", "xxs")
    label(c, 390, 160, ["Play warps", "there first"], MUTED, "m", "xxs")
    d.arrow(c, (756, 150), (860, 150), MACRO_C)
    label(c, 808, 106, ["numbered"], MUTED, "m", "xxs")
    d.arrow(c, (598, 204), (598, 262), COMBO_C)
    label(c, 610, 222, ["Make combo", "(plays it once)"], MUTED, "l", "xxs")
    d.arrow(c, (756, 316), (860, 316), SEQ_C)
    label(c, 808, 272, ["numbered"], MUTED, "m", "xxs")
    d.arrow(c, (340, 316), (440, 316), COMBO_C)
    label(c, 390, 270, ["+ Save", "last combo..."], MUTED, "m", "xxs")
    # the drill bracket, under COMBO and SEQUENCE
    bracket(c, 440, 1176, 388, GREEN, up=False)
    label(c, 808, 400, ["THE DRILL: one combo or one sequence at a time.  The combo helper grades it."], GREEN)
    c.text(24, 224, "Load only warps: it starts nothing.", "xxs", MUTED)
    c.text(24, 440, "A save, a macro, a chain, a combo and a sequence are separate files. Renaming one keeps its connections.",
           "xs", MUTED)
    c.save("recording-pieces")


def record_flow():
    c = Canvas(W, 360)
    title(c, "Record, then Play", "ZR + D-pad Down starts and stops a recording")
    # the timeline
    x0, xs, x1 = 130, 700, 820
    c.rect(x0, 78, xs, 120, fill=TILE_HI, outline=MACRO_C, width=2, radius=6)
    c.text((x0 + xs) / 2, 91, "your inputs, one recorded frame per game frame", "xs", FG, "m")
    c.rect(xs, 78, x1, 120, fill=DARK, outline=MUTED, width=2, radius=6)
    c.text((xs + x1) / 2, 91, "idle end", "xs", MUTED, "m")
    for x, word in ((x0, "start"), (x1, "stop")):
        c.line([(x, 66), (x, 132)], GREEN, 3)
        c.text(x, 46, word, "xs", GREEN, "m")
        c.text(x, 134, "ZR + D-pad Down", "xxs", GREEN, "m")
    label(c, x1 + 24, 80, ["dropped when Crop", "trailing neutral is", "On (the default)"], MUTED, "l", "xxs")
    # results
    kind_box(c, 24, 214, 440, 318, "SAVE STATE", SAVE_C, "Macro 1 · start",
             ["Both fighters, as they stood", "when you pressed Record."])
    kind_box(c, 560, 214, 1176, 318, "MACRO", MACRO_C, "Macro 1",
             ["The inputs, named Macro N (rename it later).",
              ("It becomes the active macro of the side that recorded.", GREEN),
              "A take that is idle throughout saves nothing."])
    d.arrow(c, (x0, 152), (150, 214), SAVE_C)
    d.arrow(c, (x1, 152), (x1, 214), MACRO_C)
    d.arrow(c, (560, 266), (440, 266), SAVE_C)
    label(c, 500, 232, ["starts from"], MUTED, "m", "xxs")
    c.text(24, 334, "Play on that side then warps to Macro 1 · start once (Start save On), and plays the inputs.", "xs", MUTED)
    c.save("recording-record-flow")


def passes():
    c = Canvas(W, 420)
    title(c, "Where Play warps", "once, before the first pass; never between passes")
    rows = [
        ("Loop 1 (the default)", [("warp", 1), ("macro", 1)]),
        ("Loop 3", [("warp", 1), ("macro", 1), ("macro", 1), ("macro", 1)]),
        ("Chain of 3, Loop 2", [("warp", 1), ("1", 1), ("2", 1), ("3", 1), ("1", 1), ("2", 1), ("3", 1)]),
        ("Loop ∞", [("warp", 1), ("macro", 1), ("macro", 1), ("macro", 1), ("...", 1)]),
    ]
    unit = 108
    y = 64
    for name, segs in rows:
        c.text(24, y + 10, name, "s", FG)
        x = 290
        for text, _ in segs:
            if text == "warp":
                d.box(c, x, y, x + 78, y + 36, ["warp"], col=SAVE_C, fill=TILE, font="xs")
                x += 84
            elif text == "...":
                c.text(x + 10, y + 8, "until the Play chord", "xxs", MUTED)
                c.text(x + 10, y + 22, "is pressed again", "xxs", MUTED)
            else:
                col = MACRO_C
                d.box(c, x, y, x + unit, y + 36, [text if text != "macro" else "macro"], col=col, fill=TILE_HI, font="xs")
                x += unit + 6
        y += 68
    c.text(24, 346, "Warp: both fighters go to the start save (the Start save switch is On). With it Off, Play starts from where they stand.",
           "xs", MUTED)
    c.text(24, 368, "A chain warps to its first macro's start save and repeats as one unit; the macros inside never warp.", "xs", MUTED)
    c.save("recording-passes")


def stage_fighter(c, x, y, col, text, dashed=False, faint=False):
    fill = None if dashed else col
    c.circle(x, y - 22, 14, fill=fill, outline=col, width=2)
    c.line([(x, y - 8), (x, y)], col, 4)
    c.text(x, y + 6, text, "xxs", col if not faint else MUTED, "m")


def placement():
    c = Canvas(W, 410)
    title(c, "Edit placement", "an offset added to the side that plays")
    for px0, ptitle, off in ((12, "Play: both warp to the start save", False), (606, "Play with an offset on the CPU", True)):
        c.rect(px0, 52, px0 + 582, 300, fill=PANEL, outline=LINE, radius=8)
        c.text(px0 + 14, 62, ptitle, "s", FG)
        ground = 230
        c.line([(px0 + 30, ground), (px0 + 552, ground)], LINE, 3)
        # where the start save puts them
        p1x, cx = px0 + 170, px0 + 330
        stage_fighter(c, p1x, ground, P1, "Player 1")
        if off:
            stage_fighter(c, cx, ground, CPU, "start save", dashed=True)
            nx = cx + 110
            stage_fighter(c, nx, ground, CPU, "CPU lands here")
            d.arrow(c, (cx + 18, ground - 52), (nx - 18, ground - 52), ADV, 3)
            c.text((cx + nx) / 2, ground - 80, "x+ x+ x+  (offset)", "xxs", ADV, "m")
        else:
            stage_fighter(c, cx, ground, CPU, "CPU")
            c.text(px0 + 20, 100, "The start save holds where both stood", "xxs", MUTED)
            c.text(px0 + 20, 118, "when you pressed Record.", "xxs", MUTED)
        if off:
            c.text(px0 + 20, 100, "Player 1 still lands on the start save;", "xxs", MUTED)
            c.text(px0 + 20, 118, "only the side that plays is moved.", "xxs", MUTED)
    left = [("x- / x+   y- / y+", "move the start a little"), ("X- / X+   Y- / Y+", "four times as far"),
            ("Momentum", "Keep recorded speed (the default),"), ("", "or Start at rest")]
    right = [("Write", "saves the offset into the macro file"), ("Keep", "uses it until you close the game"),
             ("New", "saves it into a copy"), ("Back / B", "leaves without writing")]
    y = 314
    for k, v in left:
        c.text(24, y, k, "xs", ADV)
        c.text(250, y, v, "xs", MUTED)
        y += 20
    y = 314
    for k, v in right:
        c.text(640, y, k, "xs", ADV)
        c.text(760, y, v, "xs", MUTED)
        y += 20
    c.save("recording-placement")


def grading():
    c = Canvas(W, 596)
    title(c, "Grading one link", "On-time tolerance 1 frame (the default); the frame numbers are an example")
    # the ruler
    gx0, gx1, fy = 300, 1150, 86
    k = (gx1 - gx0) / 12

    def fx(f):
        return gx0 + f * k

    for f in range(0, 13):
        c.line([(fx(f), fy), (fx(f), fy + 8)], MUTED, 1)
        c.text(fx(f), fy - 20, str(f), "xxs", MUTED, "m")
    c.text(24, fy - 20, "frames after A1", "xxs", MUTED)
    c.text(24, fy + 2, "(A1: the first frame Player 1 can act)", "xxs", MUTED)
    c.line([(gx0, fy + 4), (gx1, fy + 4)], LINE, 2)
    # recorded press at frame 5, tolerance band 4..6
    rec_f = 5
    rows = [
        ("on time", 5, "counts", GREEN, "the press comes at frame 5"),
        ("1 frame late", 6, "counts", GREEN, "inside the tolerance of 1"),
        ("3 frames late", 8, "drops", BAD, "outside the tolerance"),
        ("no press", None, "drops", BAD, "a missing event"),
    ]
    y = 124
    c.text(24, y - 12, "Recorded: Attack press at frame 5", "s", COMBO_C)
    y += 20
    for name, f, verdict, col, note in rows:
        c.rect(fx(rec_f - 1), y - 4, fx(rec_f + 1), y + 30, fill=(34, 44, 40), outline=None)
        c.line([(gx0, y + 13), (gx1, y + 13)], (52, 58, 72), 1)
        c.text(24, y + 6, name, "xs", FG)
        if f is not None:
            c.circle(fx(f), y + 13, 9, fill=col)
        d.tag(c, 1100 - 0, y + 4, verdict, col)
        c.text(300, y + 34, note, "xxs", MUTED)
        y += 58
    c.text(fx(rec_f), 118, "recorded", "xxs", COMBO_C, "m")
    # what else drops
    py = y + 18
    half = W // 2
    c.rect(12, py, half - 6, py + 176, fill=PANEL, outline=BAD, radius=8)
    c.text(26, py + 10, "Also drops the attempt", "h", FG)
    items = ["a different move comes out, even when", "  the inputs match",
             "a different direction (facing-relative)",
             "the exchange ends before the step: 20 frames", "  after the CPU can act again, or on a",
             "  shielded hit (a gap of 21 opens a new one)"]
    bullets(c, 40, py + 46, items)
    c.rect(half + 6, py, W - 12, py + 176, fill=PANEL, outline=GREEN, radius=8)
    c.text(half + 20, py + 10, "Does not drop it", "h", FG)
    bullets(c, half + 34, py + 46, ["a link that is not a true combo (TRUE is", "  shown, not required)",
                                    "the stick drift between the events", "  (shown, not graded)",
                                    "a button still held when the hit lands", "  (its release is not graded)"])
    c.save("recording-grading")


def mismatch():
    c = Canvas(W, 380)
    title(c, "Fighter Mismatch", "Play, a replay clip or a combo drill meets a different fighter")
    d.box(c, 24, 60, 250, 120, ["Play a macro,", "a chain, or drill", "a combo"], col=MACRO_C, font="xs")
    d.box(c, 300, 60, 526, 120, ["Recorded fighter", "is not the one", "you have?"], col=ADV, font="xs")
    d.box(c, 576, 60, 802, 120, ["Ask before", "switching fighter", "is On?"], col=ADV, font="xs")
    d.box(c, 852, 48, 1176, 132, ["FIGHTER MISMATCH", "Keep current fighter", "Switch to recorded fighter"], col=GREEN, font="xs")
    d.arrow(c, (250, 90), (300, 90), MUTED)
    d.arrow(c, (526, 90), (576, 90), MUTED)
    d.arrow(c, (802, 90), (852, 90), MUTED)
    c.text(551, 70, "yes", "xxs", GREEN, "m")
    c.text(827, 70, "yes", "xxs", GREEN, "m")
    # no branches
    d.arrow(c, (413, 120), (413, 190), MUTED)
    c.text(425, 142, "no", "xxs", BAD)
    d.arrow(c, (689, 120), (689, 190), MUTED)
    c.text(701, 142, "no (Off)", "xxs", BAD)
    d.box(c, 300, 190, 526, 240, ["It plays on", "the fighter you have"], col=LINE, font="xs")
    d.box(c, 576, 190, 802, 240, ["The fighter you have", "stays, as with Keep"], col=LINE, font="xs")
    # the answers
    d.arrow(c, (1014, 132), (1014, 190), MUTED)
    d.box(c, 852, 190, 1176, 250, ["Keep: plays on your fighter.", "Switch: changes fighter, then plays.", "B: cancels; nothing plays."],
          col=GREEN, font="xs")
    c.text(24, 290, "The cursor starts on Keep; Player 1's D-pad moves it, A chooses, B cancels. Play waits until it is answered.", "xs", MUTED)
    c.text(24, 314, "It asks for: a macro whose fighter differs, a chain's first macro only, a replay clip, a combo drill.", "xs", MUTED)
    c.text(24, 338, "It never asks on Load. A sequence may mix fighters.", "xs", MUTED)
    c.save("recording-mismatch")


def page_popup():
    c = Canvas(W, 624)
    mid = 600
    # ---- the pop-up
    c.rect(12, 12, mid - 8, 480, fill=PANEL, outline=LINE, radius=8)
    c.text(26, 22, "The pop-up (ZR + D-pad Right)", "h", FG)
    c.text(32, 60, "Recording", "h", FG)
    d.tabs(c, 200, 58, ["L Player", "CPU R"], 0, "xs")
    px = 32
    c.rect(px, 98, px + 180, 122, outline=GREEN, width=2, radius=3)
    c.text(px + 90, 103, "Save states & Macros", "xxs", FG, "m")
    c.rect(px + 190, 98, px + 340, 122, outline=LINE, width=1, radius=3)
    c.text(px + 265, 103, "Combos & Sequences", "xxs", MUTED, "m")
    c.text(px + 350, 103, "ZL / ZR", "xxs", MUTED)
    # the lists
    lx0, lx1 = 32, 340
    y = 138
    d.tag(c, lx0, y, "SAVE STATES (3)", SAVE_C)
    y += 28
    rows = [("Save 1", GREEN), ("Macro 2 · start", FG), ("Macro 1 · start", FG)]
    c.rect(lx0 - 4, y - 4, lx1 + 4, y + 2 * 28 + 28, outline=GREEN, width=2, radius=4)
    for i, (t, col) in enumerate(rows):
        c.rect(lx0, y, lx1, y + 24, fill=TILE, outline=LINE, radius=3)
        c.text(lx0 + 10, y + 4, t, "xs", col)
        if i == 0:
            c.text(lx1 - 8, y + 5, "bound", "xxs", GREEN, "r")
        y += 28
    y += 14
    d.tag(c, lx0, y, "MACROS (3)", MACRO_C)
    y += 28
    c.rect(lx0 - 4, y - 4, lx1 + 4, y + 28 + 28, outline=GREEN, width=2, radius=4)
    for i, (t, col) in enumerate([("Macro 2", GREEN), ("Chain 1   (2)", FG), ("+ New chain", MUTED)]):
        c.rect(lx0, y, lx1, y + 24, fill=TILE_HI if i == 0 else TILE, outline=LINE, radius=3)
        c.text(lx0 + 10, y + 4, t, "xs", col)
        if i == 0:
            c.text(lx1 - 8, y + 5, "active", "xxs", GREEN, "r")
        y += 28
        if i == 1:
            y += 6
    # connections
    cx0, cx1 = 372, mid - 24
    c.text(cx0, 138, "CONNECTIONS", "b", MUTED)
    d.box(c, cx0, 160, cx1, 200, ["MACRO  Macro 2"], col=MACRO_C, font="xxs")
    c.text(cx0, 210, "starts from", "xxs", MUTED)
    d.box(c, cx0 + 10, 228, cx1, 258, ["SAVE STATE", "Macro 2 · start"], col=SAVE_C, font="xxs")
    c.text(cx0, 270, "in chains", "xxs", MUTED)
    d.box(c, cx0 + 10, 288, cx1, 318, ["MACRO  Chain 1"], col=MACRO_C, font="xxs")
    c.text(32, 448, "A set active   X loop   B close", "xs", MUTED)
    # ---- the page
    c.rect(mid + 8, 12, W - 12, 480, fill=PANEL, outline=LINE, radius=8)
    c.text(mid + 22, 22, "The page (menu, Recording)", "h", FG)
    gx = mid + 22
    groups = [("Save states & Macros (7)", True), ("Combos & Sequences (4)", False), ("Replays", False), ("Options", False),
              ("Reset page to defaults", False)]
    y = 60
    for t, sel in groups:
        c.rect(gx, y, gx + 190, y + 26, fill=TILE_HI if sel else TILE, outline=GREEN if sel else LINE, width=2 if sel else 1,
               radius=3)
        c.text(gx + 8, y + 5, t, "xxs", FG if sel else MUTED)
        y += 32
    lx0, lx1 = gx + 206, W - 30
    y = 60
    d.tag(c, lx0, y, "SAVE STATES", SAVE_C)
    y += 26
    c.rect(lx0 - 4, y - 3, lx1 + 4, y + 2 * 26 + 22, outline=GREEN, width=2, radius=4)
    for t, col in (("Save 1", GREEN), ("Macro 2 · start", FG), ("Macro 1 · start", FG)):
        c.rect(lx0, y, lx1, y + 22, fill=TILE, outline=LINE, radius=3)
        c.text(lx0 + 8, y + 4, t, "xxs", col)
        y += 26
    y += 12
    d.tag(c, lx0, y, "MACROS", MACRO_C)
    y += 26
    c.rect(lx0 - 4, y - 3, lx1 + 4, y + 26 + 22, outline=GREEN, width=2, radius=4)
    for i, (t, col) in enumerate((("Macro 2", GREEN), ("Chain 1   (2)", FG), ("+ New chain", MUTED))):
        c.rect(lx0, y, lx1, y + 22, fill=TILE, outline=LINE, radius=3)
        c.text(lx0 + 8, y + 4, t, "xxs", col)
        y += 26
        if i == 1:
            y += 6
    c.rect(gx, 316, W - 30, 396, fill=TILE, outline=LINE, radius=4)
    c.text(gx + 10, 322, "CONNECTIONS", "b", MUTED)
    c.text(gx + 10, 344, "Macro 2: starts from Macro 2 · start", "xxs", FG)
    c.text(gx + 10, 364, "in chain: Chain 1   recorded on Mario", "xxs", FG)
    c.text(gx, 448, "A open   X loop in MACROS   ZR role   L/R page   B close", "xxs", MUTED)
    # ---- the notes
    c.text(24, 496, "A acts at once: it binds a save, sets a macro active,", "xs", FG)
    c.text(24, 516, "or drills a combo. No file screens: rename, crop and", "xs", FG)
    c.text(24, 536, "delete are on the page. L Player / CPU R choose the side", "xs", FG)
    c.text(24, 556, "that Record and Edit placement use.", "xs", FG)
    c.text(mid + 22, 496, "A opens the file's own screen (rename, crop, delete ...).", "xs", FG)
    c.text(mid + 22, 516, "ZR switches the Player and CPU views (CPU hides", "xs", FG)
    c.text(mid + 22, 536, "Combos & Sequences). The left column lists the groups,", "xs", FG)
    c.text(mid + 22, 556, "with how many files each holds.", "xs", FG)
    c.text(W / 2, 590, "Same in both: the two lists, the hot five in a green outline, green rows (bound, active, drill), CONNECTIONS.",
           "xs", GREEN, "m")
    c.save("recording-page-popup")


def spot(c, x, ground, col, text):
    c.ellipse(x - 16, ground - 4, x + 16, ground + 5, outline=col, width=2)
    c.text(x, ground + 9, text, "xxs", col, "m")


def mini_stage(c, x0, x1, y0, caption, p1, cpu, note):
    """A strip of stage: the bound save's two spots (grey rings), the setup's two spots (gold rings) and the
    fighters at p1 / cpu (fractions of the strip's width)."""
    w = x1 - x0
    ground = y0 + 62
    c.text(x0, y0, caption, "s", FG)
    c.text(x1, y0 + 3, note, "xxs", MUTED, "r")
    c.rect(x0, y0 + 22, x1, ground + 28, fill=PANEL, outline=LINE, radius=6)
    c.line([(x0 + 10, ground), (x1 - 10, ground)], LINE, 3)
    for f, label in ((0.42, "bound save"), (0.58, "")):
        spot(c, x0 + w * f, ground, MUTED, label)
    for f, label in ((0.10, "setup"), (0.90, "setup")):
        spot(c, x0 + w * f, ground, ADV, label)
    stage_fighter(c, x0 + w * p1, ground, P1, "")
    stage_fighter(c, x0 + w * cpu, ground, CPU, "")


def a_versus_y():
    c = Canvas(W, 556)
    title(c, "A or Y on a setup", "you choose in the Setup library, then close the menu")
    panels = [
        (12, "A", "go there once", SEL,
         [("Both fighters move to the setup", "needs Save States On, the same stage and the same fighters"),
          ("The next Load goes to your bound save", "as before: A moves you once")],
         (0.10, 0.90), (0.42, 0.58)),
        (606, "Y", "make Load go there", CPU,
         [("Nobody moves", "works with any stage or fighters"),
          ("Load, and Auto-load on death, go to the setup", "until you Save or bind another save")],
         (0.30, 0.70), (0.10, 0.90)),
    ]
    for x0, key, head, col, steps, after_menu, after_load in panels:
        x1 = x0 + 582
        c.rect(x0, 52, x1, 540, fill=PANEL, outline=LINE, radius=8)
        c.rect(x0, 52, x1, 96, fill=col, radius=8)
        c.text(x0 + 16, 62, key, "hh", FG)
        c.text(x0 + 52, 66, head, "h", FG)
        y = 112
        c.circle(x0 + 26, y + 9, 11, outline=FG, width=2)
        c.text(x0 + 26, y + 1, "1", "s", FG, "m")
        c.text(x0 + 50, y, "Close the menu (nothing moves until you do)", "s", FG)
        y += 46
        for i, (main, sub) in enumerate(steps, 2):
            c.circle(x0 + 26, y + 9, 11, outline=FG, width=2)
            c.text(x0 + 26, y + 1, str(i), "s", FG, "m")
            c.text(x0 + 50, y, main, "s", FG)
            c.text(x0 + 50, y + 22, sub, "xxs", MUTED)
            y += 58
        mini_stage(c, x0 + 20, x1 - 20, 300, "after the menu closes", *after_menu,
                   "both at the setup" if key == "A" else "no one has moved")
        mini_stage(c, x0 + 20, x1 - 20, 420, "the next Load", *after_load,
                   "back at your bound save" if key == "A" else "both at the setup")
    c.save("setup-a-versus-y")


ALL = {"a_versus_y": a_versus_y, "pieces": pieces, "record_flow": record_flow, "passes": passes, "placement": placement, "grading": grading,
       "mismatch": mismatch, "page_popup": page_popup}

if __name__ == "__main__":
    for name in (sys.argv[1:] or list(ALL)):
        ALL[name]()
        print("drew", name)
