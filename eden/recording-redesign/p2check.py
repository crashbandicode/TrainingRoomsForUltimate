"""rc.5 last re-acceptance (task 8): a quick check that Player 2 still drives the CPU and cycles it.

The route is the one the docs-GIF worker proved by hand on rc5-p2control-xps2 (2026-11-08): the native pause, CPU
Behavior (four rows down) set to its last value, Control; then Player 2's stick moves the CPU and Player 2's
ZL + D-pad Left (the CPU-cycle chord on the operating controller) changes the CPU's fighter. The shots are the check.
Run it last before acc-hook: the CPU stays under Player 2's control afterwards.
"""
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import steps  # noqa: E402

P2 = 1


def _p2_stick(ctx, x, seconds):
    ctx.call('input.stick', {'stick': 'left', 'player': P2, 'x': x, 'y': 0.0, 'lease_ms': int(seconds * 1000) + 300})
    time.sleep(seconds)


def p2_control(ctx):
    steps.settle(ctx)
    out = {'start': str(ctx.shot('start'))}
    steps.press(ctx, 'plus', 1.2)
    for _ in range(4):
        steps.press(ctx, 'down', 0.4)
    for _ in range(14):
        steps.press(ctx, 'right', 0.3)
    out['behavior'] = str(ctx.shot('pause-behavior-control'))
    steps.press(ctx, 'plus', 1.5)
    out['before_walk'] = str(ctx.shot('before-p2-walk'))
    _p2_stick(ctx, -1.0, 0.7)
    out['p2_walk'] = str(ctx.shot('p2-walk-left'))
    _p2_stick(ctx, 0.0, 0.5)
    ctx.call('input.button', {'button': 'zl', 'player': P2, 'state': 'press', 'duration_ms': 600, 'lease_ms': 1200})
    time.sleep(0.1)
    ctx.call('input.button', {'button': 'left', 'player': P2, 'state': 'tap', 'duration_ms': 150, 'lease_ms': 700})
    time.sleep(0.5)
    ctx.call('input.button', {'button': 'zl', 'player': P2, 'state': 'release', 'duration_ms': 1, 'lease_ms': 200})
    time.sleep(3.0)
    out['p2_cycle'] = str(ctx.shot('p2-cycle'))
    _p2_stick(ctx, 1.0, 0.7)
    out['p2_walk_after'] = str(ctx.shot('p2-walk-after-cycle'))
    _p2_stick(ctx, 0.0, 0.3)
    ctx.call('input.reset')
    return steps.save(ctx, 'p2-control', out)
