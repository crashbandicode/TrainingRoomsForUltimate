"""Write the package-2 P1 macros (`eden/macros/tr2-*.edenmacro.json`), in the format of the package-1 macros
(`eden/mkmacros.py`): sparse samples (`frame`, `buttons` bit mask, `sticks` lx ly rx ry), `duration_frames` the length.
Buttons: A=1, B=2, X=4, Y=8, L3=16, R3=32, L=64, R=128, ZL=256, ZR=512.

Usage: mkmacros_s2.py <base macro json> [outdir]
"""
import copy
import json
import sys
from pathlib import Path

A, B, X, Y, L, R, ZL, ZR = 1, 2, 4, 8, 64, 128, 256, 512


def ev(frame, buttons=0, sticks=(0, 0, 0, 0)):
    return {"buttons": buttons, "frame": frame, "motion": [{"accel": [0, 0, 0], "delta_us": 10000, "gyro": [0, 0, 0]}] * 2,
            "sticks": list(sticks), "triggers": [0, 0]}


def build():
    m = {}
    # Walk right for n frames (stick 0.6: a walk, not a dash), then rest.
    for n in (20, 40, 80):
        m[f"tr2-walk-right-{n}"] = ([ev(0), ev(5, 0, (0.6, 0, 0, 0)), ev(5 + n, 0, (0, 0, 0, 0))], 5 + n + 20)
    # Jabs: A tapped, released 4 frames later, every 24 frames.
    m["tr2-jab-x4"] = ([ev(0)] + [e for k in range(4) for e in (ev(10 + 24 * k, A), ev(14 + 24 * k, 0))], 130)
    # Tilts and smashes with the stick held through the press.
    m["tr2-ftilt"] = ([ev(0), ev(10, A, (0.4, 0, 0, 0)), ev(16, 0, (0, 0, 0, 0))], 80)
    m["tr2-utilt"] = ([ev(0), ev(10, A, (0, 0.4, 0, 0)), ev(16, 0, (0, 0, 0, 0))], 80)
    m["tr2-fsmash"] = ([ev(0), ev(10, 0, (1.0, 0, 0, 0)), ev(11, A, (1.0, 0, 0, 0)), ev(20, 0, (0, 0, 0, 0))], 110)
    m["tr2-usmash"] = ([ev(0), ev(10, 0, (0, 1.0, 0, 0)), ev(11, A, (0, 1.0, 0, 0)), ev(20, 0, (0, 0, 0, 0))], 110)
    # Grab: R is Grab on the default layout; hold it a few frames. Pummel: A taps while held.
    m["tr2-grab"] = ([ev(0), ev(10, R), ev(14, 0)], 90)
    m["tr2-grab-pummel-x3"] = ([ev(0), ev(10, R), ev(14, 0)] + [e for k in range(3) for e in (ev(50 + 14 * k, A), ev(52 + 14 * k, 0))], 140)
    # Shield and a jump (X) for the aerial rows.
    m["tr2-shield-hold-60"] = ([ev(0), ev(5, ZL), ev(65, 0)], 90)
    m["tr2-jump"] = ([ev(0), ev(5, X), ev(8, 0)], 70)
    # Attack from the air: jump then A.
    m["tr2-jump-nair"] = ([ev(0), ev(5, X), ev(8, 0), ev(28, A), ev(32, 0)], 90)
    m["tr2-idle-120"] = ([ev(0)], 120)
    return m


def main():
    base = json.load(open(sys.argv[1]))
    out = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(__file__).resolve().parent / "macros"
    out.mkdir(parents=True, exist_ok=True)
    for name, (events, duration) in build().items():
        d = {k: copy.deepcopy(base[k]) for k in ("bindings", "format", "game_id", "parent", "player", "version")}
        d.update(name=name, events=events, markers=[], duration_frames=duration)
        frames = [e["frame"] for e in events]
        assert frames == sorted(set(frames)), name
        (out / f"{name}.edenmacro.json").write_text(json.dumps(d, indent=1), encoding="utf-8")
        print(name)


if __name__ == "__main__":
    main()
