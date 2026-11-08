"""rc.5 docs-GIF hooks (docs-gif-rows.json). One setup_ and one clip_ function per docs/MEDIA.md row.

setup_ reaches the start state outside the clip (fixture, bound save, menu cursor). clip_ is the only part
inside the runner's `gif` step: the documented inputs, with lead-in and settle time so the start, the action
and the result are all in the clip. No hook waits for a diag line; the contact sheet is the check.
"""
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import accept  # noqa: E402
import steps  # noqa: E402

LOAD, RESET, JAB = steps.LOAD, steps.RESET_CENTER, steps.JAB


def _stick(ctx, x, y, seconds, stick='left'):
    """Hold a stick for `seconds` (one lease), then return it to neutral."""
    ms = max(1, int(seconds * 1000))
    ctx.call('input.stick', {'stick': stick, 'x': x, 'y': y, 'lease_ms': ms + 60})
    time.sleep(seconds)
    ctx.call('input.stick', {'stick': stick, 'x': 0, 'y': 0, 'lease_ms': 100})


def _bodies(path):
    """Mean x of P1's red body and the CPU's green body (this boot's CPU Mario wears green), or None."""
    from PIL import Image
    im = Image.open(path).convert('RGB')
    w, h = im.size
    px = im.load()
    red, green = [], []
    for y in range(int(h * 0.30), int(h * 0.72), 3):
        for x in range(0, w, 3):
            r, g, b = px[x, y]
            if r > 150 and r > g + 60 and r > b + 60:
                red.append(x)
            elif g > 90 and g > r + 30 and g > b + 30:
                green.append(x)
    if len(red) < 40 or len(green) < 40:
        return None
    return sum(red) / len(red), sum(green) / len(green)


def _face_cpu(ctx):
    """The bound save leaves P1 just right of the CPU, facing away (first XPS pass, chips frame 0).
    A short light tilt toward the CPU turns P1 without a dash. The shots are the receipt."""
    found = _bodies(ctx.shot('before-turn'))
    if found is None:
        return {'turn': 'no bodies found'}
    p1, cpu = found
    x = -0.3 if p1 > cpu else 0.3
    _stick(ctx, x, 0, 0.07)
    time.sleep(0.5)
    ctx.shot('after-turn')
    return {'turn': 'left' if x < 0 else 'right', 'p1_x': round(p1), 'cpu_x': round(cpu)}


def _strip(ctx, name='strip'):
    """Fixture, then Load the bound save (P1 beside the CPU, both 0%), then turn P1 toward the CPU."""
    accept.prep(ctx, name)
    ctx.play(LOAD)
    time.sleep(1.2)
    steps.settle(ctx)
    return {'fixture': name, 'loaded': 'jab-start', 'face': _face_cpu(ctx)}


def boot(ctx):
    out = steps.boot_training(ctx)
    out['shot'] = str(ctx.shot('in-training'))
    return out


def after(ctx):
    ctx.call('input.reset')
    time.sleep(0.3)
    steps.settle(ctx)
    return {'settled': True}


def after_menu(ctx):
    """The clip ended inside the menu: the close chord plus two B, then the HUD gate."""
    ctx.call('input.reset')
    steps.close_menu(ctx)
    return {'closed': True}


# --- frame strip ------------------------------------------------------------------------------

def setup_lanes(ctx):
    out = _strip(ctx)
    ctx.play(RESET)                   # spawn: the dash covers the gap
    time.sleep(0.8)
    return out


def clip_lanes(ctx):
    time.sleep(0.6)
    ctx.play('lanes-smash')
    time.sleep(2.5)


def setup_chips(ctx):
    return _strip(ctx)


def clip_chips(ctx):
    time.sleep(0.6)
    ctx.play('chips-attack')
    time.sleep(1.8)


def setup_cstick(ctx):
    return _strip(ctx)


def clip_cstick(ctx):
    time.sleep(0.6)
    _stick(ctx, 1, 0, 0.12, stick='right')
    time.sleep(2.0)


def setup_stick_row(ctx):
    return _strip(ctx)


