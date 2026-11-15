"""rc.14 mii-hud-portrait (11-15): the Mii HUD portrait after a swap to Mii Gunner, with Hide CPU Off and On, for the CPU and
for Player 1, on three swap paths. Built on integration-3's eggs-plan hooks (fx, hotslots, gna, gna_p2_fs), which this file
sits beside.

The defect: with Hide CPU On, the CPU frame wrapper took TR's parked portrait helper (entry 2/3) as a CPU role and hid it
during its own bust render, so the bound portrait was the bust's grey background (Eden hidetag-2). Each case:
  1. the role on Pichu (Player 1 on Mario), Hide CPU as the case says;
  2. the swap to Mii Gunner by the path; the render's lines: `[MiiPortrait] owner=N issued render helper=H`, then
     `owner=N after render helper=H reason="render completed"`;
  3. one still of the HUD after Hide CPU Off (`shown`);
  4. the portrait square's busy-pixel fraction (`busy`: an empty bust about 0.03 grey or 0.12 red, a fighter 0.47 and up);
  5. the helper's CPU-wrapper lines in the render window: `cpu_event role|hide entry=2..7`.
Paths: `warm` (a cycle press onto the background-built, kept Mii group: route group-warm) and `reload` (Group Swap Off,
so a direct pick loads the Mii into the entry: a cold Reload; Mii groups Off only drops the Mii from the cycle). The route is
recorded.

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
# The inner portrait squares as fractions of the frame, from 1280x674 shots: Player 1 (160..230, 565..640), CPU (415..485, ..).
BOX = {0: (160 / 1280, 565 / 674, 230 / 1280, 640 / 674), 1: (415 / 1280, 565 / 674, 485 / 1280, 640 / 674)}
EMPTY_BELOW = 0.3
HELPER = re.compile(r'cpu_event (role|hide) entry=([2-7]) ')


def _mode():
    path = HERE / 'MHP_MODE'
    return path.read_text().strip() if path.is_file() else 'repro'


def busy(path, owner):
    """The fraction of the owner's inner portrait square whose pixels differ from its median colour by more than 90 (summed
    over R, G, B). mhp-repro-1 (XPS): an empty bust 0.032 (CPU, grey) and 0.122 (Player 1, red); Mii Gunner 0.797/0.826,
    Mario 0.695, Pichu 0.472; hidetag-2 h09 0.032, rc10 cov-replay-window 0.798."""
    import statistics
    from PIL import Image
    im = Image.open(path).convert('RGB')
    w, h = im.size
    x0, y0, x1, y1 = BOX[owner]
    px = im.crop((int(x0 * w), int(y0 * h), int(x1 * w), int(y1 * h))).tobytes()
    trip = [px[i:i + 3] for i in range(0, len(px), 3)]
    med = [statistics.median(t[c] for t in trip) for c in range(3)]
    far = sum(1 for t in trip if sum(abs(t[c] - med[c]) for c in range(3)) > 90)
    return round(far / max(len(trip), 1), 3)

def _shot(ctx, case, name, owner, wait=1.5):
    time.sleep(wait)
    path = str(ctx.shot('%s-%s' % (case['id'], name)))
    case['shots'][name] = {'path': path, 'busy': busy(path, owner)}
    return case['shots'][name]


def _live(ctx, key):
    """A menu value as the live settings hold it (a1._live): a diag reload does not rewrite settings.json, so the newer of it
    and diag-settings-last.json."""
    import json
    root = Path(ctx.instance['settings']).parent
    base, last = root / 'settings.json', root / 'diag-settings-last.json'
    if last.is_file() and (not base.is_file() or last.stat().st_mtime > base.stat().st_mtime):
        base = last
    return (json.loads(base.read_text()).get('menu') or {}).get(key)


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


def _to_base(ctx, owner, base):
    """Back to the base fighter. mhp-repro-1: a pick of the value the menu already holds loads nothing ("pick of 0x4a not
    applied"), and the live-kind parse can lag a Reload; so a direct pick when the menu's Character is not on the base,
    else a cycle press (the lane has two members) when the live fighter is not the base."""
    key = 'player_character' if owner == 0 else 'cpu_character'
    now = _live(ctx, key)
    if now != fs._onehot({key: now}, key, fs.CHOICES.index(base)):
        done = _swap(ctx, owner, base, 'reload')
    elif fs._live_kinds(ctx).get(owner) != base:
        done = _swap(ctx, owner, base, 'warm')
    else:
        return None
    time.sleep(2.0)
    return done


def _case(ctx, owner, path, hide, base):
    """One case: `owner` from `base` to Mii Gunner by `path`, Hide CPU `hide` during the swap."""
    case = {'id': '%s-%s-%s' % ('cpu' if owner else 'p1', path, 'on' if hide else 'off'), 'shots': {}}
    case['to_base'] = _to_base(ctx, owner, base)
    _hide(ctx, hide)
    seq = hs.diag_mark(ctx)
    case['swap'] = _swap(ctx, owner, GUNNER, path)
    case['render'] = _render(ctx, owner, seq)
    if hide:
        # one still per case (the coordinator, 11-15): only after Off, where the portrait shows
        _hide(ctx, False)
    _shot(ctx, case, 'shown', owner, 2.0)
    case['grey'] = case['shots']['shown']['busy'] < EMPTY_BELOW
    return case


def _judge(case, problems):
    if not case['render']['done']:
        problems.append('%s: no `after render` line (no exact portrait job?)' % case['id'])
    if _mode() != 'proof':
        return
    if case['grey']:
        problems.append('%s: the shown portrait is an empty bust (busy=%s)' % (case['id'], case['shots']['shown']['busy']))
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
    for path in paths:
        if path == 'reload':
            # mhp-repro-1: with Mii groups Off the cycle skips the Mii, and a direct pick still attached the kept group.
            # Group Swap Off: no group or kept window, so the press loads Mii Gunner into the entry (route `reload`).
            fx._live_set(ctx, cycle_preload_group_swap=OFF)
            out['group_swap_off'] = fs._settled(ctx, hs.diag_mark(ctx), 60)
        for hide in (False, True):
            try:
                case = _case(ctx, owner, path, hide, base)
            except RuntimeError as error:
                problems.append('%s-%s: %s' % (path, hide, error))
                continue
            _judge(case, problems)
            out['cases'].append(case)
    # back to the base fighter with Hide CPU Off, then Group Swap On again
    _hide(ctx, False)
    try:
        out['back'] = _to_base(ctx, owner, base)
    except RuntimeError as error:
        problems.append('back: %s' % error)
    if 'reload' in paths:
        fx._live_set(ctx, cycle_preload_group_swap=ON)
        out['group_swap_on'] = fs._settled(ctx, hs.diag_mark(ctx), 90)
    out['summary'] = {c['id']: {'route': (c['swap'] or {}).get('route'), 'busy': c['shots']['shown']['busy'],
                                'grey': c['grey'], 'helper_lines': len(c['render']['helper_cpu_lines'])}
                      for c in out['cases']}
    out['problems'] = problems
    return _finish(ctx, name, out, problems)


def cpu_paths(ctx):
    """CPU Pichu -> Mii Gunner: warm, reload; each Hide Off then On (Player 1 stays Mario)."""
    return _role_paths(ctx, 1, ('warm', 'reload'), 'mhp-cpu')


def p1_paths(ctx):
    """Player 1 Mario -> Mii Gunner: warm, reload; each Hide Off then On (the CPU stays Pichu)."""
    return _role_paths(ctx, 0, ('warm', 'reload'), 'mhp-p1')


# The proof boot orders the warm rows of both roles before the Reload rows (Group Swap Off then On): mhp-repro-1's warm
# presses missed after a Group Swap round trip.
def cpu_warm(ctx):
    return _role_paths(ctx, 1, ('warm',), 'mhp-cpu-warm')


def p1_warm(ctx):
    return _role_paths(ctx, 0, ('warm',), 'mhp-p1-warm')


def cpu_reload(ctx):
    return _role_paths(ctx, 1, ('reload',), 'mhp-cpu-reload')


def p1_reload(ctx):
    return _role_paths(ctx, 0, ('reload',), 'mhp-p1-reload')


def restore(ctx):
    """Hide CPU Off, Mii groups On, Mario / Mario (the run's starting fighters)."""
    t0, out = _begin(ctx)
    out['picks'] = fs._picks(ctx, p1=MARIO, cpu=MARIO, hide_cpu=OFF, cycle_preload_mii_groups=ON)
    return _finish(ctx, 'mhp-restore', out, [])
