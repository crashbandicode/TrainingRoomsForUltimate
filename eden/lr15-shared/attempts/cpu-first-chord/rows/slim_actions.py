"""SOURCE ONLY: lead-owned recorded segments; never a transport or PASS oracle.

run_row(row_id, ctx) uses only play, capture, read_diag, record, out, s.
The lead's existing guarded ctx.play must verify the segment SHA, start/end
state and bounded marker in requirements.json. capture QUEUES real media for
row checkpoint; its return value is never a boolean navigation/acceptance gate.
Import has no effects. No settings writes, UI presses, native requests or traces.
"""
import re


ROW_IDS = ('r3-visual-pause', 'pilot-deluxe')
ROW_ALIASES = {'pilot': 'pilot-deluxe', 'RC-PILOT': 'pilot-deluxe'}
SEGMENTS = {
    'r3-visual-pause': (
        'r3-settings-on', 'r3-attack-observe', 'r3-catch-observe',
        'r3-stock-shield-observe', 'r3-native-pause-open',
        'r3-native-pause-close', 'r3-settings-off'),
    'pilot-deluxe': (
        'pilot-prerequisites', 'pilot-open-player-row', 'pilot-set-cpu',
        'pilot-close-to-training-armed', 'pilot-observe-p1-ai',
        'pilot-reopen-cpu-row', 'pilot-set-player',
        'pilot-close-to-training-released', 'pilot-manual-control'),
}


class MissingPositive(RuntimeError):
    pass


def fields(line):
    return dict(re.findall(r'\b([a-z_]+)=([^\s,]+)', line))


def _number(values, key):
    value = values.get(key, '')
    return int(value) if re.fullmatch(r'\d+', value) else None


def attack_pass(line):
    f = fields(line)
    return ('[Hitbox] pass ' in line and (_number(f, 'spawned') or 0) > 0
            and _number(f, 'failed') == 0 and _number(f, 'dropped') == 0)


def attack_stats(line):
    f = fields(line)
    return ('[Hitbox] stats ' in line and (_number(f, 'want_hit') or 0) > 0
            and _number(f, 'want_hurt') == 0
            and _number(f, 'heap_allocations') == 0
            and _number(f, 'failed') == 0 and _number(f, 'dropped') == 0)


def catch_command(line):
    f = fields(line)
    return ('[Hitbox] command ' in line and f.get('kind') == 'catch'
            and f.get('fighter') == 'true' and (_number(f, 'spheres') or 0) > 0)


def _engine_fresh(engine, before, predicate):
    """The actual lead runner's existing predicate-gate API (no transport)."""
    if before is None or 'diag' not in before:
        raise MissingPositive('fresh predicate requires engine before[diag]')
    mark = max((int(m[1]) for line in before['diag']
                if (m := re.search(r'\s#(\d+)\s', line))), default=-1)
    return next((line for line in engine.read_diag()
                 if (m := re.search(r'\s#(\d+)\s', line))
                 and int(m[1]) > mark and predicate(line)), None)


def framepass_gate(engine, before, args):
    return _engine_fresh(engine, before, attack_pass)


def catch_gate(engine, before, args):
    return _engine_fresh(engine, before, catch_command)


def pause_begin_gate(engine, before, args):
    return _engine_fresh(engine, before, lambda l: 'pause_owner begin ' in l)


def pause_end_gate(engine, before, args):
    return _engine_fresh(engine, before, lambda l: 'pause_owner end ' in l and fields(l).get('owners') == '0x00')


def missing_marker(engine, before, args):
    raise MissingPositive('MISSING recorded/automatic marker: ' + args['requirement'])


