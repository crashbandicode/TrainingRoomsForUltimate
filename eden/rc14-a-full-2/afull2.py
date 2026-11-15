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
- a2_kirby_grouped (a-full-3, gated_alone's revert): a2-strip's flow with the shipped switches only; Kirby's window holds
  another member (precondition), the press away strips and parks him, the target warm, back to Kirby warm.
- a2_p4 / a2_pyra_keep_all (a-full-3): setups reworked to reach their preconditions (fresh groups at Ready Fighters 12 / 10,
  a direct pick onto the built Pyra group); their assertions are unchanged.
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
import steps  # noqa: E402
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


DECLINED = re.compile(r'\[GroupSwap\] demand declined owner=1 kind=(0x[0-9a-f]+): (.*)')


def _with_reasons(ctx, name, run):
    """cycle-fix's row `run`; a failure also names `demand`'s own declines and any slot refusal (boot a2-e881df1e-y1: the
    decline had rotated out of the diag log by the time it was read)."""
    seq = hs.diag_mark(ctx)
    try:
        out = run(ctx)
    except RuntimeError as failed:
        lines = af._since(ctx, seq)
        declined = sorted({DECLINED.search(t).group(2)[:120] for t in lines if DECLINED.search(t)})[:6]
        raise RuntimeError('%s; demand declines: %s; slot refusals: %s' % (str(failed)[:900], declined or 'none',
                                                                         _slots(ctx, seq)[:2] or 'none'))
    hits = _slots(ctx, seq)
    if hits:
        raise RuntimeError('%s: a group slot made a fitting fighter cold: %s' % (name, hits[:3]))
    return out


READY = re.compile(r'\[CyclePreload\] applied .*? ready_fighters=(\d+)')


def a2_k2(ctx):
    """cf-k2 in the state its expectation was written for (its docstring: Player 1 still Mario > Pikachu; it sets Ready
    Fighters 6 itself). Boot a2-e881df1e-y1's first pass ran it with the console's P1 lane (Pikachu, Kirby, Palutena),
    whose bodies left the CPU 4 cold fighters a lap. The Ready Fighters the plugin applied is read back and named."""
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, player_roster=fs._roster([fs.PIKACHU]))
    fs._settled(ctx, seq, 120)
    try:
        return _with_reasons(ctx, 'a2-k2', cf.cf_k2)
    except RuntimeError as failed:
        applied = [int(m.group(1)) for _, _, t in hs.diag_lines(ctx) for m in [READY.search(t)] if m]
        raise RuntimeError('%s; Ready Fighters applied %s; P1 lane Mario > Pikachu; CPU lap %d members'
                           % (str(failed)[:1200], applied[-1:] or 'unread', len(cf.CPU_LANE) + 2))
    finally:
        fx._live_set(ctx, player_roster=fs._roster(cf.P1_LANE))


def a2_direct(ctx):
    """cf-direct from the console lanes at Ready Fighters 8, set again here so a retry after other rows starts the same."""
    _knob(ctx, 'off')
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, player_roster=fs._roster(cf.P1_LANE), cpu_roster=fs._roster(cf.CPU_LANE),
                 cycle_preload_ready_fighters=fs.RF(8))
    fs._settled(ctx, seq, 120)
    return _with_reasons(ctx, 'a2-direct', cf.cf_direct)


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
    out['window'] = [hex(k) for k in (af._window(ctx, 0) or [])]
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


def _strip_row(ctx, name, knob, grouped):
    """The a2-strip flow under `name`: the diag knob `knob`, Ready Fighters 10, Kirby with a hat, the press to the next
    fighter; then back to Kirby. `grouped`: Kirby's window must hold another member (gated_alone's revert, a-full-3)."""
    t0, out = _begin(ctx)
    _knob(ctx, knob)
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, player_roster=fs._roster(cf.P1_LANE), cycle_preload_ready_fighters=fs.RF(10))
    out['settle0'] = fs._settled(ctx, seq, 120)
    lines = _hat_and_press(ctx, t0, out, fs.HATS, None)
    out['k2'] = _strip_lines(lines)
    _shot(ctx, out, 'shot_after', name + '-after')
    seq = hs.diag_mark(ctx)
    out['back'] = af._to(ctx, 0, fs.KIRBY, len(cf.P1_LANE) + 2)
    time.sleep(2.0)
    back = af._since(ctx, seq)
    out['k5'] = {'reload': [t[:200] for t in back if RELOAD0.search(t)][:3],
                 'attach': [t[:200] for t in back if ATTACH0.search(t) or REUSE0.search(t)][:3],
                 'registry': [t[:200] for t in back if '[Registry] attach owner=0' in t][:3],
                 'bad': af._bad(back)}
    _shot(ctx, out, 'shot_kirby', name + '-kirby-back')
    _mark(t0, out, 'done')
    problems = [] if out['hat'] else ['no hat confirmed: the copy gate was not set up']
    if grouped and (hex(fs.KIRBY) not in out['window'] or len(out['window']) < 2):
        problems.append('precondition: Kirby is not grouped with another member: window %s' % out['window'])
    problems += _strip_problems(out['k2'], 'k2')
    if out['k5']['reload'] or not out['k5']['attach']:
        problems.append('k5: the return to Kirby was not warm: %s' % out['k5'])
    problems += ['k5 bad line: %s' % t for t in out['k5']['bad']]
    return _finish(ctx, name, out, problems)


