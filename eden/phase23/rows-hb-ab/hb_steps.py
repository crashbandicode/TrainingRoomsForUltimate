"""Hitbox own-orbs A/B (HB1 route A4 prototype, run ab2: diag NRO of p23/hitbox-orbs b771f08).

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
ELF = '/home/intpa/smash-training-lobby/artifacts/phase23/hitbox-orbs/nro-ab3/diag/libtraining_rooms.so'
PUBLISH = 'libtraining_rooms.nro!_RNvNtNtNtCskE0RJeV9NF0_14training_rooms7runtime8features6hitbox12publish_orbs'
DRAW = 'libtraining_rooms.nro!_RNvNtNtNtCskE0RJeV9NF0_14training_rooms7runtime8features6hitbox9draw_orbs'
RIDLEY, STEVE, MARIO = 63, 76, 0
VIEW_NAMES = {'effects': 'Effects', 'orbs': 'Orbs', 'both': 'Both'}


def save(ctx, name, value):
    (ctx.row_dir / (name + '.json')).write_text(json.dumps(value, indent=2, default=str) + '\n')
    # The diag log keeps 2 x 128 KiB: a copy per row keeps the evidence.
    (ctx.row_dir / 'diag-copy.log').write_text(diag_text(ctx))
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


SEQ = re.compile(r'^\d+ #(\d+) ')


def _seq(line):
    m = SEQ.match(line)
    return int(m.group(1)) if m else -1


def lines_after(ctx, n0, pat):
    """Lines with a sequence number above 
