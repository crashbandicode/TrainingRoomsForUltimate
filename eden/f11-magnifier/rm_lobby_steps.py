"""Navigate the TR menu after the bridge proves native waiting-room practice."""

import sys
import time
import unicodedata

import routes
import state_gate
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from rm_state_steps import menu, press_for_state


SCENE = {"scene": "lobby", "keyboard": "0"}
OPEN = {**SCENE, "open": "1", "recording.popup.open": "0"}


def _input(ctx, before, macro, until, predicate, expect):
    if time.monotonic() >= until:
        raise RuntimeError("waiting-room navigation reached its deadline before input")
    ctx.play(macro)

    def accept(after):
        ok, why = state_gate.satisfies(after, SCENE, baseline=before, ack=1, input_gate=True)
        if not ok:
            return False, why
        hit = predicate(after["values"])
        return hit, expect if not hit else "one handled input"

    cap = until - time.monotonic()
    if cap <= 0:
        raise RuntimeError("waiting-room navigation reached its deadline")
    result, _ = ctx.runner._state_block(ctx.on, "tr.menu", cap, accept, ask_every=.2)
    ctx.log({"lobby_menu_input": macro, "before": before, "after": result})
    return result


def _top(ctx, current, until):
    while (depth := int(current["values"]["depth"])) > 0:
        current = _input(ctx, current, "rm-tap-b", until,
                         lambda value: value["open"] == "1" and int(value["depth"]) < depth,
                         "B must back out a level with the TR menu open")
    return current


def lobby_open(ctx, page=None, cap=25):
    until = time.monotonic() + cap
    current = menu(ctx, {**SCENE, "recording.popup.open": "0"}, cap)
    if current["values"]["open"] == "0":
        current = press_for_state(ctx, "rm-menu-open", OPEN, cap=until - time.monotonic())
    if page is None:
        return current
    if page not in routes.TR_PAGES:
        raise ValueError(f"unknown TR page {page!r}")
    current = _top(ctx, current, until)
    seen = set()
    while (old := current["values"]["page"]) != page:
        if old in seen or old not in routes.TR_PAGES:
            raise RuntimeError(f"waiting-room page {page!r} did not appear in {seen!r}")
        seen.add(old)
        current = _input(ctx, current, "rm-tap-r", until,
                         lambda value: value["open"] == "1" and value["depth"] == "0" and value["page"] != old,
                         "R must move to the next page at the same menu depth")
    return current


def lobby_close(ctx, cap=20):
    until = time.monotonic() + cap
    current = menu(ctx, {**SCENE, "recording.popup.open": "0"}, cap)
    if current["values"]["open"] == "0":
        return current
    current = _top(ctx, current, until)
    return _input(ctx, current, "rm-tap-b", until, lambda value: value["open"] == "0",
                  "B at page depth 0 must close the TR menu")


def _normalize(text):
    return " ".join(unicodedata.normalize("NFKC", text).casefold().split())


def _matches(values, target):
    for key, expected in target.items():
        observed = values.get(key, "")
        if key == "row":
            observed, expected = _normalize(observed), _normalize(expected)
        if observed != expected:
            return False
    return True


def lobby_goto(ctx, page, path, cap=45):
    targets = routes._tr_path(path)
    until = time.monotonic() + cap
    current = lobby_open(ctx, page, cap)
    for index, target in enumerate(targets):
        seen = set()
        while not _matches(current["values"], target):
            values = current["values"]
            depth, group, focus = values["depth"], values["group"], values["focus"]
            if not focus or focus in seen:
                raise RuntimeError(f"waiting-room row {target!r} not found in one pass")
            seen.add(focus)

            def moved(value):
                return (value["open"] == "1" and value["page"] == page and value["depth"] == depth
                        and value["group"] == group and value["focus"] != focus)

            current = _input(ctx, current, "rm-tap-down", until, moved,
                             "Down must advance the focused row without changing screen")
        if index == len(targets) - 1:
            return current
        depth = int(current["values"]["depth"])
        current = _input(ctx, current, "rm-tap-a", until,
                         lambda value: value["open"] == "1" and value["page"] == page
                         and int(value["depth"]) == depth + 1 and bool(value["focus"]),
                         "A must enter the selected submenu")


def recover_lobby(ctx, cap=30):
    until = time.monotonic() + cap
    for _ in range(8):
        state = menu(ctx, SCENE, cap=until - time.monotonic())
        values = state["values"]
        if values.get("recording.capture.role"):
            press_for_state(ctx, "rm-record-chord", {**SCENE, "recording.capture.role": ""},
                            cap=until - time.monotonic())
        elif values.get("recording.popup.open") == "1":
            press_for_state(ctx, "rm-tap-b", {**SCENE, "recording.popup.open": "0"},
                            cap=until - time.monotonic(), require_ack=True)
        elif values.get("zone.step") or values.get("notice.header", "").startswith("Edge ·"):
            old = (values.get("zone.step"), values.get("notice.header"), values["open"])
            ctx.play("rm-tap-b")

            def changed(block):
                accepted, why = state_gate.satisfies(block, SCENE, baseline=state)
                live = block["values"]
                fresh = (live.get("zone.step"), live.get("notice.header"), live["open"])
                return accepted and fresh != old, why if not accepted else f"wizard state {fresh}"

            after, why = ctx.runner._state_block(ctx.on, "tr.menu", until - time.monotonic(), changed, ask_every=.2)
            ctx.log({"lobby_wizard_cancel": after, "result": why})
        elif values.get("game.running") == "1":
            path = (["game.1", "start.1"] if values.get("game") == "Eggs-ercise" else None)
            if not path:
                raise RuntimeError(f"waiting-room recovery needs an exact Stop route for {values.get('game')!r}")
            lobby_goto(ctx, "fighters_stages", path, cap=until - time.monotonic())
            press_for_state(ctx, "rm-tap-a", {**OPEN, "game.running": "0"},
                            cap=until - time.monotonic(), require_ack=True)
        else:
            return lobby_close(ctx, cap=until - time.monotonic())
    raise RuntimeError("waiting-room Recording recovery did not reach the idle state")

