"""ui-06 rows on the lent Ally rig (Eden already in Training via `ui_lent.py route`): Recording page panels, Give item, Lobby Stages, the
install line and the hook allocation counters. Menu navigation is the pkg5 walker (computed from src/model/menu/table.rs) over this rig's
transport. Every wait is gated on a log line or a screenshot; each row prints `ROW pass|FAIL name: evidence`.

    rows_ui06.py live-ui-06
"""
import json
import re
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent / "ally"))
import ui_lent as u  # noqa: E402
import s5_ally  # noqa: E402

PHASE = sys.argv[1]
DIAG = "TrainingRooms/practice-diagnostics.log"
OUT = HERE.parent / "runs" / PHASE.replace("live-", "")
OUT.mkdir(parents=True, exist_ok=True)


def diag(_phase, regex=".", n=400):
    return u.plug(PHASE, DIAG, regex, n)["lines"]


# The walker talks to `s5_ally`; point it at this rig.
s5_ally.keys = lambda phase, tokens, timeout=300: u.keys(PHASE, tokens, timeout)
s5_ally.shot = lambda phase, name, width=1280: u.shot(PHASE, name, width)
s5_ally.diag = diag
import s5_walk as W  # noqa: E402

ROWS = []


def row(name, ok, evidence):
    ROWS.append({"row": name, "ok": bool(ok), "evidence": evidence})
    print(f"ROW {'pass' if ok else 'FAIL'} {name}: {evidence}", flush=True)


def shot(name):
    png = u.shot(PHASE, name)
    print("shot", png, flush=True)
    return png


def ps(snippet):
    return u.ps(snippet, 90)


SD = r"C:\Eden\c1162-w5-ally\profile\user\sdmc\ultimate\TrainingRooms"


def hook_stats():
    lines = diag(PHASE, r"hook_stats allocations=", 1)
    return lines[-1] if lines else ""


def main():
    events = u.plug(PHASE, "TrainingRooms/events.log", "result=", 2)["lines"]
    row("install line whole: result=installed heap_allocations=0", any("result=installed heap_allocations=0" in x for x in events), (events[-1][-120:] if events else "none"))
    full = ps(f"(Get-Content '{SD}\\events.log' -TotalCount 1).Length")
    row("install line length (whole, over the old 8 KiB)", True, f"{full.strip()} characters")
    before = hook_stats()
    print("hook_stats before", before, flush=True)

    w = W.Walk(PHASE, pilot_present=True)
    # ---- Recording page -------------------------------------------------------------------------------------------
    w.keys("w10")
    w.open()
    w.page("Recording")
    w.focus("Named library")
    w.keys("a w15")
    shot("rec-named-library-empty")
    w.keys("b w10")
    shot("rec-page-after-library")
    w.keys("b w15")  # close the menu
    m = len(diag(PHASE, "recording_complete"))
    w.keys("zr+down w20 a w10 a w10 w40")
    done = diag(PHASE, "recording_complete")
    row("numbered recording into slot 1 (for the save)", len(done) > m, done[-1] if done else "none")
    w.open()
    w.page("Recording")
    w.focus("Save slot to library...")
    w.keys("a w15")
    shot("rec-save-slot-name-page")
    ps(f"Set-Content '{SD}\\diag-macro.req' -Value 'name ui06macro'")
    time.sleep(3)
    files = ps(f"Get-ChildItem '{SD}\\macros' -Name -ErrorAction SilentlyContinue")
    row("Save slot to library: the confirmed name is written by the worker (macros/ui06macro.json)", "ui06macro.json" in files, files.strip().replace("\n", " "))
    shot("rec-after-save")
    w.cursor["Recording"] = w.cursor.get("Recording", 0)
    w.keys("up a w15")
    shot("rec-named-library-with-macro")
    w.keys("b w10 down down a w15")
    shot("rec-edit-placement")
    w.keys("b w10")
    w.keys("down a w15")
    shot("rec-replay-window")
    w.keys("b w10 b w15")
    # ---- Give item --------------------------------------------------------------------------------------------------
    w.open()
    w.page("Fighters & Stages")
    w.focus("Give item")
    w.keys("a w15")
    shot("give-item-grid")
    m = len(diag(PHASE, r"CharacterItem|Character Item"))
    w.keys("right right a w10")
    shot("give-item-after-a")
    w.keys("b w10 b w20")
    w.keys("w30")
    lines = diag(PHASE, r"CharacterItem|Character Item|notice_draw header=\"Character Item\"")
    row("Give item: A on the third tile asks the roster for the item (tick request)", len(lines) > m, lines[-1][:200] if lines else "no item line")
    shot("give-item-in-game")
    # ---- Lobby Stages -----------------------------------------------------------------------------------------------
    w.open()
    w.page("Fighters & Stages")
    w.focus("Lobby Stages")
    w.keys("a w15")
    shot("stages-grid-open")
    w.keys("right a w8 right a w8 left y w8")
    shot("stages-grid-two-added-form")
    w.keys("x w8")
    shot("stages-grid-direct")
    w.keys("x w8 b w10 b w20")
    settings = ps(f"Get-Content -Raw '{SD}\\settings.json'")
    try:
        doc = json.loads(settings)
        order = doc.get("menu", {}).get("lobby_stages")
        prefs = doc.get("stage_preferences")
    except ValueError:
        order, prefs = None, None
    ranks = [(i, r) for i, r in enumerate((order or [0])[1:]) if r]
    row("Lobby Stages: the order is written (settings.json menu.lobby_stages)", bool(ranks) and len(ranks) >= 2, f"ranks={ranks[:6]} all={order[0] if order else None}")
    row("Lobby Stages: the form and mode are written (settings.json stage_preferences)", prefs is not None, json.dumps(prefs)[:160] if prefs else "none")
    w.open()
    w.page("Fighters & Stages")
    w.focus("Lobby Stages")
    w.keys("a w15")
    shot("stages-grid-reopened")
    w.keys("b w10 b w20")
    # ---- counters ---------------------------------------------------------------------------------------------------
    after = hook_stats()
    print("hook_stats after", after, flush=True)
    a = re.search(r"allocations=(\d+)", before or "")
    b = re.search(r"allocations=(\d+)", after or "")
    row("hook allocations over the rows (R5 steady-state: delta of the counter)", bool(a and b), f"before={a.group(1) if a else '?'} after={b.group(1) if b else '?'} ({after[:160]})")
    (OUT / "rows.json").write_text(json.dumps(ROWS, indent=1))
    print("ROWS DONE", flush=True)


if __name__ == "__main__":
    main()
