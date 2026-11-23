"""F11 rows (tile-fix-g4, 2026-11-23): a picker's CPU start is shown by the CPU itself.

Guard the Edge ("6") and Juggler ("7") pickers, driven through the menu and crosshair helpers flown in
flight-755d9936 (edge_steps.py). Every observation reads three things and keeps a still:
  tr.practice   wizard.step, wizard.x/y and wizard.cpu (in_view | magnifier; the second is the game's FLAG_IS_LOUPE)
  fighters      where the CPU's body is (bridge `fighters`)
  tr.menu       the picker's notice ("CPU (x, y) ...")

The body comparison has no tolerance and no rounding. A held body is rewritten once a frame and the game's physics
then takes one step of the fighter's gravity before the bridge samples it
(artifacts/phase23/edge-juggle/ROOT-CAUSE-g9-ready-y-drop.md), so the expected sample is, as f32 words:
  x == the picked x;  y == f32(picked y) - f32(the CPU fighter's gravity).
The picked point is the one tr.practice publishes (Rust's shortest round-trip decimal, exact as f32). The gravity is
a row argument tied to the CPU's kind, which the row checks first (f11-probe-1: Zero Suit Samus, kind 32, 0.12; 12
of 12 samples met this rule bit for bit). A missing read is a failure. After B the body's x, y and situation must
equal the read taken before the picker opened. A start is checked as rows G9/G10 check it (edge_ready_exact).
Each row saves f11-<name>.json in its folder and raises when an expectation is not met.
"""
import json
import re
import struct
import subprocess
import sys
import time
from pathlib import Path

import routes
import switch_hw

sys.path.insert(0, str(Path(__file__).resolve().parent))
import edge_ready_exact  # noqa: E402
import edge_steps as edge  # noqa: E402

PRACTICE = "tr.practice"


def _f32(value):
    return struct.unpack("<f", struct.pack("<f", float(value)))[0]


def _word(value):
    return "0x%08x" % struct.unpack("<I", struct.pack("<f", float(value)))[0]


def held_problem(body, point, gravity):
    """Why a held body's bridge sample is not the picked point (None: it is). Pure; no tolerance."""
    try:
        x, y = point
        want = [_word(x), _word(_f32(_f32(y) - _f32(gravity)))]
        have = [_word(body["x"]), _word(body["y"])]
    except (KeyError, TypeError, ValueError):
        return f"a read is missing: body {body!r}, picked point {point!r}"
    if have != want:
        return f"body {have} ({body['x']}, {body['y']}) is not the held point {want} (picked {point}, gravity {gravity})"
    return None


def back_problem(body, origin):
    """Why the body is not back where it stood (None: it is): x, y and situation equal the read before the picker."""
    try:
        have = [_word(body["x"]), _word(body["y"]), body["situation"]]
        want = [_word(origin["x"]), _word(origin["y"]), origin["situation"]]
    except (KeyError, TypeError, ValueError):
        return f"a read is missing: body {body!r}, origin {origin!r}"
    if have != want or None in have:
        return f"after B the CPU reads {have} {body!r}; before the picker it read {want} {origin!r}"
    return None


def ready_body_problem(body, cpu_kind):
    """Why a Ready sample of the selected CPU is not usable (None: it is): the row's fighter, with x and y both read
    as finite numbers. The position itself is judged at the release boundary (edge_ready_exact), not here: this
    sample may fall on either side of the release."""
    if not isinstance(body, dict) or body.get("kind") != cpu_kind:
        return f"the Ready sample is not the selected CPU (kind {cpu_kind}): {body!r}"
    for axis in ("x", "y"):
        value = body.get(axis)
        if isinstance(value, bool) or not isinstance(value, (int, float)) or value != value or abs(value) == float("inf"):
            return f"the Ready sample has no valid {axis}: {body!r}"
    return None


def _save(ctx, name, body):
    (Path(ctx.row_dir) / f"f11-{name}.json").write_text(json.dumps(body, indent=2) + "\n")
    return body


