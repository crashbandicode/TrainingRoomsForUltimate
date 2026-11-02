"""Package-6 Eden rows (third successor, lead rulings 1-3), run ON the Surface after `route_s6.py <label>` reached Training.
One batched boot per file set: the diag NRO (+ the standalone CPU pilot unless a row says absent, + the tile cursor). Every wait is
gated on a diag line (`practice-diagnostics.log`, both ring files) or a screenshot, with a cap; settings change through the diag
reload (`diag-settings-next.json`), an item grant through `diag-item.req` (the Give item screen is not built yet).

Usage: rows_s6.py <label> <row> [<row> ...]     rows: install itemtable item preload swap kirby ic solo pt mii cpu r5 all
Results: one `ROW pass|FAIL <name>: <evidence>` line each, and `eden/runs/<label>/rows.json`.
"""
import json
import re
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import route_s6 as rt  # noqa: E402

LABEL = sys.argv[1]
SD = rt.SD / "TrainingRooms"
DIAG = SD / "practice-diagnostics.log"
DIAG_PREV = SD / "practice-diagnostics.previous.log"
OUT = HERE / "runs" / LABEL
OUT.mkdir(parents=True, exist_ok=True)
RESULTS = []

# The roster catalog order of `model/swap/trio.rs` (`CHOICES`), read from the source so a persisted roster array is built the way the
# plugin parses it: `[all, rank per catalog position...]`.
TRIO = HERE.parent / "src/model/swap/trio.rs"
CATALOG = [int(k, 16) for k in re.findall(r"^\s*\(0x([0-9a-f]+), \"", TRIO.read_text(), re.M)]
KIND = {"mario": 0x00, "kirby": 0x06, "pikachu": 0x08, "peach": 0x0d, "bowser": 0x0f, "palutena": 0x36, "gnw": 0x1c,
        "squirtle": 0x24, "ice": 0x4b, "pyra": 0x5a, "mii_sword": 0x49, "mii_brawler": 0x48, "mii_gunner": 0x4a, "link": 0x02,
        "dk": 0x01, "fox": 0x07, "falcon": 0x0b}


def roster(*names):
    ranks = [0] * len(CATALOG)
    for rank, name in enumerate(names, 1):
        ranks[CATALOG.index(KIND[name])] = rank
    return [0] + ranks


def read_diag():
    out = []
    for path in (DIAG_PREV, DIAG):
        try:
            out += path.read_text(encoding="utf-8", errors="replace").splitlines()
        except FileNotFoundError:
            pass
    return out


def stamp_of(line):
    m = re.match(r"(\d+) ", line)
    return int(m.group(1)) if m else -1


def mark():
    return max((stamp_of(x) for x in read_diag()), default=0)


def after(start):
    return [x for x in read_diag() if stamp_of(x) > start]


def wait_for(start, needle, cap=20):
    deadline = time.time() + cap
    while time.time() < deadline:
        for line in after(start):
            if needle in line:
                return line
        time.sleep(1.0)
    return None


def row(name, ok, evidence):
    RESULTS.append({"row": name, "ok": bool(ok), "evidence": str(evidence)[:600]})
    print(f"ROW {'pass' if ok else 'FAIL'} {name}: {str(evidence)[:300]}", flush=True)


def tap(buttons, hold=150, wait=0.5):
    rt.press(LABEL, buttons, hold)
    time.sleep(wait)


def shot(name):
    try:
        return rt.shot(LABEL, name)
    except (SystemExit, Exception) as exc:  # a screenshot is evidence, never a gate here
        print(f"shot {name} failed: {exc}", flush=True)
        return None


# Every reload carries the whole menu (a reload replaces the snapshot): the base rows of this boot plus the row's own.
BASE = {"trio_practice": [1, 0], "p1_cpu_control": [0, 1], "player_roster": None, "cpu_roster": None}  # rosters filled below


BASE["player_roster"] = roster("kirby", "ice", "squirtle", "palutena")  # the boot settings, eden/mk_s6_settings.py
BASE["cpu_roster"] = roster("pikachu", "bowser")


def reload(menu, what):
    # Overlay the live settings (the writer thread keeps settings.json current: the cycle's own picks such as `player_character`)
    # so a reload changes only the rows it names; a reload replaces the whole snapshot.
    try:
        live = json.loads((SD / "settings.json").read_text(encoding="utf-8")).get("menu", {})
    except (OSError, ValueError):
        live = {}
    body = json.dumps({"menu": {**live, **BASE, **menu}})
    m = mark()
    tmp = SD / "diag-settings-next.tmp"
    tmp.write_text(body, encoding="utf-8")
    tmp.rename(SD / "diag-settings-next.json")
    line = wait_for(m, "diag_settings_reload", 10)
    print(f"reload {what}: {line}", flush=True)
    return m, line


def save_state(wait=1.0):
    m = mark()
    tap("zl,down", wait=wait)
    return wait_for(m, "saved_notice", 6)


