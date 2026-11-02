"""Package 4, S4-4 Eden rows (named macros, replay capture, replay window), run from the Surface over ssh after `route4.py`
reached the Training scene. The Named library, Save slot to library and Replay window screens are package 5's, so the rows drive the
layer through the diag request file (`runtime/macros/diag.rs`) and read the result lines. Every wait is gated on a diag line or a
file, with a cap.  Usage: rows_s44.py <label>
"""
import base64
import json
import re
import subprocess
import sys
import time
from pathlib import Path

import route4 as r
import hook_stats

LABEL = sys.argv[1]
SD = f"{r.PROFILE_SD}/TrainingRooms"
DIAG = f"{SD}/practice-diagnostics.log"
DIAG_PREV = f"{SD}/practice-diagnostics.previous.log"
OUT = Path(__file__).resolve().parent / "runs" / LABEL
OUT.mkdir(parents=True, exist_ok=True)
RESULTS = []


def read_diag():
    rc, out, _ = r.remote(f"foreach ($p in '{DIAG_PREV}','{DIAG}') {{ if (Test-Path $p) {{ Get-Content $p }} }}", 60)
    return out.splitlines()


def stamp_of(line):
    m = re.match(r"(\d+) ", line)
    return int(m.group(1)) if m else -1


def mark():
    return max((stamp_of(x) for x in read_diag()), default=0)


def lines_after(start):
    return [x for x in read_diag() if stamp_of(x) > start]


def line_after(start, needle, cap=20):
    deadline = time.time() + cap
    while time.time() < deadline:
        for line in lines_after(start):
            if needle in line:
                return line
        time.sleep(1.2)
    return None


def row(name, ok, evidence):
    RESULTS.append((name, ok, evidence))
    print(f"ROW {'pass' if ok else 'FAIL'} {name}: {evidence}", flush=True)
    (OUT / "results.json").write_text(json.dumps(RESULTS, indent=1))
    if not ok:
        raise SystemExit(f"precondition/row failed: {name}; stop and retry with a new label")


def tap(buttons, hold=150, wait=0.4):
    r.press(LABEL, buttons, hold)
    time.sleep(wait)


def shot(name):
    try:
        return r.snap(LABEL, name)
    except Exception as error:  # noqa: BLE001 - a screenshot is evidence, never a gate
        print(f"shot {name} failed: {error}", flush=True)


def cmd(*lines):
    """Write the request file (the diag thread takes it within 200 ms) and return the diag mark taken before."""
    m = mark()
    payload = base64.b64encode(("\n".join(lines) + "\n").encode()).decode()
    rc, out, err = r.remote(
        f"[IO.File]::WriteAllBytes('{SD}/diag-macro.req',[Convert]::FromBase64String('{payload}'))", 30)
    if rc:
        raise RuntimeError(f"diag request failed: {out} {err}")
    return m


def read_file(path):
    rc, out, _ = r.remote(f"if (Test-Path '{path}') {{ Get-Content -Raw '{path}' }}", 60)
    if rc:
        raise RuntimeError(f"remote file read failed: {path}: {out}")
    return out


def wait_file(path, predicate=lambda value: bool(value.strip()), cap=15):
    """Wait for persisted state, retaining the last value for failure evidence."""
    deadline = time.monotonic() + cap
    value = ""
    while time.monotonic() < deadline:
        value = read_file(path)
        if predicate(value):
            return value
        time.sleep(0.3)
    return value


def receipt(start, text, cap=15):
    line = line_after(start, "[NamedMacroDiag] " + text, cap)
    row("diagnostic command completed: " + text, line is not None, line or "no receipt")
    return line


def view(m=None, cap=8):
    m = cmd("view") if m is None else m
    line = line_after(m, "[NamedMacroDiag] view", cap)
    return line


def parse_view(line):
    selected = re.search(r"selected=(\d+)", line or "")
    labels = re.search(r"labels=(\[.*\])$", line or "")
    return (int(selected.group(1)) if selected else -1), (labels.group(1) if labels else "")


def label_list(raw):
    try:
        return json.loads(raw)
    except Exception:  # noqa: BLE001
        return []


