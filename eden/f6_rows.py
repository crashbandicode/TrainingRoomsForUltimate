"""F6 Eden rows (first-boot group-build hitch), run ON the Surface after `route_f6.py <label>` reached Training.

Writes the boot settings with `settings` (before `up`), or drives laps with `laps`: waits until the preload has prefetched the
groups (`[GroupSwap] prefetched`), then presses the P1 cycle chord (ZL + D-pad Right) and, with --cpu, the CPU chord (ZL + D-pad
Left) N times each, every press gated on `[CharacterRoster] complete` (or a cap), with a screenshot after each group build.
Never opens the TrainingRooms menu (known hang on menu close, 2026-11-02): every option comes from the settings file.

Usage: f6_rows.py settings <out.json>
       f6_rows.py laps <label> [--p1 N] [--cpu N] [--settle S]
Results: `eden/runs/<label>/laps.jsonl` (one record per press) and the diag lines it waited on.
"""
import argparse
import json
import re
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

TRIO = HERE.parent / "src/model/swap/trio.rs"
CATALOG = [int(k, 16) for k in re.findall(r"^\s*\(0x([0-9a-f]+), \"", TRIO.read_text(), re.M)]
KIND = {"mario": 0x00, "dk": 0x01, "link": 0x02, "kirby": 0x06, "fox": 0x07, "pikachu": 0x08, "falcon": 0x0b,
        "bowser": 0x0f, "gnw": 0x1c, "lucario": 0x2c, "rob": 0x2d, "toon_link": 0x2e, "dark_pit": 0x1f}

# P1 cycles through an ordinary group of three after the window, then a group led by Kirby (the planner forms the groups in
# cycle order; the plan lines name what it built). CPU: a second ordinary cycle for the CPU role.
P1_ORDER = ("rob", "lucario", "gnw", "kirby", "falcon", "dark_pit", "toon_link")
CPU_ORDER = ("pikachu", "bowser", "dk", "fox", "link")


def roster(*names):
    ranks = [0] * len(CATALOG)
    for rank, name in enumerate(names, 1):
        ranks[CATALOG.index(KIND[name])] = rank
    return [0] + ranks


def settings(out):
    # One-hot option rows (On is option 0): preload 5 slots, Both sources, 3 bank members, 12 ready fighters, Group Swap On,
    # every group class On (the factory values, written out so the first boot does not depend on the INI cache's default).
    on = [1, 0]
    menu = {"trio_practice": on, "player_roster": roster(*P1_ORDER), "cpu_roster": roster(*CPU_ORDER),
            "cycle_preload_slots": [0, 0, 0, 0, 0, 1], "cycle_preload_sources": [0, 0, 1],
            "cycle_preload_bank_members": [0, 0, 1], "cycle_preload_ready_fighters": [0] * 10 + [1],
            "cycle_preload_group_swap": on, "cycle_preload_original_bank": on, "cycle_preload_native_groups": on,
            "cycle_preload_pair_groups": on, "cycle_preload_ic_pair_bank": on, "cycle_preload_ic_pair_groups": on,
            "cycle_preload_mii_groups": on, "cycle_preload_solo_popo_groups": on}
    Path(out).write_text(json.dumps({"menu": menu}), encoding="utf-8")
    print(out, len(CATALOG), "catalog entries")


def laps(args):
    import route_f6 as rt
    sd = rt.SD / "TrainingRooms"
    diag, prev = sd / "practice-diagnostics.log", sd / "practice-diagnostics.previous.log"
    out = HERE / "runs" / args.label
    out.mkdir(parents=True, exist_ok=True)

    def read():
        lines = []
        for p in (prev, diag):
            try:
                lines += p.read_text(encoding="utf-8", errors="replace").splitlines()
            except FileNotFoundError:
                pass
        return lines

    def count():
        return len(read())

    def wait_for(start, needle, cap):
        deadline = time.time() + cap
        while time.time() < deadline:
            for line in read()[start:]:
                if needle in line:
                    return line
            time.sleep(1.0)
        return None

    def shot(name):
        try:
            return str(rt.shot(args.label, name))
        except (SystemExit, Exception) as exc:  # evidence, never a gate
            return f"shot failed: {exc}"

    start = 0
    ready = wait_for(start, "[GroupSwap] prefetched", args.settle)
    print("prefetched:", ready, flush=True)
    records = []
    for role, n in ((0, args.p1), (1, args.cpu)):
        for i in range(n):
            m = count()
            rt.press(args.label, "zl,right" if role == 0 else "zl,left", 200)
            queued = wait_for(m, "[CharacterRoster] queued", 8)
            done = wait_for(m, "[CharacterRoster] complete", 40)
            new = read()[m:]
            built = [x for x in new if "[GroupSwap] built" in x or "[BuildSplit]" in x or "[GroupSwap] swap " in x]
            rec = {"role": role, "press": i + 1, "queued": queued, "done": done, "lines": built,
                   "shot": shot(f"r{role}-p{i + 1}") if any("[GroupSwap] built" in x for x in built) else None}
            records.append(rec)
            print(json.dumps({k: rec[k] for k in ("role", "press", "done")})[:300], flush=True)
            for x in built:
                print("  ", x[:400], flush=True)
            # Let a prefetch of the next group finish before the next press (the planner's own pace).
            wait_for(count(), "[GroupSwap] prefetched", args.gap)
    with (out / "laps.jsonl").open("a", encoding="utf-8") as f:
        for r in records:
            f.write(json.dumps(r) + "\n")
    print("LAPS DONE", len(records), flush=True)


def main():
    p = argparse.ArgumentParser()
    sub = p.add_subparsers(dest="op", required=True)
    s = sub.add_parser("settings")
    s.add_argument("out")
    s = sub.add_parser("laps")
    s.add_argument("label")
    s.add_argument("--p1", type=int, default=8)
    s.add_argument("--cpu", type=int, default=0)
    s.add_argument("--settle", type=float, default=90)
    s.add_argument("--gap", type=float, default=6)
    a = p.parse_args()
    if a.op == "settings":
        settings(a.out)
    else:
        laps(a)


if __name__ == "__main__":
    main()
