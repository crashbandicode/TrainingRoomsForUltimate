"""Package-3 Eden helpers, driven from the Surface over one multiplexed ssh to the Yoga.

Usage: /usr/bin/python3 -I eden/s3d.py <command> <label> [args]
  req <label> <name>           create TrainingRooms/<name> (diag-save.req, diag-load.req, diag-pos.req)
  settings <label> <json>      drop TrainingRooms/diag-settings.json (the diag build applies and deletes it)
  dump <label>                 request a position dump and print both fighters' lines
  wait <label> <pattern> [cap] wait for a line matching <pattern> that appears after this command started
  log <label> [n]              the last n lines of practice-diagnostics.log (default 40)
  count <label>                number of lines of the diagnostics log now
  file <label> <name>          print TrainingRooms/<name>
  put <label> <local> <name>   copy a local file to TrainingRooms/<name>
  shot <label> <name>          screenshot into eden/runs/<label>/shots
  press <label> <buttons> [ms] / stick <label> <x> <y> <ms> / macro <label> <name>
"""
import base64
import json
import re
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
YOGA = "memento-claw-yoga-ubuntu"
WORK = "/home/intpa/trainingrooms-eden-p3"
SSH = ["/usr/bin/ssh", "-o", "BatchMode=yes", "-o", "ControlMaster=auto", "-o", "ControlPath=/tmp/trssh3-%C",
       "-o", "ControlPersist=900", YOGA]
SD = f"{WORK}/profiles/t3/user/sdmc/ultimate/TrainingRooms"
DIAG = f"{SD}/practice-diagnostics.log"


def remote(command, timeout=120):
    r = subprocess.run([*SSH, command], capture_output=True, text=True, timeout=timeout)
    return r.returncode, r.stdout, r.stderr.strip()


def lines():
    rc, out, _ = remote(f"cat {DIAG} 2>/dev/null")
    return out.splitlines()


def tr(args, timeout=120):
    return remote(f"cd {WORK}; python3 -I s3_eden.py {args}", timeout)


def wait(pattern, cap=60, after=None):
    start = len(lines()) if after is None else after
    deadline = time.time() + cap
    rx = re.compile(pattern)
    while time.time() < deadline:
        found = [l for l in lines()[start:] if rx.search(l)]
        if found:
            return found
        time.sleep(1.0)
    return []


def req(name):
    remote(f"touch {SD}/{name}")


def parse_pos(line):
    return {k: float(v) for k, v in re.findall(r"(\w+)=(-?[\d.e+-]+)", line)}


def dump(cap=30):
    start = len(lines())
    req("diag-pos.req")
    deadline = time.time() + cap
    got = {}
    while time.time() < deadline and len(got) < 2:
        for l in lines()[start:]:
            m = re.search(r"state pos entry=(\d)", l)
            if m:
                got[int(m.group(1))] = parse_pos(l.split("state pos", 1)[1])
        time.sleep(0.5)
    return got


def main():
    cmd, label = sys.argv[1], sys.argv[2]
    rest = sys.argv[3:]
    if cmd == "req":
        req(rest[0])
        print("ok")
    elif cmd == "settings":
        data = rest[0]
        tmp = Path("/tmp") / f"s3-settings-{label}.json"
        tmp.write_text(data)
        subprocess.run(["/usr/bin/scp", "-q", "-o", "ControlPath=/tmp/trssh3-%C", str(tmp), f"{YOGA}:{SD}/diag-settings.json.new"], check=True)
        remote(f"mv {SD}/diag-settings.json.new {SD}/diag-settings.json")
        print("ok", wait("diag_settings applied", 15))
    elif cmd == "dump":
        print(json.dumps(dump()))
    elif cmd == "wait":
        found = wait(rest[0], float(rest[1]) if len(rest) > 1 else 60)
        print("\n".join(found) if found else "TIMEOUT")
    elif cmd == "log":
        n = int(rest[0]) if rest else 40
        print("\n".join(lines()[-n:]))
    elif cmd == "count":
        print(len(lines()))
    elif cmd == "file":
        print(remote(f"cat {SD}/{rest[0]}")[1])
    elif cmd == "put":
        subprocess.run(["/usr/bin/scp", "-q", "-o", "ControlPath=/tmp/trssh3-%C", rest[0], f"{YOGA}:{SD}/{rest[1]}"], check=True)
        print("ok")
    elif cmd == "shot":
        d = HERE / "runs" / label / "shots"
        d.mkdir(parents=True, exist_ok=True)
        rc, out, err = tr(f"shot {label} {rest[0]}")
        remote_path = out.splitlines()[-1].strip()
        subprocess.run(["/usr/bin/scp", "-q", "-o", "ControlPath=/tmp/trssh3-%C", f"{YOGA}:{remote_path}", str(d)], check=True)
        print(d / Path(remote_path).name)
    elif cmd == "press":
        rc, out, err = tr(f"press {label} {rest[0]} {rest[1] if len(rest) > 1 else 150}")
        print(out or err)
    elif cmd == "stick":
        rc, out, err = tr(f"stick {label} {rest[0]} {rest[1]} {rest[2]}")
        print(out or err)
    elif cmd == "macro":
        rc, out, err = tr(f"macro {label} {WORK}/inbox/{rest[0]}.edenmacro.json", 200)
        print(out or err)
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main()
