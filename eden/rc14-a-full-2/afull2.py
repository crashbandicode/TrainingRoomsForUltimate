"""rc.14 a-full-2 rows, 2026-11-14: the three limits a-full left, plus cycle-fix's Gap 2
(artifacts/rc14-cycle-load/AFULL2-RESEARCH.md; the primary's Phase B approval ~23:24Z).

Knob (`diag-cycle.req`): `window` A1 on; `reload` a cold press or direct pick takes today's Reload; `strip` K-strip on;
`pt` (with `window`) a Pokemon Trainer window is kept too. Kinds: gna_p2_fs's constants (TR trio.rs); PT 0x24 and Pyra
0x5a from trio.rs :93-97. Helpers: cyclefix.py (cycle-fix), afull.py (a-full) and a1.py (a1-enable) beside this file.

Rows (hooks; evidence is the diag log and the receipts):
- a2_setup: cycle-fix's setup (Ready Fighters 8, the console's lanes), the knob off.
- a2_k2 / a2_direct (S1, Gap 2): cycle-fix's cf-k2 and cf-direct as they are; no `demand declined ... no free group slot`.
- a2_r6 (S1, A1): the console P1 lane (three members) with Kirby alone; the forced Reload keeps Kirby's window (a-full's
  boot 1 refused it: no free group slot), then the warm return at cell 0.
- a2_strip (K-strip, k2 + k5): Ready Fighters 10, Kirby with a hat; the press to Palutena strips the copy and parks him
  (`[KirbyPark] strip ... gate=Ok`), the target's group attaches warm, no Reload and no `[KirbyPark] fallback` (a fired
  fallback is a counted failure); back to Kirby: warm, copy-free.
- a2_strip_same (k3): Kirby with a Pikachu hat, the press to Pikachu, whose group is Built (the D-KIRBY1 / G68 class).
- a2_strip_move (k4): Kirby with a hat holds B (his copy special), and the press comes in the middle of it.
- a2_pt (p1-p4): Player 1 on a PT window, `reload window pt`: the window kept (special Native), its articles retired;
  back to PT warm, articles rebuilt; two P1 laps. The trainer counts S+0x1968/S+0x1980 are logged by the diag build.
- a2_aegis_probe (a0, diag only): Player 1 on a Pyra window, `reload window`: the probe lines before and after the Reload
  start (flags, +0x5a90, holder articles). Last before z: a1-enable's crash followed a Reload out of a Pyra window.
- a2_restore: the knob off, cycle-fix's restore."""
import re
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import hotslots as hs  # noqa: E402
import gna_p2_fs as fs  # noqa: E402
import fx  # noqa: E402
import cyclefix as cf  # noqa: E402
import afull as af  # noqa: E402
from gna import _begin, _mark, _shot, _finish  # noqa: E402

PT, PYRA = 0x24, 0x5a
SLOT = re.compile(r'demand declined owner=\d kind=\S+: no free group slot|not kept \(no free group slot\)')
STRIP = re.compile(r'\[KirbyPark\] strip owner=0 target=(0x[0-9a-f]+) .*? gate=(Ok|Err)')
FALLBACK = re.compile(r'\[KirbyPark\] fallback owner=0 ')
RELOAD0 = re.compile(r'\[TrioBoundary\] reload entry=\S+ owner=0 ')
ATTACH0 = re.compile(r'\[GroupSwap\] attach owner=0 group=\d+ member=\d+ kinds=\[([^\]]*)\]')
REUSE0 = re.compile(r'\[BodyBank\] reuse owner=0 ')
ARTICLES = re.compile(r'\[GroupSwap\] articles (retired|rebuilt|deactivated|NOT rebuilt) owner=0 ')
PROBE = re.compile(r'\[GroupSwap\] native window (before reload|after reload start) owner=0 .*')
COUNTS = re.compile(r'trainer_counts=\((\d+), (\d+)\)')
LEDGER = re.compile(r'\[Ledger\].*(refus|kept)')


def _knob(ctx, words):
    return cf._diag_cycle(ctx, words)


