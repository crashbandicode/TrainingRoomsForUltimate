"""Package-2 route on the Ally, from the Surface: mute, load, title -> main menu -> Training stage select -> Final Destination
(tile 4) -> character select (P1 Steve, tile 78, the CPU a random fighter) -> Training. The route is the one walked by hand on
2026-11-02 (s2-01, s5-01); every wait is gated on the tile cursor's log (`ssbu-tile-cursor/events.log`) or the plugin's own lines, with a
cap, and a miss stops the route (it never presses on onto a screen it has not seen).

    ally_route.py <phase>            e.g. live-s2-02 / live-s5-02 (Eden already launched with the stream's adapter, ally_launch.py)
"""
import json
import sys
import time

from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ally_conf as conf
import ally_ops as ao

TILE = "ssbu-tile-cursor/events.log"
DIAG = "TrainingRooms/practice-diagnostics.log"


def run(phase, items, timeout=300):
    out = ao.ops(phase, items, timeout)
    try:
        return json.loads(out[out.index("["):])
    except ValueError:
        raise SystemExit(f"ops failed: {out[-600:]}")


def count(phase, rel, regex):
    return run(phase, [["plug", rel, regex, 1]])[0]["count"]


def wait_count(phase, rel, regex, at_least, cap, what):
    end = time.time() + cap
    while time.time() < end:
        if count(phase, rel, regex) >= at_least:
            print(f"GATE ok {what}", flush=True)
            return
        time.sleep(2)
    print(f"GATE FAIL {what}: {regex!r} x{at_least} in {rel}", flush=True)
    raise SystemExit(2)


def shot_png(phase, name):
    """One frame of the game through Eden's recorder (renderer read-back; the desktop does not matter)."""
    from PIL import Image
    run(phase, [["gshot", name, 2, 960, 700]])
    gif = ao.get(phase, f"shots/{name}.gif", f"{conf.local(conf.stream(phase))}/shots")
    image = Image.open(gif)
    image.seek(image.n_frames - 1)
    return image.convert("RGB")


def close_to(image, ref_name, box, limit):
    from PIL import Image, ImageChops, ImageStat
    ref = Image.open(Path(__file__).resolve().parent.parent / "refs" / ref_name).convert("RGB")
    a, b = image.crop(box), ref.crop(box)
    return ImageStat.Stat(ImageChops.difference(a, b).convert("L")).mean[0] <= limit


def to_main_menu(phase):
    """Title -> main menu: A until the main menu shows (a screen gate), with a cap."""
    for attempt in range(8):
        image = shot_png(phase, f"menu-check{attempt}")
        if close_to(image, "ally-main-menu.png", (0, 0, 600, 540), 25):
            print("GATE ok main menu", flush=True)
            return
        run(phase, [["button", "a", 150, 3]])
    print("GATE FAIL main menu", flush=True)
    raise SystemExit(2)


def main():
    phase = sys.argv[1]
    if "from-menu" not in sys.argv:
        run(phase, [["mute"], ["load"]], 200)
        run(phase, [["waitstatus", "loading_complete", True, 180]], 260)
        wait_count(phase, "TrainingRooms/events.log", "result=installed", 1, 30, "install line")
        run(phase, [["sleep", 4]])
        to_main_menu(phase)
    # Title -> main menu: A twice (the title screen needs a moment before it takes input).
    run(phase, [["macro", "tr-main-to-stage-select", 120], ["sleep", 2]], 240)
    wait_count(phase, TILE, "screen=sss", 1, 20, "stage select")
    run(phase, [["hold", ["lstick", "right"], 150, 0.6]] * 4 + [["sleep", 1]])
    wait_count(phase, TILE, "settled screen=sss at=4 ", 1, 10, "Final Destination (tile 4)")
    run(phase, [["button", "a", 150, 1]])
    wait_count(phase, TILE, "screen=css", 1, 40, "character select")
    run(phase, [["sleep", 2], ["hold", ["lstick", "up"], 150, 1.0], ["button", "a", 150, 1.5]])
    wait_count(phase, TILE, "settled screen=css at=78 ", 1, 10, "Steve (tile 78)")
    run(phase, [["button", "a", 150, 1]])
    # Package 5 gated on the hook counts, package 2 on the CPU body's role line: either shows the match runs.
    wait_count(phase, DIAG, "cpu_event role entry=1|cpu_hook_counts frames=", 1, 120, "Training: the match runs (CPU role or hook counts)")
    print("ROUTE DONE", flush=True)


if __name__ == "__main__":
    main()
