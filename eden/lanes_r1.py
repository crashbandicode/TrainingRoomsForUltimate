"""Gameplay lanes of the R1-fix Eden rows (review R1 of package 1), run from the Surface after
`route_local.py` has put the game in Training (it drives the Yoga over the same multiplexed ssh):

    TR_EDEN_CONFIG=eden/tr_eden.r1fix.config.json /usr/bin/python3 -I eden/lanes_r1.py <label> delay|lra|hazards|pause|stats

Every lane prints GATE ok/FAIL lines and keeps its screenshots under eden/runs/<label>/. The `pause`
lane is the R2 regression: delay 5, a walk fills the queue, the native pause opens with the stick held,
the pause menu is used, it is closed with the stick released; the diag `pause_owner` lines must show the
queue emptied at the begin, no output while owned, and no leaked record after the resume. The `stats`
lane gates the diag `hook_stats` lines (R5): nonzero coverage for the hooks the lanes exercised and a
zero allocation delta on every line.
"""
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import route_local as r  # noqa: E402
import hook_stats  # noqa: E402

DIAG = f"{r.PROFILE_SD}/TrainingRooms/practice-diagnostics.log"
DIAG_PREVIOUS = f"{r.PROFILE_SD}/TrainingRooms/practice-diagnostics.previous.log"
HERE = Path(__file__).resolve().parent


def diag_text():
    """The previous and the current diagnostics file (the diag build rotates at 128 KiB, which a long run
    reaches): lines are selected by their millisecond stamp, never by their count."""
    rc, out, _ = r.remote(f"foreach ($f in '{DIAG_PREVIOUS}', '{DIAG}') {{ if (Test-Path $f) {{ Get-Content $f }} }}")
    return out


def last_stamp():
    """The newest millisecond stamp in the log (the first token of the line)."""
    stamps = [int(l.split(" ", 1)[0]) for l in diag_text().splitlines() if l.split(" ", 1)[0].isdigit()]
    return max(stamps) if stamps else 0


def lines_after(stamp):
    return [l for l in diag_text().splitlines() if l.split(" ", 1)[0].isdigit() and int(l.split(" ", 1)[0]) > stamp]


def play(label, macro, cap=60):
    """Play a macro from the inbox and wait for it to end."""
    r.play_segment(label, f"{r.WORK}/inbox/{macro}.edenmacro.json", cap)


def lane_delay(label):
    """Delay +5: continuous-run onset of a jump press at macro frame 60 (69 undelayed, 74 delayed)."""
    probes = f"{r.WORK}/probes/{label}"
    targets = "68,70,72,73,74,75,76,78"
    rc, out, err = r.remote(
        f"cd {r.WORK}; python3 -I probe.py {label} jump-scan {r.WORK}/inbox/tr-jump-probe.edenmacro.json {targets} --settle 2.5", 600)
    print(out[-400:], err[-200:])
    local = HERE / "runs" / label / "probes"
    local.mkdir(parents=True, exist_ok=True)
    subprocess.run(["/usr/bin/scp", "-q", "-r", "-o", "ControlPath=/tmp/trssh-%C", f"{r.YOGA}:{probes}/jump-scan", f"{r.YOGA}:{probes}/jump-scan.json", str(local)], check=True)
    res = subprocess.run(["/usr/bin/python3", "-I", str(HERE / "analyze_probe.py"), str(local), "jump-scan", "6.5"], capture_output=True, text=True)
    print(res.stdout[-600:])
    m = re.search(r"onset_frame=(\d+)", res.stdout)
    ok = bool(m) and int(m.group(1)) == 74
    print(("GATE ok" if ok else "GATE FAIL"), f"delay onset {m.group(1) if m else None} (undelayed 69, delay 5 -> 74)")
    return ok


def lane_lra(label):
    """L+R+A: spawn, walk right, chord, back at spawn (the shots are compared by eye and by distance)."""
    a = r.snap(label, "lra0-start")
    play(label, "tr-walk-right")
    time.sleep(0.5)
    b = r.snap(label, "lra1-moved")
    play(label, "tr-lra-chord")
    time.sleep(1.5)
    c = r.snap(label, "lra2-after-chord")
    box = (100, 120, 420, 480)
    from PIL import Image, ImageChops, ImageStat

    def diff(x, y):
        return ImageStat.Stat(ImageChops.difference(Image.open(x).convert("L").crop(box), Image.open(y).convert("L").crop(box))).mean[0]

    d_moved, d_back = diff(a, b), diff(a, c)
    ok = d_moved > 3 * d_back and d_back < 8
    print(("GATE ok" if ok else "GATE FAIL"), f"L+R+A: spawn-vs-moved {d_moved:.1f}, spawn-vs-after-chord {d_back:.1f}")
    return ok


