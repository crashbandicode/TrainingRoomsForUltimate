"""rc.14 step 2e (a-full, A1) rows, 2026-11-14: the attached window kept across its role's own Reload, and Kirby laid out
alone (artifacts/rc14-cycle-load/AFULL-RESEARCH.md; the primary's decisions ~21:25Z). The diag build's
`diag-cycle.req` turns A1 on for a row (`window`, with `reload`: a cold press or a direct pick takes the Reload).

Setup is cycle-fix's (cyclefix.cf_setup: the console's lanes, Ready Fighters 8). Kinds: gna_p2_fs's constants (TR trio.rs).
Rows (hooks; evidence is the diag log and the receipts):
- af_keep (r1): a forced Reload (a direct CPU pick of Bowser) from a CPU window of two or more members keeps it:
  `[GroupSwap] window kept across reload owner=1` with that window's kinds, Bowser applied, the pins re-requested at Ready.
- af_attach (r2, r3): the kept window's first member picked back is a warm attach of that group; then two CPU laps:
  every press applied, no StallWatch or Unmapped, and any helper-seat reload of a kept kind while it is live recorded.
- af_mii (r4): the CPU's Mii Gunner window kept (`reload mii window`), then the Mii warm-attached again.
- af_kirby (r6): Player 1's Kirby, laid out alone, kept as `[6]` by a forced Reload (a direct pick of Link), then the
  warm return to Kirby at cell 0.
- af_kirby_copy (r7): Kirby with a hat (the copy gate refuses his park): the press takes the gated Reload, and only `[6]`
  goes; (a') keeps the other Built groups.
- af_pair (r5): the CPU lap with the Ice Climbers pair (Ready Fighters 10): the pair window kept, then its warm return.
- af_restore: the remembered values, Mario / Mario."""
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
from gna import _begin, _mark, _shot, _finish  # noqa: E402

POPO, NANA = 0x4b, 0x4c
KEPT = re.compile(r'\[GroupSwap\] window kept across reload owner=(\d) group=(\d+) kinds=\[([^\]]*)\] target=(0x[0-9a-f]+)')
NOT_KEPT = re.compile(r'\[GroupSwap\] window across reload owner=(\d) .*not kept \(([^)]*)\)')
ATTACH = re.compile(r'\[GroupSwap\] attach owner=(\d) group=(\d+) member=\d+ kinds=\[([^\]]*)\]')
STARTUP = re.compile(r'\[BodyBank\] start-up window owner=(\d) R=\S+ kinds=\[([^\]]*)\]')
BUILD = re.compile(r'\[BodyBank\] build owner=(\d) entry=\S+ members=Members \{ kinds: \[([^\]]*)\], count: (\d)')
PINS = re.compile(r'\[GroupSwap\] module pins released role=(\d) re_requested=true')
KIRBY_PARK = re.compile(r'\[KirbyPark\] fallback owner=0 ')
KEPT_GROUPS = re.compile(r'\[GroupSwap\] kept across reload owner=0 ')
TEARDOWN = re.compile(r'\[GroupSwap\] teardown owner=(\d) reason=\S+ groups=(\d+) bodies=(\d+)')
HELPER = re.compile(r'\[TrioBoundary\] reload entry=\S+ owner=([23]) .*?active=\(\d+, \[([0-9a-f]+)')
BAD = re.compile(r'StallWatch|Unmapped|panicked|INVARIANT VIOLATED')


def _hex_list(text):
    return [int(k, 16) for k in re.split(r',\s*', text.strip()) if k and k != 'ffffffff']


def _window(ctx, owner):
    """The role's attached window, from its newest attach, start-up window or Reload build line."""
    found = None
    for _, _, text in hs.diag_lines(ctx):
        m = ATTACH.search(text)
        if m and int(m.group(1)) == owner:
            found = _hex_list(m.group(3))
            continue
        m = STARTUP.search(text)
        if m and int(m.group(1)) == owner:
            found = _hex_list(m.group(2))
            continue
        m = BUILD.search(text)
        if m and int(m.group(1)) == owner:
            found = [int(k) for k in re.split(r',\s*', m.group(2))][:int(m.group(3))]
    return found


