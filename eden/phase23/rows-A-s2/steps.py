"""Phase 2/3 worker-A rows (Yoga half): project hooks over the shared runner's context.

Reuses the phase-1 fixture machinery in base.py (settings-reload fixture route, fighter cycle,
quit/reenter). Evidence clips go through Eden's recording.* control methods. Every row ends back
on the settled Training HUD so the next row chains on it.
"""
import importlib.util
import json
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
_spec = importlib.util.spec_from_file_location('p23base', HERE / 'base.py')
b = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(b)


def dispatch(ctx, action, **args):
    ctx.log({'p23_action': action, 'args': args})
    return globals()[action](ctx, **args)


def chord(ctx, buttons, hold=120):
    names = buttons.split(',')
    try:
        for button in names[:-1]:
            ctx.call('input.button', {'button': button, 'state': 'press', 'lease_ms': 1500})
        ctx.press(names[-1], hold)
    finally:
        ctx.call('input.reset')


def _wait_clip(ctx, path, ms):
    end = time.monotonic() + ms / 1000 + 25
    while time.monotonic() < end:
        status = ctx.call('recording.status')
        if status.get('error'):
            raise RuntimeError(f'recording {path.name}: {status}')
        if status.get('state') in ('finished', 'idle', 'completed') and path.exists():
            return str(path)
        ctx.check_stop()
        time.sleep(0.25)
    raise RuntimeError(f'recording {path.name} did not finalize')


def hit(ctx, name, ms=6000):
    """One P1 forward smash on the CPU, filmed; the settled HUD closes the hit."""
    path = ctx.row_dir / (name + '.mp4')
    ctx.call('recording.start', {'path': str(path), 'duration_ms': ms, 'width': 1280, 'fps': 60})
    ctx.play('tr2-fsmash')
    clip = _wait_clip(ctx, path, ms)
    ctx.gate('training-hud-unpaused')
    return clip


def n10(ctx):
    """N10/E7: the SDI grid's chips show the game's spark; one live hit captured beside it."""
    since = b.fixture(ctx, 'p23-n10.json', fresh=True)
    ctx.play('ui-di')
    ctx.gate('ui-di')
    ctx.play('ui-di-to-sdi-choice')
    ctx.play('ui-sdi')
    ctx.gate('ui-sdi')
    grid = b.capture(ctx, 'sdi-grid', 'SDI Direction grid: chips draw the game spark streaks')
    ctx.play('tr-menu-close')
    ctx.gate('training-hud-unpaused')
    spark = hit(ctx, 'n10-sdi-up-high-spark')
    (ctx.row_dir / 'n10.json').write_text(json.dumps(
        {'fixture_since': since, 'grid': grid, 'spark': spark,
         'side_by_side': 'deferred: c1162/captures-ally-sdi reference not present on this host'}, indent=2) + '\n')
    return {'grid': grid, 'spark': spark, 'since': since}


def n9(ctx):
    """N9: SDI unset (empty state + High strength; Up state + None strength) never SDIs."""
    out = {'a': [], 'b': []}
    b.fixture(ctx, 'p23-n9-a.json', fresh=True)
    for i in (1, 2):
        out['a'].append(hit(ctx, f'n9-a-empty-high-hit{i}'))
    b.fixture(ctx, 'p23-n9-b.json', fresh=True)
    for i in (1, 2):
        out['b'].append(hit(ctx, f'n9-b-up-none-hit{i}'))
    (ctx.row_dir / 'n9-hits.json').write_text(json.dumps(out, indent=2) + '\n')
    return out


def n3(ctx):
    """N3: aerial delay range [2,6]: film 12 forward smashes; the input display shows the delays."""
    since = b.fixture(ctx, 'p23-n3.json', fresh=True)
    path = ctx.row_dir / 'n3-twelve-hits.mp4'
    ctx.call('recording.start', {'path': str(path), 'duration_ms': 60000, 'width': 1280, 'fps': 60})
    for i in range(12):
        ctx.play('tr2-fsmash')
        ctx.gate('training-hud-unpaused')
    clip = _wait_clip(ctx, path, 60000)
    (ctx.row_dir / 'n3-hits.json').write_text(json.dumps(
        {'hits': 12, 'clip': clip, 'pick_rule': 'uniform over {2,3,4,5,6}'}, indent=2) + '\n')
    return {'hits': 12, 'clip': clip, 'since': since}


