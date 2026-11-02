"""stream/sync Eden proof on the lent Ally rig (Eden already in Training via `ui_lent.py route`): many menu open/close cycles, the
Named-library macro load (pkg4's s4-07 path: a settings publish from the diag thread with Macro Source = Named Library), more close
cycles after it, the arbiter self-test line, the hook allocation counters and Eden's fps around every close. Every wait is gated on a
diagnostics line; each row prints `ROW pass|FAIL name: evidence`.

    rows_sync01.py live-sync-01 [cycles]
"""
import json
import re
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ui_lent as u  # noqa: E402

PHASE = sys.argv[1]
CYCLES = int(sys.argv[2]) if len(sys.argv) > 2 else 20
DIAG = "TrainingRooms/practice-diagnostics.log"
SD = r"C:\Eden\c1162-w5-ally\profile\user\sdmc\ultimate\TrainingRooms"
OUT = HERE.parent / "runs" / PHASE.replace("live-", "")
OUT.mkdir(parents=True, exist_ok=True)
ROWS = []
FPS = []


def row(name, ok, evidence):
    ROWS.append({"row": name, "ok": bool(ok), "evidence": evidence})
    print(f"ROW {'pass' if ok else 'FAIL'} {name}: {evidence}", flush=True)


def count(regex):
    return u.count(PHASE, DIAG, regex)


def lines(regex, n=5):
    return u.plug(PHASE, DIAG, regex, n)["lines"]


def wait_more(regex, before, cap):
    end = time.time() + cap
    while time.time() < end:
        if count(regex) > before:
            return True
        time.sleep(0.5)
    return False


def status():
    st = u.ops(PHASE, [["status"]])[0].get("result", {})
    return st


def fps_sample(tag):
    st = status()
    sample = {"tag": tag, "t": round(time.time(), 2)}
    for key in ("fps", "game_fps", "emulation_speed", "frame", "frame_age_ms", "last_frame_age_ms", "state"):
        if key in st:
            sample[key] = st[key]
    FPS.append(sample)
    return sample


def ps(snippet):
    return u.ps(snippet, 90)


def macro_cmd(text):
    ps(f"Set-Content -Path '{SD}\\diag-macro.req' -Value '{text}'")


