"""Source-only RC roster segment for the lead's EXISTING live Surface Session.

No CLI, configuration, lifecycle, claims, settings writes or result files. Caller
supplies current diagnostic lines, the existing f6_rows module's strict predicates,
and a review callback that actually examines each screenshot. Never import an old
rows_s6 module (it runs its CLI at import). Release steps require explicit observed
ready/body/HUD review because shipped diagnostics may be absent.

This segment is NOT runtime-qualified. Fixture/native scene navigation remains
lead-owned; do not replace visual review with a callback returning True blindly.
Queued logs plus no completion are not atomic native pending-work proof. Busy
and teardown keep short-input timestamps/order for final review; a build that
already settled is never promoted into an interrupted-build acceptance claim.
"""
import re


def _sequence(line):
    found = re.search(r"\s#(\d+)\s", line)
    return int(found[1]) if found else None


def mark(read):
    """Use the existing boot's sequence, not an index that log rotation can reuse."""
    sequences = [value for line in read() if (value := _sequence(line)) is not None]
    if not sequences:
        raise RuntimeError("No current diagnostic sequence; no input sent")
    return max(sequences)


def _new(read, since):
    by_sequence = {}
    for line in read():
        value = _sequence(line)
        if value is not None and value > since:
            if value in by_sequence and by_sequence[value] != line:
                raise RuntimeError("Conflicting diagnostic sequence; no inferred order")
            by_sequence[value] = line
    return [by_sequence[value] for value in sorted(by_sequence)]


def _wait(session, read, since, predicate, what, cap=30):
    deadline = session.clock() + cap
    while session.clock() < deadline:
        for line in _new(read, since):
            if predicate(line):
                if "UNEXPECTED" in line:
                    raise RuntimeError(line)
                return line
        session.sleep(0.1)
    raise RuntimeError("Missing positive evidence: " + what)


def _visual(session, review, name, expected):
    shot = session.snap(name)
    if shot is None or review(name, shot, expected) is not True:
        raise RuntimeError("Unreviewed/failed screenshot: " + name)
    return shot


def _request(session, read, strict, role, target, hold=200):
    if role not in (0, 1):
        raise ValueError("Only logical P1/CPU roles, never promoted owners")
    since = mark(read)
    session.press("zl,right" if role == 0 else "zl,left", hold)
    request = _wait(session, read, since, lambda line: strict.cycle_request(line, role),
                    "current role transaction submit", 10)
    if int(strict.cycle_request(request, role)[3], 16) != target:
        raise RuntimeError("Unexpected requested target: " + request)
    return since, request


def _complete(session, read, strict, role, since, request):
    result = _wait(session, read, since,
                   lambda line: strict.cycle_result(line, role, request),
                   "matching role/ticket/target and released=true")
    if strict.cycle_result(result, role, request) != "ready":
        raise RuntimeError("Transaction refused: " + result)
    return result


def cycle(session, read, strict, review, role, target, name):
    """Targets come from the candidate's reviewed fixture, not the F6 target list."""
    since, request = _request(session, read, strict, role, target)
    done = _complete(session, read, strict, role, since, request)
    shot = _visual(session, review, name,
                   f"logical role {role}, actual body/HUD {target:#x}, ready; return/original if planned")
    return {"request": request, "done": done, "shot": shot}


def _async_group(session, read, strict, role, since, request):
    build = _wait(session, read, since,
                  lambda line: re.search(rf"\[GroupSwap\] build owner={role} group=\d+ .*mode=async queued=", line),
                  "ordinary native construction queued")
    group = int(re.search(r"\bgroup=(\d+)", build)[1])
    target = int(strict.cycle_request(request, role)[3], 16)
    details = [line for line in _new(read, since) if _sequence(line) < _sequence(build)
               and re.search(rf"\[GroupSwap\] build owner={role} group={group} kinds=\[([^]]+)\]", line)]
    if not details:
        raise RuntimeError("No current group membership to correlate queued work")
    kinds = re.search(r"\bkinds=\[([^]]+)\]", details[-1])[1]
    if target not in [int(kind.strip(), 16) for kind in kinds.split(",")]:
        raise RuntimeError("Pending group does not contain this transaction's target")
    return group, build


def _terminal_observed(read, strict, role, build, request):
    suffix = _new(read, _sequence(build))
    terminal = re.compile(rf"\[GroupSwap\] (built|construction drained|teardown) owner={role}\b")
    return [line for line in suffix if terminal.search(line)
            or strict.cycle_result(line, role, request)]


