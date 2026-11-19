
import json
import struct
import time
from pathlib import Path

from PIL import Image

import eggs_hw as egg
import fx
import routes

KINDS = {"Sandbag": 0x8, "Soccer Ball": 0x5d}
MAIN_BUILD = "21450c647b8c5940ed5b68218ea128fd3c5d597d000000000000000000000000"


def point(ctx, target, captures, limit=1):
    spec = {"target": target, "max_hits": limit,
            "captures": [{"expression": exp, "type": "bytes", "size": size} for exp, size in captures]}
    return str(ctx.call("trace.arm", spec)["id"])


def captured(ctx, after, ident, raw):
    until = time.monotonic() + 3
    while time.monotonic() < until:
        pages, records = egg._read_trace(ctx, after)
        raw.extend(record for record in records if str(record.get("point_id")) == ident)
        if raw:
            return [bytes.fromhex(c["hex"]) if "hex" in c else b"" for c in raw[-1]["captures"]]
        time.sleep(.05)
    raise RuntimeError(f"Missing trace capture {ident}")


def sample(ctx, captures, out):
    after = egg._last_trace_sequence(egg._trace_file(ctx))
    ident = point(ctx, "main+0x206ae10", captures)
    try:
        body = routes.read_state(ctx)
        records = []
        values = captured(ctx, after, ident, records)
        out.setdefault("native_samples", []).append(records[-1])
        return body, values
    finally:
        ctx.call("trace.disarm", {"id": ident})


def item_vector(ctx, manager_slot, out):
    _, (vector,) = sample(ctx, [(f"[{manager_slot:#x}]+0x28", 16)], out)
    begin, end = struct.unpack("<QQ", vector)
    count = (end - begin) // 8
    if count not in (0, 1) or end != begin + 8 * count:
        raise RuntimeError(f"Expected at most one active item, found vector {begin:#x}..{end:#x}")
    return begin, count


def item_layout(ctx, slot, expected_kind, out):
    _, (obj, posture) = sample(ctx, [(f"[{slot:#x}]", 32), (f"[[{slot:#x}]+0xd0]", 16)], out)
    object_id, kind = struct.unpack_from("<II", obj, 8)
    if object_id >> 28 != 4 or kind != expected_kind or not posture:
        raise RuntimeError(f"Item identity mismatch (id={object_id:#x}, kind={kind:#x})")
    posture_ptr, = struct.unpack_from("<Q", posture, 8)
    if not posture_ptr:
        raise RuntimeError("Posture owner accessor is absent")
    # Item accessor is Item + 0x98 (20f4470), PostureModule slot is +0x38
    # (206ade8). Read the native pos getter at vtable+0x60.
    _, (instr,) = sample(ctx, [(f"[[[[{slot:#x}]+0xd0]+0]+0x60]", 8)], out)
    load, ret = struct.unpack("<II", instr)
    if ret != 0xd65f03c0:
        raise RuntimeError(f"Posture getter is not a leaf: {instr.hex()}")
    posture_expr = f"[[{slot:#x}]+0xd0]"
    if load & 0xffc003ff == 0x91000000:
        position = f"{posture_expr}+{(load >> 10) & 0xfff}"
    elif load & 0xffc003ff == 0xf9400000:
        position = f"[{posture_expr}+{8 * ((load >> 10) & 0xfff)}]"
    else:
        raise RuntimeError(f"Unrecognized position getter: {instr.hex()}")
    return object_id, position


def geometry(ctx, slot, position, out):
    bodies, (xyz, obj) = sample(ctx, [(position, 12), (f"[{slot:#x}]+8", 8)], out)
    px = bodies[0]
    x, y, z = struct.unpack("<fff", xyz)
    value = {"p1": px, "cpu": bodies[1], "item": [x, y, z],
             "id_kind": list(struct.unpack("<II", obj)),
             "forward": (x - px["x"]) * px["lr"], "vertical": abs(y - px["y"])}
    out.setdefault("geometry", []).append(value)
    return value