class _Cursor:
    """Existing current-boot complete-line accumulator, never a rotated tail."""
    def __init__(self, ctx):
        self.ctx = ctx
        self.previous = []
        self.read()

    def read(self):
        lines = list(self.ctx.read_diag())
        if lines[:len(self.previous)] != self.previous:
            raise MissingPositive('current-boot append-only diagnostic history changed')
        seen, last = {}, -1
        for line in lines:
            if not isinstance(line, str):
                raise MissingPositive('diagnostics must be complete text lines')
            match = re.search(r'\s#(\d+)\s', line)
            if not match:
                continue
            seq = int(match[1])
            if seq in seen and seen[seq] != line:
                raise MissingPositive('conflicting diagnostic sequence')
            if seq < last:
                raise MissingPositive('diagnostic sequence moved backwards')
            seen[seq], last = line, seq
        if not seen:
            raise MissingPositive('no current-boot diagnostic sequence')
        self.previous = lines
        return lines

    def mark(self):
        return max(int(m[1]) for line in self.read()
                   if (m := re.search(r'\s#(\d+)\s', line)))

    def wait(self, after, predicate, name, cap):
        end = self.ctx.s.clock() + cap
        # Also bound iterations if an injected/broken clock does not advance.
        for _ in range(int(cap * 10) + 1):
            for line in self.read():
                match = re.search(r'\s#(\d+)\s', line)
                if match and int(match[1]) > after and predicate(line):
                    self.ctx.record('slim-marker', {'name': name, 'after_seq': after,
                                                   'line': line, 'cap_seconds': cap})
                    return line
            if self.ctx.s.clock() >= end:
                break
            self.ctx.s.sleep(.1)
        raise MissingPositive('Missing positive evidence: ' + name)


def _play(ctx, name):
    ctx.record('slim-segment-request', {'name': name})
    # Existing requester/guard implementations remain behind this one method.
    # Do not bypass a MISSING segment or guard by sending individual inputs.
    return ctx.play(name)


def _capture(ctx, name, expectation):
    ctx.capture(name, expectation)
    # capture may return None, a queued token or a path. None is not approval;
    # True is not approval either. The lead checks actual media at row end.
    return {'name': name, 'expectation': expectation, 'status': 'PENDING_REVIEW'}


def _r3(ctx, cursor):
    media, observed = [], {}
    _play(ctx, 'r3-settings-on')
    # The segment's automated settings gate must establish both [1,0] plus
    # closed Training. No stale ui.close after a gameplay/settings reload.
    mark = cursor.mark()
    _play(ctx, 'r3-attack-observe')
    observed['attack_pass'] = cursor.wait(mark, attack_pass, 'fighter frame-pass spawned>0', 10)
    media.append(_capture(ctx, 'r3-attack-active', 'actual attack spheres; no hurtbox-only spheres; identified ordinary bodies/HUD'))
    observed['attack_stats'] = cursor.wait(mark, attack_stats, 'want_hit>0 / want_hurt=0 / heap_allocations=0', 15)

    mark = cursor.mark()
    _play(ctx, 'r3-catch-observe')
    observed['catch'] = cursor.wait(mark, catch_command, 'fresh fighter catch command spheres>0 (lr02 still missing)', 5)
    media.append(_capture(ctx, 'r3-catch-active', 'actual grab spheres, ordinary bodies/HUD; not inferred from input receipt'))

    _play(ctx, 'r3-stock-shield-observe')
    media.append(_capture(ctx, 'r3-stock-shield', 'actual native stock shield on P1 with visualizer On; lr02 image failed; no positive inferred'))
    # Shield has no qualified native positive marker in the inspected inputs.
    # Its requirements entry remains MISSING until lead supplies a real gate.
    mark = cursor.mark()
    _play(ctx, 'r3-native-pause-open')
    observed['pause_begin'] = cursor.wait(mark, lambda l: 'pause_owner begin ' in l, 'native pause begin', 10)
    media.append(_capture(ctx, 'r3-native-pause', 'settled stock Training menu, No. of CPUs=1, native bodies/HUD; not sliding transition'))
    mark = cursor.mark()
    _play(ctx, 'r3-native-pause-close')
    observed['pause_end'] = cursor.wait(mark, lambda l: 'pause_owner end ' in l and fields(l).get('owners') == '0x00', 'native pause end owners=0x00', 10)
    observed['resume_checked'] = cursor.wait(mark, lambda l: 'pause_owner resume_checked ' in l and _number(fields(l), 'leaked') == 0, 'native resume_checked leaked=0', 10)
    media.append(_capture(ctx, 'r3-native-resume', 'actual resumed Training, bodies/HUD intact, menu absent'))
    _play(ctx, 'r3-settings-off')
    media.append(_capture(ctx, 'r3-visuals-off', 'Hitbox and Frame Advantage Off persisted, closed Training, ordinary bodies/HUD; no lingering visualizer spheres'))
    return {'observed': observed, 'media': media,
            'missing_acceptance': ['shield positive marker/media', 'all queued media review',
                                   'independent pause producer', 'CPU replacement/destroy frame-advantage clauses']}


