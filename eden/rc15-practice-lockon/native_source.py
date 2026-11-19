"""Neutral startup controls for the guarded Palutena BTT route."""
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import lockon_accept as lockon


def observe(ctx, prior):
    import struct
    out = {"attempts": []}
    try:
        last = json.loads(Path(prior).read_text())
        pin = json.loads((Path(__file__).parent / "build-pins.json").read_text())
        teams = lockon.own_team_reads(last["start_trace"]["team"], "nro:" + pin["diag_build_id"] + "+")
        p1, = [address for address, team in teams.items() if team == 0]
        slot, _ = lockon.manager(ctx, "Palutena")
        _, (identity,) = lockon.native.sample(ctx, [(f"{p1 + 8:#x}", 4)], out)
        if int.from_bytes(identity, "little") != 0:
            raise RuntimeError("prior exact P1 actor changed")
        for _ in range(2):
            sample = {}
            out["attempts"].append(sample)
            lockon.routes.fixture(ctx, p1="Palutena", stage="Final Destination", reset=True, lr=1)
            mark = lockon.hs.diag_mark(ctx)
            specs = {
                "damage": dict(target="main+0x4028dc", max_hits=12,
                               registers=["x1", "x2", "x3", "x19", "x24", "x25"],
                               captures=[{"expression": "x24+8", "type": "bytes", "size": 4}]),
                "retirement": dict(target=lockon.REMOVE, max_hits=6, registers=["x1", "x30"]),
                "motion": dict(target="main+0x205cb30", condition=f"x0 == {p1}", max_hits=36,
                               registers=["x0", "x1", "x30"]),
                "status": dict(target="main+0x20876e0", condition=f"x0 == {p1}", max_hits=32,
                               registers=["x0", "x1", "x30"]),
                "bullet": dict(target="main+0x3489f1c", max_hits=6, registers=["x0", "x20"]),
            }
            with lockon.trace(ctx, specs) as (after, arms):
                sample["start"] = lockon.start_btt(ctx)
                ctx.play("neutral-90")
                _, (vector,) = lockon.native.sample(ctx, [(f"[{slot:#x}]+0x28", 16)], out)
                begin, end = struct.unpack("<QQ", vector)
                sample["item_count"] = (end - begin) // 8
                _, sample["trace"] = lockon.read(ctx, after, arms)
            sample["score"] = max(lockon.scores(ctx, "Btt", mark))
            sample["stop"] = lockon.stop(ctx, "Btt")
            if sample["score"]:
                return {"scored": True, "damage": sample["trace"]["damage"],
                        "motion": sample["trace"]["motion"]}
        return {"scored": False, "neutral_attempts": len(out["attempts"])}
    finally:
        (Path(ctx.row_dir) / "native-source.json").write_text(json.dumps(out, indent=2) + "\n")