def setup(ctx, fighter):
    fx._live_set(ctx, eggsercise=egg.OFF, btt=[1, 0], hide_cpu=egg.SHOW, hitbox_vis=[0] * 7)
    fixture = routes.fixture(ctx, p1=fighter, stage="Final Destination", reset=True, lr=1)
    mark = egg._last_trace_sequence(egg._trace_file(ctx))
    spec = {"target": "main+0x203db20", "condition": "x1 == 3", "max_hits": 1,
            "registers": ["x0", "x1", "x30"]}
    ident = str(ctx.call("trace.arm", spec)["id"])
    try:
        fx._live_set(ctx, hide_cpu=egg.HIDE)
        _, records = egg._read_trace(ctx, mark)
        records = [r for r in records if str(r.get("point_id")) == ident]
        if len(records) != 1 or int(records[0]["registers"]["x1"], 16) != 3:
            raise RuntimeError("No native hurtbox-off trace for the hidden CPU")
    finally:
        ctx.call("trace.disarm", {"id": ident})
    return {"fixture": fixture, "hurtbox_off": records[0]}


def fighter_select(ctx, name, stage):
    current = ctx.shot("before-exit-training")
    if grid_cell(current) is None and routes.passes(ctx, "training", 3):
        routes.play(ctx, "pause-open")
        current = ctx.shot("exit-pause")
    if grid_cell(current) is not None:
        select_cell(ctx, current, (0, 0), "native-back")
        ctx.play("item-confirm")
        ctx.shot("left-item-grid")
    routes.gate(ctx, "native-pause")
    for i in range(9):
        frame = ctx.shot(f"quit-focus-{i}")
        with Image.open(frame) as source:
            title = source.convert("RGB").crop((57, 9, 281, 49))
            pink = sum(r > 170 and g < 85 and b > 70 for r, g, b in title.getdata())
        if pink > 120:
            break
        ctx.play("grid-up")
    else:
        raise RuntimeError("Quit Training did not become focused")
    routes.play(ctx, "confirm-native-quit-frames")
    if not routes.passes(ctx, "css", 18):
        if routes.select_screen(ctx) != "sss":
            routes.back_to_main(ctx)
            routes.main_to_stage(ctx)
        routes.pick_stage(ctx, stage=stage)
    routes.pick_fighters(ctx, p1=name)
    routes.enter_training(ctx)
    routes.preload_clear(ctx)
    return routes.fixture(ctx, p1=name, stage=stage, reset=True, lr=1)


def unpause(ctx, label):
    # Plus cannot fire neutral special. Each input has a fresh screen check.
    for turn in range(3):
        if routes.passes(ctx, "training", 2.5):
            return
        ctx.shot(f"{label}-paused-{turn}")
        ctx.play("item-plus")
    ctx.gate("training-hud-unpaused")


# Zero-based row and column from captured 1280x674 native grid stills.
ITEM_CELLS = {"Sandbag": (2, 0), "Soccer Ball": (8, 1)}


def grid_cell(path):
    with Image.open(path) as source:
        if source.size != (1280, 674):
            raise RuntimeError(f"Unexpected native grid image dimensions: {source.size}")
        px = source.convert("RGB").load()
        bar = next((y for y in range(115, 545) if all(px[615, y+d] == (231, 231, 231)
                                                      for d in range(8))), None)
        if bar is None:
            return None
        offset = round((bar-123)/40.7)
        scores = []
        for row in range(7):
            for col in range(8):
                y, x = 147 + 62*row, 122 + 62*col
                score = 0
                for dy in (-38, -37, -36, 36, 37, 38):
                    for dx in range(-28, 29):
                        r, g, b = px[x+dx, y+dy]
                        score += int(r > 188 and 95 < g < 241 and b < 30)
                scores.append((score, row + offset, col))
        score, row, col = max(scores)
        return (row, col) if score >= 80 else None


