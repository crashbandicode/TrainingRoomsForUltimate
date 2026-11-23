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


FIGHTER_LINE = re.compile(r"\[Bridge\] fighter entry=(\d+) kind=(-?\d+) x=(\S+) y=(\S+) lr=\S+ dmg=\S+ status=(-?\d+) "
                          r"situation=(-?\d+) stock=-?\d+ dead=(-?\d+) ")


def fighter_lines(lines, entry):
    """The bridge's own fighter lines of one entry, in order, as the bodies the checks compare."""
    found = []
    for line in lines:
        match = FIGHTER_LINE.search(line)
        if match and int(match.group(1)) == entry:
            kind, x, y, status, situation, dead = match.group(2, 3, 4, 5, 6, 7)
            found.append({"kind": int(kind), "x": float(x), "y": float(y), "status": int(status),
                          "situation": int(situation), "dead": int(dead), "line": line.strip()[:200]})
    return found


def put_back(lines, entry):
    """Where the put-back left the body: the entry's first fighter line after the put-back's status event (Wait from
    Fall, grounded). Judged there, not a second later, when a CPU may have moved by itself (tr-primary-2, 2026-11-23
    22:09:57Z). None while either line is still to come. Pure."""
    event = re.compile(rf"cpu_event status entry={entry} .*status=0 prev=14 situation=0 ")
    at = next((index for index, line in enumerate(lines) if event.search(line)), None)
    if at is None:
        return None
    return next(iter(fighter_lines(lines[at + 1:], entry)), None)


def put_back_problem(body, origin, kind):
    """Why the put-back's line is not the fighter `kind` standing at the origin (None: it is). Pure; no tolerance."""
    if body is None:
        return "no fighter line followed a put-back event"
    if body.get("kind") != kind:
        return f"the first line after the put-back is kind {body.get('kind')!r}, not the held fighter {kind}: {body!r}"
    return back_problem(body, origin)


def attach(x, a):
    """The x the floor-attach hypothesis predicts for a body placed at `x` on a floor whose reference is `a`."""
    return _f32(_f32(_f32(x) - _f32(a)) + _f32(a))


def attach_verdict(placed, read, candidates):
    """Which of `candidates` ({name: a}) explains the x read after a floor placement at `placed`: (name, detail).
    ('SETUP', why) when this x gives the candidates the same word; ('none', why) when no candidate fits the read.
    The detail says whether the fitting word is also the placed word: such a read tells the candidates apart but
    does not show an attach by itself (at a whole x from 43 to 48 the floor end leaves x as placed)."""
    words = {name: _word(attach(placed, a)) for name, a in candidates.items()}
    if len(set(words.values())) != len(words):
        return "SETUP", f"x {placed!r} does not tell the candidates apart: {words}"
    fits = [name for name, word in words.items() if word == _word(read)]
    if not fits:
        return "none", f"read {_word(read)} {read!r}; placed {_word(placed)}; candidates {words}"
    return fits[0], {"words": words, "placed": _word(placed), "read": _word(read),
                     "read_is_the_placed_word": _word(read) == _word(placed)}


def _log_mark(ctx):
    log = Path(routes.instance_path(ctx, "bridge_log"))
    return log, log.stat().st_size


def _lines_since(mark, until=None):
    """The log's lines after `mark`, up to the later mark `until` when given."""
    log, offset = mark
    with log.open("rb") as handle:
        handle.seek(offset)
        data = handle.read() if until is None else handle.read(max(0, until[1] - offset))
    return data.decode(errors="replace").splitlines()


def _await(ctx, mark, find, cap=4.0):
    """`find(lines since the mark)` once it is not None, or None after `cap` seconds."""
    until = time.monotonic() + cap
    while True:
        ctx.check_stop()
        found = find(_lines_since(mark))
        if found is not None or time.monotonic() > until:
            return found
        time.sleep(0.1)


def _save(ctx, name, body):
    (Path(ctx.row_dir) / f"f11-{name}.json").write_text(json.dumps(body, indent=2) + "\n")
    return body


def _cpu_body(ctx, scene):
    run = routes.instance_path(ctx, "bridge_log").parent
    bodies = switch_hw.bridge_state(run, ("fighters",), cap=4)["fighters"]
    body = bodies.get(4 if scene == "lobby" else 1, {})
    return {key: body.get(key) for key in ("kind", "x", "y", "situation", "status", "dmg", "dead")}


def _entry(scene):
    return 4 if scene == "lobby" else 1


def at_rest(body, earlier):
    """Two fighters reads in a row show the CPU standing still: grounded, with the same x and y words. Pure."""
    try:
        now = [_word(body["x"]), _word(body["y"]), body["situation"]]
        before = [_word(earlier["x"]), _word(earlier["y"]), earlier["situation"]]
    except (KeyError, TypeError, ValueError):
        return False
    return now == before and body["situation"] == 0


