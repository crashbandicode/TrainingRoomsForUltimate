"""Exact rc.15 gate acceptance for native practice target lock-on."""

from contextlib import contextmanager
import hashlib
import json
from pathlib import Path
import re
import struct
import sys
import time

# The pinned runner starts Python in isolated mode.
sys.path.insert(0, str(Path(__file__).resolve().parent))

import btt_hw as btt
import eggs_hw as egg
import fx
import hotslots as hs
import itemproof as native
import practice_steps as menu
import routes

CREATE = "main+0x15db0b0"
REMOVE = "main+0x2144720"
TEAM = "main+0x208a8f0"
SET_TEAM = "main+0x208a910"
VISIBILITY = "main+0x208c0c0"
WORK_FLAG = "main+0x208dc90"
SANDBAG = native.KINDS["Sandbag"]
GAMES = {"Eggs": "Eggs-ercise", "Btt": "Break the Targets"}


def boot(ctx):
    if ctx.call("status").get("running"):
        raise RuntimeError("configure native trace before game.load")
    stub = routes._setting(ctx, "core.use_gdbstub", False)
    output = ctx.call("trace.configure", {"run_directory": str(ctx.row_dir), "label": "btt-native"})
    return {"stub": stub, "output": output, "persisted": ctx.call("settings.save", {})}


def check_build(ctx):
    pin = json.loads((Path(__file__).parent / "build-pins.json").read_text())
    path = Path(pin["diag_nro"])
    if hashlib.sha256(path.read_bytes()).hexdigest() != pin["diag_sha256"]:
        raise RuntimeError("staged DIAG SHA-256 differs from gate receipt")
    modules = ctx.call("modules.list")["modules"]
    if pin["diag_build_id"] not in {m["build_id"] for m in modules}:
        raise RuntimeError("gate DIAG build ID absent from loaded modules")
    return menu.save(ctx, "loaded-build", {"pin": pin, "modules": modules})


@contextmanager
def trace(ctx, specs):
    arms = {}
    after = egg._last_trace_sequence(egg._trace_file(ctx))
    try:
        arms.update((name, str(ctx.call("trace.arm", spec)["id"])) for name, spec in specs.items())
        yield after, arms
    finally:
        for ident in arms.values():
            ctx.call("trace.disarm", {"id": ident})


def read(ctx, after, arms):
    counts = ctx.call("trace.counters")
    pages, records = egg._read_trace(ctx, after)
    if any(int(p.get("lost", 0)) for p in pages) or int(counts.get("file_lost", 0)) or counts.get("output_error"):
        raise RuntimeError("Eden trace reported loss")
    grouped = {key: [r for r in records if str(r.get("point_id")) == ident] for key, ident in arms.items()}
    return counts, grouped


def manager(ctx, fighter):
    modules = ctx.call("modules.list")["modules"]
    main = next(m for m in modules if m["name"] == "main")
    ids = {m["build_id"] for m in modules}
    if main["build_id"] != native.MAIN_BUILD or egg.MODULE_IDS[fighter] not in ids:
        raise RuntimeError("Unexpected main or fighter NRO")
    return int(main["base"], 16) + 0x52c4070, modules


def census(ctx, manager_slot, out):
    _, (vector,) = native.sample(ctx, [(f"[{manager_slot:#x}]+0x28", 16)], out)
    begin, end = struct.unpack("<QQ", vector)
    count = (end - begin) // 8
    if not 0 <= count <= 16 or end != begin + 8 * count:
        raise RuntimeError(f"unexpected item vector {begin:#x}..{end:#x}")
    if count == 0:
        return []
    captures = [(f"[{begin + i * 8:#x}]+8", 8) for i in range(count)]
    captures += [(f"{begin + i * 8:#x}", 8) for i in range(count)]
    _, identities = native.sample(ctx, captures, out)
    result = []
    for i in range(count):
        item, kind = struct.unpack("<II", identities[i])
        item_object, = struct.unpack("<Q", identities[count + i])
        accessor = item_object + 0x98
        location = None
        if kind == SANDBAG:
            slot = begin + i * 8
            _, pos = native.item_layout(ctx, slot, SANDBAG, out)
            location = native.geometry(ctx, slot, pos, out)
            if location["id_kind"] != [item, kind]:
                raise RuntimeError(f"unstable item list during census: {location}")
        result.append({"id": item, "kind": kind, "accessor": accessor, "geometry": location})
    return result


