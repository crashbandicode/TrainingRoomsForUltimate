#!/usr/bin/python3
"""The stack numbers of the stack stream's evidence, from the diag build's logs (stream/stack, 2026-11-02).

Reads `practice-diagnostics*.log` files (any number of runs) and prints:
  * per thread: the table size, the largest `used=` of its `[ThreadStack] high_water` / `exit` lines, the least `free=`, and
    whether any line said `low=true`;
  * per hook or callback site: calls, the least `min_free=` of its `[HookStack]` lines and the stack region size seen there;
  * every `[ThreadStack] spawn` line's base/size, for matching a fault's SP to a thread.

    python3 -I tools/stack_table.py <diag log>...            Markdown tables
    python3 -I tools/stack_table.py --spawns <diag log>...   the spawn lines (name base size top)
"""
import re
import sys

THREAD_RE = re.compile(r"\[ThreadStack\] (high_water|exit) name=(\S+) used=(\d+) size=(\d+) free=(\d+) low=(true|false)")
SPAWN_RE = re.compile(r"\[ThreadStack\] spawn name=(\S+) base=(0x[0-9a-f]+) size=(0x[0-9a-f]+) top=(0x[0-9a-f]+)")
HOOK_RE = re.compile(r"\[HookStack\] site=(\S+) calls=(\d+) min_free=(\d+) stack=(0x[0-9a-f]+) base=(0x[0-9a-f]+)")


def collect(lines):
    threads, hooks, spawns = {}, {}, []
    for line in lines:
        m = THREAD_RE.search(line)
        if m:
            _, name, used, size, free, low = m.groups()
            t = threads.setdefault(name, {"size": int(size), "used": 0, "free": None, "low": False, "lines": 0})
            t["used"] = max(t["used"], int(used))
            t["free"] = int(free) if t["free"] is None else min(t["free"], int(free))
            t["low"] |= low == "true"
            t["lines"] += 1
            continue
        m = HOOK_RE.search(line)
        if m:
            site, calls, free, stack, _ = m.groups()
            h = hooks.setdefault(site, {"calls": 0, "min_free": None, "stack": 0})
            h["calls"] = max(h["calls"], int(calls))
            if h["min_free"] is None or int(free) < h["min_free"]:
                h["min_free"], h["stack"] = int(free), int(stack, 16)
            continue
        m = SPAWN_RE.search(line)
        if m:
            name, base, size, top = m.groups()
            spawns.append((name, int(base, 16), int(size, 16), int(top, 16)))
    return threads, hooks, spawns


def tables(threads, hooks):
    out = ["| Thread | Stack (B) | High-water used (B) | Least free (B) | low |", "| --- | --- | --- | --- | --- |"]
    for name in sorted(threads):
        t = threads[name]
        out.append(f"| {name} | {t['size']} | {t['used']} | {t['free']} | {'yes' if t['low'] else 'no'} |")
    out += ["", "| Hook / callback | Calls | Least free at entry (B) | Stack region (B) |", "| --- | --- | --- | --- |"]
    for site in sorted(hooks, key=lambda s: hooks[s]["min_free"]):
        h = hooks[site]
        out.append(f"| {site} | {h['calls']} | {h['min_free']} | {h['stack']} |")
    return "\n".join(out)


def main(argv):
    spawns_only = "--spawns" in argv
    paths = [a for a in argv if a != "--spawns"]
    lines = []
    for path in paths:
        with open(path, encoding="utf-8", errors="replace") as f:
            lines += f.read().splitlines()
    threads, hooks, spawns = collect(lines)
    if spawns_only:
        for name, base, size, top in spawns:
            print(f"{name} base={base:#x} size={size:#x} top={top:#x}")
    else:
        print(tables(threads, hooks))


if __name__ == "__main__":
    main(sys.argv[1:])
