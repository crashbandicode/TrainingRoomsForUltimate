"""Hitbox own-orbs A/B (HB1 route A4 prototype, diag NRO e78ad90a, branch p23/hitbox-orbs 97d6b57).

Fighters come from in-fight settings pushes (diag-settings-next.json, the Ally gbc-04 route): P1 Ridley (catalog 63),
CPU Steve (76), CPU Behavior Stop (Training's default). The route and the A/B view come from diag-hitbox-route.req.
Each pair is shot on one frozen frame: a macro pauses the game on a fixed game frame (Plus at frame N of the macro),
and while paused only the view changes (route B's spheres hidden per handle from the P1 input hook, the orb panes
from the layout tick). Trace counters on publish_orbs (game thread) prove no game frame ran between the shots.
"""
import json
import os
import re
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ELF = '/home/intpa/smash-training-lobby/artifacts/phase23/hitbox-orbs/nro-97d6b57/diag/libtraining_rooms.so'
PUBLISH = 'libtraining_rooms.nro!_RNvNtNtNtCskE0RJeV9NF0_14training_rooms7runtime8features6hitbox12publish_orbs'
DRAW = 'libtraining_rooms.nro!_RNvNtNtNtCskE0RJeV9NF0_14training_rooms7runtime8features6hitbox9draw_orbs'
RIDLEY, STEVE, MARIO = 63, 76, 0
VIEW_NAMES = {'effects': 'Effects', 'orbs': 'Orbs', 'both': 'Both'}


def save(ctx, name, value):
    (ctx.row_dir / (name + '.json')).write_text(json.dumps(value, indent=2, default=str) + '\n')
    return value


def root(ctx):
    return Path(ctx.instance['settings']).parent


def put(path, text):
    tmp = path.with_name(path.name + '.tmp')
    tmp.write_text(text)
    os.replace(tmp, path)


def diag_text(ctx):
    p = Path(ctx.instance['diag_log'])
    text = ''
    for f in (p.with_name('practice-diagnostics.previous.log'), p):
        if f.exists():
            text += f.read_text(errors='replace')
    return text


def lines_after(ctx, n0, pat):
    return [l for l in diag_text(ctx).splitlines()[n0:] if re.search(pat, l)]


def line_count(ctx):
    return len(diag_text(ctx).splitlines())


def wait_any(ctx, pat, cap):
    """A pattern anywhere in the diag log (also lines written before this row)."""
    end = time.monotonic() + cap
    while time.monotonic() < end:
        m = re.findall(pat, diag_text(ctx))
        if m:
            return m[-1]
        ctx.check_stop()
        time.sleep(0.5)
    raise RuntimeError(f'no {pat!r} within {cap} s')


def wait_new(ctx, n0, pat, cap):
    end = time.monotonic() + cap
    while time.monotonic() < end:
        found = lines_after(ctx, n0, pat)
        if found:
            return found
        ctx.check_stop()
        time.sleep(0.3)
    raise RuntimeError(f'no new {pat!r} within {cap} s')


def one_hot(i, n=87):
    return [1 if k == i else 0 for k in range(n)]


def push(ctx, player, cpu, vis):
    """A whole settings file: the boot fixture with the two fighters, the visualizer and synchronous swaps."""
    settings = json.loads((HERE / 'hb-base-settings.json').read_text())
    menu = settings['menu']
    menu['player_character'] = one_hot(player)
    menu['cpu_character'] = one_hot(cpu)
    menu['hitbox_vis'] = [1, 0] if vis else [0, 1]
    menu['trio_practice'] = [0, 1]
    menu['cycle_preload_slots'] = [1, 0, 0, 0, 0, 0]
    menu['cycle_preload_group_swap'] = [0, 1]
    menu['cycle_preload_bank_members'] = [0, 1, 0]
    n0 = line_count(ctx)
    put(root(ctx) / 'diag-settings-next.json', json.dumps(settings))
    return wait_new(ctx, n0, r'diag_settings_reload rerolls=\S+ issues=0', 20)[-1]


