"""rc.14 step 2e follow-up (a1-enable) rows, 2026-11-14: A1 (the attached window kept across its role's own Reload) with the
diag knob on, after be759456 (Design A's cold marks survive the Reload), on the operator's cycle shape (cycle-fix's lanes,
artifacts/rc14-cycle-load/console-20261114) and the three rows a-full's boots could not prove
(artifacts/rc14-cycle-load/afull/HANDOFF.md, "For the Sonnet lane").

The knob (`diag-cycle.req`): `window` is A1 on with no forced Reload (this boot's baseline after setup); `reload window` sends
a cold press or a direct pick down today's Reload and keeps its window; `reload mii window` keeps a Mii window too.
Kinds: gna_p2_fs's constants (TR trio.rs). Helpers: cyclefix.py (cycle-fix) and afull.py (a-full) beside this file.

Rows (hooks; evidence is the diag log and the receipts):
- a1_setup: cyclefix.cf_setup (Ready Fighters 8, the console's lanes), then the knob `window`.
- a1_regress: a CPU window of two or more members, a forced Reload (a direct pick of Bowser) keeps it, the kept window warm-
  attached again, then three CPU laps. Pass: the keep is logged; when it left a cold fighter (`cold=true`) a `cold release`
  follows; every `demand` and `cold release` counts at most Ready Fighters + 1 bodies (`cold` exactly when over Ready
  Fighters); no `demand declined`, no Reload, canonical replacement or teardown in the laps; the status after the laps is
  at most Ready Fighters (boot af-49c90e9c-x1 af-attach: 10 bodies at Ready Fighters 8, the laps Reloaded).
- a1_kirby: r6 again, after the marks change: Kirby alone, kept as `[6]` by a forced Reload, the warm return at cell 0.
- a1_kirby_copy: r7 on a settled P1 lane (Ready Fighters 10 so every group is resident, the target Palutena's included): Kirby
  with a hat; the press takes the gated Reload (`[KirbyPark] fallback`), Kirby's window and the target's group go, the other
  Built groups are kept.
- a1_mii: Ready Fighters 10 (the Mii is not the cold fighter): the CPU's Mii Gunner window kept by `reload mii window`, then
  warm-attached again.
- a1_pair: Ready Fighters 10, the CPU's Solo Popo mark Off, so the CPU's Ice Climbers are the pair (Popo and Nana): the pair
  window kept, then its warm return.
- a1_restore: the knob off, the remembered values (and Solo Popo), Mario / Mario."""
import json
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
from gna import _begin, _mark, _shot, _finish, OFF  # noqa: E402

KNOB = 'window mii'   # A1 on and Mii groups kept: both switches as the build ships them
DEMAND = re.compile(r'\[GroupSwap\] demand owner=(\d) group=\d+ kinds=\[[^\]]*\] special=\S+ bodies=(\d+) ready_fighters=(\d+) '
                    r'cold=(true|false)')
RELEASE = re.compile(r'\[GroupSwap\] cold release owner=(\d) group=\d+ kinds=\[([^\]]*)\] state=\S+ bodies=(\d+) '
                     r'ready_fighters=(\d+)')
DECLINED = re.compile(r'\[GroupSwap\] demand declined owner=(\d) kind=(0x[0-9a-f]+): (.*)')
KEPT_FULL = re.compile(r'\[GroupSwap\] window kept across reload owner=(\d) group=(\d+) kinds=\[([^\]]*)\] .*?identity=(true|false) '
                       r'pair=(true|false) cold=(true|false)')
NOT_KEPT_WHY = re.compile(r'\[GroupSwap\] window across reload owner=(\d) .*not kept')
KEPT_GROUPS = re.compile(r'\[GroupSwap\] kept across reload owner=(\d) reason=.*? target=(0x[0-9a-f]+) groups=(.*)$')
FALLBACK = re.compile(r'\[GroupSwap\] fallback owner=(\d) group=\d+ reason=([^;]*)')
LEFT_TARGET = re.compile(r'\[GroupSwap\] left cycle owner=1 group=\d+ kinds=\[f\]')
KIRBY_FALLBACK = re.compile(r'\[KirbyPark\] fallback owner=0 target=(0x[0-9a-f]+) ')
POPO_CPU = 'solo_popo_cpu'


def _live(ctx, key):
    """A menu value as the live settings hold it: a diag reload does not rewrite settings.json (fx._live_set builds on the newer
    of it and diag-settings-last.json), so the row reads the same newer file."""
    root = Path(ctx.instance['settings']).parent
    base, last = root / 'settings.json', root / 'diag-settings-last.json'
    if last.is_file() and (not base.is_file() or last.stat().st_mtime > base.stat().st_mtime):
        base = last
    return (json.loads(base.read_text()).get('menu') or {}).get(key)


def _ready_fighters(ctx):
    """Ready Fighters as the plugin applied it last (`[CyclePreload] applied ... ready_fighters=N`)."""
    found = [int(m.group(1)) for _, _, t in hs.diag_lines(ctx) for m in [re.search(r'\[CyclePreload\] applied .*? ready_fighters=(\d+)', t)] if m]
    return found[-1] if found else None


