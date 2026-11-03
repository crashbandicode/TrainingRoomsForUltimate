"""F6 Eden rows, run on the Eden host after the shared harness route reached Training.

Writes boot settings before launch, or drives laps after group prefetch. Cycle laps require the ordinary-owner
adapter completion matching the current role, ticket and target with its lease released. A fallback bank-ready
event is not completion of a refused request. Bootstrap separately waits for each initial body bank.
Matched performance adds --matched to both bootstrap and laps for strict transactions, expected fighter order,
and six seconds after observed ready (one-second polling). Legacy native baseline selects --native-reload and requires `[CharacterRoster] complete owner=<role>`.
A missing completion ends the lane with a retained failed receipt. Options come from the settings file.

Usage: f6_rows.py settings <out.json>
       f6_rows.py bootstrap <label> --harness <shared eden/harness>
       f6_rows.py laps <label> --harness <shared eden/harness> [--native-reload] [--p1 N] [--cpu N]
Results: `eden/runs/<label>/laps.jsonl` (one record per press) and the diag lines it waited on.
"""
import argparse
import json
import os
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


def native_complete(line, role):
    # The exact-owner adapters also own ordinary native reloads in this build.
    # Their ready event follows physical readiness, publication and lease release.
    adapter = "MiiOrdinaryPlayer" if role == 0 else "MiiOrdinaryCpu"
    return (f"[CharacterRoster] complete owner={role}" in line or
            (f"[{adapter}] ready ticket=" in line and "released=true" in line))


def cycle_request(line, role):
    adapter = "MiiOrdinaryPlayer" if role == 0 else "MiiOrdinaryCpu"
    return re.search(r"\[" + adapter + r"\] (bank switch|submit) ticket=(Ticket \{[^}]+\}) .*?target=(0x[0-9a-f]+)\b", line)


def cycle_result(line, role, request):
    # The bank may become ready on its fallback member while this transaction
    # refuses. Only this request/target and its released adapter receipt count.
    requested = cycle_request(request, role) if request else None
    if requested is None:
        return None
    adapter = "MiiOrdinaryPlayer" if role == 0 else "MiiOrdinaryCpu"
    mode, ticket, target = requested.groups()
    if any(f"[{adapter}] {event} ticket={ticket} " in line
           for event in ("bank switch refused", "bank build refused", "refused")):
        return "refused"
    event = "bank ready" if mode == "bank switch" else "ready"
    if (f"[{adapter}] {event} ticket={ticket} " in line
            and re.search(r"\btarget=" + target + r"\b", line)
            and "released=true" in line):
        return "ready"
    return None


def qualified_boot(lines):
    """Only accept qualification after this boot's latest physical observations.

    Run 39's handed-to-owner line was a consumed loading request, not a submit.
    Requiring the latest session boundary also excludes the previous log file.
    """
    boots = [i for i, line in enumerate(lines) if '[PracticeDiagnostics]' in line and 'session_start;' in line]
    if not boots:
        return None
    boot = boots[-1]
    observations = []
    for role, kind in ((0, '58'), (1, '42')):
        found = [i for i in range(boot + 1, len(lines))
                 if f'[CharacterRoster] observed owner={role} ' in lines[i]]
        if not found or not re.search(rf'kind=0x{kind}\b', lines[found[-1]]):
            return None
        observations.append(found[-1])
    queued = [i for i in range(max(observations) + 1, len(lines))
              if '[MiiAutoInit] catalog read queued ' in lines[i]]
    if not queued:
        return None
    q = queued[-1]
    identity = lines[q].split('catalog read queued ', 1)[1]
    ready = next((i for i in range(q + 1, len(lines))
                  if '[MiiAutoInit] catalog ready;' in lines[i] and lines[i].endswith(identity)), None)
    if ready is None:
        return None
    owners = []
    for role in ('Player', 'Cpu'):
        owner = next((i for i in range(ready + 1, len(lines))
                      if f'[MiiOrdinary{role}] catalog owner qualified; immutable origin retained' in lines[i]), None)
        if owner is None:
            return None
        owners.append(owner)
    def stamp(i):
        return int(lines[i].split()[0])
    return {'boot': lines[boot], 'observations': [lines[i] for i in observations],
            'catalog_queued': lines[q], 'catalog_ready': lines[ready],
            'owners': [lines[i] for i in owners],
            'boot_to_qualification_ms': stamp(max(owners)) - stamp(boot),
            'first_physical_observation_to_qualification_ms': stamp(max(owners)) - stamp(min(observations))}


