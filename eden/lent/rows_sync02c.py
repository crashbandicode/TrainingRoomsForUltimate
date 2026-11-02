"""sync-02 part c (same boot): the save-state writer's mailbox (stream/sync decision (a)). Many Saves in a row, each written by the writer
from its own copy, then a Load, with the hook allocation counter read before and after. Waits are gated on diagnostics lines, matched by
their uptime stamp across the current and the rotated log (a rotation mid-run cannot hide a line).

    rows_sync02c.py live-sync-02 [saves]
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
SAVES = int(sys.argv[2]) if len(sys.argv) > 2 else 12
LOGS = ("TrainingRooms/practice-diagnostics.log", "TrainingRooms/practice-diagnostics.previous.log")
SD = r"C:\Eden\c1162-w5-ally\profile\user\sdmc\ultimate\TrainingRooms"
OUT = HERE.parent / "runs" / PHASE.replace("live-", "")
ROWS = []


def row(name, ok, evidence):
    ROWS.append({"row": name, "ok": bool(ok), "evidence": evidence})
    print(f"ROW {'pass' if ok else 'FAIL'} {name}: {evidence}", flush=True)


def stamped(regex, n=50):
    out = []
    for rel in LOGS:
        try:
            got = u.plug(PHASE, rel, regex, n)["lines"]
        except (SystemExit, Exception):  # noqa: BLE001 (the rotated log may not exist yet)
            got = []
        for line in got:
            m = re.match(r"(\d+) ", line)
            if m:
                out.append((int(m.group(1)), line))
    return sorted(out)


def now_ms():
    lines = stamped(".", 1)
    return lines[-1][0] if lines else 0


def wait_after(regex, mark, cap):
    end = time.time() + cap
    while time.time() < end:
        hits = [x for x in stamped(regex, 5) if x[0] > mark]
        if hits:
            return hits[-1]
        time.sleep(0.7)
    return None


def allocations():
    stats = stamped(r"hook_stats allocations=", 3)
    if not stats:
        return None, ""
    m = re.search(r"allocations=(\d+) delta=(\d+)", stats[-1][1])
    return (int(m.group(1)) if m else None), stats[-1][1][:120]


def main():
    a0, s0 = allocations()
    written = 0
    for i in range(SAVES):
        mark = now_ms()
        u.ps(f"Set-Content -Path '{SD}\\diag-save.req' -Value 'x'", 60)
        hit = wait_after("file_written", mark, 10)
        if hit:
            written += 1
        else:
            print(f"save {i + 1}: no file_written after {mark}", flush=True)
    row(f"{SAVES} Saves in a row, each written by the writer thread from its own copy", written == SAVES, f"file_written {written}/{SAVES}")
    body = u.ps(f"Get-Content -Raw '{SD}\\save_states.json'", 60)
    try:
        doc = json.loads(body)
        player = doc.get("player", [{}])[0]
        ok = player.get("fighter_kind", -1) >= 0
        detail = f"slot 1 player fighter_kind={player.get('fighter_kind')} x={player.get('x')} marker={player.get('marker', player.get('state'))}"
    except ValueError as error:
        ok, detail = False, f"unparsable: {error}"
    row("save_states.json holds the captured slot (the writer's copy)", ok, detail)
    mark = now_ms()
    u.ps(f"Set-Content -Path '{SD}\\diag-load.req' -Value 'x'", 60)
    hit = wait_after(r"state load_request|state kill|state cleanup", mark, 10)
    row("Load State after the saves (the state machine kept its own file)", hit is not None, hit[1][:160] if hit else "no load line")
    time.sleep(12)  # one hook_stats period (10 s) after the last save and the load
    a1, s1 = allocations()
    row("hook allocations across the saves and the load (hook_stats)", a0 is not None and a1 == a0, f"before {s0} | after {s1}")
    st = u.ops(PHASE, [["status"]])[0].get("result", {})
    row("game still running", st.get("running") and st.get("game_fps", 0) > 55, f"game_fps={st.get('game_fps')}")
    (OUT / "rows-c.json").write_text(json.dumps(ROWS, indent=1))
    print(f"ROWS {sum(x['ok'] for x in ROWS)}/{len(ROWS)} pass", flush=True)


if __name__ == "__main__":
    main()