SETS = re.compile(r'\[CopyPkg\] sync owner=(\d) reason=\S+ sets=(\[.*\]) acquired=')


def _sets(ctx):
    """The bodies each role holds, from the newest `[CopyPkg] sync` line of each (its sets, the window included), and their
    total: informational evidence of the resident count at a point of a row."""
    found = {}
    for _, _, t in hs.diag_lines(ctx):
        m = SETS.search(t)
        if m:
            found[int(m.group(1))] = [[k for k in grp.split(', ') if k and k != 'ffffffff'] for grp in re.findall(r'\[([0-9a-f, ]+)\]', m.group(2))]
    return {'sets': {str(k): v for k, v in sorted(found.items())}, 'total': sum(len(g) for v in found.values() for g in v)}


def _status(ctx):
    found = [m for _, _, t in hs.diag_lines(ctx) for m in [hs.STATUS_LINE.search(t)] if m]
    m = found[-1] if found else None
    if not m:
        return None
    return {'state': m.group(1), 'planned': int(m.group(2)), 'ready': int(m.group(3)), 'prefetched': int(m.group(4)),
            'pending': int(m.group(6)), 'text': m.group(8)}


def _bodies(lines, owner=None):
    """Every `demand` and `cold release` in `lines`: what it counted against Ready Fighters. A demand over Ready Fighters is
    a cold fighter (`cold=true`, at most one body over, `body_bank::cold_admitted`); a release names the bodies before it."""
    seen, bad = [], []
    for t in lines:
        m = DEMAND.search(t)
        if m and (owner is None or int(m.group(1)) == owner):
            b, r, cold = int(m.group(2)), int(m.group(3)), m.group(4) == 'true'
            seen.append(('demand', b, r, cold))
            if b > r + 1 or cold != (b > r):
                bad.append(t[:220])
            continue
        m = RELEASE.search(t)
        if m and (owner is None or int(m.group(1)) == owner):
            b, r = int(m.group(3)), int(m.group(4))
            seen.append(('release', b, r, True))
            if b > r + 1 or b <= r:
                bad.append(t[:220])
    return seen, bad


def _keep(ctx, out, owner, pick, words):
    """`words` on, a direct pick of `pick` (outside the cycle: today's Reload), the baseline knob back: the kept line, its
    fields and every line since (afull._keep_across, but the knob returns to `window`, not `off`)."""
    out['knob'] = cf._diag_cycle(ctx, words)
    seq, out['picked'] = af._pick(ctx, owner, pick)
    out['knob_back'] = cf._diag_cycle(ctx, KNOB)
    out['settle'] = fs._settled(ctx, hs.diag_mark(ctx), 60)
    lines = af._since(ctx, seq)
    out['kept'] = af._kept(lines, owner)
    out['kept_lines'] = [t[:1200] for t in lines if af.KEPT.search(t) and int(af.KEPT.search(t).group(1)) == owner][:2]
    fields = [KEPT_FULL.search(t) for t in out['kept_lines']]
    out['kept_fields'] = [{'identity': m.group(4) == 'true', 'pair': m.group(5) == 'true', 'cold': m.group(6) == 'true'}
                          for m in fields if m]
    out['not_kept'] = [t[:300] for t in lines if NOT_KEPT_WHY.search(t) and int(NOT_KEPT_WHY.search(t).group(1)) == owner]
    out['pins'] = [t[:200] for t in lines if (m := af.PINS.search(t)) and int(m.group(1)) == owner][-2:]
    out['teardown'] = [t[:200] for t in lines if (m := af.TEARDOWN.search(t)) and int(m.group(1)) == owner]
    out['bad'] = af._bad(lines)
    return seq, lines


def _land(ctx, owner, kind, cap=60):
    """`owner` on `kind` by a direct Character pick (not a cycle chord: this boot's CPU original was Pyra, a native group, whose
    canonical return skips the lane's Mii and pair presses); the pick's seq and its applied line."""
    if fs._live_kinds(ctx).get(owner) == kind:
        return {'already': True}
    seq, line = af._pick(ctx, owner, kind, cap)
    return {'applied': bool(line), 'line': (line or '')[:160]}


def _fallbacks(lines, owner):
    """`[GroupSwap] fallback` lines of `owner` (a warm exchange that failed verification and was rolled back): a switch back that
    needed one is no warm return, whatever the attach line says (a1-be759456-y2c's pair row first passed on the attach line alone)."""
    return [t[:300] for t in lines if (m := FALLBACK.search(t)) and int(m.group(1)) == owner]


def _returned(ctx, out, owner, since):
    """After a switch back to a kept window: the fallback lines since `since`, recorded and returned."""
    lines = af._since(ctx, since)
    out['fallbacks'] = _fallbacks(lines, owner)
    out['refused'] = [t[:200] for t in lines if '[Txn] outcome=Refused' in t][:3]
    return not out['fallbacks'] and not out['refused']


