"""rc.14 cycle fix (Design A) rows, 2026-11-14: the operator's rc.13 report "seems like it still loads the fighters for
cpu again after cycling through. going to k-rool took a couple seconds and it unloaded everything", and the rule
"what doesn't fit stays cold and what dos stays hot so it's not rebuilding constantly" (~20:44Z), on a diag build.

The lanes have the console session's shape (artifacts/rc14-cycle-load/console-20261114): Player 1 original > Pikachu >
Kirby > Palutena (3 extras), the CPU original > Mii Gunner > Pichu > Diddy Kong > King K. Rool (4 extras), Ready
Fighters 8, every Loading row at the card's value. The boot's CSS picks the originals (P1 Mario; the CPU's coin is
Random, its kind is read from the log). Kinds: gna_p2_fs.py's constants (TR trio.rs).

Rows (hooks; evidence is the diag log, the receipts and stills):
- cf_setup: the live values, one P1 lap and one CPU lap (both banks Ready), then the notice settled.
- cf_rf8_laps: five CPU laps, each press after the background settled (20 s cap), then two fast laps (1.5 s apart).
  Pass: no Reload or canonical replacement after the first lap; each lap's cold presses are the same fighters;
  every other press is warm; no teardown.
- cf_rf9: Ready Fighters 9 (the lap fits), settled, one lap: all warm. The headroom with King K. Rool on screen is the
  memory comparison with Ready Fighters 8.
- cf_restore: the remembered values, Mario / Mario.
Each press records its route, request-to-ready ms, its diag lines (demand, cold release, prefetch headroom/need,
ready headroom, any Reload or teardown)."""
import re
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import hotslots as hs  # noqa: E402
import gna_p2_fs as fs  # noqa: E402
import steps  # noqa: E402
import fx  # noqa: E402
from gna import _begin, _mark, _shot, _finish, _reset, MARIO  # noqa: E402

P1_LANE = [fs.PIKACHU, fs.KIRBY, fs.PALUTENA]
CPU_LANE = [fs.GUNNER, fs.PICHU, fs.DIDDY, fs.KROOL]

PATTERNS = {
    'demand': re.compile(r'\[GroupSwap\] demand owner=1 '),
    'cold_release': re.compile(r'\[GroupSwap\] cold release owner=1 '),
    'target_first': re.compile(r'\[BodyBank\] target first owner=1 '),
    'reload': re.compile(r'\[TrioBoundary\] reload entry=\S+ owner=1 '),
    'teardown': re.compile(r'\[GroupSwap\] teardown owner=1 '),
    'canonical': re.compile(r'\[BodyBank\] canonical replacement owner=1|\[MiiOrdinaryCpu\] staged .*bank_build=false'),
    'prefetch': re.compile(r'\[GroupSwap\] (prefetch|prefetched) owner=1 '),
    'built': re.compile(r'\[GroupSwap\] (background built|built) owner=1 '),
    'ready': re.compile(r'\[BodyBank\] ready owner=1 '),
    'left': re.compile(r'\[GroupSwap\] (left cycle|frees now) owner=1 '),
    'stall': re.compile(r'StallWatch'),
    'declined': re.compile(r'\[GroupSwap\] demand declined owner=1 '),
    'published': re.compile(r'\[MiiOrdinaryCpu\] Mii group record published owner=1 |\[GroupSwap\] published owner=1 '),
    'refused': re.compile(r'\[MiiOrdinaryCpu\] request refused'),
}
HEADROOM = re.compile(r'headroom=(\d+)')
NEED = re.compile(r'need=(\d+)')


def _values(rf):
    return fs._preload_base(player_roster=fs._roster(P1_LANE), cpu_roster=fs._roster(CPU_LANE),
                            cycle_preload_ready_fighters=fs.RF(rf))


