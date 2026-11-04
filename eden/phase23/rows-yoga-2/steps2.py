"""Phase 2/3 Yoga-2 rows (the rows the first Yoga run deferred): project hooks over the shared runner's context.

Reuses rows-A's base.py fixture machinery and steps.py helpers. The menu rows (Q7, N7) navigate the
plugin menu with hook presses and gates (the menu itself is under test; scripted inputs in a hook are
fine per MACROS "Test runs"). B7 side 2 builds its missing CSS side-swap route on the boot and records
it; the N2 CPU half drives the CPU with the plugin's own input playback. Every row ends back on the
settled Training HUD.
"""
import hashlib
import importlib.util
import json
import re
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent


def _load(name, path):
    spec = importlib.util.spec_from_file_location(name, HERE / path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


b = _load('p23base', 'base.py')
steps = _load('p23steps', 'steps.py')
nav = _load('p23nav', 'navigation.py')


def dispatch(ctx, action, **args):
    try:
        settled = steps.settle(ctx)
    except Exception as error:                     # a CSS/unknown state: the known-screen recover routes
        settled = b.recover(ctx, f'settle after {error}')
    ctx.log({'p23_action': action, 'args': args, 'settle': settled})
    return globals()[action](ctx, **args)


TRACE_ELF = '/home/intpa/tr-p23-yoga-2/libtraining_rooms.so'


def _arm(ctx, target, registers, on_return=True, max_hits=256, rate_limit=1000, captures=None):
    try:
        ctx.call('symbols.load', {'module': 'libtraining_rooms.nro', 'elf': TRACE_ELF})
    except RuntimeError as error:
        ctx.log({'symbols_load': str(error)[:120]})
    point = ctx.call('trace.arm', {'target': target, 'registers': registers, 'on_return': on_return,
                                   'max_hits': max_hits, 'rate_limit': rate_limit, 'captures': captures or []})
    ident = (point.get('id') if isinstance(point, dict) else None) or '0'
    ctx.log({'trace_armed': {'id': ident, 'target': target}})
    return ident


def _disarm(ctx, ident):
    try:
        ctx.call('trace.disarm', {'id': ident})
    except RuntimeError as error:
        ctx.log({'trace_disarm': str(error)[:120]})


def _trace_summary(ctx, ident):
    """The armed point's hit/record counters, for the receipt."""
    try:
        points = ctx.call('trace.list').get('points', [])
        for p in points:
            if str(p.get('id')) == str(ident):
                return {k: p.get(k) for k in ('id', 'state', 'hits', 'recorded', 'dropped', 'return_overflow')}
    except RuntimeError as error:
        return {'error': str(error)[:120]}
    return None


def chord(ctx, buttons, hold=120):
    """A two-button chord as a same-frame macro (MACROS rule 14); steps.chord's sequential
    input.button calls straddle frames and the chord never fires."""
    names = {'zl,up': 'p23-chord-zl-up', 'minus': 'p23-chord-minus',
             'zr,down': 'p23-chord-zr-down', 'zr,up': 'p23-chord-zr-up'}
    ctx.play(names[buttons])


def plugin_dir(ctx):
    return Path(ctx.instance['settings']).parent


def copy_ini(ctx, name, label):
    src = plugin_dir(ctx) / name
    dst = ctx.row_dir / f'{label}-{name}'
    dst.write_text(src.read_text(errors='replace'), newline='')
    ctx.log({'copied': str(src), 'to': str(dst), 'sha256': hashlib.sha256(src.read_bytes()).hexdigest()})
    return str(dst)


def diag_lines_since(ctx, since):
    g, _, _, read = b.tools(ctx)
    return g._new(read, since)


def open_menu(ctx, label):
    ctx.gate('training-hud-unpaused')
    ctx.play('tr-menu-open')
    ctx.wait_log('diag', r'\[Ui\] menu opened', cap=15)
    return b.capture(ctx, label, 'Plugin menu open; page and focused row reviewed in the row receipt')


def _step_once(ctx, direction, expect_from):
    """One tile-cursor step; returns the tile the step landed on (from the cursor's own log)."""
    since = b.mark(ctx)
    nav._tile_step(ctx, direction, expect_from, r'\d+')
    line = None
    for x in diag_lines_since(ctx, since):
        if re.search(rf'step screen=css dir={direction} from={expect_from} to=(\d+)', x):
            line = x
    if line is None:
        return None
    return int(re.search(rf'dir={direction} from={expect_from} to=(\d+)', line).group(1))


def _settled_tile(ctx, since):
    at = None
    for line in diag_lines_since(ctx, since):
        m = re.search(r'settled screen=css at=(\d+)', line)
        if m:
            at = int(m.group(1))
    return at


def _film_hit(ctx, name, ms=9000):
    path = ctx.row_dir / (name + '.gif')
    ctx.call('recording.start', {'path': str(path), 'duration_ms': ms, 'width': 640, 'fps': 15})
    ctx.play('tr2-fsmash')
    clip = steps._wait_clip(ctx, path, ms)
    ctx.gate('training-hud-unpaused')
    return clip


def _route(ctx, name, navigate, cap=40):
    """Play the recorded route segment when it exists; otherwise record it once (MACROS rule 13)."""
    path = HERE / 'segments' / f'{name}.edenmacro.json'
    seg = ctx.runner.plan['segments'].get(name)
    if seg and (HERE / seg['file']).is_file():
        ctx.play(name)
        return 'played'
    ctx.call('macro.record', {'name': name, 'path': str(path), 'player': 0, 'countdown_ms': 0})
    try:
        navigate()
    finally:
        ctx.call('input.reset')
        time.sleep(0.12)
        ctx.call('macro.stop')
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    ctx.runner.plan['segments'][name] = {'file': f'segments/{name}.edenmacro.json', 'sha256': digest, 'cap': cap,
                                         'start_gate': None, 'end_gate': None}
    ctx.runner.manifest_path.write_text(json.dumps(ctx.runner.plan, indent=2) + '\n')
    ctx.log({'recorded_segment': name, 'path': str(path), 'sha256': digest})
    return f'recorded sha256={digest}'


SDI_TRACE = 'libtraining_rooms.nro!cpu_sdi_hook'


def n9t(ctx):
    """N9 redo: SDI unset never answers an input; the SDI hook's returns are traced per hit."""
    out = {}
    ident = _arm(ctx, SDI_TRACE, ['x0', 'x1', 'x30'], max_hits=256)
    b.fixture(ctx, 'p23-n9-a.json', fresh=True)        # sdi_state empty, strength High
    out['a'] = [_film_hit(ctx, f'n9-a-empty-high-hit{i}', ms=6000) for i in (1, 2)]
    b.fixture(ctx, 'p23-n9-b.json', fresh=True)        # sdi_state Up, strength None
    out['b'] = [_film_hit(ctx, f'n9-b-up-none-hit{i}', ms=6000) for i in (1, 2)]
    out['trace'] = _trace_summary(ctx, ident)
    _disarm(ctx, ident)
    (ctx.row_dir / 'n9.json').write_text(json.dumps(out, indent=2) + '\n')
    return out


def n10t(ctx):
    """N10 redo: the SDI grid shows the fixture's own Up chip (no ui-* segment touches the grid),
    and one live hit with the SDI hook traced."""
    out = {}
    b.fixture(ctx, 'p23-n10.json', fresh=True)         # sdi_state Up (index 2), strength High
    ctx.gate('training-hud-unpaused')
    ctx.play('tr-menu-open')
    ctx.wait_log('diag', r'\[Ui\] menu opened', cap=15)
    time.sleep(1.5)
    for _ in range(7):                                  # Defence group: SDI Direction is the 7th row
        ctx.press('down', 150)
        time.sleep(0.35)
    ctx.press('a', 150)                                 # open the SDI Direction grid
    time.sleep(1.5)
    out['grid'] = b.capture(ctx, 'n10-sdi-grid', 'SDI Direction grid with the fixture applied: the Up chip ticked x1, no A press inside')
    ctx.press('b', 120)                                 # back to the page
    time.sleep(0.8)
    steps.close_menu(ctx)
    ctx.gate('training-hud-unpaused')
    ident = _arm(ctx, SDI_TRACE, ['x0', 'x1', 'x30'], max_hits=256)
    out['spark'] = _film_hit(ctx, 'n10-sdi-up-high-spark', ms=8000)
    out['trace'] = _trace_summary(ctx, ident)
    _disarm(ctx, ident)
    (ctx.row_dir / 'n10.json').write_text(json.dumps(out, indent=2) + '\n')
    return out


def n3t(ctx):
    """N3 redo: 12 forward smashes under the [2,6] range fixture, filmed; the CPU's aerial issuance
    hook is traced (the pick itself is inlined model code with no symbol to trace)."""
    out = {}
    b.fixture(ctx, 'p23-n3.json', fresh=True)
    ctx.gate('training-hud-unpaused')
    ident = _arm(ctx, 'libtraining_rooms.nro!cpu_attack_air_kind_hook', ['x0', 'x30'], max_hits=512)
    path = ctx.row_dir / 'n3-twelve-hits.gif'
    ctx.call('recording.start', {'path': str(path), 'duration_ms': 60000, 'width': 640, 'fps': 15})
    for _ in range(12):
        ctx.play('tr2-fsmash')
        ctx.gate('training-hud-unpaused')
    out['clip'] = steps._wait_clip(ctx, path, 60000)
    out['trace'] = _trace_summary(ctx, ident)
    _disarm(ctx, ident)
    (ctx.row_dir / 'n3.json').write_text(json.dumps(out, indent=2) + '\n')
    return out


def n4t(ctx):
    """N4 redo: the recorded slot is a real stick sway; the Input Playback chord fires during the
    CPU's hitstun (the p23-hit-playback macro), Off vs On override, filmed and DI-traced."""
    out = {}
    b.fixture(ctx, 'p23-n4.json', fresh=True)          # DI Up, SDI none, settings_override_playback Off
    record_path = ctx.row_dir / 'n4-slot1-record.gif'
    ctx.call('recording.start', {'path': str(record_path), 'duration_ms': 9000, 'width': 640, 'fps': 15})
    chord(ctx, 'zr,down')                  # Input Record chord
    ctx.stick(0, -1, 200)
    ctx.stick(-1, 0, 400)                  # neutral-to-left sway, about two seconds
    ctx.stick(0, 0, 200)
    ctx.stick(1, 0, 400)
    ctx.stick(0, 0, 300)
    chord(ctx, 'zr,down')                  # stop the record
    out['record'] = steps._wait_clip(ctx, record_path, 9000)
    ctx.gate('training-hud-unpaused')
    ident = _arm(ctx, 'libtraining_rooms.nro!cpu_di_hook', ['x0', 'x30'], max_hits=512)
    for case in ('off', 'on'):
        if case == 'on':
            b.reload_settings(ctx, {'menu': {'settings_override_playback': [1, 0]}}, 'n4-override-on')
        path = ctx.row_dir / f'n4-playback-{case}.gif'
        since = b.mark(ctx)
        ctx.call('recording.start', {'path': str(path), 'duration_ms': 9000, 'width': 640, 'fps': 15})
        ctx.play('p23-hit-playback-air')   # the aerial hit, and the playback chord inside the CPU's air hitstun
        out[f'playback_{case}'] = steps._wait_clip(ctx, path, 9000)
        time.sleep(1.0)
        lines = diag_lines_since(ctx, since)
        out[f'di_lines_{case}'] = [x[-160:] for x in lines if 'di entry=' in x or 'di_before' in x][:8]
        if case == 'on' and not out[f'di_lines_{case}']:
            raise RuntimeError('no DI applied during playback with the override On')
        steps.settle(ctx)
        ctx.gate('training-hud-unpaused')
    b.reload_settings(ctx, {'menu': {'settings_override_playback': [0, 0]}}, 'n4-override-off')
    out['di_trace'] = _trace_summary(ctx, ident)
    _disarm(ctx, ident)
    (ctx.row_dir / 'n4.json').write_text(json.dumps(out, indent=2) + '\n')
    return out


def q7(ctx):
    """Q7: the Replay-capture toggle writes replay_capture.ini once per confirmed change."""
    out = {'writes': []}
    out['boot_ini'] = copy_ini(ctx, 'replay_capture.ini', 'q7-boot')
    b.fixture(ctx, 'p23-q7.json', fresh=True)          # last_page=recording (pushed before any menu open)
    ctx.gate('training-hud-unpaused')
    ctx.play('tr-menu-open')
    ctx.wait_log('diag', r'\[Ui\] menu opened', cap=15)
    time.sleep(1.5)                                     # the menu's open fade discards the first inputs

    def navigate():
        for _ in range(14):                             # rows only (headers skipped): land on Replay capture
            ctx.press('down', 150)
            time.sleep(0.35)
    out['route'] = _route(ctx, 'q7-menu-to-row', navigate)
    out['page_shot'] = b.capture(ctx, 'q7-replay-row', 'Recording page, focus on the Replay capture row')
    since = b.mark(ctx)
    ctx.press('a', 150)                                 # confirmed change 1: On -> Off
    out['writes'].append(b.wait(ctx, since, r'\[TrainingRooms\] replay_capture\.ini written by the writer thread enabled=\S+', 15))
    out['ini_off'] = copy_ini(ctx, 'replay_capture.ini', 'q7-after-off')
    out['off_shot'] = b.capture(ctx, 'q7-after-off', 'Replay capture row Off (the Deluxe Plugins-page row check is not staged in this lane)')
    steps.close_menu(ctx)
    ctx.gate('training-hud-unpaused')
    ctx.play('tr-menu-open')                            # reopen: the page and the cursor are kept
    ctx.wait_log('diag', r'\[Ui\] menu opened', cap=15)
    time.sleep(1.5)
    since = b.mark(ctx)
    ctx.press('a', 150)                                 # confirmed change 2: Off -> On
    out['writes'].append(b.wait(ctx, since, r'\[TrainingRooms\] replay_capture\.ini written by the writer thread enabled=\S+', 15))
    out['ini_on'] = copy_ini(ctx, 'replay_capture.ini', 'q7-after-on')
    out['on_shot'] = b.capture(ctx, 'q7-after-on', 'Replay capture row On again')
    steps.close_menu(ctx)
    ctx.gate('training-hud-unpaused')
    out['down_ini'] = copy_ini(ctx, 'replay_capture.ini', 'q7-after-down')
    (ctx.row_dir / 'q7.json').write_text(json.dumps(out, indent=2) + '\n')
    return out


def n7(ctx):
    """N7: capture a menu shortcut in the editor, confirm, then apply it with the menu closed."""
    out = {}
    out['boot_ini'] = copy_ini(ctx, 'button_config.ini', 'n7-boot')
    b.fixture(ctx, 'p23-n7.json', fresh=True)          # last_page=display_controls
    ctx.gate('training-hud-unpaused')
    ctx.play('tr-menu-open')
    ctx.wait_log('diag', r'\[Ui\] menu opened', cap=15)
    time.sleep(1.5)                                     # the menu's open fade discards the first inputs

    def navigate():
        for _ in range(8):                              # rows only (headers skipped): land on Menu shortcut 1
            ctx.press('down', 100)
            time.sleep(0.25)
        ctx.press('a', 150)                             # open the chord editor
        time.sleep(1.2)
    out['route'] = _route(ctx, 'n7-menu-to-editor', navigate)
    out['page_shot'] = b.capture(ctx, 'n7-shortcut-row', 'Menu shortcut 1 editor reached')
    out['editor_shot'] = b.capture(ctx, 'n7-editor-open', 'Menu shortcut 1 editor open, captured chord preview shown')
    ctx.press('x', 120)                                 # arm the physical capture
    time.sleep(0.6)
    ctx.call('input.button', {'button': 'zl', 'state': 'press', 'lease_ms': 2500})
    time.sleep(0.35)
    ctx.press('up', 250)                                # the second button while ZL is held
    time.sleep(0.8)
    ctx.call('input.reset')                             # release everything: the capture settles
    time.sleep(0.8)
    out['captured_shot'] = b.capture(ctx, 'n7-captured', 'Editor readback shows the captured chord (ZL + DPad Up)')
    since = b.mark(ctx)
    ctx.press('a', 150)                                 # confirm: the INI is written at once
    out['write'] = b.wait(ctx, since, r'\[TrainingRooms\] button_config\.ini written by the writer thread', 15)
    out['ini_after'] = copy_ini(ctx, 'button_config.ini', 'n7-after-confirm')
    out['readback_shot'] = b.capture(ctx, 'n7-readback', 'The row now shows the new chord')
    steps.close_menu(ctx)
    ctx.gate('training-hud-unpaused')
    # With the menu closed: the new chord opens the menu, the old default chord does not; repeat once.
    for i in (1, 2):
        chord(ctx, 'zl,up')
        ctx.wait_log('diag', r'\[Ui\] menu opened', cap=15)
        out[f'new-opens-{i}'] = b.capture(ctx, f'n7-new-opens-{i}', 'Menu opened with the new chord, menu closed')
        steps.close_menu(ctx)
        ctx.gate('training-hud-unpaused')
        since = b.mark(ctx)
        chord(ctx, 'minus')
        time.sleep(3.0)
        stray = [x for x in diag_lines_since(ctx, since) if '[Ui] menu opened' in x]
        if stray:
            raise RuntimeError(f'the old default chord opened the menu: {stray[-1]}')
        out[f'old-idle-{i}'] = b.capture(ctx, f'n7-old-idle-{i}', 'Minus with the menu closed: no menu opened')
    out['ini_final'] = copy_ini(ctx, 'button_config.ini', 'n7-final')
    (ctx.row_dir / 'n7.json').write_text(json.dumps(out, indent=2) + '\n')
    return out


def q16(ctx):
    """Q16: the plugin's grids draw the game's own stock icon at run time (ARCropolis pinned).

    Hook navigation: the menu opens on the CPU page (no last_page in the boot fixture), R three times
    reaches Fighters & Stages, one Down is Player Character (the roster grid), ten more reach
    Lobby Stages (the stage grid). The counts are verified by the shots in the receipt.
    """
    out = {'arcropolis': 'pinned in the run config (card libarcropolis.nro 92171b2f…) and live in the profile mods'}
    b.fixture(ctx, 'p23-q16.json', fresh=True)          # last_page=fighters_stages: the menu opens on the page
    ctx.gate('training-hud-unpaused')
    ctx.play('tr-menu-open')
    ctx.wait_log('diag', r'\[Ui\] menu opened', cap=15)
    time.sleep(1.5)                                     # the menu's open fade discards the first inputs
    ctx.press('down', 150)                              # Player Character (the first row after the header)
    time.sleep(0.5)
    out['page_shot'] = b.capture(ctx, 'q16-page', 'Fighters & Stages page, focus on Player Character')
    ctx.press('a', 150)                                 # -> the roster grid
    ctx.wait_log('diag', r'\[Thumbs\] bound=\d+ loading=\d+', cap=25)
    out['roster_grid'] = b.capture(ctx, 'q16-roster-grid', 'Roster grid under ARCropolis: stock icons draw (picture=0) or the known blank-tile finding reproduces')
    ctx.press('b', 120)                                 # back to the page
    time.sleep(0.8)
    for _ in range(10):
        ctx.press('down', 150)
        time.sleep(0.35)
    ctx.press('a', 150)                                 # Lobby Stages -> the stage grid
    time.sleep(2.0)
    out['stage_grid'] = b.capture(ctx, 'q16-stage-grid', 'Stages grid under ARCropolis: the Final Destination tile draws the Cosmic Destination art')
    ctx.press('b', 120)                                 # back to the page
    time.sleep(0.6)
    steps.close_menu(ctx)
    ctx.gate('training-hud-unpaused')
    out['note'] = ('the modded-costume icon case and the unreadable-file fallback case are not staged in this lane: '
                   'no owned fighter/costume mod exists on either host, and staging a fixture under an owned temporary '
                   'mods directory needs its own manifest plus the lead\u2019s helper-hold admission (EDEN-UI-SCREENS.md)')
    (ctx.row_dir / 'q16.json').write_text(json.dumps(out, indent=2) + '\n')
    return out


def _record_segment(ctx, name):
    path = HERE / 'segments' / f'{name}.edenmacro.json'
    ctx.call('macro.record', {'name': name, 'path': str(path), 'player': 0, 'countdown_ms': 0})
    return path


def _stop_segment(ctx, path, name):
    ctx.call('input.reset')
    time.sleep(0.12)
    ctx.call('macro.stop')
    digest = hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else None
    ctx.log({'recorded_segment': name, 'path': str(path), 'sha256': digest})
    return digest


def _recall_and_move(ctx, log, steps_left=3):
    """Take P1's token back into the hand and place it on a free tile just left of the CPU's.

    Side 1 (run A): P1's token at 78 spawned LEFT of the CPU at 74, and the CPU was re-picked after
    P1. Moving P1 to an index below the CPU's AND picking it last satisfies both candidate side rules
    (tile index and pick order) at once, so the mirror side needs only a step or two, not a walk.
    """
    since = b.mark(ctx)
    ctx.press('b', 120)                    # take P1's token back into the hand
    time.sleep(0.8)
    at = _settled_tile(ctx, since)
    log['hand_after_recall'] = at
    placed = None
    for _ in range(steps_left):
        to = _step_once(ctx, 'left', at)
        if to is None:
            break
        at = to
        ctx.press('a', 120)                # place the token
        time.sleep(1.0)
        placed = _settled_tile(ctx, since)
        log['placed_at'] = placed
        if placed == at:
            break
        ctx.press('b', 120)                # the tile took nothing: take the token back and try again
        time.sleep(0.8)
    return placed


def b7side2(ctx):
    """B7 side 2: put P1 on the other side of the CPU, re-enter, and repeat the side-1 hit.

    Side 1 (run A): P1's token at 78 spawned LEFT of the CPU at 74, and the CPU was re-picked after
    P1. The hook recalls P1's token after the CPU re-pick and places it just left of the CPU's tile,
    so both candidate side rules (tile index, pick order) land P1 on the mirrored side; the clip and
    the placement shot decide, and the CSS work is recorded as a reusable segment.
    """
    out = {}
    b.fixture(ctx, 'p23-b7.json', fresh=True)
    b.quit_css(ctx)
    ctx.gate('ready')
    out['css_start'] = b.capture(ctx, 'b7-side2-css-start', 'CSS after the quit: the CPU is re-picked next')
    nav.return_cpu_to_lucas(ctx)           # the CPU token back on Lucas
    ctx.gate('ready')
    seg = _record_segment(ctx, 'b7-side2-swap')
    try:
        placed = _recall_and_move(ctx, out)
    finally:
        out['swap_sha256'] = _stop_segment(ctx, seg, 'b7-side2-swap')
    if placed is None:
        raise RuntimeError('P1\'s token did not place on a tile left of the CPU; see the row shots')
    ctx.gate('ready')
    out['css_swapped'] = b.capture(ctx, 'b7-side2-css-swapped', 'CSS after the swap: which side P1 takes is read from this frame')
    ctx.play('enter-training')
    ctx.gate('training-hud-unpaused')
    out['side2_clip'] = _film_hit(ctx, 'b7-side2-nair', ms=9000)
    out['side2_after'] = b.capture(ctx, 'b7-side2-after', 'Side-2 hit: the CPU airdodge/shield-tilt directions are read from the clip')
    (ctx.row_dir / 'b7-side2.json').write_text(json.dumps(out, indent=2) + '\n')
    return out


def n2t(ctx):
    """N2 redo: real natural deaths (past the blast zone, no ledge grab) for P1 and the CPU, respawn
    invincibility Off vs On, each respawn filmed at 640/15, with the param-override hook traced."""
    out = {}
    b.fixture(ctx, 'p23-n2-off.json', fresh=True)
    out['p1_off'] = _death(ctx, 'n2-p1-off-walk')
    b.fixture(ctx, 'p23-n2-on.json', fresh=True)
    out['p1_on'] = _death(ctx, 'n2-p1-on-walk')
    # The CPU: record P1's walk-off inputs into slot 1, then play them back on the CPU.
    record_path = ctx.row_dir / 'n2-record-slot1.gif'
    ctx.call('recording.start', {'path': str(record_path), 'duration_ms': 10000, 'width': 640, 'fps': 15})
    chord(ctx, 'zr,down')            # Input Record chord (factory ZR + D-pad Down)
    ctx.stick(-1, 0, 5200)                 # hold left ~5 s (the walk-off direction)
    chord(ctx, 'zr,down')            # stop the record
    out['record'] = steps._wait_clip(ctx, record_path, 10000)
    ctx.gate('training-hud-unpaused')
    for case, fixture in (('on', 'p23-n2-on.json'), ('off', 'p23-n2-off.json')):
        b.fixture(ctx, fixture, fresh=True)
        ident = _arm(ctx, 'libtraining_rooms.nro!state_get_param_int_hook', ['x0', 'x1', 'x2', 'x30'], max_hits=200, rate_limit=200)
        path = ctx.row_dir / f'n2-cpu-{case}.gif'
        ctx.call('recording.start', {'path': str(path), 'duration_ms': 12000, 'width': 640, 'fps': 15})
        ctx.play('tr2-fsmash')             # hit the CPU, then play slot 1 during the hitstun
        chord(ctx, 'zr,up')          # Input Playback chord (factory ZR + D-pad Up)
        out[f'cpu_{case}'] = steps._wait_clip(ctx, path, 12000)
        out[f'cpu_{case}_respawn'] = b.capture(ctx, f'n2-cpu-respawn-{case}', f'CPU natural death by the playback walk-off; respawn invincibility {case}')
        out[f'cpu_{case}_trace'] = _trace_summary(ctx, ident)
        _disarm(ctx, ident)
        steps.settle(ctx)
        ctx.gate('training-hud-unpaused')
    (ctx.row_dir / 'n2.json').write_text(json.dumps(out, indent=2) + '\n')
    return out


def _death(ctx, name):
    path = ctx.row_dir / (name + '.gif')
    ctx.call('recording.start', {'path': str(path), 'duration_ms': 12000, 'width': 640, 'fps': 15})
    ctx.play('p23-walkoff-left')
    clip = steps._wait_clip(ctx, path, 12000)
    b.capture(ctx, name + '-after', 'After the natural death: the respawn frame')
    return clip