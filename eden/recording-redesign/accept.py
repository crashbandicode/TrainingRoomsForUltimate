"""rc.5 acceptance rows. One checklist item each.

Every row copies its own fixture onto that instance's TrainingRooms card, opens and
closes the menu so the catalog reloads, then checks one steady state. A row does
not read another row's result. `shown=` and `link verdict=` are change events this
build does not print on a clip that starts with the strip already up, so no row
waits for them.
"""
import json
import re
import shutil
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import steps

FIXTURES = Path(__file__).resolve().parent / 'fixtures'


def _card(ctx):
    return Path(ctx.instance['diag_log']).parent


def _replace_tree(src, dest):
    if dest.exists():
        shutil.rmtree(dest)
    if src.is_dir():
        shutil.copytree(src, dest)
    else:
        dest.mkdir()


def _patch_settings(path):
    """Combo helper stays On. The menu writes this file when it closes."""
    if not path.is_file():
        return
    data = json.loads(path.read_text())

    def walk(node):
        if isinstance(node, dict):
            if 'combo_helper' in node:
                node['combo_helper'] = [1, 0]
            for value in node.values():
                walk(value)
        elif isinstance(node, list):
            for value in node:
                walk(value)

    walk(data)
    path.write_text(json.dumps(data) + '\n')


def _warps(ctx):
    path = Path(steps.gate_source(ctx))
    if not path.is_file():
        return 0
    return len(re.findall(r'playback .*warp=1', path.read_text(errors='replace')))


def acc_boot(ctx, release=False):
    """Leave the intro and enter Training. The shot is the proof Start was taken."""
    out = steps.boot_training(ctx, release=release)
    shot = ctx.shot('in-training')
    out['shot'] = str(shot)
    return out


def prep(ctx, name):
    """Copy one fixture, reload the catalog, and center. Returns the card directory."""
    src = FIXTURES / name
    if not src.is_dir():
        raise RuntimeError('missing fixture %s' % src)
    dest = _card(ctx)
    _replace_tree(src / 'macros', dest / 'macros')
    _replace_tree(src / 'saves', dest / 'saves')
    _replace_tree(src / 'combos', dest / 'combos')
    index = src / 'recording-index.json'
    if not index.is_file():
        raise RuntimeError('fixture %s has no recording-index.json' % name)
    shutil.copyfile(index, dest / 'recording-index.json')
    _patch_settings(dest / 'settings.json')
    steps.open_menu(ctx)
    steps.close_menu(ctx)
    ctx.play(steps.RESET_CENTER)
    time.sleep(0.8)
    return dest


def acc_record(ctx):
    prep(ctx, 'empty')
    return steps.rr_record(ctx)


def acc_record_cpu(ctx):
    prep(ctx, 'empty')
    return steps.rr_record_cpu(ctx)


def acc_play_p1(ctx):
    prep(ctx, 'play-warp')
    before = _warps(ctx)
    ctx.play(steps.PLAY_P1)
    line = steps.diag(ctx, r'playback .*role=player .*warp=1', cap=15)
    warps = _warps(ctx) - before
    shot = ctx.shot('play-p1')
    if warps != 1:
        raise RuntimeError('play on P1 warped %d times (wanted 1): %s' % (warps, shot))
    return steps.save(ctx, 'play-p1', {'line': line, 'warps': warps, 'shot': str(shot)})


# The frame strip's lanes in a 1280-wide shot (rc5-accept-yoga2 adv-hit frame). An attack paints
# orange (startup) and red (active) cells before its blue recovery; a busy state without a hitbox
# is blue only. rc5-opus-diag1 showed a blue-only 11-frame CPU bar after Play on CPU, which a plain
# before/after difference counted as playback, so the check is the attack colours.
P1_LANE = (288, 50, 792, 66)
CPU_LANE = (288, 80, 792, 96)


def _busy_cells(shot, lane):
    """Recovery-blue cells ([82, 136, 232], `frame_strip::Cell::Recovery`). The CPU lane never shows
    startup or active colours (`frame_strip::classify` keeps those for P1), so a CPU action is a blue bar."""
    from PIL import Image
    with Image.open(shot) as image:
        pixels = list(image.convert('RGB').crop(lane).getdata())
    return sum(1 for r, g, b in pixels if abs(r - 82) < 30 and abs(g - 136) < 30 and abs(b - 232) < 30)