def a2_strip(ctx):
    return _strip_row(ctx, 'a2-strip', 'window strip', False)


def a2_kirby_grouped(ctx):
    """gated_alone's revert (a-full-3): Kirby is laid out like any member again, so his window holds other fighters. With
    the shipped switches only (the knob off) a Kirby with a hat is pressed away: his copy goes natively and he parks, no
    Reload and no `[KirbyPark] fallback`; the next fighter's window attaches warm (or is reused); back to Kirby warm.
    a2-strip's checks, plus the precondition that Kirby's window holds another member."""
    return _strip_row(ctx, 'a2-kirby-grouped', 'off', True)


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


def _pause(ctx):
    """Training's pause (Plus, held, Plus): parked groups are freed as it opens (operator decision 23:42Z)."""
    steps.press(ctx, 'plus', 3.0)
    steps.press(ctx, 'plus', 2.0)


def a2_pt(ctx):
    t0, out = _begin(ctx)
    _knob(ctx, 'window')
    lane = [fs.PIKACHU, PT, fs.PALUTENA]
    # Precondition (boot a2-99b9083f-y1: PT never built as a group, "Loaded 6/8 · swap P1 to load 2 more", so the pick was
    # a Reload and nothing could be kept): P1 on Pikachu, any PT group from an earlier row freed (a lane without PT and a
    # pause), then the PT lane at Ready Fighters 12, and the pick only once a fresh `[24, 25, 26]` group is built.
    _, out['to_pikachu'] = af._pick(ctx, 0, fs.PIKACHU)
    fx._live_set(ctx, player_roster=fs._roster([fs.PIKACHU, fs.PALUTENA]), cycle_preload_ready_fighters=fs.RF(12))
    time.sleep(2.0)
    _pause(ctx)
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, player_roster=fs._roster(lane), player_kirby_hats=fs._roster(fs.HATS),
                 cycle_preload_ready_fighters=fs.RF(12))
    out['settle0'] = fs._settled(ctx, seq, 120)
    # Boot a2-e881df1e-y1: cycle presses never reached PT (no `request owner=0 kind=0x24`); a direct pick once its group
    # is built, as a1-enable's `_land` does.
    out['pt_built'] = bool(hs.wait_diag(ctx, seq, r'background built owner=0 group=\d+ ok=true reason=\S+ kinds=\[24, 25, 26\]', 120))
    if not out['pt_built']:
        _mark(t0, out, 'precondition')
        raise RuntimeError('a2-pt precondition: no PT group built at Ready Fighters 12 (settle %s)' % out['settle0'])
    _, out['to_pt'] = af._pick(ctx, 0, PT)
    out['settle1'] = fs._settled(ctx, hs.diag_mark(ctx), 90)
    out['window'] = [hex(k) for k in (af._window(ctx, 0) or [])]
    seq_keep = hs.diag_mark(ctx)
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
    # The kept record's pool comes back before or at its attach: rc.14 prepares a retired PT group's articles on the task
    # worker once its object kind is re-taken (`articles prepared … pool=0->1`, boot a2-e881df1e-y1 #9761-#9864), else
    # the attach rebuilds them (`articles rebuilt`).
    out.update(pt_return_verdict(kept, af._since(ctx, seq_keep), back))
    problems += out['pt_problems']
    if not all(p.get('outcome') for p in out['laps']):
        problems.append('a lap press did not apply: %s' % [(p.get('kind'), p.get('outcome')) for p in out['laps']])
    problems += ['bad line: %s' % t for t in out['bad'] + out.get('bad_after', []) + out['bad_laps']]
    return _finish(ctx, 'a2-pt', out, problems)


