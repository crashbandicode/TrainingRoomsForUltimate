"""Hitbox-stream Eden driver (run from the Surface; the Yoga runs `tr_eden_hb.py`). Thin commands over `route_hb.py`'s helpers so each
row is driven step by step and every wait is gated on a diag line or a screenshot:

    hb_drive.py route <label>                       # boot -> Training (route_hb.main)
    hb_drive.py lines <label> <regex> [after]       # matching lines of the plugin's diag log (and events.log), numbered
    hb_drive.py count <label>                       # number of diag log lines (an `after` mark for the next wait)
    hb_drive.py wait <label> <regex> <cap s> [after] # wait for a diag line, print it
    hb_drive.py shot <label> <name>                 # screenshot -> eden/runs/<label>/shots/
    hb_drive.py hold <label> <buttons> <ms>         # press and hold (comma-separated buttons), then release
    hb_drive.py call <label> <method> [json]        # any control method
    hb_drive.py speed <label> <percent|off>         # emulation speed limit (slow motion for the hitbox frames)
"""
import json
import re
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import route_hb as r  # noqa: E402

DIAG = f"{r.PROFILE_SD}/TrainingRooms/practice-diagnostics.log"
EVENTS = f"{r.PROFILE_SD}/TrainingRooms/events.log"


def log_lines(path):
    rc, out, _ = r.remote(f"if (Test-Path {path}) {{ Get-Content {path} }}")
    return out.splitlines() if rc == 0 else []


def main():
    op, label = sys.argv[1], sys.argv[2]
    rest = sys.argv[3:]
    if op == "route":
        sys.argv = ["route_hb.py", label]
        r.main()
    elif op in ("lines", "count", "wait"):
        if op == "count":
            print(len(log_lines(DIAG)))
            return
        pattern = re.compile(rest[0])
        if op == "lines":
            after = int(rest[1]) if len(rest) > 1 else 0
            for path in (EVENTS, DIAG):
                for i, line in enumerate(log_lines(path)):
                    if i >= after and pattern.search(line):
                        print(f"{Path(path).name}:{i + 1}: {line[:400]}")
            return
        cap, after = float(rest[1]), int(rest[2]) if len(rest) > 2 else 0
        deadline = time.time() + cap
        while time.time() < deadline:
            lines = log_lines(DIAG)
            for i, line in enumerate(lines[after:], after):
                if pattern.search(line):
                    print(f"GATE ok line {i + 1}: {line[:400]}")
                    return
            time.sleep(1.0)
        print(f"GATE FAIL no line matching {rest[0]!r} within {cap}s")
        sys.exit(2)
    elif op == "shot":
        print(r.snap(label, rest[0]))
    elif op == "hold":
        r.press(label, rest[0], int(rest[1]))
    elif op == "call":
        print(json.dumps(r.call(label, rest[0], json.loads(rest[1]) if len(rest) > 1 else None)))
    elif op == "speed":
        if rest[0] == "off":
            print(json.dumps(r.call(label, "settings.set", {"key": "core.use_speed_limit", "value": True})))
            print(json.dumps(r.call(label, "settings.set", {"key": "core.speed_limit", "value": 100})))
        else:
            print(json.dumps(r.call(label, "settings.set", {"key": "core.use_speed_limit", "value": True})))
            print(json.dumps(r.call(label, "settings.set", {"key": "core.speed_limit", "value": int(rest[0])})))
    else:
        sys.exit(f"unknown op {op}")


if __name__ == "__main__":
    main()
