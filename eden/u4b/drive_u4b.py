"""Package-5 Eden driver (runs on the Surface, controls the Yoga over ssh): a list of steps, each
`press:<buttons>[:<hold ms>]`, `stick:<x>:<y>:<ms>`, `wait:<seconds>` or `shot:<name>`; screenshots are copied to
eden/runs/<label>/shots/. Usage: drive_s5.py <label> step [step ...]"""
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
YOGA = "memento-claw-yoga-ubuntu"
WORK = "/home/intpa/trainingrooms-eden-u4b"
SSH = ["/usr/bin/ssh", "-o", "BatchMode=yes", "-o", "ControlMaster=auto", "-o", "ControlPath=/tmp/trssh-u4b-%C",
       "-o", "ControlPersist=900", YOGA]


def tr(args, timeout=120):
    r = subprocess.run([*SSH, f"cd {WORK}; python3 -I tr_eden.py {args}"], capture_output=True, text=True, timeout=timeout)
    if r.returncode != 0:
        raise SystemExit(f"tr_eden {args} failed: {r.stdout} {r.stderr}")
    return r.stdout.strip()


def main():
    label, steps = sys.argv[1], sys.argv[2:]
    out = HERE.parent / "runs" / label / "shots"
    out.mkdir(parents=True, exist_ok=True)
    for step in steps:
        kind, _, rest = step.partition(":")
        if kind == "press":
            buttons, _, hold = rest.partition(":")
            tr(f"press {label} {buttons} {hold or 150}")
            time.sleep(0.25)
        elif kind == "stick":
            x, y, ms = rest.split(":")
            tr(f"stick {label} {x} {y} {ms}")
        elif kind == "wait":
            time.sleep(float(rest))
        elif kind == "shot":
            remote = tr(f"shot {label} {rest}").splitlines()[-1].strip()
            subprocess.run(["/usr/bin/scp", "-q", "-o", "ControlPath=/tmp/trssh-u4b-%C", f"{YOGA}:{remote}", str(out)], check=True)
            print(out / Path(remote).name, flush=True)
        else:
            raise SystemExit(f"bad step {step}")


main()