def stop(ctx, code):
    return menu.stop_game(ctx, GAMES[code], code)


def start_eggs(ctx):
    fx._live_set(ctx, eggsercise=egg.FREE, eggsercise_break=[1, 0, 0], eggsercise_target=egg.DISC,
                 eggsercise_speed=[1, 0, 0], practice_cpu=egg.HIDE, hitbox_vis=[0] * 7,
                 combo_helper=[0, 1])
    return menu.start_game(ctx, GAMES["Eggs"], r"\[Eggs\] notice=", cap=20)



def grounded_x(ctx, left, right, face_right=False, moves=10):
    """Use the recorded stick-only routes and verify actual settled position."""
    observed = []
    for _ in range(moves + 1):
        state = routes.read_state(ctx)[0]
        observed.append(state)
        if left <= state["x"] <= right:
            if state["status"] == 0 and state["situation"] == 0:
                if not face_right or state["lr"] == 1:
                    return observed
                ctx.play("step-toward")
                continue
            ctx.play("neutral-10")
        else:
            ctx.play("step-away" if state["x"] > right else "step-toward")
    raise RuntimeError(f"grounded launch position did not settle in {left}..{right}: {observed[-1]}")


def parked(ctx):
    p1 = routes.read_state(ctx)[0]
    if not -70 <= p1["x"] <= -55 or p1["status"] != 0 or p1["situation"] != 0:
        raise RuntimeError(f"BTT setup requires grounded, stationary P1 away from the front target: {p1}")
    return p1


def start_btt(ctx):
    away = grounded_x(ctx, -70, -55)
    fx._live_set(ctx, btt_projectiles=btt.ON, practice_cpu=egg.HIDE, hitbox_vis=[0] * 7, combo_helper=[0, 1])
    menu.open_game(ctx, GAMES["Btt"])
    menu.focus_sub(ctx, GAMES["Btt"], "Place targets")
    mark = hs.diag_mark(ctx)
    ctx.press("a")
    placing = btt.wait_phase(ctx, mark, "Placing", 15)
    for i in range(6):
        ctx.play("btt-zl-right")
        state = btt.state(ctx)
        if state and "Lock-on acceptance:" in state["text"]:
            break
        time.sleep(.25)
    else:
        raise RuntimeError(f"pinned course was not in the saved layout list: {btt.state(ctx)}")
    mark = hs.diag_mark(ctx)
    ctx.play("btt-zl-up")
    if not hs.wait_diag(ctx, mark, r'\[Btt\].*text="Loaded: Lock-on acceptance', 10):
        raise RuntimeError("native load has not selected the pinned course")
    before_x = parked(ctx)
    mark = hs.diag_mark(ctx)
    ctx.play("btt-x")
    ready = btt.wait_phase(ctx, mark, "Ready", 15)
    ctx.play("neutral-90")
    before_l = parked(ctx)
    mark = hs.diag_mark(ctx)
    ctx.press("l")
    running = btt.wait_phase(ctx, mark, "Running", 10)
    ctx.play("neutral-90")
    after_l = parked(ctx)
    approach = grounded_x(ctx, -40, -33, face_right=True)
    return {"placing": placing, "ready": ready, "running": running,
            "park": away, "before_x": before_x, "before_l": before_l,
            "after_l": after_l, "approach": approach}


def scores(ctx, code, mark):
    if code == "Eggs":
        return [int(m[1]) for _, _, line in hs.diag_since(ctx, mark)
                if (m := re.search(r'\[Eggs\] notice="Eggs (\d+)', line))]
    result = []
    for _, _, text in hs.diag_since(ctx, mark):
        found = btt.PHASE.search(text)
        if found:
            count = btt.COUNT.fullmatch(found[3])
            if count:
                result.append(int(count[1]))
    return result