def _fixture(ctx, scene, cpu_kind, cap=10):
    """The CPU before the picker: the fighter the row's gravity belongs to, standing still. A CPU that is in the air
    when the picker opens is put back at that air point by B and falls, so its place after B is not the read before
    (f11-proof-1, first f11-t3: read at y 15.2 right after the row before had stopped its drill). The row waits for
    rest; a CPU that does not come to rest within `cap` seconds is a fixture error."""
    until, earlier = time.monotonic() + cap, None
    while True:
        ctx.check_stop()
        body = _cpu_body(ctx, scene)
        if body.get("kind") != cpu_kind:
            raise RuntimeError(f"FIXTURE: the CPU's kind is {body.get('kind')!r}; this row's gravity is for kind {cpu_kind}")
        if earlier is not None and at_rest(body, earlier):
            return body
        if time.monotonic() > until:
            raise RuntimeError(f"FIXTURE: the CPU did not come to rest within {cap} s: {body!r}")
        earlier = body
        time.sleep(0.5)


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


def _finish(ctx, name, body, problems, unverified=()):
    """`problems`: clauses that failed. `unverified`: clauses whose witness is missing; they are not failures of the
    product, and the row does not pass either."""
    body["problems"], body["unverified"] = problems, list(unverified)
    _save(ctx, name, body)
    if problems or unverified:
        raise RuntimeError(f"{name}: FAILED {problems}; UNVERIFIED {list(unverified)}")
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
    mark = _log_mark(ctx)
    edge._press(ctx, scene, "B", open="1")
    body["back"] = _await(ctx, mark, lambda lines: put_back(lines, _entry(scene)))
    body["back_still"] = str(ctx.shot(f"{name}-left"))
    edge._close(ctx, scene)
    back = put_back_problem(body["back"], origin, cpu_kind)
    if back:
        problems.append(back)
    return _finish(ctx, name, body, problems)


def swap_back(ctx, cpu_kind, drill_kind, drill_choice, drill_gravity, ident="6", scene="training", name="swap-back"):
    """Row f11-t4: a drill with its own CPU fighter, its picker, B. The drill's fighter must be put back where the
    CPU stood, and the original fighter must come back there, standing, with its KO count unchanged.

    `drill_choice`: the fighter's index in the drill's fighter setting (0 is Current CPU); `drill_kind` and
    `drill_gravity`: that fighter's kind and one gravity step, for the held check. The setting is put back at the end.
    Red take: early install 2 boot, Yoga, f11-t1 (the first line after the put-back was the original, at the held
    point, in the air; it was KO'd).
    """
    key = "edge_cpu_fighter" if ident == "6" else "juggle_cpu_fighter"
    entry = _entry(scene)
    origin = _fixture(ctx, scene, cpu_kind)
    before = list(routes._current_settings(ctx, 8)["menu"][key])
    chosen = [int(index == drill_choice) for index in range(len(before))]
    body = {"ident": ident, "scene": scene, "origin": origin, "setting": key, "setting_before": before}
    problems, unverified = [], []
    if not isinstance(origin.get("dead"), int):
        unverified.append(f"the KO count: none read before the picker: {origin!r}")
    try:
        routes.tr_settings(ctx, values={key: chosen})
        if list(routes._current_settings(ctx, 8)["menu"][key]) != chosen:
            raise RuntimeError(f"SETUP: {key} did not read back choice {drill_choice}")
        state = edge._setup(ctx, scene, ident)
        seen = _observe(ctx, scene, state, f"{name}-held")
        body["seen"] = [seen]
        if seen["cpu_body"].get("kind") != drill_kind:
            raise RuntimeError(f"SETUP: the picker holds kind {seen['cpu_body'].get('kind')!r}, not the drill's {drill_kind}")
        held = held_problem(seen["cpu_body"], _published(seen), drill_gravity)
        if held or seen.get("problem"):
            problems.append(f"{seen['name']}: {held or seen['problem']}")
        mark = _log_mark(ctx)
        edge._press(ctx, scene, "B", open="1")
        # The put-back is read before the menu is closed.
        body["back"] = _await(ctx, mark, lambda lines: put_back(lines, entry))
        if body["back"] is None:
            unverified.append("the put-back: no fighter line followed a put-back event within 4 s of B")
        else:
            back = put_back_problem(body["back"], origin, drill_kind)
            if back:
                problems.append(back)
        body["back_still"] = str(ctx.shot(f"{name}-menu"))
        # The close is the boundary: the roster does not change the CPU under the open menu, so the original must
        # not be in the slot before it and its first body is read after it.
        close = _log_mark(ctx)
        under_menu = [line for line in fighter_lines(_lines_since(mark, close), entry) if line["kind"] == cpu_kind]
        body["under_menu"] = under_menu[:1]
        if under_menu:
            problems.append(f"the original fighter was in the slot before the menu closed: {under_menu[0]}")
        edge._close(ctx, scene)
        arrived = _await(ctx, close, lambda lines: next(
            (line for line in fighter_lines(lines, entry) if line["kind"] == cpu_kind), None), cap=12.0)
        last = [line for line in fighter_lines(_lines_since(mark), entry) if line["kind"] == drill_kind][-1:]
        body["original"], body["drill_last"] = arrived, (last[0] if last else None)
        if arrived is None:
            unverified.append(f"the original's return: no line of kind {cpu_kind} within 12 s of the menu closing")
        elif not last:
            unverified.append("the original's place: the drill's fighter wrote no line after B to compare it with")
        else:
            took = back_problem(arrived, last[0])
            if took:
                problems.append(f"the original did not take the drill fighter's place: {took}")
            if scene == "training" and back_problem(arrived, origin):
                problems.append(f"the original came back away from the CPU's place: {back_problem(arrived, origin)}")
        if arrived is not None:
            rest = _fixture(ctx, scene, cpu_kind)
            body["rest"] = rest
            if not isinstance(rest.get("dead"), int):
                unverified.append(f"the KO count: none read after the return: {rest!r}")
            elif isinstance(origin.get("dead"), int) and rest["dead"] != origin["dead"]:
                problems.append(f"the KO count went from {origin['dead']} to {rest['dead']}")
        body["original_still"] = str(ctx.shot(f"{name}-original"))
    finally:
        _save(ctx, name, {**body, "problems": problems, "unverified": unverified})
        routes.tr_settings(ctx, values={key: before})
        if list(routes._current_settings(ctx, 8)["menu"][key]) != before:
            raise RuntimeError(f"SETUP: {key} was not put back")
    return _finish(ctx, name, body, problems, unverified)


