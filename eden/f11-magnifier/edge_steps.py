"""Combined-proof checks for the Edge lane, driven by live bridge state."""

from pathlib import Path
import re
import sys
import time

import edge_ready_exact
import routes
import state_gate
import switch_hw

sys.path.insert(0, str(Path(__file__).resolve().parent))
from rm_state_steps import menu, press_for_state
from rm_lobby_steps import lobby_close, lobby_goto


GAME_PAGE = "fighters_stages"
TAP = {"A": "rm-tap-a", "B": "rm-tap-b", "Down": "rm-tap-down",
       "Left": "ej-left", "Right": "ej-right", "Up": "ej-up",
       "X": "ej-x", "Y": "ej-y"}


def observe(ctx, scene, accept, description, before=None, cap=8, ack=False):
    def verify(block):
        if block["schema"] != "tr.menu/1" or block["filtered"]:
            return False, "expected complete tr.menu/1"
        okay, reason = state_gate.satisfies(
            block, {"scene": scene, "keyboard": "0"}, baseline=before,
            ack=1 if ack else None, input_gate=ack)
        if not okay:
            return False, reason
        return (True, description) if accept(block["values"]) else (False, description)

    block, reason = ctx.runner._state_block(ctx.on, "tr.menu", cap, verify, ask_every=.08)
    ctx.log({"edge_menu": block, "checked": reason})
    return block


def _goto(ctx, scene, page, path):
    if scene == "lobby":
        return lobby_goto(ctx, page, path)
    routes.tr_goto(ctx, page, path)


def _close(ctx, scene):
    if scene == "lobby":
        return lobby_close(ctx)
    routes.tr_close(ctx)


def _read(ctx, scene, **where):
    return menu(ctx, {"scene": scene, "keyboard": "0", **where})


def _press(ctx, scene, button, *, before=None, cap=8, **where):
    return press_for_state(ctx, TAP[button], {"scene": scene, "keyboard": "0", **where},
                           cap=cap, before=before)


def _index(block):
    return int(block["values"]["list.index"])


def _steps(before, after, delta=1):
    first, last = before["values"], after["values"]
    if first["list"] != last["list"] or first["list.count"] != last["list.count"]:
        raise RuntimeError(f"list identity changed during hold: {first!r} to {last!r}")
    return ((_index(after) - _index(before)) * delta) % int(first["list.count"])


def _top_cpu(ctx, scene):
    _goto(ctx, scene, "cpu", ["mash_state"])
    state = _read(ctx, scene, **{"open": "1", "list": "page", "list.index": "0"})
    if int(state["values"]["list.count"]) < 20:
        raise RuntimeError("list must expose at least 20 focus positions for the long hold")
    return state


def _hold_steps(ctx, scene, segment):
    before = _top_cpu(ctx, scene)
    ctx.play(segment)
    after = observe(ctx, scene,
                    lambda values: values["list.index"] != str(_index(before)),
                    "held direction changes list.index", before)
    count = _steps(before, after)
    ctx.log({"edge_hold": segment, "steps": count, "before": before, "after": after})
    return count


