"""Frame strip rows (SPEC-input-06 §8, IV-1..IV-5) for the diag build of p23/input-viz.

Every row passes on the strip's own diag lines (`[FrameStrip] ...`), read from the plugin's diag file, plus
screenshots of the strip. Inputs are authored same-frame macros (rows.json segments).
"""
import json
import re
import time
from pathlib import Path


def save(ctx, name, value):
    text = json.dumps(value, indent=2, default=str)
    (ctx.row_dir / (name + '.json')).write_text(text + '\n')
    return json.loads(text)  # the runner writes the return into its receipt


def diag_lines(ctx):
    diag = Path(ctx.instance['diag_log'])
    text = ''
    for p in (diag.with_name('practice-diagnostics.previous.log'), diag):
        if p.exists():
            text += p.read_text(errors='replace')
    return [l for l in text.splitlines() if '[FrameStrip]' in l or 'diag_settings_reload' in l]


def new_lines(ctx, before, pattern=None, cap=10.0):
    """Strip lines written after the `before` snapshot (by content), waiting up to cap s for `pattern`."""
    seen = set(before)
    deadline = time.monotonic() + cap
    while True:
        fresh = [l for l in diag_lines(ctx) if l not in seen]
        if pattern is None or any(re.search(pattern, l) for l in fresh) or time.monotonic() > deadline:
            return fresh
        time.sleep(0.25)


def push(ctx, changes):
    """The plugin's diag settings reload: the live settings with `changes` in the menu object; wait for the ack."""
    live_path = Path(ctx.instance['settings'])
    live = json.loads(live_path.read_text())
    live.setdefault('menu', {}).update(changes)
    root = live_path.parent
    temp = root / 'diag-settings-next.tmp'
    temp.write_text(json.dumps(live) + '\n')
    temp.replace(root / 'diag-settings-next.json')
    return ctx.wait_log('diag', r'diag_settings_reload .*issues=0\b', cap=20)


def ready(ctx):
    """The strip's panes and constants are there, and the old input_display row migrated to the strip."""
    ctx.gate('training-hud-unpaused')
    lines = diag_lines(ctx)
    panes = [l for l in lines if 'panes resolved' in l]
    consts = [l for l in lines if 'constants resolved' in l]
    settings = json.loads(Path(ctx.instance['settings']).read_text())
    menu = settings.get('menu', {})
    shot = ctx.shot('iv-ready')
    out = save(ctx, 'ready', {'panes': panes[-2:], 'constants': consts[-2:], 'input_display_strip': menu.get('input_display_strip'),
                               'old_rows_left': [k for k in ('input_display', 'input_display_status') if k in menu], 'shot': shot})
    if not panes or 'found=121 of 121' not in panes[-1]:
        raise RuntimeError('strip panes not all resolved: %s' % panes[-1:])
    if not consts or 'missing=0' not in consts[-1]:
        raise RuntimeError('strip constants: %s' % consts[-1:])
    return out


def attempt(ctx, seg):
    before = diag_lines(ctx)
    ctx.play('iv-reset-walk')
    time.sleep(0.4)
    ctx.play(seg)
    fresh = new_lines(ctx, before, r'\[FrameStrip\] (link|advantage)', cap=4.0)
    time.sleep(0.9)  # the strip freezes 30 frames after both can act: the exchange stays on screen
    shot = ctx.shot(seg)
    fresh = [l for l in diag_lines(ctx) if l not in set(before)]
    keep = [l for l in fresh if 'reaction r=' not in l]
    return {'segment': seg, 'lines': keep, 'reactions': [l for l in fresh if 'reaction r=' in l][-40:], 'shot': str(shot)}


def links(ctx):
    """IV-1/IV-2: jab -> up tilt at several press timings; the strip's link lines (TRUE/DROP, input) and the stun
    checks (the hitstun read's prediction against the free frame seen)."""
    ctx.gate('training-hud-unpaused')
    tries = []
    # Up tilt first (the brief's move; it misses a short CPU), then the jab's own follow-up and down tilt.
    segs = ['iv-combo-%02d' % n for n in (10, 14, 18, 26)] + ['iv-jj-%02d' % n for n in (6, 9, 12, 16, 22, 30)] \
        + ['iv-jd-%02d' % n for n in (10, 16, 22, 30)]
    for seg in segs:
        tries.append(attempt(ctx, seg))
        ctx.check_stop()
    text = '\n'.join(l for t in tries for l in t['lines'])
    verdicts = re.findall(r'link verdict=(\w+)\(([^)]*)\)[^\n]*input=(-?\d+)', text)
    checks = re.findall(r'stun_check predicted=(\d+) seen=(\d+)', text)
    out = save(ctx, 'links', {'tries': tries, 'verdicts': verdicts, 'stun_checks': checks})
    if not verdicts:
        raise RuntimeError('no link line in %d tries' % len(tries))
    return {'verdicts': verdicts, 'stun_checks': checks}