def _attack_cells(shot, lane):
    from PIL import Image
    with Image.open(shot) as image:
        if image.size[0] != 1280:
            raise RuntimeError('lane check needs a 1280-wide shot: %s' % (image.size,))
        pixels = list(image.convert('RGB').crop(lane).getdata())
    return sum(1 for r, g, b in pixels if r > 200 and b < 110 and g < 200)


def acc_play_cpu(ctx):
    """Play on CPU of a macro with no start save: the CPU plays it and nothing warps.

    This build prints `screen=playback role=… warp=1` only when it warps (rr_host::play_role);
    there is no warp=0 line. The no-warp check is the absence of a new warp=1 line, and the
    playback check is attack cells (the jab) in the strip's CPU lane after the chord.
    """
    prep(ctx, 'play-cpu')
    time.sleep(2.0)
    before = _warps(ctx)
    idle = ctx.shot('cpu-lane-before')
    ctx.play(steps.PLAY_CPU)
    time.sleep(0.6)
    during = ctx.shot('cpu-lane-during')
    time.sleep(2.5)
    after = ctx.shot('play-cpu-after')
    warps = _warps(ctx) - before
    attack = max(_attack_cells(during, CPU_LANE), _attack_cells(after, CPU_LANE))
    out = {'warps': warps, 'cpu_attack_cells': attack,
           'cpu_attack_cells_before': _attack_cells(idle, CPU_LANE),
           'shots': [str(idle), str(during), str(after)]}
    steps.save(ctx, 'play-cpu', out)
    if warps != 0:
        raise RuntimeError('play on CPU warped %d times (wanted 0): %s' % (warps, during))
    if attack < 20:
        raise RuntimeError('no attack cells in the strip CPU lane after Play on CPU (%d): %s'
                           % (attack, during))
    return out


def acc_cpu_menu(ctx):
    """Play on CPU with the CPU macro set through the menu, no fixture.

    The pop-up's CPU view sets the record role, the record chord records a jab while
    possessing the CPU, and the stopped file becomes the CPU's active macro. Then Play on CPU.
    The shots and the strip's CPU lane are the effect; the trace points in the run config
    (play_role, step_role, the AI-epilogue apply) are read back into the row folder.
    """
    steps.settle(ctx)
    card = _card(ctx)
    steps.popup(ctx)
    steps.press(ctx, 'r', 0.5)
    role = steps.diag(ctx, r'screen=recording-popup role=cpu\b', cap=6)
    steps.press(ctx, 'b', 0.6)
    before_record = time.time()
    ctx.play(steps.RECORD)
    start = steps.diag(ctx, r'screen=recording role=cpu recording=1', cap=10)
    # The record chord starts a 30-frame lockout (recording::LOCKOUT_FRAMES) that drops presses,
    # then standby until the first press. A jab sent with the start line lands in the lockout.
    time.sleep(1.2)
    ctx.play(steps.JAB)
    time.sleep(0.8)
    ctx.play(steps.RECORD)
    stop = steps.diag(ctx, r'screen=recording role=cpu recording=0', cap=10)
    time.sleep(1.0)
    written = [p for p in (card / 'macros').glob('*.json') if p.stat().st_mtime >= before_record - 1]
    if not written:
        raise RuntimeError('Record on CPU wrote no macro in %s' % (card / 'macros'))
    path = max(written, key=lambda item: item.stat().st_mtime)
    frames = json.loads(path.read_text()).get('frames') or []
    pressed = [f.get('buttons') for f in frames if f.get('buttons') not in (None, 32768)]
    index = json.loads((card / 'recording-index.json').read_text())
    ctx.play(steps.RESET_CENTER)
    time.sleep(1.5)
    counters_before = ctx.call('trace.counters', {})
    shots = [str(ctx.shot('cpu-before'))]

    def play():
        ctx.play(steps.PLAY_CPU)
        for name, gap in (('cpu-t03', 0.3), ('cpu-t06', 0.3), ('cpu-t10', 0.4)):
            time.sleep(gap)
            shots.append(str(ctx.shot(name)))

    clip = steps.gif(ctx, 'cpu-play.gif', 3, play)
    shots.append(str(ctx.shot('cpu-t30')))
    counters_after = ctx.call('trace.counters', {})
    (ctx.row_dir / 'trace-counters-before.json').write_text(json.dumps(counters_before, indent=2) + '\n')
    (ctx.row_dir / 'trace-counters-after.json').write_text(json.dumps(counters_after, indent=2) + '\n')
    busy_before = _busy_cells(shots[0], CPU_LANE)
    busy = max(_busy_cells(shot, CPU_LANE) for shot in shots[1:])
    out = {'role': role, 'start': start, 'stop': stop, 'file': path.name, 'frames': len(frames),
           'pressed': pressed[:8], 'active': index.get('active'), 'cpu_busy_cells_before': busy_before,
           'cpu_busy_cells_after': busy, 'gif': clip, 'shots': shots}
    steps.save(ctx, 'cpu-menu', out)
    if busy <= busy_before + 20:
        raise RuntimeError('the strip CPU lane shows no new action after Play on CPU (%d, before %d): %s'
                           % (busy, busy_before, shots[2]))
    return out


