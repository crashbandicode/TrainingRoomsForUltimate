"""rc.5 CPU playback fidelity rows (2026-11-08).

Each play is traced by the run config's points (fid-rows.json "trace_points" names their ids): the frame
the playback writes, every consuming frame (the CPU's AI frame, Player 1's poll), the worker step, and
`MotionModule::change_motion`. A row copies its trace window into the row folder and compares it with
the file frame by frame (`fid_trace.compare`). `strict` rows fail on a dropped frame, a consuming frame
with no frame inside the run, or a CPU smash where the file has none; probe rows only record.
"""
import json
import shutil
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import accept
import fid_trace
import steps

MULTI = 'multi-input'
SMASHES = {'attack_s4_s', 'attack_s4_hi', 'attack_s4_lw', 'attack_s4_hold', 'attack_hi4', 'attack_lw4'}


def _arm(ctx):
    """Arm rows.json "trace_points" (per build: the targets are offsets in that build's ELF) and return
    {"p1": {"apply", "register", "tick"}, "cpu": {...}, "motion", "step_role", "armed"} with the point ids."""
    plan = ctx.runner.plan['trace_points']
    armed = []

    def arm(spec):
        result = ctx.call('trace.arm', spec)
        armed.append(result)
        point = result.get('point', result) if isinstance(result, dict) else {}
        ident = point.get('id') if isinstance(point, dict) else None
        if ident is None:
            raise RuntimeError('trace.arm gave no id for %s: %s' % (spec.get('target'), result))
        return str(ident)

    ids = {'motion': arm(plan['motion']), 'step_role': arm(plan['step_role'])}
    for role in ('p1', 'cpu'):
        side = plan[role]
        ids[role] = {'apply': arm(side['apply']), 'register': side['register'], 'tick': arm(side['tick'])}
        if side.get('ai_own'):
            ids[role]['ai_own'] = arm(side['ai_own'])
    ids['armed'] = armed
    (ctx.row_dir / 'trace-armed.json').write_text(json.dumps(armed, indent=2, default=str) + '\n')
    ctx.points = ids
    return ids


def _disarm(ctx):
    for result in getattr(ctx, 'points', {}).get('armed', []):
        point = result.get('point', result) if isinstance(result, dict) else {}
        if isinstance(point, dict) and point.get('id') is not None:
            ctx.call('trace.disarm', {'id': str(point['id'])})


def _points(ctx):
    return ctx.points


def _trace_size(ctx):
    path = Path(ctx.instance['trace_jsonl'])
    return path.stat().st_size if path.is_file() else 0


def _window(ctx, start, name):
    """Wait for the stream to drain, copy [start, end) into the row folder, return the records."""
    time.sleep(0.6)
    path = Path(ctx.instance['trace_jsonl'])
    with path.open('rb') as handle:
        handle.seek(start)
        data = handle.read()
    (ctx.row_dir / (name + '.trace.jsonl')).write_bytes(data)
    return fid_trace.records(data.decode('utf-8', 'replace').splitlines())


def _gif(ctx, name, seconds, do):
    """Evidence clip: 640 wide, 15 fps (GIFS, "Evidence clips are not README GIFs")."""
    path = ctx.row_dir / name
    if path.exists():
        # A retried row keeps the earlier attempt's clip; Eden does not overwrite.
        path = path.with_name('%s-%d%s' % (path.stem, int(time.time()), path.suffix))
    ctx.call('recording.start', {'path': str(path), 'fps': 15, 'width': 640, 'duration_ms': int(seconds * 1000)})
    time.sleep(0.3)
    do()
    status = None
    for _ in range(40):
        status = ctx.call('recording.status')
        if isinstance(status, dict) and status.get('state') not in ('recording', 'finalizing'):
            break
        time.sleep(0.5)
    return {'gif': str(path), 'written': path.exists(), 'status': status}


def _judge(ctx, role, recs, expected, strict):
    points = _points(ctx)
    side = points[role]
    report = fid_trace.compare(expected, fid_trace.applied(recs, side['apply'], side['register']),
                               fid_trace.ticks(recs, side['tick']))
    moves = fid_trace.motions(recs, points['motion'])
    report['motions'] = moves
    names = [name for _, _, name in moves]
    report['smashes'] = sorted(set(names) & SMASHES)
    report['jab'] = 'attack_11' in names
    report['worker_steps'] = sum(1 for r in recs if r.get('point_id') == points['step_role'])
    problems = []
    if report['dropped']:
        problems.append('%d file frames never applied (first %s)' % (report['dropped'], report['dropped_first']))
    if report['ticks_without_frame']:
        problems.append('%d %s frames inside the run got no frame' % (report['ticks_without_frame'], role))
    if report['unmatched']:
        problems.append('%d applied frames are not the file\'s next frames' % report['unmatched'])
    expects_smash = False
    if report['smashes'] and not expects_smash:
        problems.append('smash motions %s from a file with no smash' % report['smashes'])
    if role == 'cpu' and not report['jab']:
        problems.append('no attack_11 motion')
    report['problems'] = problems
    if strict and problems:
        raise RuntimeError('%s play: %s' % (role, '; '.join(problems)))
    return report