def n4(ctx):
    """N4: during slot-1 playback the recorded motion wins (override Off) or DI applies (override On)."""
    since = b.fixture(ctx, 'p23-n4.json', fresh=True)
    record_path = ctx.row_dir / 'n4-slot1-record.mp4'
    ctx.call('recording.start', {'path': str(record_path), 'duration_ms': 9000, 'width': 1280, 'fps': 30})
    chord(ctx, 'zr,down')          # Input Record chord (factory ZR + D-pad Down)
    ctx.stick(0, -1, 200)
    ctx.stick(-1, 0, 400)          # neutral-to-left sway, about two seconds
    ctx.stick(0, 0, 200)
    ctx.stick(1, 0, 400)
    ctx.stick(0, 0, 300)
    chord(ctx, 'zr,down')          # same chord stops the record
    _wait_clip(ctx, record_path, 9000)
    ctx.gate('training-hud-unpaused')
    b.capture(ctx, 'n4-recorded', 'Record/playback notification after the slot-1 record')
    # Hit the CPU once, then during the next hit play slot 1 (override Off: recording wins).
    ctx.play('tr2-fsmash')
    ctx.gate('training-hud-unpaused')
    ctx.call('recording.start', {'path': str(ctx.row_dir / 'n4-playback-off.mp4'),
                                 'duration_ms': 8000, 'width': 1280, 'fps': 60})
    ctx.play('tr2-fsmash')
    chord(ctx, 'zr,up')
    off = _wait_clip(ctx, ctx.row_dir / 'n4-playback-off.mp4', 8000)
    ctx.gate('training-hud-unpaused')
    # Override On: the same hit + playback, now with the blue DI streak expected.
    b.reload_settings(ctx, {'menu': {'settings_override_playback': [1, 0]}}, 'n4-override-on')
    ctx.play('tr2-fsmash')
    ctx.gate('training-hud-unpaused')
    ctx.call('recording.start', {'path': str(ctx.row_dir / 'n4-playback-on.mp4'),
                                 'duration_ms': 8000, 'width': 1280, 'fps': 60})
    ctx.play('tr2-fsmash')
    chord(ctx, 'zr,up')
    on = _wait_clip(ctx, ctx.row_dir / 'n4-playback-on.mp4', 8000)
    ctx.gate('training-hud-unpaused')
    b.reload_settings(ctx, {'menu': {'settings_override_playback': [0, 0]}}, 'n4-override-off')
    out = {'record': str(record_path), 'playback_off': off, 'playback_on': on}
    (ctx.row_dir / 'n4.json').write_text(json.dumps(out, indent=2) + '\n')
    return out


def n2(ctx):
    """N2: natural death by walk-off, respawn invincibility Off and On (P1; the CPU half deferred)."""
    out = {}
    b.fixture(ctx, 'p23-n2-off.json', fresh=True)
    out['off'] = [_death(ctx, 'n2-off-walk1'), _death(ctx, 'n2-off-walk2')]
    b.fixture(ctx, 'p23-n2-on.json', fresh=True)
    out['on'] = [_death(ctx, 'n2-on-walk1'), _death(ctx, 'n2-on-walk2')]
    (ctx.row_dir / 'n2.json').write_text(json.dumps(out, indent=2) + '\n')
    return out


def _death(ctx, name):
    path = ctx.row_dir / (name + '.mp4')
    ctx.call('recording.start', {'path': str(path), 'duration_ms': 12000, 'width': 1280, 'fps': 60})
    ctx.play('p23-walkoff')
    clip = _wait_clip(ctx, path, 12000)
    b.capture(ctx, name + '-after', 'After the natural death: respawn platform frame')
    return clip


def s2_boot2(ctx):
    """S2 boot 2: no new settings issue, the recovered file is what this boot read, values kept."""
    lines = []
    diag = Path(ctx.instance['diag_log'])
    for path in (diag.with_name('practice-diagnostics.previous.log'), diag):
        if path.exists():
            lines.extend(path.read_text(errors='replace').splitlines())
    issues = [x for x in lines if 'settings issue at=' in x]
    applied = [x for x in lines if 'diag_settings applied' in x]
    settings_path = Path(ctx.instance['settings'])
    doc = json.loads(settings_path.read_text())
    tilt = doc['menu'].get('shield_tilt', [])
    strength = doc['menu'].get('sdi_strength', [])
    out = {'issues_this_boot': issues, 'applied': applied,
           'shield_tilt_kept': tilt == [0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0],
           'sdi_strength_defaulted': strength == [1, 0, 0, 0]}
    (ctx.row_dir / 's2-boot2.json').write_text(json.dumps(out, indent=2) + '\n')
    ctx.shot('s2-boot2-training')
    if issues:
        raise RuntimeError(f'boot 2 re-reported settings issues: {issues}')
    if not out['shield_tilt_kept'] or not out['sdi_strength_defaulted']:
        raise RuntimeError(f'recovered values not kept: shield_tilt={tilt} sdi_strength={strength}')
    return out


def b7(ctx):
    """B7 side 1: hit the CPU into aerial hitstun; its Out/In air dodge and shield tilt are
    opponent-relative. The second side needs a recorded side-swap route (deferred this boot)."""
    since = b.fixture(ctx, 'p23-b7.json', fresh=True)
    b.capture(ctx, 'b7-side1-ready', 'P1 and CPU placement before the up-air (side 1)')
    clip = hit(ctx, 'b7-side1-nair', ms=9000)
    (ctx.row_dir / 'b7.json').write_text(json.dumps(
        {'side': 1, 'clip': clip,
         'side2': 'deferred: no recorded CSS side-swap segment in this lane'}, indent=2) + '\n')
    return {'side1': clip, 'since': since}