def _pick(ctx, owner, kind, cap=60):
    """A direct Character pick (phase 2a): the menu's one-hot value; waits for the role to show `kind`."""
    seq = hs.diag_mark(ctx)
    menu = fs._menu(ctx)
    key = 'player_character' if owner == 0 else 'cpu_character'
    fx._live_set(ctx, **{key: fs._onehot(menu, key, fs.CHOICES.index(kind))})
    line = fs._line(ctx, seq, fs._kind_line(owner, kind), cap)
    time.sleep(1.5)
    return seq, line


def _since(ctx, seq):
    return [t for _, _, t in hs.diag_since(ctx, seq)]


def _bad(lines):
    return [t[:200] for t in lines if BAD.search(t)]


def _to(ctx, owner, kind, presses):
    """Cycle presses of `owner` until it shows `kind` (at most `presses`); the presses' briefs."""
    briefs = []
    for _ in range(presses):
        if fs._live_kinds(ctx).get(owner) == kind:
            break
        briefs.append(fs._brief(fs._press_cycle(ctx, owner)))
        time.sleep(1.5)
    return briefs


def _kept(lines, owner):
    return [(int(m.group(2)), _hex_list(m.group(3))) for t in lines for m in [KEPT.search(t)] if m and int(m.group(1)) == owner]


def _keep_across(ctx, out, owner, pick, words='reload window'):
    """The knob on, the forced Reload (a direct pick of `pick`), the knob off: the kept lines and every line since."""
    out['knob'] = cf._diag_cycle(ctx, words)
    seq, out['picked'] = _pick(ctx, owner, pick)
    out['knob_off'] = cf._diag_cycle(ctx, 'off')
    out['settle'] = fs._settled(ctx, hs.diag_mark(ctx), 60)
    lines = _since(ctx, seq)
    out['kept'] = _kept(lines, owner)
    out['not_kept'] = [m.group(2) for t in lines for m in [NOT_KEPT.search(t)] if m and int(m.group(1)) == owner]
    out['pins'] = [t[:200] for t in lines if (m := PINS.search(t)) and int(m.group(1)) == owner][-2:]
    out['teardown'] = [t[:200] for t in lines if (m := TEARDOWN.search(t)) and int(m.group(1)) == owner]
    out['bad'] = _bad(lines)
    return lines


def _warm_back(ctx, out, owner, group, kinds, lines_from):
    """After the row's presses: the kept group's attach (warm, no Reload layout line) and no bad line."""
    lines = _since(ctx, lines_from)
    attaches = [(int(m.group(2)), _hex_list(m.group(3))) for t in lines for m in [ATTACH.search(t)] if m and int(m.group(1)) == owner]
    out['attaches'] = attaches[:6]
    out['bad_after'] = _bad(lines)
    return (group, kinds) in attaches


def af_keep(ctx):
    t0, out = _begin(ctx)
    cf._diag_cycle(ctx, 'off')
    briefs = []
    for _ in range(len(cf.CPU_LANE) + 1):
        window = _window(ctx, 1)
        if window and len(window) >= 2:
            break
        briefs.append(cf._press(ctx, 1.5))
    out['presses'] = briefs
    out['window_before'] = [hex(k) for k in window] if window else None
    _keep_across(ctx, out, 1, fs.BOWSER)
    hs._remember(ctx, 'af_kept_cpu', out['kept'][-1] if out['kept'] else None)
    _shot(ctx, out, 'shot_bowser', 'af-keep-bowser')
    _mark(t0, out, 'done')
    problems = []
    if not window or len(window) < 2:
        problems.append('no CPU window of two or more members to keep: %s' % out['window_before'])
    if not out['picked']:
        problems.append('Bowser did not apply')
    if not out['kept'] or sorted(out['kept'][-1][1]) != sorted(window or []):
        problems.append('the window was not kept: kept %s, not kept %s, window %s' % (out['kept'], out['not_kept'],
                                                                                  out['window_before']))
    if not out['pins']:
        problems.append('no module pins re-requested at Ready')
    problems += ['bad line: %s' % t for t in out['bad']]
    return _finish(ctx, 'af-keep', out, problems)