def a1_setup(ctx):
    """cycle-fix's setup (the console's lanes, Ready Fighters 8) with A1 off, then the baseline knob."""
    out = cf.cf_setup(ctx)
    cf._diag_cycle(ctx, KNOB)
    return out


LAP_PICKS = [fs.PICHU, fs.DIDDY, fs.KROOL]


def _pick_rec(ctx, kind, i, cap=40):
    """One direct CPU Character pick of a lane member (the group path of 2a, as a cycle press takes it) and its route, with
    cyclefix's diag patterns. A direct pick, not a chord: a CPU original that is a native group (this boot's Random coin was
    Pyra, 0x5a) takes every lap's return through a canonical Reload that destroys the bank (a1-be759456-y2's first regress)."""
    seq = hs.diag_mark(ctx)
    menu = fs._menu(ctx)
    fx._live_set(ctx, cpu_character=fs._onehot(menu, 'cpu_character', fs.CHOICES.index(kind)))
    line = fs._line(ctx, seq, fs._kind_line(1, kind), cap)
    time.sleep(1.5)
    lines = [t for _, _, t in hs.diag_since(ctx, seq)]
    rec = {'i': i, 'kind': hex(kind), 'outcome': 'Applied' if line else None, 'route': hs._route(1, lines)}
    for key, regex in cf.PATTERNS.items():
        hits = [t[:300] for t in lines if regex.search(t)]
        if hits:
            rec[key] = hits[:8]
    if rec.get('canonical'):
        rec['route'] = 'canonical'
    return rec


def a1_regress(ctx, rf=8, picks=None, name='a1-regress'):
    """cycle-fix's lanes at Ready Fighters {rf} (8 here, 7 in a1_regress7), the CPU on a lane member with a Ready bank (direct picks, see `_pick_rec`), a forced
    Reload (a direct pick of Bowser, outside the lane) keeps the CPU's window, the kept window warm-attached again, then three
    laps of direct picks over everything that stays."""
    t0, out = _begin(ctx)
    problems = []
    seq = hs.diag_mark(ctx)
    cf._diag_cycle(ctx, 'off')
    fx._live_set(ctx, **cf._values(rf))
    out['settle0'] = fs._settled(ctx, seq, 150)
    cf._diag_cycle(ctx, KNOB)
    seq0 = hs.diag_mark(ctx)
    picks = picks or LAP_PICKS
    ready = _ready_fighters(ctx) or (_live(ctx, 'cycle_preload_ready_fighters') or [0] * 11).index(1) + 2
    out['ready_fighters'] = ready
    out['to_bank'] = [_pick_rec(ctx, kind, i) for i, kind in enumerate(picks)]
    out['settle1'] = fs._settled(ctx, hs.diag_mark(ctx), 120)
    out['window_before'] = [hex(k) for k in (af._window(ctx, 1) or [])]
    out['status_before'] = _status(ctx)
    out['bodies_before'] = _sets(ctx)
    seq_keep, keep_lines = _keep(ctx, out, 1, fs.BOWSER, 'reload window mii')
    _shot(ctx, out, 'shot_bowser', 'a1-regress-bowser')
    kept = out['kept'][-1] if out['kept'] else None
    out['status_after_keep'] = _status(ctx)
    out['bodies_after_keep'] = _sets(ctx)
    # The kept window's first member picked back: a warm attach, which leaves the Reload's target (Design A's cold fighter).
    if kept:
        group, kinds = int(kept[0]), [int(k) for k in kept[1]]
        target = next((k for k in kinds if k not in (fs.GUNNER, 0x48, 0x49)), kinds[0])
        seq_back, out['picked_back'] = af._pick(ctx, 1, target)
        out['warm_back'] = af._warm_back(ctx, out, 1, group, kinds, seq_back) and _returned(ctx, out, 1, seq_back)
        time.sleep(2.0)
        out['status_after_back'] = _status(ctx)
        out['bodies_after_back'] = _sets(ctx)
    # Three laps of the lane's members (cycle-fix's judge: no Reload, teardown, canonical replacement or declined press).
    laps = []
    live = fs._live_kinds(ctx).get(1)
    order = picks[picks.index(live) + 1:] + picks[:picks.index(live) + 1] if live in picks else picks
    for lap in range(3):
        for kind in order:
            ctx.check_stop()
            fs._settled(ctx, hs.diag_mark(ctx), 20)
            laps.append(_pick_rec(ctx, kind, len(laps)))
    out['a1_laps'] = laps
    out['a1_laps_cold'], lap_problems = cf._judge(laps, len(picks))
    out['settle_end'] = fs._settled(ctx, hs.diag_mark(ctx), 90)
    out['status_end'] = _status(ctx)
    out['bodies_end'] = _sets(ctx)
    lines = af._since(ctx, seq0)
    after = af._since(ctx, seq_keep)
    out['bodies_seen'], bodies_bad = _bodies(after)
    # The forced Reload's own line (diag 
    # Only presses after the keep count; the forced Reload's own line ("today's Reload forced (diag)") is the row's setup.
    out['declined'] = [t[:240] for t in after if DECLINED.search(t) and 'forced (diag)' not in t]
    out['cold_release'] = [t[:240] for t in lines if RELEASE.search(t)][:6]
    out['left_target'] = [t[:240] for t in lines if LEFT_TARGET.search(t)][:4]
    out['bad_all'] = af._bad(lines)
    _mark(t0, out, 'done')
    if not out['picked']:
        problems.append('Bowser did not apply')
    if not kept:
        problems.append('the window was not kept: kept %s, not kept %s, window %s' % (out['kept'], out['not_kept'], out['window_before']))
    if not out['pins']:
        problems.append('no module pins re-requested at Ready')
    if kept and not out.get('warm_back'):
        problems.append('no warm attach of the kept window: attaches %s, fallbacks %s' % (out.get('attaches'), out.get('fallbacks')))
    if any(f['cold'] for f in out['kept_fields']) and not (out['cold_release'] or out['left_target']):
        problems.append('the keep left a cold fighter and neither a `cold release` nor its group leaving the cycle followed: status %s' % out['status_end'])
    if rf < 8 and not any(kind == 'demand' for kind, *_ in out['bodies_seen']):
        problems.append('no cold press in the laps at Ready Fighters %d: the lane fit' % rf)
    problems += ['over Ready Fighters + 1, or cold flag wrong: %s' % t for t in bodies_bad]
    problems += ['demand declined: %s' % t for t in out['declined']]
    problems += lap_problems
    problems += ['bad line: %s' % t for t in out['bad_all']]
    return _finish(ctx, name, out, problems)


