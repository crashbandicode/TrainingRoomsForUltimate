"""rc.14 SL4 Save setup here (save-here lane, 2026-11-14): Saves > Save setup here... keeps where both fighters stand
now as a named setup and leaves the bound save as it was. One row; start: Training, P1 Mario vs CPU Mario (g-training).

  S1 fixture cov-replay (bound save jab-start: P1 x -5, CPU x 5); Load puts both there.
  S2 a forward smash sends the CPU away (it takes damage) and P1 walks left; diag-pos.req reads both live spots.
  S3 Saves > Save setup here..., the keyboard, "Here spot": a new setups/ file with the live spots, damage and stage, not
     the bound save's; save_states.json and saves/jab-start.json keep their bytes.
  S4 Load: P1 placed at x -5 (Load still goes to the bound save).
  S5 Setup library, A on Here spot (first, A to Z: its own screen), A on Restore here, close: both placed at its spots.
  S6 Save States Off (live): Save setup here keeps nothing; the help line shows Save's notice (still).
  S7 Cycle CPU (Mii Gunner, after its load): Save setup here "Here mii" keeps CPU kind 0x4a with an exact Mii; Save then
     keeps the same Mii in jab-start.json; A on Here mii restores with the Mii CPU.
"""
import re
import time
from pathlib import Path

import cov
import fx
import hotslots as hs
import steps
from gna import _begin, _finish, _mark

def _open_menu(ctx):
    """steps.open_menu, checked on `[Ui] menu opened`: the first B + D-pad Up after a close can miss (gna._open_on), and
    the close that follows would then press B in the game (sh-here-f63c186b-1, 19:13Z). One more try, then stop."""
    for _ in range(2):
        seq = hs.diag_mark(ctx)
        ctx.play(steps.MENU_OPEN)
        if hs.wait_diag(ctx, seq, r'\[Ui\] menu opened', 4):
            time.sleep(1.0)
            return
    raise RuntimeError('harness: the menu did not open after two B + D-pad Up')


steps.open_menu = _open_menu                     # cov.prep, cov._page_row and accept.prep call it through steps

POS = re.compile(r'state pos entry=(\d+) kind=(-?\d+) x=(-?[\d.]+) y=(-?[\d.]+) lr=(-?[\d.]+) dmg=(-?[\d.]+)')
BOUND = {'player_x': -5.0, 'cpu_x': 5.0}


def _setups(ctx):
    folder = cov.card(ctx) / 'setups'
    return {p.name: cov.read_json(p) for p in sorted(folder.glob('*.json'))} if folder.is_dir() else {}


def _bound_bytes(ctx):
    root = cov.card(ctx)
    return {name: (root / name).read_bytes() if (root / name).is_file() else None
            for name in ('save_states.json', 'saves/jab-start.json')}


def _live(ctx):
    """Both bodies' x, y, facing and damage from one diag-pos.req dump (states.rs `state pos`)."""
    tail = cov.diag(ctx)
    (cov.card(ctx) / 'diag-pos.req').write_text('1\n')
    seen = {}
    deadline = time.time() + 8
    while len(seen) < 2 and time.time() < deadline:
        for m in POS.finditer(tail.text()):
            entry = int(m.group(1))
            seen['player' if entry == 0 else 'cpu'] = {'kind': int(m.group(2)), 'x': float(m.group(3)),
                                                       'y': float(m.group(4)), 'lr': float(m.group(5)),
                                                       'dmg': float(m.group(6))}
        time.sleep(0.2)
    if len(seen) < 2:
        raise RuntimeError('no state pos dump for both bodies: %s' % tail.text()[-400:])
    return seen


def _stage(ctx):
    log = Path(ctx.instance['diag_log'])
    text = ''.join(p.read_text(errors='replace') for p in (log.with_name(log.stem + '.previous' + log.suffix), log)
                   if p.is_file())
    stages = re.findall(r'camera_setup [^\n]*stage=(\d+)', text)
    return int(stages[-1]) if stages else None