def _orange_cells(shot):
    """Floppy-orange pixels ([242, 152, 72], `recording_menu::SAVE_ON`) in the first row band of a file
    screen, right half (the value column), scaled from the 1280x720 frame to the shot's size."""
    from PIL import Image
    with Image.open(shot) as image:
        width, height = image.size
        box = (int(width * 0.45), int(height * 125 / 720), width, int(height * 180 / 720))
        pixels = list(image.convert('RGB').crop(box).getdata())
    return sum(1 for r, g, b in pixels if abs(r - 242) < 45 and abs(g - 152) < 45 and abs(b - 72) < 45)


def acc_floppy(ctx):
    """A macro's own screen shows the orange floppy on its Start save row (RECORDING-REDESIGN floppy chip)."""
    dest = prep(ctx, 'play-warp')
    files = len(list((dest / 'saves').glob('*.json'))) + len(list((dest / 'macros').glob('*.json')))
    _, title = steps.open_library_item(ctx, True, 0)
    time.sleep(0.8)
    shot = ctx.shot('file-screen')
    orange = _orange_cells(shot)
    steps.press(ctx, 'b', 0.6)
    steps.close_menu(ctx)
    # The shot is also the check of the left column (`Save states & Macros (<files>)`) and the footer hint.
    out = {'title': title, 'orange_cells': orange, 'shot': str(shot), 'saves_and_macros_on_card': files}
    steps.save(ctx, 'floppy', out)
    if title != 'Start save' or orange < 30:
        raise RuntimeError('no floppy on the Start save row (title %r, orange %d): %s' % (title, orange, shot))
    return out


def _mapped_records(ctx, since):
    """P1's mapped record as input_display stores it (the trace point on that store, x8): buttons, then the four
    stick bytes. Only records written after `since` (a trace_jsonl byte offset)."""
    path = Path(ctx.instance['trace_jsonl'])
    with path.open('rb') as handle:
        handle.seek(since)
        lines = handle.read().decode('utf-8', 'replace').splitlines()
    out = []
    for line in lines:
        try:
            record = json.loads(line)
        except ValueError:
            continue
        value = (record.get('registers') or {}).get('x8')
        if value is None:
            continue
        word = int(value, 16) if isinstance(value, str) else int(value)
        signed = lambda shift: ((word >> shift) & 0xff) - 256 if (word >> shift) & 0x80 else (word >> shift) & 0xff
        out.append({'frame': int(record.get('frame', 0)), 'buttons': word & 0xffffffff, 'lx': signed(32),
                    'ly': signed(40), 'rx': signed(48), 'ry': signed(56)})
    return out


def acc_cstick_probe(ctx):
    """What the mapped record holds while a C-stick smash comes out: left stick neutral, then left stick up."""
    cases = {}
    for name, left_y in (('left-neutral', 0.0), ('left-up', 0.6)):
        steps.reset(ctx)
        time.sleep(0.8)
        since = Path(ctx.instance['trace_jsonl']).stat().st_size
        if left_y:
            ctx.call('input.stick', {'stick': 'left', 'x': 0, 'y': left_y, 'lease_ms': 1500})
            time.sleep(0.3)
        ctx.call('input.stick', {'stick': 'right', 'x': 1, 'y': 0, 'lease_ms': 300})
        time.sleep(0.25)
        shot = ctx.shot('cstick-' + name)
        time.sleep(1.0)
        ctx.call('input.stick', {'stick': 'right', 'x': 0, 'y': 0, 'lease_ms': 100})
        ctx.call('input.stick', {'stick': 'left', 'x': 0, 'y': 0, 'lease_ms': 100})
        time.sleep(0.3)
        records = _mapped_records(ctx, since)
        marked = [r for r in records if r['buttons'] & 0x80]
        first = marked[0]['frame'] if marked else None
        window = [r for r in records if first is not None and first - 3 <= r['frame'] <= first + 8]
        cases[name] = {'records': len(records), 'cstick_on_frames': len(marked), 'around': window, 'shot': str(shot)}
    steps.save(ctx, 'cstick-probe', cases)
    if not any(case['cstick_on_frames'] for case in cases.values()):
        raise RuntimeError('no CSTICK_ON record in either case: %s' % {k: v['records'] for k, v in cases.items()})
    return cases