def route(ctx, words):
    n0 = line_count(ctx)
    put(root(ctx) / 'diag-hitbox-route.req', words + '\n')
    return wait_new(ctx, n0, r'\[Hitbox\] orbs request route=', 15)[-1]


def joint_word(ctx):
    decided = ctx.runner.out / 'joint-flag.txt'
    return 'joint=' + (decided.read_text().strip() if decided.exists() else '0')


def counters(ctx):
    got = ctx.call('trace.counters') or {}
    points = got.get('points', got) if isinstance(got, dict) else got
    out = {}
    for p in points if isinstance(points, list) else []:
        out[str(p.get('id'))] = {k: p.get(k) for k in ('target', 'hits', 'matching_hits', 'condition_hits') if k in p}
    return out or got


# ---- rows -------------------------------------------------------------------------------------------------------


def preload(ctx):
    """Boot tracing (unmapped-access log, the diag ELF's symbols, counters on the orb pass and draw), then load."""
    dev = root(ctx) / 'developer.ini'
    if dev.exists():
        dev.unlink()
    tdir = ctx.runner.out / 'trace'
    tdir.mkdir(parents=True, exist_ok=True)
    got = {'configure': ctx.call('trace.configure', {'run_directory': str(tdir), 'label': 'hb', 'unmapped_access': 'log'})}
    got['symbols'] = ctx.call('symbols.load', {'module': 'libtraining_rooms.nro', 'elf': ELF})
    got['publish'] = ctx.call('trace.arm', {'target': PUBLISH, 'mode': 'counters'})
    got['draw'] = ctx.call('trace.arm', {'target': DRAW, 'mode': 'counters'})
    ctx.call('game.load', {'path': ctx.instance['game']}, job_cap=180)
    ctx.gate('loaded')
    return save(ctx, 'preload', got)


def css_pick_and_enter(ctx):
    """The rc2d route: stage-css, one tile-cursor step onto the first row, place, Training."""
    ctx.play('stage-css')
    ctx.gate('css')
    for b in ('lstick', 'up'):
        ctx.call('input.button', {'button': b, 'state': 'press', 'lease_ms': 900})
    time.sleep(0.2)
    ctx.call('input.reset')
    line = ctx.wait_log('tile', r'settled screen=css at=\d+ hovered=\d+', cap=8)
    a, b = re.search(r'at=(\d+) hovered=(\d+)', line).groups()
    if a != b:
        raise RuntimeError('Hovered and settled native tiles differ')
    ctx.press('a', ms=150)
    time.sleep(0.35)
    ctx.gate('actual-ready')
    before = ctx.shot('before-Training')
    ctx.runner.marks[(ctx.on, 'before-Training')] = before
    ctx.press('plus', ms=150)
    time.sleep(0.35)
    ctx.gate('actual-training-current')
    ctx.runner.check_gate_spec(ctx.on, {'kind': 'change', 'mark': 'before-Training', 'roi': [1095, 160, 1240, 205],
                                        'min_diff': 2, 'stable': True, 'cap': 15}, 'Training-HUD-settled')
    ctx.shot('Training-HUD-settled')
    return {'native_hovered_tile': a}


def fighters(ctx):
    """P1 Ridley, CPU Steve by two in-fight pushes (a reset pair first, so the wanted pair differs), visualizer Off."""
    obs = [wait_any(ctx, r'\[CharacterRoster\] observed owner=%d[^\n]*' % o, 60) for o in (0, 1)]
    reset = push(ctx, MARIO, MARIO, False)
    time.sleep(3)
    n0 = line_count(ctx)
    wanted = push(ctx, RIDLEY, STEVE, False)
    cpu = wait_new(ctx, n0, r'\[CharacterRoster\] complete owner=1 target=Steve', 90)[-1]
    p1 = wait_new(ctx, n0, r'\[CharacterRoster\] HUD refresh owner=0 kind=0x42', 90)[-1]
    time.sleep(3)
    ctx.play('hb-lra')
    time.sleep(2.5)
    ctx.shot('ridley-steve-vis-off')
    return save(ctx, 'fighters', {'observed': obs, 'reset': reset, 'wanted': wanted, 'cpu': cpu, 'p1': p1})


