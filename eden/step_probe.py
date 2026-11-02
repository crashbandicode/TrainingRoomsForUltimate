"""Frame stepping probe (runs ON the Yoga, imports tr_eden). One macro run, paused and screenshot at
every macro frame in [start, end]: play the macro, then for each frame k poll macro.status until
frame >= k, call game.pause, read the frame the game stopped at (a paused game does not advance the
macro; probe.py showed frame_paused == target on every one of its 26 pauses in boot 1), screenshot the paused
frame, game.resume. Everything lands inside ~35 game frames, so stage hazards cannot move the camera
mid-measurement.

Usage: step_probe.py <label> <name> <macro-path> <start> <end>
Writes ROOT/probes/<label>/<name>.json and ROOT/probes/<label>/<name>/f<frame>.png.
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import tr_eden as t  # noqa: E402


def main():
    label, name, macro = sys.argv[1], sys.argv[2], sys.argv[3]
    # <start> <end> (contiguous) or one comma list of frames
    frames = ([int(x) for x in sys.argv[4].split(",")] if "," in sys.argv[4]
              else list(range(int(sys.argv[4]), int(sys.argv[5]) + 1)))
    out = t.ROOT / "probes" / label / name
    out.mkdir(parents=True, exist_ok=True)
    tree = t.client_call(label, "ui.tree", {"brief": True, "visible_only": True})
    wid = next(n for n in t.nodes(tree) if n["class"] == "GRenderWindow")["id"]
    t.client_call(label, "macro.play", {"path": macro}, timeout=10)
    rows = []
    for k in frames:
        while True:
            st = t.client_call(label, "macro.status", timeout=5)
            if st["frame"] >= k or st["state"] != "playing":
                break
        t.client_call(label, "game.pause", timeout=5)
        reads = []
        for _ in range(10):
            reads.append(t.client_call(label, "macro.status", timeout=5)["frame"])
            if len(reads) >= 3 and reads[-1] == reads[-2] == reads[-3]:
                break
            time.sleep(0.02)
        fp = reads[-1]
        shot = out / f"f{fp:03d}.png"
        t.client_call(label, "ui.screenshot", {"path": str(shot), "target": wid}, timeout=30)
        rows.append({"target": k, "frame_paused": fp, "shot": shot.name, "reads": reads})
        t.client_call(label, "game.resume", timeout=5)
        print(json.dumps(rows[-1]), flush=True)
    t.client_call(label, "macro.stop", timeout=5)
    t.client_call(label, "input.reset", timeout=5)
    (t.ROOT / "probes" / label / f"{name}.json").write_text(json.dumps(rows, indent=1), encoding="utf-8")


main()