def confirm_cpu_hidden(ctx):
    return {"hurtbox": dict(target="main+0x203db20", condition="x1 == 3", max_hits=4,
                           registers=["x0", "x1", "x30"])}


def own_team_reads(rows, diag_module):
    """Pair the binding's entries and returns by native thread."""
    pending = {}
    teams = {}
    for row in rows:
        registers = row["registers"]
        thread = row["thread_id"]
        if row["event"] == "entry":
            caller = row.get("locations", {}).get(registers["x30"], "")
            pending[thread] = int(registers["x0"], 16) if caller.startswith(diag_module) else None
        elif row["event"] == "return":
            accessor = pending.pop(thread, None)
            if accessor is not None:
                teams[accessor] = int(registers["x0"], 16) & 0xFFFFFFFF
    return teams


def run(ctx, fighter, code, diagnostic_only=False):
    out = {"fighter": fighter, "game": code, "diagnostic_only": diagnostic_only}
    try:
        slot, out["modules"] = manager(ctx, fighter)
        out["start_fixture"] = routes.fixture(ctx, p1=fighter, stage="Final Destination", reset=True, lr=1)
        out["stock"] = census(ctx, slot, out)
        if out["stock"]:
            raise RuntimeError("start census contains prior or stock item")
        mark = hs.diag_mark(ctx)
        diag_module = "nro:" + json.loads((Path(__file__).parent / "build-pins.json").read_text())["diag_build_id"] + "+"
        specs = {
            **confirm_cpu_hidden(ctx),
            "pre_action_contact": dict(target="main+0x4028dc", max_hits=16,
                registers=["x1", "x3", "x24", "x30"],
                captures=[{"expression": "x24+8", "type": "bytes", "size": 4}]),
            "pre_action_remove": dict(target=REMOVE, max_hits=8, registers=["x0", "x1", "x30"]),
            "pre_action_search": egg.SEARCH_POINT,
            "create": dict(target=CREATE, condition="x2 == 1", max_hits=8, on_return=True,
                           registers=["x0", "x1", "x2", "x3", "x4", "x5", "x30"]),
            "team": dict(target=TEAM, max_hits=24, on_return=True, registers=["x0", "x30"]),
            "set_team": dict(target=SET_TEAM, max_hits=32, registers=["x0", "x1", "x2", "x30"]),
            "no_pocket": dict(target=WORK_FLAG, condition="x1 == 0x20000000", max_hits=16,
                              registers=["x0", "x1", "x30"]),
        }
        with trace(ctx, specs) as (after, arms):
            out["start"] = start_eggs(ctx) if code == "Eggs" else start_btt(ctx)
            if code == "Eggs":
                egg.place(ctx, 24.0, 8.0)
            out["before"] = census(ctx, slot, out)
            _, out["start_trace"] = read(ctx, after, arms)
        out["pre_action_scores"] = scores(ctx, code, mark)
        if code == "Btt" and (
                set(out["pre_action_scores"]) != {0}
                or any(out["start_trace"][f"pre_action_{name}"]
                       for name in ("contact", "remove", "search"))):
            raise RuntimeError("BTT startup had a native search, hit, retirement or score before the special")
        count = 1 if code == "Eggs" else 2
        if (not out["before"] or any(item["kind"] != SANDBAG for item in out["before"])
                or (len(out["before"]) != count and not diagnostic_only)):
            raise RuntimeError(f"wanted {count} native Sandbag(s), observed {out['before']}")
        successful = {int(r["registers"]["x0"], 16) + 0x98
                      for r in out["start_trace"]["create"] if r["event"] == "return" and int(r["registers"]["x0"], 16)}
        if not {item["accessor"] for item in out["before"]} <= successful or not out["start_trace"]["hurtbox"]:
            raise RuntimeError("native Sandbag creation or the CPU hurtbox-off step was not observed")
        teams = own_team_reads(out["start_trace"]["team"], diag_module)
        hidden = {int(r["registers"]["x0"], 16) for r in out["start_trace"]["hurtbox"]}
        cpu_teams = {teams[a] for a in teams.keys() & hidden}
        p1_accessors = teams.keys() - hidden - {item["accessor"] for item in out["before"]}
        if len(p1_accessors) != 1:
            raise RuntimeError(f"expected one independent P1 accessor: {p1_accessors}")
        p1_accessor = next(iter(p1_accessors))
        _, (p1_id,) = native.sample(ctx, [(f"{p1_accessor + 8:#x}", 4)], out)
        out["p1_id"] = int.from_bytes(p1_id, "little")
        other_teams = {teams[p1_accessor]}
        item_teams = {int(r["registers"]["x0"], 16): int(r["registers"]["x1"], 16) & 0xFFFFFFFF
                      for r in out["start_trace"]["set_team"]}
        no_pocket = {int(r["registers"]["x0"], 16) for r in out["start_trace"]["no_pocket"]}
        out["team_check"] = {"cpu": sorted(cpu_teams), "other": sorted(other_teams), "item": item_teams}
        if len(cpu_teams) != 1 or not other_teams or cpu_teams & other_teams:
            raise RuntimeError(f"CPU and P1 native teams were not distinguished: {out['team_check']}")
        item_specs = {f"team-{item['id']}": dict(target=TEAM, condition=f"x0 == {item['accessor']}",
                      max_hits=4, on_return=True, registers=["x0", "x30"]) for item in out["before"]}
        with trace(ctx, item_specs) as (after, arms):
            ctx.play("neutral-10")
            _, out["steady_team"] = read(ctx, after, arms)
        out["team_stable"] = all(
            own_team_reads(out["steady_team"][f"team-{item['id']}"], diag_module).get(item["accessor"]) in cpu_teams
            for item in out["before"])
        out["no_pocket_set"] = all(item["accessor"] in no_pocket for item in out["before"])
        if not diagnostic_only and not (out["team_stable"] and out["no_pocket_set"]):
            raise RuntimeError("owned item is missing its CPU team or Sandbag NO_POCKET flag")
        visibility = {str(item["id"]): dict(
            target=VISIBILITY, condition=f"x0 == {item['accessor']}", max_hits=2,
            registers=["x0", "x1", "x30"]) for item in out["before"]}
        with trace(ctx, visibility) as (after, arms):
            ctx.play("neutral-10")
            _, out["visibility"] = read(ctx, after, arms)
        if any(not rows or any(int(r["registers"]["x1"], 16) != 0 for r in rows)
               for rows in out["visibility"].values()):
            raise RuntimeError("owned item visibility-off was not observed")
        before = scores(ctx, code, mark)
        if not before or before[-1] != 0:
            raise RuntimeError(f"expected a zero-score start: {before}")
        aimed = [item for item in out["before"] if 20 <= item["geometry"]["forward"] <= 30]
        if len(aimed) != 1:
            raise RuntimeError(f"wanted one forward item in range: {out['before']}")
        target = aimed[0]
        others = {item["id"] for item in out["before"] if item != target}
        if not 2 <= target["geometry"]["vertical"] <= 8 or target["geometry"]["p1"]["status"] != 0:
            raise RuntimeError(f"target depth/height or grounded waiting condition failed: {target}")
        out["before_still"] = str(ctx.shot("target-and-invisible-item-before"))
        _, (damage_module,) = native.sample(ctx, [(f"{target['accessor'] + 0xa8:#x}", 8)], out)
        damage_ptr, = struct.unpack("<Q", damage_module)
        out["damage_module_ptr"] = hex(damage_ptr)
        if damage_ptr:
            _, (before_damage,) = native.sample(ctx, [(f"{damage_ptr:#x}", 128)], out)
            out["before_damage"] = before_damage.hex()
        action = hs.diag_mark(ctx)
        specs = {**egg._flag_points(fighter), "search": egg.SEARCH_POINT}
        for candidate in out["before"]:
            item_id = candidate["id"]
            specs[f"remove-{item_id}"] = dict(
                target=REMOVE, condition=f"x1 == {item_id}", max_hits=16, registers=["x0", "x1", "x30"])
            specs[f"native-contact-{item_id}"] = dict(
                target="main+0x4028dc", condition=f"x3 == {item_id}", max_hits=8,
                registers=["x1", "x3", "x24"],
                captures=[{"expression": "x24+8", "type": "bytes", "size": 4}])
        with trace(ctx, specs) as (after, arms):
            ctx.play("btt-b")
            ctx.play("neutral-180")
            out["after_still"] = str(ctx.shot("target-after-native-special"))
            out["after"] = census(ctx, slot, out)
            counts, out["action_trace"] = read(ctx, after, arms)
            out["search"] = egg._counter(counts, arms["search"])
            flags = [r for key, rows in out["action_trace"].items() if key.startswith(("homing-", "reticle-")) for r in rows]
            out["flags"] = egg._read_flag_records(flags, set(arms.values()))
        if damage_ptr and any(item["id"] == target["id"] for item in out["after"]):
            _, (after_damage,) = native.sample(ctx, [(f"{damage_ptr:#x}", 128)], out)
            out["after_damage"] = after_damage.hex()
        out["scores"] = scores(ctx, code, action)
        out["action_diag"] = [t for _, _, t in hs.diag_since(ctx, action)
                              if "[Btt] projectile " in t or "[Eggs] pass " in t or "[Btt] phase=" in t]
        surviving = {item["id"] for item in out["after"]}
        cleared = [item for item in out["before"] if item["id"] not in surviving]
        if len(cleared) != 1:
            raise RuntimeError(f"expected exactly one original target identity to retire: {cleared}")
        target = cleared[0]
        out["selected_target"] = target
        others = {item["id"] for item in out["before"] if item != target}
        out["removed_target"] = True
        out["action_trace"]["remove"] = out["action_trace"][f"remove-{target['id']}"]
        out["action_trace"]["native_contact"] = out["action_trace"][f"native-contact-{target['id']}"]
        if int(out["search"].get("matched", 0)) != 1 or not any(f["found"] is True for f in out["flags"]):
            raise RuntimeError("single native SEARCH and positive fighter target flag were not both observed")
        contacts = out["action_trace"]["native_contact"]
        if not contacts or any(hit["captures"][0].get("hex") != p1_id.hex() for hit in contacts):
            raise RuntimeError(f"expected exact native contact credited to P1: {contacts}")
        expected_category = 0 if fighter == "Sonic" else 1
        if any(int(hit["registers"]["x1"], 16) >> 28 != expected_category for hit in contacts):
            raise RuntimeError("native source category differs from the move")
        if not out["scores"] or max(out["scores"]) != 1 or out["scores"][-1] != 1:
            raise RuntimeError(f"wanted exactly one break: {out['scores']}")
        if len(out["action_trace"]["remove"]) != 1 or not out["removed_target"]:
            raise RuntimeError("break did not show the target item removed")
        if code == "Eggs" and (len(out["after"]) != 1 or out["after"][0]["kind"] != SANDBAG):
            raise RuntimeError("one new Egg item must follow the broken one")
        if code == "Btt" and len(out["before"]) == count and {item["id"] for item in out["after"]} != others:
            raise RuntimeError("a BTT break must leave exactly the other target items")
        out["stop"] = stop(ctx, code)
        ctx.play("neutral-90")
        out["off"] = census(ctx, slot, out)
        if out["off"]:
            raise RuntimeError("stopped game left owned item(s)")
        return {"diagnostic_only": diagnostic_only, "found": True, "score": 1,
                "before_count": len(out["before"]), "expected_count": count,
                "after_ids": [item["id"] for item in out["after"]], "off": 0}
    finally:
        (Path(ctx.row_dir) / "lockon-acceptance.json").write_text(json.dumps(out, indent=2) + "\n")