def _play(ctx, role, chord, expected, strict, warp, seconds=8.0):
    """`seconds` covers the run at game speed plus the warp: a file of n frames takes n/60 s."""
    steps.center(ctx)
    time.sleep(0.6)
    start = _trace_size(ctx)
    shots = []

    def do():
        ctx.play(chord)
        if warp:
            steps.diag(ctx, r'playback role=%s warp=1' % ('player' if role == 'p1' else 'cpu'), cap=10)
        for name, gap in (('t1', 0.9), ('t2', 0.9), ('t3', 1.2)):
            time.sleep(gap)
            shots.append(str(ctx.shot('%s-%s' % (role, name))))
        time.sleep(max(0.5, seconds - 3.6))

    clip = _gif(ctx, '%s-play.gif' % role, min(seconds + 1.0, 20.0), do)
    recs = _window(ctx, start, role + '-play')
    report = _judge(ctx, role, recs, expected, strict)
    report.update({'clip': clip, 'shots': shots})
    return report


def fid_fixture(ctx, strict=False):
    """The fidelity fixture on both roles: Play on P1, then Play on CPU of the same file."""
    card = _prep(ctx, 'fidelity')
    expected = fid_trace.file_frames(json.loads((card / 'macros' / 'fidelity.json').read_text()))
    out = {'file_frames': len(expected)}
    _arm(ctx)
    try:
        out['p1'] = _play(ctx, 'p1', steps.PLAY_P1, expected, False, False)
        out['cpu'] = _play(ctx, 'cpu', steps.PLAY_CPU, expected, False, False)
    finally:
        _disarm(ctx)
    steps.save(ctx, 'fidelity-fixture', out)
    _verdict(out, ('p1', 'cpu'), strict)
    return out


def _verdict(out, keys, strict):
    problems = ['%s: %s' % (key, '; '.join(out[key]['problems'])) for key in keys if out[key]['problems']]
    if strict and problems:
        raise RuntimeError(' | '.join(problems))


def _popup_role(ctx, want):
    line = steps.popup(ctx)
    if 'role=%s' % want not in line:
        steps.press(ctx, 'r' if want == 'cpu' else 'l', 0.5)
        line = steps.diag(ctx, r'screen=recording-popup role=%s\b' % want, cap=6)
    steps.press(ctx, 'b', 0.6)
    return line


def _record(ctx, role):
    card = accept._card(ctx)
    steps.center(ctx)
    word = 'cpu' if role == 'cpu' else 'player'
    popup = _popup_role(ctx, word)
    before = time.time()
    ctx.play(steps.RECORD)
    start = steps.diag(ctx, r'screen=recording role=%s recording=1' % word, cap=10)
    # A CPU-role record starts with the 30-frame lockout (recording::LOCKOUT_FRAMES).
    time.sleep(1.2 if role == 'cpu' else 0.6)
    ctx.play(MULTI)
    time.sleep(0.4)
    ctx.play(steps.RECORD)
    stop = steps.diag(ctx, r'screen=recording role=%s recording=0' % word, cap=10)
    time.sleep(1.0)
    written = [p for p in (card / 'macros').glob('*.json') if p.stat().st_mtime >= before - 1]
    if not written:
        raise RuntimeError('Record (%s) wrote no macro in %s' % (role, card / 'macros'))
    path = max(written, key=lambda item: item.stat().st_mtime)
    shutil.copyfile(path, ctx.row_dir / ('%s-recorded-%s' % (role, path.name)))
    expected = fid_trace.file_frames(json.loads(path.read_text()))
    return {'popup': popup, 'start': start, 'stop': stop, 'file': path.name, 'frames': len(expected)}, expected


def fid_record(ctx, strict=False):
    """The multi-input segment recorded through the chord on Player 1 and played on Player 1, then recorded
    on the CPU role through the pop-up and played on the CPU."""
    out = {}
    _arm(ctx)
    try:
        meta, expected = _record(ctx, 'p1')
        out['p1_record'] = meta
        out['p1'] = _play(ctx, 'p1', steps.PLAY_P1, expected, False, True, seconds=len(expected) / 60 + 3)
        meta, expected = _record(ctx, 'cpu')
        out['cpu_record'] = meta
        out['cpu'] = _play(ctx, 'cpu', steps.PLAY_CPU, expected, False, True, seconds=len(expected) / 60 + 3)
    finally:
        _disarm(ctx)
    steps.save(ctx, 'fidelity-record', out)
    steps.save(ctx, 'unmapped-access', ctx.call('trace.unmapped_access', {}))
    _verdict(out, ('p1', 'cpu'), strict)
    return out


def _prep(ctx, name):
    """accept.prep, then the fixture's index again: the menu close writes the menu's older selection back over it
    (LANE-OPUS-RC5-ACCEPT-REPORT open item 1), and the worker reads the index from the card at the Play chord."""
    card = accept.prep(ctx, name)
    shutil.copyfile(accept.FIXTURES / name / 'recording-index.json', card / 'recording-index.json')
    time.sleep(0.3)
    return card


def fid_chord3(ctx, strict=False):
    """The docs-GIF fixture of the reported "smash" (rc5-docs-gifs 0fbd068 `chord3`: three one-frame 0x8201 presses,
    loop 5, a start save): Play on CPU, traced. The judge reads the motions; the loop repeats the file."""
    card = _prep(ctx, 'chord3')
    expected = fid_trace.file_frames(json.loads((card / 'macros' / 'jab.json').read_text()))
    _arm(ctx)
    try:
        report = _play(ctx, 'cpu', steps.PLAY_CPU, expected * 5, False, True, seconds=19.0)
    finally:
        _disarm(ctx)
    steps.save(ctx, 'fidelity-chord3', report)
    _verdict({'cpu': report}, ('cpu',), strict)
    return report


def fid_chord3_strict(ctx):
    return fid_chord3(ctx, strict=True)


def fid_fixture_strict(ctx):
    return fid_fixture(ctx, strict=True)


def fid_record_strict(ctx):
    return fid_record(ctx, strict=True)