def _save_here(ctx, out, key, name):
    """The menu on Saves > Save setup here..., A, the name typed; returns the new setups/ file."""
    before = set(_setups(ctx))
    cov.real_keyboard(ctx, True)
    try:
        cov._page_row(ctx, 'saves', 'Save setup here...')
        out[key + '_row_shot'] = str(ctx.shot(key + '-row'))
        steps.press(ctx, 'a', 1.0)
        out[key + '_keyboard'] = cov.keyboard_type(ctx, name)
        time.sleep(0.6)
        out[key + '_saved_shot'] = str(ctx.shot(key + '-saved'))
        steps.close_menu(ctx)
    finally:
        cov.real_keyboard(ctx, False)
    new = cov.wait_until(lambda: sorted(set(_setups(ctx)) - before), cap=6) or []
    out[key + '_new'] = new
    return _setups(ctx).get(new[0]) if new else None


def _restore(ctx, out, key, name):
    """Setup library, A on `name` (the first row, A to Z: its own screen), A on Restore here, the menu closed: the x
    each body is placed at."""
    order = sorted((doc or {}).get('name', '').lower() for doc in _setups(ctx).values())
    if order.index(name.lower()) != 0:
        raise RuntimeError('harness: %s is not the first setup (%s)' % (name, order))
    cov._page_row(ctx, 'saves', 'Setup library')
    steps.press(ctx, 'a', 0.8)                   # the list, the cursor on the first setup
    out[key + '_list_view'] = cov._panel_view(ctx)
    out[key + '_list_shot'] = str(ctx.shot(key + '-library'))
    steps.press(ctx, 'a', 0.8)                   # its own screen, the cursor on Restore here
    out[key + '_own_view'] = cov._panel_view(ctx)
    out[key + '_own_shot'] = str(ctx.shot(key + '-own'))
    d = cov.diag(ctx)
    steps.press(ctx, 'a', 0.8)                   # Restore here
    out[key + '_chosen_shot'] = str(ctx.shot(key + '-chosen'))
    steps.close_menu(ctx)
    try:
        out[key + '_p1'] = cov.placed(ctx, d, 0, cap=8)
        out[key + '_cpu'] = cov.placed(ctx, d, 1, cap=4)
    except RuntimeError as error:
        out[key + '_error'] = str(error)[:300]
    time.sleep(1.2)
    out[key + '_shot'] = str(ctx.shot(key + '-restored'))
    return out.get(key + '_p1'), out.get(key + '_cpu')


def _near(a, b, tol=1.0):
    return a is not None and b is not None and abs(a - b) <= tol


def setup(ctx):
    return {}