def switch(ctx, fighter):
    return native.fighter_select(ctx, fighter, "Final Destination")


def recover_training(ctx, fighter):
    """Recover known Training and plugin-menu states after a failed effect row."""
    active = None
    opened = False
    with routes.instance_path(ctx, "diag_log").open(errors="replace") as log:
        for line in log:
            if "[Ui] menu opened " in line:
                opened = True
            elif "[Ui] menu closed " in line:
                opened = False
            if "[Practice] stop game=" in line:
                active = None
            elif match := re.search(r"\[Practice\] start game=(Eggs|Btt)\b", line):
                active = match[1]
    if not routes.passes(ctx, "training", 2) and opened:
        menu.close_all(ctx)
    routes.gate(ctx, "training")
    if active:
        stop(ctx, active)
    expected = routes.fighter_entry(fighter)["kind"]
    if routes.read_state(ctx)[0]["kind"] != expected:
        return switch(ctx, fighter)
    return routes.fixture(ctx, p1=fighter, stage="Final Destination", reset=True, lr=1)


def request(ctx, mode, completion):
    root = Path(ctx.instance["settings"]).parent
    mark = hs.diag_mark(ctx)
    (root / f"diag-{mode}.req").write_text("\n")
    receipt = hs.wait_diag(ctx, mark, completion, 30)
    if not receipt:
        raise RuntimeError(f"missing native save-state {mode} completion")
    return receipt


