"""F11 rows (tile-fix-g4, 2026-11-23): a picker's CPU start is shown by the CPU itself.

Guard the Edge ("6") and Juggler ("7") pickers, driven through the menu and crosshair helpers flown in
flight-755d9936 (edge_steps.py). Every observation reads three things and keeps a still:
  tr.practice   wizard.step, wizard.x/y and wizard.cpu (in_view | magnifier; the second is the game's FLAG_IS_LOUPE)
  fighters      where the CPU's body is (bridge `fighters`)
  tr.menu       the picker's notice ("CPU (x, y) ...")
Each row saves f11-<name>.json in its folder and raises when an expectation given in its args is not met.
"""
import json
import re
import subprocess
import sys
import time
from pathlib import Path

import routes
import switch_hw

sys.path.insert(0, str(Path(__file__).resolve().parent))
import edge_steps as edge  # noqa: E402

PRACTICE = "tr.practice"


def _save(ctx, name, body):
    (Path(ctx.row_dir) / f"f11-{name}.json").write_text(json.dumps(body, indent=2) + "\n")
    return body


def _cpu_body(ctx, scene):
    run = routes.instance_path(ctx, "bridge_log").parent
    bodies = switch_hw.bridge_state(run, ("fighters",), cap=4)["fighters"]
    body = bodies.get(4 if scene == "lobby" else 1, {})
    return {key: body.get(key) for key in ("x", "y", "situation", "status", "dmg")}


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


def _at_point(seen, tolerance=0.6):
    """The CPU's body stands on the picker's point (the bridge's fighters against the practice state)."""
    try:
        return (abs(float(seen["cpu_body"]["x"]) - float(seen["wizard"]["wizard.x"])) <= tolerance
                and abs(float(seen["cpu_body"]["y"]) - float(seen["wizard"]["wizard.y"])) <= tolerance)
    except (KeyError, TypeError, ValueError):
        return None


def _finish(ctx, name, body, problems):
    body["problems"] = problems
    _save(ctx, name, body)
    if problems:
        raise RuntimeError(f"{name}: {problems}")
    return {key: body[key] for key in ("origin", "back") if key in body} | {
        "seen": [(s["name"], s["wizard"].get("wizard.cpu"), s["cpu_body"].get("x"), s["cpu_body"].get("y"))
                 for s in body.get("seen", [])]}


def picker(ctx, ident="6", scene="training", points=(), expect=None, leave=True, name="picker"):
    """Open a drill's picker (its CPU step), walk the crosshair through `points`, then leave it with B.

    `expect`: one wizard.cpu word per observation (the default point first, then each of `points`); None records only.
    With `leave`, B backs out to the menu and the CPU must be back where it stood before the picker.
    """
    origin = _cpu_body(ctx, scene)
    state = edge._setup(ctx, scene, ident)
    wanted = list(expect) if expect else [None] * (len(points) + 1)
    seen = [_observe(ctx, scene, state, f"{name}-00-default", expect=wanted[0])]
    for index, point in enumerate(points, 1):
        state = edge._move(ctx, scene, state, tuple(point))
        seen.append(_observe(ctx, scene, state, f"{name}-{index:02d}-{point[0]}_{point[1]}", expect=wanted[index]))
    body = {"ident": ident, "scene": scene, "origin": origin, "seen": seen}
    problems = [f"{s['name']}: {s['problem']}" for s in seen if s.get("problem")]
    problems += [f"{s['name']}: the CPU's body is not on the point: {s['cpu_body']} vs {s['wizard']}"
                 for s in seen if _at_point(s) is False]
    if leave:
        edge._press(ctx, scene, "B", open="1")
        time.sleep(1.0)
        body["back"] = _cpu_body(ctx, scene)
        body["back_still"] = str(ctx.shot(f"{name}-left"))
        edge._close(ctx, scene)
        try:
            moved = (abs(float(body["back"]["x"]) - float(origin["x"])) > 0.6
                     or abs(float(body["back"]["y"]) - float(origin["y"])) > 0.6)
        except (TypeError, ValueError):
            moved = True
        if moved:
            problems.append(f"after B the CPU is at {body['back']}, it stood at {origin}")
    return _finish(ctx, name, body, problems)