def _press(ctx, gap):
    p = fs._press_cycle(ctx, 1)
    time.sleep(gap)
    lines = [t for _, _, t in hs.diag_since(ctx, p['seq'])]
    rec = fs._brief(p)
    for key, regex in PATTERNS.items():
        hits = [t[:300] for t in lines if regex.search(t)]
        if hits:
            rec[key] = hits[:8]
    if rec.get('canonical'):
        rec['route'] = 'canonical'
    ready = [int(m.group(1)) for t in rec.get('ready', []) for m in [HEADROOM.search(t)] if m]
    rec['ready_headroom'] = ready[-1] if ready else None
    pre = [(int(h.group(1)), int(n.group(1))) for t in rec.get('prefetch', []) if ' prefetch owner' in t
           for h in [HEADROOM.search(t)] for n in [NEED.search(t)] if h and n]
    rec['prefetch_headroom_need'] = pre[0] if pre else None
    return rec


def _lap(ctx, out, tag, laps, gap, settle):
    presses = []
    for n in range(laps * (len(CPU_LANE) + 1)):
        ctx.check_stop()
        if settle:
            fs._settled(ctx, hs.diag_mark(ctx), settle)
        rec = _press(ctx, gap)
        rec['i'] = n
        presses.append(rec)
        if len(presses) >= 3 and all(p.get('outcome') != 'Applied' or not p.get('kind') for p in presses[-3:]):
            out[tag + '_stuck'] = 'three presses in a row not applied (cf-p2-x2: a press waited for good)'
            break
        if rec.get('kind') == '0x45' and tag + '_krool_shot' not in out:
            time.sleep(0.8)
            out[tag + '_krool_shot'] = str(ctx.shot('%s-krool' % tag))
    out[tag] = presses
    return presses


def _judge(presses, lap_len, skip_laps=1):
    """Per lap after `skip_laps`: the cold kinds (a demand or a group build), and the problems: a Reload, a canonical
    replacement, a teardown, or cold kinds that change between laps. (`proposed` prints its `target first` line on every
    plan, a waiting press's too, so that line alone is no Reload: cf-p2-x2.)"""
    problems, cold = [], []
    for start in range(skip_laps * lap_len, len(presses), lap_len):
        lap = presses[start:start + lap_len]
        cold.append([p.get('kind') for p in lap if p.get('demand') or p.get('route') == 'group-build'])
        for p in lap:
            if p.get('outcome') != 'Applied' or not p.get('kind'):
                problems.append('press %d: not applied (%s, kind %s)' % (p['i'], p.get('outcome'), p.get('kind')))
            for key in ('reload', 'teardown', 'canonical', 'stall', 'declined', 'refused'):
                if p.get(key):
                    problems.append('press %d (%s): %s %s' % (p['i'], p.get('kind'), key, p[key][0][:120]))
    if any(c != cold[0] for c in cold):
        problems.append('the cold fighters change between laps: %s' % cold)
    return cold, problems


def cf_setup(ctx):
    fs._remember_original(ctx)
    t0, out = _begin(ctx)
    out['live'] = fs._base(ctx)
    seq = hs.diag_mark(ctx)
    out['picks'] = fs._picks(ctx, MARIO, MARIO, cap=120, **_values(8))
    _reset(ctx, 1.5)
    out['quiet'] = fs._wait_quiet(ctx, cap=20)
    out['p1_lap'] = [fs._brief(fs._press_cycle(ctx, 0)) for _ in range(4)]
    out['cpu_lap'] = [_press(ctx, 1.5) for _ in range(len(CPU_LANE) + 1)]
    out['settle'] = fs._settled(ctx, seq, 150)
    observed = [m.group(2) for _, _, t in hs.diag_lines(ctx) for m in [fs.OBSERVED.search(t)] if m and m.group(1) == '1']
    out['cpu_original'] = '0x' + observed[0] if observed else None
    _shot(ctx, out, 'shot_settled', 'cf-setup-settled')
    _mark(t0, out, 'done')
    applied = [p for p in out['cpu_lap'] if p.get('outcome')]
    return _finish(ctx, 'cf-setup', out, [] if len(applied) == len(CPU_LANE) + 1 else ['CPU lap: %s' % out['cpu_lap']])