def a1_regress7(ctx):
    """The same at Ready Fighters 7 with the Mii in the laps: two of the lane's fighters don't fit, so the laps need cold presses (`demand`) after the keep, which boot af-49c90e9c-x1 declined (no room)."""
    return a1_regress(ctx, rf=7, picks=[fs.GUNNER] + LAP_PICKS, name='a1-regress7')

def a1_kirby(ctx):
    """a-full's r6 with the knob left on (`window`) and the lane settled at the end."""
    t0, out = _begin(ctx)
    cf._diag_cycle(ctx, KNOB)
    # A two-member lane leaves a group slot for Kirby's window (af-5256aeab-y1: the console's three-member P1 lane fills them).
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, player_roster=fs._roster([fs.PIKACHU, fs.KIRBY]))
    out['settle0'] = fs._settled(ctx, seq, 120)
    out['to_kirby'] = af._to(ctx, 0, fs.KIRBY, len(cf.P1_LANE) + 2)
    out['window'] = [hex(k) for k in (af._window(ctx, 0) or [])]
    _keep(ctx, out, 0, fs.LINK, 'reload window mii')
    seq = hs.diag_mark(ctx)
    out['back'] = af._to(ctx, 0, fs.KIRBY, len(cf.P1_LANE) + 2)
    time.sleep(3.0)
    kept = out['kept'][-1] if out['kept'] else None
    warm = kept is not None and af._warm_back(ctx, out, 0, kept[0], kept[1], seq) and _returned(ctx, out, 0, seq)
    out['registry'] = [t[:200] for t in af._since(ctx, seq) if '[Registry] attach owner=0' in t][:3]
    _shot(ctx, out, 'shot_kirby', 'a1-kirby-back')
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, player_roster=fs._roster(cf.P1_LANE))
    out['settle_lane'] = fs._settled(ctx, seq, 120)
    _mark(t0, out, 'done')
    problems = []
    if out['window'] != [hex(fs.KIRBY)]:
        problems.append('Kirby is not alone in his window: %s' % out['window'])
    if not kept or kept[1] != [fs.KIRBY]:
        problems.append("Kirby's window was not kept: %s %s" % (out['kept'], out['not_kept']))
    if not warm:
        problems.append('no warm return to the kept Kirby: attaches %s, fallbacks %s' % (out.get('attaches'), out.get('fallbacks')))
    if not any(re.search(r'cells=\[\(\d+, 6\)', t) for t in out['registry']):
        problems.append('Kirby is not at cell 0 in the registry: %s' % out['registry'])
    problems += ['bad line: %s' % t for t in out['bad'] + out.get('bad_after', [])]
    return _finish(ctx, 'a1-kirby', out, problems)