def items_focused(path):
    with Image.open(path) as source:
        px = source.convert("RGB").load()
        for y in range(472, 480):
            samples = (px[x, y] for x in range(355, 570))
            if sum(r > 185 and 90 < g < 235 and b < 30 for r, g, b in samples) > 190:
                return True
    return False


def open_grid(ctx, cur, label):
    if grid_cell(cur) is not None:
        return cur
    routes.gate(ctx, "native-pause")
    if not items_focused(cur):
        ctx.play("item-enter")
        cur = ctx.shot(f"{label}-native-items-focus")
    if grid_cell(cur) is not None:
        return cur
    if not items_focused(cur):
        raise RuntimeError("Native pause did not focus Items")
    ctx.play("item-confirm")
    cur = ctx.shot(f"{label}-grid-open")
    if grid_cell(cur) is None:
        raise RuntimeError("Native Items selection did not open its grid")
    return cur


def select_item(ctx, item, take, facts):
    label = f"{item}-{take}"
    ctx.play("item-plus")
    cur = open_grid(ctx, ctx.shot(f"{label}-pause"), label)
    facts["grid_open"] = str(cur)
    facts["grid_selection"] = str(select_cell(ctx, cur, ITEM_CELLS[item], label))


def select_cell(ctx, cur, expected, label):
    actual = grid_cell(cur)
    if actual is None:
        raise RuntimeError("Native pause did not reach the item grid")
    for i in range(3):
        if actual == expected:
            return cur
        for now, want, negative, positive in zip(actual, expected, ("up", "left"), ("down", "right")):
            for _ in range(abs(want-now)):
                ctx.play("grid-" + (negative if want < now else positive))
        cur = ctx.shot(f"{label}-grid-{i}")
        actual = grid_cell(cur)
        if actual is None:
            break
    if actual != expected:
        raise RuntimeError(f"Native item grid selected {actual}, expected {expected}")
    return cur