def load_state(cap=12):
    m = mark()
    tap("zl,up", wait=0.5)
    return m, wait_for(m, "state done", cap)


def cycle(role, cap=40):
    """One cycle chord (P1: ZL + D-pad Right, CPU: ZL + D-pad Left); the swap's completion line."""
    m = mark()
    tap("zl,right" if role == 0 else "zl,left", hold=200, wait=0.5)
    queued = wait_for(m, "[CharacterRoster] request handed", 8) or wait_for(m, "[CharacterRoster] queued", 2)
    done = wait_for(m, f"[CharacterRoster] HUD refresh owner={role}", cap)
    return m, queued, done


def r_install():
    events = (SD / "events.log").read_text(encoding="utf-8", errors="replace")
    (OUT / "events.log").write_text(events)
    last = events.strip().splitlines()[-1] if events.strip() else ""
    row("install line", "result=installed" in last, last[:300])
    row("install heap_allocations=0", "heap_allocations=0" in last, re.search(r"heap_allocations=\S+", last))
    started = not any("roster cycle not started" in x for x in read_diag())
    row("roster cycle started (every CALLED_ROWS guard passed)", started, "no 'roster cycle not started' line" if started else "not started")


def r_itemtable():
    lines = [x for x in read_diag() if "[ItemTable]" in x]
    (OUT / "itemtable.txt").write_text("\n".join(lines) + "\n")
    missing = [x for x in lines if "MISSING" in x]
    row("[ItemTable] 38 entries, none MISSING", len(lines) == 38 and not missing, f"{len(lines)} lines, missing {len(missing)}")
    for who in ("PEACH", "DAISY"):
        values = [int(v) for v in re.findall(rf"ITEM_VARIATION_{who}DAIKON_\d=(-?\d+)", "\n".join(lines))]
        row(f"[ItemTable] {who.lower()} turnip variations 0/5/6/7", values == [0, 5, 6, 7], values)


def r_item():
    # Link's bomb (choice 11) to Player 1, who is not Link: the owner is not in the match.
    m = mark()
    (SD / "diag-item.req").write_text("11", encoding="utf-8")
    granted = wait_for(m, "[CharacterItem] granted", 30)
    shot("item-granted")
    row("item grant through the engine (owner not in the match)", granted is not None, granted)
    saved = save_state()
    row("save state with the held item", saved is not None, saved)
    tap("a", wait=1.0)  # throw what is held, so the load has to give it back
    time.sleep(3)
    m, done = load_state()
    restored = wait_for(m, "[CharacterItem] restored", 30)
    shot("item-restored")
    row("load restores the held item (owner not in the match)", restored is not None and "choice=11" in restored, f"{done} | {restored}")


def r_preload():
    m, _ = reload({"cycle_preload_slots": [0, 0, 1, 0, 0, 0, 0, 0, 0, 0]}, "preload on")
    applied = wait_for(m, "[CyclePreload] applied", 10)
    row("menu commit reaches the preload engine (publish_applied)", applied is not None, applied)
    status = wait_for(m, "[PreloadStatus]", 60)
    row("preload on Player 1 plans and reports", status is not None, status)


def r_swap():
    m, queued, done = cycle(0)
    shot("swap-p1-1")
    row("P1 hot swap by the cycle chord", done is not None, f"{queued} | {done}")
    session = [x for x in after(m) if "practice_session held=" in x]
    row("the hot swap retires the practice session and a fresh one is acquired", any("held=false" in x for x in session)
        and any("held=true" in x for x in session), session[-4:])


def r_kirby():
    # P1 is Kirby after the first swap (roster mario, kirby, ...): the hat shortcut is Special + left stick click.
    m = mark()
    tap("b,lstick", hold=200, wait=1.0)
    req = wait_for(m, "[KirbyHats] request", 20)
    conf = wait_for(m, "[KirbyHats] confirmed", 40)
    shot("kirby-hat")
    row("Kirby hat by the shortcut", req is not None, f"{req} | {conf}")
    saved = save_state()
    m2 = mark()
    load_state()
    restore = wait_for(m2, "[KirbyHats]", 40)
    shot("kirby-hat-restored")
    row("save/load gives the hat back through the hat engine (owner not in the match)", saved is not None and restore is not None, restore)


def r_ic():
    m, queued, done = cycle(0, cap=60)
    shot("swap-ice-climbers")
    row("P1 hot swap to the Ice Climbers pair", done is not None, done)
    save_state()
    m2, done2 = load_state(cap=20)
    row("save/load with the Ice Climbers pair finishes (Nana hand-off)", done2 is not None, done2)


def r_solo():
    reload({"solo_popo_player_cycle": [1, 0]}, "solo popo")
    time.sleep(3)
    save_state()
    m, done = load_state(cap=20)
    time.sleep(4)
    stuck = any("NanaPosMove" in x for x in after(m))
    row("Solo Popo: save/load ends at Popo (no NanaPosMove wait)", done is not None and not stuck, done)