def acc_loop(ctx):
    """Loop 2 on Player 1, raised with X on the macro row; two passes, one warp.

    The pop-up opens on the save-state row, so Down puts the cursor on the macro before X.
    The line is `screen=recording-popup role=player tab=… loop=2 …`.
    """
    prep(ctx, 'play-warp')
    steps.popup(ctx)
    steps.press(ctx, 'down', 0.4)
    ctx.play(steps.X)
    line = steps.diag(ctx, r'screen=recording-popup role=player .*loop=2', cap=8)
    shot_popup = ctx.shot('loop-2-popup')
    steps.popup_close(ctx)
    time.sleep(0.5)
    before = _warps(ctx)
    ctx.play(steps.PLAY_P1)
    first = steps.diag(ctx, r'playback role=player warp=1', cap=15)
    time.sleep(1.0)
    pass_two = ctx.shot('loop-pass-2')
    time.sleep(4.0)
    warps = _warps(ctx) - before
    out = {'x': line, 'first_warp': first, 'warps': warps,
           'shots': [str(shot_popup), str(pass_two)]}
    steps.save(ctx, 'loop', out)
    if warps != 1:
        raise RuntimeError('loop 2 warped %d times (wanted 1): %s' % (warps, pass_two))
    return out


def acc_chain(ctx):
    prep(ctx, 'chain')
    before = _warps(ctx)
    ctx.play(steps.PLAY_CPU)
    line = steps.diag(ctx, r'playback .*warp=1', cap=15)
    time.sleep(3.0)
    warps = _warps(ctx) - before
    shot = ctx.shot('chain')
    if warps != 1:
        raise RuntimeError('the chain warped %d times (wanted 1): %s' % (warps, shot))
    return steps.save(ctx, 'chain', {'line': line, 'warps': warps, 'shot': str(shot)})


def acc_mismatch(ctx):
    """One Mario macro, P1 moved off Mario. The product prints ask=macro."""
    prep(ctx, 'play-warp')
    return steps.rr_mismatch_macro(ctx)


def acc_rename(ctx):
    prep(ctx, 'play-warp')
    return steps.rr_rename(ctx)


def acc_crop(ctx):
    prep(ctx, 'play-warp')
    return steps.rr_crop(ctx)


def acc_make_combo(ctx):
    prep(ctx, 'play-warp')
    return steps.rr_make_combo(ctx)


def acc_notice(ctx):
    """The fixture macro names a start save that is not on the card."""
    prep(ctx, 'notice')
    ctx.play(steps.PLAY_P1)
    line = steps.diag(ctx, r'screen=notice kind=macro-start-save-missing', cap=12)
    shot = ctx.shot('notice')
    return steps.save(ctx, 'notice', {'line': line, 'shot': str(shot)})


def acc_migrated(ctx):
    """Steady state of a migrated library. The once=1 line is written at boot, not by this row."""
    dest = prep(ctx, 'play-warp')
    line = steps.popup(ctx)
    names = sorted(p.name for p in (dest / 'macros').glob('*.json'))
    index = json.loads((dest / 'recording-index.json').read_text())
    shot = ctx.shot('catalog')
    steps.press(ctx, 'b', 0.4)
    steps.press(ctx, 'b', 0.4)
    if 'jab.json' not in names or not index.get('migrated'):
        raise RuntimeError('migrated catalog missing (files=%s migrated=%s) %s'
                           % (names, index.get('migrated'), shot))
    return steps.save(ctx, 'migrated', {
        'files': names, 'migrated': index.get('migrated'), 'line': line, 'shot': str(shot),
    })


def acc_grade(ctx):
    """The drill is Combo 1 (two jab links) from the fixture index; the helper grades the attempts.

    The trace point on tr_combo_attempt_v1 comes from the run config (on_return).
    """
    prep(ctx, 'grade')
    return steps.rr_grade_bar(ctx)


