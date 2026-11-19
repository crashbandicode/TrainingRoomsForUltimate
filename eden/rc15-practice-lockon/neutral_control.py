"""Neutral startup controls for the guarded Palutena BTT route."""
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import lockon_accept as lockon


def verify(ctx):
    out = {}
    try:
        slot, _ = lockon.manager(ctx, "Palutena")
        out["fixture"] = lockon.routes.fixture(ctx, p1="Palutena",
                                              stage="Final Destination", reset=True, lr=1)
        if lockon.census(ctx, slot, out):
            raise RuntimeError("pre-existing native item at BTT start")
        mark = lockon.hs.diag_mark(ctx)
        points = {
            "search": lockon.egg.SEARCH_POINT,
            "contact": dict(target="main+0x4028dc", max_hits=16,
                            registers=["x1", "x3", "x24", "x30"]),
            "retirement": dict(target=lockon.REMOVE, max_hits=8,
                               registers=["x0", "x1", "x30"]),
        }
        with lockon.trace(ctx, points) as (after, arms):
            out["start"] = lockon.start_btt(ctx)
            ctx.play("neutral-90")
            items = out["items"] = lockon.census(ctx, slot, out)
            _, out["trace"] = lockon.read(ctx, after, arms)
        out["scores"] = lockon.scores(ctx, "Btt", mark)
        out["stop"] = lockon.stop(ctx, "Btt")
        if len(items) != 2 or any(item["kind"] != lockon.SANDBAG for item in items):
            raise RuntimeError("neutral startup did not preserve both native item IDs")
        if not out["scores"] or set(out["scores"]) != {0} or any(out["trace"].values()):
            raise RuntimeError("neutral startup had a score, native search, hit or retirement")
        return {"score": 0, "item_ids": [item["id"] for item in items], "native_contacts": 0}
    finally:
        (Path(ctx.row_dir) / "neutral-control.json").write_text(json.dumps(out, indent=2) + "\n")
