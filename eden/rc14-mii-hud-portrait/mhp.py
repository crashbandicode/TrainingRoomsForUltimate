"""rc.14 mii-hud-portrait (11-15): the Mii HUD portrait after a swap to Mii Gunner, with Hide CPU Off and On, for the CPU and
for Player 1, on three swap paths. Built on integration-3's eggs-plan hooks (fx, hotslots, gna, gna_p2_fs), which this file
sits beside.

The defect: with Hide CPU On, the CPU frame wrapper took TR's parked portrait helper (entry 2/3) as a CPU role and hid it
during its own bust render, so the bound portrait was the bust's grey background (Eden hidetag-2). Each case:
  1. the role on Pichu (Player 1 on Mario), Hide CPU as the case says;
  2. the swap to Mii Gunner by the path; the render's lines: `[MiiPortrait] owner=N issued render helper=H`, then
     `owner=N after render helper=H reason="render completed"`;
  3. stills of the HUD: while hidden (Hide On), then after Hide CPU Off (`shown`); with Hide Off one still;
  4. the portrait square's colourful-pixel fraction (`colourful`, calibrated on hidetag-2 h09: 0.0 grey; rc10 and int3
     smoke2 Mii Gunner busts 0.198/0.199; Richter 0.159);
  5. the helper's CPU-wrapper lines in the render window: `cpu_event role|hide entry=2..7`.
Paths: `warm` (a cycle press onto the background-built, kept Mii group: route group-warm), `cold` (Mii groups Off, so the
press loads the Mii into the entry), `reload` (a direct Character pick outside the lane). The route is recorded.

Mode (`MHP_MODE` file beside this one, or `repro` by default): `repro` records and never fails on the portrait (the build
before the fix); `proof` fails a case whose shown still is grey or whose render window has a helper role or hide line.
"""
import re
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import fx  # noqa: E402
import hotslots as hs  # noqa: E402
import gna_p2_fs as fs  # noqa: E402
from gna import _begin, _finish, ON, OFF, MARIO  # noqa: E402

PICHU, GUNNER, DIDDY = fs.PICHU, fs.GUNNER, fs.DIDDY
# The portrait squares as fractions of the frame, from 1280x674 shots: Player 1 (145..245, 552..655), CPU (400..500, ..).
BOX = {0: (145 / 1280, 552 / 674, 245 / 1280, 655 / 674), 1: (400 / 1280, 552 / 674, 500 / 1280, 655 / 674)}
GREY_BELOW = 0.05
HELPER = re.compile(r'cpu_event (role|hide) entry=([2-7]) ')


def _mode():
    path = HERE / 'MHP_MODE'
    return path.read_text().strip() if path.is_file() else 'repro'


def colourful(path, owner):
    """The fraction of the owner's portrait square whose pixels have a channel spread above 60 (a grey bust: 0)."""
    from PIL import Image
    im = Image.open(path).convert('RGB')
    w, h = im.size
    x0, y0, x1, y1 = BOX[owner]
    crop = im.crop((int(x0 * w), int(y0 * h), int(x1 * w), int(y1 * h)))
    px = crop.tobytes()
    n = len(px) // 3
    hits = sum(1 for i in range(0, len(px), 3) if max(px[i:i + 3]) - min(px[i:i + 3]) > 60)
    return round(hits / max(n, 1), 3)


def _shot(ctx, case, name, owner, wait=1.5):
    time.sleep(wait)
    path = str(ctx.shot('%s-%s' % (case['id'], name)))
    case['shots'][name] = {'path': path, 'colourful': colourful(path, owner)}
    return case['shots'][name]


def _hide(ctx, on):
    fx._live_set(ctx, hide_cpu=ON if on else OFF)
    time.sleep(1.0)


def _swap(ctx, owner, kind, path):
    """`owner` to `kind` by `path`; the press or pick receipt."""
    if path in ('warm', 'cold'):
        for _ in range(3):
            p = fs._press_cycle(ctx, owner)
            if p.get('kind') == kind or fs._live_kinds(ctx).get(owner) == kind:
                return fs._brief(p)
        raise RuntimeError('harness: owner %d never reached %#x by the cycle' % (owner, kind))
    seq = hs.diag_mark(ctx)
    menu = fs._menu(ctx)
    key = 'player_character' if owner == 0 else 'cpu_character'
    fx._live_set(ctx, **{key: fs._onehot(menu, key, fs.CHOICES.index(kind))})
    line = fs._line(ctx, seq, fs._kind_line(owner, kind), 60)
    if not line:
        raise RuntimeError('harness: owner %d pick of %#x not applied' % (owner, kind))
    return {'pick': hex(kind), 'route': hs._route(owner, [t for _, _, t in hs.diag_since(ctx, seq)]), 'line': line[:200]}


def _render(ctx, owner, seq, cap=40):
    """The render of this swap: its issue and terminal lines, and the helper role/hide lines between them."""
    done = hs.wait_diag(ctx, seq, r'\[MiiPortrait\] owner=%d after render helper=' % owner, cap)
    lines = hs.diag_since(ctx, seq)
    issued = [t[:200] for _, _, t in lines if '[MiiPortrait] owner=%d issued render' % owner in t]
    portrait = [t[:260] for _, _, t in lines if '[MiiPortrait] owner=%d ' % owner in t
                and any(k in t for k in ('issued render', 'bound', 'after render', 'refused', 'entered bust', 'native reset'))]
    helper = [t[:200] for _, _, t in lines if HELPER.search(t)]
    return {'done': bool(done), 'issued': issued[:3], 'portrait': portrait[:12], 'helper_cpu_lines': helper[:12]}