def cf_rf8_laps(ctx):
    t0, out = _begin(ctx)
    lap_len = len(CPU_LANE) + 1
    patient = _lap(ctx, out, 'rf8_patient', 5, 1.5, 20)
    out['rf8_patient_cold'], problems = _judge(patient, lap_len)
    fast = _lap(ctx, out, 'rf8_fast', 2, 1.5, None)
    out['rf8_fast_cold'], fast_problems = _judge(fast, lap_len, skip_laps=0)
    problems += ['fast: ' + p for p in fast_problems]
    _mark(t0, out, 'done')
    return _finish(ctx, 'cf-rf8-laps', out, problems)


def cf_rf9(ctx):
    t0, out = _begin(ctx)
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, cycle_preload_ready_fighters=fs.RF(9))
    out['settle'] = fs._settled(ctx, seq, 150)
    lap_len = len(CPU_LANE) + 1
    # One lap to let the planner build the member that now fits, then the measured lap.
    _lap(ctx, out, 'rf9_warmup', 1, 1.5, 20)
    presses = _lap(ctx, out, 'rf9', 1, 1.5, 20)
    cold = [p.get('kind') for p in presses if p.get('demand') or p.get('route') == 'group-build']
    out['rf9_cold'] = cold
    _mark(t0, out, 'done')
    problems = [] if not cold else ['Ready Fighters 9 still loads %s' % cold]
    problems += ['press %d: %s' % (p['i'], k) for p in presses for k in ('reload', 'teardown', 'canonical') if p.get(k)]
    return _finish(ctx, 'cf-rf9', out, problems)


def cf_restore(ctx):
    return fs._fs_restore(ctx)


# --- k = 1 with the CPU short, deterministically (cf-p1-y1: at Ready Fighters 8 the Eden planner gave the CPU all five
# and left Player 1 one short, so no CPU press was cold). Player 1 cycles Mario > Pikachu only (one extra, depth 0),
# the CPU's lap is unchanged (4 extras): Ready Fighters 6 leaves 4 bodies, the CPU's deepest member doesn't fit.

def _k1_values(rf):
    return fs._preload_base(player_roster=fs._roster([fs.PIKACHU]), cpu_roster=fs._roster(CPU_LANE),
                            cycle_preload_ready_fighters=fs.RF(rf))


def cf_k1_setup(ctx):
    t0, out = _begin(ctx)
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, **_k1_values(6))
    out['settle0'] = fs._settled(ctx, seq, 120)
    out['p1_lap'] = [fs._brief(fs._press_cycle(ctx, 0)) for _ in range(2)]
    out['cpu_lap'] = [_press(ctx, 1.5) for _ in range(len(CPU_LANE) + 1)]
    out['settle'] = fs._settled(ctx, hs.diag_mark(ctx), 150)
    _shot(ctx, out, 'shot_settled', 'cf-k1-settled')
    _mark(t0, out, 'done')
    return _finish(ctx, 'cf-k1-setup', out, [])


def cf_k1_laps(ctx):
    t0, out = _begin(ctx)
    lap_len = len(CPU_LANE) + 1
    patient = _lap(ctx, out, 'k1_patient', 5, 1.5, 20)
    out['k1_patient_cold'], problems = _judge(patient, lap_len)
    fast = _lap(ctx, out, 'k1_fast', 2, 1.5, None)
    out['k1_fast_cold'], fast_problems = _judge(fast, lap_len, skip_laps=0)
    problems += ['fast: ' + p for p in fast_problems]
    if not any(out['k1_patient_cold']):
        problems.append('no cold press in the patient laps: the CPU was not short')
    _mark(t0, out, 'done')
    return _finish(ctx, 'cf-k1-laps', out, problems)