def acc_adv_shield(ctx):
    prep(ctx, 'empty')
    return steps.advantage(ctx, steps.JAB, True)


def acc_adv_hit(ctx):
    prep(ctx, 'empty')
    return steps.advantage(ctx, steps.FSMASH, False)


def acc_strip(ctx):
    """The strip sits at the top. The shot is the proof, not a visibility-change line."""
    shot = ctx.shot('strip')
    from PIL import Image
    image = Image.open(shot).convert('RGB')
    width, height = image.size
    band = image.crop((0, 0, width, max(1, int(height * 0.22))))
    colors = band.getcolors(maxcolors=80000)
    if not colors or len(colors) < 12:
        raise RuntimeError('top band has no strip detail: %s' % shot)
    return steps.save(ctx, 'strip', {'shot': str(shot), 'colors': len(colors)})


def acc_helper(ctx):
    """On by default. A fresh settings.json omits combo_helper; the factory stores index 0, which is [1, 0]."""
    shot = ctx.shot('helper-hud')
    path = Path(ctx.instance['settings'])
    found = []

    def walk(node):
        if isinstance(node, dict):
            if 'combo_helper' in node:
                found.append(node['combo_helper'])
            for value in node.values():
                walk(value)
        elif isinstance(node, list):
            for value in node:
                walk(value)

    if path.is_file():
        walk(json.loads(path.read_text()))
    if any(item != [1, 0] for item in found):
        raise RuntimeError('combo helper is not On in %s: %s' % (path, found))
    return steps.save(ctx, 'helper', {
        'shot': str(shot),
        'combo_helper': found,
        'code_default_index': 0,
        'code_default_stored': [1, 0],
        'source': 'src/model/factory.rs combo_helper Single [0]',
    })


def acc_rel_record(ctx):
    """Release window: Record on Player 1 writes the jab and its start save, and Play on P1 runs it.

    The release NRO prints no screen= lines, so the card files and the shots are the check.
    """
    dest = prep(ctx, 'empty')
    before = time.time()
    ctx.play(steps.RECORD)
    time.sleep(0.6)
    ctx.play(steps.JAB)
    time.sleep(0.8)
    ctx.play(steps.RECORD)
    time.sleep(1.5)
    macros = [p for p in (dest / 'macros').glob('*.json') if p.stat().st_mtime >= before - 1]
    if not macros:
        raise RuntimeError('Record on P1 wrote no macro in %s' % (dest / 'macros'))
    path = max(macros, key=lambda item: item.stat().st_mtime)
    frames = json.loads(path.read_text()).get('frames') or []
    pressed = [frame.get('buttons') for frame in frames if frame.get('buttons') not in (None, 32768)]
    if not pressed:
        raise RuntimeError('%s holds only idle buttons 32768' % path.name)
    saves = sorted(p.name for p in (dest / 'saves').glob('*.json') if p.stat().st_mtime >= before - 1)
    ctx.play(steps.RESET_CENTER)
    time.sleep(0.8)
    idle = ctx.shot('before-play')
    ctx.play(steps.PLAY_P1)
    time.sleep(0.5)
    during = ctx.shot('during-play')
    time.sleep(1.5)
    after = ctx.shot('after-play')
    if not saves:
        raise RuntimeError('Record on P1 wrote no start save in %s' % (dest / 'saves'))
    return steps.save(ctx, 'rel-record', {
        'file': path.name, 'frames': len(frames), 'pressed': pressed[:8], 'start_saves': saves,
        'shots': [str(idle), str(during), str(after)],
    })


def acc_heap(ctx):
    return steps.heap_crash(ctx)


def acc_hook(ctx):
    """The diag hook counter. The gate is allocations=0 and no recorder_ai charge."""
    path = Path(ctx.instance['diag_log'])
    text = path.read_text(errors='replace') if path.is_file() else ''
    lines = [line for line in text.splitlines() if 'hook_stats' in line or 'allocs=' in line]
    if not lines:
        raise RuntimeError('no hook_stats line in %s' % path)
    last = lines[-1]
    (ctx.row_dir / 'hook-stats-snapshot.log').write_text(last + '\n')
    count = re.search(r'allocations=(\d+)', last)
    named = re.search(r'allocs=(\S+)', last)
    charged = named and re.search(r':[1-9]', named.group(1))
    if count is None or count.group(1) != '0' or charged:
        raise RuntimeError('hook allocation is not 0: %s' % last)
    return steps.save(ctx, 'hook', {'line': last})