def af_attach(ctx):
    t0, out = _begin(ctx)
    kept = hs._state(ctx).get('af_kept_cpu')
    out['kept'] = kept
    problems = []
    if not kept:
        return _finish(ctx, 'af-attach', out, ['af-keep kept no window'])
    group, kinds = int(kept[0]), [int(k) for k in kept[1]]
    target = next((k for k in kinds if k not in (fs.GUNNER, 0x48, 0x49)), kinds[0])
    seq0 = hs.diag_mark(ctx)
    seq, out['picked'] = _pick(ctx, 1, target)
    warm = _warm_back(ctx, out, 1, group, kinds, seq)
    layout = [t[:160] for t in _since(ctx, seq) if '[BodyBank] layout owner=1' in t]
    _shot(ctx, out, 'shot_back', 'af-attach-back')
    # r3: two CPU laps over the kept group; a helper seat that reloads a kept kind while it is live is recorded.
    out['laps'] = [cf._press(ctx, 1.5) for _ in range(2 * (len(cf.CPU_LANE) + 1))]
    lines = _since(ctx, seq0)
    out['helper_reloads_of_kept'] = [t[:200] for t in lines if (m := HELPER.search(t)) and int(m.group(2), 16) in kinds][:6]
    out['bad'] = _bad(lines)
    _mark(t0, out, 'done')
    if not out['picked']:
        problems.append('%#x did not apply' % target)
    if not warm or layout:
        problems.append('the pick of %#x was no warm attach of group %d %s: attaches %s, layout %s' % (target, group, kinds,
                                                                                                   out['attaches'], layout))
    applied = [p for p in out['laps'] if p.get('outcome')]
    if len(applied) != len(out['laps']):
        problems.append('a lap press did not apply: %s' % [(p.get('kind'), p.get('outcome')) for p in out['laps']])
    problems += ['bad line: %s' % t for t in out['bad']]
    return _finish(ctx, 'af-attach', out, problems)


def af_mii(ctx):
    t0, out = _begin(ctx)
    cf._diag_cycle(ctx, 'off')
    out['to_gunner'] = _to(ctx, 1, fs.GUNNER, len(cf.CPU_LANE) + 1)
    lines = _keep_across(ctx, out, 1, fs.BOWSER, 'reload mii window')
    seq = hs.diag_mark(ctx)
    out['back'] = _to(ctx, 1, fs.GUNNER, len(cf.CPU_LANE) + 2)
    time.sleep(3.0)
    kept = out['kept'][-1] if out['kept'] else None
    warm = kept is not None and _warm_back(ctx, out, 1, kept[0], kept[1], seq)
    _shot(ctx, out, 'shot_gunner', 'af-mii-back')
    _mark(t0, out, 'done')
    problems = []
    if not kept or kept[1] != [fs.GUNNER] or not any('identity=true' in t for t in lines if 'window kept across' in t):
        problems.append('the Mii window was not kept: %s %s' % (out['kept'], out['not_kept']))
    if not warm:
        problems.append('no warm attach of the kept Mii window: %s' % out.get('attaches'))
    problems += ['bad line: %s' % t for t in out['bad'] + out.get('bad_after', [])]
    return _finish(ctx, 'af-mii', out, problems)


def af_kirby(ctx):
    t0, out = _begin(ctx)
    cf._diag_cycle(ctx, 'off')
    out['to_kirby'] = _to(ctx, 0, fs.KIRBY, len(cf.P1_LANE) + 2)
    out['window'] = [hex(k) for k in (_window(ctx, 0) or [])]
    _keep_across(ctx, out, 0, fs.LINK)
    seq = hs.diag_mark(ctx)
    out['back'] = _to(ctx, 0, fs.KIRBY, len(cf.P1_LANE) + 2)
    time.sleep(3.0)
    kept = out['kept'][-1] if out['kept'] else None
    warm = kept is not None and _warm_back(ctx, out, 0, kept[0], kept[1], seq)
    out['registry'] = [t[:200] for t in _since(ctx, seq) if '[Registry] attach owner=0' in t][:3]
    _shot(ctx, out, 'shot_kirby', 'af-kirby-back')
    _mark(t0, out, 'done')
    problems = []
    if out['window'] != [hex(fs.KIRBY)]:
        problems.append('Kirby is not alone in his window: %s' % out['window'])
    if not kept or kept[1] != [fs.KIRBY]:
        problems.append('Kirby\'s window was not kept: %s %s' % (out['kept'], out['not_kept']))
    if not warm:
        problems.append('no warm return to the kept Kirby: %s' % out.get('attaches'))
    problems += ['bad line: %s' % t for t in out['bad'] + out.get('bad_after', [])]
    return _finish(ctx, 'af-kirby', out, problems)