def shield(ctx):
    """IV-3: the CPU shields (Infinite) a jab: advantage on shield, no link."""
    ctx.gate('training-hud-unpaused')
    ack = push(ctx, {'shield_state': [0, 1, 0, 0]})
    time.sleep(1.0)
    tries = []
    for _ in range(3):
        before = diag_lines(ctx)
        ctx.play('iv-reset-walk')
        time.sleep(1.0)
        ctx.play('iv-jab')
        fresh = new_lines(ctx, before, r'advantage value=-?\d+ on_shield=true', cap=4.0)
        time.sleep(0.9)
        shot = ctx.shot('iv-shield')
        tries.append({'lines': [l for l in fresh if 'reaction r=' not in l], 'shot': str(shot)})
        if any('on_shield=true' in l for l in fresh):
            break
    back = push(ctx, {'shield_state': [1, 0, 0, 0]})
    out = save(ctx, 'shield', {'ack': ack, 'tries': tries, 'restored': back})
    if not any('on_shield=true' in l for t in tries for l in t['lines']):
        raise RuntimeError('no advantage on shield')
    return out


def hide(ctx):
    """IV-4: the vanilla pause and the plugin menu hide the strip; it comes back after each."""
    ctx.gate('training-hud-unpaused')
    before = diag_lines(ctx)
    ctx.press('plus', 150)
    paused = new_lines(ctx, before, r'shown=false menu_open=true', cap=6.0)
    time.sleep(0.6)
    s1 = ctx.shot('iv-pause-hidden')
    ctx.press('plus', 150)
    resumed = new_lines(ctx, before, r'shown=true', cap=6.0)
    ctx.gate('training-hud-unpaused')
    before = diag_lines(ctx)
    ctx.play('iv-menu-open')
    menu = new_lines(ctx, before, r'shown=false menu_open=true', cap=6.0)
    time.sleep(0.8)
    s2 = ctx.shot('iv-menu-hidden')
    ctx.play('iv-menu-close')
    back = new_lines(ctx, before, r'shown=true', cap=6.0)
    ctx.gate('training-hud-unpaused')
    s3 = ctx.shot('iv-shown-again')
    out = save(ctx, 'hide', {'pause': paused + resumed, 'menu': menu + back, 'shots': [str(s1), str(s2), str(s3)]})
    if not any('shown=false menu_open=true' in l for l in paused) or not any('shown=false menu_open=true' in l for l in menu):
        raise RuntimeError('the strip did not hide: %s' % out)
    if not any('shown=true' in l for l in back):
        raise RuntimeError('the strip did not come back after the menu')
    return out


def off(ctx):
    """IV-5: the row Off hides the strip and stops sampling; On brings it back."""
    ctx.gate('training-hud-unpaused')
    before = diag_lines(ctx)
    ack = push(ctx, {'input_display_strip': [1, 0]})
    hidden = new_lines(ctx, before, r'shown=false menu_open=false on=false', cap=6.0)
    time.sleep(0.5)
    s1 = ctx.shot('iv-off')
    before2 = diag_lines(ctx)
    ctx.play('iv-jab')
    time.sleep(1.0)
    quiet = [l for l in diag_lines(ctx) if l not in set(before2) and ('link' in l or 'advantage' in l or 'reaction' in l)]
    back = push(ctx, {'input_display_strip': [0, 1]})
    shown = new_lines(ctx, before2, r'shown=true', cap=6.0)
    s2 = ctx.shot('iv-on-again')
    out = save(ctx, 'off', {'ack': ack, 'hidden': hidden, 'lines_while_off': quiet, 'back': back, 'shown': shown,
                             'shots': [str(s1), str(s2)]})
    if not any('on=false' in l and 'shown=false' in l for l in hidden):
        raise RuntimeError('Off did not hide the strip')
    if quiet:
        raise RuntimeError('the strip sampled while Off: %s' % quiet[:3])
    return out


def gif(ctx):
    """The docs GIF: 1280 wide, under 10 s, the strip during jab -> up tilt and a second exchange."""
    ctx.gate('training-hud-unpaused')
    ctx.play('iv-reset-walk')
    time.sleep(0.3)
    path = ctx.row_dir / 'frame-strip.gif'
    ctx.call('recording.start', {'path': str(path), 'fps': 15, 'width': 1280, 'duration_ms': 9000})
    time.sleep(0.3)
    ctx.play('iv-combo-14')
    time.sleep(1.5)
    ctx.play('iv-combo-26')
    status = None
    for _ in range(40):
        status = ctx.call('recording.status')
        if isinstance(status, dict) and status.get('state') not in ('recording', 'finalizing'):
            break
        time.sleep(0.5)
    out = save(ctx, 'gif', {'path': str(path), 'status': status, 'exists': path.exists(),
                             'bytes': path.stat().st_size if path.exists() else 0})
    if not path.exists():
        raise RuntimeError('no GIF written: %s' % status)
    return out