def cf_k1_fits(ctx):
    """Ready Fighters 7: the same lap fits; one settling lap, then the measured lap, all warm (memory comparison)."""
    t0, out = _begin(ctx)
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, cycle_preload_ready_fighters=fs.RF(7))
    out['settle'] = fs._settled(ctx, seq, 150)
    _lap(ctx, out, 'k1_fits_warmup', 1, 1.5, 20)
    presses = _lap(ctx, out, 'k1_fits', 1, 1.5, 20)
    out['k1_fits_cold'] = [p.get('kind') for p in presses if p.get('demand') or p.get('route') == 'group-build']
    _mark(t0, out, 'done')
    problems = ['Ready Fighters 7 still loads %s' % out['k1_fits_cold']] if out['k1_fits_cold'] else []
    return _finish(ctx, 'cf-k1-fits', out, problems)


# --- phase 2 rows (rc.14 2a-2d) ---------------------------------------------------------------------------------------

DIAG_CYCLE = 'diag-cycle.req'


def _diag_cycle(ctx, text):
    """Write `diag-cycle.req` (`reload`, `reload mii`, `off`) and wait for the plugin's `[GroupSwap] diag cycle` line."""
    seq = hs.diag_mark(ctx)
    path = Path(ctx.instance['diag_log']).with_name(DIAG_CYCLE)
    tmp = path.with_name(DIAG_CYCLE + '.tmp')
    tmp.write_text(text + '\n')
    tmp.replace(path)
    return fs._line(ctx, seq, r'\[GroupSwap\] diag cycle ', 10)


def _to(ctx, kind, cap=12):
    """CPU presses until the CPU is `kind` (one lap at most); the press that reached it."""
    last = None
    for _ in range(len(CPU_LANE) + 1):
        if fs._live_kinds(ctx).get(1) == kind:
            return last
        last = _press(ctx, 1.5)
    return last if fs._live_kinds(ctx).get(1) == kind else None


def cf_direct(ctx):
    """2a: a direct CPU Character pick takes the group path. First the lap's cold fighter (King K. Rool) picked
    directly, then a fighter outside the cycle (Bowser): each a `demand`, no Reload and no teardown; the next cycle
    press frees the cold one (`cold release`)."""
    t0, out = _begin(ctx)
    picks = []
    for kind in (fs.KROOL, fs.BOWSER):
        seq = hs.diag_mark(ctx)
        menu = fs._menu(ctx)
        fx._live_set(ctx, cpu_character=fs._onehot(menu, 'cpu_character', fs.CHOICES.index(kind)))
        line = fs._line(ctx, seq, fs._kind_line(1, kind), 60)
        time.sleep(1.0)
        lines = [t for _, _, t in hs.diag_since(ctx, seq)]
        rec = {'kind': hex(kind), 'applied': line}
        for key, regex in PATTERNS.items():
            hits = [t[:300] for t in lines if regex.search(t)]
            if hits:
                rec[key] = hits[:6]
        picks.append(rec)
    out['picks'] = picks
    out['after'] = _press(ctx, 1.5)
    _mark(t0, out, 'done')
    problems = []
    for rec in picks:
        if not rec.get('applied'):
            problems.append('the pick of %s did not apply' % rec['kind'])
        if rec['kind'] == hex(fs.BOWSER) and not rec.get('demand'):
            problems.append('the pick of %s (outside the cycle) was not served by a demand group' % rec['kind'])
        for key in ('reload', 'teardown', 'canonical', 'stall'):
            if rec.get(key):
                problems.append('the pick of %s: %s' % (rec['kind'], rec[key][0][:120]))
    return _finish(ctx, 'cf-direct', out, problems)




