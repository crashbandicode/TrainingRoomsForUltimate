"""ui-10 rows on the lent Ally rig (Eden already in Training via `ui_lent.py route`): the menu-open allocation attribution (lead,
2026-11-02: u4b saw 30-40 hook allocations per open), then the ui-06 rows (Recording page panels, Give item, Lobby Stages, counters).

    rows_ui10.py live-ui-10 [cycles]
"""
import json
import re
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
PHASE = sys.argv[1]
CYCLES = int(sys.argv[2]) if len(sys.argv) > 2 else 8
sys.argv = [sys.argv[0], PHASE]
import ui_lent as u  # noqa: E402
import rows_ui06 as r6  # noqa: E402

DIAG = "TrainingRooms/practice-diagnostics.log"
OUT = HERE.parent / "runs" / PHASE.replace("live-", "")
OUT.mkdir(parents=True, exist_ok=True)


def stats():
    lines = u.plug(PHASE, DIAG, r"hook_stats allocations=", 1)["lines"]
    if not lines:
        return None, {}, ""
    line = lines[-1]
    total = re.search(r"allocations=(\d+)", line)
    allocs = dict((k, int(v)) for k, v in re.findall(r"(\w+):(\d+)", line.split(" allocs=", 1)[1])) if " allocs=" in line else {}
    return (int(total.group(1)) if total else None), allocs, line


def count(regex):
    return u.count(PHASE, DIAG, regex)


def wait_more(regex, before, cap):
    end = time.time() + cap
    while time.time() < end:
        if count(regex) > before:
            return True
        time.sleep(0.5)
    return False


def main():
    st = u.plug(PHASE, DIAG, r"\[Sync\] arbiter self_test", 1)["lines"]
    r6.row("hos-lock arbiter self-test", bool(st) and "ok=true" in st[-1], st[-1] if st else "no line")
    time.sleep(11)  # one hook_stats period, so the "before" line is current
    t0, a0, l0 = stats()
    done = 0
    for _ in range(CYCLES):
        o, c = count(r"\[Ui\] menu opened"), count(r"\[Ui\] menu closed")
        u.keys(PHASE, "b+up")
        if not wait_more(r"\[Ui\] menu opened", o, 10):
            break
        u.keys(PHASE, "w10 b")
        if not wait_more(r"\[Ui\] menu closed", c, 10):
            break
        done += 1
        u.keys(PHASE, "w10")
    time.sleep(11)
    t1, a1, l1 = stats()
    per_scope = {k: a1.get(k, 0) - a0.get(k, 0) for k in set(a0) | set(a1) if a1.get(k, 0) != a0.get(k, 0)}
    per_open = (t1 - t0) / done if done and t0 is not None and t1 is not None else None
    r6.row(f"menu open/close x{done}: hook allocations per open, by scope", done == CYCLES,
           f"total {t0} -> {t1} ({per_open} per open); by scope {json.dumps(per_scope)}")
    (OUT / "attribution.json").write_text(json.dumps({"cycles": done, "before": l0, "after": l1, "per_scope": per_scope}, indent=1))
    r6.main()


if __name__ == "__main__":
    main()
