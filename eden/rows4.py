"""Package-4 Eden rows (SPEC-input-01/02/04/05), run from the Surface/Yoga over ssh after `route4.py` reached
the Training scene. Every wait is gated on a plugin log line (the diag build writes them) or a screenshot,
with a cap. Usage: rows4.py <label> A|B    (A: defaults + 60-frame recording; B: loop on, save-state playback
slot 1, reversed load order with the CPU pilot).
"""
import re
import subprocess
import sys
import time
from pathlib import Path

import route4 as r

LABEL = sys.argv[1]
BOOT = sys.argv[2]
SD = f"{r.PROFILE_SD}/TrainingRooms"
DIAG = f"{SD}/practice-diagnostics.log"
OUT = Path(__file__).resolve().parent / "runs" / LABEL
OUT.mkdir(parents=True, exist_ok=True)
RESULTS = []


def read_diag():
    rc, out, _ = r.remote(f"if (Test-Path {DIAG}) {{ Get-Content {DIAG} }}")
    return out.splitlines()


def mark():
    return len(read_diag())


def line_after(start, needle, cap=20):
    deadline = time.time() + cap
    while time.time() < deadline:
        for line in read_diag()[start:]:
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


def common():
    rc, events, _ = r.remote(f"Get-Content {SD}/events.log")
    (OUT / "events.log").write_text(events + "\n")
    installed = all(x in events for x in ("ai_epilogue=stock/near", "recording=installed", "notices=installed", "chords=installed", "result=installed"))
    row("install line: recorder hook, chords, notices installed", installed, events.splitlines()[-1][:300] if events else "none")
    row("heap_allocations at install", "heap_allocations=0" in events, "heap_allocations=0")


def boot_a():
    common()
    # Empty slot: the playback chord is refused with a notice (B8), and a notice waits while the menu is open.
    m = mark()
    tap("zr,up")
    shot("empty-slot")
    ln = line_after(m, "chords_passed")
    row("playback chord passes (raw controller, P1 poll)", ln is not None, ln or "no chords_passed line")
    ln = line_after(m, "Record into this slot first")
    row("empty slot refused with a notice (Q11, B8)", ln is not None, ln or "no notice_draw line")
    m = mark()
    tap("b,up")
    ln = line_after(m, "standin menu_owner open=true")
    row("menu owner stand-in on (B + D-pad Up shortcut)", ln is not None, ln or "none")
    ln = line_after(m, "notice_visible=false menu_open=true", 6)
    row("notice hidden while the menu is open (B8)", ln is not None, ln or "none")
    time.sleep(1.0)
    tap("b,up", wait=1.5)
    ln = line_after(m, "standin menu_owner open=false")
    row("menu owner stand-in off", ln is not None, ln or "none")
    ln = line_after(m, "notice_visible=true menu_open=false", 6)
    row("notice visible again after the menu closes", ln is not None, ln or "none")
    # Record: lockout, standby, P1 acts, crop.
    m = mark()
    tap("zr,down", wait=0.1)
    shot("lockout")
    ln = line_after(m, "record_lockout")
    row("record chord starts the lockout", ln is not None, ln or "none")
    time.sleep(0.8)
    shot("standby")
    ln = line_after(m, "notice_draw header=\"Input Recording\" body=\"Standby\"")
    row("standby notice posted once", ln is not None, ln or "none")
    tap("a", hold=200, wait=0.2)
    shot("recording")
    ln = line_after(m, "recording_complete", 25)
    frames = int(re.search(r"frames=(\d+)", ln).group(1)) if ln else -1
    row("recording ends and is cropped (60-frame duration)", ln is not None and 0 < frames < 60, ln or "none")
    # Playback.
    m = mark()
    tap("zr,up", wait=0.15)
    shot("playback")
    ln = line_after(m, "playback_shortcut")
    row("playback chord starts a playback of the recorded slot", ln is not None and f"frames={frames}" in ln, ln or "none")
    ln = line_after(m, "playback_complete", 10)
    row("playback ends (every stored frame played)", ln is not None, ln or "none")
    # Chord edit applied at once (N7 stand-in): X+Y does not open the menu, the edit makes it.
    m = mark()
    tap("x,y", wait=0.8)
    row("X+Y opens nothing before the edit", line_after(m, "standin menu_owner", 3) is None, "no menu_owner line")
    tap("zl,up", wait=1.0)
    ln = line_after(m, "standin shortcut_edit")
    ini = r.remote(f"Get-Content {SD}/button_config.ini")[1]
    (OUT / "button_config.ini").write_text(ini + "\n")
    row("shortcut edit written to button_config.ini", ln is not None and "Ok(())" in ln and "X + Y" in ini, ini.replace("\n", " | "))
    m = mark()
    tap("x,y", wait=1.0)
    ln = line_after(m, "standin menu_owner open=true", 6)
    row("the edited shortcut applies at once (X+Y opens the menu stand-in)", ln is not None, ln or "none")
    tap("x,y", wait=1.0)
    # Input display data path and hook counts.
    m = mark()
    for b in ("a", "b", "x"):
        tap(b, hold=100, wait=0.3)
    ln = line_after(m, "input_display_plan")
    row("input display plan produced for new rows (panes wait for package 5)", ln is not None, ln or "none")
    time.sleep(12)
    counts = [x for x in read_diag() if "input_hook_counts" in x]
    (OUT / "hook-counts.txt").write_text("\n".join(counts) + "\n")
    heap = {re.search(r"heap_allocations=(\d+)", x).group(1) for x in counts}
    row("hook invocation counts and heap_allocations after the hooks ran (R5)", bool(counts) and heap == {"0"}, (counts[-1] if counts else "no counts")[:200])


def boot_b():
    common()
    m = mark()
    tap("zr,down", wait=0.1)
    time.sleep(0.8)
    tap("a", hold=200, wait=0.2)
    ln = line_after(m, "recording_complete", 25)
    row("recorded one slot", ln is not None, ln or "none")
    m = mark()
    tap("zr,up", wait=0.15)
    time.sleep(3)
    lines = [x for x in read_diag()[m:] if "playback_complete" in x]
    row("Playback Loop On: the playback restarts", any("restarted=1" in x for x in lines) and len(lines) >= 2, f"{len(lines)} completions, last: {lines[-1][:120] if lines else 'none'}")
    # Stop it through the reset chord (decision B8: stop on reset).
    m = mark()
    tap("l,r,a", hold=400, wait=0.5)
    time.sleep(1.5)
    after = [x for x in read_diag()[m:] if "playback_complete" in x]
    row("L+R+A reset stops the recorder (B8)", len(after) <= 1, f"{len(after)} completion lines in 1.5 s after the reset")
    # Save-state playback through the stand-in (save-state chord = state restored).
    m = mark()
    tap("zl,down", wait=0.2)
    ln = line_after(m, "playback_shortcut")
    row("save-state playback slot plays on a restore (stand-in)", ln is not None, ln or "none")


boot = {"A": boot_a, "B": boot_b}[BOOT]
boot()
(OUT / "rows.txt").write_text("\n".join(f"{'pass' if ok else 'FAIL'}\t{name}\t{ev}" for name, ok, ev in RESULTS) + "\n")
sys.exit(0 if all(ok for _, ok, _ in RESULTS) else 1)