def cf_k2(ctx):
    """k = 2: the CPU lap gets Ganondorf too (6 members; Player 1 still Mario > Pikachu) at Ready Fighters 6, so two of
    them don't fit. Three laps; the last two each load the same two cold fighters, no Reload, no teardown."""
    global CPU_LANE
    t0, out = _begin(ctx)
    lane = CPU_LANE
    CPU_LANE = CPU_LANE + [fs.GANON]
    try:
        seq = hs.diag_mark(ctx)
        fx._live_set(ctx, cpu_roster=fs._roster(CPU_LANE), cycle_preload_ready_fighters=fs.RF(6))
        out['settle'] = fs._settled(ctx, seq, 120)
        presses = _lap(ctx, out, 'k2', 3, 1.5, 20)
        out['k2_cold'], problems = _judge(presses, len(CPU_LANE) + 1)
    finally:
        CPU_LANE = lane
        fx._live_set(ctx, cpu_roster=fs._roster(CPU_LANE))
    _mark(t0, out, 'done')
    if out['k2_cold'] and any(len(c) != 2 for c in out['k2_cold']):
        problems.append('not two cold loads per lap: %s' % out['k2_cold'])
    return _finish(ctx, 'cf-k2', out, problems)


def _forced(ctx, out, knob, pick):
    """Ready Fighters 8 (the whole lap fits, Player 1 Mario > Pikachu), the CPU on Diddy Kong (an ordinary window, the Mii
    group Built and detached), then `knob` on and a direct CPU Character pick of `pick` (outside the cycle): the press
    takes today's Reload, a target-first one. Then the knob off and CPU presses until Mii Gunner."""
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, cycle_preload_ready_fighters=fs.RF(8))
    out['settle'] = fs._settled(ctx, seq, 120)
    _lap(ctx, out, 'warm', 1, 1.5, 20)
    out['to_diddy'] = _to(ctx, fs.DIDDY)
    out['knob'] = _diag_cycle(ctx, knob)
    seq = hs.diag_mark(ctx)
    menu = fs._menu(ctx)
    fx._live_set(ctx, cpu_character=fs._onehot(menu, 'cpu_character', fs.CHOICES.index(pick)))
    out['picked'] = fs._line(ctx, seq, fs._kind_line(1, pick), 60)
    time.sleep(2.0)
    lines = [t for _, _, t in hs.diag_since(ctx, seq)]
    out['target_first'] = [t[:240] for t in lines if PATTERNS['target_first'].search(t)][:2]
    out['reload'] = [t[:240] for t in lines if PATTERNS['reload'].search(t)][:2]
    out['kept'] =[t[:300] for t in lines if '[GroupSwap] kept across reload owner=1' in t][:2]
    out['teardown'] = [t[:240] for t in lines if PATTERNS['teardown'].search(t)][:2]
    out['knob_off'] = _diag_cycle(ctx, 'off')
    out['settle_after'] = fs._settled(ctx, hs.diag_mark(ctx), 60)
    presses = []
    for _ in range(len(CPU_LANE) + 1):
        rec = _press(ctx, 2.0)
        presses.append(rec)
        if rec.get('kind') == hex(fs.GUNNER):
            break
    out['presses'] = presses
    time.sleep(3.0)
    out['stalls'] = [t[:200] for _, _, t in hs.diag_lines(ctx) if 'StallWatch' in t][-3:]
    return presses


def cf_forced_reload(ctx):
    """2c: with today's Reload forced (`diag-cycle.req` `reload`), a direct pick of Bowser from Diddy Kong is a
    target-first Reload that keeps the Built ordinary groups (`kept across reload`, rc.13's (a′) and module pins) and
    releases the Mii group (rc.13's rule); the presses then reach Mii Gunner again without a fault."""
    t0, out = _begin(ctx)
    presses = _forced(ctx, out, 'reload', fs.BOWSER)
    _mark(t0, out, 'done')
    problems = []
    if not out['picked'] or not out['target_first'] or not out['reload']:
        problems.append('no target-first Reload for the pick: %s %s' % (out['picked'], out['target_first']))
    if not out['kept']:
        problems.append('no group kept across the Reload')
    if any('4a' in t.split('groups=')[-1] for t in out['kept']):
        problems.append('a Mii group was kept with the switch off: %s' % out['kept'])
    if out['stalls']:
        problems.append('StallWatch: %s' % out['stalls'][-1])
    if not any(p.get('kind') == hex(fs.GUNNER) for p in presses):
        problems.append('the presses never reached Mii Gunner: %s' % [(p.get('kind'), p.get('route')) for p in presses])
    return _finish(ctx, 'cf-forced-reload', out, problems)


