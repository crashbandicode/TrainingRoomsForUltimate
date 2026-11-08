"""rc.5 final acceptance probe: Play on Player 1 right after Record on CPU (rc5-opus-final-diag, 2026-11-08).

acc-play-p1 failed only when it ran right after acc-record-cpu (3 of 3; it passed in the other order), and the card's
recording-index.json then had `active.player = null` although the play-warp fixture names `jab`. This row reads the
index on the card after the fixture copy, after the menu open/close, and after the chord, so the run shows which step
cleared it. The run config's trace points on rr_host::play_role / step_role say whether the chord reached play_role.
"""
import json
import shutil
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import accept  # noqa: E402
import steps  # noqa: E402

WARP = r'playback .*role=player .*warp=1'


def _active(card):
    path = card / 'recording-index.json'
    try:
        return json.loads(path.read_text()).get('active')
    except (OSError, ValueError) as error:
        return 'unreadable: %s' % error


def menu_after_record_cpu(ctx):
    """No fixture: acc-record-cpu just made its file the CPU's active macro. Does opening and closing the menu keep it?"""
    card = accept._card(ctx)
    out = {'after_record': _active(card)}
    steps.open_menu(ctx)
    out['menu_open'] = _active(card)
    steps.close_menu(ctx)
    time.sleep(0.5)
    out['menu_closed'] = _active(card)
    return steps.save(ctx, 'menu-after-record-cpu', out)


def after_record_cpu(ctx):
    card = accept._card(ctx)
    src = accept.FIXTURES / 'play-warp'
    out = {'after_record': _active(card)}
    # As accept.prep: a full close drops the model the pop-up kept, so the fixture is not overwritten by it.
    steps.open_menu(ctx)
    steps.close_menu(ctx)
    out['before_copy'] = _active(card)
    accept._replace_tree(src / 'macros', card / 'macros')
    accept._replace_tree(src / 'saves', card / 'saves')
    accept._replace_tree(src / 'combos', card / 'combos')
    shutil.copyfile(src / 'recording-index.json', card / 'recording-index.json')
    accept._patch_settings(card / 'settings.json')
    out['after_copy'] = _active(card)
    time.sleep(1.0)
    out['after_copy_1s'] = _active(card)
    steps.open_menu(ctx)
    out['menu_open'] = _active(card)
    steps.close_menu(ctx)
    out['menu_closed'] = _active(card)
    ctx.play(steps.RESET_CENTER)
    time.sleep(0.8)
    out['before_chord'] = _active(card)
    ctx.play(steps.PLAY_P1)
    try:
        out['warp'] = steps.diag(ctx, WARP, cap=8)
    except RuntimeError as error:
        out['warp'] = 'none: %s' % error
    out['after_chord'] = _active(card)
    out['shot'] = str(ctx.shot('after-chord'))
    return steps.save(ctx, 'play-p1-after-record-cpu', out)
