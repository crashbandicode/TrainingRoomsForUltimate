"""rc.14 hide-cpu-tag (11-15): a hidden CPU's player tag and HUD panel go and come back with its body. Wraps the
integration hooks that already drive Hide CPU (hidecpu.h_shots) and the practice game's request (eggshide.run), and gates
each on the diag build's own lines:
  cpu_event hide entry=1 partner=false hidden=Some(true) tag=Some(false) ...   (the tag flags written off)
  hide_cpu panel entry=1 visible=false found=true                              (the CPU's info_melee part hidden)
  ... hidden=Some(false) tag=Some(true) / panel entry=1 visible=true found=true (both back on the show)
Player 1's part is never touched (no `panel entry=0` line). Stills are the visual evidence (the primary reads them).
"""
import re
import time

import eggshide
import hidecpu
import hotslots as hs
from gna import _begin, _finish

HIDE = r'cpu_event hide entry=1 partner=false hidden=Some\(true\) tag=Some\(false\)'
SHOW = r'cpu_event hide entry=1 partner=false hidden=Some\(false\) tag=Some\(true\)'
PANEL_OFF = r'hide_cpu panel entry=1 visible=false found=true'
PANEL_ON = r'hide_cpu panel entry=1 visible=true found=true'
P1_PANEL = r'hide_cpu panel entry=0 '


def _lines(ctx, mark):
    return [(s, text) for s, _, text in hs.diag_since(ctx, mark)]


def _check(lines, problems, label, sessions):
    """Each hide session: a tag-off and a panel-off line, then (before the next session) a tag-on and a panel-on line.
    `sessions` is how many hide/show pairs the step played."""
    def seqs(pattern):
        regex = re.compile(pattern)
        return [s for s, text in lines if regex.search(text)]
    got = {name: seqs(p) for name, p in (('hide', HIDE), ('show', SHOW), ('panel_off', PANEL_OFF), ('panel_on', PANEL_ON),
                                         ('p1_panel', P1_PANEL))}
    for name in ('hide', 'show', 'panel_off', 'panel_on'):
        if len(got[name]) < sessions:
            problems.append('%s: %d %s line(s), want %d' % (label, len(got[name]), name, sessions))
    if got['p1_panel']:
        problems.append("%s: Player 1's panel was written (%d lines)" % (label, len(got['p1_panel'])))
    # No flash: between a panel-off and the next tag-on (the show), no panel-on line.
    for off in got['panel_off']:
        show = min([s for s in got['show'] if s > off], default=None)
        flashes = [s for s in got['panel_on'] if off < s and (show is None or s < show)]
        if flashes:
            problems.append('%s: the panel came back while hidden (seq %s)' % (label, flashes[:3]))
    return {name: v[:12] for name, v in got.items()}


def tag_row(ctx):
    """Hide CPU On / L+R+A / Cycle CPU while hidden / Off (hidecpu.h_shots' stills), then the line gates."""
    t0, out = _begin(ctx)
    mark = hs.diag_mark(ctx)
    out['shots'] = hidecpu.h_shots(ctx)
    time.sleep(1.0)
    problems = []
    # h_shots hides twice (the walk/jab pass, then the shield pass) and shows twice.
    out['lines'] = _check(_lines(ctx, mark), problems, 'tag_row', 2)
    out['problems'] = problems
    return _finish(ctx, 'hidetag-row', out, problems)


def eggs_row(ctx):
    """Eggs-ercise with CPU during a game on Hide (eggshide.run's H1..H5 stills), then the line gates."""
    t0, out = _begin(ctx)
    mark = hs.diag_mark(ctx)
    out['run'] = eggshide.run(ctx)
    eggshide.restore(ctx)
    time.sleep(2.0)
    out['restored'] = str(ctx.shot('hidetag-eggs-restored'))
    problems = []
    # H1 (Hide) and H5 (Hide CPU On with Target CPU) hide; H2 (Eggs-ercise Off) and the restore (Hide CPU Off) show.
    out['lines'] = _check(_lines(ctx, mark), problems, 'eggs_row', 2)
    out['problems'] = problems
    return _finish(ctx, 'hidetag-eggs', out, problems)