def a1_kirby_copy(ctx):
    """r7: Ready Fighters 10 (every group resident, the target's too), P1 Pikachu > Kirby > Palutena settled, Kirby with a hat.
    The cycle press to Palutena asks the copy gate; the gated Reload destroys Kirby's window and Palutena's group; the other
    Built groups are kept."""
    t0, out = _begin(ctx)
    cf._diag_cycle(ctx, KNOB)
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, player_roster=fs._roster(cf.P1_LANE), player_kirby_hats=fs._roster(fs.HATS),
                 cycle_preload_ready_fighters=fs.RF(10))
    out['settle0'] = fs._settled(ctx, seq, 150)
    out['to_kirby'] = af._to(ctx, 0, fs.KIRBY, len(cf.P1_LANE) + 2)
    out['settle1'] = fs._settled(ctx, hs.diag_mark(ctx), 90)
    out['status_before'] = _status(ctx)
    out['hat'] = bool(fs._hat(ctx, t0, out, 'hat'))
    time.sleep(1.0)
    seq = hs.diag_mark(ctx)
    out['press'] = fs._brief(fs._press_cycle(ctx, 0))
    out['settle'] = fs._settled(ctx, hs.diag_mark(ctx), 60)
    lines = af._since(ctx, seq)
    out['gate'] = [t[:200] for t in lines if KIRBY_FALLBACK.search(t)][:2]
    out['kept_groups'] = [t[:240] for t in lines if KEPT_GROUPS.search(t) and KEPT_GROUPS.search(t).group(1) == '0'][:2]
    out['not_kept'] = [t[:300] for t in lines if NOT_KEPT_WHY.search(t) and NOT_KEPT_WHY.search(t).group(1) == '0'][:2]
    out['teardown'] = [t[:200] for t in lines if (m := af.TEARDOWN.search(t)) and int(m.group(1)) == 0]
    out['bad'] = af._bad(lines)
    _shot(ctx, out, 'shot_after', 'a1-kirby-copy-after')
    _mark(t0, out, 'done')
    problems = []
    if not out['hat']:
        problems.append('no hat confirmed: the copy gate was not set up')
    if not out['gate']:
        problems.append('the press was not refused by the copy gate (no [KirbyPark] fallback): route %s' % out['press'].get('route'))
    if not out['kept_groups']:
        problems.append('the gated Reload kept no group: %s' % out['teardown'])
    else:
        kept = KEPT_GROUPS.search(out['kept_groups'][0]).group(3)
        kinds = {int(k, 16) for k in re.findall(r'\b([0-9a-f]{1,2})\b', kept)}
        if fs.KIRBY in kinds or fs.PALUTENA in kinds:
            problems.append("a group of Kirby's window or the target was kept: %s" % kept)
        if not ({0x00, fs.PIKACHU} & kinds):
            problems.append('none of the other Built groups (Mario, Pikachu) was kept: %s' % kept)
    if out['press'].get('outcome') not in ('Applied', 'complete'):
        problems.append('the press did not apply: %s' % out['press'])
    problems += ['bad line: %s' % t for t in out['bad']]
    return _finish(ctx, 'a1-kirby-copy', out, problems)


def a1_mii(ctx):
    """Ready Fighters 10: the Mii Gunner window is not Design A's cold fighter, so A1 may keep it (`reload mii window`)."""
    t0, out = _begin(ctx)
    cf._diag_cycle(ctx, KNOB)
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, cycle_preload_ready_fighters=fs.RF(10))
    out['settle0'] = fs._settled(ctx, seq, 150)
    out['to_gunner'] = _land(ctx, 1, fs.GUNNER)
    out['settle1'] = fs._settled(ctx, hs.diag_mark(ctx), 90)
    out['window'] = [hex(k) for k in (af._window(ctx, 1) or [])]
    seq_keep, lines = _keep(ctx, out, 1, fs.BOWSER, 'reload mii window')
    seq = hs.diag_mark(ctx)
    out['back'] = _land(ctx, 1, fs.GUNNER)
    time.sleep(3.0)
    kept = out['kept'][-1] if out['kept'] else None
    warm = kept is not None and af._warm_back(ctx, out, 1, kept[0], kept[1], seq) and _returned(ctx, out, 1, seq)
    _shot(ctx, out, 'shot_gunner', 'a1-mii-back')
    _mark(t0, out, 'done')
    problems = []
    fields = out['kept_fields'][-1] if out['kept_fields'] else {}
    if not kept or kept[1] != [fs.GUNNER] or not fields.get('identity'):
        problems.append('the Mii window was not kept: %s %s' % (out['kept'], out['not_kept']))
    if fields.get('cold'):
        problems.append('the Mii was the cold fighter at Ready Fighters 10: %s' % out['kept_lines'])
    if not warm:
        problems.append('no warm attach of the kept Mii window: attaches %s, fallbacks %s' % (out.get('attaches'), out.get('fallbacks')))
    problems += ['bad line: %s' % t for t in out['bad'] + out.get('bad_after', [])]
    return _finish(ctx, 'a1-mii', out, problems)