def af_kirby_copy(ctx):
    t0, out = _begin(ctx)
    cf._diag_cycle(ctx, 'off')
    fx._live_set(ctx, player_kirby_hats=fs._roster(fs.HATS))
    out['to_kirby'] = _to(ctx, 0, fs.KIRBY, len(cf.P1_LANE) + 2)
    out['hat'] = bool(fs._hat(ctx, t0, out, 'hat'))
    time.sleep(1.0)
    seq = hs.diag_mark(ctx)
    out['press'] = fs._brief(fs._press_cycle(ctx, 0))
    out['settle'] = fs._settled(ctx, hs.diag_mark(ctx), 60)
    lines = _since(ctx, seq)
    out['gate'] = [t[:200] for t in lines if KIRBY_PARK.search(t)][:2]
    out['kept_groups'] = [t[:240] for t in lines if KEPT_GROUPS.search(t)][:2]
    out['teardown'] = [t[:200] for t in lines if (m := TEARDOWN.search(t)) and int(m.group(1)) == 0]
    out['bad'] = _bad(lines)
    _shot(ctx, out, 'shot_after', 'af-kirby-copy-after')
    _mark(t0, out, 'done')
    problems = []
    if not out['hat']:
        problems.append('no hat confirmed: the copy gate was not set up')
    if not out['gate']:
        problems.append('the press was not refused by the copy gate (no [KirbyPark] fallback)')
    if not out['kept_groups']:
        problems.append('the gated Reload kept no group: %s' % out['teardown'])
    if out['press'].get('outcome') not in ('Applied', 'complete'):
        problems.append('the press did not apply: %s' % out['press'])
    problems += ['bad line: %s' % t for t in out['bad']]
    return _finish(ctx, 'af-kirby-copy', out, problems)


def af_pair(ctx):
    t0, out = _begin(ctx)
    cf._diag_cycle(ctx, 'off')
    lane = [POPO, fs.PICHU, fs.DIDDY]
    seq = hs.diag_mark(ctx)
    fx._live_set(ctx, cpu_roster=fs._roster(lane), cycle_preload_ready_fighters=fs.RF(10))
    out['settle0'] = fs._settled(ctx, seq, 120)
    out['to_pair'] = _to(ctx, 1, POPO, len(lane) + 2)
    out['settle1'] = fs._settled(ctx, hs.diag_mark(ctx), 90)
    out['window'] = [hex(k) for k in (_window(ctx, 1) or [])]
    _keep_across(ctx, out, 1, fs.BOWSER)
    seq = hs.diag_mark(ctx)
    out['back'] = _to(ctx, 1, POPO, len(lane) + 2)
    time.sleep(3.0)
    kept = out['kept'][-1] if out['kept'] else None
    warm = kept is not None and _warm_back(ctx, out, 1, kept[0], kept[1], seq)
    out['pair_lines'] = [t[:200] for t in _since(ctx, seq) if '[IcPair]' in t][:6]
    _shot(ctx, out, 'shot_pair', 'af-pair-back')
    fx._live_set(ctx, cpu_roster=fs._roster(cf.CPU_LANE), cycle_preload_ready_fighters=fs.RF(8))
    _mark(t0, out, 'done')
    problems = []
    if not kept or sorted(kept[1]) != [POPO, NANA]:
        problems.append('the pair window was not kept: window %s, kept %s, not kept %s' % (out['window'], out['kept'],
                                                                                          out['not_kept']))
    if not warm:
        problems.append('no warm return to the kept pair: %s' % out.get('attaches'))
    problems += ['bad line: %s' % t for t in out['bad'] + out.get('bad_after', [])]
    return _finish(ctx, 'af-pair', out, problems)


def af_restore(ctx):
    cf._diag_cycle(ctx, 'off')
    return cf.cf_restore(ctx)