def laps(args):
    bootstrap = getattr(args, "bootstrap", False)
    matched = getattr(args, "matched", False)
    expected_targets = {0: [0x2d] if bootstrap else [0x2c, 0x1c, 6, 0xb, 0x1f, 0x2e, 0x58, 0x2d, 0x2c, 0x1c, 6],
                        1: [8] if bootstrap else [0xf, 1, 7, 2]}
    if matched and (args.p1 != len(expected_targets[0]) or args.cpu != len(expected_targets[1])):
        raise SystemExit("GATE FAIL: matched route requires bootstrap1+1 or laps11+4")
    # Use the shared, reviewed harness; do not start or claim an instance here.
    harness = Path(args.harness or os.environ.get("TR_HARNESS", str(HERE / "harness")))
    if not (harness / "route.py").is_file():
        raise SystemExit(f"shared harness missing: {harness}")
    sys.path.insert(0, str(harness))
    import params
    import route
    config = params.load()
    session = route.session(args.label, config)
    sd = config.data_dir
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
                if (needle(line) if callable(needle) else needle in line):
                    return line
            time.sleep(1.0)
        return None

    def shot(name):
        try:
            return str(session.t.shot(name))
        except (SystemExit, Exception) as exc:  # evidence, never a gate
            return f"shot failed: {exc}"

    start = 0
    if bootstrap and not args.native_reload and wait_for(0, '[PreloadStatus] delivered text=Some("Swap both once to load")', args.settle) is None:
        raise SystemExit("GATE FAIL: initial Training preload notice was not delivered; no cycle inputs sent")
    if args.native_reload and not any(
            "[CyclePreload] loaded slots=0 " in line and "bank_members=0 group_swap=false" in line
            for line in read()):
        raise SystemExit("GATE FAIL: native baseline preload/bank Off was not observed; no cycle inputs sent")
    if matched and not args.native_reload and not any(
            "[CyclePreload] loaded slots=5 " in line and "bank_members=3 group_swap=true" in line
            for line in read()):
        raise SystemExit("GATE FAIL: matched candidate preload/bank On was not observed")
    if matched and bootstrap:
        for owner, kind in ((0, 0x58), (1, 0x42)):
            observed = wait_for(0, lambda line: re.search(
                rf"\[CharacterRoster\] observed owner={owner} .*?kind=0x{kind:x}\b", line), args.settle)
            if observed is None:
                raise SystemExit(f"GATE FAIL: matched starting fighter owner={owner} kind={kind:#x} missing")
        gate_started = time.monotonic()
        deadline = gate_started + args.settle
        qualification = qualified_boot(read())
        while qualification is None and time.monotonic() < deadline:
            time.sleep(1.0)
            qualification = qualified_boot(read())
        receipt = {'qualified': qualification is not None, 'evidence': qualification,
                   'wait_seconds': time.monotonic() - gate_started,
                   'observed_monotonic': time.monotonic(), 'timeout_seconds': args.settle}
        go = out / 'go.json'
        if go.exists():
            receipt['css_plus_to_qualification_observed_seconds'] = time.monotonic() - json.loads(go.read_text())['monotonic']
        (out / 'bootstrap-qualification.json').write_text(json.dumps(receipt, indent=2) + '\n')
        if qualification is None:
            raise SystemExit('GATE FAIL: current-boot catalog and both ordinary owners unqualified; no cycle inputs sent')
    if not args.native_reload and not bootstrap:
        ready = wait_for(start, "[GroupSwap] prefetched", args.settle)
        if ready is None:
            raise SystemExit("GATE FAIL: no prefetched group; no cycle inputs sent")
        print("prefetched:", ready, flush=True)
    records = []
    for role, n in ((0, args.p1), (1, args.cpu)):
        for i in range(n):
            if bootstrap and wait_for(0, f"[CharacterRoster] observed owner={role}", args.settle) is None:
                raise SystemExit(f"GATE FAIL: bootstrap owner={role} was not observed; no cycle input sent")
            m = count()
            pressed_at = time.monotonic()
            session.press("zl,right" if role == 0 else "zl,left", 200)
            transaction_lap = matched or (not args.native_reload and not bootstrap)
            queued = wait_for(m, (lambda line: cycle_request(line, role)) if transaction_lap
                              else f"[CharacterRoster] queued owner={role}", 8)
            completion = "matching cycle transaction" if transaction_lap else (
                "[CharacterRoster] complete" if args.native_reload else "[BodyBank] ready")
            refused = None
            if transaction_lap:
                result = wait_for(m, lambda line: cycle_result(line, role, queued), 20)
                refused = result if result and cycle_result(result, role, queued) == "refused" else None
                done = result if result and refused is None else None
            else:
                done = wait_for(m, (lambda line: native_complete(line, role)) if args.native_reload
                                else f"{completion} owner={role}", 20)
            ready_observed_at = time.monotonic()
            expected_target = expected_targets[role][i] if matched else None
            if matched and queued:
                actual_target = int(cycle_request(queued, role).group(3), 16)
                if actual_target != expected_target:
                    refused = f"unexpected target={actual_target:#x}, expected={expected_target:#x}"
                    done = None
            bank_ready = None
            if matched and bootstrap and not args.native_reload and done:
                bank_ready = wait_for(m, f"[BodyBank] ready owner={role}", 20)
                if bank_ready is None:
                    refused = "matched bootstrap body bank readiness missing"
                    done = None
            new = read()[m:]
            built = [x for x in new if "[GroupSwap] built" in x or "[BuildSplit]" in x or "[GroupSwap] swap " in x]
            rec = {"phase": "bootstrap" if bootstrap else "lap", "role": role, "press": i + 1, "queued": queued, "done": done, "refused": refused, "lines": built,
                   "shot": shot(f"r{role}-p{i + 1}") if any("[GroupSwap] built" in x for x in built) else None}
            if matched:
                rec.update(expected_target=expected_target, bank_ready=bank_ready,
                           pressed_monotonic=pressed_at, ready_observed_monotonic=ready_observed_at,
                           cadence_seconds=args.gap,
                           cadence_boundary="ready observed by one-second polling")
            records.append(rec)
            # Persist each attempted lap, including a failure, before another input.
            with (out / ("bootstrap.jsonl" if bootstrap else "laps.jsonl")).open("a", encoding="utf-8") as f:
                f.write(json.dumps(rec) + "\n")
            if done is None:
                shot(f"r{role}-p{i + 1}-gate-failed")
                raise SystemExit(f"GATE FAIL: {completion} owner={role} refused: {refused}" if refused else
                                 f"GATE FAIL: {completion} owner={role} missing after cycle")
            print(json.dumps({k: rec[k] for k in ("role", "press", "done")})[:300], flush=True)
            for x in built:
                print("  ", x[:400], flush=True)
            # Let a prefetch of the next group finish before the next press (the planner's own pace).
            if matched:
                time.sleep(max(0.0, args.gap - (time.monotonic() - ready_observed_at)))
            elif not args.native_reload and not bootstrap:
                wait_for(count(), "[GroupSwap] prefetched", args.gap)
    print("BOOTSTRAP DONE" if bootstrap else "LAPS DONE", len(records), flush=True)


