"""Guard the Edge Ready rows G9 and G10, corrected (tr-primary-2 ruling 2026-11-23 ~04:15Z).

The rows used to compare the bridge's `fighters` y during Ready with a whole number. That is the wrong quantity: the
game's physics keeps running under the hold, so the bridge samples the held CPU one step of its gravity below the
point, and the picked point is not the whole number the notice shows
(/home/intpa/smash-training-lobby/artifacts/phase23/edge-juggle/ROOT-CAUSE-g9-ready-y-drop.md).

What is compared now, with no tolerance and no rounding:
  * the picked point, as the plugin publishes it at the wizard's Preview (`tr.practice`: `wizard.x`, `wizard.y`);
  * the CPU's held position, as the plugin reads it at its Ready-to-Live boundary (the release trace's first
    `edge_juggle_attempt` record with event 0 after the start input, same boot).
Both f32 words of the two must be identical. x of both fighters, Player 1 standing and the damage are checked from the
bridge's `fighters` exactly as before.

`tr.practice` prints each f32 with Rust's shortest round-trip decimal, so `struct.pack('<f', float(text))` gives the
plugin's own f32 back bit for bit. That conversion is exact; it is not a rounding of the comparison.

Not flown. `words`, `picked` and `problems` are host-tested (test_edge_ready_exact.py) on the records of run
rc15-ib-1f7e0174-yoga-main-1.
"""
import json
import struct
import time
from pathlib import Path

PICKED_FILE = "edge-picked-point.json"


def words(point):
    """The f32 words of a published point (two decimal texts), in the trace reader's `0x%08x` form."""
    return ["0x%08x" % struct.unpack("<I", struct.pack("<f", float(value)))[0] for value in point]


def picked(block):
    """The Preview's picked points from one complete `tr.practice` block (CPU and Player 1)."""
    values = block.get("values", {})
    if block.get("schema") != "tr.practice/1" or values.get("phase") != "wizard" or values.get("wizard.step") != "preview":
        raise RuntimeError(f"expected the wizard's Preview in tr.practice; saw {values!r}")
    keys = ("wizard.x", "wizard.y", "wizard.player.x", "wizard.player.y")
    missing = [key for key in keys if key not in values]
    if missing:
        raise RuntimeError(f"tr.practice Preview lacks {missing!r}")
    cpu, player = [values[keys[0]], values[keys[1]]], [values[keys[2]], values[keys[3]]]
    return {"boot": block.get("boot"), "game": values.get("game"), "cpu": cpu, "cpu_words": words(cpu),
            "p1": player, "p1_words": words(player)}


def problems(point, release, bodies, partner, player_x, cpu_x, damage, *, juggler=False):
    """Every way the Ready differs from the picked start (empty: the row passes).

    `point`: `picked()`'s result. `release`: `trace_attempts.attempt()` of the first event-0 record after the start.
    `bodies`: the bridge's `fighters` read during Ready. `partner`: 1 in Training, 4 in a waiting room.
    """
    found = []
    if release["event"] != 0 or release["juggler"] != juggler:
        found.append(f"unexpected drill release record for {point['game']!r}: {release!r}")
    if release["cpu_words"] != point["cpu_words"]:
        found.append(f"held CPU {release['cpu_words']} {release['cpu']} is not the picked point "
                     f"{point['cpu_words']} {point['cpu']}")
    if release["damage"] != damage:
        found.append(f"CPU damage at release {release['damage']!r}, expected {damage!r}")
    p1, other = bodies.get(0, {}), bodies.get(partner, {})
    if abs(p1.get("x", float("inf")) - player_x) > .02:
        found.append(f"Player 1 x {p1.get('x')!r}, expected {player_x!r}")
    if p1.get("situation") != 0:
        found.append(f"Player 1 situation {p1.get('situation')!r}, expected 0 (standing)")
    if abs(other.get("x", float("inf")) - cpu_x) > .02:
        found.append(f"CPU x {other.get('x')!r}, expected {cpu_x!r}")
    if other.get("dmg") != damage:
        found.append(f"CPU dmg {other.get('dmg')!r}, expected {damage!r}")
    return found


