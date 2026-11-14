"""Break the Targets rows that btt-4's boot did not run (integration-3, rc.14 final pass; the primary's 18:0xZ list from btt-4's
final report), built on btt_rows.py and A-3's eggsrows.py fighter switch. btt-5 (2026-11-14): saved layouts are files
(layouts/, one per layout, the hot five per stage), so `saves` and `stages` are btt_rows.py's x_saves and x_stages.

  btt-X-ice    (a) Ice Climbers as Player 1: the pair on screen, Placing, three targets placed, X, a run that breaks all three and
               ends with the Done notice. Ice Climbers' second climber also passes the mode's step and must not tick the clock twice:
               the sampled Running clock never advances by more than a sampling interval between two lines (btt_rows.SAMPLE_HUNDREDTHS).
  btt-X-saves  (b) repeated saves and the chords on the hot five (btt_rows.x_saves).
  btt-X-stages saving and loading on two stages (btt_rows.x_stages).

(c) Break the Targets in the waiting room needs the LDN pair: it is a row for Sol's run, not this boot.
"""
import sys
import time
from pathlib import Path

if str(Path(__file__).resolve().parent) not in sys.path:
    sys.path.insert(0, str(Path(__file__).resolve().parent))

import btt_rows as br  # noqa: E402
import eggsrows as er  # noqa: E402
import fx  # noqa: E402
import hotslots as hs  # noqa: E402


def saves(ctx, key='bf'):
    """Row (b), btt-5: btt_rows.x_saves."""
    return br.x_saves(ctx, key)


def stages(ctx, first='bf', second='bbf'):
    """The operator's question (save and load on different stages), btt-5: btt_rows.x_stages."""
    return br.x_stages(ctx, first, second)

def ice(ctx):
    """Row (a): Ice Climbers as Player 1 through Placing and a three-target run."""
    out = {'problems': []}
    # The Player cycle's Solo Popo mark Off ([On, Off] order: Off is [0, 1], as A-3's seed fix writes it), so a pick of Ice
    # Climbers brings the pair (B-2's fixture note, A-3's eggs15).
    fx._live_set(ctx, solo_popo_player_cycle=[0, 1])
    if not er._fighter(ctx, er.ICS, out, 'ics_hud'):
        raise RuntimeError('Player 1 did not become Ice Climbers: %s' % out)
    out['menu'] = {k: er._menu(ctx).get(k) for k in ('solo_popo_player_cycle', 'solo_popo_original', 'trio_practice')}
    st = br.to_placing(ctx, out, 'ice')
    if st['n']:
        st = br.clear_layout(ctx)
    out['shot_placing_empty'] = str(ctx.shot('btt-X-ice-placing'))
    placed = br.put_layout(ctx, br.latest(ctx), out)
    out['shot_placed'] = str(ctx.shot('btt-X-ice-placed'))
    targets = hs._state(ctx).get('btt_layout')
    mark = hs.diag_mark(ctx)
    ctx.play('x-press')
    ready = br.wait_state(ctx, mark, lambda s: s['phase'] == 'Ready' and s['n'] == 3, 8, 'Ready with the three targets')
    out['ready'] = ready['text']
    time.sleep(1.0)
    out['shot_ready'] = str(ctx.shot('btt-X-ice-ready'))
    done, attempts, run_mark = br.run_targets(ctx, targets)
    found = br._DONE.search(done['text'])
    if not found or found.group(1) != '3':
        out['problems'].append('Done text %r' % done['text'])
    out['done'], out['attempts'] = done['text'], attempts
    out['shot_done'] = str(ctx.shot('btt-X-ice-done'))
    states = [s for _, s in br.since(ctx, run_mark)]
    clocks = [br.clock_of(s['text']) for s in states if s['phase'] == 'Running' and br.clock_of(s['text']) is not None]
    deltas = [b - a for a, b in zip(clocks, clocks[1:])]
    out['clock_samples'], out['clock_deltas'] = len(clocks), deltas[:40]
    # One sampling interval is 17 hundredths (ten ticks); both climbers ticking would show about 33 between every two lines. The
    # median, so one line the log dropped (a delta of 34) does not fail the row.
    median = sorted(deltas)[len(deltas) // 2] if deltas else None
    out['clock_median'] = median
    if not clocks or median is None or median > br.SAMPLE_HUNDREDTHS + 3:
        out['problems'].append('the median clock advance between two Running lines is %s, more than one sampling interval (%d): %s'
                               % (median, br.SAMPLE_HUNDREDTHS, deltas))
    for why in br.clock_stop_problems(states, False):
        out['problems'].append('clock: %s' % why)
    counts = [br.count_of(s['text'])[0] for s in states if s['phase'] == 'Running' and br.count_of(s['text'])]
    out['counts'] = counts
    if counts != sorted(counts):
        out['problems'].append('the count went backwards: %s' % counts)
    br.save(ctx, 'x_ice', out)
    br.settle(ctx)
    if out['problems']:
        raise RuntimeError('; '.join(out['problems']))
    return {'done': done['text'], 'attempts': len(attempts)}