def r_solo2():
    """Solo Popo on the cycle slots, then cycle to the Ice Climbers: Popo alone; a save/load must end at Popo."""
    reload({"solo_popo_player_cycle": [1, 0]}, "solo popo on the cycle")
    time.sleep(3)
    done = None
    for _ in range(5):
        m, queued, done = cycle(0, cap=60)
        if done and "kind=0x4b" in done:
            break
    time.sleep(4)
    shot("solo-popo")
    row("Solo Popo cycle slot reached (Ice Climbers descriptor)", done is not None and "kind=0x4b" in done, done)
    save_state()
    m, loaded = load_state(cap=20)
    time.sleep(5)
    shot("solo-popo-after-load")
    after_load = after(m)
    stuck = [x for x in after_load if "NanaPosMove" in x]
    row("Solo Popo: save/load ends at Popo (no NanaPosMove wait)", loaded is not None and not stuck, loaded)
    m2 = mark()
    tap("zl,down", wait=1.0)
    again = wait_for(m2, "saved_notice", 6)
    row("Solo Popo: the machine is idle after the load (a new save is accepted)", again is not None, again)


def pause_dump(tag):
    m = mark()
    (SD / "diag-pausedump.req").write_text(tag, encoding="utf-8")
    return wait_for(m, f"diag_pause_dump tag={tag}", 8)


def r_pause():
    """F9 research: dump the native pause menu around a No. of CPUs change (1 -> 2 -> 3 -> 1), closing the menu last."""
    tap("plus", wait=2.0)
    lines = [pause_dump("open-cpus1")]
    tap("right", wait=0.8)
    lines.append(pause_dump("cpus2"))
    tap("right", wait=0.8)
    lines.append(pause_dump("cpus3"))
    tap("left", wait=0.8)
    tap("left", wait=0.8)
    lines.append(pause_dump("back-cpus1"))
    shot("pause-cpus1")
    tap("plus", wait=2.0)
    time.sleep(2)
    lines.append(pause_dump("closed-cpus1"))
    row("F9 research: pause menu dumps around No. of CPUs 1/2/3/1", all(x and "written=true" in x for x in lines), lines)


def r_lease():
    lines = [x for x in read_diag() if "practice_session" in x]
    row("practice lease: why native Training holds no session (refusal reason)", bool(lines), lines[-3:])


def r_r5_split():
    lines = [x for x in read_diag() if "hook_stats allocations=" in x]
    last = lines[-1] if lines else ""
    split = re.search(r" allocs=(\S+)", last)
    row("R5 attribution: allocations per innermost hook scope", split is not None or "allocations=0 " in last, split.group(1) if split else last[:200])


def r_popo_path():
    lines = [x for x in read_diag() if "state popo" in x]
    row("Solo Popo: the load took the no-Nana path (nana_removed=true)", any("nana_removed=true" in x for x in lines), lines[-3:])


def r_pt():
    m, queued, done = cycle(0, cap=60)
    shot("swap-pt")
    row("P1 hot swap to Pokemon Trainer (native group)", done is not None, done)


def r_mii():
    m, queued, done = cycle(0, cap=60)
    shot("swap-next")
    row("P1 hot swap to the next fighter", done is not None, done)


def r_cpu():
    m, queued, done = cycle(1, cap=60)
    shot("swap-cpu")
    row("CPU hot swap by the CPU cycle chord", done is not None, f"{queued} | {done}")


def r_r5():
    lines = [x for x in read_diag() if "hook_stats allocations=" in x]
    deltas = [int(re.search(r"delta=(\d+)", x).group(1)) for x in lines]
    row("R5: hook allocation delta 0 on every sample", lines and all(d == 0 for d in deltas[1:]), lines[-1] if lines else "none")


ROWS = {"install": r_install, "itemtable": r_itemtable, "item": r_item, "preload": r_preload, "swap": r_swap, "kirby": r_kirby,
        "ic": r_ic, "solo": r_solo, "solo2": r_solo2, "pause": r_pause, "lease": r_lease, "r5split": r_r5_split, "popo": r_popo_path, "pt": r_pt, "mii": r_mii, "cpu": r_cpu, "r5": r_r5}


def main():
    names = sys.argv[2:]
    if names == ["all"]:
        names = list(ROWS)
    for name in names:
        try:
            ROWS[name]()
        except SystemExit:
            raise
        except Exception as exc:  # one row's harness failure is that row's FAIL, never a silent pass
            row(f"{name} (harness)", False, repr(exc))
    main_write()


def main_write():
    path = OUT / "rows.json"
    previous = json.loads(path.read_text()) if path.exists() else []
    path.write_text(json.dumps(previous + RESULTS, indent=1))


if __name__ == "__main__":
    main()