def main():
    st = lines(r"\[Sync\] arbiter self_test", 1)
    row("hos-lock arbiter self-test at boot (kernel result codes as the source says)", bool(st) and "ok=true" in st[-1], st[-1] if st else "no line")
    fps_sample("before")

    # ---- many open/close cycles -----------------------------------------------------------------------------------------
    def cycles(n, tag):
        ok = 0
        for i in range(n):
            o, c = count(r"\[Ui\] menu opened"), count(r"\[Ui\] menu closed")
            u.keys(PHASE, "b+up")
            if not wait_more(r"\[Ui\] menu opened", o, 10):
                row(f"{tag} cycle {i + 1}: menu opened", False, "no [Ui] menu opened line in 10 s")
                return ok
            # every third cycle changes a row first, so the close commits a change (a publish with changed rows)
            u.keys(PHASE, "w5 down w3 right w3" if i % 3 == 2 else "w5")
            u.keys(PHASE, "b")
            if not wait_more(r"\[Ui\] menu closed", c, 10):
                row(f"{tag} cycle {i + 1}: menu closed", False, "no [Ui] menu closed line in 10 s (the hang)")
                fps_sample(f"{tag}-hang-{i + 1}")
                return ok
            s = fps_sample(f"{tag}-{i + 1}")
            ok += 1
            u.keys(PHASE, "w8")
        return ok

    done = cycles(CYCLES, "first")
    closed = lines(r"\[Ui\] menu closed", 3)
    row(f"menu open/close cycles complete ({done}/{CYCLES}; every third with a changed row)", done == CYCLES, closed[-1] if closed else "none")
    u.shot(PHASE, "after-first-cycles")

    # ---- the Named-library macro load (pkg4 s4-07 path) -------------------------------------------------------------------
    m = count("recording_complete")
    u.keys(PHASE, "zr+down w12 a w10")
    rec = wait_more("recording_complete", m, 25)
    row("numbered recording into slot 1", rec, (lines("recording_complete", 1) or ["none"])[-1])
    m = count(r"\[NamedMacroDiag\] save_slot")
    macro_cmd("save_slot 0 synctest")
    wait_more(r"\[NamedMacroDiag\] save_slot", m, 10)
    time.sleep(1.5)
    files = ps(f"Get-ChildItem '{SD}\\macros' -Name -ErrorAction SilentlyContinue")
    row("Save slot to library writes macros/synctest.json", "synctest.json" in files, files.strip().replace("\n", " "))
    m = count(r"\[NamedMacroDiag\]")
    macro_cmd("open_library 0")
    wait_more(r"\[NamedMacroDiag\]", m, 10)
    target = None
    for _ in range(12):
        m = count(r"\[NamedMacroDiag\] view")
        macro_cmd("view")
        wait_more(r"\[NamedMacroDiag\] view", m, 6)
        view = (lines(r"\[NamedMacroDiag\] view", 1) or [""])[-1]
        sel = re.search(r"selected=(\d+)", view)
        labels = re.search(r"labels=\[(.*)\]", view)
        items = re.findall(r'"((?:[^"\\]|\\.)*)"', labels.group(1)) if labels else []
        if sel and int(sel.group(1)) < len(items) and "synctest" in items[int(sel.group(1))]:
            target = int(sel.group(1))
            break
        macro_cmd("pad down")
        time.sleep(0.6)
    macro_cmd("pad confirm")
    time.sleep(1.5)
    selected = ps(f"Get-Content -Raw '{SD}\\macros\\selected.txt' -ErrorAction SilentlyContinue").strip()
    row("Load a named macro from the library (selected.txt follows)", target is not None and selected == "synctest", f"selected.txt={selected!r}, row {target}")
    # Macro Source = Named Library through the diag reload thread: a settings publish from a non-game thread (the s4-07 hang)
    settings = json.loads(ps(f"Get-Content -Raw '{SD}\\settings.json'") or "{}")
    settings.setdefault("menu", {})["macro_source"] = [0, 1]
    body = json.dumps(settings).replace("'", "''")
    m = count("diag_settings_reload")
    ps(f"Set-Content -Path '{SD}\\diag-settings-next.tmp' -Value '{body}'; Move-Item -Force '{SD}\\diag-settings-next.tmp' '{SD}\\diag-settings-next.json'")
    reloaded = wait_more("diag_settings_reload", m, 10)
    m = count(r"\[NamedMacroDiag\] status")
    macro_cmd("status")
    wait_more(r"\[NamedMacroDiag\] status", m, 6)
    st = (lines(r"\[NamedMacroDiag\] status", 1) or ["none"])[-1]
    row("publish from the diag thread (Macro Source = Named Library) completes; the game keeps running", reloaded and "named_source=true" in st, st[:200])
    fps_sample("after-named-publish")
    ps(f"Set-Content -Path '{SD}\\diag-save.req' -Value 'x'")
    time.sleep(2.0)
    m = count("named_playback frames")
    u.keys(PHASE, "zr+up")
    played = wait_more("named_playback frames", m, 20)
    done_play = wait_more("named_playback_complete", count("named_playback_complete") - 1, 25)
    row("named playback of the loaded macro runs and completes", played and done_play, (lines("named_playback", 2) or ["none"])[-1][:200])
    u.shot(PHASE, "after-named-playback")

    # ---- cycles again with Macro Source = Named (prepare_buffers ran on the publishing thread) ----------------------------
    done2 = cycles(max(10, CYCLES // 2), "second")
    row(f"menu open/close cycles after the named publish ({done2})", done2 == max(10, CYCLES // 2), (lines(r"\[Ui\] menu closed", 1) or ["none"])[-1])
    fps_sample("end")

    # ---- counters, fps ----------------------------------------------------------------------------------------------------
    stats = lines(r"hook_stats", 2)
    row("hook allocations (hook_stats)", bool(stats), stats[-1][:240] if stats else "no hook_stats line")
    (OUT / "rows.json").write_text(json.dumps(ROWS, indent=1))
    (OUT / "fps.json").write_text(json.dumps(FPS, indent=1))
    print("FPS", json.dumps(FPS[:3]), "...", json.dumps(FPS[-2:]), flush=True)
    print(f"ROWS {sum(r['ok'] for r in ROWS)}/{len(ROWS)} pass", flush=True)


if __name__ == "__main__":
    main()