def _cpu_body(ctx, scene):
    run = routes.instance_path(ctx, "bridge_log").parent
    bodies = switch_hw.bridge_state(run, ("fighters",), cap=4)["fighters"]
    body = bodies.get(4 if scene == "lobby" else 1, {})
    return {key: body.get(key) for key in ("kind", "x", "y", "situation", "status", "dmg")}


def _fixture(ctx, scene, cpu_kind):
    """The CPU before the picker; its kind must be the one the row's gravity belongs to."""
    origin = _cpu_body(ctx, scene)
    if origin.get("kind") != cpu_kind:
        raise RuntimeError(f"FIXTURE: the CPU's kind is {origin.get('kind')!r}; this row's gravity is for kind {cpu_kind}")
    return origin


def _practice(ctx, step, cap=6, cpu=None):
    def accept(block):
        values = block.get("values", {})
        if block.get("schema") != "tr.practice/1":
            return False, f"schema {block.get('schema')!r}"
        if step is not None and values.get("wizard.step") != step:
            return False, f"wizard.step {values.get('wizard.step')!r}"
        if cpu is not None and values.get("wizard.cpu", "") != cpu:
            return False, f"wizard.cpu {values.get('wizard.cpu', '')!r}"
        return True, "ok"

    block, _ = ctx.runner._state_block(ctx.on, PRACTICE, cap, accept, ask_every=.3)
    return block["values"]


_last_still = [None]


def _fresh_shot(ctx, name, cap=45):
    """A still of a frame presented after the last observation's. The first magnifier of a boot stalls Eden's
    presentation while it builds pipelines (f11-probe-1: three stills in 23 s were byte-identical to the one before);
    the guest keeps running, so the still waits for the build to end and for a frame that differs."""
    until = time.monotonic() + cap
    while True:
        ctx.check_stop()
        status = ctx.call("status") or {}
        path = ctx.shot(name)
        data = path.read_bytes()
        age = status.get("last_frame_age_ms")
        if (status.get("shaders_building", 0) == 0 and isinstance(age, (int, float)) and age < 1000
                and data != _last_still[0]):
            _last_still[0] = data
            return str(path), True
        if time.monotonic() > until:
            _last_still[0] = data
            return str(path), False
        path.unlink(missing_ok=True)
        time.sleep(1.0)


def _observe(ctx, scene, state, name, step="cpu", expect=None, settle=0.8):
    """One observation of the picker. `expect`: the wizard.cpu word that must show within 6 s."""
    time.sleep(settle)  # the body is placed for a few frames before it is held; the HUD eases its bubble in
    notice = edge._notice(state)
    try:
        values = _practice(ctx, step, cpu=expect)
        problem = None
    except RuntimeError as error:
        values, problem = _practice(ctx, None, cap=4), repr(error)[:200]
    still, fresh = _fresh_shot(ctx, name)
    seen = {
        "name": name, "notice": notice,
        "wizard": {key: values.get(key) for key in ("phase", "wizard.step", "wizard.x", "wizard.y", "wizard.cpu")},
        "cpu_body": _cpu_body(ctx, scene), "still": still, "still_fresh": fresh,
    }
    if problem:
        seen["problem"] = problem
    if not fresh:
        seen["problem"] = (seen.get("problem", "") + " no new frame was presented for the still").strip()
    return seen


def recover(ctx, scene="training"):
    """After a failed row: B out of a drill picker step by step, stop a running drill, close the menu."""
    state = edge._read(ctx, scene)
    looks = []
    for _ in range(5):
        header = state["values"]["notice.header"]
        if not header.startswith(("Edge ·", "Juggle ·")):
            break
        looks.append(header)
        ctx.play(edge.TAP["B"])
        state = edge.observe(ctx, scene, lambda values: values["notice.header"] != header,
                             "the picker backs one step", state)
    if state["values"].get("game.running") == "1":
        ident = "7" if state["values"].get("game") == "Juggler" else "6"
        edge._stop(ctx, scene, ident)
    if state["values"].get("open") == "1" or looks:
        edge._close(ctx, scene)
    return {"backed_out_of": looks, "cpu_body": _cpu_body(ctx, scene)}


