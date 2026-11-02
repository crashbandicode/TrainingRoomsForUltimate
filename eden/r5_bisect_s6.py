"""R5 bisection in a running boot: reload preload settings and measure the roster_preload allocations per 10 s sample under each.
Usage: r5_bisect_s6.py <label>"""
import re
import time

import rows_s6 as r


def preload_allocs():
    lines = [x for x in r.read_diag() if "hook_stats allocations=" in x]
    if not lines:
        return None
    m = re.search(r"roster_preload:(\d+)", lines[-1])
    return int(m.group(1)) if m else 0


def measure(menu, what, seconds=35):
    r.reload(menu, what)
    time.sleep(12)
    a = preload_allocs()
    time.sleep(seconds)
    b = preload_allocs()
    rate = None if a is None or b is None else (b - a) / seconds
    r.row(f"R5 bisect: {what}", True, f"roster_preload {a} -> {b} over {seconds} s: {rate} per s")


measure({"cycle_preload_slots": [1, 0, 0, 0, 0, 0]}, "slots 0 (planner returns early)")
measure({"cycle_preload_slots": [0, 0, 1, 0, 0, 0], "cycle_preload_sources": [0, 1, 0]}, "slots 2, sources hats only")
measure({"cycle_preload_slots": [0, 0, 1, 0, 0, 0], "cycle_preload_sources": [1, 0, 0]}, "slots 2, sources fighters only")
measure({"cycle_preload_slots": [0, 0, 1, 0, 0, 0], "cycle_preload_sources": [0, 0, 1], "cycle_preload_group_swap": [0, 1]}, "slots 2, both, group swap off")
r.main_write()