def first_frame(ctx):
    """Route B cold (vis Off -> On, both fighters asked at once) against own orbs cold (route orbs, vis Off -> On)."""
    n0 = line_count(ctx)
    push(ctx, RIDLEY, STEVE, True)
    time.sleep(4)
    route_b = lines_after(ctx, n0, r'\[Hitbox\] (pass|refused|orbs pass)')
    ctx.shot('routeB-cold')
    push(ctx, RIDLEY, STEVE, False)
    time.sleep(2)
    route(ctx, 'route=orbs view=both ' + joint_word(ctx))
    n1 = line_count(ctx)
    push(ctx, RIDLEY, STEVE, True)
    time.sleep(4)
    orbs = lines_after(ctx, n1, r'\[Hitbox\] (pass|refused|orbs pass|orbs drawn)')
    ctx.shot('orbs-cold')
    return save(ctx, 'first-frame', {'route_b_first_lines': route_b[:12], 'orbs_first_lines': orbs[:12]})


def calibrate(ctx):
    """Both layers at once, for each value of the joint call's last argument; the ModelModule slot 0x158 target."""
    got = {}
    got['impl'] = ctx.call('trace.arm', {'target': 'main+0x2057640', 'registers': ['x0', 'x1', 'x4', 'x30'],
                                         'captures': [{'expression': '[[[x0+0x78]]+0x158]', 'type': 'u64'},
                                                      {'expression': '[[x0+0x78]]', 'type': 'u64'}],
                                         'max_hits': 3})
    for flag in ('0', '1'):
        route(ctx, f'route=both view=both joint={flag}')
        time.sleep(3)
        got[f'shot_j{flag}'] = str(ctx.shot(f'both-joint{flag}'))
    got['records'] = ctx.call('trace.read', {'after': '0', 'limit': 64})
    return save(ctx, 'calibrate', got)


def _paused_views(ctx, tag, suffix):
    shots = {}
    for view in ('effects', 'orbs', 'both'):
        n0 = line_count(ctx)
        route(ctx, f'view={view}')
        poll = wait_new(ctx, n0, r'\[Hitbox\] orbs view_poll view=%s' % VIEW_NAMES[view], 10)[-1]
        time.sleep(1.0)
        shots[view] = {'shot': str(ctx.shot(f'{tag}{suffix}-{view}')), 'poll': poll}
    return shots


def _resume(ctx):
    for _ in range(3):
        try:
            ctx.gate('training-hud-unpaused')
            return
        except RuntimeError:
            ctx.play('hb-b')
            time.sleep(0.8)
    ctx.play('hb-plus')
    time.sleep(0.8)
    ctx.gate('training-hud-unpaused')


def _last_placed(ctx):
    """The last `orbs pass` state of every set (route Both): {set: placed}."""
    last = {}
    for l in re.findall(r'\[Hitbox\] orbs pass set=(\d+) route=Both placed=(\d+)', diag_text(ctx)):
        last[int(l[0])] = int(l[1])
    return last


def _ridley(ctx):
    return json.loads((ctx.runner.out / 'ridley.json').read_text())


def _pair(ctx, tag, segment, accept):
    """Play `segment` (it pauses the game on its own frame), check `accept(Ridley's placed orbs)`, shoot the three
    views in the pause menu and in camera mode, with trace counters before and after."""
    n0 = line_count(ctx)
    ctx.play(segment)
    time.sleep(1.6)
    placed = _last_placed(ctx)
    if not accept(placed):
        _resume(ctx)
        return None
    before = counters(ctx)
    menu = _paused_views(ctx, tag, '')
    ctx.play('hb-y')
    time.sleep(1.5)
    cam = _paused_views(ctx, tag, '-cam')
    after = counters(ctx)
    lines = lines_after(ctx, n0, r'\[Hitbox\] (orbs|pass|command|refused)')
    _resume(ctx)
    return {'segment': segment, 'placed': placed, 'counters_before': before, 'counters_after': after, 'menu': menu,
            'camera': cam, 'lines': lines[-40:]}


