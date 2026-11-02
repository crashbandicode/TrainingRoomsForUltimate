"""Frame-counted probe (runs ON the Yoga, imports tr_eden): play an input macro, pause the game at a
macro frame >= target, read the macro frame the game actually stopped at (macro.status `frame`, exact
because a paused game does not advance the macro) and take a screenshot of the paused frame.

Usage: probe.py <label> <name> <macro-path> <target,target,...> [--settle s]
Writes ROOT/probes/<label>/<name>.json and ROOT/probes/<label>/<name>/*.png.
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, "/home/intpa/trainingrooms-eden-p1")
import tr_eden as t  # noqa: E402


def status(label):
    return t.client_call(label, "macro.status", timeout=5)


def main():
    label, name, macro, targets = sys.argv[1], sys.argv[2], sys.argv[3], [int(x) for x in sys.argv[4].split(",")]
    settle = float(sys.argv[sys.argv.index("--settle") + 1]) if "--settle" in sys.argv else 2.5
    out = t.ROOT / "probes" / label / name
    out.mkdir(parents=True, exist_ok=True)
    results = []
    for i, target in enumerate(targets):
        trial = {"target": target, "i": i}
        base = out / f"{i:02d}-base.png"
        t.client_call(label, "ui.screenshot", {"path": str(base), "target": nodes_id(label)}, timeout=30)
        trial["base"] = base.name
        t0 = time.time()
        t.client_call(label, "macro.play", {"path": macro}, timeout=10)
        paused_call = None
        while True:
            st = status(label)
            if st["frame"] >= target or st["state"] != "playing":
                trial["frame_at_decision"] = st["frame"]
                trial["state_at_decision"] = st["state"]
                t1 = time.time()
                t.client_call(label, "game.pause", timeout=5)
                paused_call = time.time() - t1
                break
        frames = []
        for _ in range(12):
            st = status(label)
            frames.append(st["frame"])
            if len(frames) >= 3 and frames[-1] == frames[-2] == frames[-3]:
                break
            time.sleep(0.03)
        trial["frame_reads_after_pause"] = frames
        trial["frame_paused"] = frames[-1]
        trial["pause_call_s"] = round(paused_call, 4)
        trial["wall_to_pause_s"] = round(time.time() - t0, 3)
        shot = out / f"{i:02d}-f{trial['frame_paused']:03d}.png"
        t.client_call(label, "ui.screenshot", {"path": str(shot), "target": nodes_id(label)}, timeout=30)
        trial["shot"] = shot.name
        st = status(label)
        trial["frame_after_shot"] = st["frame"]
        if "--fast" in sys.argv:
            t.client_call(label, "macro.stop", timeout=5)
            t.client_call(label, "game.resume", timeout=5)
        else:
            t.client_call(label, "game.resume", timeout=5)
            deadline = time.time() + 20
            while time.time() < deadline and status(label)["state"] == "playing":
                time.sleep(0.2)
        t.client_call(label, "input.reset", timeout=5)
        results.append(trial)
        print(json.dumps(trial), flush=True)
        time.sleep(settle)
    (t.ROOT / "probes" / label / f"{name}.json").write_text(json.dumps(results, indent=1), encoding="utf-8")


_ID = {}


def nodes_id(label):
    if label in _ID:
        return _ID[label]
    tree = t.client_call(label, "ui.tree", {"brief": True, "visible_only": True})
    _ID[label] = next(n for n in t.nodes(tree) if n["class"] == "GRenderWindow")["id"]
    return _ID[label]


main()