def pt_return_verdict(kept, since_keep, back):
    """The kept PT window's return, as rc.14 does it (the primary ~01:35Z): the task worker prepares the retired pool once
    the object kind is re-taken (`articles prepared owner=0 group=<kept> … pool=0->N`, between the keep and the attach),
    then the kept `[24, 25, 26]` group attaches, the bank is ready on a PT form, and no fallback comes. Pure: lines in,
    findings out (the offline check of a saved log uses it too)."""
    group = kept[0] if kept else None
    prepared = [t[:240] for t in since_keep
                if group is not None and re.search(r'\[GroupSwap\] articles prepared owner=0 group=%d .*pool=0->[1-9]' % group, t)]
    attach = [t[:200] for t in back
              if group is not None and re.search(r'\[GroupSwap\] attach owner=0 group=%d member=\d+ kinds=\[24, 25, 26\]' % group, t)]
    ready = [t[:200] for t in back if re.search(r'\[BodyBank\] ready owner=0 index=\d+ kind=0x2[456] ', t)]
    fallback = [t[:200] for t in back if re.search(r'\[GroupSwap\] fallback owner=0 ', t)]
    problems = []
    if not prepared:
        problems.append('no worker-prepared pool for the kept group %s' % group)
    if not attach:
        problems.append('no attach of the kept [24, 25, 26] group %s' % group)
    if not ready:
        problems.append('no ready on a PT form after the attach')
    if fallback:
        problems.append('a fallback at the return: %s' % fallback[:2])
    return {'articles_prepared': prepared[:2], 'pt_attach': attach[:2], 'pt_ready': ready[:2], 'pt_fallback': fallback[:2],
            'pt_problems': problems}


DESTROY_RECORDS = re.compile(r'\[Destroy\] records owner=0 holder=(0x[0-9a-f]+) trainer_counts Some\(\((\d+), (\d+)\)\) -> '
                             r'Some\(\((\d+), (\d+)\)\)')
FREES = r'\[GroupSwap\] (frees now|left cycle|pause frees) owner=0 group=\d+ kinds=\[%s\]'
OBJECT_MODULE = re.compile(r'object module (held|dropped|handed to R|kept for the window)')


def a2_aegis(ctx):
    """Limit 2, Aegis (after probe a0): a Pyra/Mythra window kept across a forced Reload (`reload window aegis`), then its
    warm return. Pass: the keep and its `partner holder` line, the kept `[5a, 5b]` group's attach, ready on Pyra or Mythra,
    no fallback, two presses applied after; no bad line."""
    t0, out = _begin(ctx)
    _knob(ctx, 'window')
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, player_roster=fs._roster([fs.PIKACHU, PYRA]), cycle_preload_ready_fighters=fs.RF(10))
    out['settle0'] = fs._settled(ctx, seq, 120)
    out['pyra_built'] = bool(hs.wait_diag(ctx, seq, r'background built owner=0 group=\d+ ok=true reason=\S+ kinds=\[5a, 5b\]', 90))
    _, out['to_pyra'] = af._pick(ctx, 0, PYRA)
    out['settle1'] = fs._settled(ctx, hs.diag_mark(ctx), 90)
    out['window'] = [hex(k) for k in (af._window(ctx, 0) or [])]
    lines = af._keep_across(ctx, out, 0, fs.BOWSER, 'reload window aegis')
    _knob(ctx, 'window')
    out['partner'] = [t[:300] for t in lines if 'partner holder=' in t][:2]
    out['probe'] = [t[:400] for t in lines if PROBE.search(t)][:2]
    kept = out['kept'][-1] if out['kept'] else None
    seq = hs.diag_mark(ctx)
    _, out['picked_back'] = af._pick(ctx, 0, PYRA)
    time.sleep(3.0)
    back = af._since(ctx, seq)
    group = kept[0] if kept else None
    out['attach'] = [t[:200] for t in back if group is not None
                     and re.search(r'\[GroupSwap\] attach owner=0 group=%d member=\d+ kinds=\[5a, 5b\]' % group, t)][:2]
    out['ready'] = [t[:200] for t in back if re.search(r'\[BodyBank\] ready owner=0 index=\d+ kind=0x5[ab] ', t)][:2]
    out['fallback'] = [t[:200] for t in back if re.search(r'\[GroupSwap\] fallback owner=0 ', t)][:2]
    out['laps'] = [fs._brief(fs._press_cycle(ctx, 0)) for _ in range(2)]
    out['bad'] = af._bad(af._since(ctx, seq) + lines)
    _shot(ctx, out, 'shot_pyra', 'a2-aegis-back')
    _mark(t0, out, 'done')
    problems = []
    if not kept or sorted(kept[1]) != [0x5a, 0x5b] or not out['partner']:
        problems.append('the Aegis window was not kept: window %s, kept %s, not kept %s' % (out['window'], out['kept'], out['not_kept']))
    if not out['attach'] or not out['ready'] or out['fallback']:
        problems.append('no warm return to the kept Aegis window: attach %s ready %s fallback %s' % (out['attach'], out['ready'], out['fallback']))
    if not all(p.get('outcome') for p in out['laps']):
        problems.append('a press after the return did not apply: %s' % out['laps'])
    problems += ['bad line: %s' % t for t in out['bad']]
    return _finish(ctx, 'a2-aegis', out, problems)