def _published(seen):
    """The picker's point as tr.practice published it, or None when either word is missing."""
    point = (seen["wizard"].get("wizard.x"), seen["wizard"].get("wizard.y"))
    return None if None in point else point


def _finish(ctx, name, body, problems):
    body["problems"] = problems
    _save(ctx, name, body)
    if problems:
        raise RuntimeError(f"{name}: {problems}")
    return {key: body[key] for key in ("origin", "back") if key in body} | {
        "seen": [(s["name"], s["wizard"].get("wizard.cpu"), s["cpu_body"].get("x"), s["cpu_body"].get("y"))
                 for s in body.get("seen", [])]}


def picker(ctx, cpu_kind, cpu_gravity, ident="6", scene="training", points=(), expect=None, name="picker"):
    """Open a drill's picker (its CPU step), walk the crosshair through `points`, then leave it with B.

    `expect`: one wizard.cpu word per observation (the default point first, then each of `points`); None records
    only. At every observation the CPU's body must be the held point; after B it must be back where it stood.
    """
    origin = _fixture(ctx, scene, cpu_kind)
    state = edge._setup(ctx, scene, ident)
    wanted = list(expect) if expect else [None] * (len(points) + 1)
    seen = [_observe(ctx, scene, state, f"{name}-00-default", expect=wanted[0])]
    for index, point in enumerate(points, 1):
        state = edge._move(ctx, scene, state, tuple(point))
        seen.append(_observe(ctx, scene, state, f"{name}-{index:02d}-{point[0]}_{point[1]}", expect=wanted[index]))
    body = {"ident": ident, "scene": scene, "cpu_kind": cpu_kind, "cpu_gravity": cpu_gravity, "origin": origin,
            "seen": seen}
    problems = [f"{s['name']}: {s['problem']}" for s in seen if s.get("problem")]
    for s in seen:
        held = held_problem(s["cpu_body"], _published(s), cpu_gravity)
        if held:
            problems.append(f"{s['name']}: {held}")
    edge._press(ctx, scene, "B", open="1")
    time.sleep(1.0)
    body["back"] = _cpu_body(ctx, scene)
    body["back_still"] = str(ctx.shot(f"{name}-left"))
    edge._close(ctx, scene)
    back = back_problem(body["back"], origin)
    if back:
        problems.append(back)
    return _finish(ctx, name, body, problems)


