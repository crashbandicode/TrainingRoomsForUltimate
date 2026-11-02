"""Summarise the hitbox visualizer's diag lines (SPEC-core-06 cost and effect-pool rows; eden/EDEN-HITBOX.md cites its output).

    hitbox_stats.py <practice-diagnostics.log>

Reads every `[Hitbox] stats ...` line (one per 600 fighter passes) and prints: lines, passes, the spheres the plugin held at most
(`held_max`), the effect requests the game refused (`failed`, or `refused` in the respawn proof build), spawned / killed / ended
spheres, and the pass cost in ticks (19.2 MHz: average over all passes, the highest single pass, both also in microseconds). Exit 0.
Host test: /usr/bin/python3 -I tests/eden/test_hitbox_stats.py (also run by `cargo test`).
"""
import re
import sys
from pathlib import Path

LINE = re.compile(r"\[Hitbox\] stats (.*)")
TICK_HZ = 19_200_000


def fields(text):
    """Every stats line as a dict of its integer fields."""
    rows = []
    for line in text.splitlines():
        m = LINE.search(line)
        if m:
            rows.append({k: int(v) for k, v in re.findall(r"(\w+)=(\d+)", m.group(1))})
    return rows


def summary(text):
    rows = fields(text)
    if not rows:
        return {"lines": 0}
    passes = sum(r.get("passes", 0) for r in rows)
    ticks = sum(r.get("pass_ticks_avg", 0) * r.get("passes", 0) for r in rows)
    out = {
        "lines": len(rows),
        "passes": passes,
        "held_max": max(r.get("held_max", 0) for r in rows),
        "failed": sum(r.get("failed", 0) + r.get("refused", 0) for r in rows),
        "requests": sum(r.get("requests", 0) for r in rows),
        "spawned": sum(r.get("spawned", 0) for r in rows),
        "killed": sum(r.get("killed", 0) for r in rows),
        "ended": sum(r.get("ended", 0) for r in rows),
        "ticks_avg": ticks // passes if passes else 0,
        "ticks_max": max(r.get("pass_ticks_max", 0) for r in rows),
    }
    out["us_avg"] = round(out["ticks_avg"] * 1e6 / TICK_HZ, 1)
    out["us_max"] = round(out["ticks_max"] * 1e6 / TICK_HZ, 1)
    return out


def main():
    s = summary(Path(sys.argv[1]).read_text(encoding="utf-8", errors="replace"))
    print(" ".join(f"{k}={v}" for k, v in s.items()))


if __name__ == "__main__":
    main()
