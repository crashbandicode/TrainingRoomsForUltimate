#!/usr/bin/python3 -I
"""Render the illustration of docs/design/rc8-binds.md (docs/design/rc8-binds.png): the two new rows in the Button
chords editor, the speed pop-up while the combo is held, and where each place writes the speed.

Same look as tools/render_decisions.py (its canvas and helpers are reused). Drawn, not captured: no game-derived file.
Pillow only.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import render_decisions as rd  # noqa: E402

m = rd.m
OUT = Path(__file__).resolve().parent.parent / "docs" / "design" / "rc8-binds.png"
W, H = rd.WIDE, 470

ROWS = [
    ("Save State Save", "ZL + D-pad Down"),
    ("Save State Load", "ZL + D-pad Up"),
    ("Record", "ZR + D-pad Down"),
    ("Play on CPU", "ZR + D-pad Up"),
    ("Play on Player 1", "ZR + D-pad Left"),
    ("Recording pop-up", "ZR + D-pad Right"),
    ("Cycle Player", "ZL + D-pad Right"),
    ("Cycle CPU", "ZL + D-pad Left"),
    ("Input display", "R + D-pad Down"),
    ("Game speed", "R + D-pad Up"),
]


def triangle(c, x, y, left, col):
    """A D-pad direction mark (the game draws its own glyph there)."""
    if left:
        c.polygon([(x, y + 8), (x + 12, y), (x + 12, y + 16)], fill=col)
    else:
        c.polygon([(x + 12, y + 8), (x, y), (x, y + 16)], fill=col)


def editor(c, x0, y0, x1, y1):
    rd.panel(c, x0, y0, x1, y1, "1 · Button chords: 2 new rows", note="Display & Controls > Controls")
    y = y0 + 48
    for i, (label, chord) in enumerate(ROWS):
        new = i >= 8
        if new:
            c.rect(x0 + 10, y - 5, x1 - 10, y + 23, fill=m.HILITE)
            c.rect(x0 + 10, y - 5, x0 + 14, y + 23, fill=m.OURS)
        c.text(x0 + 24, y, label, "xs", m.FG if new else m.MUTED)
        c.text(x1 - 20, y, chord, "xs", m.OURS if new else m.MUTED, "r")
        y += 31


def popup(c, x0, y0, x1, y1):
    rd.panel(c, x0, y0, x1, y1, "2 · Hold: the pop-up", note="release: pop-up goes, speed stays")
    # The notice slot where the plugin's notices draw.
    nx0, ny0, nx1, ny1 = x0 + 70, y0 + 56, x1 - 70, y0 + 142
    c.rect(nx0, ny0, nx1, ny1, fill=(10, 12, 16), outline=m.LINE, width=2, radius=6)
    c.text(nx0 + 14, ny0 + 10, "Game speed", "b", m.ADV)
    mid = (nx0 + nx1) / 2
    triangle(c, mid - 70, ny0 + 44, True, m.FG)
    c.text(mid, ny0 + 40, "1/2", "hh", m.FG, "m")
    triangle(c, mid + 58, ny0 + 44, False, m.FG)
    steps = [
        ("hold R + D-pad Up", "opens at the current speed"),
        ("D-pad Left / Right", "one speed, stops at the ends"),
        ("other chords", "blocked while it is open"),
        ("P1's other input", "still reaches the game"),
    ]
    y = y0 + 160
    for a, b in steps:
        c.text(x0 + 24, y, a, "xs", m.FG)
        c.text(x0 + 40, y + 18, b, "xs", m.MUTED)
        y += 42
    y += 4
    c.text(x0 + 24, y, "Training: 1/4  1/2  2/3  x1  x1.5", "xs", m.FG)
    c.text(x0 + 24, y + 22, "Lobby:    1/4  1/2  x1", "xs", m.FG)


def writes(c, x0, y0, x1, y1):
    rd.panel(c, x0, y0, x1, y1, "3 · Where it is written", note="the plugin never writes a rate global")
    bw = x1 - x0 - 40
    y = y0 + 52
    rd.box(c, x0 + 20, y, x0 + 20 + bw, y + 92, [
        "Training: PauseMenu+0xb54 = value",
        "then 0x1bbbd30(menu, value, +0x538)",
        "the native menu shows the same speed;",
        "the game applies it next update",
    ], col=m.OURS)
    y += 108
    rd.box(c, x0 + 20, y, x0 + 20 + bw, y + 92, [
        "Lobby practice: one plugin slot",
        "{in_use 1, N 2|4} in *(0x52c3610)",
        "taken and freed the native way;",
        "released before the round starts",
    ], col=m.CPU)
    y += 108
    rd.box(c, x0 + 20, y, x0 + 20 + bw, y + 70, [
        "Quickplay / online: nothing",
        "no plugin slot, rate 60, code 0",
    ], col=m.P1)


def main():
    c = rd.DCanvas(W, H)
    third = (W - 24 - 24) / 3
    xs = [12 + i * (third + 12) for i in range(3)]
    editor(c, xs[0], 12, xs[0] + third, H - 12)
    popup(c, xs[1], 12, xs[1] + third, H - 12)
    writes(c, xs[2], 12, xs[2] + third, H - 12)
    OUT.parent.mkdir(parents=True, exist_ok=True)
    c.img.convert("RGB").save(OUT, optimize=True)
    print(OUT)


if __name__ == "__main__":
    main()