def _practice(ctx, accept, cap=6):
    block, _ = ctx.runner._state_block(ctx.on, "tr.practice", cap, accept)
    return block


def save_picked(ctx, *, keep=True):
    """G9, at the wizard's Preview and before the start input: keep the picked points for this boot."""
    def accept(block):
        values = block.get("values", {})
        ok = values.get("phase") == "wizard" and values.get("wizard.step") == "preview" and "wizard.player.y" in values
        return ok, f"{values.get('phase')!r} {values.get('wizard.step')!r}"
    point = picked(_practice(ctx, accept))
    if keep:
        (Path(ctx.runner.out) / PICKED_FILE).write_text(json.dumps(point, indent=2) + "\n")
    ctx.log({"edge_picked": point})
    return point


def load_picked(ctx):
    """G10 (the remembered start has no Preview): the points G9 kept, which must be from this boot."""
    path = Path(ctx.runner.out) / PICKED_FILE
    if not path.exists():
        raise RuntimeError(f"no picked point from G9 in this run: {path}")
    point = json.loads(path.read_text())
    now = _practice(ctx, lambda block: (block.get("schema") == "tr.practice/1", "any block"))
    if now.get("boot") != point["boot"]:
        raise RuntimeError(f"the kept picked point is from boot {point['boot']!r}; this is boot {now.get('boot')!r}")
    return point


def open_trace(ctx):
    """Immediately before the start input: fence the release trace so only this start's records are read."""
    import routes
    import trace_attempts
    path = ctx.instance.get("trace_jsonl")
    if not isinstance(path, str) or not path:
        raise RuntimeError("G9/G10 need the release trace of this boot (edge_juggle_attempt armed)")
    stream = trace_attempts.Stream(path, "")
    fence = routes._trace_snapshot(ctx, path)
    stream.point = fence["attempt"]["id"]
    return {"path": path, "stream": stream, "fence": fence}


def first_release(ctx, watch, cap=12):
    """The first event-0 `edge_juggle_attempt` record after the fence: the plugin's read at Ready-to-Live."""
    import routes
    until = time.monotonic() + cap
    earlier = []
    while time.monotonic() < until:
        ctx.check_stop()
        for mark in watch["stream"].read(until):
            if mark["sequence"] <= watch["fence"]["sequence"] or mark["record"] != "attempt":
                continue
            if mark["event"] != 0:
                earlier.append({key: mark[key] for key in ("sequence", "frame", "event", "terminal")})
                continue
            end = routes._trace_snapshot(ctx, watch["path"], expected=watch["fence"])
            routes._trace_delta(watch["fence"], end)
            return mark, earlier
        time.sleep(.05)
    raise RuntimeError(f"no release record within {cap} s of the start (records before it: {earlier!r})")


def check_ready(ctx, scene, point, watch, player_x, cpu_x, damage, *, bodies=None, juggler=False):
    """After `tr.menu` reads Ready: the bridge's bodies as before, then the exact held-point comparison."""
    import routes
    import switch_hw
    if bodies is None:
        run = routes.instance_path(ctx, "bridge_log").parent
        bodies = switch_hw.bridge_state(run, ("fighters",), cap=3)["fighters"]
    release, earlier = first_release(ctx, watch)
    partner = 4 if scene == "lobby" else 1
    found = problems(point, release, bodies, partner, player_x, cpu_x, damage, juggler=juggler)
    proof = {"edge_ready_exact": {"picked": point, "release": release, "records_before_release": earlier,
                                  "fighters_during_ready": bodies, "problems": found}}
    ctx.log(proof)
    (Path(ctx.row_dir) / "edge-ready-exact.json").write_text(json.dumps(proof, indent=2, default=str) + "\n")
    if found:
        raise RuntimeError("Ready does not match the picked start: " + "; ".join(found))
    return proof