def _case(ctx, owner, path, hide, base):
    """One case: `owner` from `base` to Mii Gunner by `path`, Hide CPU `hide` during the swap."""
    case = {'id': '%s-%s-%s' % ('cpu' if owner else 'p1', path, 'on' if hide else 'off'), 'shots': {}}
    if fs._live_kinds(ctx).get(owner) != base:
        case['to_base'] = _swap(ctx, owner, base, 'reload' if path == 'reload' else 'warm')
        time.sleep(2.0)
    _hide(ctx, hide)
    seq = hs.diag_mark(ctx)
    case['swap'] = _swap(ctx, owner, GUNNER, path)
    case['render'] = _render(ctx, owner, seq)
    if hide:
        _shot(ctx, case, 'hidden', owner)
        _hide(ctx, False)
    _shot(ctx, case, 'shown', owner, 2.0)
    case['grey'] = case['shots']['shown']['colourful'] < GREY_BELOW
    return case


def _judge(case, problems):
    if not case['render']['done']:
        problems.append('%s: no `after render` line (no exact portrait job?)' % case['id'])
    if _mode() != 'proof':
        return
    if case['grey']:
        problems.append('%s: the shown portrait is grey (colourful=%s)' % (case['id'], case['shots']['shown']['colourful']))
    if case['render']['helper_cpu_lines']:
        problems.append('%s: the helper took a CPU role in the render: %s' % (case['id'], case['render']['helper_cpu_lines'][:2]))


def setup(ctx):
    """Player 1 Mario, CPU Pichu; lanes Mario > Mii Gunner and Pichu > Mii Gunner; Mii groups On; Hide CPU Off; settle (the
    background builds of both Mii groups)."""
    t0, out = _begin(ctx)
    out['picks'] = fs._picks(ctx, p1=MARIO, cpu=PICHU, hide_cpu=OFF, cycle_preload_mii_groups=ON,
                             player_roster=fs._roster([MARIO, GUNNER]), cpu_roster=fs._roster([PICHU, GUNNER]))
    out['settle'] = fs._settled(ctx, hs.diag_mark(ctx), 90)
    time.sleep(5.0)
    out['start'] = str(ctx.shot('mhp-start'))
    return _finish(ctx, 'mhp-setup', out, [] if out['picks']['now'].get(1) == hex(PICHU) else ['setup: CPU not Pichu'])


def _role_paths(ctx, owner, paths, name):
    t0, out = _begin(ctx)
    problems = []
    base = MARIO if owner == 0 else PICHU
    out['mode'] = _mode()
    out['cases'] = []
    lane = 'player_roster' if owner == 0 else 'cpu_roster'
    for path in paths:
        if path == 'cold':
            fx._live_set(ctx, cycle_preload_mii_groups=OFF)
            out['mii_groups_off'] = fs._settled(ctx, hs.diag_mark(ctx), 60)
        if path == 'reload':
            # Mii Gunner out of the lane: a direct pick of a lane member takes the group path, outside it today's Reload.
            fx._live_set(ctx, **{lane: fs._roster([base, DIDDY])})
            out['lane_without_gunner'] = fs._settled(ctx, hs.diag_mark(ctx), 60)
        for hide in (False, True):
            try:
                case = _case(ctx, owner, path, hide, base)
            except RuntimeError as error:
                problems.append('%s-%s: %s' % (path, hide, error))
                continue
            _judge(case, problems)
            out['cases'].append(case)
        if path == 'cold':
            fx._live_set(ctx, cycle_preload_mii_groups=ON)
            out['mii_groups_on'] = fs._settled(ctx, hs.diag_mark(ctx), 90)
    # back to the base fighter with Hide CPU Off, then the lane with Mii Gunner again
    _hide(ctx, False)
    if fs._live_kinds(ctx).get(owner) != base:
        out['back'] = _swap(ctx, owner, base, 'reload')
    if 'reload' in paths:
        fx._live_set(ctx, **{lane: fs._roster([base, GUNNER])})
        out['lane_restored'] = fs._settled(ctx, hs.diag_mark(ctx), 90)
    out['summary'] = {c['id']: {'route': (c['swap'] or {}).get('route'), 'colourful': c['shots']['shown']['colourful'],
                                'grey': c['grey'], 'helper_lines': len(c['render']['helper_cpu_lines'])}
                      for c in out['cases']}
    out['problems'] = problems
    return _finish(ctx, name, out, problems)


def cpu_paths(ctx):
    """CPU Pichu -> Mii Gunner: warm, cold, reload; each Hide Off then On (Player 1 stays Mario)."""
    return _role_paths(ctx, 1, ('warm', 'cold', 'reload'), 'mhp-cpu')


def p1_paths(ctx):
    """Player 1 Mario -> Mii Gunner: warm, cold, reload; each Hide Off then On (the CPU stays Pichu)."""
    return _role_paths(ctx, 0, ('warm', 'cold', 'reload'), 'mhp-p1')


def restore(ctx):
    """Hide CPU Off, Mii groups On, Mario / Mario (the run's starting fighters)."""
    t0, out = _begin(ctx)
    out['picks'] = fs._picks(ctx, p1=MARIO, cpu=MARIO, hide_cpu=OFF, cycle_preload_mii_groups=ON)
    return _finish(ctx, 'mhp-restore', out, [])