def a1_pair(ctx):
    """Ready Fighters 10 and the CPU's Solo Popo Off: the CPU's Ice Climbers load as the pair, which is the window kept."""
    t0, out = _begin(ctx)
    cf._diag_cycle(ctx, KNOB)
    before = _live(ctx, POPO_CPU)
    out['solo_popo_cpu_before'] = before
    hs._remember(ctx, 'a1_solo_popo_cpu', before)
    lane = [af.POPO, fs.PICHU, fs.DIDDY]
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, cpu_roster=fs._roster(lane), cycle_preload_ready_fighters=fs.RF(10), **{POPO_CPU: OFF})
    out['solo_popo_cpu_now'] = _live(ctx, POPO_CPU)
    out['settle0'] = fs._settled(ctx, seq, 150)
    out['to_pair'] = _land(ctx, 1, af.POPO)
    out['settle1'] = fs._settled(ctx, hs.diag_mark(ctx), 90)
    out['window'] = [hex(k) for k in (af._window(ctx, 1) or [])]
    _shot(ctx, out, 'shot_pair_before', 'a1-pair-before')
    _keep(ctx, out, 1, fs.BOWSER, 'reload window mii')
    seq = hs.diag_mark(ctx)
    out['back'] = _land(ctx, 1, af.POPO)
    time.sleep(3.0)
    kept = out['kept'][-1] if out['kept'] else None
    warm = kept is not None and af._warm_back(ctx, out, 1, kept[0], kept[1], seq) and _returned(ctx, out, 1, seq)
    back_lines = af._since(ctx, seq)
    out['pair_lines'] = [t[:200] for t in back_lines if '[IcPair]' in t][:6]
    out['registry'] = [t[:200] for t in back_lines if '[Registry] attach owner=1' in t][:3]
    _shot(ctx, out, 'shot_pair_back', 'a1-pair-back')
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, cpu_roster=fs._roster(cf.CPU_LANE), cycle_preload_ready_fighters=fs.RF(8))
    out['settle_lane'] = fs._settled(ctx, seq, 120)
    _mark(t0, out, 'done')
    problems = []
    fields = out['kept_fields'][-1] if out['kept_fields'] else {}
    if out['solo_popo_cpu_now'] != OFF:
        problems.append('Solo Popo (CPU) is not Off: %s' % out['solo_popo_cpu_now'])
    if af.NANA not in [int(k, 16) for k in out['window']]:
        problems.append('the CPU window is not the pair (no Nana): %s' % out['window'])
    if not kept or [hex(k) for k in kept[1]] != out['window'] or af.POPO not in kept[1] or af.NANA not in kept[1]:
        problems.append('the Ice Climbers window was not kept: window %s, kept %s, not kept %s' % (out['window'], out['kept'], out['not_kept']))
    if not fields.get('pair'):
        problems.append('the kept window is not a pair (pair=false): %s' % out['kept_lines'])
    if not warm or not (out['back'].get('applied') or out['back'].get('already')):
        problems.append('no warm return to the kept pair: attaches %s, fallbacks %s, refused %s, back %s' % (out.get('attaches'), out.get('fallbacks'), out.get('refused'), out['back']))
    problems += ['bad line: %s' % t for t in out['bad'] + out.get('bad_after', [])]
    return _finish(ctx, 'a1-pair', out, problems)


def a1_restore(ctx):
    cf._diag_cycle(ctx, 'off')
    original = hs._state(ctx).get('a1_solo_popo_cpu')
    if original is not None:
        fx._live_set(ctx, **{POPO_CPU: original})
    return cf.cf_restore(ctx)

def a1_paircontrol(ctx):
    """Control for a1_pair: the same lane and settings, but the pair is a group the planner built in the background (never a window
    A1 kept). From Diddy Kong (a window the pair is not part of: with Pichu the window is the pair-lead window, and the pick is no exchange), a pick of Popo is the warm exchange into that pair group; it must attach it, come ready and not fall back. If this
    fails too, the pair group's warm attach is the defect, not A1's kept record."""
    t0, out = _begin(ctx)
    cf._diag_cycle(ctx, KNOB)
    lane = [af.POPO, fs.PICHU, fs.DIDDY]
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, cpu_roster=fs._roster(lane), cycle_preload_ready_fighters=fs.RF(10), **{POPO_CPU: OFF})
    out['settle0'] = fs._settled(ctx, seq, 150)
    out['to_first'] = _land(ctx, 1, fs.DIDDY)
    out['settle1'] = fs._settled(ctx, hs.diag_mark(ctx), 120)
    out['status'] = _status(ctx)
    seq = hs.diag_mark(ctx)
    out['to_pair'] = _land(ctx, 1, af.POPO)
    time.sleep(3.0)
    lines = af._since(ctx, seq)
    out['attaches'] = [(int(m.group(2)), af._hex_list(m.group(3))) for t in lines for m in [af.ATTACH.search(t)] if m and int(m.group(1)) == 1]
    out['route'] = hs._route(1, lines)
    out['bad'] = af._bad(lines)
    ok = _returned(ctx, out, 1, seq)
    _shot(ctx, out, 'shot_pair', 'a1-paircontrol-pair')
    fx._live_set(ctx, cpu_roster=fs._roster(cf.CPU_LANE), cycle_preload_ready_fighters=fs.RF(8))
    _mark(t0, out, 'done')
    problems = []
    if not out['to_pair'].get('applied'):
        problems.append('Popo (the pair) never came ready: %s' % out['to_pair'])
    if not any(set(kinds) >= {af.POPO, af.NANA} for _, kinds in out['attaches']):
        problems.append('no attach of a pair group (the pair was not a group): attaches %s, route %s' % (out['attaches'], out['route']))
    if not ok:
        problems.append('the pair group exchange fell back or was refused: %s %s' % (out['fallbacks'], out['refused']))
    problems += ['bad line: %s' % t for t in out['bad']]
    return _finish(ctx, 'a1-paircontrol', out, problems)

