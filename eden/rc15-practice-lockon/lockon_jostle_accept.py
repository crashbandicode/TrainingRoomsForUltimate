"""Exact same-boot native jostle state for an isolated Disc Sandbag."""
import json
from pathlib import Path
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
import lockon_accept as lockon


def read_pickup(ctx, accessor, out):
    native = lockon.native
    _, (module_ptr,) = native.sample(ctx, [(f"{accessor + 0xc8:#x}", 8)], out)
    module, = struct.unpack("<Q", module_ptr)
    if not module:
        raise RuntimeError("P1 is missing native ItemModule")
    _, (candidate, held, success) = native.sample(ctx, [
        (f"{module + 0x48:#x}", 8), (f"[{module + 0x18:#x}]+8", 8),
        (f"{module + 0x64:#x}", 1),
    ], out)
    return {"module": module, "candidate": int.from_bytes(candidate, "little"),
            "held": int.from_bytes(held, "little"), "success": bool(success[0])}


def modules(ctx, item, fighter, out):
    def fields(accessor):
        slot = f"[{accessor + 0x118:#x}]"
        return [(f"{accessor + 8:#x}", 4), (f"{accessor + 0x118:#x}", 8),
                (slot, 8), (f"[{slot}]+0x48", 8),
                (f"{slot}+0x70", 8), (f"{slot}+0x20", 4)]
    _, item_raw = lockon.native.sample(ctx, fields(item), out)
    bodies, fighter_raw = lockon.native.sample(ctx, fields(fighter), out)
    result = {}
    for name, raw in (("item", item_raw), ("p1", fighter_raw)):
        ident, jostle, vtable, setter, flags, speed = raw
        result[name] = {
            "id": int.from_bytes(ident, "little"),
            "module": int.from_bytes(jostle, "little"),
            "vtable": int.from_bytes(vtable, "little"),
            "setter": int.from_bytes(setter, "little"),
            "flags_at_70": list(flags), "speed_x": struct.unpack("<f", speed)[0],
        }
    result["bodies"] = bodies
    return result


def verify(ctx):
    """Fresh run: keep an isolated, grounded Sandbag nonphysical and unheld."""
    out = {}
    try:
        ctx_fighter = "Sonic"
        lockon.routes.fixture(ctx, p1=ctx_fighter, stage="Final Destination", reset=True, lr=1)
        slot, loaded = lockon.manager(ctx, ctx_fighter)
        base = int(next(mod for mod in loaded if mod["name"] == "main")["base"], 16)
        out["real_setter"] = base + 0x4784a0
        if lockon.census(ctx, slot, out):
            raise RuntimeError("expected an empty native item list before starting")
        lockon.fx._live_set(ctx, eggsercise=lockon.egg.FREE, eggsercise_break=[0, 0, 1],
                           eggsercise_target=lockon.egg.DISC, eggsercise_speed=[1, 0, 0],
                           practice_cpu=lockon.egg.HIDE, hitbox_vis=[0] * 7)
        lockon.menu.start_game(ctx, "Eggs-ercise", r"\[Eggs\] notice=", cap=20)
        lockon.egg.place(ctx, 24.0, 4.0)
        target, = lockon.census(ctx, slot, out)
        if target["kind"] != lockon.SANDBAG:
            raise RuntimeError("expected an isolated native Sandbag")
        pin = json.loads((Path(__file__).parent / "build-pins.json").read_text())
        specs = {"team": dict(target=lockon.TEAM, max_hits=18, on_return=True,
                              registers=["x0", "x30"])}
        with lockon.trace(ctx, specs) as (after, arms):
            ctx.play("neutral-10")
            _, records = lockon.read(ctx, after, arms)
        teams = out["teams"] = lockon.own_team_reads(records["team"], "nro:" + pin["diag_build_id"] + "+")
        p1, = [accessor for accessor, team in teams.items() if team == 0]
        initial = out["initial"] = modules(ctx, target["accessor"], p1, out)
        if (initial["p1"]["id"] != 0 or initial["item"]["id"] != target["id"]
                or initial["item"]["setter"] != out["real_setter"]):
            raise RuntimeError("native discriminator does not match this item and P1")
        jostle = initial["item"]["module"]
        specs = {
            "registration": dict(target="main+0x4785f0", condition=f"x19 == {jostle}",
                                 max_hits=14, registers=["x8", "x19"]),
            "disable": dict(target="main+0x4784a0", condition=f"x0 == {jostle}",
                            max_hits=14, registers=["x0", "x1", "x30"]),
        }
        with lockon.trace(ctx, specs) as (after, arms):
            lockon.egg.place(ctx, 1.5, 4.0)
            ctx.play("neutral-10")
            near = out["near"] = modules(ctx, target["accessor"], p1, out)
            _, out["jostle"] = lockon.read(ctx, after, arms)
        item, = lockon.census(ctx, slot, out)
        out["near_item"] = item
        geometry = item["geometry"]
        out["p1_displacement"] = near["bodies"][0]["x"] - initial["bodies"][0]["x"]
        if (item["id"] != target["id"] or near["item"]["id"] != target["id"]
                or not 0 < geometry["forward"] <= 2.0 or geometry["vertical"] > 1.0
                or geometry["p1"]["status"] != 0):
            raise RuntimeError(f"isolated item must be beside grounded, waiting P1: {geometry}")
        jostle_trace = out["jostle"]
        if (abs(out["p1_displacement"]) > 0.1 or near["item"]["flags_at_70"][3]
                or not jostle_trace["registration"] or not jostle_trace["disable"]
                or any(int(r["registers"]["x8"], 16) & 0xff for r in jostle_trace["registration"])
                or any(int(r["registers"]["x1"], 16) & 1 for r in jostle_trace["disable"])):
            raise RuntimeError("grounded Sandbag jostled P1 or lacked the native disable proof")
        out["before_still"] = str(ctx.shot("grounded-isolated-disc-before-ordinary-pickup"))
        state = out["pickup_pre"] = read_pickup(ctx, p1, out)
        selector = {"eligible": dict(target="main+0x471cc0", condition=f"x0 == {state['module']}",
                                     max_hits=12, on_return=True, registers=["x0"])}
        with lockon.trace(ctx, selector) as (after, arms):
            ctx.press("a", 100)
            out["after_input"] = lockon.routes.read_state(ctx)
            ctx.play("neutral-90")
            out["pickup_post"] = read_pickup(ctx, p1, out)
            repeat, = lockon.census(ctx, slot, out)
            _, out["pickup_trace"] = lockon.read(ctx, after, arms)
        if (repeat["id"] != target["id"] or any(out[phase][key]
                for phase in ("pickup_pre", "pickup_post") for key in ("candidate", "held", "success"))
                or any(int(r["registers"]["x0"], 16) != 0x50000000
                       for r in out["pickup_trace"]["eligible"] if r["event"] == "return")):
            raise RuntimeError("ordinary grounded A acquired or removed the isolated target")
        out["after_still"] = str(ctx.shot("same-isolated-disc-after-ordinary-pickup"))
        out["stop"] = lockon.stop(ctx, "Eggs")
        if lockon.census(ctx, slot, out):
            raise RuntimeError("Stop left a native practice item")
        return {"item": target["id"], "forward": geometry["forward"],
                "p1_displacement": out["p1_displacement"], "held": False,
                "native_skips": len(jostle_trace["registration"])}
    finally:
        (Path(ctx.row_dir) / "native-jostle.json").write_text(json.dumps(out, indent=2) + "\n")