def item_pair(ctx, fighter, item, resume_paused=False):
    out = {"fighter": fighter, "item": item, "takes": []}
    try:
        if resume_paused:
            unpause(ctx, "same-boot-recovery")
        modules = ctx.call("modules.list")["modules"]
        main = next(m for m in modules if m["name"] == "main")
        if main["build_id"] != MAIN_BUILD or egg.MODULE_IDS[fighter] not in {m["build_id"] for m in modules}:
            raise RuntimeError("Exact main/fighter NRO build ID mismatch")
        out["modules"] = [m for m in modules if m["build_id"] in (MAIN_BUILD, egg.MODULE_IDS[fighter])]
        manager_slot = int(main["base"], 16) + 0x52c4070
        for take in range(1, 3):
            facts = {"take": take}
            out["takes"].append(facts)
            # The stock L+R+A reset must clear every prior stock item.
            facts["setup"] = setup(ctx, fighter)
            _, present = item_vector(ctx, manager_slot, facts)
            if present:
                raise RuntimeError("Training reset did not clear the previous item")
            select_item(ctx, item, take, facts)
            ctx.play("item-confirm")
            facts["confirmed"] = str(ctx.shot(f"{item}-{take}-confirmed"))
            unpause(ctx, f"{item}-{take}")
            ctx.play("neutral-90")
            slot, present = item_vector(ctx, manager_slot, facts)
            if present != 1:
                raise RuntimeError("The native Training selection did not create one item")
            ident, position = item_layout(ctx, slot, KINDS[item], facts)
            current = geometry(ctx, slot, position, facts)
            for _ in range(5):
                if 20 <= current["forward"] <= 30:
                    break
                if current["forward"] < 20:
                    ctx.play("step-away")
                else:
                    ctx.play("step-toward")
                ctx.play("neutral-10")
                current = geometry(ctx, slot, position, facts)
            # The ball posture is its center; the before still shows the
            # ball resting on the same Final Destination surface as P1.
            floor_offset = 3.8 if item == "Soccer Ball" else 0.0
            previous = current
            ctx.play("neutral-10")
            current = geometry(ctx, slot, position, facts)
            facts["item_floor_origin_offset"] = floor_offset
            if (not 20 <= current["forward"] <= 30
                    or abs(current["item"][1] - current["p1"]["y"] - floor_offset) > 0.1
                    or any(abs(a-b) > 0.01 for a, b in zip(previous["item"], current["item"]))
                    or current["p1"]["situation"] != 0 or current["p1"]["status"] != 0
                    or current["id_kind"] != [ident, KINDS[item]]):
                raise RuntimeError(f"Unqualified geometry: {current}")
            facts["before"] = str(ctx.shot(f"{item}-{take}-before"))
            arms = {}
            tracefile = egg._trace_file(ctx)
            after = egg._last_trace_sequence(tracefile)
            try:
                for key, spec in egg._flag_points(fighter).items():
                    arms[key] = ctx.call("trace.arm", spec)["id"]
                arms["search"] = ctx.call("trace.arm", egg.SEARCH_POINT)["id"]
                ctx.play("btt-b")
                ctx.play("neutral-180")
                facts["after"] = str(ctx.shot(f"{item}-{take}-after"))
                counts = ctx.call("trace.counters")
                facts["counters"] = counts
                pages, records = egg._read_trace(ctx, after)
                facts["arms"] = arms
                facts["search"] = egg._counter(counts, arms["search"])
                facts["flags"] = egg._read_flag_records(records, set(arms.values()) - {arms["search"]})
                if (int(facts["search"].get("matched", 0)) != 1 or not facts["flags"]
                        or any(f["found"] is None for f in facts["flags"])
                        or any(int(p.get("lost", 0)) for p in pages)
                        or int(counts.get("file_lost", 0)) or counts.get("output_error")):
                    raise RuntimeError(f"Unqualified move trace at take {take}")
                facts["flag_found"] = any(f["found"] for f in facts["flags"])
            finally:
                for pid in arms.values():
                    ctx.call("trace.disarm", {"id": str(pid)})
        return {"fighter": fighter, "item": item, "found": [t["flag_found"] for t in out["takes"]],
                "forward": [t["geometry"][-1]["forward"] for t in out["takes"]]}
    finally:
        (Path(ctx.row_dir) / "item-proof.json").write_text(json.dumps(out, indent=2) + "\n")


def sandbag_creation(ctx):
    """Read the neutral item creation used by the stock Training menu."""
    out = {"probes": {}}
    setup(ctx, "Sonic")
    select_item(ctx, "Sandbag", 1, out)
    mark = egg._last_trace_sequence(egg._trace_file(ctx))
    ids = {}
    registers = [f"x{i}" for i in range(8)] + ["x30"]
    specs = {
        "safe_pos": {"target": "main+0x2284310", "registers": registers, "max_hits": 4},
        "manager_create": {"target": "main+0x15e3f40", "registers": registers, "max_hits": 4,
                           "captures": [{"expression": "x1", "type": "bytes", "size": 128}]},
    }
    try:
        for name, spec in specs.items():
            ids[name] = str(ctx.call("trace.arm", spec)["id"])
        ctx.play("item-confirm")
        unpause(ctx, "native-create")
        ctx.play("neutral-90")
        _, records = egg._read_trace(ctx, mark)
        for name, pid in ids.items():
            out["probes"][name] = [r for r in records if str(r.get("point_id")) == pid]
        (Path(ctx.row_dir) / "creation.json").write_text(json.dumps(out, indent=2) + "\n")
        return {name: len(records) for name, records in out["probes"].items()}
    finally:
        for pid in ids.values():
            ctx.call("trace.disarm", {"id": pid})
