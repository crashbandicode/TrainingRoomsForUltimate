"""One F6 Eden boot on the Surface, end to end: up (claim, muted, settings in the copy) -> route to Training -> P1 (and CPU) laps
-> collect -> console.log scan -> down (release) -> restore the profile's plugin set. Down and restore run also after a failure.

Usage: f6_boot.py <label> --nro <diag nro> [--p1 N] [--cpu N] [--settings eden/settings/f6-boot.json] [--p1-steps up]
"""
import argparse
import json
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import f6_eden as se  # noqa: E402

PILOT = "/home/intpa/ssbu-cpu-pilot-install/switch-mtp-v0101-v021/pilot/after/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libssbu_cpu_pilot.nro"
TILE = "/home/intpa/trainingrooms-eden-pkg2/nro/aid/libssbu_tile_cursor.nro"


def py(*args):
    print("+", " ".join(args), flush=True)
    return subprocess.run(["/usr/bin/python3", "-I", *args], cwd=HERE).returncode


def main():
    p = argparse.ArgumentParser()
    p.add_argument("label", type=se.label_ok)
    p.add_argument("--nro", required=True)
    p.add_argument("--settings", default=str(HERE / "settings/f6-boot.json"))
    p.add_argument("--p1", type=int, default=8)
    p.add_argument("--cpu", type=int, default=4)
    p.add_argument("--p1-steps", default="up")
    a = p.parse_args()
    up = argparse.Namespace(label=a.label, nro=[f"{a.nro}:libtraining_rooms.nro", PILOT, TILE], settings=a.settings, diag_txt=None)
    started = time.time()
    se.up(up)  # claims; on its own failure it runs down itself and raises
    code = 1
    try:
        if py("route_f6.py", a.label, "--p1-steps", a.p1_steps) != 0:
            raise SystemExit("route failed")
        code = py("f6_rows.py", "laps", a.label, "--p1", str(a.p1), "--cpu", str(a.cpu))
    finally:
        try:
            se.collect(argparse.Namespace(label=a.label))
            se.scan(argparse.Namespace(label=a.label))
        finally:
            se.down(argparse.Namespace(label=a.label))
            try:
                se.restore(argparse.Namespace())
            except Exception as exc:  # reported, never hides the run's result
                print("restore failed:", exc, flush=True)
    print(json.dumps({"label": a.label, "laps_exit": code, "minutes": round((time.time() - started) / 60, 1)}), flush=True)
    sys.exit(code)


if __name__ == "__main__":
    main()