def scroll(ctx, row, scene="training"):
    if row == "S1":
        if _hold_steps(ctx, scene, "hold-ddown-30f") < 2:
            raise RuntimeError("30-frame Down must repeat")
    elif row in ("S2", "S3"):
        prefix = "hold-ddown" if row == "S2" else "hold-lstick-down"
        one = _hold_steps(ctx, scene, f"{prefix}-60f")
        two = _hold_steps(ctx, scene, f"{prefix}-120f")
        if one <= 1 or two - one <= one:
            raise RuntimeError(f"expected repeat acceleration, saw {one} and {two} steps")
        if row == "S2":
            before = _top_cpu(ctx, scene)
            ctx.play("hold-dup-60f")
            after = observe(ctx, scene, lambda values: values["list.index"] != "0",
                            "held Up changes list.index", before)
            if _steps(before, after, -1) <= 1:
                raise RuntimeError("60-frame Up must repeat in the upward direction")
    elif row == "S4":
        # The numeric zoom row has exact adjacent values, including away from the ends.
        _goto(ctx, scene, GAME_PAGE, ["game.6", "edge_zoom_window"])
        before = _read(ctx, scene, **{"open": "1", "list": "rows/26"})
        get_number = lambda block: int(re.search(r"\d+", block["values"]["list.value"]).group())
        start = get_number(before)
        ctx.play(TAP["Right"])
        single = observe(ctx, scene, lambda v: v["list.value"] != before["values"]["list.value"],
                         "one Right changes the numeric setting", before)
        ctx.play("hold-dright-60f")
        held = observe(ctx, scene, lambda v: v["list.value"] != single["values"]["list.value"],
                       "held Right changes the numeric setting", single)
        if any(_steps(before, state) != 0 for state in (single, held)):
            raise RuntimeError("Right changed the focused row")
        if (get_number(single), get_number(held)) != (start + 1, start + 2):
            raise RuntimeError("held Right must make exactly one numeric change")
        for reverse in (held, single):
            ctx.play(TAP["Left"])
            observe(ctx, scene, lambda v: int(re.search(r"\d+", v["list.value"]).group())
                    == get_number(reverse) - 1, "restore the original zoom value", reverse)
    elif row == "S5":
        first = _top_cpu(ctx, scene)
        _close(ctx, scene)
        before = _read(ctx, scene, **{"open": "0"})
        ctx.play("open-menu-hold-dup-60f")
        opened = observe(ctx, scene, lambda v: v["open"] == "1", "menu opened", before)
        if _steps(first, opened) != 0:
            raise RuntimeError("Up held from the opening chord moved the focus")
        ctx.play("press-dup-6f")
        after = observe(ctx, scene, lambda v: v["list.index"] != opened["values"]["list.index"],
                        "fresh Up changes focus", opened)
        if _steps(opened, after, -1) != 1:
            raise RuntimeError("fresh Up did not move by exactly one")
    elif row in ("S6", "S7"):
        if scene == "training":
            routes.tr_open(ctx, page="recording")
        else:
            from rm_lobby_steps import lobby_open
            lobby_open(ctx, page="recording")
        section = "recording/page/8/0" if row == "S6" else "recording/page/8/1"
        before = _read(ctx, scene, **{"list": section, "list.index": "0"})
        count = int(before["values"]["list.count"])
        if row == "S6":
            if not 12 < count < 25:
                raise RuntimeError(f"S6 needs a 13–24 row save list, saw {count}")
            for expected in (12, count - 1):
                press_for_state(ctx, "ej-page-down", {"scene": scene, "list": section,
                                                      "list.index": str(expected)})
            press_for_state(ctx, "ej-page-down", {"scene": scene, "list": "recording/page/8/1",
                                                  "list.index": "0"})
            return  # Leave S7 at the following section, index zero.
        if count <= 36:
            raise RuntimeError(f"S7 needs more than 36 macro rows, saw {count}")
        ctx.play("hold-zl-ddown-60f")
        after = observe(ctx, scene, lambda v: v["list.index"] != "0",
                        "held page-down moved the list", before)
        measured = _steps(before, after)
        if measured not in (24, 36):
            raise RuntimeError(f"expected two or three 12-row jumps, saw displacement {measured}")
    elif row == "S8":
        _close(ctx, scene)
        closed = _read(ctx, scene, **{"open": "0", "recording.popup.open": "0"})
        ctx.play("rm-popup-chord")
        def popup_list(values):
            return (values["open"] == "0" and values["recording.popup.open"] == "1"
                    and values["list"].startswith("recording/popup/")
                    and int(values["list.count"]) > 1)

        try:
            first = observe(ctx, scene, popup_list, "standalone popup owns the published list", closed)
            if _index(first) == int(first["values"]["list.count"]) - 1:
                ctx.play(TAP["Up"])
                first = observe(ctx, scene, lambda v: popup_list(v) and
                                int(v["list.index"]) < int(v["list.count"]) - 1,
                                "move off the last entry before testing Down", first)
            ctx.play(TAP["Down"])
            second = observe(ctx, scene, lambda v: popup_list(v) and
                             v["list.index"] != first["values"]["list.index"],
                             "one Down moves the standalone popup list", first)
            if _steps(first, second) != 1:
                raise RuntimeError("one Down must advance the popup's published list exactly once")
        finally:
            if _read(ctx, scene)["values"]["recording.popup.open"] == "1":
                _press(ctx, scene, "B", **{"open": "0", "recording.popup.open": "0", "list": ""})
        return
    else:
        raise ValueError(f"unsupported scroll row {row}")
    _close(ctx, scene)


