"""Training CSS return convergence, executed only by the shared Eden runner.

lr04's retained tile log proves two pickup endpoints: Roy27 and Sonic40.
Accept either fresh CPU-token endpoint, then use indexed steps to Lucas39.
No lifecycle, transport, generic row engine, or relaxed image tolerance here.
"""
import re


def _tile_step(ctx, direction, source, target):
    ctx.call('input.button', {'button': 'lstick', 'state': 'press', 'lease_ms': 2000})
    try:
        ctx.press(direction, 120)
    finally:
        ctx.call('input.button', {'button': 'lstick', 'state': 'release'})
    ctx.wait_log('tile', rf'step screen=css dir={direction} from={source} to={target} token=p2', cap=8)
    ctx.wait_log('tile', rf'settled screen=css at={target} hovered={target}\b', cap=8)


def return_cpu_to_lucas(ctx):
    ctx.gate('ready')
    ctx.play('lr04-return-cpu-pick')
    observed = ctx.wait_log('tile', r'step screen=css dir=right from=(?:26|39) to=(?:27|40) token=p2', cap=8)
    match = re.search(r'from=(\d+) to=(\d+) token=p2', observed)
    edge = tuple(map(int, match.groups()))
    if edge not in ((26, 27), (39, 40)):
        raise RuntimeError(f'unqualified CPU pickup edge: {edge}')
    ctx.wait_log('tile', rf'settled screen=css at={edge[1]} hovered={edge[1]}\b', cap=8)
    ctx.log({'return_cpu_pickup': {'edge': list(edge), 'token': 'p2'}})
    ctx.shot('return-cpu-held-' + str(edge[1]))
    if edge == (26, 27):
        ctx.gate('return-roy-held')
        _tile_step(ctx, 'down', 27, 40)
    else:
        ctx.gate('return-sonic-held')
    _tile_step(ctx, 'left', 40, 39)
    ctx.gate('lucas')
    ctx.play('lucas-pick')
    ctx.gate('ready')
    ctx.shot('returned-steve-lucas-ready')
    return {'pickup_edge': list(edge), 'final_tile': 39, 'token': 'p2', 'placed': True}