def run(ctx):
    t0, out = _begin(ctx)
    problems = []
    # S1: the fixture and its bound save, from a Training reset (damage 0, the boot rows' item gone).
    _mark(t0, out, 'S1')
    ctx.play(steps.RESET_CENTER)
    time.sleep(1.5)
    out['s1_start_shot'] = str(ctx.shot('s1-start'))
    cov.prep(ctx, 'cov-replay')
    d = cov.diag(ctx)
    ctx.play(steps.LOAD)
    out['s1_load_p1'] = cov.placed(ctx, d, 0, cap=8)
    time.sleep(1.0)
    # S2: somewhere else than the bound save.
    _mark(t0, out, 'S2')
    ctx.play(steps.FSMASH)
    time.sleep(3.0)
    ctx.stick(-1.0, 0.0, 700)
    time.sleep(1.5)
    out['s2_live'] = live = _live(ctx)
    out['s2_stage'] = stage = _stage(ctx)
    out['s2_shot'] = str(ctx.shot('s2-live-spot'))
    bound = _bound_bytes(ctx)
    # S3: Save setup here.
    _mark(t0, out, 'S3')
    here = _save_here(ctx, out, 's3', 'Here spot')
    time.sleep(1.5)                              # the state writer's 200 ms pass and the worker's tick
    out['s3_file'] = here
    out['s3_bound_unchanged'] = _bound_bytes(ctx) == bound
    if not here:
        problems.append('S3: no new setups/ file named Here spot')
    else:
        p, c = here.get('player') or {}, here.get('cpu') or {}
        if not (_near(p.get('x'), live['player']['x']) and _near(c.get('x'), live['cpu']['x'])):
            problems.append('S3: the setup is not where the fighters stood (%s, %s vs live %s, %s)'
                            % (p.get('x'), c.get('x'), live['player']['x'], live['cpu']['x']))
        if _near(p.get('x'), BOUND['player_x'], 3.0) and _near(c.get('x'), BOUND['cpu_x'], 3.0):
            problems.append('S3: the setup holds the bound save spots')
        if not _near(c.get('percent'), live['cpu']['dmg'], 0.5):
            problems.append('S3: CPU damage %s vs live %s' % (c.get('percent'), live['cpu']['dmg']))
        if stage is not None and here.get('stage_id') != stage:
            problems.append('S3: stage_id %s vs live %s' % (here.get('stage_id'), stage))
    if not out['s3_bound_unchanged']:
        problems.append('S3: Save setup here changed save_states.json or the bound save file')
    # S4: Load still goes to the bound save.
    _mark(t0, out, 'S4')
    d = cov.diag(ctx)
    ctx.play(steps.LOAD)
    out['s4_load_p1'] = cov.placed(ctx, d, 0, cap=8)
    time.sleep(1.0)
    out['s4_shot'] = str(ctx.shot('s4-load-bound'))
    if not _near(out['s4_load_p1'], BOUND['player_x'], 0.5):
        problems.append('S4: Load placed P1 at %s, not the bound save' % out['s4_load_p1'])
    # S5: A on the new setup moves both there.
    _mark(t0, out, 'S5')
    if here:
        p1, cpu = _restore(ctx, out, 's5', 'Here spot')
        if not (_near(p1, here['player']['x']) and _near(cpu, here['cpu']['x'])):
            problems.append('S5: Restore here did not move both to the setup: %s' % {k: out.get(k) for k in ('s5_p1', 's5_cpu', 's5_error')})
    # S6: Save States Off: refused, nothing kept.
    _mark(t0, out, 'S6')
    fx._live_set(ctx, save_state_enable=[0, 1])
    before = set(_setups(ctx))
    cov._page_row(ctx, 'saves', 'Save setup here...')
    steps.press(ctx, 'a', 1.2)
    out['s6_refused_shot'] = str(ctx.shot('s6-off-refused'))
    steps.close_menu(ctx)
    time.sleep(1.5)
    out['s6_new'] = sorted(set(_setups(ctx)) - before)
    fx._live_set(ctx, save_state_enable=[1, 0])
    if out['s6_new']:
        problems.append('S6: Save States Off still kept %s' % out['s6_new'])
    # S7: a Mii CPU keeps its exact Mii, as Save keeps it.
    _mark(t0, out, 'S7')
    seq = hs.diag_mark(ctx)
    ctx.play('p2-fs-cpu-cycle')
    hud = hs.wait_diag(ctx, seq, r'\[CharacterRoster\] HUD refresh owner=1 kind=0x4a(?![0-9a-f])', 90)
    out['s7_hud'] = hud[2][:200] if hud else None
    time.sleep(2.0)
    if not hud:
        problems.append('S7: the CPU never became a Mii Gunner (fixture cycle)')
    else:
        mii = _save_here(ctx, out, 's7', 'Here mii')
        out['s7_file_cpu'] = (mii or {}).get('cpu')
        cpu = (mii or {}).get('cpu') or {}
        if cpu.get('fighter_kind') != 0x4a or not cpu.get('mii'):
            problems.append('S7: the setup has no Mii Gunner with an exact Mii: %s' % {k: cpu.get(k) for k in ('fighter_kind', 'mii')})
        ctx.play(steps.SAVE)
        time.sleep(2.0)
        saved = cov.read_json(cov.card(ctx) / 'saves' / 'jab-start.json') or {}
        out['s7_save_cpu_mii'] = (saved.get('cpu') or {}).get('mii')
        if cpu.get('mii') != out['s7_save_cpu_mii']:
            problems.append('S7: the setup Mii differs from the one Save keeps')
        if mii:
            p1, cpu_x = _restore(ctx, out, 's7', 'Here mii')
            if not _near(cpu_x, cpu.get('x')):
                problems.append('S7: Restore here did not restore the Mii setup: %s' % {k: out.get(k) for k in ('s7_p1', 's7_cpu', 's7_error')})
    out['problems'] = problems
    return _finish(ctx, 'setuphere', out, problems)


def restore(ctx):
    fx._live_set(ctx, save_state_enable=[1, 0])
    return {}