def through(ctx, cpu_kind, cpu_gravity, ident="6", scene="training", cpu=(-100, -20), player=(-40, 0), expect=None,
            name="through"):
    """Guard the Edge's whole picker: CPU point, player point, preview; then start the drill and stop it.

    `expect`: wizard.cpu words for the CPU step, the player step and the preview. In every step the CPU's body must
    be the held CPU point (the one published at the CPU step). The start is checked as rows G9/G10 check it: the
    plugin's own read at its Ready-to-Live boundary equals the picked point word for word (edge_ready_exact).
    """
    if ident != "6":
        raise ValueError("through covers Guard the Edge; the Juggler's start is row J6's")
    _fixture(ctx, scene, cpu_kind)
    wanted = list(expect) if expect else [None] * 3
    state = edge._move(ctx, scene, edge._setup(ctx, scene, ident), tuple(cpu))
    seen = [_observe(ctx, scene, state, f"{name}-cpu", expect=wanted[0])]
    cpu_point = _published(seen[0])
    state = edge._move(ctx, scene, edge._select(ctx, scene, ident, 2), tuple(player))
    seen.append(_observe(ctx, scene, state, f"{name}-player", step="player", expect=wanted[1]))
    state = edge._select(ctx, scene, ident, 3)
    seen.append(_observe(ctx, scene, state, f"{name}-preview", step="preview", expect=wanted[2]))
    body = {"ident": ident, "scene": scene, "cpu_kind": cpu_kind, "cpu_gravity": cpu_gravity, "cpu_point": cpu_point,
            "seen": seen}
    problems = [f"{s['name']}: {s['problem']}" for s in seen if s.get("problem")]
    for s in seen:  # the CPU waits on its own start in every step, also while the crosshair is the player's
        held = held_problem(s["cpu_body"], cpu_point, cpu_gravity)
        if held:
            problems.append(f"{s['name']}: {held}")
    if _published(seen[2]) != cpu_point:
        problems.append(f"the preview publishes {_published(seen[2])}, the CPU step published {cpu_point}")
    _save(ctx, name, {**body, "problems": problems})
    point = edge_ready_exact.save_picked(ctx, keep=False)
    before = edge._read(ctx, scene)
    watch = edge_ready_exact.open_trace(ctx)
    ctx.play(edge.TAP["A"])
    edge.observe(ctx, scene, lambda values: values["game.running"] == "1" and values["notice.body"].startswith("Ready"),
                 "the running drill reached Ready", before, cap=3)
    # One fighters read, required to hold the selected CPU with a valid x and y, and the same sample goes to the
    # release comparison (rv-edge-juggle-sol, 2026-11-23 19:42:41Z).
    bodies = switch_hw.bridge_state(routes.instance_path(ctx, "bridge_log").parent, ("fighters",), cap=3)["fighters"]
    sample = bodies.get(4 if scene == "lobby" else 1)
    unusable = ready_body_problem(sample, cpu_kind)
    if unusable:
        _save(ctx, name, {**body, "ready_sample": sample, "problems": problems + [unusable]})
        raise RuntimeError(f"{name}: {unusable}")
    body["ready"] = edge_ready_exact.check_ready(ctx, scene, point, watch, player[0], cpu[0], 100,
                                                 bodies=bodies)["edge_ready_exact"]
    body["ready"]["practice"] = {key: value for key, value in _practice(ctx, None).items()
                                 if key in ("phase", "step", "wizard.cpu")}
    body["ready"]["still"] = str(ctx.shot(f"{name}-ready"))
    if body["ready"]["practice"].get("wizard.cpu"):
        problems.append(f"a running drill still reports wizard.cpu: {body['ready']['practice']}")
    # The observations are kept before the tail that puts the game back. Stop and the close run once: a failure
    # of either in a one-Eden boot is a finding (tr-primary-2, 2026-11-23 19:26:28Z), not something to retry.
    _save(ctx, name, {**body, "problems": problems})
    edge._stop(ctx, scene, ident)
    edge._close(ctx, scene)
    return _finish(ctx, name, body, problems)


def fullscreen(ctx):
    """Stills at the display's size: the owned Eden goes fullscreen; the display's size is recorded, not required."""
    target = "action_Fullscreen"

    def action():
        tree = ctx.call("ui.tree", {"target": target})
        return next(node for node in tree if "checked" in node.get("properties", {}))

    if action()["properties"]["checked"]["value"] is not True:
        ctx.call("ui.trigger", {"target": target})
    time.sleep(1.5)
    config = json.loads((Path(ctx.runner.out).parent / "config.json").read_text())
    instance = next(item for item in config["instances"] if item["key"] == ctx.on)
    screen = subprocess.check_output(["/usr/bin/xwininfo", "-display", instance["display"], "-root"], text=True, timeout=4)
    size = {axis: int(re.search(rf"{axis}: (\d+)", screen)[1]) for axis in ("Width", "Height")}
    resolution = ctx.call("settings.get", {"key": "core.resolution_setup"})
    return _save(ctx, "fullscreen", {"display": instance["display"], "display_size": size,
                                     "fullscreen": action()["properties"]["checked"]["value"],
                                     "core.resolution_setup": resolution, "still": str(ctx.shot("fullscreen"))})
