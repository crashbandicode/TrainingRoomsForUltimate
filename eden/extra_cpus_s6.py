"""Row: native Training with an extra CPU (No. of CPUs 2, CPU Behavior Walk, set in the native pause menu before this runs): a
save/load must leave the extra CPU unfrozen (it keeps walking). Evidence: three screenshots after the load, 2 s apart, and the
per-image difference of the area right of P1 (the extra CPU walks; a frozen one leaves it identical). Usage: extra_cpus_s6.py <label>"""
import sys
import time

from PIL import Image, ImageChops, ImageStat

import rows_s6 as r


def diff(a, b):
    box = (300, 150, 1240, 520)
    return ImageStat.Stat(ImageChops.difference(Image.open(a).convert("L").crop(box), Image.open(b).convert("L").crop(box))).mean[0]


def main():
    r.tap("plus", wait=2.0)  # close the native pause menu
    time.sleep(3)
    saved = r.save_state()
    time.sleep(3)
    m, done = r.load_state(cap=20)
    time.sleep(1.0)
    shots = []
    for i in range(3):
        shots.append(r.shot(f"extra-cpu-after-load-{i}"))
        time.sleep(2.0)
    moves = [diff(shots[i], shots[i + 1]) for i in range(2)] if all(shots) else []
    r.row("extra CPU: save/load finishes with two CPUs", saved is not None and done is not None, f"{saved} | {done}")
    r.row("extra CPU: unfrozen after the load (the walking CPUs change the frame)", moves and min(moves) > 2.0, f"mean diffs {moves}")
    r.main_write()


if __name__ == "__main__":
    main()
