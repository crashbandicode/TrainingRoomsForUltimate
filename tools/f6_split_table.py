#!/usr/bin/env python3
"""F6: the group build's game-thread time, from the diag log (`[BuildSplit]` and `[GroupSwap]` lines of `practice-diagnostics*.log`).

    python3 -I tools/f6_split_table.py <log> [<log> ...] [--json out.json]

Prints one Markdown row per asynchronous group build, in log order: build index since boot, owner, group kinds, the carried
`setup_us`, the split's steps (each native call and the plugin's own code), the swap's `max_tick_gap_us` and the largest gaps with
the work of the tick before each (`setup`, `settle`, `poll`). Then the first-of-boot build next to the median of the later ones per
step, which is where a first-use cost shows. Numbers are copied from the log lines; nothing is estimated.
"""
import json
import re
import statistics
import sys

SETUP = re.compile(r"\[BuildSplit\] setup owner=(\d+) group=(\d+) kinds=\[([0-9a-f, ]*)\] build_of_boot=(\d+) carried_setup_us=(\d+) (.*)$")
SETTLE = re.compile(r"\[BuildSplit\] settle owner=(\d+) group=(\d+) settle_of_boot=(\d+) (.*)$")
GAPS = re.compile(r"\[BuildSplit\] gaps owner=(\d+) kind=(0x[0-9a-f]+) ticks=(\d+) top=\[(.*)\]")
SWAP = re.compile(r"\[GroupSwap\] swap owner=(\d+) kind=(0x[0-9a-f]+) request_to_ready_us=(\d+) .* max_tick_gap_us=(\d+) ticks=(\d+)")
STEP = re.compile(r"([a-z_0-9]+)=(\d+)/(\d+)")
SUMMARY = re.compile(r"setup_us=(\d+) total_us=(\d+) largest=(\S+)")
GAP = re.compile(r"t(\d+):(\d+):(\w+)")


def parse_steps(text):
    steps = {name: (int(us), int(calls)) for name, us, calls in STEP.findall(text)}
    m = SUMMARY.search(text)
    summary = {"setup_us": int(m.group(1)), "total_us": int(m.group(2)), "largest": m.group(3)} if m else {}
    return steps, summary


def parse(lines):
    """Builds in log order. A build collects its settlement and the swap/gaps lines that follow for the same owner."""
    builds, open_by_owner = [], {}
    for line in lines:
        if (m := SETUP.search(line)):
            steps, summary = parse_steps(m.group(6))
            b = {"owner": int(m.group(1)), "group": int(m.group(2)),
                 "kinds": [int(k, 16) for k in m.group(3).replace(" ", "").split(",") if k],
                 "build_of_boot": int(m.group(4)), "carried_setup_us": int(m.group(5)), "steps": steps, **summary,
                 "settle": None, "max_tick_gap_us": None, "request_to_ready_us": None, "gaps": []}
            builds.append(b)
            open_by_owner[b["owner"]] = b
        elif (m := SETTLE.search(line)):
            b = open_by_owner.get(int(m.group(1)))
            if b is not None and b["settle"] is None:
                steps, summary = parse_steps(m.group(4))
                b["settle"] = {"steps": steps, **summary}
        elif (m := SWAP.search(line)):
            b = open_by_owner.get(int(m.group(1)))
            if b is not None and b["max_tick_gap_us"] is None:
                b["request_to_ready_us"] = int(m.group(3))
                b["max_tick_gap_us"] = int(m.group(4))
        elif (m := GAPS.search(line)):
            b = open_by_owner.pop(int(m.group(1)), None)
            if b is not None:
                b["gaps"] = [{"tick": int(t), "us": int(us), "after": w} for t, us, w in GAP.findall(m.group(4))]
    return builds


def compare(builds):
    """First build of the boot against the median of the later ones, per step: (step, first_us, later_median_us)."""
    first = [b for b in builds if b["build_of_boot"] == 1]
    later = [b for b in builds if b["build_of_boot"] > 1]
    if not first:
        return []
    names = sorted({n for b in builds for n in b["steps"]})
    rows = []
    for n in names:
        f = first[0]["steps"].get(n, (0, 0))[0]
        values = [b["steps"].get(n, (0, 0))[0] for b in later]
        rows.append((n, f, int(statistics.median(values)) if values else None))
    rows.sort(key=lambda r: -(r[1] - (r[2] or 0)))
    return rows


def table(builds):
    out = ["| # | owner | kinds | carried setup_us | split setup_us | largest step | top steps (us) | max_tick_gap_us | largest gaps (tick:us:after) |",
           "| --- | --- | --- | ---: | ---: | --- | --- | ---: | --- |"]
    for b in builds:
        top = sorted(b["steps"].items(), key=lambda kv: -kv[1][0])[:4]
        steps = ", ".join(f"{n} {us}" for n, (us, _) in top)
        gaps = ", ".join(f"t{g['tick']}:{g['us']}:{g['after']}" for g in b["gaps"][:3])
        kinds = "[" + ", ".join(f"{k:x}" for k in b["kinds"]) + "]"
        out.append(f"| {b['build_of_boot']} | {b['owner']} | {kinds} | {b['carried_setup_us']} | {b.get('setup_us')} | "
                   f"{b.get('largest')} | {steps} | {b['max_tick_gap_us']} | {gaps} |")
    rows = compare(builds)
    if rows:
        out += ["", "| step | first build (us) | later builds, median (us) |", "| --- | ---: | ---: |"]
        out += [f"| {n} | {f} | {'-' if m is None else m} |" for n, f, m in rows]
    return "\n".join(out)


def main(argv):
    paths, out = [], None
    it = iter(argv)
    for a in it:
        if a == "--json":
            out = next(it)
        else:
            paths.append(a)
    lines = []
    for p in paths:
        with open(p, encoding="utf-8", errors="replace") as f:
            lines += f.read().splitlines()
    builds = parse(lines)
    print(table(builds))
    if out:
        with open(out, "w", encoding="utf-8") as f:
            json.dump(builds, f, indent=1)


if __name__ == "__main__":
    main(sys.argv[1:])
