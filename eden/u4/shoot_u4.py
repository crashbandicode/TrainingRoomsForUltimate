"""u4 screenshots (runs on the Surface, controls the Yoga over ssh): waits for the diag probe's `[Texture] shown cycle=N` line
in the plugin log, then takes screenshots of the game while the tiles are bound (the probe holds them for 900 UI frames), and
copies them to eden/runs/<label>/shots/. Usage: shoot_u4.py <label> <name> [cycles] [log pattern, default "[Texture] shown"]"""
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
YOGA = "memento-claw-yoga-ubuntu"
WORK = "/home/intpa/trainingrooms-eden-u4"
LOG = f"{WORK}/profiles/u4/user/sdmc/ultimate/TrainingRooms/practice-diagnostics.log"
SSH = ["/usr/bin/ssh", "-o", "BatchMode=yes", "-o", "ControlMaster=auto", "-o", "ControlPath=/tmp/trssh-u4-%C",
       "-o", "ControlPersist=900", YOGA]


def remote(command, timeout=120):
    r = subprocess.run([*SSH, command], capture_output=True, text=True, timeout=timeout)
    return r.returncode, r.stdout, r.stderr


def lines():
    rc, out, _ = remote(f"Get-Content {LOG} | Select-String '\\[Texture\\]' | ForEach-Object {{ $_.Line }}")
    return out.splitlines()


def shot(label, name, out):
    rc, text, err = remote(f"cd {WORK}; python3 -I tr_eden.py shot {label} {name}")
    if rc != 0:
        raise SystemExit(f"shot failed: {text} {err}")
    remote_path = text.splitlines()[-1].strip()
    subprocess.run(["/usr/bin/scp", "-q", "-o", "ControlPath=/tmp/trssh-u4-%C", f"{YOGA}:{remote_path}", str(out)], check=True)
    return out / Path(remote_path).name


def main():
    label, name = sys.argv[1], sys.argv[2]
    cycles = int(sys.argv[3]) if len(sys.argv) > 3 else 1
    pattern = sys.argv[4] if len(sys.argv) > 4 else "[Texture] shown"
    out = HERE.parent / "runs" / label / "shots"
    out.mkdir(parents=True, exist_ok=True)
    seen = sum(1 for l in lines() if pattern in l)
    done = 0
    deadline = time.time() + 900
    while done < cycles and time.time() < deadline:
        now = [l for l in lines() if pattern in l]
        if len(now) > seen:
            seen = len(now)
            for k in range(3 if pattern.endswith("shown") else 1):
                print(shot(label, f"{name}-c{seen}-{k}", out), flush=True)
                time.sleep(1.0)
            done += 1
            print(now[-1][:200], flush=True)
        time.sleep(1.0)
    print(f"done cycles={done}", flush=True)


main()
