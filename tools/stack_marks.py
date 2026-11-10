#!/usr/bin/python3
"""The diag build's measured stack marks against `model::thread_stacks` (rc.6 cov-crop, primary 2026-11-10 02:59Z).

    python3 -I tools/stack_marks.py <diag log or folder> ... [--table src/model/thread_stacks.rs]

Reads every `[ThreadStack] high_water name=<thread> used=<bytes> size=<bytes> ...` line in the given logs (a folder: every
`*.log` under it) and keeps each thread's largest mark. Exit 1 when a mark passes the row's recorded `measured` (growth the
table has not seen: record the run in the row and size the stack by the rule) or the row's stack itself. The case it exists
for: `tr-menu`'s row said 30,848 B measured while the diag build painted 54,960 B, and the next combo crop overflowed.
"""
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import stack_depth  # noqa: E402

MARK_RE = re.compile(r"\[ThreadStack\] high_water name=(\S+) used=(\d+) size=(\d+)")


def marks(paths):
    """{thread: (used, size, file)}: the largest mark of each thread over every log."""
    files = []
    for path in map(Path, paths):
        files.extend(sorted(path.rglob("*.log")) if path.is_dir() else [path])
    out = {}
    for f in files:
        try:
            text = f.read_text(errors="replace")
        except OSError:
            continue
        for m in MARK_RE.finditer(text):
            name, used, size = m.group(1), int(m.group(2)), int(m.group(3))
            if name not in out or used > out[name][0]:
                out[name] = (used, size, str(f))
    return out


def check(found, rows):
    """Lines of the report and the number of marks past their row."""
    by_name = {name: (size, static_bound, measured) for _, name, size, static_bound, measured, _, _ in rows}
    lines, bad = [], 0
    for name, (used, size, where) in sorted(found.items()):
        if name not in by_name:
            # A stack the table does not own (the boot thread's: `spawn … table=0x0`) is held to its own logged size.
            low = size - used < stack_depth.SDK_RESERVE
            bad += low
            lines.append(f"{name:20s} used={used:7d} no row (its own stack {size}) "
                         + (f"less than the SDK reserve left ({where})" if low else "ok"))
            continue
        stack, static_bound, measured = by_name[name]
        problems = []
        if used > measured:
            problems.append(f"past the recorded measured {measured}")
        if used > stack:
            problems.append(f"past the stack {stack}")
        bad += bool(problems)
        lines.append(f"{name:20s} used={used:7d} measured={measured:7d} stack={stack:7d} logged_size={size:7d} "
                     + ("; ".join(problems) + f" ({where})" if problems else "ok"))
    return lines, bad


def main(argv):
    table = Path(__file__).resolve().parent.parent / "src/model/thread_stacks.rs"
    if "--table" in argv:
        at = argv.index("--table")
        table = Path(argv[at + 1])
        argv = argv[:at] + argv[at + 2:]
    if not argv:
        print(__doc__)
        return 2
    rows = stack_depth.table_rows(table.read_text())
    lines, bad = check(marks(argv), rows)
    print("\n".join(lines) if lines else "no [ThreadStack] high_water lines")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