def main():
    p = argparse.ArgumentParser()
    sub = p.add_subparsers(dest="op", required=True)
    s = sub.add_parser("settings")
    s.add_argument("out")
    s = sub.add_parser("laps")
    s.add_argument("label")
    s.add_argument("--p1", type=int, default=8)
    s.add_argument("--cpu", type=int, default=0)
    s.add_argument("--settle", type=float, default=15)
    s.add_argument("--gap", type=float, default=6)
    s.add_argument("--harness", help="shared eden/harness directory (or TR_HARNESS)")
    s.add_argument("--native-reload", action="store_true", help="no bank prefetch; require native roster completion")
    s.add_argument("--matched", action="store_true", help="strict matched11+4 route and fixed post-observed-ready cadence")
    s = sub.add_parser("bootstrap", help="enter each role's first body bank before waiting for group prefetch")
    s.add_argument("label")
    s.add_argument("--harness")
    s.add_argument("--settle", type=float, default=20)
    s.add_argument("--native-reload", action="store_true")
    s.add_argument("--matched", action="store_true")
    s.add_argument("--gap", type=float, default=6)
    s.set_defaults(bootstrap=True, p1=1, cpu=1)
    a = p.parse_args()
    if a.op == "settings":
        settings(a.out)
    else:
        laps(a)


if __name__ == "__main__":
    main()