def game_lists(ctx, row, scene="lobby"):
    titles = ("Eggs-ercise", "Break the Targets", "Tech practice", "Mash out",
              "Parry practice", "Guard the Edge", "Juggler")
    if row == "W1":
        _goto(ctx, scene, GAME_PAGE, ["game.1"])
        before = _read(ctx, scene, **{"list": "page", "action": "game.1", "row": titles[0]})
        for number, title in enumerate(titles[1:], 2):
            after = _press(ctx, scene, "Down", **{"list": "page", "action": f"game.{number}", "row": title})
            if _steps(before, after) != 1:
                raise RuntimeError(f"expected single Down between practice games, saw {after!r}")
            before = after
    else:
        for ident, group in (("6", "26"), ("7", "27")):
            titles = ["Start", "CPU fighter and mode", "Reaction", "Set up…"]
            if ident == "6":
                titles.append("CPU percent")
            titles += ["Attempts", "Zoom landing window"]
            _goto(ctx, scene, GAME_PAGE, [f"game.{ident}", f"start.{ident}"])
            common = {"list": f"rows/{group}", "list.count": str(len(titles))}
            _read(ctx, scene, **common, row="Start", action=f"start.{ident}",
                  **{"list.index": "0", "list.value": ">"})
            for index, title in enumerate(titles[1:], 1):
                _press(ctx, scene, "Down", **common, row=title, **{"list.index": str(index)})
    _close(ctx, scene)


def _wizard(ctx, scene, ident, page):
    header = f"Edge · {page}/3" if ident == "6" else f"Juggle · {page}/4"
    return _read(ctx, scene, **{"open": "0", "notice.header": header})


def _setup(ctx, scene, ident):
    _goto(ctx, scene, GAME_PAGE, [f"game.{ident}", {"row": "Set up…"}])
    _press(ctx, scene, "A", open="0")
    return _wizard(ctx, scene, ident, 1)


def _select(ctx, scene, ident, page):
    header = f"Edge · {page}/3" if ident == "6" else f"Juggle · {page}/4"
    return _press(ctx, scene, "A", **{"open": "0", "notice.header": header})


def _notice(block):
    return block["values"]["notice.body"]


def _point(body):
    match = re.search(r"^\w+ \((-?\d+), (-?\d+)\)", body)
    if not match:
        match = re.search(r"^CPU x (-?\d+) · Height ([+-]?\d+)", body)
    if not match:
        raise RuntimeError(f"no crosshair point in {body!r}")
    return tuple(map(int, match.groups()))


def _expect_body(block, prefix):
    if not _notice(block).startswith(prefix):
        raise RuntimeError(f"expected {prefix!r}, saw {_notice(block)!r}")
    return block


def _tap_point(ctx, scene, before, button):
    ctx.play(TAP[button])
    return observe(ctx, scene,
                   lambda current: current["notice.header"] == before["values"]["notice.header"]
                   and current["notice.body"] != _notice(before),
                   f"{button} moves the current crosshair", before)


def _move(ctx, scene, before, to):
    """Use measured displacement for long moves and one-unit D-pad steps for the tail."""
    for axis, (negative, positive) in enumerate((("Left", "Right"), ("Down", "Up"))):
        target = to[axis]
        for _ in range(30):
            point = _point(_notice(before))
            gap = target - point[axis]
            if gap == 0:
                break
            if abs(gap) > 16:
                strength = 1 if gap > 0 else -1
                span = min(350, max(60, round(abs(gap) * 5)))
                ctx.stick(strength if axis == 0 else 0, strength if axis == 1 else 0, span)
                before = observe(ctx, scene, lambda v: v["notice.body"] != _notice(before),
                                 "stick changed the crosshair", before)
            else:
                before = _tap_point(ctx, scene, before, positive if gap > 0 else negative)
        else:
            raise RuntimeError(f"crosshair could not reach {to!r}: {_notice(before)!r}")
    return before


def _refuse(ctx, scene, before, text):
    state = _press(ctx, scene, "A", cap=2, **{"notice.header": "Input Recording", "notice.body": text})
    ctx.log({"edge_refusal_previous": before, "edge_refusal": state})
    return state


def _ready(ctx, scene, before, ident, player, cpu, damage):
    _press_ready = lambda values: values["game.running"] == "1" and values["notice.body"].startswith("Ready")
    state = observe(ctx, scene, _press_ready, "running drill reached Ready", before, cap=3)
    if player is not None:
        run = routes.instance_path(ctx, "bridge_log").parent
        bodies = switch_hw.bridge_state(run, ("fighters",), cap=3)["fighters"]
        partner = 4 if scene == "lobby" else 1
        p1, other = bodies.get(0, {}), bodies.get(partner, {})
        ctx.log({"edge_ready": state, "fighters": bodies})
        if (abs(p1.get("x", float("inf")) - player[0]) > .02
                or p1.get("situation") != 0 or abs(other.get("x", float("inf")) - cpu[0]) > .02
                or abs(other.get("y", float("inf")) - cpu[1]) > .12 or other.get("dmg") != damage):
            raise RuntimeError(f"unexpected bodies during Ready: {bodies!r}")


