"""Wire-stream Eden rows (one batched boot, label w-NN), run from the Surface over ssh after `route_w.py` reached the
Training scene. The diag build writes one line per event; every wait is gated on such a line or a screenshot, with a cap.
Usage: rows_w.py <label>

Rows (the cross-stream wiring between packages 2-5):
  1  install line, hud draw registered, heap_allocations
  2  notice and input log drawn (diag lines + screenshots), Frame Advantage notice
  3  save / load by chord (pkg4 chords -> pkg3), position restored
  4  recorder stop on a load request while a playback loops, then state restored with Recording Trigger = Save State Load
  5  Playback Mash Interrupt stops a running playback (pkg2 -> pkg4)
  6  shortcut edit and replay-capture toggle written by the writer thread, R5 delta 0
  7  the real menu closes with a change: settings.json written by the writer thread
"""
import json
import re
import sys
import time
from pathlib import Path

import route_w as r

LABEL = sys.argv[1]
SD = f"{r.PROFILE_SD}/TrainingRooms"
DIAG = f"{SD}/practice-diagnostics.log"
DIAG_PREV = f"{SD}/practice-diagnostics.previous.log"
OUT = Path(__file__).resolve().parent / "runs" / LABEL
OUT.mkdir(parents=True, exist_ok=True)
RESULTS = []


def read_diag():
    """Both diag files (the ring rotates at 128 KiB), oldest first."""
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


def heap_counts():
    counts = [x for x in read_diag() if "input_hook_counts" in x]
    return [int(re.search(r"heap_allocations=(\d+)", x).group(1)) for x in counts]


# The boot settings (`settings/boot-w.json`): every reload carries them again, since a reload replaces the whole snapshot.
BASE = {"recording_duration": [1] + [0] * 18, "frame_advantage": [1, 0]}


def reload(settings, what):
    """Replace the live settings through the diag reload (the file is written under another name, then renamed)."""
    body = json.dumps({"menu": {**BASE, **settings}})
    m = mark()
    r.remote(f"Set-Content -Path '{SD}/diag-settings-next.tmp' -Value '{body}' -NoNewline; "
             f"Move-Item -Force '{SD}/diag-settings-next.tmp' '{SD}/diag-settings-next.json'")
    ln = line_after(m, "diag_settings_reload", 10)
    print(f"reload {what}: {ln}", flush=True)
    return ln


def touch(name, text=""):
    r.remote(f"Set-Content -Path '{SD}/{name}' -Value '{text}' -NoNewline")


def ones(n, *indices):
    return [1 if i in indices else 0 for i in range(n)]


