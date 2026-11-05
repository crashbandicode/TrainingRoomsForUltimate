"""Q7's last item, run by the primary (35137355) on the Yoga lane's live a12 boot during the lane's provider outage
(2026-11-04 ~23:1xZ): the Deluxe Plugins page row (TrainingRooms' own `deluxe_row.rs` "Replay capture", plugin
"Training Rooms") stays in sync with the TrainingRooms menu toggle. Before any change, after the change to Off and
after the change back to On, the Deluxe overlay is stepped page by page (L+R+Down, `pilot-page-step`) until it closes,
with a screenshot per page; the primary reviews which page is Plugins. Reuses the lane's steps2.py helpers unchanged.
"""
import importlib.util
import json
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent


def _load(name, path):
    spec = importlib.util.spec_from_file_location(name, HERE / path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


s2 = _load('p23steps2_primary_q7', 'steps2.py')
b = s2.b
steps = s2.steps


def _tr_bright(path):
    """Share of bright pixels in the top-right HUD corner (proportional crop): the Deluxe overlay darkens it."""
    from PIL import Image
    with Image.open(path) as image:
        gray = image.convert('L')
        w, h = gray.size
        px = list(gray.crop((int(w * 0.78), 0, int(w * 0.96), int(h * 0.09))).getdata())
    return sum(p > 140 for p in px) / len(px)


def _tour(ctx, label, baseline, max_steps=7):
    pages = []
    for i in range(max_steps):
        ctx.play('pilot-page-step')
        time.sleep(1.5)
        shot = b.capture(ctx, f'q7dx-{label}-p{i}', f'{label}: the Deluxe overlay after {i + 1} L+R+Down')
        frac = _tr_bright(shot)
        pages.append({'shot': shot, 'bright': round(frac, 3)})
        if i > 0 and abs(frac - baseline) < 0.08:
            break
    return pages


def _toggle(ctx, out, label, first):
    ctx.gate('training-hud-unpaused')
    ctx.play('tr-menu-open')
    ctx.wait_log('diag', r'\[Ui\] menu opened', cap=15)
    time.sleep(1.5)  # the menu's open fade discards the first inputs
    if first:
        def navigate():
            for _ in range(14):  # rows only (headers skipped): land on Replay capture
                ctx.press('down', 150)
                time.sleep(0.35)
        out['route'] = s2._route(ctx, 'q7-menu-to-row', navigate)
        out['row_shot'] = b.capture(ctx, 'q7dx-replay-row', 'Recording page, focus on the Replay capture row')
    tick = s2._now_tick(ctx)
    ctx.press('a', 150)
    out[f'{label}_menu_shot'] = b.capture(ctx, f'q7dx-{label}-menu', f'{label}: the Replay capture row after A')
    steps.close_menu(ctx)
    ctx.gate('training-hud-unpaused')
    out[f'{label}_write'] = s2._wait_tick(ctx, tick, 'replay_capture.ini written', 20)[-160:]
    out[f'{label}_ini'] = s2.copy_ini(ctx, 'replay_capture.ini', f'q7dx-{label}')


def q7dx2(ctx):
    """Resume after q7dx stopped at its second toggle (on reopen this boot's menu cursor was back on the first
    Recording row, so A opened Record Slot; nothing changed). Every open now replays the recorded route
    `q7-menu-to-row` from the top. Starts from the current file value (Off after q7dx's first change)."""
    out = {'run_by': 'primary 35137355', 'resume_of': 'p23-q7-deluxe-primary'}
    try:
        settled = steps.settle(ctx)
    except Exception as error:
        settled = b.recover(ctx, f'settle after {error}')
    ctx.log({'primary_q7dx2_settle': settled})
    ctx.gate('training-hud-unpaused')
    out['start_ini'] = s2.copy_ini(ctx, 'replay_capture.ini', 'q7dx2-start')
    base = b.capture(ctx, 'q7dx2-closed-baseline', 'Training HUD with no overlay: the brightness baseline')
    baseline = _tr_bright(base)
    out['baseline'] = {'shot': base, 'bright': round(baseline, 3)}
    out['tour_start'] = _tour(ctx, 'start2', baseline)
    for label in ('on', 'off2'):
        _toggle(ctx, out, label, first=True)
        out[f'tour_{label}'] = _tour(ctx, label, baseline)
        (ctx.row_dir / 'q7dx2.json').write_text(json.dumps(out, indent=2, default=str) + '\n')
    ctx.gate('training-hud-unpaused')
    return out


def q7dx(ctx):
    out = {'run_by': 'primary 35137355'}
    try:
        settled = steps.settle(ctx)
    except Exception as error:  # a CSS/unknown state: the lane's known-screen recover routes
        settled = b.recover(ctx, f'settle after {error}')
    ctx.log({'primary_q7dx_settle': settled})
    b.fixture(ctx, 'p23-q7.json', fresh=True)  # last_page=recording (pushed before any menu open)
    ctx.gate('training-hud-unpaused')
    out['start_ini'] = s2.copy_ini(ctx, 'replay_capture.ini', 'q7dx-start')
    base = b.capture(ctx, 'q7dx-closed-baseline', 'Training HUD with no overlay: the brightness baseline')
    baseline = _tr_bright(base)
    out['baseline'] = {'shot': base, 'bright': round(baseline, 3)}
    out['tour_start'] = _tour(ctx, 'start', baseline)
    (ctx.row_dir / 'q7dx.json').write_text(json.dumps(out, indent=2, default=str) + '\n')
    _toggle(ctx, out, 'off', first=True)
    out['tour_off'] = _tour(ctx, 'off', baseline)
    (ctx.row_dir / 'q7dx.json').write_text(json.dumps(out, indent=2, default=str) + '\n')
    _toggle(ctx, out, 'on', first=False)
    out['tour_on'] = _tour(ctx, 'on', baseline)
    (ctx.row_dir / 'q7dx.json').write_text(json.dumps(out, indent=2, default=str) + '\n')
    ctx.gate('training-hud-unpaused')
    return out