def a2_setup(ctx):
    out = cf.cf_setup(ctx)
    _knob(ctx, 'off')
    return out


def _slots(ctx, seq):
    return [t[:200] for t in af._since(ctx, seq) if SLOT.search(t)]


def a2_k2(ctx):
    seq = hs.diag_mark(ctx)
    out = cf.cf_k2(ctx)
    hits = _slots(ctx, seq)
    if hits:
        raise RuntimeError('a2-k2: a group slot made a fitting fighter cold: %s' % hits[:3])
    return out


def a2_direct(ctx):
    seq = hs.diag_mark(ctx)
    out = cf.cf_direct(ctx)
    hits = _slots(ctx, seq)
    if hits:
        raise RuntimeError('a2-direct: a group slot made a fitting fighter cold: %s' % hits[:3])
    return out


def a2_r6(ctx):
    """a-full's af_kirby on the console's three-member P1 lane (its boot 1 r6 refused the keep: no free group slot)."""
    t0, out = _begin(ctx)
    _knob(ctx, 'window')
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, player_roster=fs._roster(cf.P1_LANE))
    out['settle0'] = fs._settled(ctx, seq, 120)
    out['to_kirby'] = af._to(ctx, 0, fs.KIRBY, len(cf.P1_LANE) + 2)
    out['window'] = [hex(k) for k in (af._window(ctx, 0) or [])]
    af._keep_across(ctx, out, 0, fs.LINK)
    _knob(ctx, 'window')
    seq = hs.diag_mark(ctx)
    out['back'] = af._to(ctx, 0, fs.KIRBY, len(cf.P1_LANE) + 2)
    time.sleep(3.0)
    kept = out['kept'][-1] if out['kept'] else None
    warm = kept is not None and af._warm_back(ctx, out, 0, kept[0], kept[1], seq)
    out['registry'] = [t[:200] for t in af._since(ctx, seq) if '[Registry] attach owner=0' in t][:3]
    _shot(ctx, out, 'shot_kirby', 'a2-r6-back')
    _mark(t0, out, 'done')
    problems = []
    if out['window'] != [hex(fs.KIRBY)]:
        problems.append('Kirby is not alone in his window: %s' % out['window'])
    if not kept or kept[1] != [fs.KIRBY]:
        problems.append('Kirby\'s window was not kept: %s %s' % (out['kept'], out['not_kept']))
    if any('no free group slot' in t for t in out['not_kept']):
        problems.append('the keep still found no free group slot: %s' % out['not_kept'])
    if not warm:
        problems.append('no warm return to the kept Kirby: %s' % out.get('attaches'))
    problems += ['bad line: %s' % t for t in out['bad'] + out.get('bad_after', [])]
    return _finish(ctx, 'a2-r6', out, problems)


def _strip_lines(lines):
    return {'strip': [t[:400] for t in lines if STRIP.search(t)][:3],
            'fallback': [t[:200] for t in lines if FALLBACK.search(t)][:3],
            'reload': [t[:200] for t in lines if RELOAD0.search(t)][:3],
            'attach': [t[:200] for t in lines if ATTACH0.search(t) or REUSE0.search(t)][:3],
            'ledger': [t[:200] for t in lines if LEDGER.search(t)][:3],
            'bad': af._bad(lines)}


def _strip_problems(rec, tag):
    problems = []
    if not rec['strip'] or not all(STRIP.search(t).group(2) == 'Ok' for t in rec['strip']):
        problems.append('%s: no successful strip: %s' % (tag, rec['strip']))
    if rec['fallback']:
        problems.append('%s: the K-strip fallback fired (a counted failure): %s' % (tag, rec['fallback']))
    if rec['reload']:
        problems.append('%s: the press took a Reload: %s' % (tag, rec['reload']))
    if not rec['attach']:
        problems.append('%s: no warm attach or reuse' % tag)
    problems += ['%s bad line: %s' % (tag, t) for t in rec['bad']]
    return problems