BUILT = r'background built owner=0 group=\d+ ok=true reason=\S+ kinds=\[%s\]'
# Each step's group must be BUILT fresh (a hard precondition, boot a2-f4acc30e-x1: the lanes at Ready Fighters 8 built
# neither the PT nor the Pyra group, so there was no teardown to count). The first column of a step is its lane, the
# second the line its group's build prints (None: no build in this step), the third the group the lane change frees.
P4_STEPS = [([fs.PIKACHU, PT], BUILT % '24, 25, 26', None),
            ([fs.PIKACHU, PYRA], BUILT % '5a, 5b', FREES % '24, 25, 26'),
            ([fs.PIKACHU, fs.PALUTENA], None, FREES % '5a, 5b')]
P4_READY_FIGHTERS = 12


def a2_p4(ctx):
    """p4: tearing down a PT and then an Aegis group gives back finalize's shared trainer count. P1 on Pikachu (outside
    both groups), the lane goes PT -> Pyra -> Palutena, so each native group is built in the background, leaves the cycle
    and parks; a Training pause after each step frees what parked (operator decision 23:42Z, as `hs.pause_frees`). Pass:
    every `[Destroy] records` line drops exactly one of `S+0x1968` (PT holder) or `S+0x1980` (Aegis holder), and at least
    one of each kind came.

    Boot a2-99b9083f-y1 crashed here (`0x33c23f8`, the Aegis group a2-aegis kept, torn down by the first step's shrink):
    every switch is on for it now (`window mii pt aegis strip`, no forced Reload), and the object modules the groups hold
    are reported (`object module held` / `dropped` / `handed to R`). No crash is the first verdict (z-boot reads the console).

    Setup (a-full-3; the assertions are unchanged): boot a2-f4acc30e-x1 built no fresh PT or Pyra group, so there was no
    teardown to count. Ready Fighters 8 leaves no room for a 3-body PT group beside the console's CPU lane (a2-pt's own
    precondition: "Loaded 6/8 · swap P1 to load 2 more"), so every step runs at Ready Fighters 12. A group an earlier row
    left in the cycle would be reused, not built, so the row first drops PT and Pyra from the lane and pauses (freeing what
    parked) before its own count starts; those frees are recorded apart (`preclean`). P1 is picked to Pikachu only when it
    isn't there (a pick to the fighter already shown waits its whole 60 s cap). A group that is not built within its wait
    is a precondition failure, named as one."""
    t0, out = _begin(ctx)
    _knob(ctx, 'window mii pt aegis strip')
    live = fs._live_kinds(ctx).get(0)
    out['p1_before'] = hex(live) if live is not None else None
    if live != fs.PIKACHU:
        _, out['to_pikachu'] = af._pick(ctx, 0, fs.PIKACHU)
    clean = hs.diag_mark(ctx)
    fx._live_set(ctx, player_roster=fs._roster([fs.PIKACHU, fs.PALUTENA]),
                 cycle_preload_ready_fighters=fs.RF(P4_READY_FIGHTERS))
    out['preclean_settle'] = fs._settled(ctx, clean, 90)
    _pause(ctx)
    time.sleep(2.0)
    out['preclean'] = [t[:200] for t in af._since(ctx, clean) if '] pause frees ' in t or '[Destroy] records' in t][:6]
    seq0 = hs.diag_mark(ctx)
    out['steps'] = []
    for i, (lane, built, frees) in enumerate(P4_STEPS):
        seq = hs.diag_mark(ctx)
        fx._live_set(ctx, player_roster=fs._roster(lane), cycle_preload_ready_fighters=fs.RF(P4_READY_FIGHTERS))
        step = {'lane': [hex(k) for k in lane]}
        if built:
            step['built'] = bool(hs.wait_diag(ctx, seq, built, 120))
            if not step['built']:
                out['steps'].append(step)
                _knob(ctx, 'off')
                _mark(t0, out, 'precondition')
                return _finish(ctx, 'a2-p4', out, ['precondition: no fresh group built for the lane %s at Ready Fighters %d '
                                                   '(waited 120 s, last status %s)' % (step['lane'], P4_READY_FIGHTERS,
                                                                                       fs._last_planned(ctx))])
        if frees:
            step['left'] = bool(hs.wait_diag(ctx, seq, frees, 90))
        # Training's pause (Plus, held, Plus) frees what parked, as `hs.pause_frees` does; that helper also marks a guest
        # trace this run has none of (boot a2-99b9083f-y1: KeyError 'trace_jsonl'). Informational: the verdict is the counts.
        pause = hs.diag_mark(ctx)
        _pause(ctx)
        step['pause_frees'] = [t[:200] for t in af._since(ctx, pause) if '] pause frees ' in t][:4]
        out['steps'].append(step)
    time.sleep(2.0)
    found = [m.groups() for t in af._since(ctx, seq0) for m in [DESTROY_RECORDS.search(t)] if m]
    out['destroys'] = found[:8]
    pt = aegis = 0
    problems = []
    for holder, a, b, a2, b2 in found:
        da, db = int(a) - int(a2), int(b) - int(b2)
        if (da, db) == (1, 0):
            pt += 1
        elif (da, db) == (0, 1):
            aegis += 1
        else:
            problems.append('holder %s: counts (%s, %s) -> (%s, %s), not one given back' % (holder, a, b, a2, b2))
    out['given_back'] = {'pt': pt, 'aegis': aegis}
    out['object_modules'] = [t[:220] for t in af._since(ctx, seq0) if OBJECT_MODULE.search(t)][:10]
    _knob(ctx, 'off')
    _mark(t0, out, 'done')
    if not pt or not aegis:
        problems.append('a PT and an Aegis teardown were both needed: %s (steps %s)' % (out['given_back'], out['steps']))
    problems += ['bad line: %s' % t for t in af._bad(af._since(ctx, seq0))]
    return _finish(ctx, 'a2-p4', out, problems)