def cf_mii_failfirst(ctx):
    """2d fail-first (the rc.12 crash, Eden rc12-int2-f44fac2f-1: Unmapped Read64 via TR +0xfaca4 on the warm attach of a
    Mii group kept across its owner's Reload): the same forced Reload (a direct pick of Peach from Diddy Kong) with
    `reload mii`, so (a′) keeps the Built Mii group `[4a]` too, then the presses reach Mii Gunner: a warm attach of the
    kept group. Fails without the keep, without a warm attach, or on a StallWatch line; the console's Unmapped scan is
    z-boot's."""
    t0, out = _begin(ctx)
    presses = _forced(ctx, out, 'reload mii', fs.PEACH)
    _mark(t0, out, 'done')
    gunner = [p for p in presses if p.get('kind') == hex(fs.GUNNER)]
    problems = []
    if not out['picked'] or not out['target_first'] or not out['reload']:
        problems.append('no target-first Reload for the pick: %s %s' % (out['picked'], out['target_first']))
    if not any('4a' in t.split('groups=')[-1] for t in out['kept']):
        problems.append('the Mii group was not kept across the Reload: %s' % out['kept'])
    if not gunner or gunner[0].get('route') != 'group-warm':
        problems.append('no warm attach of the kept Mii group: %s' % [(p.get('kind'), p.get('route')) for p in presses])
    if out['stalls']:
        problems.append('StallWatch: %s' % out['stalls'][-1])
    return _finish(ctx, 'cf-mii-failfirst', out, problems)


# --- rc.14 Gaps 1 and 3 (Eden cf-p3-y2), and k = 1 from the boot --------------------------------------------------------
# The boot's settings.json fixture (mk-rows.py phase 3, cf_fixture.py) already holds the k = 1 values, Ready Fighters 6,
# Player 1 Mario > Pikachu, the CPU cycle above: the scene builds with them, so no lowered value is resident (Gap 1).

def k1_fixture():
    """The menu values the phase 3 boot's settings.json fixture holds (mk-rows.py)."""
    return _k1_values(6)


def cf_boot(ctx):
    """The fixture's values are live from the scene's first frame; the CPU short (the notice at Ready Fighters 6)."""
    fs._remember_original(ctx)
    t0, out = _begin(ctx)
    menu = fs._menu(ctx)
    want = k1_fixture()
    out['mismatch'] = sorted(k for k, v in want.items() if menu.get(k) != v)
    out['settle'] = fs._settled(ctx, 0, 150)
    observed = [m.group(2) for _, _, t in hs.diag_lines(ctx) for m in [fs.OBSERVED.search(t)] if m and m.group(1) == '1']
    out['cpu_original'] = '0x' + observed[0] if observed else None
    _shot(ctx, out, 'shot_settled', 'cf-boot-settled')
    _mark(t0, out, 'done')
    problems = ['live values differ from the fixture: %s' % out['mismatch']] if out['mismatch'] else []
    if not out['settle'] or (out['settle'].get('planned') or 99) > 6:
        problems.append('the scene did not settle within Ready Fighters 6: %s' % out['settle'])
    return _finish(ctx, 'cf-boot', out, problems)


def cf_shrink(ctx):
    """Gap 1: Ready Fighters back from 7 (everything resident, cf-k1-fits) to 6: the farthest group leaves at once
    (`[GroupSwap] shrink`), the notice settles within 6, and the next two laps each load the same one cold fighter."""
    t0, out = _begin(ctx)
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, cycle_preload_ready_fighters=fs.RF(6))
    out['shrink'] = fs._line(ctx, seq, r'\[GroupSwap\] shrink owner=', 30)
    out['settle'] = fs._settled(ctx, seq, 120)
    presses = _lap(ctx, out, 'shrunk', 3, 1.5, 20)
    out['shrunk_cold'], problems = _judge(presses, len(CPU_LANE) + 1)
    _mark(t0, out, 'done')
    if not out['shrink']:
        problems.append('no shrink line after Ready Fighters dropped to 6')
    if not out['settle'] or (out['settle'].get('planned') or 99) > 6:
        problems.append('not settled within Ready Fighters 6: %s' % out['settle'])
    if any(len(c) != 1 for c in out['shrunk_cold']):
        problems.append('not one cold load per lap: %s' % out['shrunk_cold'])
    return _finish(ctx, 'cf-shrink', out, problems)