def _hat_and_press(ctx, t0, out, hats, target, during=None):
    """Kirby with a hat from `hats`, then a press to `target` (a cycle press, or a direct pick when `target` isn't next);
    `during` holds a button first and presses inside it. The lines since the press."""
    fx._live_set(ctx, player_kirby_hats=fs._roster(hats))
    out['to_kirby'] = af._to(ctx, 0, fs.KIRBY, len(cf.P1_LANE) + 2)
    out['hat'] = bool(fs._hat(ctx, t0, out, 'hat'))
    time.sleep(1.0)
    seq = hs.diag_mark(ctx)
    if during:
        # The copy special starts on the tap; the press follows inside its animation (Falco's blaster: about 40 frames).
        ctx.call('input.button', {'button': during, 'state': 'tap', 'duration_ms': 150, 'lease_ms': 650})
        time.sleep(0.2)
    if target is None:
        out['press'] = fs._brief(fs._press_cycle(ctx, 0))
    else:
        _, out['picked'] = af._pick(ctx, 0, target)
    out['settle'] = fs._settled(ctx, hs.diag_mark(ctx), 60)
    return af._since(ctx, seq)


def a2_strip(ctx):
    t0, out = _begin(ctx)
    _knob(ctx, 'window strip')
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, player_roster=fs._roster(cf.P1_LANE), cycle_preload_ready_fighters=fs.RF(10))
    out['settle0'] = fs._settled(ctx, seq, 120)
    lines = _hat_and_press(ctx, t0, out, fs.HATS, None)
    out['k2'] = _strip_lines(lines)
    _shot(ctx, out, 'shot_after', 'a2-strip-after')
    seq = hs.diag_mark(ctx)
    out['back'] = af._to(ctx, 0, fs.KIRBY, len(cf.P1_LANE) + 2)
    time.sleep(2.0)
    back = af._since(ctx, seq)
    out['k5'] = {'reload': [t[:200] for t in back if RELOAD0.search(t)][:3],
                 'attach': [t[:200] for t in back if ATTACH0.search(t) or REUSE0.search(t)][:3],
                 'registry': [t[:200] for t in back if '[Registry] attach owner=0' in t][:3],
                 'bad': af._bad(back)}
    _shot(ctx, out, 'shot_kirby', 'a2-strip-kirby-back')
    _mark(t0, out, 'done')
    problems = [] if out['hat'] else ['no hat confirmed: the copy gate was not set up']
    problems += _strip_problems(out['k2'], 'k2')
    if out['k5']['reload'] or not out['k5']['attach']:
        problems.append('k5: the return to Kirby was not warm: %s' % out['k5'])
    problems += ['k5 bad line: %s' % t for t in out['k5']['bad']]
    return _finish(ctx, 'a2-strip', out, problems)


def a2_strip_same(ctx):
    t0, out = _begin(ctx)
    _knob(ctx, 'window strip')
    lines = _hat_and_press(ctx, t0, out, [fs.PIKACHU], fs.PIKACHU)
    out['k3'] = _strip_lines(lines)
    _shot(ctx, out, 'shot_after', 'a2-strip-same-after')
    _mark(t0, out, 'done')
    problems = [] if out['hat'] else ['no Pikachu hat confirmed']
    problems += _strip_problems(out['k3'], 'k3')
    problems += ['k3 ledger: %s' % t for t in out['k3']['ledger']]
    return _finish(ctx, 'a2-strip-same', out, problems)


def a2_strip_move(ctx):
    t0, out = _begin(ctx)
    _knob(ctx, 'window strip')
    lines = _hat_and_press(ctx, t0, out, fs.HATS, None, during='b')
    out['k4'] = _strip_lines(lines)
    _shot(ctx, out, 'shot_after', 'a2-strip-move-after')
    _mark(t0, out, 'done')
    problems = [] if out['hat'] else ['no hat confirmed']
    problems += _strip_problems(out['k4'], 'k4')
    return _finish(ctx, 'a2-strip-move', out, problems)


