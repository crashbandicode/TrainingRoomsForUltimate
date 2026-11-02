"""Host-side analysis of probe.py output (run on the Surface, PIL only).

Usage: analyze_probe.py <probe dir holding <name>.json and the pngs> <name> [threshold]
For every trial: mean absolute grey difference over Steve's region (x 100-420, y 120-480 of the 1280x674
render shot) between the paused frame and (a) the baseline shot taken before the macro started and
(b) the first trial's paused frame (macro frame == the first target). The onset is the first macro frame
whose difference to the baseline exceeds the threshold, provided every later frame stays above it.
"""
import json
import sys
from pathlib import Path

from PIL import Image, ImageChops, ImageStat

BOX = (100, 120, 420, 480)


def grey(path):
    return Image.open(path).convert("L").crop(BOX)


def diff(a, b):
    return ImageStat.Stat(ImageChops.difference(a, b)).mean[0]


def main():
    d = Path(sys.argv[1])
    name = sys.argv[2]
    threshold = float(sys.argv[3]) if len(sys.argv) > 3 else 6.5
    rows = json.loads((d / f"{name}.json").read_text())
    folder = d / name
    ref = grey(folder / rows[0]["shot"])
    table = []
    for r in rows:
        f = grey(folder / r["shot"])
        b = grey(folder / r["base"])
        table.append((r["frame_paused"], diff(f, b), diff(f, ref)))
    print("macro_frame d_to_baseline d_to_first")
    for frame, db, dr in table:
        print(f"{frame:4d} {db:6.2f} {dr:6.2f}")
    onset = None
    for i, (frame, db, _) in enumerate(table):
        if db > threshold and all(x[1] > threshold for x in table[i:]):
            onset = frame
            break
    quiet = [db for frame, db, _ in table if onset is None or frame < onset]
    print(f"threshold={threshold} onset_frame={onset} max_noise_before_onset={max(quiet):.2f}")


if __name__ == "__main__":
    main()