def load_and_depart(ctx, fighter, following_fighter):
    out = {}
    try:
        slot, out["modules"] = manager(ctx, fighter)
        routes.fixture(ctx, p1=fighter, stage="Final Destination", reset=True, lr=1)
        out["saved"] = request(ctx, "save", r"state saved_notice slot=1")
        out["start"] = start_eggs(ctx)
        egg.place(ctx, 24.0, 8.0)
        out["before"] = census(ctx, slot, out)
        if len(out["before"]) != 1 or out["before"][0]["kind"] != SANDBAG:
            raise RuntimeError("expected one live practice item before load")
        original = out["before"][0]["id"]
        specs = {"remove": dict(target=REMOVE, condition=f"x1 == {original}", max_hits=8,
                                registers=["x0", "x1", "x30"])}
        with trace(ctx, specs) as (after, arms):
            out["loaded"] = request(ctx, "load", r"save_state loaded ledge=")
            out["after_load"] = census(ctx, slot, out)
            _, removed = read(ctx, after, arms)
        out["load_removal"] = removed["remove"]
        if len(out["load_removal"]) != 1:
            raise RuntimeError("load must remove the old lock-on item exactly once")
        out["before_scene"] = census(ctx, slot, out)
        if not out["before_scene"]:
            out["restart_after_load"] = start_eggs(ctx)
            egg.place(ctx, 24.0, 8.0)
            out["before_scene"] = census(ctx, slot, out)
        if len(out["before_scene"]) != 1 or out["before_scene"][0]["kind"] != SANDBAG:
            raise RuntimeError("scene departure needs one live Sandbag")
        old = out["before_scene"][0]
        out["after_scene"] = switch(ctx, following_fighter)
        slot, out["after_scene_modules"] = manager(ctx, following_fighter)
        out["scene_items"] = census(ctx, slot, out)
        if any((item["id"], item["accessor"]) == (old["id"], old["accessor"])
               for item in out["scene_items"]):
            raise RuntimeError("the departed Sandbag identity is still active")
        if out["scene_items"]:
            out["stop_new_scene"] = stop(ctx, "Eggs")
            out["off"] = census(ctx, slot, out)
            if out["off"]:
                raise RuntimeError("the restarted scene left a practice item after Stop")
        return {"load_removals": 1, "departed_id": old["id"], "scene_items": len(out["scene_items"])}
    finally:
        (Path(ctx.row_dir) / "lockon-lifecycle.json").write_text(json.dumps(out, indent=2) + "\n")


def enter_existing(ctx, fighter):
    if routes.passes(ctx, "training", 3):
        return recover_training(ctx, fighter)
    if not routes.passes(ctx, "css", 5):
        if routes.select_screen(ctx) != "sss":
            routes.back_to_main(ctx)
            routes.main_to_stage(ctx)
        routes.pick_stage(ctx, stage="Final Destination")
    routes.pick_fighters(ctx, p1=fighter)
    routes.enter_training(ctx)
    routes.preload_clear(ctx)
    return routes.fixture(ctx, p1=fighter, stage="Final Destination", reset=True, lr=1)