def cold_group(session, read, strict, review, role, target, name):
    since, request = _request(session, read, strict, role, target)
    group, build = _async_group(session, read, strict, role, since, request)
    done = _complete(session, read, strict, role, since, request)
    built = _wait(session, read, _sequence(build),
                  lambda line: re.search(rf"\[GroupSwap\] built owner={role} group={group} ok=true\b", line)
                  and "mode=async" in line, "ordinary asynchronous group built")
    shot = _visual(session, review, name, f"actual role {role} body/HUD {target:#x}; ordinary group ready")
    return {"request": request, "build": build, "built": built, "done": done, "shot": shot}


def busy(session, read, strict, review, role, target, later_target, name):
    """Short-input attempt; final raw ordering, not a human atomic-pending claim.

    Not-exercised is NOT row PASS. Caller may use another already-planned cold
    group in this same process, without restarting or failing unrelated rows.
    """
    since, request = _request(session, read, strict, role, target, hold=40)
    group, build = _async_group(session, read, strict, role, since, request)
    observed = session.clock()
    terminal = _terminal_observed(read, strict, role, build, request)
    if terminal:
        done = _complete(session, read, strict, role, since, request)
        return {"exercised": False, "reason": "settled before second press; busy row remains open",
                "request": request, "build": build, "done": done, "terminal": terminal}
    dispatched = session.clock()
    session.press("zl,right" if role == 0 else "zl,left", 40)
    returned = session.clock()
    done = _complete(session, read, strict, role, since, request)
    _visual(session, review, name + "-settled", "first build intact; correct body/HUD; no corruption")
    later = cycle(session, read, strict, review, role, later_target, name + "-later")
    return {"exercised": None, "attempted": True,
            "acceptance": "review native build/input ordering; no atomic pending claim",
            "queue_observed_host": observed, "second_press_host": [dispatched, returned],
            "build": build, "group": group, "request": request, "done": done, "later": later}


def mii_gunner(session, read, strict, review, scene_since, fixture_review):
    """Saved identity is independently reviewed; class 0x4a alone never qualifies it."""
    if fixture_review() is not True:
        raise RuntimeError("Saved Gunner canonical/native record not qualified; no input sent")
    result = cycle(session, read, strict, review, 1, 0x4a, "rc-mii-gunner")
    done = result["done"]
    if "mii=true" not in done or "resource_pair=Some(true)" not in done:
        raise RuntimeError("Exact saved-Mii bank proof absent: " + done)
    lines = _new(read, scene_since)
    ready_seq = _sequence(done)
    builds = [line for line in lines if _sequence(line) < ready_seq and
              re.search(r"\[GroupSwap\] build owner=1 group=\d+ .*mode=sync reason=mii group(?:\s|$)", line)]
    if not builds:
        raise RuntimeError("No synchronous CPU Mii-group construction")
    build = builds[-1]
    group = re.search(r"\bgroup=(\d+)", build)[1]
    record = next((line for line in reversed(lines) if _sequence(line) < _sequence(build)
                   and re.search(rf"\[MiiOrdinaryCpu\] Mii group record published owner=1 group={group} .*kind=0x4a\b", line)), None)
    if record is None:
        raise RuntimeError("No exact Gunner record publication before construction")
    prefetch = next((line for line in lines if _sequence(record) < _sequence(line) < _sequence(build)
                     and re.search(rf"\[GroupSwap\] prefetched owner=1 group={group}\b", line)), None)
    built = next((line for line in lines if _sequence(build) < _sequence(line) < ready_seq
                  and re.search(rf"\[GroupSwap\] built owner=1 group={group} ok=true\b", line)), None)
    if prefetch is None or built is None:
        raise RuntimeError("Missing ordered record -> prefetch -> build -> built -> exact ready chain")
    return {**result, "record": record, "prefetch": prefetch, "build": build, "built": built}


def hats(session, read, review, expected_kinds):
    if len(expected_kinds) != 5 or len(set(expected_kinds)) != 5:
        raise ValueError("RC-HATS requires five distinct reviewed choices")
    results = []
    for index, kind in enumerate(expected_kinds, 1):
        since = mark(read)
        session.press("b,lstick", 200)
        requested = _wait(session, read, since,
                          lambda line: re.search(rf"\[KirbyHats\] request role=0 kind=0x{kind:x}\b", line),
                          "expected Kirby hat request")
        body = re.search(r"\bbody=(0x[0-9a-f]+)\b", requested)
        if body is None:
            raise RuntimeError("Hat request lacks body identity")
        confirmed = _wait(session, read, _sequence(requested),
                          lambda line: re.search(rf"\[KirbyHats\] confirmed role=0 kind=0x{kind:x} body={body[1]}\b", line),
                          "same-body confirmed hat")
        shot = _visual(session, review, f"rc-hat-{index}", f"actual Kirby hat {kind:#x}, ready; notice not falsely Paused")
        results.append({"request": requested, "confirmed": confirmed, "shot": shot})
    return results


