#!/usr/bin/python3 -I
"""Draw the guide diagrams in docs/diagrams/ for the readability pass (cycle numbers, frame strip timeline, ...).

Same look as tools/render_recording_diagrams.py and tools/render_decisions.py (their canvas, colours and helpers are
reused). Every picture is drawn from the facts in the page it sits on; no game-derived file is used, so these are
drawings, not Eden footage.

usage: /home/intpa/smash-training-lobby/tmp-integration/venv-pillow/bin/python -I tools/render_guide_diagrams.py [name ...]
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import render_decisions as d  # noqa: E402
from render_recording_diagrams import (  # noqa: E402,F401
    ADV, BAD, CPU, FG, GREEN, LINE, MUTED, P1, PANEL, TILE, TILE_HI, W, Canvas, bracket, title,
)


def cycle_numbers():
    c = Canvas(W, 360)
    title(c, "The numbers in Character Cycles", "each press of a cycle chord loads the next number; the fighters are an example")
    rows = [("Player 1", P1, "Cycle Player: ZL + D-pad Right",
             [("0", "Mario", "started as"), ("1", "Link", ""), ("2", "Kirby", ""), ("-", "Mii Gunner", "greyed: skipped"),
              ("3", "Samus", "")]),
            ("CPU", CPU, "Cycle CPU: ZL + D-pad Left",
             [("0", "Pikachu", "started as"), ("1", "Bowser", ""), ("2", "Peach", "")])]
    y = 76
    for who, col, chord, tiles in rows:
        c.text(24, y + 4, who, "h", col)
        c.text(24, y + 32, chord, "xxs", MUTED)
        x = 280
        for n, name, note in tiles:
            greyed = n == "-"
            c.rect(x, y, x + 140, y + 78, fill=TILE, outline=LINE if greyed else col, width=2, radius=8)
            if not greyed:
                c.text(x + 14, y + 6, n, "hh", col)
            c.text(x + 14, y + 42, name, "s", MUTED if greyed else FG)
            if note:
                c.text(x + 14, y + 60, note, "xxs", MUTED)
            x += 170
        xs = [280 + 170 * i for i, tile in enumerate(tiles) if tile[0] != "-"]
        for a, b in zip(xs, xs[1:]):
            if b - a == 170:
                d.arrow(c, (a + 142, y + 39), (b - 2, y + 39), col)
            else:
                c.line([(a + 70, y - 2), (a + 70, y - 12), (b + 70, y - 12)], col, 2)
                d.arrow(c, (b + 70, y - 12), (b + 70, y - 2), col)
        first_mid, last_mid = 280 + 70, 280 + (len(tiles) - 1) * 170 + 70
        c.line([(last_mid, y + 80), (last_mid, y + 104), (first_mid, y + 104)], col, 2, dash=(6, 4))
        d.arrow(c, (first_mid, y + 104), (first_mid, y + 82), col)
        c.text(last_mid - 10, y + 108, "after the last, back to 0", "xxs", MUTED, "r")
        y += 150
    c.save("cycle-numbers")


def strip_timeline():
    c = Canvas(W, 470)
    title(c, "One exchange on the frame strip", "frames run left to right; the numbers are an example")
    k = 12  # pixels per frame
    ax, ay = 60, 214
    hit0, you, free = 100, 300, 560
    lim = free + 20 * k
    c.line([(ax, ay), (1150, ay)], LINE, 3)
    # the CPU's hitstun, the 20-frame band, and what comes after
    c.rect(hit0, ay - 30, free, ay - 6, fill=(60, 44, 96), outline=None)
    c.text((hit0 + free) / 2, ay - 24, "the CPU is in hitstun", "xs", FG, "m")
    c.rect(free, ay - 30, lim, ay - 6, fill=(86, 70, 30), outline=None)
    c.text((free + lim) / 2, ay - 24, "still the same exchange", "xs", FG, "m")
    c.rect(lim, ay - 30, 1140, ay - 6, fill=(40, 44, 54), outline=None)
    c.text((lim + 1140) / 2, ay - 24, "later: a new exchange", "xs", MUTED, "m")
    # the marks on the axis
    marks = [(hit0, "last hit", "frame 0", FG), (you, "you can act", "your first free frame", P1),
             (free, "the CPU can act", "its first free frame", CPU), (lim, "+20 frames", "after the CPU can act", ADV)]
    for x, name, sub, col in marks:
        c.line([(x, ay - 40), (x, ay + 12)], col, 3)
        c.text(x, ay + 18, name, "s", col, "m")
        c.text(x, ay + 40, sub, "xxs", MUTED, "m")
    # ADVANTAGE bracket
    bracket(c, you, free, ay + 76, GREEN, up=False)
    c.text((you + free) / 2, ay + 88, "ADVANTAGE: the CPU's first free frame minus yours (here plus)", "xs", GREEN, "m")
    # the three follow-up hits
    hits = [(free - 4 * k, "TRUE by 4", "the CPU never got a free frame before the hit (shown as -4)", (111, 195, 154)),
            (free + 9 * k, "DROP +9", "the CPU could act for 9 frames first", BAD),
            (lim + 8 * k, "not a drop", "a new exchange; ADVANTAGE waits", MUTED)]
    for x, name, sub, col in hits:
        d.arrow(c, (x, 74), (x, ay - 42), col, 3)
        c.text(x, 50, name, "s", col, "m")
    y = 330
    for x, name, sub, col in hits:
        c.text(24, y, name + ": " + sub, "xs", col)
        y += 22
    c.text(24, y + 8, "A follow-up is a later hit from a different action, on hit, within 20 frames of the CPU being free.", "xs", MUTED)
    c.text(24, y + 30, "Hits inside one multi-hit move are not follow-ups, and neither is a hit on shield.", "xs", MUTED)
    c.save("frame-strip-timeline")

ALL = {"cycle_numbers": cycle_numbers, "strip_timeline": strip_timeline}

if __name__ == "__main__":
    for name in (sys.argv[1:] or list(ALL)):
        ALL[name]()
        print("drew", name)
