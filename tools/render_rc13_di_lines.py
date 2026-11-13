#!/usr/bin/python3 -I
"""Draw docs/decisions/rc13-DI-lines.png for docs/DECISIONS.md ("Open (2026-11-13)", rc13-DI-lines).

The picture is author-2's mockup (artifacts/rc13-di-lines/MOCKUP.png in the workspace, drawn by its mockup.py over
the frame of docs/media/rc11-di.gif) with the recommended option, panel D (fight-like) and its close-up, outlined in
green in tools/render_decisions.py's style, and a line with the recommendation under it.

usage: /usr/bin/python3 -I tools/render_rc13_di_lines.py [MOCKUP.png]
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import render_decisions as d  # noqa: E402
from PIL import Image  # noqa: E402

DEFAULT = Path("/home/intpa/smash-training-lobby/artifacts/rc13-di-lines/MOCKUP.png")
# Boxes measured on the 1852 x 1288 mockup: panel D (title bar and chip grid) and D's close-up at the bottom.
PANEL_D = (6, 548, 618, 1026)
CLOSE_D = (1036, 1058, 1379, 1250)
FOOT = 44


def main():
    src = Image.open(sys.argv[1] if len(sys.argv) > 1 else DEFAULT).convert("RGBA")
    assert src.size == (1852, 1288), src.size
    w, h = src.size
    c = d.DCanvas(w, h + FOOT)
    c.img.paste(src, (0, 0))
    for (x0, y0, x1, y1), tag in ((PANEL_D, True), (CLOSE_D, False)):
        c.rect(x0, y0, x1, y1, outline=d.OURS, width=3, radius=8)
        if tag:
            tw = c.textlength("recommended", "b") + 16
            c.rect(x1 - 12 - tw, y0 + 13, x1 - 12, y0 + 35, fill=d.OURS, radius=5)
            c.text(x1 - 12 - tw / 2, y0 + 16, "recommended", "b", d.DARK, "m")
    c.text(12, h + 12, "Recommended: D, if the launch-trail texture is resident in Training, with brighter "
           "unselected chips (half tint -> about 70 %); otherwise C.", "s", d.OURS)
    c.save("rc13-DI-lines")
    print("drew rc13-DI-lines")


if __name__ == "__main__":
    main()
