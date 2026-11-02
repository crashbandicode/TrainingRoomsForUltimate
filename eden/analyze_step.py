"""Host-side analysis of step_probe.py output (Surface, PIL only).

Usage: analyze_step.py <dir with <name>.json and <name>/f<frame>.png> <name> [threshold] [--box x0,y0,x1,y1]
For every paused macro frame: mean absolute grey difference over the box between that frame and the
first frame (idle, before the macro's press at frame 60), and between it and the previous frame. The
onset is the first frame whose difference to the first frame exceeds the threshold and stays above it.

Host test: /usr/bin/python3 -I tests/eden/test_analyze.py   (also run by `cargo test`)
"""
import json
import sys
from pathlib import Path

from PIL import Image, ImageChops, ImageStat

BOX = (100, 120, 420, 480)


def table_for(d, name, box=BOX):
    """[(macro frame, difference to the first frame, difference to the previous frame)]."""
    d = Path(d)
    rows = json.loads((d / f"{name}.json").read_text())

    def img(r):
        return Image.open(d / name / r["shot"]).convert("L").crop(box)

    first = img(rows[0])
    prev = first
    table = []
    for r in rows:
        cur = img(r)
        table.append((r["frame_paused"], ImageStat.Stat(ImageChops.difference(cur, first)).mean[0],
                      ImageStat.Stat(ImageChops.difference(cur, prev)).mean[0]))
        prev = cur
    return table


def find_onset(table, thr):
    return next((f for i, (f, a, _) in enumerate(table) if a > thr and all(x[1] > thr for x in table[i:])), None)


def main():
    d = Path(sys.argv[1])
    name = sys.argv[2]
    thr = float(sys.argv[3]) if len(sys.argv) > 3 and not sys.argv[3].startswith("--") else 6.5
    box = BOX
    if "--box" in sys.argv:
        box = tuple(int(x) for x in sys.argv[sys.argv.index("--box") + 1].split(","))
    table = table_for(d, name, box)
    print("macro_frame d_to_first d_to_prev")
    for f, a, b in table:
        print(f"{f:4d} {a:7.2f} {b:7.2f}")
    onset = find_onset(table, thr)
    quiet = [a for f, a, _ in table if onset is None or f < onset]
    print(f"threshold={thr} box={box} onset_frame={onset} max_before_onset={max(quiet):.2f}")


if __name__ == "__main__":
    main()
