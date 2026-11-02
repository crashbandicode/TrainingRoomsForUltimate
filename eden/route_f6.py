"""F6 route (copy of route_s6.py with the f6 harness), run ON the host that runs the instance (the Surface): boot -> title -> main menu -> Training stage
select -> a stage by tile (default Final Destination) -> CSS (P1 by tile steps) -> Training. Every wait is gated on a log
line or a screenshot match against a reference screenshot of the same screen (`eden/refs/`, from the package-1 runs), with
a cap; on a miss it stops and leaves the last screenshot (it never presses on onto a screen it has not seen).

Usage: route_s6.py <label> [--stage-steps right:6] [--tile-gate 6] [--p1-steps up] [--cpu-steps ...]
Also importable (`import route_s6 as rt`) for the row scripts: `rt.press`, `rt.stick`, `rt.shot`, `rt.macro`, `rt.events`.
"""
import argparse
import json
import sys
import time
from pathlib import Path

from PIL import Image, ImageChops, ImageStat

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import f6_eden as se  # noqa: E402

REFS = HERE / "refs"
SD = se.PROFILE / "user/sdmc/ultimate"
EVENTS = SD / "TrainingRooms/events.log"
DIAG = SD / "TrainingRooms/practice-diagnostics.log"
TILE_LOG = SD / "ssbu-tile-cursor/events.log"
MACROS = HERE / "macros"


def press(label, buttons, hold=150):
    for b in buttons.split(","):
        se.client_call(label, "input.button", {"button": b, "state": "press", "lease_ms": hold + 500})
    time.sleep(hold / 1000)
    for b in buttons.split(","):
        se.client_call(label, "input.button", {"button": b, "state": "release"})


def stick(label, x, y, ms):
    se.client_call(label, "input.stick", {"stick": "left", "x": x, "y": y, "lease_ms": ms})
    time.sleep(ms / 1000)


def shot(label, name):
    return se.shot(label, name)


def lines(path, after=0):
    try:
        return path.read_text(encoding="utf-8", errors="replace").splitlines()[after:]
    except FileNotFoundError:
        return []


def wait_line(path, needle, cap, what, after=0):
    deadline = time.time() + cap
    while time.time() < deadline:
        for line in lines(path, after):
            if needle in line:
                print(f"GATE ok {what}: {line[:170]}", flush=True)
                return line
        time.sleep(1)
    print(f"GATE FAIL {what}: no line with {needle!r}", flush=True)
    raise SystemExit(2)


def dist(path, ref, box):
    a = Image.open(path).convert("L").crop(box)
    b = Image.open(REFS / ref).convert("L").crop(box)
    return ImageStat.Stat(ImageChops.difference(a, b)).mean[0]


def wait_match(label, ref, box, thr, cap, what):
    deadline = time.time() + cap
    best = 999.0
    while time.time() < deadline:
        try:
            p = shot(label, what.replace(" ", "-"))
        except (SystemExit, Exception):  # the first shot after a load can find no render window yet
            time.sleep(1.5)
            continue
        d = dist(p, ref, box)
        best = min(best, d)
        if d <= thr:
            print(f"GATE ok {what} dist={d:.1f} (thr {thr}) {p.name}", flush=True)
            return p
        time.sleep(1.0)
    print(f"GATE FAIL {what} best_dist={best:.1f} thr={thr}", flush=True)
    raise SystemExit(2)


def macro(label, path, cap=120):
    """Play a macro file and wait for it to finish (frames count displayed frames, not wall time)."""
    se.client_call(label, "macro.play", {"path": str(path)})
    deadline = time.time() + cap
    while time.time() < deadline:
        if str(se.client_call(label, "macro.status").get("state")) not in ("playing", "starting", "running"):
            break
        time.sleep(0.5)
    se.client_call(label, "input.reset")


def to_training(label, stage_steps, tile_gate, p1_steps):
    # The integrated tree's install line (8,142 characters in f6-01) is cut before `result=`: gate on its start; the full line is
    # read after the run (reported to the lead as the known install-line length defect).
    wait_line(EVENTS, "plugin=training_rooms/", 240, "install line")
    menu_box = (40, 60, 620, 380)
    for attempt in range(25):  # f6-04: the title stayed black past 10 tries (30 s) once
        press(label, "a")
        time.sleep(3)
        try:
            p = shot(label, "after-a")
        except (SystemExit, Exception):
            time.sleep(3)
            continue
        d = dist(p, "menu.png", menu_box)
        print(f"after A#{attempt + 1}: menu dist {d:.1f}", flush=True)
        if d <= 30:
            print("GATE ok main menu", flush=True)
            break
    else:
        raise SystemExit("main menu never matched")
    time.sleep(1.5)
    macro(label, MACROS / "tr-main-to-stage-select.edenmacro.json", 90)
    wait_match(label, "sss-t0.png", (440, 50, 1240, 660), 14, 40, "stage select")
    after = len(lines(TILE_LOG))
    for step in stage_steps:
        press(label, f"lstick,{step}")
        time.sleep(0.5)
    if tile_gate is not None:
        wait_line(TILE_LOG, f"settled screen=sss at={tile_gate}", 10, f"tile {tile_gate}", after=after)
    shot(label, "stage-chosen")
    press(label, "a")
    wait_match(label, "css.png", (55, 50, 1225, 420), 12, 40, "character select")
    for step in p1_steps:
        press(label, f"lstick,{step}")
        time.sleep(0.6)
    time.sleep(1)
    press(label, "a")
    wait_match(label, "ready.png", (40, 40, 1240, 330), 25, 25, "ready to fight")
    press(label, "a")
    # The plugin's own line, not the reference screenshot: profile g's stage tile 6 is not the reference's stage (f6-01 failed this
    # gate inside a valid Training scene on Kongo Jungle). The screenshot stays as evidence.
    wait_line(DIAG, "[CharacterRoster] observed owner=0", 90, "training scene (P1 entry observed)")
    time.sleep(2)
    shot(label, "training-scene")
    print("ROUTE DONE", flush=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("label", type=se.label_ok)
    parser.add_argument("--stage-steps", default="right:6", help="direction:count[,direction:count] from the first tile")
    parser.add_argument("--tile-gate", type=int, default=6)
    parser.add_argument("--p1-steps", default="up", help="comma separated directions after the first L3 step")
    args = parser.parse_args()
    steps = []
    for part in args.stage_steps.split(","):
        d, n = part.split(":")
        steps += [d] * int(n)
    to_training(args.label, steps, args.tile_gate, [s for s in args.p1_steps.split(",") if s])


if __name__ == "__main__":
    main()
