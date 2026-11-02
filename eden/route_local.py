"""Scripted route, driven from the Surface over one multiplexed ssh to the Yoga (the Yoga has no
Pillow): boot -> title -> main menu -> Training stage select -> Pokemon Stadium 2 (tile 41) ->
CSS (Steve) -> Training. Every wait is gated on a log line or a screenshot match against a
reference screenshot of the same screen from boot 1 (tr101), with a cap; on a miss it stops and
leaves the last screenshot (never presses on onto a screen it has not seen).

Usage: route_local.py <label>   (run with /usr/bin/python3 -I eden/route_local.py tr102)
"""
import base64
import json
import os
import subprocess
import sys
import time
from pathlib import Path

from PIL import Image, ImageChops, ImageStat

HERE = Path(__file__).resolve().parent
YOGA = "memento-claw-yoga-ubuntu"
# Package 1's work root and profile; another stream passes its config file (TR_EDEN_CONFIG, e.g.
# `tr_eden.r1fix.config.json`: the same file its `tr_eden.py` is deployed with).
_CFG = json.loads(Path(os.environ["TR_EDEN_CONFIG"]).read_text(encoding="utf-8")) if os.environ.get("TR_EDEN_CONFIG") else {}
WORK = _CFG.get("root", "/home/intpa/trainingrooms-eden-p1")
PROFILE_NAME = _CFG.get("profile", "t1")
SSH = ["/usr/bin/ssh", "-o", "BatchMode=yes", "-o", "ControlMaster=auto", "-o", "ControlPath=/tmp/trssh-%C",
       "-o", "ControlPersist=900", YOGA]
REFS = HERE / "refs"
PROFILE_SD = f"{WORK}/profiles/{PROFILE_NAME}/user/sdmc/ultimate"


def remote(command, timeout=120):
    r = subprocess.run([*SSH, command], capture_output=True, text=True, timeout=timeout)
    return r.returncode, r.stdout.strip(), r.stderr.strip()


def tr(args, timeout=120):
    return remote(f"cd {WORK}; python3 -I tr_eden.py {args}", timeout)


def call(label, method, params=None):
    b64 = base64.b64encode(json.dumps(params or {}).encode()).decode()
    rc, out, err = tr(f"call {label} {method} b64:{b64}")
    if rc != 0:
        raise RuntimeError(f"{method} failed: {out} {err}")
    return json.loads(out.splitlines()[-1])


def press(label, buttons, hold=150):
    rc, out, err = tr(f"press {label} {buttons} {hold}")
    if rc != 0:
        raise RuntimeError(f"press {buttons} failed: {out} {err}")


def snap(label, name):
    d = HERE / "runs" / label / "shots"
    d.mkdir(parents=True, exist_ok=True)
    rc, out, err = tr(f"shot {label} {name}")
    if rc != 0:
        raise RuntimeError(f"shot failed: {out} {err}")
    remote_path = out.splitlines()[-1].strip()
    subprocess.run(["/usr/bin/scp", "-q", "-o", "ControlPath=/tmp/trssh-%C", f"{YOGA}:{remote_path}", str(d)], check=True)
    return d / Path(remote_path).name


def dist(path, ref, box):
    a = Image.open(path).convert("L").crop(box)
    b = Image.open(REFS / ref).convert("L").crop(box)
    return ImageStat.Stat(ImageChops.difference(a, b)).mean[0]


def wait_match(label, ref, box, thr, cap, what):
    deadline = time.time() + cap
    best = 999.0
    while time.time() < deadline:
        try:
            p = snap(label, what.replace(" ", "-"))
        except (RuntimeError, subprocess.CalledProcessError):
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


def wait_line(path, needle, cap, what, after=0):
    deadline = time.time() + cap
    while time.time() < deadline:
        rc, out, _ = remote(f"if (Test-Path {path}) {{ Get-Content {path} }}")
        for line in out.splitlines()[after:]:
            if needle in line:
                print(f"GATE ok {what}: {line[:170]}", flush=True)
                return line
        time.sleep(2)
    print(f"GATE FAIL {what}: no line with {needle!r}", flush=True)
    raise SystemExit(2)


def play_segment(label, path, cap=90):
    call(label, "macro.play", {"path": path})
    deadline = time.time() + cap
    while time.time() < deadline:
        if call(label, "macro.status")["state"] != "playing":
            break
        time.sleep(0.6)
    call(label, "input.reset")


def main():
    label = sys.argv[1]
    events = f"{PROFILE_SD}/TrainingRooms/events.log"
    tile_log = f"{PROFILE_SD}/ssbu-tile-cursor/events.log"
    wait_line(events, "result=installed", 240, "install line")
    menu_box = (40, 60, 620, 380)
    for attempt in range(10):
        press(label, "a")
        time.sleep(3)
        try:
            p = snap(label, "after-a")  # the first shot after a load can find no render window yet
        except (RuntimeError, subprocess.CalledProcessError):
            print(f"after A#{attempt + 1}: no render window yet", flush=True)
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
    play_segment(label, f"{WORK}/inbox/tr-main-to-stage-select.edenmacro.json")
    wait_match(label, "sss-t0.png", (440, 50, 1240, 660), 14, 40, "stage select")
    rc, out, _ = remote(f"if (Test-Path {tile_log}) {{ (Get-Content {tile_log}).Count }} else {{ 0 }}")
    after = int(out or 0)
    for d in ["down"] * 3 + ["right"] * 8:
        press(label, f"lstick,{d}")
        time.sleep(0.5)
    wait_line(tile_log, "settled screen=sss at=41", 10, "tile 41", after=after)
    wait_match(label, "sss-t41.png", (45, 205, 435, 515), 8, 15, "stage name Pokemon Stadium 2")
    press(label, "a")
    wait_match(label, "css.png", (55, 50, 1225, 420), 12, 40, "character select")
    press(label, "lstick,up")
    time.sleep(1)
    press(label, "a")
    wait_match(label, "ready.png", (40, 40, 1240, 330), 25, 25, "ready to fight")
    press(label, "a")
    wait_match(label, "training.png", (100, 200, 430, 500), 30, 90, "training scene")
    print("ROUTE DONE", flush=True)


if __name__ == "__main__":
    main()
