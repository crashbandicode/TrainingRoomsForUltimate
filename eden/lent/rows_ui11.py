"""ui-11 rows on the lent Ally rig (Eden in Training via `ui_lent.py route`): the portrait roster grid (Character Cycles with the
Player and CPU lists, Player Character), the install line read whole, and the hook counters. Uses the ui-06 walker and helpers.

    rows_ui11.py live-ui-11
"""
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
import hook_stats  # noqa: E402
sys.path.insert(0, str(HERE.parents[1] / "tools"))
import stack_table  # noqa: E402
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
    w = r6.W.Walk(PHASE, pilot_present=True, developer=True)
    # Exercise the effect-picture path whose old stack exceeded the UI fiber
    # budget. Gate on native texture binding, then retain each rendered grid.
    w.open()
    for title, visual in [("DI Direction", "Di"), ("SDI Direction", "Sdi")]:
        w.focus(title)
        ready = rf"\[Chips\] visual={visual} bound=10 texture=\d+x\d+"
        before_ready = u.count(PHASE, r6.DIAG, ready)
        w.keys("a")
        u.wait_count(PHASE, r6.DIAG, ready, before_ready + 1, 15, f"{visual} effect pointers bound")
        r6.shot(f"effect-chips-{visual.lower()}")
        r6.row(f"{visual} grid binds ten resident effect pointers", True,
               u.plug(PHASE, r6.DIAG, ready, 1)["lines"][-1])
        w.keys("b w8")
    w.keys("b w20")
    # Character Cycles: Mario then Donkey Kong into the Player list, then ZR and Mario into the CPU list.
    w.keys("w10")
    w.open()
    w.page("Fighters & Stages")
    w.focus("Character Cycles")
    w.keys("a w15")
    r6.shot("roster-cycles-open")
    # Reset through the visible All tile (cell 86), then select Mario/DK.
    w.keys(" ".join(["down"] * 6 + ["right"] * 8) + " a w8")
    w.keys(" ".join(["up"] * 6 + ["left"] * 10) + " a w8 right w8 a w8")
    r6.shot("roster-cycles-two-ranked")
    w.keys("zr w8")
    w.keys(" ".join(["down"] * 6 + ["right"] * 8) + " a w8")
    w.keys(" ".join(["up"] * 6 + ["left"] * 10) + " a w8")
    r6.shot("roster-cycles-cpu-list")
    w.keys("b w10 b w20")
    player, cpu = kept("player_roster"), kept("cpu_roster")
    ranks = [(i, r) for i, r in enumerate((player or [1])[1:]) if r]
    r6.row("Character Cycles: Player list ranked in order (player_roster)", player is not None and player[0] == 0 and ranks == [(0, 1), (1, 2)],
           f"all={player[0] if player else None} ranks={ranks}")
    cpu_ranks = [(i, r) for i, r in enumerate((cpu or [1])[1:]) if r]
    r6.row("Character Cycles: ZR edits the CPU list (cpu_roster)", cpu is not None and cpu[0] == 0 and cpu_ranks == [(0, 1)], f"ranks={cpu_ranks[:4]}")
    # Player Character: three steps right from Mario is Samus.
    w.open()
    w.page("Fighters & Stages")
    w.focus("Player Character")
    w.keys("a w15 right w6 right w6 right w6 a w8")
    r6.shot("roster-player-pick")
    w.keys("b w10 b w20")
    pick = kept("player_character")
    picked = [i for i, v in enumerate(pick or []) if v == 1]
    r6.row("Player Character: one fighter picked (player_character one-hot)", picked == [3], f"index={picked}; expected Samus index 3")
    # Ice Climbers is row 1, column 3 of the carried 13-column roster.
    # Each reopening starts at Mario / Player; check the other role is preserved.
    before_player, before_cpu = kept("solo_popo_player_cycle"), kept("solo_popo_cpu")
    player_on = before_player == [1, 0]
    w.open()
    w.focus("Character Cycles")
    w.keys("a w15 down right right right y w8")
    r6.shot("solo-player-toggled")
    w.keys("zr w8")
    r6.shot("solo-cpu-before-toggle")
    w.keys("b w8 b w20")
    after_player, after_cpu = kept("solo_popo_player_cycle"), kept("solo_popo_cpu")
    r6.row("Y Solo Popo changes Player only", after_player == ([0, 1] if player_on else [1, 0]) and after_cpu == before_cpu,
           f"player={after_player} cpu={after_cpu}")
    cpu_on = after_cpu == [1, 0]
    w.open()
    w.focus("Character Cycles")
    w.keys("a w15 down right right right")
    # Role changes synchronously request a fresh portrait page. Wait for its
    # observed ready state before sending the short Y press; a fixed 0.3 s
    # gap was shorter than ui-14's 429 ms portrait load and the press was lost.
    ready = r"\[Thumbs\] bound=86 loading=0"
    before_ready = u.count(PHASE, r6.DIAG, ready)
    w.keys("zr")
    u.wait_count(PHASE, r6.DIAG, ready, before_ready + 1, 15, "CPU roster portraits ready before Solo input")
    w.keys("y w8")
    r6.shot("solo-cpu-toggled")
    w.keys("zr w8")
    r6.shot("solo-player-preserved")
    w.keys("b w8 b w20")
    final_player, final_cpu = kept("solo_popo_player_cycle"), kept("solo_popo_cpu")
    r6.row("Y Solo Popo changes CPU only", final_player == after_player and final_cpu == ([0, 1] if cpu_on else [1, 0]),
           f"player={final_player} cpu={final_cpu}")
    # Read whole lines from the retained file: the plug transport truncates each
    # line, which can hide coverage and the per-hook allocation attribution.
    stats = r6.ps(f"Get-Content '{r6.SD}\\practice-diagnostics.log' | Select-String -Pattern 'hook_stats allocations=|\\[LayoutArchive\\]|\\[ThreadStack\\]|\\[Thumbs\\]' | ForEach-Object {{ $_.Line }}")
    (r6.OUT / "roster-hook-stats.log").write_text(stats, encoding="utf-8")
    ok, reasons = hook_stats.gate(stats, ["ui_archive", "menu_draw"])
    r6.row("hook allocations: every delta and final total zero, archive/draw covered", ok,
           "; ".join(reasons) if reasons else f"{len(hook_stats.parse(stats))} complete samples")
    ok, reasons = hook_stats.gate_layout_archives(stats, ["info_training"])
    r6.row("archive hand-outs allocate zero and retire after at least five seconds", ok,
           "; ".join(reasons) if reasons else "all hand-outs matched by layout/size; grace >=5000ms")
    requests = [int(n) for n in re.findall(r"\[Thumbs\] page .* requested=(\d+)", stats)]
    bound = [int(n) for n in re.findall(r"\[Thumbs\] bound=(\d+)", stats)]
    r6.row("roster requests and binds runtime portrait textures",
           any(requests) and any(bound), f"requested={requests} bound={bound}")
    threads, _, spawns = stack_table.collect(stats.splitlines())
    menu_stack = threads.get("tr-menu")
    # The approved table uses 25% headroom plus 16 KiB SDK reserve,
    # rounded to a 4 KiB page; the compiled static path is 35,600 bytes.
    need = max(35_600, menu_stack["used"]) if menu_stack else 0
    required_stack = ((need + need // 4 + 16_384 + 4095) // 4096) * 4096
    menu_spawns = [size for name, _, size, _ in spawns if name == "tr-menu"]
    r6.row("menu stack retains measured margin on the approved 60 KiB stack",
           bool(menu_stack) and menu_spawns == [61_440]
           and menu_stack["size"] == 61_440 and not menu_stack["low"]
           and menu_stack["size"] >= required_stack,
           f"stack={menu_stack} spawns={menu_spawns} required={required_stack}")
    console = u.get(PHASE, "console.log", r6.OUT)
    unmapped = [line for line in console.read_text(errors="replace").splitlines() if "unmapped" in line.lower()]
    (r6.OUT / "unmapped-lines.log").write_text("\n".join(unmapped), encoding="utf-8")
    r6.row("console has zero Unmapped errors", not unmapped, f"{len(unmapped)} lines; full console retained")
    (r6.OUT / "rows-roster.json").write_text(json.dumps(r6.ROWS, indent=1))
    print("ROWS DONE", flush=True)
    if not all(row["ok"] for row in r6.ROWS):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