def _pyra_keep(ctx, extra):
    fx._live_set(ctx, player_roster=fs._roster(cf.P1_LANE))
    orig = cf._diag_cycle
    cf._diag_cycle = lambda c, words: orig(c, words + extra if words.startswith('reload') else words)
    try:
        return cf.cf_pyra_keep(ctx)
    finally:
        cf._diag_cycle = orig


def a2_pyra_keep_a1(ctx):
    """integration-3's smoke2 crash row (head 79ba5ba9, the primary 02:20Z): cycle-fix's cf-pyra-keep with A1's window keep
    on for its forced Reload (`reload mii window`, what the A1 constant does there; the Aegis keep stays off, so the
    Pyra window is refused as it was there), on this stack (PT ON, K-strip ON, p4). P1's lane goes back to cycle-fix's
    first. The Reload out of the CPU's Pyra window runs state 2's bit-5 destroy of its holder (`0x651200-0x6512a0`)."""
    return _pyra_keep(ctx, ' window')


PYRA_BUILT_CPU = r'background built owner=1 group=\d+ ok=true reason=\S+ kinds=\[5a, 5b\]'
PYRA_READY_FIGHTERS = 10


def _fresh_cpu_pyra_group(ctx, out):
    """a2-pyra-keep-all's precondition: the CPU's Pyra group built fresh. A group an earlier row left in the CPU's cycle is
    reused, not built, and one the lane could not fit (Ready Fighters 8: the console lanes plan 7) stays cold, so the lane
    drops Pyra, a pause frees what parked, and the lane with Pyra is set again at Ready Fighters 10, the shipped value."""
    clean = hs.diag_mark(ctx)
    fx._live_set(ctx, cpu_roster=fs._roster([fs.GUNNER, fs.DIDDY]), cycle_preload_ready_fighters=fs.RF(PYRA_READY_FIGHTERS))
    time.sleep(2.0)
    _pause(ctx)
    time.sleep(2.0)
    out['preclean'] = [t[:200] for t in af._since(ctx, clean) if '] pause frees ' in t or '[Destroy] records' in t][:6]
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, cpu_roster=fs._roster([fs.GUNNER, PYRA, fs.DIDDY]), cycle_preload_ready_fighters=fs.RF(PYRA_READY_FIGHTERS))
    out['pyra_built'] = bool(hs.wait_diag(ctx, seq, PYRA_BUILT_CPU, 120))
    out['settle_fresh'] = fs._settled(ctx, seq, 90)
    if not out['pyra_built']:
        raise RuntimeError('a2-pyra-keep-all precondition: no fresh CPU Pyra group built at Ready Fighters %d (waited 120 s, '
                           'settle %s)' % (PYRA_READY_FIGHTERS, out['settle_fresh']))


