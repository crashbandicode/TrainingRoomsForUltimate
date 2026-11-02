"""ui-11 rows on the lent Ally rig (Eden in Training via `ui_lent.py route`): the portrait roster grid (Character Cycles with the
Player and CPU lists, Player Character), the install line read whole, and the hook counters. Uses the ui-06 walker and helpers.

    rows_ui11.py live-ui-11
"""
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
import hook_stats  # noqa: E402
PHASE = sys.argv[1]
sys.argv = [sys.argv[0], PHASE]
import ui_lent as u  # noqa: E402
import rows_ui06 as r6  # noqa: E402


def kept(key):
    body = r6.ps(f"Get-Content -Raw '{r6.SD}\\settings.json'")
    try:
        return json.loads(body).get("menu", {}).get(key)
    except ValueError:
        return None


def main():
    tail = r6.ps(f"$l = (Get-Content '{r6.SD}\\events.log' | Select-String -Pattern 'result=' | Select-Object -Last 1).Line; "
                 "$l.Substring([Math]::Max(0, $l.Length - 160))").strip()
    r6.row("install line whole: ends result=installed heap_allocations=0", "result=installed heap_allocations=0" in tail, tail[-120:] or "none")
    w = r6.W.Walk(PHASE, pilot_present=True)
    # Character Cycles: Mario then Donkey Kong into the Player list, then ZR and Mario into the CPU list.
    w.keys("w10")
    w.open()
    w.page("Fighters & Stages")
    w.focus("Character Cycles")
    w.keys("a w15")
    r6.shot("roster-cycles-open")
    w.keys("a w8 right w8 a w8")
    r6.shot("roster-cycles-two-ranked")
    w.keys("zr w8 a w8")
    r6.shot("roster-cycles-cpu-list")
    w.keys("b w10 b w20")
    player, cpu = kept("player_roster"), kept("cpu_roster")
    ranks = [(i, r) for i, r in enumerate((player or [1])[1:]) if r]
    r6.row("Character Cycles: Player list ranked in order (player_roster)", player is not None and player[0] == 0 and len(ranks) == 2,
           f"all={player[0] if player else None} ranks={ranks}")
    cpu_ranks = [(i, r) for i, r in enumerate((cpu or [1])[1:]) if r]
    r6.row("Character Cycles: ZR edits the CPU list (cpu_roster)", cpu is not None and len(cpu_ranks) >= 1, f"ranks={cpu_ranks[:4]}")
    # Player Character: three steps right from Mario is Samus.
    w.open()
    w.page("Fighters & Stages")
    w.focus("Player Character")
    w.keys("a w15 right w6 right w6 right w6 a w8")
    r6.shot("roster-player-pick")
    w.keys("b w10 b w20")
    pick = kept("player_character")
    picked = [i for i, v in enumerate(pick or []) if v == 1]
    r6.row("Player Character: one fighter picked (player_character one-hot)", len(picked) == 1, f"index={picked}")
    # Read whole lines from the retained file: the plug transport truncates each
    # line, which can hide coverage and the per-hook allocation attribution.
    stats = r6.ps(f"Get-Content '{r6.SD}\\practice-diagnostics.log' | Select-String -Pattern 'hook_stats allocations=' | ForEach-Object {{ $_.Line }}")
    (r6.OUT / "roster-hook-stats.log").write_text(stats, encoding="utf-8")
    ok, reasons = hook_stats.gate(stats, ["ui_archive", "menu_draw"])
    r6.row("hook allocations: every delta and final total zero, archive/draw covered", ok,
           "; ".join(reasons) if reasons else f"{len(hook_stats.parse(stats))} complete samples")
    (r6.OUT / "rows-roster.json").write_text(json.dumps(r6.ROWS, indent=1))
    print("ROWS DONE", flush=True)
    if not all(row["ok"] for row in r6.ROWS):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