def main():
    rc, events, _ = r.remote(f"Get-Content {SD}/events.log")
    (OUT / "events.log").write_text(events + "\n")
    installed = all(x in events for x in ("ai_epilogue=stock/near", "recording=installed", "notices=installed", "chords=installed",
                                          "result=installed"))
    row("1 install line: recorder, chords, notices", installed, events.splitlines()[-1][:260] if events else "none")
    row("1 heap_allocations at install", "heap_allocations=0" in events, "heap_allocations=0")
    ln = line_after(0, "hud_draw layout registrations", 15)
    row("1 the HUD draw runs on the info_training layout (registered after ui::install)", ln is not None, ln or "no hud_draw line")

    # ---- 2: a notice and the input log on screen ---------------------------------------------------------------
    m = mark()
    for b in ("a", "x", "b"):
        tap(b, hold=100, wait=0.25)
    ln = line_after(m, "input_display_plan", 10)
    row("2 input display plan applied to the panes (rows and icons)", ln is not None and "icons=" in ln, ln or "none")
    time.sleep(0.4)
    shot("input-log")
    m = mark()
    tap("zr,up", wait=0.15)  # empty slot: a refusal notice
    shot("notice-refusal")
    ln = line_after(m, "notice_draw header=\"Input Recording\" body=\"Record into this slot first\"", 10)
    row("2 a notice is drawn into the notice slot", ln is not None, ln or "none")

    # ---- 3: save / load by chord -------------------------------------------------------------------------------
    m = mark()
    touch("diag-pos.req")
    pos_a = line_after(m, "state pos entry=0", 10)
    tap("zl,down", wait=0.6)
    saved = line_after(m, "state saved_notice", 10)
    row("3 Save chord (ZL + D-pad Down) reaches package 3", saved is not None, saved or "no saved_notice line")
    r.tr(f"stick {LABEL} 1 0 500")
    time.sleep(0.5)
    m = mark()
    touch("diag-pos.req")
    pos_b = line_after(m, "state pos entry=0", 10)
    m = mark()
    tap("zl,up", wait=0.5)
    req = line_after(m, "state load_request", 10)
    restored = line_after(m, "state restored", 12)
    row("3 Load chord (ZL + D-pad Up) loads the state", req is not None and restored is not None, f"{req} | {restored}")
    time.sleep(1.0)
    m = mark()
    touch("diag-pos.req")
    pos_c = line_after(m, "state pos entry=0", 10)

    def xof(line):
        mm = re.search(r" x=(-?[\d.]+)", line or "")
        return float(mm.group(1)) if mm else None

    xa, xb, xc = xof(pos_a), xof(pos_b), xof(pos_c)
    row("3 P1 is back at the saved position", None not in (xa, xb, xc) and abs(xc - xa) < 1.0 and abs(xb - xa) > 5.0,
        f"saved x={xa}, moved x={xb}, loaded x={xc}")

    # ---- 4: recorder stop on a load request, state restored -> record ---------------------------------------------
    m = mark()
    tap("zr,down", wait=0.1)
    time.sleep(0.8)
    tap("a", hold=200, wait=0.2)
    rec = line_after(m, "recording_complete", 25)
    row("4 a slot is recorded (60-frame duration, cropped)", rec is not None, rec or "none")
    reload({"playback_loop": [1, 0]}, "playback loop on")
    m = mark()
    tap("zr,up", wait=0.15)
    start = line_after(m, "playback_shortcut", 10)
    time.sleep(1.2)
    shot("playback-loop")
    row("4 a looping playback is running", start is not None and any("restarted=1" in x for x in lines_after(m)), start or "none")
    m = mark()
    tap("zl,up", wait=0.3)
    stop = line_after(m, "recorder_stop", 10)
    restored = line_after(m, "state_restored", 12)
    t0 = stamp_of(stop) if stop else 0
    later = [x for x in lines_after(t0 + 500) if "playback_complete" in x and "state" not in x]
    row("4 a load request stops the playback (recorder::stop from package 3)", stop is not None and "mode=3" in stop and not later,
        f"{stop} | completions 0.5 s later: {len(later)}")
    row("4 package 3 tells the recorder the restore finished (recorder::on_state_restored)", restored is not None, restored or "none")
    reload({"playback_loop": [0, 1], "record_trigger": [0, 1]}, "Recording Trigger = Save State Load")
    time.sleep(1.0)
    m = mark()
    tap("zl,up", wait=0.5)
    ln = line_after(m, "state_restored", 12)
    lock = line_after(m, "record_lockout", 12)
    row("4 Recording Trigger = Save State Load: the restore starts a recording", ln is not None and "action=1" in ln and lock is not None,
        f"{ln} | {lock}")
    tap("a", hold=200, wait=0.3)
    reload({"record_trigger": [1, 0]}, "Recording Trigger = Button Combination")
    time.sleep(3.0)

    # ---- 2 (cont.): the Frame Advantage notice ------------------------------------------------------------------
    m = mark()
    tap("a", hold=100, wait=0.1)  # a whiffed jab: the CPU is actionable first
    ln = line_after(m, "notice_draw header=\"Frame Advantage\"", 12)
    shot("frame-advantage")
    row("2 frame advantage is shown as a notification (pkg2 -> pkg4 notices)", ln is not None, ln or "none")

    # ---- 5: Playback Mash Interrupt -------------------------------------------------------------------------------
    # The CPU mashes Jump whenever it can (Always), a slot-1 playback is started: the mash stops it.
    reload({"mash_state": [0, 1] + [0] * 28, "mash_triggers": ones(17, 16), "playback_loop": [1, 0]}, "mash Jump, always; loop")
    time.sleep(1.0)
    m = mark()
    tap("zr,up", wait=0.3)
    ln = line_after(m, "playback_interrupt", 20)
    stop = line_after(m, "playback_stop entry=1", 5)
    row("5 Playback Mash Interrupt stops the playback (pkg2 inbox -> pkg4 recorder)", ln is not None, f"{ln} | {stop}")
    reload({"mash_state": [0] * 30, "mash_triggers": ones(17, 0, 1, 3, 7), "playback_loop": [0, 1]}, "mash off")

    # ---- 6: the writer thread ------------------------------------------------------------------------------------
    before = heap_counts()
    m = mark()
    touch("diag-shortcut.req", "1 12")
    ln = line_after(m, "button_config.ini written by the writer thread", 12)
    ini = r.remote(f"Get-Content {SD}/button_config.ini")[1]
    (OUT / "button_config.ini").write_text(ini + "\n")
    row("6 a shortcut edit is written to button_config.ini by the writer thread", ln is not None and "shortcut_2=X + Y" in ini,
        f"{ln} | {ini.replace(chr(10), ' | ')}")
    m = mark()
    touch("diag-capture.req", "on")
    ln = line_after(m, "replay_capture.ini written by the writer thread", 12)
    cap = r.remote(f"Get-Content {SD}/replay_capture.ini")[1]
    (OUT / "replay_capture.ini").write_text(cap + "\n")
    row("6 the Replay capture toggle is written to replay_capture.ini by the writer thread", ln is not None and "enabled=on" in cap,
        f"{ln} | {cap.replace(chr(10), ' | ')}")
    time.sleep(14)
    after = heap_counts()
    (OUT / "hook-counts.txt").write_text("\n".join(x for x in read_diag() if "input_hook_counts" in x) + "\n")
    row("6 R5: heap_allocations in hook scope stays 0 across both writes", len(after) > len(before) and set(after) == {0},
        f"{len(after)} reports, values {sorted(set(after))}")

    # ---- 7: the real menu: a change on close is written through the writer thread ---------------------------------
    m = mark()
    tap("b,up", wait=1.5)
    opened = line_after(m, "[Ui] menu opened", 10)
    shot("menu-open")
    tap("a", wait=0.8)   # first row (Mash): its list opens
    tap("a", wait=0.5)   # toggles the first entry
    tap("b", wait=0.6)   # back to the page
    shot("menu-changed")
    tap("b", wait=1.5)   # close: commit
    closed = line_after(m, "[Ui] menu closed", 10)
    written = line_after(m, "settings.json written by the writer thread", 12)
    row("7 the menu opens (B + D-pad Up), a row is changed and the close hands the settings to the writer thread",
        opened is not None and closed is not None and "changed=true" in (closed or "") and written is not None,
        f"{opened} | {closed} | {written}")

    time.sleep(1.0)
    rc, console, _ = r.remote(f"Get-Content {SD}/events.log | Select-Object -Last 3")
    (OUT / "rows.txt").write_text("\n".join(f"{'pass' if ok else 'FAIL'}\t{name}\t{ev}" for name, ok, ev in RESULTS) + "\n")
    sys.exit(0 if all(ok for _, ok, _ in RESULTS) else 1)


if __name__ == "__main__":
    main()