EXIT = {
    'demand': re.compile(r'\[GroupSwap\] demand owner=1 .*kinds=\[f\]'),
    'declined': re.compile(r'\[GroupSwap\] demand declined owner=1 '),
    'background': re.compile(r'\[GroupSwap\] background built owner=1 '),
    'warm_exit': re.compile(r'\[GroupSwap\] warm exit owner=1 kind=0xf'),
    'reload': PATTERNS['reload'],
    'teardown': PATTERNS['teardown'],
    'canonical': PATTERNS['canonical'],
    'stall': PATTERNS['stall'],
}


def cf_exit(ctx):
    """Gap 3: out of the live Mii Gunner onto a fighter no Built group holds (a direct CPU Character pick of Bowser,
    outside a small cycle): the press waits for Bowser's own group through its prefetch and its background build beside
    the Mii window, then exits warm. No Reload, no teardown, no canonical replacement. The groups the smaller cycle
    leaves are parked until a pause (operator decision 2026-11-08 23:42Z), so Training's pause opens and closes first:
    their slots are free for Bowser's group (the slot limit itself is Gap 2, a-full-2's)."""
    global CPU_LANE
    t0, out = _begin(ctx)
    lane = CPU_LANE
    CPU_LANE = [fs.GUNNER, fs.PICHU]
    try:
        seq = hs.diag_mark(ctx)
        fx._live_set(ctx, cpu_roster=fs._roster(CPU_LANE), cycle_preload_ready_fighters=fs.RF(8))
        time.sleep(1.0)
        steps.press(ctx, 'plus', 2.0)
        steps.press(ctx, 'plus', 2.0)
        out['pause_frees'] = [t[:200] for _, _, t in hs.diag_since(ctx, seq) if '[GroupSwap] pause frees owner=1' in t][:4]
        out['settle'] = fs._settled(ctx, seq, 120)
        out['to_gunner'] = _to(ctx, fs.GUNNER)
        out['gunner'] = fs._live_kinds(ctx).get(1)
        out['settle_gunner'] = fs._settled(ctx, hs.diag_mark(ctx), 60)
        seq = hs.diag_mark(ctx)
        menu = fs._menu(ctx)
        fx._live_set(ctx, cpu_character=fs._onehot(menu, 'cpu_character', fs.CHOICES.index(fs.BOWSER)))
        out['picked'] = fs._line(ctx, seq, fs._kind_line(1, fs.BOWSER), 60)
        time.sleep(1.5)
        lines = [t for _, _, t in hs.diag_since(ctx, seq)]
        for key, regex in EXIT.items():
            out[key] = [t[:300] for t in lines if regex.search(t)][:4]
        _shot(ctx, out, 'shot_bowser', 'cf-exit-bowser')
        out['after'] = _press(ctx, 1.5)
    finally:
        CPU_LANE = lane
        fx._live_set(ctx, cpu_roster=fs._roster(CPU_LANE))
    _mark(t0, out, 'done')
    problems = []
    if out['gunner'] != fs.GUNNER:
        problems.append('the CPU never reached Mii Gunner: %s' % out['gunner'])
    if not out['picked']:
        problems.append('the pick of Bowser did not apply')
    for key in ('demand', 'background', 'warm_exit'):
        if not out[key]:
            problems.append('no %s line' % key)
    for key in ('declined', 'reload', 'teardown', 'canonical', 'stall'):
        if out[key]:
            problems.append('%s: %s' % (key, out[key][0][:160]))
    return _finish(ctx, 'cf-exit', out, problems)