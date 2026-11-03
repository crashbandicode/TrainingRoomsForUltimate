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

"""RC-BUSY / pending-build scene-end observations in the shared runner."""
import importlib.util
import time
from pathlib import Path


def _load(name):
    spec=importlib.util.spec_from_file_location(name,Path(__file__).with_name(name+'.py'))
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    return module


class PendingSession:
    def __init__(self,ctx):self.ctx=ctx
    def clock(self):return time.monotonic()
    def sleep(self,seconds):self.ctx.check_stop();time.sleep(seconds)
    def press(self,chord,hold):
        if chord!='zl,right':raise ValueError(chord)
        self.ctx.play('lr10-p1-short' if hold==40 else 'p1-next-frames')


def _pending_tools(ctx):
    gates=_load('rc_roster_segment');strict=_load('strict_predicates')
    path=Path(ctx.instance['diag_log'])
    def read():
        # Both generations; sequence identity supplies freshness across rotation.
        lines=[]
        for p in (path.with_name('practice-diagnostics.previous.log'),path):
            if p.exists():lines.extend(p.read_text(errors='replace').splitlines())
        return lines
    def capture(_session,_review,name,expected):
        ctx.log({'checkpoint_expectation':expected,'name':name})
        return ctx.shot(name)
    gates._visual=capture
    return gates,strict,PendingSession(ctx),read


def pending_prefix(ctx,first=False):
    ctx.gate('training-hud-unpaused')
    if first:
        ctx.play('p1-next-frames');ctx.gate('p1-b')
    ctx.play('p1-next-frames');ctx.gate('p1-1f')
    ctx.shot('pending-prefix-darkpit')
    return {'target':31,'first':first}


def _quit_pending(ctx):
    # No HUD/image before the pause press: preserve the observed pending window.
    ctx.press('plus',40)
    ctx.gate('pause');ctx.gate('native-pause')
    ctx.play('focus-native-quit-frames');ctx.gate('quit-focus')
    ctx.play('confirm-native-quit-frames')
    ctx.gate('ready')


def pending_quit(ctx,index):
    gates,strict,session,read=_pending_tools(ctx)
    def reenter(_session):
        return_cpu_to_lucas(ctx)
        ctx.play('enter-training');ctx.gate('training-hud-unpaused')
    try:
        result=gates.interrupted_scene(session,read,strict,None,0,0x2e,
            lambda _session:_quit_pending(ctx),reenter,0x0b,index)
        ctx.gate('training-hud-unpaused')
        result['row_acceptance']='pending-build teardown observed' if result['drained'] else 'OPEN: no native construction-drained evidence'
        ctx.log({'pending_quit_observation':result})
        return result
    finally:
        ctx.log({'trace_checkpoint':ctx.call('trace.counters')})


def pending_busy(ctx):
    gates,strict,session,read=_pending_tools(ctx)
    since=gates.mark(read)
    try:
        result=gates.busy(session,read,strict,None,0,0x2e,0x2d,'busy')
        lines=gates._new(read,since)
        result['busy_rejections']=[line for line in lines if '[CharacterRoster] request rejected: Wait for the character swap to finish' in line]
        result['raw']=lines
        result['row_acceptance']='review native rejection/build ordering' if result['busy_rejections'] else 'OPEN: no explicit busy rejection'
        ctx.log({'pending_busy_observation':result})
        return result
    finally:
        ctx.log({'trace_checkpoint':ctx.call('trace.counters')})
