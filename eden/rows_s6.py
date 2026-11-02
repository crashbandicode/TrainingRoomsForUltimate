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



def r_boot_cache():
    """Run before any reload: explicit JSON Off survives the boot/cache path."""
    lines = [line for line in read_diag() if "[CyclePreload] loaded " in line]
    loaded = lines[-1] if lines else ""
    menu = json.loads((SD / "settings.json").read_text())["menu"]
    cache = (SD / "cycle_preload.ini").read_text()
    (OUT / "boot-cycle-preload.ini").write_text(cache)
    (OUT / "boot-settings.json").write_text(json.dumps({"menu": menu}, indent=2))
    active_off = bool(re.search(r"\bslots=0(?:\s|$)", loaded) and
                      re.search(r"\bbank_members=0(?:\s|$)", loaded))
    cache_off = bool(re.search(r"^slots=0$", cache, re.M) and
                     re.search(r"^bank_members=0$", cache, re.M))
    json_off = menu.get("cycle_preload_slots") == [1, 0, 0, 0, 0, 0] and \
        menu.get("cycle_preload_bank_members") == [1, 0, 0]
    row("boot JSON Off remains active and is mirrored to INI", active_off and cache_off and json_off, loaded)


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
    m, _ = reload({"cycle_preload_slots": [0, 0, 1, 0, 0, 0]}, "preload on")
    applied = wait_for(m, "[CyclePreload] applied", 10)
    row("menu commit reaches the preload engine (publish_applied)", applied is not None, applied)
    status = wait_for(m, "[PreloadStatus]", 60)
    row("preload on Player 1 plans and reports", status is not None, status)


def r_swap():
    m, queued, done = cycle(0)
    shot("swap-p1-1")
    row("P1 hot swap by the cycle chord", done is not None, f"{queued} | {done}")
    # Native Training hot swaps need not retire the practice context. Lobby
    # lease ownership is a separate paired-run row coordinated with the lead.


def r_kirby():
    # P1 is Kirby after the first swap (roster mario, kirby, ...): the hat shortcut is Special + left stick click.
    m = mark()
    tap("b,lstick", hold=200, wait=1.0)
    req = wait_for(m, "[KirbyHats] request", 20)
    conf = wait_for(m, "[KirbyHats] confirmed", 40)
    shot("kirby-hat")
    row("Kirby hat by the shortcut", req is not None and conf is not None, f"{req} | {conf}")
    saved = save_state()
    m2 = mark()
    load_state()
    restore = wait_for(m2, "[KirbyHats]", 40)
    shot("kirby-hat-restored")
    row("save/load gives the hat back through the hat engine (owner not in the match)", saved is not None and restore is not None, restore)


def r_ic():
    m, queued, done = cycle(0, cap=60)
    shot("swap-ice-climbers")
    row("P1 hot swap to the Ice Climbers pair", done is not None and "kind=0x4b " in done, done)
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


def set_cpus(target, current):
    """Native pause menu: open (the cursor starts on No. of CPUs), step to `target`, close. Returns the mark before the close."""
    tap("plus", wait=2.0)
    for _ in range(abs(target - current)):
        tap("right" if target > current else "left", wait=0.8)
    m = mark()
    tap("plus", wait=2.0)
    return m


def r_f9():
    """F9: with a helper source loaded (preload on), raise No. of CPUs 1 -> 2 -> 3 and back to 1; every extra CPU shows its native
    fighter (HUD and body), the helper is given back before it is shown, and a helper is admitted again at 1."""
    before = [x for x in read_diag() if "[CharacterSources] install slot=" in x]
    row("F9 precondition: a helper source was loaded into a parked entry", bool(before), before[-1:] if before else "no install line")
    m0 = mark()
    m = set_cpus(2, 1)
    time.sleep(4)
    seen = [x for x in after(m0) if "cpu_quantity=2" in x]
    gb = [x for x in after(m0) if "give_back" in x]
    shot("f9-cpus2")
    row("F9 count 2: the loader callback sees it (paused: before the close)", bool(seen), seen[:2])
    row("F9 count 2: the helper in entry 2 is given back to its native fighter", any("give_back done slot=2" in x for x in gb), gb[-3:])
    m0 = mark()
    set_cpus(3, 2)
    time.sleep(4)
    gb = [x for x in after(m0) if "give_back" in x]
    shot("f9-cpus3")
    row("F9 count 3: entry 3 given back too (or held no helper)", all("refused" not in x for x in gb), gb[-3:] or "no helper in entry 3")
    m0 = mark()
    set_cpus(1, 3)
    time.sleep(20)
    again = [x for x in after(m0) if "[CharacterSources] install slot=" in x]
    shot("f9-cpus1")
    row("F9 back to 1: a helper is admitted again", bool(again), again[:2])


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


def r_reload_alive():
    """A diag settings reload, then the log must keep going (stack st-11 went silent after one)."""
    m, line = reload({"cycle_preload_slots": [0, 0, 0, 1, 0, 0]}, "reload liveness")
    time.sleep(12)
    later = [x for x in after(m) if "hook_stats" in x or "hook_counts" in x]
    row("diag settings reload: the log keeps going afterwards", line is not None and len(later) >= 1, f"{line} | {len(later)} periodic lines after")