def move_to(text, tries=64):
    """Navigate the model's 4-column grid; inspect each receipt and selected cell."""
    line = view()
    for _ in range(tries):
        sel, raw = parse_view(line)
        items = label_list(raw)
        if not 0 <= sel < len(items):
            return None
        if text in items[sel]:
            return sel
        target = next((i for i, label in enumerate(items) if text in label), None)
        if target is None:
            direction = "down"  # library's bottom row advances to the next page
        elif sel // 4 != target // 4:
            direction = "down" if sel // 4 < target // 4 else "up"
        else:
            direction = "right" if sel < target else "left"
        receipt(cmd("pad " + direction), "pad " + direction)
        line = view()
    return None


def ui_do(method, params, cap=6.0):
    """A UI operation of Eden's control API, waited for: it returns a job before it runs (CLIControl.md)."""
    res = r.call(LABEL, method, params)
    job = res.get("job") if isinstance(res, dict) else None
    deadline = time.time() + cap
    while job and time.time() < deadline:
        state = r.call(LABEL, "jobs.get", {"job": job})
        if isinstance(state, dict) and state.get("state") not in ("queued", "running", None):
            return state
        time.sleep(0.3)
    return res


START = 0


def main():
    global START
    START = mark()
    events = read_file(f"{SD}/events.log")
    (OUT / "events.log").write_text(events)
    last = events.strip().splitlines()[-1] if events.strip() else ""
    row("install line: named_macros installed", "named_macros=installed" in events, last[:400])
    row("install line complete (result=installed) and heap_allocations=0", "result=installed" in events and "heap_allocations=0" in events, "install")
    sites = ("control_get_button", "control_get_stick_x", "control_get_stick_y",
             "control_get_stick_x_no_clamp", "control_get_stick_y_no_clamp",
             "control_get_sub_stick_x", "control_get_sub_stick_y", "kinetic_get_energy",
             "kinetic_get_sum_speed_x", "kinetic_get_sum_speed_y",
             "stop_is_special_stop", "stop_set_special_stop")
    missing = [site for site in sites
               if not re.search(r"\b" + site + r"=(?:stock|other|shared)\b", events)]
    row("all 12 named-macro sites report usable guards", not missing,
        "missing/unusable=" + repr(missing))
    for name in ("evtest2", "evtest", "evcrop", "kbtest"):
        prior = read_file(f"{SD}/macros/{name}.json")
        row("fresh output path: " + name, not prior.strip(),
            "absent" if not prior.strip() else "prior file exists; prepare a fresh seed")
    cap = line_after(0, "[ReplayCapture] guarded live adapter ready", 5)
    row("live replay adapter: guards passed and resident hooks installed", cap is not None, cap or "no ready line; " + (line_after(0, "live adapter disabled", 3) or "no disabled line either"))

    # named capture with the stop chord
    m = mark()
    tap("zr,down", wait=0.1)
    ln = line_after(m, "named_record_lockout", 8)
    row("named record chord: lockout", ln is not None, ln or "none")
    time.sleep(1.6)
    tap("a", hold=200, wait=0.3)
    tap("b", hold=200, wait=0.3)
    m2 = mark()
    tap("zr,down", wait=0.1)
    ln = line_after(m2, "named_capture_stop", 8)
    stop_frames = None
    if ln:
        mm = re.search(r"frames=(\d+)", ln)
        stop_frames = int(mm.group(1)) if mm else None
    row("named stop chord ends the capture (consumed before the frame is stored)", ln is not None, ln or "none")
    m3 = cmd("name evtest2")
    receipt(m3, 'name "evtest2"')
    body2 = wait_file(f"{SD}/macros/evtest2.json")
    ok2 = False
    detail2 = "no evtest2.json"
    if body2.strip():
        try:
            d2 = json.loads(body2)
            # Crop (the recorder row, factory on) trims the idle lead-in before the first input: the saved length is at most the stop count.
            crop_line = line_after(m3 - 60_000, "named_recording_complete", 2) or ""
            cropped = "crop=true" in crop_line
            ok2 = (0 < len(d2["frames"]) <= stop_frames) if cropped and stop_frames else len(d2["frames"]) == stop_frames
            last = d2["frames"][-1]
            detail2 = f"frames={len(d2['frames'])} (stop reported {stop_frames}) last={last}"
        except Exception as error:  # noqa: BLE001
            detail2 = f"unparsable: {error}"
    (OUT / "evtest2.json").write_text(body2)
    row("named capture saved to the library with the captured frames", ok2, detail2)

    # list and load from the library
    m = cmd("open_library 0")
    receipt(m, "open_library slot=0")
    line = view()
    sel, labels = parse_view(line)
    (OUT / "library-view.txt").write_text(line or "")
    row("Named library lists the saved macro and the seeded ones", bool(line) and "evtest2" in labels, (line or "no view")[:300])
    target = move_to("evtest2")
    m = cmd("pad confirm")
    receipt(m, "pad confirm")
    selected = wait_file(f"{SD}/macros/selected.txt", lambda value: value.strip() == "evtest2").strip()
    row("Load a named macro from the library (selected.txt follows)", target is not None and selected == "evtest2", f"selected.txt={selected!r}, cursor row {target}")
    shot("library-loaded")
    cmd("status")
    time.sleep(1)

    # switch to Macro Source = Named Library, save a state, play the named macro
    st = cmd("status")
    ln = line_after(st, "[NamedMacroDiag] status", 6)
    row("Macro Source is Named Library (boot settings)", ln is not None and "named_source=true" in ln, ln or "none")
    r.remote(f"Set-Content -Path {SD}/diag-save.req -Value 'x'", 30)
    time.sleep(2.0)
    m = mark()
    tap("zr,up", wait=0.1)
    ln = line_after(m, "named_playback frames", 20)
    row("named playback chord: restore, then the named macro plays", ln is not None, ln or "no named_playback line; " + " | ".join(x[:140] for x in lines_after(m)[-6:]))
    # input 0 reached: the restore pose was remembered (sync-01 refused every named playback with "Restore the Training setup again")
    ln0 = line_after(m, "named_playback_start", 10)
    refused = [x for x in lines_after(m) if "Restore the Training setup again" in x or "idle/fall saved setup" in x]
    row("named playback reaches input 0 at the restored pose (no restore refusal)", ln0 is not None and not refused,
        (ln0 or "no named_playback_start") + (" | refused: " + refused[0][-120:] if refused else ""))
    ln2 = line_after(m, "named_playback_complete", 20)
    row("named playback completes", ln2 is not None, ln2 or "none")
    shot("after-named-playback")

    # copy the loaded named macro into Record Slot 1 (Y in the library), then save that slot back to the library (export)
    receipt(cmd("open_library 0"), "open_library slot=0")
    at = move_to("evtest2")
    m = cmd("pad extra")
    ln = line_after(m, "Copied", 8)
    row("Copy a named macro into Record Slot 1 (import_named_slot)", ln is not None, ln or "no Copied notice")
    receipt(cmd("pad back"), "pad back")
    m = cmd("save_slot 0 evtest")
    receipt(m, "save_slot slot=0")
    body = wait_file(f"{SD}/macros/evtest.json")
    src = read_file(f"{SD}/macros/evtest2.json")
    ok = False
    detail = "no evtest.json"
    if body.strip() and src.strip():
        try:
            a = json.loads(body); b = json.loads(src)
            ok = a["frames"] == b["frames"] and a["name"] == "evtest" and a.get("schema_version") in (1, 2)
            detail = f"evtest frames={len(a['frames'])} equal to evtest2's: {a['frames'] == b['frames']}; kind={a['start']['fighter_kind']}"
        except Exception as error:  # noqa: BLE001
            detail = f"unparsable: {error}"
    (OUT / "evtest.json").write_text(body)
    row("Save slot to library: the slot's frames written as macros/evtest.json", ok, detail)
    ln = line_after(m, "Saved evtest", 4)
    row("saved notice posted", ln is not None or ok, ln or "no notice line (the file is the evidence)")

    # The placement panel uses the real loaded macro and the restore anchor from playback.
    baseline = json.loads(read_file(f"{SD}/macros/evtest2.json"))
    receipt(cmd("open_library 0"), "open_library slot=0")
    at = move_to("evtest2")
    row("placement source found", at is not None, f"cursor={at}")
    receipt(cmd("pad confirm"), "pad confirm")
    receipt(cmd("open_placement"), "open_placement")
    placement_view = view()
    row("placement editor opens with restored anchor",
        'title="Placement"' in (placement_view or ""), placement_view or "no view")
    selected, _ = parse_view(placement_view)
    row("placement editor selects X +1", selected == 1, placement_view)
    receipt(cmd("pad confirm"), "pad confirm")
    changed_view = view()
    (OUT / "placement-view.txt").write_text(changed_view or "")
    at = move_to("Write")
    row("placement overwrite action found", at is not None, f"cursor={at}")
    receipt(cmd("pad confirm"), "pad confirm")
    original_x = baseline.get("placement", {}).get("delta_x", 0)

    def placement_written(value):
        try:
            return json.loads(value).get("placement", {}).get("delta_x") == original_x + 1
        except (ValueError, TypeError):
            return False

    placed = wait_file(f"{SD}/macros/evtest2.json", placement_written)
    (OUT / "placement-saved.json").write_text(placed)
    result = json.loads(placed) if placed.strip() else {}
    row("placement X +1 persists without changing recorded inputs",
        placement_written(placed) and result.get("frames") == baseline["frames"],
        f"before_x={original_x}; placement={result.get('placement')}")
    shot("after-placement")

    # replay window over the seeded capture
    m = cmd("list_captures")
    ln = line_after(m, "[NamedMacroDiag] captures", 8)
    lines = [x for x in lines_after(m) if "[NamedMacroDiag] capture" in x]
    (OUT / "captures.txt").write_text("\n".join(lines))
    row("Replay Window source: the seeded capture is listed by the archive", any("capture id=" in x for x in lines), "; ".join(x[-160:] for x in lines[:3]) or "none")
    m = cmd("replay_window")
    receipt(m, "replay_window opened")
    line = view()
    (OUT / "browser-view.txt").write_text(line or "")
    row("Replay Window browser opens and lists the capture", bool(line) and "none" not in (line or "none")[:60], (line or "no view")[:300])
    shot("replay-browser")
    m = cmd("pad confirm")
    time.sleep(3.0)
    line = view()
    (OUT / "mismatch-view.txt").write_text(line or "")
    if line and "Fighter Mismatch" in line:
        cmd("pad confirm")  # Keep current fighter (the roster cycle is behind the port)
    time.sleep(6.0)
    st = cmd("status")
    ln = line_after(st, "[NamedMacroDiag] status", 6)
    (OUT / "status-after-open.txt").write_text(ln or "")
    row("capture opens in the clip editor", ln is not None and "editor=true" in ln, ln or "none")
    m = cmd("clip confirm")
    receipt(m, "clip confirm")
    cr = [x for x in lines_after(m)][-12:]
    (OUT / "crop-lines.txt").write_text("\n".join(cr))
    m = cmd("name evcrop")
    receipt(m, 'name "evcrop"')
    body3 = wait_file(f"{SD}/macros/evcrop.json")
    ok3 = False
    detail3 = "no evcrop.json"
    if body3.strip():
        try:
            d3 = json.loads(body3)
            ok3 = len(d3["frames"]) > 0
            detail3 = f"frames={len(d3['frames'])} kind={d3['start']['fighter_kind']} schema={d3.get('schema_version')}"
        except Exception as error:  # noqa: BLE001
            detail3 = f"unparsable: {error}"
    (OUT / "evcrop.json").write_text(body3)
    row("Replay Window crops a macro into the library", ok3, detail3)
    shot("after-crop")

    # the release path: the real software keyboard applet, answered through Eden's control API (ui.tree / ui.set / ui.click)
    r.remote(f"Set-Content -Path {SD}/diag-keyboard.on -Value 'on'", 30)
    time.sleep(1)
    cmd("export_slot 0")
    dialog = None
    for _ in range(20):
        time.sleep(1.5)
        try:
            tree = r.call(LABEL, "ui.tree", {})
        except Exception as error:  # noqa: BLE001
            print(f"ui.tree: {error}", flush=True)
            continue
        found = {}

        def walk(node):
            if isinstance(node, dict):
                name = node.get("name") or node.get("object_name") or ""
                if node.get("id") and name in ("line_edit_osk", "button_ok"):
                    found[name] = node["id"]
                for key, value in node.items():
                    if key != "properties":
                        walk(value)
            elif isinstance(node, list):
                for value in node:
                    walk(value)

        walk(tree)
        if "line_edit_osk" in found and "button_ok" in found:
            dialog = found
            break
    ok = False
    detail = "keyboard dialog not found in ui.tree"
    if dialog:
        first = line_after(cmd("frame_probe"), "[NamedMacroDiag] frame_probe", 8)
        first_count = re.search(r"frames=(\d+)", first or "")
        row("keyboard frame probe available", first_count is not None, first or "missing")
        baseline = int(first_count.group(1))
        advanced = None
        deadline = time.time() + 8
        while time.time() < deadline:
            current = line_after(cmd("frame_probe"), "[NamedMacroDiag] frame_probe", 3)
            count = re.search(r"frames=(\d+)", current or "")
            if count and int(count.group(1)) > baseline:
                advanced = current
                break
        row("game callbacks advance while keyboard is open", advanced is not None,
            f"before={first}; after={advanced}")
        set_state = ui_do("ui.set", {"target": dialog["line_edit_osk"], "property": "text", "value": "kbtest"})
        click_state = ui_do("ui.click", {"target": dialog["button_ok"]})
        (OUT / "keyboard-ui.json").write_text(json.dumps({"dialog": dialog, "set": set_state, "click": click_state}, indent=1))
        body = wait_file(f"{SD}/macros/kbtest.json")
        ok = bool(body.strip())
        detail = f"typed kbtest through the applet; macros/kbtest.json {'present' if ok else 'missing'}"
    row("Keyboard applet (release path): a name typed through the real applet saves the macro", ok, detail)
    shot("after-keyboard")
    r.remote(f"Remove-Item -Force {SD}/diag-keyboard.on", 30)

    # roster path: load a named macro of another fighter, answer the prompt Switch, the roster cycle's real provider changes the CPU
    try:
        dh = json.loads(read_file(f"{SD}/macros/duck-hunt-dash.json"))["start"]["fighter_kind"]
    except Exception:  # noqa: BLE001
        dh = None
    st = cmd("status")
    before = line_after(st, "[NamedMacroDiag] status", 6) or ""
    receipt(cmd("open_library 0"), "open_library slot=0")
    at = move_to("Duck Hunt dash")
    cmd("pad confirm")
    time.sleep(2.0)
    line = view()
    (OUT / "roster-prompt-view.txt").write_text(line or "")
    row("Loading a macro of another fighter asks (Fighter Mismatch)", bool(line) and "Fighter Mismatch" in line, (line or "no view")[:260])
    cmd("pad down")
    time.sleep(0.6)
    cmd("pad confirm")  # Switch to recorded fighter
    switched = None
    for _ in range(25):
        time.sleep(3.0)
        st = cmd("status")
        ln = line_after(st, "[NamedMacroDiag] status", 6) or ""
        mm = re.search(r"cpu_kind=Some\((-?\d+)\)", ln)
        if mm and dh is not None and int(mm.group(1)) == dh:
            switched = ln
            break
    roster_lines = [x for x in read_diag() if "[CharacterRoster]" in x or "[MiiOrdinaryCpu]" in x][-6:]
    (OUT / "roster-lines.txt").write_text("\n".join(roster_lines))
    row("Switch to recorded fighter: the roster cycle's provider changes the CPU to the macro's fighter", switched is not None, (switched or "cpu kind did not become " + str(dh) + "; before: " + before[-120:])[-260:])
    shot("after-fighter-switch")

    # hook counts and allocations
    time.sleep(4)
    lines = [x for x in read_diag() if "input_hook_counts" in x or "hook_stats" in x]
    (OUT / "hook-counts.txt").write_text("\n".join(lines))
    last_counts = lines[-1] if lines else ""
    row("hook allocation counter after the layer ran (R5)", "heap_allocations=0" in last_counts, last_counts[-200:] or "no report")
    # Include load and every retained scope; no inherited allocation exception.
    all_diag = "\n".join(read_diag())
    required = ("live_replay", "recorder_ai", "recorder_poll", "named_service", "named_restore")
    passed, reasons = hook_stats.gate(all_diag, required)
    row("R5: every retained hook delta/total is zero and pkg4 scopes ran",
        passed, "; ".join(reasons) or f"covered={required}")
    losses = [line for line in all_diag.splitlines()
              if "[MacroHandoff]" in line and re.search(r"\bd?\w*dropped=[1-9]\d*", line)]
    row("worker handoffs lost no events", not losses, "\n".join(losses) or "no dropped-event reports")
    (OUT / "results.json").write_text(json.dumps(RESULTS, indent=1))
    print(f"ROWS {sum(1 for x in RESULTS if x[1])}/{len(RESULTS)} pass", flush=True)


if __name__ == "__main__":
    main()
