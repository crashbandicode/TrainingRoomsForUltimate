"""stream/stack Eden runs (labels st-NN), driven from the Surface over one multiplexed ssh to the Yoga.

Up/Down are this folder's copies of the playbook's eden-solo `solo.ps1` / `gv.py` (copied to the work root on the Yoga);
the route to Training is pkg4's `route4.py` (screen-gated against its reference shots `TrainingRoomsForUltimate-pkg4/eden/refs`,
every wait on a log line or a screenshot match with a cap; a miss stops the run). Every run ends with Down, also after a
failure, then the logs come back to `eden/runs/<label>/` and console.log is scanned.

    st_run.py cold <label> <nro>...    boot to the install line, wait for two heartbeat stack reports, Down
    st_run.py busy <label> <nro>...    boot, route to Training, save/load, settings reload, capture toggle, recording,
                                       menu open/close last; stack reports after each; Down
"""
import json
import re
import subprocess
import sys
import time
from pathlib import Path

from PIL import Image, ImageChops, ImageStat

HERE = Path(__file__).resolve().parent
YOGA = "memento-claw-yoga-ubuntu"
WORK = "/home/intpa/trainingrooms-eden-st"
SSH = ["/usr/bin/ssh", "-o", "BatchMode=yes", "-o", "ControlMaster=auto", "-o", "ControlPath=/tmp/trssh-st-%C",
       "-o", "ControlPersist=900", YOGA]
REFS = Path("/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-pkg4/eden/refs")
SD = f"{WORK}/profiles/y/user/sdmc/ultimate"
TR = f"{SD}/TrainingRooms"
EVENTS, DIAG, DIAG_PREV = f"{TR}/events.log", f"{TR}/practice-diagnostics.log", f"{TR}/practice-diagnostics.previous.log"
TILE_LOG = f"{SD}/ssbu-tile-cursor/events.log"
MACRO_STAGE = f"{WORK}/inbox/tr-main-to-stage-select.edenmacro.json"


def remote(command, timeout=180):
    r = subprocess.run([*SSH, command], capture_output=True, text=True, timeout=timeout)
    return r.returncode, r.stdout.strip(), r.stderr.strip()


def gv(args, timeout=120):
    rc, out, err = remote(f"/usr/bin/python3 -I {WORK}/gv.py {args}", timeout)
    if rc != 0:
        raise RuntimeError(f"gv {args}: {out} {err}")
    return out


def call(label, method, params=None):
    arg = f" '{json.dumps(params)}'" if params is not None else ""
    return json.loads(gv(f"call {label} {method}{arg}").splitlines()[-1])


def press(label, buttons, hold=150):
    gv(f"press {label} {buttons} {hold}")


def snap(label, name):
    d = HERE.parent / "runs" / label / "shots"
    d.mkdir(parents=True, exist_ok=True)
    path = gv(f"shot {label} {name}").splitlines()[-1].strip()
    subprocess.run(["/usr/bin/scp", "-q", "-o", "ControlPath=/tmp/trssh-st-%C", f"{YOGA}:{path}", str(d)], check=True)
    return d / Path(path).name


def dist(path, ref, box):
    a = Image.open(path).convert("L").crop(box)
    b = Image.open(REFS / ref).convert("L").crop(box)
    return ImageStat.Stat(ImageChops.difference(a, b)).mean[0]


def wait_match(label, ref, box, thr, cap, what):
    deadline, best = time.time() + cap, 999.0
    while time.time() < deadline:
        try:
            p = snap(label, what.replace(" ", "-"))
        except (RuntimeError, subprocess.CalledProcessError):
            time.sleep(1.5)
            continue
        d = dist(p, ref, box)
        best = min(best, d)
        if d <= thr:
            print(f"GATE ok {what} dist={d:.1f}", flush=True)
            return p
        time.sleep(1.0)
    raise RuntimeError(f"GATE FAIL {what} best_dist={best:.1f} thr={thr}")


def lines(path):
    _, out, _ = remote(f"if (Test-Path '{path}') {{ Get-Content '{path}' }}")
    return out.splitlines()


def diag_lines():
    return lines(DIAG_PREV) + lines(DIAG)


def wait_line(read, needle, cap, what, after=0):
    deadline = time.time() + cap
    while time.time() < deadline:
        for line in read()[after:]:
            if needle in line:
                print(f"GATE ok {what}: {line[:160]}", flush=True)
                return line
        time.sleep(2)
    raise RuntimeError(f"GATE FAIL {what}: no line with {needle!r}")


def wait_reports(count, cap=60):
    """Wait until the heartbeat has written `count` more `[ThreadStack] high_water name=tr-heartbeat` lines (10 s beat)."""
    start = sum("high_water name=tr-heartbeat" in x for x in diag_lines())
    deadline = time.time() + cap
    while time.time() < deadline:
        if sum("high_water name=tr-heartbeat" in x for x in diag_lines()) >= start + count:
            return
        # the heartbeat logs only when the mark grows; after the first line, wait one beat per report
        if start > 0:
            time.sleep(11 * count)
            return
        time.sleep(2)
    raise RuntimeError("no heartbeat stack report")


