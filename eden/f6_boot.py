"""F6 Eden on the Surface under ONE claim: up (claim, muted, settings in the copy) -> route to Training -> P1 (and CPU) laps ->
collect -> console.log scan; then, with --then, a relaunch with the second NRO (the file set changed) only when the first launch's
first group build of the boot shows `control_64f820` as its largest step (the cause the fix moves to the task worker, read by the
committed tools/f6_split_table.py); -> down (one RELEASED line) -> restore the profile's plugin set. Down and restore run also
after a failure.

Usage: f6_boot.py <label> --nro <diag nro> [--then <label2> --nro2 <fix diag nro> [--settings2 <json>]]
                  [--p1 N] [--cpu N] [--settings eden/settings/f6-boot.json] [--p1-steps up]
"""
import argparse
import json
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "tools"))
import f6_eden as se  # noqa: E402
import f6_split_table as table  # noqa: E402

PILOT = "/home/intpa/ssbu-cpu-pilot-install/switch-mtp-v0101-v021/pilot/after/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libssbu_cpu_pilot.nro"
TILE = "/home/intpa/trainingrooms-eden-pkg2/nro/aid/libssbu_tile_cursor.nro"
HELD = []  # labels launched under our claim (a failed first up() releases the claim itself)


def py(*args):
    print("+", " ".join(args), flush=True)
    return subprocess.run(["/usr/bin/python3", "-I", *args], cwd=HERE).returncode


def one_launch(label, nro, settings, a, claim, resume=False):
    if not resume:
        up = argparse.Namespace(label=label, nro=[f"{nro}:libtraining_rooms.nro", PILOT, TILE], settings=settings, diag_txt=None)
        se.up(up, claim=claim)
    # resume: the launch is already up under our claim (a driver stopped before its route); continue from the route
    HELD.append(label)
    code = 1
    try:
        if py("route_f6.py", label, "--p1-steps", a.p1_steps) != 0:
            print("route failed (precondition): retry under the next label", flush=True)
            return 2
        code = py("f6_rows.py", "laps", label, "--p1", str(a.p1), "--cpu", str(a.cpu))
    finally:
        try:
            se.collect(argparse.Namespace(label=label))
            se.scan(argparse.Namespace(label=label))
        finally:
            se.down(argparse.Namespace(label=label), release=False)
    return code


def first_build_cause(label):
    logs = sorted((HERE / "runs" / label / "TrainingRooms").glob("practice-diagnostics*.log"))
    lines = []
    for p in sorted(logs, key=lambda p: (".previous" not in p.name, p.name)):
        lines += p.read_text(encoding="utf-8", errors="replace").splitlines()
    builds = table.parse(lines)
    print(table.table(builds), flush=True)
    first = next((b for b in builds if b["build_of_boot"] == 1), None)
    return first and first.get("largest")


def main():
    p = argparse.ArgumentParser()
    p.add_argument("label", type=se.label_ok)
    p.add_argument("--nro", required=True)
    p.add_argument("--then", type=se.label_ok)
    p.add_argument("--nro2")
    p.add_argument("--settings", default=str(HERE / "settings/f6-boot.json"))
    p.add_argument("--settings2")
    p.add_argument("--p1", type=int, default=8)
    p.add_argument("--cpu", type=int, default=4)
    p.add_argument("--p1-steps", default="up")
    p.add_argument("--resume", action="store_true", help="the first launch is already up under our claim")
    a = p.parse_args()
    started = time.time()
    result = {"label": a.label}
    try:
        result["laps_exit"] = one_launch(a.label, a.nro, a.settings, a, claim=True, resume=a.resume)
        cause = first_build_cause(a.label)
        result["first_build_largest_step"] = cause
        if a.then and a.nro2:
            if cause == "control_64f820":
                result["then_exit"] = one_launch(a.then, a.nro2, a.settings2 or a.settings, a, claim=False)
                first_build_cause(a.then)
            else:
                result["then"] = f"not run: the first build's largest step is {cause}, not control_64f820"
    finally:
        if HELD:
            se.down(argparse.Namespace(label=HELD[-1]))  # the one RELEASED line
        try:
            se.restore(argparse.Namespace())
        except Exception as exc:  # reported, never hides the run's result
            print("restore failed:", exc, flush=True)
    result["minutes"] = round((time.time() - started) / 60, 1)
    print(json.dumps(result), flush=True)
    sys.exit(0 if result.get("laps_exit") == 0 else 1)


if __name__ == "__main__":
    main()