def clip_stick_row(ctx):
    time.sleep(0.5)
    _stick(ctx, -1, 0, 0.5)
    time.sleep(0.25)
    _stick(ctx, 0.7, -0.7, 0.5)
    time.sleep(0.25)
    ctx.play('dash-dance')
    time.sleep(1.0)


def setup_freeze(ctx):
    return _strip(ctx)


def clip_freeze(ctx):
    time.sleep(0.5)
    ctx.play(JAB)
    time.sleep(1.6)
    ctx.play(JAB)
    time.sleep(1.0)


def setup_freeze_stick(ctx):
    return _strip(ctx)


def clip_freeze_stick(ctx):
    time.sleep(0.4)
    ctx.play(JAB)
    time.sleep(0.9)
    _stick(ctx, -1, 0, 0.7)
    _stick(ctx, 1, 0, 0.7)
    time.sleep(0.8)


def setup_adv_hit(ctx):
    if steps._shield_index(ctx) != 0:
        steps.cpu_shield(ctx, 'none')
    return _strip(ctx)


def clip_adv_hit(ctx):
    time.sleep(0.6)
    ctx.play(JAB)
    time.sleep(2.8)


def setup_adv_shield(ctx):
    accept.prep(ctx, 'strip')
    shield = steps.cpu_shield(ctx, 'hold')
    out = _strip(ctx)
    time.sleep(0.8)                   # the CPU raises its shield after the load
    out['shield'] = shield
    return out


def clip_adv_shield(ctx):
    time.sleep(0.6)
    ctx.play(JAB)
    time.sleep(3.2)


def setup_link(ctx):
    return _strip(ctx)


def clip_link(ctx):
    time.sleep(0.5)
    ctx.play('jab-link')
    time.sleep(2.5)


def setup_drop(ctx):
    return _strip(ctx)


def clip_drop(ctx):
    """jab-drop's second press came 42 frames late, past the 20-frame exchange: no LINK at all
    (first XPS pass). jab-drop2 presses at frame 34, after the CPU is free and inside 20 frames."""
    time.sleep(0.5)
    ctx.play('jab-drop2')
    time.sleep(2.5)


def setup_input(ctx):
    return _strip(ctx)


def clip_input(ctx):
    time.sleep(0.5)
    ctx.play('jab-drop2')
    time.sleep(2.5)


def setup_last(ctx):
    return _strip(ctx)


def clip_last(ctx):
    time.sleep(0.5)
    ctx.play('jab-link-3')
    time.sleep(3.0)


def setup_exchange(ctx):
    return _strip(ctx)


def clip_exchange(ctx):
    time.sleep(0.5)
    ctx.play('exchange-jabs')
    time.sleep(2.5)


# --- recording --------------------------------------------------------------------------------

def _away(ctx, fixture):
    """Fixture, then the spawn reset, so the play's warp to the start save shows."""
    accept.prep(ctx, fixture)
    if steps._shield_index(ctx) != 0:  # settings.json; hook modules reload per step, so no module state
        steps.cpu_shield(ctx, 'none')
    ctx.play(RESET)
    time.sleep(0.8)
    steps.settle(ctx)
    return {'fixture': fixture, 'start': 'spawn'}


def setup_play_chord(ctx):
    return _away(ctx, 'play-chord')


def clip_play_chord(ctx):
    time.sleep(0.8)
    ctx.play(steps.PLAY_CPU)
    time.sleep(1.4)
    ctx.play(steps.PLAY_CPU)          # the second press stops it
    time.sleep(2.0)


def setup_loop(ctx):
    return _away(ctx, 'loop2')


def clip_loop(ctx):
    time.sleep(0.8)
    ctx.play(steps.PLAY_CPU)
    time.sleep(4.0)


def setup_chain(ctx):
    """chain2: macro 1 walks left for 30 frames then jabs (the first pass's two plain jabs did not read)."""
    return _away(ctx, 'chain2')


def clip_chain(ctx):
    time.sleep(0.8)
    ctx.play(steps.PLAY_CPU)
    time.sleep(4.5)


def setup_mismatch(ctx):
    return _away(ctx, 'mismatch')


def clip_mismatch(ctx):
    time.sleep(0.6)
    ctx.play(steps.PLAY_CPU)
    time.sleep(2.0)                   # the prompt is up
    steps.press(ctx, 'a', 0.3)        # Keep current fighter (the cursor's row)
    time.sleep(3.0)