def floor_attach(ctx, ident="6", scene="training", cpu=(-100, -20), player=(45, 0), floor_end=-79.9856,
                 ledge=-85.9856, name="p1-placement-control"):
    """A CONTROL on Player 1's placement (rows f11-c5 and f11-wc5). It is not the deciding row for the room's
    one-step x after a put-back: that row parks the partner and leaves the Juggler picker with B, and is not written
    (tr-primary-2, 2026-11-23 23:12:40Z). This control's result does not verify that clause.

    Hypothesis: a body placed on a floor gets x = f32(f32(x - a) + a) for a reference `a` at the floor's left side.
    The readings so far do not tell the floor's end from the ledge. No flown tool parks a room partner at a chosen x,
    so this row uses the same placement call on Player 1 (the drill's start stands Player 1 on the picked Player
    point). At a whole x from 43 to 48 the ledge gives the word one step below and the floor end leaves x as placed.
    The row records which one Player 1's first standing line after the start has; it fails only when neither fits.
    It tells the two candidates apart. It does not test the hypothesis against "no attach at all": that rests on
    the room's Juggler reading, where x did change.
    """
    state = edge._move(ctx, scene, edge._setup(ctx, scene, ident), tuple(cpu))
    state = edge._move(ctx, scene, edge._select(ctx, scene, ident, 2), tuple(player))
    edge._select(ctx, scene, ident, 3)
    point = edge_ready_exact.save_picked(ctx, keep=False)
    placed = float(point["p1"][0])
    before = edge._read(ctx, scene)
    mark = _log_mark(ctx)
    ctx.play(edge.TAP["A"])
    edge.observe(ctx, scene, lambda values: values["game.running"] == "1" and values["notice.body"].startswith("Ready"),
                 "the running drill reached Ready", before, cap=3)
    stood = _await(ctx, mark, lambda lines: next(
        (line for line in fighter_lines(lines, 0) if line["situation"] == 0 and line["status"] == 0), None))
    body = {"ident": ident, "scene": scene, "picked": point, "placed_x": placed, "player_line": stood,
            "candidates": {"floor_end": floor_end, "ledge": ledge}}
    if stood is None:
        verdict, detail = "none", "Player 1 wrote no standing line after the start"
    else:
        verdict, detail = attach_verdict(placed, stood["x"], body["candidates"])
    body["verdict"], body["detail"] = verdict, detail
    _save(ctx, name, body)
    edge._stop(ctx, scene, ident)
    edge._close(ctx, scene)
    if verdict in ("SETUP", "none"):
        raise RuntimeError(f"{name}: {verdict}: {detail}")
    return {"verdict": verdict, "placed_x": placed, "read_x": stood["x"], "words": detail}


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