PINS_HELD = re.compile(r'\[GroupSwap\] module pins (released|held) role=1 ')
READY_BOWSER = re.compile(r'\[BodyBank\] ready owner=1 index=\d+ kind=0xf ')


def _unmapped_total(ctx):
    got = ctx.call('trace.unmapped_access', {})
    try:
        return int(got.get('total')) if isinstance(got, dict) else None
    except (TypeError, ValueError):
        return None


def a1_crash(ctx):
    """a1-be759456-y2's crash row with A1 and Mii groups kept ON: the CPU's cycle Mii Gunner > Pichu > Diddy Kong > King K. Rool >
    Pyra/Mythra (0x5a, a native group) at Ready Fighters 8, the CPU on its Pyra window with the lane's other members built as
    groups (`[[13], [27, 45], [4a]]` in the crash), a forced Reload (a direct pick of Bowser) out of that window, then the warm
    attach of the kept Mii Gunner group. Pass: no Unmapped access and no StallWatch in the row; the module pins go with
    `re_requested=true` only after Bowser's bank is Ready (a `held ... not the bank's tick` line may come first: the wait path);
    the Mii group is kept and attaches warm."""
    t0, out = _begin(ctx)
    lane = [fs.GUNNER, fs.PICHU, fs.DIDDY, fs.KROOL, cf.PYRA]
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, cpu_roster=fs._roster(lane), cycle_preload_ready_fighters=fs.RF(8))
    out['settle0'] = fs._settled(ctx, seq, 150)
    out['to_gunner'] = _land(ctx, 1, fs.GUNNER)
    out['settle1'] = fs._settled(ctx, hs.diag_mark(ctx), 120)
    out['to_pyra'] = _land(ctx, 1, cf.PYRA)
    out['settle2'] = fs._settled(ctx, hs.diag_mark(ctx), 90)
    out['on_pyra'] = fs._live_kinds(ctx).get(1)
    out['unmapped_before'] = _unmapped_total(ctx)
    out['knob'] = cf._diag_cycle(ctx, 'reload mii window')
    seq = hs.diag_mark(ctx)
    seq_pick, out['picked'] = af._pick(ctx, 1, fs.BOWSER)
    time.sleep(3.0)
    out['knob_back'] = cf._diag_cycle(ctx, KNOB)
    lines = [(s, t) for s, _, t in hs.diag_since(ctx, seq)]
    out['window_across'] = [t[:240] for _, t in lines if '[GroupSwap] window across reload owner=1' in t][:2]
    out['kept_groups'] = [t[:300] for _, t in lines if '[GroupSwap] kept across reload owner=1' in t][:2]
    out['pins'] = [(s, t[:300]) for s, t in lines if PINS_HELD.search(t)][:6]
    out['ready'] = [(s, t[:200]) for s, t in lines if READY_BOWSER.search(t)][:1]
    out['settle3'] = fs._settled(ctx, hs.diag_mark(ctx), 60)
    seq_back = hs.diag_mark(ctx)
    out['back'] = _land(ctx, 1, fs.GUNNER)
    time.sleep(3.0)
    back = af._since(ctx, seq_back)
    out['route'] = hs._route(1, back)
    out['fallbacks'] = _fallbacks(back, 1)
    out['stalls'] = [t[:200] for _, _, t in hs.diag_lines(ctx) if 'StallWatch' in t][-3:]
    out['unmapped_after'] = _unmapped_total(ctx)
    out['bad'] = af._bad(af._since(ctx, seq))
    fx._live_set(ctx, cpu_roster=fs._roster(cf.CPU_LANE))
    _shot(ctx, out, 'shot_after', 'a1-crash-after')
    _mark(t0, out, 'done')
    problems = []
    if out['on_pyra'] != cf.PYRA:
        problems.append('the CPU never reached its Pyra window: %s' % out['on_pyra'])
    if not out['picked']:
        problems.append('Bowser did not apply')
    if not any('4a' in t.split('groups=')[-1] for t in out['kept_groups']):
        problems.append('the Mii group was not kept across the Reload: %s' % out['kept_groups'])
    released = [(s, t) for s, t in out['pins'] if ' released ' in t]
    if any('re_requested=false' in t for _, t in released):
        problems.append('pins dropped with no re-request: %s' % released)
    if not released or not out['ready'] or released[0][0] < out['ready'][0][0]:
        problems.append('pins not released after the new bank was Ready: pins %s ready %s' % (released, out['ready']))
    if not out['back'].get('applied') or out['route'] != 'group-warm' or out['fallbacks']:
        problems.append('no warm attach of the kept Mii group: back %s route %s fallbacks %s' % (out['back'], out['route'], out['fallbacks']))
    if out['stalls']:
        problems.append('StallWatch: %s' % out['stalls'][-1])
    if out['unmapped_before'] is None or out['unmapped_after'] is None or out['unmapped_after'] != out['unmapped_before']:
        problems.append('Unmapped accesses during the row: %s -> %s' % (out['unmapped_before'], out['unmapped_after']))
    problems += ['bad line: %s' % t for t in out['bad']]
    return _finish(ctx, 'a1-crash', out, problems)

