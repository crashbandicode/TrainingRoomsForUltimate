"""Training return/quit steps executed by the shared runner's context."""
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


def quit_warm_training(ctx):
    """The zero-damage HUD is stable when the stage camera changes after swaps.

    lr06 passed that gate but failed pause-open's grid-background start gate.
    Verify real native pause before reusing the library's Quit recordings.
    """
    ctx.gate('training-hud-unpaused')
    ctx.press('plus', 120)
    ctx.gate('pause')
    ctx.gate('native-pause')
    ctx.play('focus-native-quit-frames')
    ctx.gate('quit-focus')
    ctx.play('confirm-native-quit-frames')
    ctx.gate('lr06-clean-finalize-0')
    ctx.gate('lr08-player-ledger-empty')
    ctx.gate('lr08-cpu-native-retired')
    ctx.gate('lr08-source-retired')
    verify_cpu_finalize_trace(ctx)
    ctx.gate('ready')
    ctx.shot('warm-return-css')
    return {'native_pause_verified': True, 'clean_bank_owner': 0, 'native_retired_owner': 1, 'source_retired': True, 'end': 'css-return'}


"""Trace-backed observations; source-derived native ABI, actual point IDs."""
import re

INSTALL = 'main+0x653240'
FINALIZE = 'main+0x6524e0'


def _point(ctx, target):
    points = ctx.call('trace.list')['points']
    selected = [p for p in points if p['target'] == target]
    if len(selected) != 1 or selected[0]['state'] != 'armed':
        raise RuntimeError(f'trace point unresolved or no longer armed: {target}: {selected}')
    return str(selected[0]['id'])


def _capture_pattern(expression, value):
    # Captures evaluate an address; typed reads happen at that address.
    return (r'\{(?=[^{}]*"expression"\s*:\s*"' + re.escape(expression) + r'")'
            r'(?=[^{}]*"error"\s*:\s*"")(?=[^{}]*"value"\s*:\s*"' + str(value) + r'")[^{}]*\}')


def trace_pattern(point, captures):
    return (r'(?m)^(?=[^\n]*"event"\s*:\s*"entry")'
            r'(?=[^\n]*"point_id"\s*:\s*"' + re.escape(point) + r'")' +
            ''.join(r'(?=[^\n]*' + _capture_pattern(expr, value) + ')' for expr, value in captures) + r'[^\n]+')


def trace_wiring(ctx):
    if not ctx.instance.get('trace_jsonl'):
        raise RuntimeError('lifecycle did not expose trace_jsonl')
    ids = {target: _point(ctx, target) for target in (INSTALL, FINALIZE)}
    modules = ctx.call('modules.list')
    ctx.log({'trace_wiring': ids, 'trace_jsonl': ctx.instance['trace_jsonl'], 'modules': modules})
    return {'point_ids': ids, 'trace_jsonl': ctx.instance['trace_jsonl'], 'modules': modules}


def verify_cpu_install_trace(ctx):
    point = _point(ctx, INSTALL)
    ctx.wait_log('trace_jsonl', trace_pattern(point, [('x0+0xe0', 1), ('x1+0x18', 74), ('x2', 1)]), cap=15)
    ctx.log({'native_cpu_install_trace': {'point_id': point, 'owner': 1, 'incoming_kind': 74, 'reload_flag': 1}})
    return {'point_id': point, 'owner': 1, 'incoming_kind': 74, 'reload_flag': 1,
            'scope': 'fresh-row native descriptor install entry; not a construction-completion or bank_build assertion'}


def verify_cpu_finalize_trace(ctx):
    point = _point(ctx, FINALIZE)
    ctx.wait_log('trace_jsonl', trace_pattern(point, [('x0+0xe0', 1), ('x0+0x18', 74)]), cap=15)
    return {'point_id': point, 'owner': 1, 'active_kind': 74, 'scope': 'native finalize entry; clean outcomes checked separately'}


def finish_trace(ctx):
    ctx.call('trace.disarm')
    counters = ctx.call('trace.counters')
    ctx.log({'final_trace_counters': counters})
    return counters