def _macro_column(ctx):
    """Recording page, cursor on the first macro file. open_library_item without its final A."""
    steps.open_menu(ctx)
    steps.goto_page(ctx, 'recording')
    steps._jump_top(ctx)
    before = steps._last_page_line(ctx)
    steps.press(ctx, 'right', 0.4)
    landed = steps._await_page_line(ctx, before)
    if steps._page_field(landed, 'header') != 'on':
        before = landed
        steps.press(ctx, 'right', 0.4)
        landed = steps._await_page_line(ctx, before)
    if steps._page_field(landed, 'header') != 'on':
        raise RuntimeError('saves column not shown (%s)' % landed[-180:])
    steps._library_index_zero(ctx)
    before = steps._last_page_line(ctx)
    steps.press(ctx, 'right', 0.45)
    line = steps._await_page_line(ctx, before)
    if not steps._macros_have_file(line):
        raise RuntimeError('macro column has no file (%s)' % line[-180:])
    return line


def setup_macro_screen(ctx):
    accept.prep(ctx, 'play-warp')
    return {'page': _macro_column(ctx)}


def clip_macro_screen(ctx):
    time.sleep(1.0)
    steps.press(ctx, 'a', 0.3)
    time.sleep(3.5)


def _open_first_combo(ctx):
    """Left column Down onto Combos & Sequences, Right into its list, A on the first combo.

    The first XPS pass used ZL+Down here: the gates log shows `press JumpDown … cursor=0` with no
    move, so Right and A opened the jabs3 macro instead (its rows ended at Delete). Down from cursor 0
    stays in the saves/macros library. The group cursor is 1 (`group=combos-sequences`); Up from the
    Options rows reaches it, so this presses Up until a [Ui] line says cursor=1."""
    steps.open_menu(ctx)
    steps.goto_page(ctx, 'recording')
    offset = steps._gate_offset(ctx)
    reached = False
    for _ in range(24):
        steps.press(ctx, 'up', 0.3)
        offset, match = steps._await_ui(ctx, offset, cap=1.2)
        if match is not None and match.group('page') == 'recording' and match.group('cursor') == '1':
            reached = True
            break
    if not reached:
        raise RuntimeError('Combos & Sequences group (cursor 1) not reached')
    steps.press(ctx, 'right', 0.45)
    offset = steps._gate_offset(ctx)
    steps.press(ctx, 'a', 0.6)
    offset, title = steps._await_title(ctx, offset)
    if not title:
        raise RuntimeError('no combo file opened')
    return offset, title


def _drill_first_combo(ctx):
    offset, title = _open_first_combo(ctx)
    steps.down_to_file_row(ctx, offset, title, 'Drill')
    steps.press(ctx, 'a', 0.6)
    steps.close_menu(ctx)


def setup_make_combo(ctx):
    """Combo 1 from the three-jab macro, drilled; then the cursor on that macro's Make combo row."""
    accept.prep(ctx, 'jabs3')
    steps._open_file_row(ctx, 'Make combo')
    steps.press(ctx, 'a', 0.6)
    time.sleep(3.5)
    steps.close_menu(ctx)
    _drill_first_combo(ctx)
    ctx.play(RESET)
    time.sleep(0.8)
    steps._open_file_row(ctx, 'Make combo')
    return {'drill': 'combo 1', 'cursor': 'Make combo'}


def clip_make_combo(ctx):
    time.sleep(0.8)
    steps.press(ctx, 'a', 0.3)
    time.sleep(5.0)


def setup_crop_combo(ctx):
    """The combos made by setup_make_combo are on the card. Open the first, cursor on Crop."""
    offset, title = _open_first_combo(ctx)
    steps.down_to_file_row(ctx, offset, title, 'Crop')
    return {'cursor': 'Crop'}


def clip_crop_combo(ctx):
    time.sleep(0.8)
    steps.press(ctx, 'a', 0.6)        # the handle screen
    time.sleep(0.8)
    steps.press(ctx, 'r', 0.6)        # move a handle
    time.sleep(0.6)
    steps.press(ctx, 'a', 0.6)        # save the kept links as a new combo
    time.sleep(2.0)