def _stop(ctx, scene, ident):
    _goto(ctx, scene, GAME_PAGE, [f"game.{ident}", f"start.{ident}"])
    _press(ctx, scene, "A", **{"game.running": "0"})


def training_crosshair(ctx, row, scene="training"):
    if row == "G1":
        _expect_body(_setup(ctx, scene, "6"), "CPU (-94, -30)")
    elif row == "G2":
        state = _wizard(ctx, scene, "6", 1)
        for button in ("Right", "Up", "Up", "Up"):
            state = _tap_point(ctx, scene, state, button)
        _expect_body(state, "CPU (-93, -27)")
    elif row == "G3":
        before = _wizard(ctx, scene, "6", 1)
        ctx.stick(-1, 0, 300)
        state = observe(ctx, scene, lambda v: v["notice.body"] != _notice(before),
                        "left stick moves the Guard crosshair", before)
        if _point(_notice(state))[0] > _point(_notice(before))[0] - 20:
            raise RuntimeError("left stick moved fewer than 20 units")
    elif row == "G4":
        for button, body in (("X", "CPU (-94, -30)"), ("Y", "CPU (94, -30)"), ("Y", "CPU (-94, -30)")):
            before = _wizard(ctx, scene, "6", 1)
            ctx.play(TAP[button])
            observe(ctx, scene, lambda v: v["notice.body"].startswith(body),
                    f"{button} selects {body}", before)
    elif row == "G7":
        _move(ctx, scene, _wizard(ctx, scene, "6", 1), (-100, -20))
        _expect_body(_select(ctx, scene, "6", 2), "P1 (")
    elif row == "G8":
        _move(ctx, scene, _wizard(ctx, scene, "6", 2), (-40, 0))
        _expect_body(_select(ctx, scene, "6", 3), "CPU (-100, -20) · 100% · P1 (-40, 0)")
    elif row in ("G9", "G10"):
        if row == "G10":
            _stop(ctx, scene, "6")
            _goto(ctx, scene, GAME_PAGE, ["game.6", "start.6"])
            point = edge_ready_exact.load_picked(ctx)
        else:
            point = edge_ready_exact.save_picked(ctx)
        before = _read(ctx, scene)
        watch = edge_ready_exact.open_trace(ctx)
        ctx.play(TAP["A"])
        _ready(ctx, scene, before, "6", None, None, None)
        edge_ready_exact.check_ready(ctx, scene, point, watch, -40, -100, 100)
    elif row == "G11":
        _stop(ctx, scene, "6")
        _setup(ctx, scene, "6")
        _select(ctx, scene, "6", 2)
        _select(ctx, scene, "6", 3)
        for header in ("Edge · 2/3", "Edge · 1/3"):
            _press(ctx, scene, "B", **{"notice.header": header})
        _press(ctx, scene, "B", open="1")
        _close(ctx, scene)
    elif row == "J1":
        _expect_body(_setup(ctx, scene, "7"), "CPU x")
    elif row == "J3":
        _move(ctx, scene, _wizard(ctx, scene, "7", 1), (0, 65))
        state = _expect_body(_select(ctx, scene, "7", 2), "60%")
        for button in ("Right", "Right", "Up"):
            state = _tap_point(ctx, scene, state, button)
        _expect_body(state, "72%")
    elif row == "J4":
        start = _select(ctx, scene, "7", 3)
        at_x = _move(ctx, scene, start, (-38, _point(_notice(start))[1]))
        ctx.stick(0, 1, 330)
        platform = observe(ctx, scene, lambda v: v["notice.body"].startswith("P1 (-38, 24)"),
                           "left platform snaps to height 24", at_x)
        ctx.stick(0, -1, 190)
        observe(ctx, scene, lambda v: v["notice.body"].startswith("P1 (-38, 0)"),
                "lower cursor snaps to the main floor", platform)
    elif row == "J6":
        before = _select(ctx, scene, "7", 4)
        ctx.play(TAP["A"])
        _ready(ctx, scene, before, "7", (-38, 0), (0, 65), 72)
        _stop(ctx, scene, "7")
    else:
        raise ValueError(f"unsupported crosshair row {row}")


