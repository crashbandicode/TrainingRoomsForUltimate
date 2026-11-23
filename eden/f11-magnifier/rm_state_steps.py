"""Recording checks for the combined run, using the live bridge and its file receipts."""

import hashlib
import json
from pathlib import Path
import re
import time

import routes
import state_gate
import switch_hw


MENU = "tr.menu"
SCHEMA = "tr.menu/1"
ATTACK = 0x27  # FIGHTER_STATUS_KIND_ATTACK from the 13.0.5 skyline-smash constants.
MAPPED_ATTACK = 1


def menu(ctx, where=None, cap=8, baseline=None, require_ack=False, ask_every=.2):
    where = where or {}

    def accept(block):
        if block["schema"] != SCHEMA or block["filtered"]:
            return False, "expected a complete tr.menu/1 block"
        return state_gate.satisfies(block, where, baseline=baseline,
                                    ack=1 if require_ack else None,
                                    input_gate=require_ack)

    block, why = ctx.runner._state_block(ctx.on, MENU, cap, accept, ask_every=ask_every)
    ctx.log({"recording_bridge": block, "checked": where, "result": why})
    return block


def press_for_state(ctx, macro, where, cap=8, require_ack=False, before=None):
    baseline = menu(ctx, before, cap)
    if all(baseline["values"].get(key) == str(value) for key, value in where.items()):
        raise RuntimeError(f"{macro}: destination already matches before input: {where!r}")
    ctx.play(macro)
    return menu(ctx, where, cap, baseline, require_ack)


def _file(ctx, name, cap, missing=False):
    log = routes.instance_path(ctx, "bridge_log")
    cursor = routes.LogCursor(log, hold=False)
    routes._spool_file(ctx, "bridge-file.req", f"{name}\n".encode())
    end = re.compile(rf"\[Bridge\] file_end name={re.escape(name)} bytes=(\d+) "
                     r"sha256=([0-9a-f]{64}) result=ok.*\bmirrored=true(?: |$)")
    error = f"[Bridge] req file result=error reason=read name={name} "

    def accepted(line):
        match = end.search(line)
        if match:
            return match
        if line.startswith(error):
            if missing:
                return "missing"
            raise RuntimeError(line)
        return None

    match = routes._wait_bridge_lines(ctx, cursor, time.monotonic() + cap, accepted, name)
    if match == "missing":
        return {}
    raw = (log.parent / "bridge" / "files" / name).read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if len(raw) != int(match[1]) or digest != match[2]:
        raise RuntimeError(f"{name} did not match the bridge's byte count and SHA-256")
    ctx.log({"recording_file": name, "bytes": len(raw), "sha256": digest})
    return json.loads(raw)


def role_file(ctx, popup, page, cap=10):
    expected = {"popup": popup, "page": page}
    until = time.monotonic() + cap
    last = {}
    while time.monotonic() < until:
        ctx.check_stop()
        raw = routes._fetch_settings(ctx, cap=until - time.monotonic())
        last = json.loads(raw).get("recording_roles", {})
        if last == expected:
            ctx.log({"recording_roles_on_disk": last, "sha256": hashlib.sha256(raw).hexdigest()})
            return last
    raise RuntimeError(f"settings role write missed deadline: saw {last!r}; expected {expected!r}")


def _active(ctx, cap, *, missing=False):
    return _file(ctx, "recording-index.json", cap, missing).get("active", {})


def _new_cpu_take(ctx, old, cpu_kind, cap=10):
    until = time.monotonic() + cap
    active = {}
    while time.monotonic() < until:
        ctx.check_stop()
        active = _active(ctx, until - time.monotonic(), missing=True)
        take = active.get("cpu")
        if take and take != old.get("cpu"):
            break
    else:
        raise RuntimeError(f"no new active CPU macro reached disk: {active!r}")
    if active.get("player") != old.get("player"):
        raise RuntimeError("the Player active macro changed during a CPU recording")
    try:
        record = _file(ctx, f"macros/{take}.json", max(.1, until - time.monotonic()))
    except RuntimeError as error:
        raise RuntimeError(f"active CPU macro {take} could not be verified: {error}") from error
    frames = record.get("frames", [])
    if record.get("start", {}).get("fighter_kind") != cpu_kind or not any(frame["buttons"] & MAPPED_ATTACK for frame in frames):
        raise RuntimeError(f"CPU macro {take} lacks the starting fighter or attack input")
    proof = {"take": take, "name": record["name"], "fighter_kind": cpu_kind,
             "frames": len(frames), "start_save": record.get("start_save")}
    ctx.log({"recorded_cpu_macro": proof})
    (Path(ctx.row_dir) / "recorded-cpu-macro.json").write_text(json.dumps(proof) + "\n")
    return proof


