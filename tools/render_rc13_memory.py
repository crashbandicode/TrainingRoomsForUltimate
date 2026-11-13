#!/usr/bin/python3 -I
"""Draw docs/decisions/rc13-memory.png for docs/DECISIONS.md ("Open (2026-11-13)", rc13-memory).

The rows are read from worker B-2's charts (artifacts/rc13-memory/01-baseline.svg and 03-levers.svg in the workspace):
each row's label, its bar widths in the SVG's scale, and its value text. They are redrawn here in
tools/render_decisions.py's look, with the recommended levers (L4, L5, L6) outlined in green.

usage: /usr/bin/python3 -I tools/render_rc13_memory.py [folder with the two SVGs]
"""
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import render_decisions as d  # noqa: E402

DEFAULT = Path("/home/intpa/smash-training-lobby/artifacts/rc13-memory")
NS = "{http://www.w3.org/2000/svg}"
FG, MUTED, OURS, ADV, LINE, TILE, TILE_HI = d.FG, d.MUTED, d.OURS, d.ADV, d.LINE, d.TILE, d.TILE_HI
W = d.WIDE
BUDGET = 3285  # MiB, 01-baseline.svg's title
REC = {"L4", "L5", "L6"}
OFFERED = {"L1", "L2", "L3"}


def rows(svg):
    """[(label, [rect widths], value text, last rect title)] in the SVG's order; a row starts at a right-anchored
    label."""
    out = []
    for el in ET.parse(svg).getroot():
        if el.tag == NS + "text" and el.get("text-anchor") == "end":
            out.append([el.text, [], None, None])
        elif out and el.tag == NS + "rect":
            out[-1][1].append(float(el.get("width")))
            title = el.find(NS + "title")
            out[-1][3] = title.text if title is not None else None
        elif out and el.tag == NS + "text" and el.get("fill") != "white" and out[-1][2] is None:
            out[-1][2] = el.text
    return out


def main():
    src = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT
    base, levers = rows(src / "01-baseline.svg"), rows(src / "03-levers.svg")
    assert len(base) == 6 and len(levers) == 7, (len(base), len(levers))
    c = d.DCanvas(W, 800)
    x0, x1 = 420, W - 270

    # Baseline: used (game and plugins) and free, of the 3,285 MiB budget.
    y = 12
    c.rect(12, y, W - 12, y + 320, fill=d.PANEL, outline=LINE, radius=8)
    c.text(26, y + 10, "Game process memory by scene", "h", FG)
    c.text(26 + c.textlength("Game process memory by scene", "h") + 10, y + 14,
           f"Eden, rc.12 candidate diag; MiB of {BUDGET:,} (01-baseline.svg)", "xs", MUTED)
    scale = (x1 - x0) / sum(base[0][1])  # every row's rects add up to the whole budget
    yy = y + 50
    for label, widths, value, free in base:
        used = sum(widths[:-1]) * scale
        c.text(x0 - 10, yy + 5, label, "xs", FG, "r")
        c.rect(x0, yy, x0 + used, yy + 26, fill=(110, 120, 140), radius=2)
        c.rect(x0 + used, yy, x1, yy + 26, fill=TILE_HI, outline=LINE, radius=2)
        c.text(x1 + 10, yy + 5, value + ", " + free, "xs", FG)
        yy += 36
    d.lines(c, 26, yy + 4, [
        "Grey: used (range over two boots); dark: free. The game alone uses about 2.1 GiB at the menu and about "
        "2,450 MiB in Training,",
        "so a whole-process 1 GiB isn't reachable. TrainingRooms' share: 398 MiB (card), 476 (defaults), about 582 "
        "(cycle-lap peak)."], font="xxs", step=17)

    # Levers: the outlined bar is the high end, the solid part the low end.
    y = 344
    c.rect(12, y, W - 12, y + 444, fill=d.PANEL, outline=LINE, radius=8)
    c.text(26, y + 10, "Levers: estimated saving", "h", FG)
    c.text(26 + c.textlength("Levers: estimated saving", "h") + 10, y + 14,
           "MiB; solid = low end, outlined = up to the high end (03-levers.svg)", "xs", MUTED)
    hi = max(sum(w) for _, w, _, _ in levers)
    lx0, lx1 = 420, W - 200
    yy = y + 54
    for label, widths, value, _ in levers:
        key = label.split()[0]
        lo_w, hi_w = widths[0] / hi * (lx1 - lx0), sum(widths) / hi * (lx1 - lx0)
        if key in REC:
            c.rect(20, yy - 6, W - 20, yy + 34, fill=TILE, outline=OURS, width=3, radius=6)
        c.text(lx0 - 10, yy + 5, label, "xs", MUTED if key == "L7" else FG, "r")
        col = OURS if key in REC else ADV if key in OFFERED else LINE
        c.rect(lx0, yy, lx0 + max(hi_w, 2), yy + 26, fill=TILE_HI, outline=col, width=1, radius=2)
        if lo_w:
            c.rect(lx0, yy, lx0 + lo_w, yy + 26, fill=col, radius=2)
        c.text(lx0 + max(hi_w, 2) + 8, yy + 5, value, "xs", FG)
        if key in REC or key in OFFERED:
            d.tag(c, W - 140 if key in REC else W - 112, yy + 3, "recommended" if key in REC else "offered",
                  OURS if key in REC else ADV)
        yy += 46
    d.lines(c, 26, yy + 2, [
        ("Recommended: L5 + L6 as the core (peak safety for the 11-13 abort; the settled figure and every default "
         "unchanged),", OURS),
        ("plus L4 (hats and helper seats on first use). L1-L3 are offered: they trade swap speed for memory.", OURS),
        "No lever drops a feature. L7 is too small to matter.",
    ], font="xs", step=20)
    c.save("rc13-memory")
    print("drew rc13-memory")


if __name__ == "__main__":
    main()
