"""r2fix-stream Eden rows (labels r2-NN), run from the Surface over ssh after `route_r2.py` reached the Training scene.

Usage: rows_r2.py <label> poison|clean

  poison  (r2-01) a diag boot with `diag-poison.txt` = `motion_kind transition_term_hook` (two rows forced foreign, as if another
          plugin or a patch had changed those bytes) and a fork folder whose `macros` folder is unreadable (mode 000):
            R2-1  the install line names the rejected rows and every feature whose closure holds them is not installed
                  (cpu_mash and cpu_common: motion_kind; cpu_ledge and recording: transition_term_hook), everything else is;
                  the game then runs in Training with the plugin's remaining features, with no crash and no Unmapped line.
            R2-8  the import is partial and withholds the marker (`migration=partial captures=incomplete`, no migration.json),
                  the readable files were copied, the log names the unreadable path and its error.
  clean   (r2-02) the same profile booted again without the poison file and with the folder readable again: the whole closure
                  is installed (nothing is over-disabled), the recorder, the frame advantage notice and the save states work,
                  and the import completes and writes the marker.
"""
import json
import re
import sys
import time
from pathlib import Path

import route_r2 as r

LABEL = sys.argv[1]
MODE = sys.argv[2]
assert MODE in ("poison", "clean")
SD = f"{r.PROFILE_SD}/TrainingRooms"
DIAG = f"{SD}/practice-diagnostics.log"
DIAG_PREV = f"{SD}/practice-diagnostics.previous.log"
OUT = Path(__file__).resolve().parent / "runs" / LABEL
OUT.mkdir(parents=True, exist_ok=True)
RESULTS = []


def read_diag():
    rc, out, _ = r.remote(f"foreach ($p in '{DIAG_PREV}','{DIAG}') {{ if (Test-Path $p) {{ Get-Content $p }} }}")
    return out.splitlines()


def stamp_of(line):
    m = re.match(r"(\d+) ", line)
    return int(m.group(1)) if m else -1


def mark():
    return max((stamp_of(x) for x in read_diag()), default=0)


def lines_after(start):
    return [x for x in read_diag() if stamp_of(x) > start]


def line_after(start, needle, cap=20):
    deadline = time.time() + cap
    while time.time() < deadline:
        for line in lines_after(start):
            if needle in line:
                return line
        time.sleep(1.5)
    return None


def row(name, ok, evidence):
    RESULTS.append((name, ok, evidence))
    print(f"ROW {'pass' if ok else 'FAIL'} {name}: {evidence}", flush=True)


def tap(buttons, hold=150, wait=0.4):
    r.press(LABEL, buttons, hold)
    time.sleep(wait)


def shot(name):
    return r.snap(LABEL, name)


def touch(name, text=""):
    r.remote(f"Set-Content -Path '{SD}/{name}' -Value '{text}' -NoNewline")


BASE = {"recording_duration": [1] + [0] * 18, "frame_advantage": [1, 0]}


def reload(settings, what):
    body = json.dumps({"menu": {**BASE, **settings}})
    m = mark()
    r.remote(f"Set-Content -Path '{SD}/diag-settings-next.tmp' -Value '{body}' -NoNewline; "
             f"Move-Item -Force '{SD}/diag-settings-next.tmp' '{SD}/diag-settings-next.json'")
    ln = line_after(m, "diag_settings_reload", 10)
    print(f"reload {what}: {ln}", flush=True)
    return ln


def ones(n, *indices):
    return [1 if i in indices else 0 for i in range(n)]


def exists(path):
    return r.remote(f"Test-Path '{path}'")[1].strip() == "True"