def a2_pt(ctx):
    t0, out = _begin(ctx)
    _knob(ctx, 'window')
    seq = hs.diag_mark(ctx)
    lane = [fs.PIKACHU, PT, fs.PALUTENA]
    fx._live_set(ctx, player_roster=fs._roster(lane), player_kirby_hats=fs._roster(fs.HATS),
                 cycle_preload_ready_fighters=fs.RF(10))
    out['settle0'] = fs._settled(ctx, seq, 120)
    out['to_pt'] = af._to(ctx, 0, PT, len(lane) + 2)
    out['settle1'] = fs._settled(ctx, hs.diag_mark(ctx), 90)
    out['window'] = [hex(k) for k in (af._window(ctx, 0) or [])]
    lines = af._keep_across(ctx, out, 0, fs.BOWSER, 'reload window pt')
    _knob(ctx, 'window')
    out['probe'] = [t[:600] for t in lines if PROBE.search(t)][:4]
    out['articles_keep'] = [t[:240] for t in lines if ARTICLES.search(t)][:4]
    out['counts_keep'] = [COUNTS.search(t).groups() for t in lines if COUNTS.search(t)][:6]
    seq = hs.diag_mark(ctx)
    _, out['picked_back'] = af._pick(ctx, 0, PT)
    time.sleep(3.0)
    kept = out['kept'][-1] if out['kept'] else None
    warm = kept is not None and af._warm_back(ctx, out, 0, kept[0], kept[1], seq)
    back = af._since(ctx, seq)
    out['articles_back'] = [t[:240] for t in back if ARTICLES.search(t)][:4]
    out['laps'] = [fs._brief(fs._press_cycle(ctx, 0)) for _ in range(2 * len(lane))]
    after = af._since(ctx, seq)
    out['counts_after'] = [COUNTS.search(t).groups() for t in after if COUNTS.search(t)][:6]
    out['bad_laps'] = af._bad(after)
    _shot(ctx, out, 'shot_pt', 'a2-pt-back')
    _mark(t0, out, 'done')
    problems = []
    if not kept or sorted(kept[1]) != [0x24, 0x25, 0x26]:
        problems.append('the PT window was not kept: window %s, kept %s, not kept %s' % (out['window'], out['kept'],
                                                                                        out['not_kept']))
    if not warm:
        problems.append('no warm return to the kept PT window: %s' % out.get('attaches'))
    if not any('rebuilt' in t for t in out['articles_back']):
        problems.append('the trainer articles were not rebuilt at the return: %s' % out['articles_back'])
    if not all(p.get('outcome') for p in out['laps']):
        problems.append('a lap press did not apply: %s' % [(p.get('kind'), p.get('outcome')) for p in out['laps']])
    problems += ['bad line: %s' % t for t in out['bad'] + out.get('bad_after', []) + out['bad_laps']]
    return _finish(ctx, 'a2-pt', out, problems)


def a2_aegis_probe(ctx):
    """a0, diag only: what the Reload start does to an Aegis window's holder (AFULL2-RESEARCH.md "Aegis"). No keep."""
    t0, out = _begin(ctx)
    _knob(ctx, 'off')
    seq = hs.diag_mark(ctx)
    lane = [fs.PIKACHU, PYRA]
    fx._live_set(ctx, player_roster=fs._roster(lane), cycle_preload_ready_fighters=fs.RF(10))
    out['settle0'] = fs._settled(ctx, seq, 120)
    out['to_pyra'] = af._to(ctx, 0, PYRA, len(lane) + 2)
    out['settle1'] = fs._settled(ctx, hs.diag_mark(ctx), 90)
    lines = af._keep_across(ctx, out, 0, fs.BOWSER, 'reload')
    out['probe'] = [t[:600] for t in lines if PROBE.search(t)][:4]
    _mark(t0, out, 'done')
    problems = []
    if len(out['probe']) < 2:
        problems.append('the Aegis probe lines are missing: %s' % out['probe'])
    problems += ['bad line: %s' % t for t in out['bad']]
    return _finish(ctx, 'a2-aegis-probe', out, problems)


def a2_restore(ctx):
    _knob(ctx, 'off')
    return cf.cf_restore(ctx)
