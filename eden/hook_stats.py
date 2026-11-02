"""Gate for the diag build's `hook_stats` lines (review R5 of package 1).

The diag build logs, sparsely and after the hooks have worked (never from a hook), lines like

    [TrainingRooms] hook_stats allocations=0 delta=0 hook_calls=412 hooks=input_caller_a_apply:100,reset_check:3

A hook lane passes only when every hook it exercises shows a nonzero invocation count (the lane really
ran the hook) and no line reports an allocation (`delta` is 0 on every line and the last `allocations`
total is 0). A run without any `hook_stats` line fails the gate: no coverage, no evidence.

Usage: hook_stats.py <practice-diagnostics.log> <hook> [<hook> ...]     (exit 0 = gate passed)
Host test: /usr/bin/python3 -I tests/eden/test_analyze.py   (also run by `cargo test`)
"""
import re
import sys
from pathlib import Path

LINE = re.compile(r"\[TrainingRooms\] hook_stats allocations=(\d+) delta=(\d+) hook_calls=(\d+) hooks=(\S+)")


def parse(text):
    """[{allocations, delta, hook_calls, hooks: {name: count}}] in file order."""
    rows = []
    for line in text.splitlines():
        m = LINE.search(line)
        if not m:
            continue
        hooks = {}
        if m.group(4) != "none":
            for item in m.group(4).split(","):
                name, _, count = item.rpartition(":")
                hooks[name] = int(count)
        rows.append({"allocations": int(m.group(1)), "delta": int(m.group(2)),
                     "hook_calls": int(m.group(3)), "hooks": hooks})
    return rows


def gate(text, required):
    """(ok, reasons): every required hook has a nonzero count on the last line and no line allocated."""
    rows = parse(text)
    if not rows:
        return False, ["no hook_stats line in the log (the lane is not covered)"]
    reasons = []
    for i, row in enumerate(rows):
        if row["delta"] != 0:
            reasons.append(f"line {i + 1}: allocation delta {row['delta']}")
    if rows[-1]["allocations"] != 0:
        reasons.append(f"allocations total {rows[-1]['allocations']} at the last line")
    last = rows[-1]["hooks"]
    for hook in required:
        if last.get(hook, 0) <= 0:
            reasons.append(f"hook {hook} has no invocations (coverage is zero)")
    return not reasons, reasons


def main():
    log, required = Path(sys.argv[1]), sys.argv[2:]
    ok, reasons = gate(log.read_text(encoding="utf-8", errors="replace"), required)
    rows = parse(log.read_text(encoding="utf-8", errors="replace"))
    print(f"hook_stats lines={len(rows)} last={rows[-1] if rows else None}")
    for reason in reasons:
        print(f"FAIL: {reason}")
    print("PASS" if ok else "FAIL")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