def kirby_to_rob(session, read, strict, review, next_target, rob_target=0x2d):
    since = mark(read)
    departure = cycle(session, read, strict, review, 0, next_target, "rc-kirby-away")
    fallback = _wait(session, read, since,
                     lambda line: "[KirbyPark] fallback owner=0 " in line and "canonical Reload" in line,
                     "ordinary canonical departure from copied Kirby")
    # Caller cycles through other reviewed fixture members if R.O.B. is not next.
    rob = departure if next_target == rob_target else None
    return {"departure": departure, "fallback": fallback, "rob_if_next": rob}


def rob_visual(session, review):
    return _visual(session, review, "rc-rob-normal",
                   "actual R.O.B. body/HUD, normal white/grey and red eye; ordinary group ready")


def interrupted_scene(session, read, strict, review, role, target, quit_to_css, reenter, after_target, index):
    """Call three times in the SAME process; native navigation is lead-owned."""
    since, request = _request(session, read, strict, role, target, hold=40)
    group, build = _async_group(session, read, strict, role, since, request)
    observed = session.clock()
    terminal = _terminal_observed(read, strict, role, build, request)
    dispatched = session.clock()
    quit_to_css(session)  # Must itself use observed native menus and screenshot gates.
    returned = session.clock()
    _visual(session, review, f"rc-end-{index}-css", "actual CSS after native Training quit")
    teardown = _wait(session, read, _sequence(build),
                     lambda line: re.search(rf"\[GroupSwap\] teardown owner={role}\b", line)
                     and "reason=finalize" in line, "finalize teardown")
    if "skipped=0" not in teardown or "alias_ok=true" not in teardown:
        raise RuntimeError("Unclean teardown: " + teardown)
    destroyed = _wait(session, read, _sequence(build),
                      lambda line: "[TrioBoundary] destroy " in line and "owner=1 " in line,
                      "CPU trio destruction")
    # A poll can race settlement. Preserve actual drain vs settled-before-finalize,
    # rather than upgrading the input-time observation to a drained-task claim.
    drained = [line for line in _new(read, _sequence(build)) if
               re.search(rf"\[GroupSwap\] construction drained owner={role} group={group}\b", line)]
    reenter(session)
    _visual(session, review, f"rc-end-{index}-reentry", "actual re-entered Training with current fixture")
    continuation = cycle(session, read, strict, review, role, after_target, f"rc-end-{index}-build-again")
    return {"build": build, "teardown": teardown, "destroyed": destroyed,
            "queue_observed_host": observed, "quit_route_host": [dispatched, returned],
            "settled_before_quit_dispatch": bool(terminal),
            "interrupted_acceptance": "unexercised" if terminal else "review actual build/quit ordering",
            "drained": drained, "continuation": continuation}


def notice(session, read, review, name, expected, scene_since):
    """Call while P1/CPU/both are loading and again at Ready; inspect actual text."""
    lines = _new(read, scene_since)
    bad = [line for line in lines if "[PreloadStatus]" in line
           and "state=Paused" in line and re.search(r"\bpending=0\b", line)]
    if bad:
        raise RuntimeError("False Paused retained in current log: " + bad[-1])
    return _visual(session, review, name, expected)


def release_laps(session, review, wait_ready, p1_targets, cpu_targets):
    """Reuse Surface RELEASE replay/online process; no diag absence is a PASS."""
    if len(p1_targets) != 12 or len(cpu_targets) != 3:
        raise ValueError("Release fixture must give exact P1x12 and CPUx3 target order")
    shots = []
    for role, targets in ((0, p1_targets), (1, cpu_targets)):
        for index, target in enumerate(targets, 1):
            session.press("zl,right" if role == 0 else "zl,left", 200)
            if wait_ready(session, role, target) is not True:
                raise RuntimeError("Release ready/body/HUD gate missing; no next input")
            # Review must gate observed readiness, not merely acknowledge the shot.
            shots.append(_visual(session, review, f"rc-release-r{role}-{index}",
                                 f"wait for actual ready, body/HUD {target:#x}, no residual loading; original on planned return"))
    return shots