def through(ctx, ident="6", scene="training", cpu=(-100, -20), player=(-40, 0), expect=None, start=True,
            name="through"):
    """The whole picker: CPU point, (Juggler: percent,) player point, preview; then start the drill and stop it.

    `expect`: wizard.cpu words for the CPU step, the player step and the preview. Starting must reach Ready with the
    CPU on its start, as before the fix.
    """
    pages = {"6": (1, 2, 3), "7": (1, 3, 4)}[ident]
    wanted = list(expect) if expect else [None] * 3
    state = edge._move(ctx, scene, edge._setup(ctx, scene, ident), tuple(cpu))
    seen = [_observe(ctx, scene, state, f"{name}-cpu", expect=wanted[0])]
    if ident == "7":
        edge._select(ctx, scene, ident, 2)
    state = edge._select(ctx, scene, ident, pages[1])
    state = edge._move(ctx, scene, state, tuple(player))
    seen.append(_observe(ctx, scene, state, f"{name}-player", step="player", expect=wanted[1]))
    state = edge._select(ctx, scene, ident, pages[2])
    seen.append(_observe(ctx, scene, state, f"{name}-preview", step="preview", expect=wanted[2]))
    body = {"ident": ident, "scene": scene, "seen": seen}
    problems = [f"{s['name']}: {s['problem']}" for s in seen if s.get("problem")]
    # The CPU waits on its own start in every step, also while the crosshair is the player's.
    for s in seen:
        try:
            on_start = (abs(float(s["cpu_body"]["x"]) - cpu[0]) <= 0.6 and abs(float(s["cpu_body"]["y"]) - cpu[1]) <= 0.6)
        except (TypeError, ValueError):
            on_start = False
        if not on_start:
            problems.append(f"{s['name']}: the CPU's body is at {s['cpu_body']}, its start is {cpu}")
    if start:
        before = edge._read(ctx, scene)
        ctx.play(edge.TAP["A"])
        edge._ready(ctx, scene, before, ident, None, None, None)
        time.sleep(0.5)
        body["ready"] = {"cpu_body": _cpu_body(ctx, scene), "practice": {
            key: value for key, value in _practice(ctx, None).items() if key in ("phase", "step", "wizard.cpu")},
            "still": str(ctx.shot(f"{name}-ready"))}
        if body["ready"]["practice"].get("wizard.cpu"):
            problems.append(f"a running drill still reports wizard.cpu: {body['ready']['practice']}")
        _save(ctx, name, body)  # the observations are kept before the tail that puts the game back
        body["tail"] = _stop_and_close(ctx, scene, ident)
    return _finish(ctx, name, body, problems)


def _stop_and_close(ctx, scene, ident):
    """Stop the running drill and close the menu. In f11-probe-1 each failed once with a second Eden on the host
    (a Stop press without effect during a presentation stall; a close on an inactive ssbu.ui read), so each gets one
    more try, and the tries are recorded."""
    tail = {"stop_tries": 0, "close_tries": 0}
    for attempt in (1, 2):
        tail["stop_tries"] = attempt
        try:
            edge._stop(ctx, scene, ident)
            break
        except RuntimeError as error:
            tail[f"stop_error_{attempt}"] = repr(error)[:240]
            if attempt == 2:
                raise
            time.sleep(3.0)
    for attempt in (1, 2):
        tail["close_tries"] = attempt
        try:
            edge._close(ctx, scene)
            break
        except RuntimeError as error:
            tail[f"close_error_{attempt}"] = repr(error)[:240]
            if edge._read(ctx, scene)["values"].get("open") == "0":
                break
            if attempt == 2:
                raise
            time.sleep(3.0)
    return tail


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