def _pilot(ctx, cursor):
    # This segment also checks current-boot standalone-pilot events.log and
    # provider/common install receipts, which read_diag alone cannot supply.
    _play(ctx, 'pilot-prerequisites')
    # lr02 diag-observed.log contains NO stand-down/install line. Those live
    # in separate events logs. Do not invent a diag predicate or manufacture
    # them here: the existing lead prerequisite guard owns their real reads.
    media = []
    phases = (
        ('pilot-open-player-row', 'pilot-player-row', 'actual Deluxe CPU Pilot selected row, Player, not TR menu'),
        ('pilot-set-cpu', 'pilot-cpu-row', 'same selected pilot row now CPU; native/provider state agrees'),
        ('pilot-close-to-training-armed', 'pilot-armed-training', 'actual closed Training/P1 armed; bodies/HUD intact'),
        ('pilot-observe-p1-ai', 'pilot-p1-ai-action', 'successive frames establish autonomous P1 action, not opponent movement'),
        ('pilot-reopen-cpu-row', 'pilot-cpu-row-before-release', 'same pilot row CPU, actual selected row before release'),
        ('pilot-set-player', 'pilot-player-restored', 'same pilot row Player, actual native/provider inactive state'),
        ('pilot-close-to-training-released', 'pilot-released-training', 'actual closed Training, pilot released, bodies/HUD intact'),
        ('pilot-manual-control', 'pilot-p1-manual-control', 'bounded real P1 manual input with visible response and pilot still inactive'),
    )
    for segment, name, expected in phases:
        _play(ctx, segment)
        media.append(_capture(ctx, name, expected))
    return {'observed': {'pilot_install_and_stand_down': 'requires lead prerequisite guard receipt, not inferred from diag'}, 'media': media,
            'missing_acceptance': ['all queued media review', 'current-boot pilot install/state and exact Deluxe routes must be guarded by segments'],
            'end_state': 'Training/player-control/menus-closed (only if all segment guards and media qualify)'}


def run_row(row_id, ctx):
    """Return PENDING_CHECKPOINT, never product PASS; fail first missing marker.

    Preboot dependency: load real bounded gates/recipes. Operator permits Up
    for recording missing segments first; record_missing saves/hashes each one
    before its reuse. Missing runtime positives still refuse, never auto-True.
    ctx.play must be the lead's existing guarded executor, not a permissive mock.
    No recovery navigation is attempted on an unknown screen. Engine owns stop,
    input reset, pilot disarm recovery and once-at-Down full restoration.
    """
    row_id = ROW_ALIASES.get(row_id, row_id)
    if row_id not in ROW_IDS:
        raise ValueError('unsupported row: ' + str(row_id))
    for name in ('play', 'capture', 'read_diag', 'record'):
        if not callable(getattr(ctx, name, None)):
            raise TypeError('missing ctx.' + name)
    for name in ('clock', 'sleep'):
        if not callable(getattr(ctx.s, name, None)):
            raise TypeError('missing ctx.s.' + name)
    cursor = _Cursor(ctx)
    start = cursor.mark()
    ctx.record('slim-row-start', {'row_id': row_id, 'after_seq': start})
    try:
        value = _r3(ctx, cursor) if row_id == 'r3-visual-pause' else _pilot(ctx, cursor)
        result = {'row_id': row_id, 'status': 'PENDING_CHECKPOINT', **value}
        ctx.record('slim-row-pending-checkpoint', result)
        return result
    except Exception as error:
        ctx.record('slim-row-failed', {'row_id': row_id, 'after_seq': start,
                                     'error': str(error), 'stop': True,
                                     'pilot_state': 'unknown until actual current state checked',
                                     'recovery_owner': 'lead engine; no blind navigation or automatic PASS'})
        raise
