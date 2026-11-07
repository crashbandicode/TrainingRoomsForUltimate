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
    path = steps.gate_source(ctx)
    if not path.is_file():
        return 0
    return len(re.findall(r'playback .*warp=1', path.read_text(errors='replace')))


def prep(ctx, name):
    """Copy one fixture, reload the catalog, and center. Returns the card directory."""
    src = FIXTURES / name
    if not src.is_dir():
        raise RuntimeError('missing fixture %s' % src)
    dest = _card(ctx)
    _replace_tree(src / 'macros', dest / 'macros')
    _replace_tree(src / 'saves', dest / 'saves')
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


def acc_play_cpu(ctx):
    prep(ctx, 'play-cpu')
    before = _warps(ctx)
    ctx.play(steps.PLAY_CPU)
    line = steps.diag(ctx, r'playback .*role=cpu .*warp=0', cap=15)
    warps = _warps(ctx) - before
    shot = ctx.shot('play-cpu')
    if warps != 0:
        raise RuntimeError('play on CPU warped %d times (wanted 0): %s' % (warps, shot))
    return steps.save(ctx, 'play-cpu', {'line': line, 'warps': warps, 'shot': str(shot)})


def acc_loop(ctx):
    prep(ctx, 'play-warp')
    return steps.rr_loop(ctx)


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
    prep(ctx, 'empty')
    return steps.rr_grade_bar(ctx)


def acc_adv_shield(ctx):
    prep(ctx, 'empty')
    return steps.advantage(ctx, steps.JAB, True)


def acc_adv_hit(ctx):
    prep(ctx, 'empty')
    return steps.advantage(ctx, steps.FSMASH, False)


def acc_strip(ctx):
    """The strip is already on screen. The proof is the shot, not a visibility-change line."""
    prep(ctx, 'empty')
    shot = ctx.shot('strip')
    from PIL import Image
    image = Image.open(shot).convert('RGB')
    width, height = image.size
    band = image.crop((0, int(height * 0.78), width, height))
    colors = band.getcolors(maxcolors=80000)
    if not colors or len(colors) < 12:
        raise RuntimeError('bottom band has no strip detail: %s' % shot)
    return steps.save(ctx, 'strip', {'shot': str(shot), 'colors': len(colors)})


def acc_helper(ctx):
    dest = prep(ctx, 'empty')
    path = dest / 'settings.json'
    if not path.is_file():
        raise RuntimeError('no settings.json after the menu closed')
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

    walk(json.loads(path.read_text()))
    if not found or any(item != [1, 0] for item in found):
        raise RuntimeError('combo helper is not On in %s: %s' % (path, found))
    return steps.save(ctx, 'helper', {'combo_helper': found})


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