def refusal(ctx, row, scene="training"):
    if row == "R1":
        before = _move(ctx, scene, _setup(ctx, scene, "6"), (0, 42))
        refused = _refuse(ctx, scene, before, "Pick a point off the stage")
        ctx.shot("pick-a-point-off-the-stage")
        observe(ctx, scene, lambda v: v["notice.header"] == "Edge · 1/3",
                "Guard setup notice returns", refused, cap=4)
    elif row == "R2":
        before = _wizard(ctx, scene, "6", 1)
        x, y = _point(_notice(before))
        refused = _refuse(ctx, scene, before, "Pick a point off the stage")
        for _ in range(3):
            ctx.play(TAP["Right"])
        resumed = observe(ctx, scene, lambda v: v["notice.header"] == "Edge · 1/3",
                          "Guard setup notice returns after moving it", refused, cap=4)
        _expect_body(resumed, f"CPU ({x + 3}, {y})")
        _press(ctx, scene, "B", open="1")
    elif row in ("R3", "R4", "R5"):
        if row == "R3":
            _setup(ctx, scene, "7")
            _select(ctx, scene, "7", 2)
            _select(ctx, scene, "7", 3)
        before = _wizard(ctx, scene, "7", 3)
        if row == "R3":
            before = _move(ctx, scene, before, (0, _point(_notice(before))[1]))
            # tr.menu exposes the last valid floor, so only the result of A
            # can attest to the below-floor selection in this build.
            ctx.stick(0, -1, 240)
        elif row == "R4":
            ctx.stick(-1, 0, 1300)
        else:
            _press(ctx, scene, "B", **{"notice.header": "Juggle · 2/4"})
            before = _select(ctx, scene, "7", 3)
            ctx.stick(0, 1, 110)
        if row == "R5":
            _select(ctx, scene, "7", 4)
            for header in ("Juggle · 3/4", "Juggle · 2/4", "Juggle · 1/4"):
                _press(ctx, scene, "B", **{"notice.header": header})
            _press(ctx, scene, "B", open="1")
        else:
            refused = _refuse(ctx, scene, before, "Pick a point above a floor")
            observe(ctx, scene, lambda v: v["notice.header"] == "Juggle · 3/4",
                    "Juggler setup notice returns", refused, cap=4)
    elif row == "R6":
        _setup(ctx, scene, "6")
        _select(ctx, scene, "6", 2)
        _move(ctx, scene, _wizard(ctx, scene, "6", 2), (0, 0))
        ctx.stick(0, -1, 500)
        after = _select(ctx, scene, "6", 3)
        if "P1 (0, 0)" not in _notice(after):
            raise RuntimeError(f"Guard did not choose the main floor: {_notice(after)!r}")
        for header in ("Edge · 2/3", "Edge · 1/3"):
            _press(ctx, scene, "B", **{"notice.header": header})
        _press(ctx, scene, "B", open="1")
        _close(ctx, scene)
    elif row in ("R7", "R8", "R9"):
        if row == "R7":
            _goto(ctx, scene, "saves", ["save_state_enable"])
            before = _read(ctx, scene, **{"action": "save_state_enable"})
            if before["values"]["list.value"] != "[OFF]":
                _press(ctx, scene, "A", **{"list.value": "[OFF]"})
            _goto(ctx, scene, GAME_PAGE, ["game.6", "start.6"])
            before = _read(ctx, scene)
            ctx.play(TAP["A"])
            _ready(ctx, scene, before, "6", None, None, None)
        _read(ctx, scene, **{"open": "0", "game.running": "1", "notice.header": "Guard the Edge"})
        header = "Save State" if row == "R9" else "Input Recording"
        message = ("Save States is Off (Saves page)" if row == "R9"
                   else "Stop the drill to start recording or play inputs")
        macro = {"R7": "rm-record-chord", "R8": "ej-play-chord", "R9": "ej-save-chord"}[row]
        refused = press_for_state(ctx, macro, {"scene": scene, "game.running": "1",
                                              "notice.header": header, "notice.body": message}, cap=3)
        observe(ctx, scene, lambda v: v["game.running"] == "1" and v["notice.header"] == "Guard the Edge",
                "drill stays running and reclaims the notice", refused, cap=5)
        if row == "R9":
            _stop(ctx, scene, "6")
            _close(ctx, scene)
    else:
        raise ValueError(f"unsupported refusal row {row}")