def main():
    rc, events, _ = r.remote(f"Get-Content {SD}/events.log")
    (OUT / "events.log").write_text(events + "\n")
    install = events.splitlines()[-1] if events else ""
    (OUT / "install-line.txt").write_text(install + "\n")
    everything = ["input_delay", "lra_reset", "stage_hazards", "reset_hub", "cpu_core", "cpu_defence", "cpu_ledge", "cpu_tech",
                  "cpu_shield", "cpu_mash", "state_core", "chords", "recording", "notices", "ui"]
    if MODE == "poison":
        ln = line_after(0, "diag_poison motion_kind found=true", 15)
        ln2 = line_after(0, "diag_poison transition_term_hook found=true", 15)
        row("R2-1 the two rows were poisoned (diag_poison lines)", ln is not None and ln2 is not None, f"{ln} | {ln2}")
        want = [" motion_kind=foreign/skipped", " transition_term_hook=foreign/skipped",
                " cpu_mash=not_installed reason=site:motion_kind", " cpu_common=not_installed reason=site:motion_kind",
                " cpu_ledge=not_installed reason=site:transition_term_hook", " recording=not_installed reason=site:transition_term_hook"]
        row("R2-1 the install line names the rejected row for every feature whose closure holds it",
            all(w in install for w in want), "; ".join(w.strip() for w in want if w not in install) or install[-420:])
        kept = ["cpu_core", "cpu_defence", "cpu_tech", "cpu_shield", "state_core", "input_delay", "notices", "ui", "chords", "reset_hub"]
        # input_delay needs fighter_manager_get_entry and status_kind: untouched by this poison.
        row("R2-1 no other feature is over-disabled", all(f" {f}=installed" in install for f in kept), ", ".join(f for f in kept if f" {f}=installed" not in install) or "all kept installed")
        row("R2-1 the install line is the supported one", "result=installed" in install and "heap_allocations=0" in install, install[-90:])
        # The rest of the plugin runs: the CPU core sees the CPU, the save states work, nothing of the disabled features runs.
        role = line_after(0, "role entry=1", 30)
        row("R2-1 the CPU core still runs (it reads the CPU body's role)", role is not None, role or "no role line")
    else:
        row("R2-1 clean boot: every feature of the closure is installed (nothing over-disabled)",
            all(f" {f}=installed" in install for f in everything) and "cpu_common=" in install and "reason=site" not in install,
            "; ".join(f for f in everything if f" {f}=installed" not in install) or "all installed")
        row("R2-1 clean boot: heap_allocations at install", "heap_allocations=0" in install and "result=installed" in install, install[-90:])

    # ---- R2-8 --------------------------------------------------------------------------------------------------
    marker = exists(f"{SD}/migration.json")
    if MODE == "poison":
        row("R2-8 the import is partial and the install line says so", " migration=partial captures=incomplete" in install, re.search(r"migration=\S+( captures=\S+)?", install).group(0) if "migration=" in install else "no migration token")
        cannot = line_after(0, "cannot list macros:", 15)
        row("R2-8 the unreadable folder is logged with its path and error, the import stays incomplete", cannot is not None and "io:" in cannot, cannot or "no line")
        row("R2-8 no migration.json is written (the next boot imports again)", not marker, f"migration.json exists={marker}")
        copied = exists(f"{SD}/setups/a.json") and exists(f"{SD}/mii-presets.json")
        row("R2-8 the readable files were still imported, the unreadable folder was not", copied and not exists(f"{SD}/macros/m.json"), f"setups/a.json+mii-presets.json copied={copied}")
    else:
        row("R2-8 second boot with the folder readable: the import completes", " migration=done" in install, re.search(r"migration=\S+", install).group(0) if "migration=" in install else "no migration token")
        row("R2-8 the missing content is now imported and the marker is written", exists(f"{SD}/macros/m.json") and marker, f"macros/m.json={exists(f'{SD}/macros/m.json')} migration.json={marker}")
        if marker:
            text = r.remote(f"Get-Content {SD}/migration.json -Raw")[1]
            (OUT / "migration.json").write_text(text + "\n")
            row("R2-8 the receipt says done and names the late file", '"result": "done"' in text and "macros/m.json" in text, re.search(r'"result": "\w+"', text).group(0) if '"result"' in text else "no result")
        kept = line_after(0, "[Migration] copied macros/m.json", 10)
        row("R2-8 the second boot copied only what was missing (macros/m.json), keeping what exists", kept is not None and line_after(0, "[Migration] copied setups/a.json", 2) is None, kept or "no copied line")

    # ---- gameplay: save states work in both boots --------------------------------------------------------------
    m = mark()
    touch("diag-pos.req")
    pos_a = line_after(m, "state pos entry=0", 10)
    tap("zl,down", wait=0.6)
    saved = line_after(m, "state saved_notice", 10)
    row("save chord (ZL + D-pad Down) saves a state", saved is not None, saved or "none")
    r.tr(f"stick {LABEL} 1 0 500")
    time.sleep(0.5)
    m = mark()
    tap("zl,up", wait=0.5)
    restored = line_after(m, "state restored", 12)
    row("load chord (ZL + D-pad Up) restores it", restored is not None, restored or "none")
    time.sleep(1.0)
    m = mark()
    touch("diag-pos.req")
    pos_c = line_after(m, "state pos entry=0", 10)

    def xof(line):
        mm = re.search(r" x=(-?[\d.]+)", line or "")
        return float(mm.group(1)) if mm else None

    xa, xc = xof(pos_a), xof(pos_c)
    row("P1 is back at the saved position", None not in (xa, xc) and abs(xc - xa) < 1.0, f"saved x={xa}, loaded x={xc}")

    # ---- the disabled features do nothing (poison), the enabled ones work (clean) ------------------------------
    m = mark()
    tap("zr,down", wait=0.1)
    time.sleep(0.8)
    tap("a", hold=200, wait=0.2)
    rec = line_after(m, "recording_complete", 12 if MODE == "clean" else 6)
    if MODE == "poison":
        row("R2-1 the recorder is dormant (recording not installed): the record chord starts nothing", rec is None and not [x for x in lines_after(m) if "record_lockout" in x], rec or "no recording lines")
    else:
        row("recorder works on the clean boot: a slot is recorded", rec is not None, rec or "none")
    m = mark()
    tap("a", hold=100, wait=0.1)  # a whiffed jab: the CPU is actionable first
    fa = line_after(m, 'notice_draw header="Frame Advantage"', 10)
    shot("frame-advantage")
    if MODE == "poison":
        row("R2-1 no frame advantage measurement without cpu_mash (its natives are never read)", fa is None, fa or "no Frame Advantage notice")
    else:
        row("frame advantage notice appears on the clean boot", fa is not None, fa or "none")
    reload({"mash_state": [0, 1] + [0] * 28, "mash_triggers": ones(17, 16)}, "mash Jump, always")
    time.sleep(3.0)
    shot("mash-on")
    mashed = [x for x in lines_after(m) if "command_request_status" in x or "mash_" in x]
    if MODE == "poison":
        row("R2-1 the mash settings change nothing (cpu_mash not installed), and the game keeps running", not mashed, f"{len(mashed)} mash lines")
    reload({"mash_state": [0] * 30}, "mash off")
    time.sleep(2.0)
    shot("end")
    # The game is still alive: the diag heartbeat still advances.
    t1 = mark()
    time.sleep(3.0)
    t2 = mark()
    row("the game keeps running after the exercise (diag clock advances)", t2 > t1, f"{t1} -> {t2}")

    (OUT / "rows.txt").write_text("\n".join(f"{'pass' if ok else 'FAIL'}\t{name}\t{ev}" for name, ok, ev in RESULTS) + "\n")
    (OUT / "diag.log").write_text("\n".join(read_diag()) + "\n")
    sys.exit(0 if all(ok for _, ok, _ in RESULTS) else 1)


if __name__ == "__main__":
    main()
