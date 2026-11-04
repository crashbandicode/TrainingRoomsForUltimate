"""Self-correcting navigation to Training for eden-runner rows (primary 35137355, 2026-11-04).

Why: the opening chain plays recorded segments blind between gates. One press lost to a menu transition (e.g.
`main-stage`'s first `right` while the main menu is still animating, so A opens Smash instead of Games & More),
or a loading tip over the Training HUD, failed the whole boot with no way back. This hook looks at which known
screen the game is on (the rows' own image gates with short caps) and takes the step that leads from THAT screen
toward the target, until the target gate passes. Every input still goes through the runner (pace cap, STOP,
actions.jsonl).

Rows use it as one step, e.g. {"run": "nav_recover.py:to_training", "check": true}. Gates and segments come from
rows.json: title, main, stage, css, steve-selected, ready, training-hud-unpaused, native-pause; main-stage,
stage-css, steve-hover, steve-place, lr04-cpu-pick, enter-training, pause-close (and intro-cancel if present).
"""
import time

PROBE_CAP = 2          # s per screen probe
MAX_STEPS = 14         # actions before giving up (each one is logged)


def _probe(ctx, name, cap=PROBE_CAP):
    gates = ctx.runner.plan.get('gates', {})
    if name not in gates:
        return False
    try:
        ctx.runner.check_gate_spec(ctx.on, {**gates[name], 'cap': cap}, f'probe-{name}')
        return True
    except RuntimeError:
        return False


def _screen(ctx):
    """The first known screen that matches, most specific first."""
    for name in ('training-hud-unpaused', 'native-pause', 'ready', 'steve-selected', 'css', 'stage', 'main', 'title'):
        if _probe(ctx, name):
            return name
    return None


def _has_segment(ctx, name):
    return name in ctx.runner.plan.get('segments', {})


def _step(ctx, screen, unknown_run):
    """One action from `screen` toward Training. Returns a short description for the log."""
    if screen == 'native-pause':
        ctx.play('pause-close')
        return 'pause-close'
    if screen == 'ready':
        ctx.play('enter-training')
        _await_training(ctx)
        return 'enter-training'
    if screen == 'steve-selected':
        ctx.play('lr04-cpu-pick')
        time.sleep(1.0)
        ctx.press('a', 120)          # drop the CPU coin where the hand is (CPU = Random; fixtures set fighters)
        time.sleep(1.2)
        return 'cpu-coin'
    if screen == 'css':
        ctx.play('steve-hover')
        time.sleep(0.6)
        ctx.play('steve-place')
        time.sleep(1.0)
        return 'steve'
    if screen == 'stage':
        time.sleep(1.0)              # let the stage select settle before the recorded route
        ctx.play('stage-css')
        return 'stage-css'
    if screen == 'main':
        time.sleep(1.5)              # the main menu ignores input while it animates in
        ctx.play('main-stage')
        return 'main-stage'
    if screen == 'title':
        ctx.press('a', 120)
        time.sleep(3.0)
        return 'title-a'
    # Unknown: a submenu reached by a lost press (Smash, Tourney, Help), a dialog, or the attract demo.
    if unknown_run % 3 == 2 and _has_segment(ctx, 'intro-cancel'):
        ctx.play('intro-cancel')
        return 'intro-cancel'
    ctx.press('b', 120)
    time.sleep(1.5)
    return 'back'


def _await_training(ctx):
    """After enter-training: the loading screen, then the HUD. A loading tip can sit over the HUD; A dismisses it."""
    for attempt in range(3):
        if _probe(ctx, 'training-hud-unpaused', cap=10 if attempt == 0 else 4):
            return True
        ctx.press('a', 120)
        time.sleep(1.0)
    return False


def to_training(ctx, **_args):
    path, unknown_run = [], 0
    for _ in range(MAX_STEPS):
        ctx.check_stop()
        screen = _screen(ctx)
        if screen == 'training-hud-unpaused':
            ctx.log({'nav_recover': 'training', 'path': path})
            return {'end': 'training', 'path': path}
        unknown_run = unknown_run + 1 if screen is None else 0
        action = _step(ctx, screen, unknown_run)
        path.append(f'{screen or "unknown"}>{action}')
        ctx.log({'nav_recover_step': path[-1]})
    ctx.gate('training-hud-unpaused')    # last word: raises with the gate's own error if still not there
    return {'end': 'training', 'path': path}


def detour_smash(ctx, **_args):
    """Test helper: from the main menu, press A on Smash (the exact wrong screen a lost `right` produces)."""
    ctx.gate('main')
    time.sleep(1.5)
    ctx.press('a', 120)
    time.sleep(2.0)
    ctx.shot('detour-smash')
    return {'detour': 'smash'}