def a1_cold(ctx):
    """The path boot a1-be759456-y2c did not exercise (the primary's order, 00:3xZ): a cold press AFTER the keep, with no room.
    cycle-fix's k = 1 shape (Player 1 Mario > Pikachu, the CPU's four lane members, Ready Fighters 6): one CPU fighter does not
    fit. The CPU takes its lane by direct picks (so its original, Pyra or not, never enters), A1 keeps its window across a forced
    Reload (a direct pick of Bowser), the kept window attaches back, then three laps over the lane: the cold member is served by
    a `demand` (not declined for room), and the switch away frees it (`cold release`). Pass: the keep is logged, every demand and
    release counts at most Ready Fighters + 1 bodies, no declined press after the keep, no Reload or teardown in the laps, at
    least one cold press and one cold release, no StallWatch, no Unmapped access."""
    t0, out = _begin(ctx)
    problems = []
    cf._diag_cycle(ctx, KNOB)
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, **cf._k1_values(6))
    out['settle0'] = fs._settled(ctx, seq, 120)
    out['p1_lap'] = [fs._brief(fs._press_cycle(ctx, 0)) for _ in range(2)]
    out['settle1'] = fs._settled(ctx, hs.diag_mark(ctx), 120)
    picks = [fs.GUNNER] + LAP_PICKS
    out['ready_fighters'] = _ready_fighters(ctx)
    out['unmapped_before'] = _unmapped_total(ctx)
    out['bank'] = [_pick_rec(ctx, kind, i) for i, kind in enumerate(picks)]
    out['settle2'] = fs._settled(ctx, hs.diag_mark(ctx), 120)
    out['status_before'] = _status(ctx)
    seq_keep, _lines = _keep(ctx, out, 1, fs.BOWSER, 'reload window mii')
    kept = out['kept'][-1] if out['kept'] else None
    out['status_after_keep'] = _status(ctx)
    if kept:
        group, kinds = int(kept[0]), [int(k) for k in kept[1]]
        target = next((k for k in kinds if k != fs.GUNNER), kinds[0])
        seq_back, out['picked_back'] = af._pick(ctx, 1, target)
        out['warm_back'] = af._warm_back(ctx, out, 1, group, kinds, seq_back) and _returned(ctx, out, 1, seq_back)
        time.sleep(2.0)
    live = fs._live_kinds(ctx).get(1)
    order = picks[picks.index(live) + 1:] + picks[:picks.index(live) + 1] if live in picks else picks
    laps = []
    for lap in range(3):
        for kind in order:
            ctx.check_stop()
            fs._settled(ctx, hs.diag_mark(ctx), 20)
            laps.append(_pick_rec(ctx, kind, len(laps)))
    out['laps'] = laps
    out['laps_cold'], lap_problems = cf._judge(laps, len(picks))
    out['settle_end'] = fs._settled(ctx, hs.diag_mark(ctx), 90)
    out['status_end'] = _status(ctx)
    after = af._since(ctx, seq_keep)
    out['bodies_seen'], bodies_bad = _bodies(after)
    out['declined'] = [t[:240] for t in after if DECLINED.search(t) and 'forced (diag)' not in t]
    out['cold_release'] = [t[:240] for t in after if RELEASE.search(t)][:8]
    out['cold_demands'] = [t[:240] for t in after if (m := DEMAND.search(t)) and m.group(4) == 'true'][:8]
    out['stalls'] = [t[:200] for _, _, t in hs.diag_lines(ctx) if 'StallWatch' in t][-3:]
    out['unmapped_after'] = _unmapped_total(ctx)
    out['bad'] = af._bad(after)
    fx._live_set(ctx, **cf._values(8))
    _mark(t0, out, 'done')
    if not out['ready_fighters'] or out['ready_fighters'] > 6:
        problems.append('Ready Fighters is %s, not 6' % out['ready_fighters'])
    if not out['picked']:
        problems.append('Bowser did not apply')
    if not kept:
        problems.append('the window was not kept: kept %s, not kept %s' % (out['kept'], out['not_kept']))
    elif not out.get('warm_back'):
        problems.append('no warm attach of the kept window: attaches %s, fallbacks %s' % (out.get('attaches'), out.get('fallbacks')))
    if not out['cold_demands']:
        problems.append('no cold press (a demand over Ready Fighters) after the keep: the lane fit')
    if not out['cold_release']:
        problems.append('no `cold release` after the keep')
    problems += ['over Ready Fighters + 1, or cold flag wrong: %s' % t for t in bodies_bad]
    problems += ['demand declined: %s' % t for t in out['declined']]
    problems += lap_problems
    if out['stalls']:
        problems.append('StallWatch: %s' % out['stalls'][-1])
    if out['unmapped_before'] is None or out['unmapped_after'] is None or out['unmapped_after'] != out['unmapped_before']:
        problems.append('Unmapped accesses during the row: %s -> %s' % (out['unmapped_before'], out['unmapped_after']))
    problems += ['bad line: %s' % t for t in out['bad']]
    return _finish(ctx, 'a1-cold', out, problems)
