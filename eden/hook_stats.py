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


def gate_layout_archives(text, required, *, required_base=None, minimum_handouts=1):
    """Require allocation-free hand-outs and a matching release after >=5 s.

    Per-load accounting is separate from steady-state hook_stats. Retain the
    full diagnostic log through the final retirement before applying this gate.
    B3/reload rows can require a base and repeated hand-outs for each layout.
    """
    if required_base not in (None, "stock", "vanilla") or minimum_handouts < 1:
        raise ValueError("archive evidence needs a known base and positive hand-out count")
    required = set(required)
    handed = re.compile(r"\[LayoutArchive\] handed out (\w+) bytes=(\d+) stock=\d+ base=(\w+) load_allocations=(\d+)")
    released = re.compile(r"\[LayoutArchive\] released (\w+) bytes=(\d+) after=(\d+)ms")
    pending, seen, reasons = {}, {}, []
    for number, line in enumerate(text.splitlines(), 1):
        if "[LayoutArchive]" not in line:
            continue
        if "layout_arc=skipped" in line:
            reasons.append(f"line {number}: archive injection skipped")
        match = handed.search(line)
        if match:
            name, size, base, allocations = match.groups()
            seen[name] = seen.get(name, 0) + 1
            if name in required and required_base is not None and base != required_base:
                reasons.append(f"line {number}: {name} base {base}, expected {required_base}")
            key = (name, int(size))
            pending[key] = pending.get(key, 0) + 1
            if int(allocations):
                reasons.append(f"line {number}: {name} load allocations {allocations}")
        elif "handed out" in line:
            reasons.append(f"line {number}: incomplete hand-out evidence")
        match = released.search(line)
        if match:
            name, size, elapsed = match.groups()
            key = (name, int(size))
            if pending.get(key, 0) == 0:
                reasons.append(f"line {number}: release without matching hand-out")
            else:
                pending[key] -= 1
            if int(elapsed) < 5000:
                reasons.append(f"line {number}: {name} grace only {elapsed}ms")
        elif "released" in line:
            reasons.append(f"line {number}: incomplete release evidence")
    if not seen:
        reasons.append("no archive hand-out in the log (coverage is zero)")
    for name in required:
        if seen.get(name, 0) < minimum_handouts:
            reasons.append(f"{name}: {seen.get(name, 0)} hand-outs, need {minimum_handouts} (insufficient coverage)")
    for (name, size), count in pending.items():
        if count:
            reasons.append(f"{name} bytes={size}: {count} hand-outs not retired")
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