def _jab_witness(ctx, before, run, cpu_entry, cap=2):
    until = time.monotonic() + cap
    current = {}
    if any(slot not in before or before[slot].get("frame") is None or before[slot]["status"] == ATTACK
           for slot in (0, cpu_entry)):
        raise RuntimeError(f"both bodies must be present and out of jab before A: {before!r}")
    ctx.call("input.button", {"button": "A", "state": "tap", "duration_ms": 120, "lease_ms": 620})
    while time.monotonic() < until:
        ctx.check_stop()
        current = switch_hw.bridge_state(run, ("fighters",), cap=min(1, until - time.monotonic()))["fighters"]
        player, cpu = current.get(0), current.get(cpu_entry)
        if player is None or cpu is None:
            continue
        frame = player.get("frame")
        if frame is None or frame != cpu.get("frame") or frame <= before[cpu_entry]["frame"]:
            continue
        if player["status"] == ATTACK:
            raise RuntimeError(f"Player 1 jabbed during CPU capture: {current!r}")
        if cpu["status"] == ATTACK:
            ctx.log({"recording_actor_before": before, "recording_actor_after": current})
            return current
    raise RuntimeError(f"no fresh sample of CPU entry {cpu_entry} jabbing without a P1 jab: {current!r}")


def cold_cpu_take(ctx, scene="training", cpu_entry=1, cold=True, page="player"):
    closed = {"scene": scene, "open": "0", "recording.popup.open": "0"}
    if cold:
        closed["ack"] = "0"
    menu(ctx, {**closed, "recording.capture.role": ""})
    role_file(ctx, popup="cpu", page=page)
    old = _active(ctx, 5, missing=True)
    run = routes.instance_path(ctx, "bridge_log").parent
    fighters = switch_hw.bridge_state(run, ("fighters",), cap=3)["fighters"]
    if 0 not in fighters or cpu_entry not in fighters:
        raise RuntimeError(f"Recording needs live bodies at entries 0 and {cpu_entry}: {fighters!r}")

    active = {**closed, "recording.capture.role": "cpu", "notice.header": "Input Recording"}
    starting = menu(ctx, {**closed, "recording.capture.role": "", "notice.header": ""}, cap=25)
    lockout_failure = None
    try:
        # The 30-frame Lockout can end while a blocking play call is still
        # returning. Poll the already attested bridge as the same-frame chord plays.
        segment = ctx.runner.plan["segments"]["rm-record-chord"]
        macro = ctx.runner.base / segment["file"]
        if hashlib.sha256(macro.read_bytes()).hexdigest() != segment["sha256"]:
            raise RuntimeError("record chord macro does not match the manifest")
        ctx.call("macro.play", {"path": str(macro)})
        try:
            lockout = menu(ctx, {**active, "notice.body": "Lockout"}, 1, starting, ask_every=.05)
            ctx.shot("cpu-blue-lockout")
        except RuntimeError as error:
            lockout_failure = str(error)
            ctx.log({"cold_lockout_missing": lockout_failure})
            lockout = starting
        ctx.runner.wait_macro(ctx.on, cap=2)
        standby = menu(ctx, {**active, "notice.body": "Standby"}, 7, lockout)
        ctx.shot("cpu-magenta-standby")
        standby_bodies = switch_hw.bridge_state(run, ("fighters",), cap=3)["fighters"]
        _jab_witness(ctx, standby_bodies, run, cpu_entry)
        menu(ctx, {**active, "notice.body": "Recording"}, 3, standby)
        ctx.shot("cpu-red-recording")
    finally:
        visible = menu(ctx, {"scene": scene, "open": "0"})
        if visible["values"].get("recording.capture.role"):
            press_for_state(ctx, "rm-record-chord",
                            {**closed, "recording.capture.role": ""}, cap=6)
    take = _new_cpu_take(ctx, old, fighters[cpu_entry]["kind"])
    if lockout_failure:
        raise RuntimeError(f"CPU take {take['take']} saved correctly; Lockout unobserved: {lockout_failure}")
    return take


def recover_training(ctx):
    block = menu(ctx, {"scene": "training"})
    values = block["values"]
    if values.get("recording.capture.role"):
        press_for_state(ctx, "rm-record-chord", {"scene": "training", "recording.capture.role": ""})
    if values.get("recording.popup.open") == "1":
        press_for_state(ctx, "rm-tap-b", {"scene": "training", "recording.popup.open": "0"}, require_ack=True)
    if values.get("zone.step"):
        press_for_state(ctx, "rm-tap-b", {"scene": "training", "zone.step": "", "open": "1"})
    return routes.tr_idle(ctx)