0 (#N of the diag line; survives the log's rotation)."""
    return [l for l in diag_text(ctx).splitlines() if _seq(l) > n0 and re.search(pat, l)]


def line_count(ctx):
    """The newest diag line's sequence number."""
    return max((_seq(l) for l in diag_text(ctx).splitlines()), default=0)


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
    menu['trio_practice'] = [1, 0]  # Player Swapping On: the Player role's swaps need it
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
    got = {'configure': ctx.call('trace.configure', {'run_directory': str(tdir), 'label': 'hb'})}
    try:
        got['unmapped_mode'] = ctx.call('trace.unmapped_access', {'mode': 'log'})
    except RuntimeError as error:
        got['unmapped_mode_error'] = repr(error)
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
    # Both reset swaps must finish first: a push during a swap is rejected ("Wait for the character swap to finish").
    wait_any(ctx, r'\[CharacterRoster\] HUD refresh owner=0 kind=0x0 ', 60)
    wait_any(ctx, r'\[CharacterRoster\] complete owner=1 target=Mario ', 60)
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
    # Frame identity: the first view again, after the flips. Pixel-identical to the first shot means no game frame and
    # no camera move happened in between (the plugin's per-fighter pass still runs in the pause; the bones do not move).
    n0 = line_count(ctx)
    route(ctx, 'view=effects')
    wait_new(ctx, n0, r'\[Hitbox\] orbs view_poll view=Effects', 10)
    time.sleep(1.0)
    shots['effects_again'] = {'shot': str(ctx.shot(f'{tag}{suffix}-effects-again'))}
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


STATE = re.compile(r'\[Hitbox\] orbs pass set=(\d+) route=Both (?:asked=\d+ )?placed=(\d+) held=\d+ failed=\d+ hits=(\d+) grabs=(\d+) '
                   r'status=(-?\d+) pass=(\d+)')


def _last_state(ctx):
    """The last `orbs pass` state of every set (route Both): {set: {placed, hits, grabs, status, pass}}."""
    last = {}
    for m in STATE.finditer(diag_text(ctx)):
        last[int(m.group(1))] = {'placed': int(m.group(2)), 'hits': int(m.group(3)), 'grabs': int(m.group(4)),
                                 'status': int(m.group(5)), 'pass': int(m.group(6))}
    return last


def _ridley(ctx):
    return json.loads((ctx.runner.out / 'ridley.json').read_text())


def _pair(ctx, tag, segment, accept):
    """Play `segment` (it pauses the game on its own frame), check `accept(Ridley's last pass state)`, shoot the views in
    the pause menu and in camera mode (each ends with a repeat of the first view: the frame-identity check), with trace
    counters before and after."""
    r = _ridley(ctx)
    n0 = line_count(ctx)
    ctx.play(segment)
    time.sleep(1.6)
    state = _last_state(ctx)
    ridley = state.get(r['set'], {})
    if not accept(ridley):
        _resume(ctx)
        return None, ridley
    before = counters(ctx)
    menu = _paused_views(ctx, tag, '')
    ctx.play('hb-y')
    time.sleep(4.0)  # the camera toggle's "Zoom" hint fades first
    cam = _paused_views(ctx, tag, '-cam')
    after = counters(ctx)
    lines = lines_after(ctx, n0, r'\[Hitbox\] (orbs|pass|command|refused)')
    _resume(ctx)
    return {'segment': segment, 'paused_state': state, 'counters_before': before, 'counters_after': after, 'menu': menu,
            'camera': cam, 'lines': lines[-60:]}, ridley


def ab_standing(ctx):
    """Visualizer On (route Both, both layers), positions reset, then one pause: hurtboxes only."""
    route(ctx, 'route=both view=both ' + joint_word(ctx))
    push(ctx, RIDLEY, STEVE, True)
    ctx.play('hb-lra')
    time.sleep(4)
    last = _last_state(ctx)
    ridley_set = max(last, key=lambda k: last[k]['placed'])
    (ctx.runner.out / 'ridley.json').write_text(json.dumps({'set': ridley_set, 'standing': last[ridley_set], 'all': last}))
    got, state = _pair(ctx, 'standing', 'hb-plus', lambda st: st.get('hits') == 0 and st.get('grabs') == 0)
    if not got:
        raise RuntimeError(f'standing pause showed {state}')
    return save(ctx, 'ab-standing', got | {'baseline': last})


def _sweep(ctx, name, ks, seg, accept, what):
    tries = []
    for k in ks:
        ctx.play('hb-lra')
        time.sleep(2.5)
        got, state = _pair(ctx, f'{name}-p{k}', f'{seg}{k}', accept)
        tries.append({'k': k, 'got': bool(got), 'ridley_at_pause': state})
        if got:
            return save(ctx, name, got | {'tries': tries})
    save(ctx, name, {'tries': tries})
    raise RuntimeError(f'no {name} pause frame showed {what}; tries {tries}')


def ab_attack(ctx):
    """Ridley's right-stick forward smash connects on Steve from the reset spots: the hit spheres stay through the hitlag
    freeze, so a pause K frames on lands on them for a wide band of K. Accepted only when Ridley's pass held hit spheres."""
    ks = (24, 28, 32, 20, 36, 26, 30, 22, 34, 38, 18, 40, 16, 42, 14, 44, 12, 10, 8)
    return _sweep(ctx, 'ab-attack', ks, 'hb-csmash-p', lambda st: st.get('hits', 0) > 0, 'hit spheres on Ridley')


def ab_grab(ctx):
    """A standing grab (shield + attack): accepted only when Ridley's pass held a grab sphere."""
    ks = (8, 9, 10, 7, 11, 6, 12, 13, 5, 14, 15, 4, 16, 17, 3, 18)
    return _sweep(ctx, 'ab-grab', ks, 'hb-grab-p', lambda st: st.get('grabs', 0) > 0, 'a grab sphere on Ridley')


def lag_clips(ctx):
    """Both layers on the same frames while the camera moves, the orbs drawn cyan so the layers part: P1 walks right and
    back (the camera follows, unpaused; GIF plus a burst of shots), then the paused camera pans (GIF). The lag is the
    offset between the two layers' centroids over the layers' own motion per frame."""
    route(ctx, 'view=both orbtint=cyan')
    ctx.play('hb-lra')
    time.sleep(2.5)
    clips = {}
    walk = ctx.runner.out / f'{ctx.row_dir.name}-walk.gif'
    ctx.call('recording.start', {'path': str(walk), 'fps': 30, 'width': 1280, 'duration_ms': 4500})
    time.sleep(0.3)
    ctx.play('hb-walk')
    clips['walk'] = _recording_done(ctx)
    ctx.play('hb-lra')
    time.sleep(2.5)
    # The still reference: both layers with nothing moving (the centroid offset the lag is measured against).
    clips['still_shots'] = [str(ctx.shot(f'still-{i}')) for i in range(2)]
    # No paused camera pan: the Training menu does not stop the scene here and its stick moves menu values (12:17Z).
    route(ctx, 'orbtint=off')
    return save(ctx, 'lag-clips', {'walk': str(walk), 'status': clips})


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



def probe_pause(ctx):
    """Is the scene frozen in the Training pause? Shots at 1.5 s steps with no change (view orbs: the orbs do not
    animate), after a standing pause and after a csmash pause; the pass state and pause lines around them."""
    got = {}
    for tag, seg in (('stand', 'hb-plus'), ('csmash20', 'hb-csmash-p20')):
        route(ctx, 'view=orbs')
        ctx.play('hb-lra')
        time.sleep(2.5)
        n0 = line_count(ctx)
        ctx.play(seg)
        time.sleep(1.0)
        shots = []
        for i in range(3):
            shots.append(str(ctx.shot(f'probe-{tag}-{i}')))
            time.sleep(1.5)
        got[tag] = {'shots': shots, 'lines': lines_after(ctx, n0, r'\[(Hitbox\] orbs pass|PauseHelper|TrainingRooms\] pause_owner|TrainingRooms\] cpu_event status)')}
        _resume(ctx)
    route(ctx, 'view=both')
    return save(ctx, 'probe', got)


# ---- ab2 frozen-frame method (12:10Z): the Training menu does not stop the fighters (row 07b-probe), so the A/B is shot
# with Eden's speed limit at 1 % (one game frame lasts 1.7 s) and, for the hitbox, inside the fsmash hitlag freeze. The
# trace counter on publish_orbs (two passes per game frame) counts the game frames between the shots.

def speed(ctx, pct):
    return ctx.call('settings.set', {'key': 'core.speed_limit', 'value': str(pct)})


def _passes(ctx):
    for v in (counters(ctx) or {}).values():
        if isinstance(v, dict) and 'publish_orbs' in str(v.get('target', '')):
            return int(v.get('hits') or 0)
    return None


def _slow_views(ctx, tag, settle=2.4):
    """Shots of the effects, orbs and both layers and the effects layer again, each after the flip's view_poll line and
    one slowed frame; the publish_orbs pass count and the diag line count at each shot."""
    shots = {}
    for key, view in (('effects', 'effects'), ('orbs', 'orbs'), ('effects_again', 'effects'), ('both', 'both')):
        n0 = line_count(ctx)
        route(ctx, f'view={view}')
        poll = wait_new(ctx, n0, r'\[Hitbox\] orbs view_poll view=%s' % VIEW_NAMES[view], 20)[-1]
        time.sleep(settle)
        before = _passes(ctx)
        path = str(ctx.shot(f'{tag}-{key}'))
        shots[key] = {'shot': path, 'poll': poll, 'passes_before_shot': before, 'passes_after_shot': _passes(ctx),
                      'utc': time.strftime('%H:%M:%SZ', time.gmtime())}
    return shots


def _ridley_lines(ctx, n0, r):
    return lines_after(ctx, n0, r'\[Hitbox\] orbs pass set=%d ' % r['set'])


def slow_standing(ctx):
    """Hurtboxes, both fighters idle, at 1 % speed."""
    r = _ridley(ctx)
    route(ctx, 'route=both view=both orbtint=off')
    ctx.play('hb-lra')
    time.sleep(4)
    try:
        speed(ctx, 1)
        time.sleep(2.0)
        n0 = line_count(ctx)
        shots = _slow_views(ctx, 'standing')
        changes = _ridley_lines(ctx, n0, r)
    finally:
        speed(ctx, 100)
    route(ctx, 'view=both')
    return save(ctx, 'slow-standing', {'shots': shots, 'ridley_state': _last_state(ctx).get(r['set']),
                                       'ridley_changes_during_shots': changes})


def _slow_event(ctx, name, segment, pattern, what, strict, delay=1.0):
    r = _ridley(ctx)
    route(ctx, 'route=both view=effects orbtint=off')
    ctx.play('hb-lra')
    time.sleep(3)
    got = {'segment': segment}
    try:
        speed(ctx, 10)
        n0 = line_count(ctx)
        ctx.play(segment)
        found = wait_new(ctx, n0, r'\[Hitbox\] orbs pass set=%d [^\n]*%s' % (r['set'], pattern), 40)
        speed(ctx, 1)
        got['trigger_line'] = found[-1]
        # The game's own hit flash covers the hit spheres in the first hitlag frames: shoot from ~4 slowed frames on.
        time.sleep(delay)
        n1 = line_count(ctx)
        got['shots'] = _slow_views(ctx, name)
        got['ridley_changes_during_shots'] = _ridley_lines(ctx, n1, r)
        got['all_lines'] = lines_after(ctx, n0, r'\[Hitbox\] (orbs pass|orbs view_poll)')
    finally:
        speed(ctx, 100)
    time.sleep(3)
    route(ctx, 'view=both')
    if strict and got['ridley_changes_during_shots']:
        save(ctx, name, got)
        raise RuntimeError(f'{what}: Ridley changed during the shots: {got["ridley_changes_during_shots"]}')
    return save(ctx, name, got)


def slow_attack(ctx):
    """Ridley's right-stick fsmash at 10 % speed; at its first pass with hit spheres the speed drops to 1 %; the hit lands on
    Steve, so the shots fall in the hitlag freeze (Ridley's bones still: no orbs-pass change during the shots)."""
    return _slow_event(ctx, 'slow-attack', 'hb-csmash', r'hits=[1-9]', 'attack', True, delay=7.0)


def slow_grab(ctx):
    """A standing grab at 10 % speed; at its first pass with a grab sphere the speed drops to 1 %."""
    return _slow_event(ctx, 'slow-grab', 'hb-sideb', r'grabs=[1-9]', 'grab', False)


# ---- ab3: Training's own frame-by-frame speed (operator tip, primary 12:12Z) -------------------------------------
# Speed "1 Frame (Press ZL)" (SmashWiki, Training Mode): the battle stays frozen; a ZL tap advances one frame. Both views
# are shot with no tap in between, so they show the same game frame; the pass counter is recorded with every shot.

def menu_route(ctx, seq=(), tag='route', settle=0.5):
    """Play the named segments in order (Training menu navigation), then one shot."""
    for name in seq:
        ctx.play(name)
        time.sleep(settle)
    time.sleep(1.0)
    return save(ctx, tag, {'seq': list(seq), 'shot': str(ctx.shot(tag))})


def _note(ctx, tag):
    r = _ridley(ctx)
    return {'shot': str(ctx.shot(tag)), 'passes': _passes(ctx), 'seq': line_count(ctx),
            'ridley': _last_state(ctx).get(r['set']), 'utc': time.strftime('%H:%M:%SZ', time.gmtime())}


def fbf_check(ctx):
    """Frozen? Two shots 2 s apart with no tap, then one ZL tap and a shot: the pass counter must stay, then move."""
    a = _note(ctx, 'fbf-still-0')
    time.sleep(2.0)
    b = _note(ctx, 'fbf-still-1')
    ctx.play('hb-zl')
    time.sleep(1.0)
    c = _note(ctx, 'fbf-after-tap')
    got = {'still0': a, 'still1': b, 'after_tap': c}
    save(ctx, 'fbf-check', got)
    if a['passes'] != b['passes'] or c['passes'] == b['passes']:
        raise RuntimeError(f'not frozen / no step: passes {a["passes"]} {b["passes"]} {c["passes"]}')
    return got


def _fbf_views(ctx, tag):
    shots = {}
    for key, view in (('effects', 'effects'), ('orbs', 'orbs'), ('effects_again', 'effects'), ('both', 'both')):
        n0 = line_count(ctx)
        route(ctx, f'view={view}')
        poll = wait_new(ctx, n0, r'\[Hitbox\] orbs view_poll view=%s' % VIEW_NAMES[view], 20)[-1]
        time.sleep(1.0)
        shots[key] = _note(ctx, f'{tag}-{key}') | {'poll': poll}
    return shots


def _fbf_set(ctx, tag, r):
    """Every view on the current (frozen) frame, then the pane probe and a cyan-orbs shot of the same frame."""
    n1 = line_count(ctx)
    shots = _fbf_views(ctx, tag)
    n2 = line_count(ctx)
    route(ctx, 'view=orbs orbtint=cyan orbprobe=1')
    probe = wait_new(ctx, n2, r'\[Hitbox\] orbs drawn ', 20)[-1]
    time.sleep(1.0)
    shots['orbs_cyan'] = _note(ctx, f'{tag}-orbs_cyan') | {'probe': probe}
    route(ctx, 'orbtint=off view=effects')
    changes = lines_after(ctx, n1, r'\[Hitbox\] orbs pass set=%d ' % r['set'])
    passes = {k: v['passes'] for k, v in shots.items()}
    return {'shots': shots, 'passes': passes, 'same_frame': len(set(passes.values())) == 1 and not changes,
            'ridley_changes_during_shots': changes,
            'drawn_lines': lines_after(ctx, n1, r'\[Hitbox\] orbs drawn ')}


def fbf_pair(ctx, start='hb-zl-csmash', pattern='hits=[1-9]', tag='fbf-attack', taps=60, then=()):
    """Reset (ZL held = normal speed), start the move on one stepped frame, tap ZL one frame at a time until Ridley's
    pass has the wanted spheres, then shoot every view with no tap in between; `then` = more frames to step and shoot."""
    r = _ridley(ctx)
    route(ctx, 'route=both view=effects orbtint=off')
    ctx.play('hb-lra')
    ctx.play('hb-zl-hold')
    time.sleep(0.5)
    n0 = line_count(ctx)
    ctx.play(start)
    steps = 1
    found = None
    while steps <= taps:
        time.sleep(0.4)
        found = lines_after(ctx, n0, r'\[Hitbox\] orbs pass set=%d [^\n]*%s' % (r['set'], pattern))
        if found:
            break
        ctx.play('hb-zl')
        steps += 1
    if not found:
        raise RuntimeError(f'{tag}: no {pattern} after {steps} stepped frames')
    time.sleep(1.0)
    got = {'start': start, 'stepped_frames': steps, 'trigger_line': found[-1], 'sets': {}}
    got['sets']['0'] = _fbf_set(ctx, f'{tag}-f0', r)
    total = 0
    for k in then:
        for _ in range(k):
            ctx.play('hb-zl')
            time.sleep(0.4)
        total += k
        time.sleep(1.0)
        got['sets'][str(total)] = _fbf_set(ctx, f'{tag}-f{total}', r)
    got['pass_lines'] = lines_after(ctx, n0, r'\[Hitbox\] (orbs pass set=|pass held)')
    save(ctx, tag, got)
    if not got['sets']['0']['same_frame']:
        raise RuntimeError(f'{tag}: the frame moved between shots: {got["sets"]["0"]["passes"]}')
    return got


def prepare(ctx):
    """Visualizer On, route Both, positions reset; which set is Ridley (the larger), for the frame picks."""
    route(ctx, 'route=both view=both orbtint=off ' + joint_word(ctx))
    push(ctx, RIDLEY, STEVE, True)
    ctx.play('hb-lra')
    time.sleep(4)
    last = _last_state(ctx)
    ridley_set = max(last, key=lambda k: last[k]['placed'])
    (ctx.runner.out / 'ridley.json').write_text(json.dumps({'set': ridley_set, 'standing': last[ridley_set], 'all': last}))
    return save(ctx, 'prepare', {'ridley_set': ridley_set, 'last': last, 'shot': str(ctx.shot('prepared'))})


def speed_scan(ctx, n=10):
    """Right on the focused Speed row n times; a shot after each press and one montage of the value and the help line."""
    from PIL import Image
    shots = []
    for i in range(n):
        ctx.play('hb-right')
        time.sleep(0.6)
        shots.append(str(ctx.shot(f'speed-right{i + 1}')))
    tiles = [Image.open(p).convert('RGB') for p in shots]
    out = Image.new('RGB', (1200, 60 * n), 'black')
    for i, im in enumerate(tiles):
        out.paste(im.crop((300, 130, 600, 185)), (0, 60 * i))
        out.paste(im.crop((100, 600, 1000, 645)), (300, 60 * i))
    montage = ctx.row_dir / 'speed-montage.png'
    out.save(montage)
    return save(ctx, 'speed-scan', {'shots': shots, 'montage': str(montage)})


def flip_probe(ctx, tint='off', tag='flip', n=3):
    """ab2's sequence (view effects, side special at 10 % then 1 %, flip to orbs), with the orbs view shot three times,
    one slowed frame apart, and the pass counter at each: does the orb layer appear a frame later?"""
    r = _ridley(ctx)
    route(ctx, f'route=both view=effects orbtint={tint}')
    ctx.play('hb-lra')
    time.sleep(3)
    got = {'tint': tint, 'shots': []}
    try:
        speed(ctx, 10)
        n0 = line_count(ctx)
        ctx.play('hb-sideb')
        got['trigger'] = wait_new(ctx, n0, r'\[Hitbox\] orbs pass set=%d [^\n]*grabs=[1-9]' % r['set'], 40)[-1]
        speed(ctx, 1)
        time.sleep(1.0)
        got['shots'].append({'view': 'effects', 'shot': str(ctx.shot(f'{tag}-effects')), 'passes': _passes(ctx)})
        n1 = line_count(ctx)
        route(ctx, 'view=orbs')
        got['poll'] = wait_new(ctx, n1, r'\[Hitbox\] orbs view_poll view=Orbs', 20)[-1]
        for i in range(n):
            time.sleep(1.7)
            st = ctx.call('status') or {}
            got['shots'].append({'view': 'orbs', 'shot': str(ctx.shot(f'{tag}-orbs-{i}')), 'passes': _passes(ctx),
                                 'eden': {k: st.get(k) for k in ('game_fps', 'last_frame_age_ms', 'shaders_building', 'stats_age_ms')}})
        got['lines'] = lines_after(ctx, n0, r'\[Hitbox\] (orbs pass set=|pass held|orbs drawn|orbs view_poll)')
    finally:
        speed(ctx, 100)
    time.sleep(3)
    route(ctx, 'orbtint=off view=both')
    return save(ctx, f'{tag}-probe', got)


def sideb_probe(ctx, frames=10, tint='cyan', view='orbs', tag='sideb'):
    """ab2's missing hurt discs: Ridley's side special at 10 % speed, then 1 %; from its first pass with a grab sphere, per
    slowed frame: the orbs view tinted cyan, `orbprobe=1` (where the hurt-tinted panes went) and a shot."""
    r = _ridley(ctx)
    route(ctx, f'route=both view={view} orbtint={tint}')
    ctx.play('hb-lra')
    time.sleep(3)
    got = {'frames': [], 'tint': tint, 'view': view}
    try:
        speed(ctx, 10)
        n0 = line_count(ctx)
        ctx.play('hb-sideb')
        got['trigger'] = wait_new(ctx, n0, r'\[Hitbox\] orbs pass set=%d [^\n]*grabs=[1-9]' % r['set'], 40)[-1]
        speed(ctx, 1)
        for i in range(frames):
            n1 = line_count(ctx)
            route(ctx, 'orbprobe=1')
            probe = wait_new(ctx, n1, r'\[Hitbox\] orbs drawn ', 20)[-1]
            got['frames'].append({'probe': probe, 'shot': str(ctx.shot(f'{tag}-{i}')), 'passes': _passes(ctx)})
            time.sleep(0.8)
        got['lines'] = lines_after(ctx, n0, r'\[Hitbox\] (orbs pass set=|pass held)')
    finally:
        speed(ctx, 100)
    time.sleep(3)
    route(ctx, 'orbtint=off view=both')
    return save(ctx, f'{tag}-probe', got)