def ab_standing(ctx):
    route(ctx, 'route=both view=both ' + joint_word(ctx))
    ctx.play('hb-lra')
    time.sleep(4)
    last = _last_placed(ctx)
    ridley_set = max(last, key=last.get)
    (ctx.runner.out / 'ridley.json').write_text(json.dumps({'set': ridley_set, 'standing': last[ridley_set], 'all': last}))
    return save(ctx, 'ab-standing', _pair(ctx, 'standing', 'hb-plus', lambda placed: True) | {'baseline': last})


def ab_attack(ctx):
    tries = []
    r = _ridley(ctx)
    for k in (18, 22, 26, 14, 30):
        ctx.play('hb-lra')
        time.sleep(2.5)
        got = _pair(ctx, f'fsmash-p{k}', f'hb-fsmash-p{k}', lambda placed: placed.get(r['set'], 0) > r['standing'])
        tries.append({'k': k, 'got': bool(got)})
        if got:
            return save(ctx, 'ab-attack', got | {'tries': tries})
    save(ctx, 'ab-attack', {'tries': tries})
    raise RuntimeError('no fsmash pause frame showed Ridley with hit spheres')


def ab_grab(ctx):
    tries = []
    r = _ridley(ctx)
    for k in (8, 9, 7, 10, 6, 12):
        ctx.play('hb-lra')
        time.sleep(2.5)
        got = _pair(ctx, f'grab-p{k}', f'hb-grab-p{k}', lambda placed: 0 < placed.get(r['set'], 0) < 20)
        tries.append({'k': k, 'got': bool(got)})
        if got:
            return save(ctx, 'ab-grab', got | {'tries': tries})
    save(ctx, 'ab-grab', {'tries': tries})
    raise RuntimeError('no grab pause frame showed a grab box alone on Ridley')


def lag_clips(ctx):
    """Both layers at once while the camera moves: P1 walks (unpaused), then the paused camera pans."""
    route(ctx, 'view=both')
    ctx.play('hb-lra')
    time.sleep(2.5)
    clips = {}
    walk = ctx.runner.out / f'{ctx.row_dir.name}-walk.gif'
    ctx.call('recording.start', {'path': str(walk), 'fps': 15, 'width': 640, 'duration_ms': 7000})
    time.sleep(0.3)
    ctx.play('hb-walk')
    clips['walk'] = _recording_done(ctx)
    ctx.play('hb-plus')
    time.sleep(1.6)
    ctx.play('hb-y')
    time.sleep(1.5)
    pan = ctx.runner.out / f'{ctx.row_dir.name}-pan.gif'
    ctx.call('recording.start', {'path': str(pan), 'fps': 15, 'width': 640, 'duration_ms': 5000})
    time.sleep(0.3)
    ctx.play('hb-cam-pan')
    clips['pan'] = _recording_done(ctx)
    _resume(ctx)
    return save(ctx, 'lag-clips', {'walk': str(walk), 'pan': str(pan), 'status': clips})


def _recording_done(ctx, cap=30):
    end = time.monotonic() + cap
    status = None
    while time.monotonic() < end:
        status = ctx.call('recording.status') or {}
        if status.get('state') in ('finished', 'idle', 'stopped', 'complete', 'completed') or status.get('error'):
            return status
        time.sleep(0.5)
    return status


def off_and_scan(ctx):
    """Visualizer Off: every orb pane hidden, route B nothing; then route B back for the release behaviour, unmapped."""
    n0 = line_count(ctx)
    push(ctx, RIDLEY, STEVE, False)
    time.sleep(2.5)
    off = lines_after(ctx, n0, r'\[Hitbox\] orbs (drawn|pass)')
    ctx.shot('vis-off')
    route(ctx, 'route=effects view=both')
    got = {'off_lines': off[-10:]}
    try:
        got['unmapped'] = ctx.call('trace.unmapped_access', {})
    except Exception as error:  # noqa: BLE001 (the method name differs on older builds)
        got['unmapped_error'] = repr(error)
    got['counters'] = counters(ctx)
    return save(ctx, 'off-and-scan', got)