def a2_pyra_keep_all(ctx):
    """The same with every switch on (the primary 03:00Z): the Aegis keep too (`reload mii window pt aegis strip`), so the
    CPU's Pyra window is kept as a group and its holder lives on past the Reload.

    Setup (a-full-3; cf-pyra-keep's assertions are unchanged, they run as written): in boot a2-f4acc30e-x1 the warm lap's
    first press onto Pyra found no group built (route `canonical`, "demand declined … no one-member group holds it"), the
    later presses went round the cycle, and the row ended on Mii Gunner ("the CPU never reached Pyra: 74", decimal for
    0x4a). So its warm lap starts only after the CPU's Pyra group is built fresh (`_fresh_cpu_pyra_group`; a precondition
    failure otherwise), and the CPU lands on Pyra by a direct pick of that group, as a2-aegis does for Player 1. The
    receipt also names the Pyra window's keep (`pyra_window_kept`, `partner_holder`) and the object-module lines; they are
    evidence, not a verdict."""
    fx._live_set(ctx, player_roster=fs._roster(cf.P1_LANE))
    seq0 = hs.diag_mark(ctx)
    prepared = {}
    orig_diag, orig_lap, orig_to, orig_finish = cf._diag_cycle, cf._lap, cf._to, cf._finish

    def diag(c, words):
        return orig_diag(c, words + ' window pt aegis strip' if words.startswith('reload') else words)

    def lap(c, out, tag, laps, gap, settle):
        if tag == 'warm' and not prepared:
            prepared['done'] = True
            _fresh_cpu_pyra_group(c, out)
        return orig_lap(c, out, tag, laps, gap, settle)

    def to(c, kind, cap=12):
        if kind != PYRA:
            return orig_to(c, kind, cap)
        return {'direct_pick': af._pick(c, 1, PYRA)[1]}

    def finish(c, name, out, problems):
        lines = af._since(c, seq0)
        out['pyra_window_kept'] = [t[:300] for t in lines if '[GroupSwap] window kept across reload owner=1' in t][:2]
        out['partner_holder'] = [t[:300] for t in lines if 'partner holder=' in t][:2]
        out['object_modules'] = [t[:220] for t in lines if OBJECT_MODULE.search(t)][:8]
        return orig_finish(c, name, out, problems)

    cf._diag_cycle, cf._lap, cf._to, cf._finish = diag, lap, to, finish
    try:
        return cf.cf_pyra_keep(ctx)
    finally:
        cf._diag_cycle, cf._lap, cf._to, cf._finish = orig_diag, orig_lap, orig_to, orig_finish


def a2_aegis_probe(ctx):
    """a0, diag only: what the Reload start does to an Aegis window's holder (AFULL2-RESEARCH.md "Aegis"). No keep."""
    t0, out = _begin(ctx)
    _knob(ctx, 'off')
    seq = hs.diag_mark(ctx)
    lane = [fs.PIKACHU, PYRA]
    fx._live_set(ctx, player_roster=fs._roster(lane), cycle_preload_ready_fighters=fs.RF(10))
    out['settle0'] = fs._settled(ctx, seq, 120)
    out['pyra_built'] = bool(hs.wait_diag(ctx, seq, r'background built owner=0 group=\d+ ok=true reason=\S+ kinds=\[5a, 5b\]', 90))
    _, out['to_pyra'] = af._pick(ctx, 0, PYRA)
    out['settle1'] = fs._settled(ctx, hs.diag_mark(ctx), 90)
    out['window'] = [hex(k) for k in (af._window(ctx, 0) or [])]
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