def solo(action, label, nros=()):
    args = f"-Action {action} -Label {label}" + (f" -Nro {','.join(nros)}" if nros else "")
    rc, out, err = remote(f"/usr/bin/pwsh -NoLogo -NoProfile -NonInteractive -File {WORK}/solo.ps1 {args}", 400)
    print(f"solo {action}: rc={rc} {out[-600:]} {err[-300:]}", flush=True)
    if rc != 0 and action == "Up":
        raise RuntimeError(f"Up failed: {err[-300:]}")


def fetch(label):
    out = HERE.parent / "runs" / label
    out.mkdir(parents=True, exist_ok=True)
    for path in (EVENTS, DIAG, DIAG_PREV, f"{WORK}/runs/{label}/console.log", f"{WORK}/runs/{label}/run.json",
                 f"{WORK}/runs/{label}/memory.peak"):
        subprocess.run(["/usr/bin/scp", "-q", "-o", "ControlPath=/tmp/trssh-st-%C", f"{YOGA}:{path}", str(out)], check=False)
    console = out / "console.log"
    hits = []
    if console.exists():
        for line in console.read_text(encoding="utf-8", errors="replace").splitlines():
            if re.search(r"panic|abort|crash|Unmapped|Cannot execute", line, re.I):
                hits.append(line[:200])
    (out / "scan.txt").write_text("\n".join(hits) + "\n", encoding="utf-8")
    print(f"scan {label}: {len(hits)} line(s) matching panic/abort/crash/Unmapped/Cannot execute", flush=True)
    for h in hits[:12]:
        print("   ", h, flush=True)
    return hits


def route(label):
    menu_box = (40, 60, 620, 380)
    for attempt in range(10):
        press(label, "a")
        time.sleep(3)
        try:
            p = snap(label, "after-a")
        except (RuntimeError, subprocess.CalledProcessError):
            time.sleep(3)
            continue
        if dist(p, "menu.png", menu_box) <= 30:
            print("GATE ok main menu", flush=True)
            break
    else:
        raise RuntimeError("main menu never matched")
    time.sleep(1.5)
    call(label, "macro.play", {"path": MACRO_STAGE})
    deadline = time.time() + 90
    while time.time() < deadline and call(label, "macro.status")["state"] == "playing":
        time.sleep(0.6)
    call(label, "input.reset")
    wait_match(label, "sss-t0.png", (440, 50, 1240, 660), 14, 40, "stage select")
    after = len(lines(TILE_LOG))
    for d in ["down"] * 3 + ["right"] * 8:
        press(label, f"lstick,{d}")
        time.sleep(0.5)
    wait_line(lambda: lines(TILE_LOG), "settled screen=sss at=41", 10, "tile 41", after=after)
    press(label, "a")
    wait_match(label, "css.png", (55, 50, 1225, 420), 12, 40, "character select")
    press(label, "lstick,up")
    time.sleep(1)
    press(label, "a")
    wait_match(label, "ready.png", (40, 40, 1240, 330), 25, 25, "ready to fight")
    press(label, "a")
    wait_match(label, "training.png", (100, 200, 430, 500), 30, 90, "training scene")


def request(name, text="x"):
    remote(f"Set-Content -Path '{TR}/{name}.tmp' -Value '{text}' -NoNewline; Move-Item -Force '{TR}/{name}.tmp' '{TR}/{name}'")


def busy(label):
    route(label)
    wait_reports(1)
    n = len(diag_lines())
    request("diag-save.req")
    wait_line(diag_lines, "state diag_request diag-save.req", 20, "tr-state-diag took the save request", after=n)
    wait_line(diag_lines, "state file_written", 20, "tr-state-writer wrote save_states.json", after=n)
    n = len(diag_lines())
    request("diag-load.req")
    wait_line(diag_lines, "state diag_request diag-load.req", 20, "tr-state-diag took the load request", after=n)
    n = len(diag_lines())
    request("diag-capture.req", "on")
    wait_line(diag_lines, "replay_capture.ini written by the writer thread", 20, "tr-ini-writer wrote replay_capture.ini", after=n)
    n = len(diag_lines())
    settings = json.dumps({"menu": {"recording_duration": [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]}})
    request("diag-settings-next.json", settings)
    wait_line(diag_lines, "diag_settings_reload", 20, "tr-diag-reload published", after=n)
    wait_reports(1)
    # Menu open/close last (the close hang is stream/sync's known defect; a hang there is told apart by fault lines).
    n = len(diag_lines())
    press(label, "minus")
    time.sleep(3)
    snap(label, "menu-open")
    press(label, "b", 150)
    time.sleep(4)
    snap(label, "menu-closed")
    wait_reports(2, cap=40)


def main():
    mode, label, nros = sys.argv[1], sys.argv[2], sys.argv[3:]
    assert re.fullmatch(r"st-\d{2}", label), label
    ok = False
    try:
        solo("Up", label, nros)
        wait_line(lambda: lines(EVENTS), "result=installed", 240, "install line")
        if mode == "busy":
            busy(label)
        else:
            wait_reports(2)
        ok = True
    except Exception as error:  # noqa: BLE001 - every failure still ends with Down and the scan
        print(f"RUN FAILED {label}: {error}", flush=True)
    finally:
        try:
            snap(label, "end")
        except Exception:  # noqa: BLE001
            pass
        solo("Down", label)
        hits = fetch(label)
    print(f"RESULT {label} mode={mode} ok={ok} fault_lines={len(hits)}", flush=True)
    sys.exit(0 if ok and not hits else 1)


if __name__ == "__main__":
    main()