def r_record():
    """P1 moves, the record chord (ZR + D-pad Down) records a slot, the playback chord (ZR + D-pad Up) plays it."""
    m = mark()
    tap("zr,down", wait=0.1)
    time.sleep(0.8)
    rt.stick(LABEL, 1, 0, 600)
    tap("a", hold=200, wait=0.2)
    rec = wait_for(m, "recording_complete", 25)
    row("recording: a slot is recorded", rec is not None, rec)
    m = mark()
    tap("zr,up", wait=0.15)
    start = wait_for(m, "playback_shortcut", 10)
    time.sleep(1.5)
    shot("playback")
    row("recording: the playback chord plays it", start is not None, start)


def r_menu_cycles():
    """Last rows of a boot (known menu-close hang class): open and close the TrainingRooms menu three times (Minus, then B)."""
    results = []
    for i in range(3):
        m = mark()
        tap("minus", wait=1.5)
        opened = wait_for(m, "[Ui] menu opened", 8)
        m = mark()
        tap("b", wait=2.0)
        closed = wait_for(m, "[Ui] menu closed", 8)
        time.sleep(2)
        alive = wait_for(m, "hook_stats", 15)
        results.append((opened is not None, closed is not None, alive is not None))
    shot("after-menu-cycles")
    row("menu open/close x3, the game keeps running after each close", all(all(r) for r in results), results)


def r_pt():
    m, queued, done = cycle(0, cap=60)
    shot("swap-pt")
    row("P1 hot swap to Pokemon Trainer (native group)", done is not None and "kind=0x24 " in done, done)


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
    # Ring rotation can remove the sample that first reported an allocation.
    # The cumulative total must still be zero; a later zero delta cannot erase it.
    stats = [re.search(r"allocations=(\d+) delta=(\d+)", x) for x in lines]
    row("R5: no hook allocations across the boot", bool(stats) and all(
        match and int(match[1]) == 0 and int(match[2]) == 0 for match in stats),
        lines[-1] if lines else "none")


def r_r5_soak():
    """R5 acceptance: explicit fighters+hats, >=180 s of live zero-allocation samples."""
    m, applied = reload({"cycle_preload_slots": [0, 0, 1, 0, 0, 0],
                         "cycle_preload_sources": [0, 0, 1],
                         "cycle_preload_bank_members": [0, 0, 1]}, "R5 fighters+hats soak")
    plan = wait_for(m, "[CyclePreload] applied", 15)
    configured = applied is not None and plan is not None and "slots=2 fighters=true hats=true" in plan and "bank_members=3" in plan
    row("R5 fighters+hats settings applied", configured, plan)
    if not configured:
        return
    samples = {}
    deadline = time.monotonic() + 225
    while time.monotonic() < deadline:
        for line in after(m):
            if "hook_stats allocations=" in line:
                samples[stamp_of(line)] = line
        stamps = sorted(samples)
        if len(stamps) >= 2 and stamps[-1] - stamps[0] >= 180_000:
            break
        time.sleep(2)
    ordered = [samples[t] for t in sorted(samples)]
    (OUT / "r5-soak-samples.txt").write_text("\n".join(ordered) + "\n")
    stats = [re.search(r"allocations=(\d+) delta=(\d+) hook_calls=(\d+)", line) for line in ordered]
    span = max(samples) - min(samples) if samples else 0
    live = len(stats) >= 2 and all(x and int(x[3]) > 0 for x in stats)
    zero = bool(stats) and all(x and int(x[2]) == 0 for x in stats)
    preload_calls = []
    for line in ordered:
        hooks = re.search(r" hooks=(\S+)", line)
        counts = dict(re.findall(r"([^,:]+):(\d+)", hooks[1] if hooks else ""))
        preload_calls.append(int(counts.get("roster_preload", "0")))
    preload_live = bool(preload_calls) and preload_calls[0] > 0 and all(
        b > a for a, b in zip(preload_calls, preload_calls[1:]))
    gaps = [b - a for a, b in zip(sorted(samples), sorted(samples)[1:])]
    row("R5 >=3 minutes live fighters+hats with every allocation delta zero",
        span >= 180_000 and live and preload_live and zero and max(gaps, default=0) <= 25_000,
        f"samples={len(stats)} span_ms={span} live={live} preload_live={preload_live} all_deltas_zero={zero} max_gap_ms={max(gaps, default=0)}")
    # Preserve each cumulative hook counter as well; no scope is omitted from attribution.
    counters = []
    for line in ordered:
        field = re.search(r" allocs=(\S+)", line)
        counters.append(dict((name, int(value)) for name, value in
                             re.findall(r"([^,:]+):(\d+)", field[1] if field else "")))
    reached = set()
    for line in ordered:
        field = re.search(r" hooks=(\S+)", line)
        if field:
            reached.update(name for name, _ in re.findall(r"([^,:]+):(\d+)", field[1]))
    names = sorted(reached | set().union(*(c.keys() for c in counters)))
    deltas = {name: counters[-1].get(name, 0) - counters[0].get(name, 0) for name in names} if counters else {}
    (OUT / "r5-hook-deltas.json").write_text(json.dumps(deltas, indent=2) + "\n")
    row("R5 every named hook counter unchanged throughout soak", live and all(v == 0 for v in deltas.values()), deltas)


ROWS = {"install": r_install, "bootcache": r_boot_cache, "itemtable": r_itemtable, "item": r_item, "preload": r_preload, "swap": r_swap, "kirby": r_kirby,
        "ic": r_ic, "solo": r_solo, "solo2": r_solo2, "pause": r_pause, "f9": r_f9, "reloadalive": r_reload_alive, "record": r_record, "menu": r_menu_cycles, "lease": r_lease, "r5split": r_r5_split, "popo": r_popo_path, "pt": r_pt, "mii": r_mii, "cpu": r_cpu, "r5": r_r5, "r5soak": r_r5_soak}


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