def lane_hazards(label, seconds=75, every=7):
    """Hazards On: the stage changes form within the window (compare shots of the stage with the first one)."""
    from PIL import Image, ImageChops, ImageStat
    shots = []
    end = time.time() + seconds
    while time.time() < end:
        shots.append(r.snap(label, f"haz{len(shots):02d}"))
        time.sleep(every)
    box = (0, 0, 1280, 200)  # the sky and the stage's upper part change with the form
    base = Image.open(shots[0]).convert("L").crop(box)
    diffs = [ImageStat.Stat(ImageChops.difference(Image.open(s).convert("L").crop(box), base)).mean[0] for s in shots]
    ok = max(diffs) > 12
    print(("GATE ok" if ok else "GATE FAIL"), "hazards: max difference of the stage region to the first shot", f"{max(diffs):.1f}", [f"{d:.0f}" for d in diffs])
    return ok


def lane_pause(label):
    """R2: delay 5, walk, pause with the stick held, use the pause menu, resume with the stick released."""
    before = last_stamp()
    r.snap(label, "pause0-before")
    r.call(label, "macro.play", {"path": f"{r.WORK}/inbox/tr-pause-lane.edenmacro.json"})
    shots = {}
    deadline = time.time() + 40
    while time.time() < deadline:
        st = r.call(label, "macro.status")
        frame = st.get("frame", 0)
        if "menu" not in shots and frame >= 200:
            shots["menu"] = r.snap(label, "pause1-menu-open")
        if st.get("state") != "playing":
            break
        time.sleep(0.4)
    r.call(label, "input.reset")
    time.sleep(1.0)
    shots["after"] = r.snap(label, "pause2-after-resume")
    time.sleep(3.0)  # the diag drain thread writes every 50 ms
    lines = [l for l in lines_after(before) if "pause_owner" in l]
    for l in lines:
        print("  ", l)
    begin = [re.search(r"queue_dropped=(\d+) owners=0x([0-9a-f]+)", l) for l in lines if " begin " in l]
    end_ = [re.search(r"queued_at_begin=(\d+) owned_polls=(\d+) owned_non_neutral=(\d+) owners=0x([0-9a-f]+)", l) for l in lines if " end " in l]
    chk = [re.search(r"resume_checked delay=(\d+) leaked=(\d+)", l) for l in lines if "resume_checked" in l]
    ok = (len(begin) == 1 and begin[0] and int(begin[0].group(1)) == 5 and begin[0].group(2) == "10"
          and len(end_) == 1 and end_[0] and int(end_[0].group(1)) == 5 and int(end_[0].group(2)) > 0
          and int(end_[0].group(3)) == 0 and end_[0].group(4) == "00"
          and len(chk) == 1 and chk[0] and chk[0].group(1) == "5" and chk[0].group(2) == "0")
    print(("GATE ok" if ok else "GATE FAIL"), "pause lane: begin(queue 5, owners 0x10), end(queued_at_begin 5, owned polls > 0, none non-neutral, owners 0x00), resume check (delay 5, leaked 0)")
    return ok


def lane_stats(label, required):
    time.sleep(12)  # one 10 s sample after the last lane's hooks
    text = diag_text()
    ok, reasons = hook_stats.gate(text, required)
    rows = hook_stats.parse(text)
    print("hook_stats lines:", len(rows), "last:", json.dumps(rows[-1]) if rows else None)
    for reason in reasons:
        print("  FAIL:", reason)
    print(("GATE ok" if ok else "GATE FAIL"), "hook_stats: nonzero coverage of", required, "and allocation delta 0 on every line")
    return ok


def main():
    label, lane = sys.argv[1], sys.argv[2]
    lanes = {"delay": lane_delay, "lra": lane_lra, "hazards": lane_hazards, "pause": lane_pause,
             "stats": lambda l: lane_stats(l, sys.argv[3].split(",") if len(sys.argv) > 3 else
                                           ["input_caller_a_apply", "reset_check", "reset_hub", "hazard_rule", "pause_enter", "pause_exit"])}
    ok = lanes[lane](label)
    print("LANE", lane, "PASS" if ok else "FAIL")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
