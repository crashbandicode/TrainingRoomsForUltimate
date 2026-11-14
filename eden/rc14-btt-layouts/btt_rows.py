"""Break the Targets Eden rows (btt-3, rc.14 SPEC V2.5, acceptance rows 26-31 and 41-43). Authored offline against the diag
build's `[Btt]` lines; nothing here has run in Eden yet (see EDEN-ROWS.md, "Not run").

The diag build prints, whenever the notice text or the phase changes (and per tick while Running):
  [Btt] phase=<Off|Placing|Ready|Running|Done> n=<targets> cursor=<None|Some([x, y])> p1=[x, y, z] text="<notice>"
  [Btt] camera want=<bool> mode_before=<n> mode_after=<n> changed=<bool>
Rows gate on those lines, never on a sleep, and read the cursor and P1's position from them. Inputs are frame-exact macros
(make_btt_segments.py): a stick held N frames moves the cursor 2 N units, every chord goes down in one frame. No extra B
press after a menu close: in Ready it would be the first input and start the clock (`steps.close_menu` presses B twice).
"""
import json
import re
import sys
import time
from pathlib import Path

# The runner loads a hook by file path; the shared helpers (steps, hotslots, fx) sit beside it.
if str(Path(__file__).resolve().parent) not in sys.path:
    sys.path.insert(0, str(Path(__file__).resolve().parent))

import cov  # noqa: E402
import fx  # noqa: E402
import hotslots as hs  # noqa: E402
import steps  # noqa: E402

ON, OFF = [0, 1], [1, 0]  # the `btt` row, one-hot [Off, On]
PRIMS = (40, 20, 10, 5, 2, 1)  # frames of the btt-r/l/u/d segments
TOL = 8.0  # units a placed target may differ from where the macros aimed before the row fails
STAGE_IDS = {'bf': 0, 'bbf': 2}  # model::stages::lobby_stages::STAGES: Battlefield, Big Battlefield; checked against the game
STAGE_RIGHTS = {'bf': 1, 'bbf': 3}  # Change Stage's grid opens on Waiting Room; this many tiles right (stage_cam / camshot)
ATTACKS = ('jab', 'ftilt', 'sh-nair', 'fsmash')
HUD = 'training-hud-unpaused'

_NUM = r'(-?[\d.]+(?:e[+-]?\d+)?)'
_CAMERA = re.compile(r'\[Btt\] camera want=(\w+) mode_before=(?:Some\()?(-?\d+)\)? mode_after=(-?\d+) changed=(?:Some\()?(\w+)\)?')
_COUNT = re.compile(r'Targets (\d+)/(\d+)   (\d+):(\d\d)\.(\d\d)')
_DONE = re.compile(r'Done! (\d+) in (\d+):(\d\d)\.(\d\d) \(best (\d+):(\d\d)\.(\d\d)\)')


# --- the [Btt] lines -------------------------------------------------------------------------------------------------

def parse(text):
    """One `[Btt] phase=` line as a dict (phase, n, cursor, p1, text), None for any other line."""
    found = re.search(r'\[Btt\] phase=(\w+) n=(\d+)', text)
    if not found:
        return None
    cursor = re.search(r'cursor=Some\(\[' + _NUM + ', ' + _NUM + r'\]\)', text)
    p1 = re.search(r'p1=\[' + _NUM + ', ' + _NUM + ', ' + _NUM + r'\]', text)
    shown = re.search(r'text="([^"]*)"', text)
    return {'phase': found.group(1), 'n': int(found.group(2)),
            'cursor': (float(cursor.group(1)), float(cursor.group(2))) if cursor else None,
            'p1': tuple(float(p1.group(i)) for i in (1, 2, 3)) if p1 else None,
            'text': shown.group(1) if shown else ''}


def since(ctx, mark):
    """(seq, state) of every `[Btt] phase=` line after `mark`."""
    return [(seq, st) for seq, _, text in hs.diag_since(ctx, mark) for st in [parse(text)] if st]


def latest(ctx):
    """The newest state in the log, or None."""
    found = since(ctx, 0)
    return found[-1][1] if found else None


def cameras(ctx, mark):
    """The `[Btt] camera` lines after `mark`: dicts (want, before, after, changed)."""
    out = []
    for _, _, text in hs.diag_since(ctx, mark):
        found = _CAMERA.search(text)
        if found:
            out.append({'want': found.group(1) == 'true', 'before': int(found.group(2)), 'after': int(found.group(3)),
                        'changed': found.group(4) == 'true', 'line': text[:200]})
    return out


def wait_state(ctx, mark, test, cap, what):
    """The first state after `mark` that satisfies `test`, else a RuntimeError with the last states seen."""
    deadline = time.time() + cap
    seen = []
    while time.time() < deadline:
        ctx.check_stop()
        seen = since(ctx, mark)
        for _, st in seen:
            if test(st):
                return st
        time.sleep(0.25)
    raise RuntimeError('%s: not seen in %.0f s; last states %s' % (what, cap, [s for _, s in seen[-4:]]))


def wait_camera(ctx, mark, want, after, cap=10, what='camera'):
    deadline = time.time() + cap
    while time.time() < deadline:
        ctx.check_stop()
        for line in cameras(ctx, mark):
            if line['want'] == want and line['after'] == after:
                return line
        time.sleep(0.3)
    raise RuntimeError('%s: no `[Btt] camera want=%s ... mode_after=%d` line in %.0f s; saw %s'
                       % (what, str(want).lower(), after, cap, cameras(ctx, mark)[-3:]))


def p1_of(st):
    if st is None or not st['p1']:
        raise RuntimeError('the [Btt] line has no p1=[x, y, z]: %s' % st)
    return st['p1']


def count_of(text):
    found = _COUNT.search(text)
    return (int(found.group(1)), int(found.group(2))) if found else None


def clock_of(text):
    found = _COUNT.search(text)
    return hundredths(*found.group(3, 4, 5)) if found else None


def hundredths(m, s, c):
    return (int(m) * 60 + int(s)) * 100 + int(c)


def save(ctx, name, value):
    return hs.save(ctx, name, value)


# --- inputs ----------------------------------------------------------------------------------------------------------

def settle(ctx):
    ctx.gate(HUD)


def set_row(ctx, on):
    """The Break the Targets row through the live settings reload (no menu), as the other rows' hooks set theirs."""
    fx._live_set(ctx, btt=ON if on else OFF)


def close_menu(ctx):
    """The plugin menu closed without a stray B afterwards: a B after the 15-frame grace is the game's, and in Ready it is
    the first input."""
    ctx.play(steps.MENU_CLOSE)
    time.sleep(0.6)


def push(ctx, direction, frames):
    for prim in PRIMS:
        while frames >= prim:
            ctx.play('btt-%s%d' % (direction, prim))
            frames -= prim


def goto(ctx, cursor, x, y):
    """The cursor from `cursor` to (x, y) with frame-exact stick macros (2 units a frame); to the nearest 2."""
    dx, dy = x - cursor[0], y - cursor[1]
    push(ctx, 'r' if dx > 0 else 'l', round(abs(dx) / 2))
    push(ctx, 'u' if dy > 0 else 'd', round(abs(dy) / 2))


def place_at(ctx, cursor, x, y, expect_n):
    """Move to (x, y), A, and wait for the line that says `expect_n` targets. The cursor the game reports."""
    goto(ctx, cursor, x, y)
    mark = hs.diag_mark(ctx)
    ctx.play('btt-a')
    st = wait_state(ctx, mark, lambda s: s['n'] == expect_n and s['cursor'] is not None, 6, 'target %d placed' % expect_n)
    miss = max(abs(st['cursor'][0] - x), abs(st['cursor'][1] - y))
    if miss > TOL:
        raise RuntimeError('the cursor is at %s, aimed at (%s, %s): off by %.1f (> %.1f)' % (st['cursor'], x, y, miss, TOL))
    return st


def layout_near(st):
    """Three targets for P1: 14, 34 and 54 units in front of it, at a fighter's chest (the cursor starts 15 up; the macros
    move on a 2-unit grid, so 6 down is exact: 3 frames). x is rounded by `goto` to the grid; `place_at` checks the miss."""
    px = p1_of(st)[0]
    y = st['cursor'][1] - 6
    return [(px + 10, y), (px + 18, y), (px + 26, y)]  # btt-4: 14/34/54 put the third past a side platform's edge

def put_layout(ctx, st, out):
    """Place the three targets of `layout_near`, nearest the cursor first; the states reported after each A."""
    spots = sorted(layout_near(st), key=lambda t: abs(t[0] - st['cursor'][0]))
    cursor, placed = st['cursor'], []
    for k, (x, y) in enumerate(spots, 1):
        got = place_at(ctx, cursor, x, y, k)
        cursor = got['cursor']
        placed.append({'aim': [x, y], 'cursor': list(cursor), 'text': got['text']})
    out.setdefault('placed', []).extend(placed)
    hs._remember(ctx, 'btt_layout', [p['cursor'] for p in placed])
    hs._remember(ctx, 'btt_p1', list(p1_of(st)))
    return placed


def to_placing(ctx, out, tag=''):
    """Placing through the Place targets row (SPEC V2.5); if the row path fails the live setting plus a menu close does it
    (the mode On with no targets), and the receipt says so. The state of the Placing line."""
    mark = hs.diag_mark(ctx)
    try:
        steps.open_menu(ctx)
        steps.goto_page(ctx, 'fighters_stages')
        if not steps._focus_row(ctx, 'Place targets'):
            raise RuntimeError('the Place targets row was not focused')
        steps.press(ctx, 'a', 0.8)
        # btt-4: the `[Btt] phase=` line prints when the notice text changes. Already Placing with the same text (a run
        # that was Placing before the menu), the close re-opens Placing and prints nothing: the latest state then stands.
        was = latest(ctx)
        if was is not None and was['phase'] == 'Placing':
            try:
                return wait_state(ctx, mark, lambda s: s['phase'] == 'Placing', 3, 'Placing after the Place targets row')
            except RuntimeError:
                settle(ctx)  # the menu did close: the HUD is back unpaused
                out['placing_unchanged' + tag] = was['text']
                return was
        return wait_state(ctx, mark, lambda s: s['phase'] == 'Placing', 12, 'Placing after the Place targets row')
    except RuntimeError as error:
        out['menu_route_failed' + tag] = str(error)[:300]
    close_menu(ctx)  # a menu the first route left open
    mark = hs.diag_mark(ctx)
    set_row(ctx, True)
    steps.open_menu(ctx)
    close_menu(ctx)
    return wait_state(ctx, mark, lambda s: s['phase'] == 'Placing', 12, 'Placing after a menu close with the mode On')


def clear_layout(ctx):
    mark = hs.diag_mark(ctx)
    ctx.play('btt-y2')
    return wait_state(ctx, mark, lambda s: s['phase'] == 'Placing' and s['n'] == 0, 6, 'Y twice clears')


# --- saved layouts: one file each in layouts/ (btt-5, artifacts/rc14-btt-layouts/DESIGN.md) ---------------------------------

STAGE_NAMES = {'bf': 'Battlefield', 'bbf': 'Big Battlefield'}  # the lobby catalog's names (the default file names)
SEEDED_ID, SEEDED_NAME = 'seeded-big-battlefield', 'Seeded · Big Battlefield'  # make_btt_fixture.py
_LISTED = re.compile(r'\[Btt\] layouts stage=(-?\d+) files=(\d+) hot=(\d+) skipped=(\d+) saved=(\w+) first="([^"]*)"')
_PICK = re.compile(r'^Place targets (\d+)/10   (\d+)/(\d+) (.*): (\d+)$')
_VIEW = re.compile(r'\[NamedMacroDiag\] view title=("(?:[^"\\]|\\.)*") footer=("(?:[^"\\]|\\.)*") selected=(\d+) labels=(\[.*\])')
OWN_ROWS, ASK_ROWS = ['Load', 'Rename', 'Duplicate', 'Delete'], ['Keep it', 'Delete it']


def layout_dir(ctx):
    return Path(ctx.instance['settings']).parent / 'layouts'


def layouts(ctx, stage=None):
    """The saved layout files as (id, doc), of `stage` only when given, the newest use first."""
    found = []
    folder = layout_dir(ctx)
    for path in sorted(folder.glob('*.json')) if folder.is_dir() else []:
        try:
            doc = json.loads(path.read_text())
        except ValueError:
            continue
        if stage is None or doc.get('stage') == stage:
            found.append((path.stem, doc))
    return sorted(found, key=lambda kv: (-kv[1].get('used', 0), kv[0]))


def list_order(files):
    """Recording's order of a stage's files: the five used last, newest first, then the rest by name (the plugin's `listed`)."""
    hot = files[:5]
    return [d['name'] for _, d in hot] + sorted(d['name'] for _, d in files[5:])


def wait_layouts(ctx, test, cap=8, what='the layouts folder'):
    """`layouts(ctx)` once `test` holds (the writer thread's drain is up to 100 ms)."""
    deadline = time.time() + cap
    got = []
    while time.time() < deadline:
        ctx.check_stop()
        got = layouts(ctx)
        if test(got):
            return got
        time.sleep(0.3)
    raise RuntimeError('%s: not reached in %.0f s (files %s)' % (what, cap, [(k, d.get('name'), d.get('used')) for k, d in got]))


def stray(ctx):
    folder = layout_dir(ctx)
    return [p.name for p in folder.iterdir() if not p.name.endswith('.json')] if folder.is_dir() else []


def listed(ctx, mark):
    """The `[Btt] layouts` lines after `mark` (the writer's list of the stage after a change), as dicts."""
    out = []
    for _, _, text in hs.diag_since(ctx, mark):
        found = _LISTED.search(text)
        if found:
            out.append({'stage': int(found.group(1)), 'files': int(found.group(2)), 'hot': int(found.group(3)),
                        'skipped': int(found.group(4)), 'saved': found.group(5) == 'true', 'first': found.group(6)})
    return out


def wait_listed(ctx, mark, test, cap, what):
    deadline = time.time() + cap
    while time.time() < deadline:
        ctx.check_stop()
        for line in listed(ctx, mark):
            if test(line):
                return line
        time.sleep(0.25)
    raise RuntimeError('%s: no matching `[Btt] layouts` line in %.0f s; saw %s' % (what, cap, listed(ctx, mark)[-3:]))


def picked(text):
    """The Placing notice with a saved layout chosen: dict(n, index, count, name, targets), else None."""
    found = _PICK.match(text or '')
    if not found:
        return None
    return {'n': int(found.group(1)), 'index': int(found.group(2)), 'count': int(found.group(3)), 'name': found.group(4),
            'targets': int(found.group(5))}


def zl_step(ctx, right=True):
    """ZL + Right (or Left) while Placing: the chosen layout the notice names."""
    mark = hs.diag_mark(ctx)
    ctx.play('btt-zl-right' if right else 'btt-zl-left')
    st = wait_state(ctx, mark, lambda s: picked(s['text']) is not None or s['text'] == 'No saved layouts on this stage yet', 6,
                    'ZL + %s' % ('Right' if right else 'Left'))
    return picked(st['text']) or {'none': st['text']}


def lowest_free(names, stage_name):
    taken = {n.lower() for n in names}
    n = 1
    while ('Layout %d · %s' % (n, stage_name)).lower() in taken:
        n += 1
    return 'Layout %d · %s' % (n, stage_name)


def save_new(ctx, stage, stage_name):
    """ZL + Down while Placing: a NEW file named the lowest free `Layout N · <stage>`; the notice says so and the file
    appears beside the others, none of which changes. The new (id, doc)."""
    before = {k: (layout_dir(ctx) / ('%s.json' % k)).read_bytes() for k, _ in layouts(ctx)}
    want = lowest_free([d['name'] for _, d in layouts(ctx, stage)], stage_name)
    mark = hs.diag_mark(ctx)
    ctx.play('chord-save')
    wait_state(ctx, mark, lambda s: s['text'] == 'Saved: ' + want, 8, 'ZL + Down saves %s' % want)
    got = wait_layouts(ctx, lambda ls: any(k not in before for k, _ in ls), 6, 'the new file of %s' % want)
    new = [(k, d) for k, d in got if k not in before]
    changed = [k for k, b in before.items() if (layout_dir(ctx) / ('%s.json' % k)).read_bytes() != b]
    if len(new) != 1 or new[0][1].get('name') != want or new[0][1].get('stage') != stage or changed:
        raise RuntimeError('a Save made %s (want one file %r on stage %d) and changed %s' % (new, want, stage, changed))
    return new[0]


def view(ctx):
    """What the library panel shows (`[NamedMacroDiag] view`): title, footer, selected, labels."""
    line = str(cov._panel_view(ctx))
    found = _VIEW.search(line)
    if not found:
        raise RuntimeError('no panel view: %s' % line[:200])
    return {'title': json.loads(found.group(1)), 'footer': json.loads(found.group(2)), 'selected': int(found.group(3)),
            'labels': json.loads(found.group(4))}


def open_list(ctx):
    """The menu on Fighters & Stages, Saved layouts, A: this stage's list (the panel under the drawn list carries its rows in
    the list's order)."""
    steps.open_menu(ctx)
    steps.goto_page(ctx, 'fighters_stages')
    if not steps._focus_row(ctx, 'Saved layouts'):
        raise RuntimeError('the Saved layouts row was not focused')
    steps.press(ctx, 'a', 1.0)
    return view(ctx)


def leave_list(ctx):
    """B closes the list, then the menu closes (no stray B: in Ready it would start the clock)."""
    steps.press(ctx, 'b', 0.8)
    close_menu(ctx)


def own_row(ctx, index, labels=OWN_ROWS):
    """From the list (the cursor on the layout): A opens its own screen; Right `index` times and A does that row. The own
    screen's view before the press."""
    steps.press(ctx, 'a', 0.8)
    own = view(ctx)
    if own['labels'] != labels:
        raise RuntimeError('the own screen shows %s, not %s' % (own['labels'], labels))
    for _ in range(index):
        steps.press(ctx, 'right', 0.4)
    steps.press(ctx, 'a', 1.0)
    return own


# --- Change Stage (stage_cam.py's route) ---------------------------------------------------------------------------

def change_stage(ctx, key='bf'):
    """Change Stage to Battlefield or Big Battlefield with the mode Off, the stage id checked against the game's request."""
    out = {'stage': key}
    set_row(ctx, False)
    mark = hs.diag_mark(ctx)
    steps.open_menu(ctx)
    steps.goto_page(ctx, 'fighters_stages')
    if not steps._focus_row(ctx, 'Change Stage'):
        steps.close_menu(ctx)
        raise RuntimeError('harness: the Change Stage row was not focused')
    steps.press(ctx, 'a', 1.5)
    for _ in range(STAGE_RIGHTS[key]):
        steps.press(ctx, 'right', 0.6)
    out['tile'] = str(ctx.shot('btt-tile-%s' % key))
    steps.press(ctx, 'a', 1.0)
    time.sleep(1.0)
    for _ in range(3):
        try:
            ctx.gate(HUD)
            break
        except RuntimeError:
            steps.press(ctx, 'b', 0.8)
    time.sleep(8.0)
    for _ in range(20):
        try:
            ctx.gate(HUD)
            break
        except RuntimeError:
            time.sleep(1.0)
    # `id=` is the plugin's request counter; the stage is the selection's own field (the trailing `stage=`).
    request = hs.wait_diag(ctx, mark, r'\[PracticeStageRequest\] queued id=\d+ selection=Selection \{ stage: \d+', 20)
    got = int(re.search(r'selection=Selection \{ stage: (\d+)', request[2]).group(1)) if request else None
    out['stage_id'] = got
    hs._remember(ctx, 'stage_' + key, got)
    save(ctx, 'change-stage-' + key, out)
    if got != STAGE_IDS[key]:
        raise RuntimeError('Change Stage reached stage id %s, the stage table says %s for %s' % (got, STAGE_IDS[key], key))
    time.sleep(2.0)
    ctx.play(steps.RESET_CENTER)
    time.sleep(1.5)
    settle(ctx)
    return out


# --- rows --------------------------------------------------------------------------------------------------------------

def e26_menu(ctx):
    """The Place targets row closes the menu into the placing view with the mode turned On (SPEC V2.5 Placing)."""
    out = {}
    mark = hs.diag_mark(ctx)
    was = latest(ctx)
    set_row(ctx, False)
    if was and was['phase'] != 'Off':
        wait_state(ctx, mark, lambda s: s['phase'] == 'Off', 10, 'the mode Off before the row')
    mark = hs.diag_mark(ctx)
    steps.open_menu(ctx)
    steps.goto_page(ctx, 'fighters_stages')
    if not steps._focus_row(ctx, 'Place targets'):
        steps.close_menu(ctx)
        raise RuntimeError('the Place targets row was not focused')
    out['shot_row'] = str(ctx.shot('btt-E26m-row'))
    steps.press(ctx, 'a', 0.8)
    st = wait_state(ctx, mark, lambda s: s['phase'] == 'Placing', 12, 'Placing after the Place targets row')
    time.sleep(1.0)
    out['shot_placing'] = str(ctx.shot('btt-E26m-placing'))
    out['placing'] = st
    for _ in range(20):  # the writer thread saves the row a moment after the menu closes
        out['row_on'] = row_value(ctx)
        if out['row_on'] == ON:
            break
        time.sleep(0.5)
    save(ctx, 'e26m', out)
    if out['row_on'] != ON:
        raise RuntimeError('Place targets did not turn Break the Targets On (settings.json menu.btt = %s)' % out['row_on'])
    return {'placing_text': st['text'], 'row': out['row_on']}


def row_value(ctx):
    try:
        return json.loads(Path(ctx.instance['settings']).read_text())['menu'].get('btt')
    except (OSError, ValueError, KeyError):
        return None


def e26(ctx):
    """Row 26: place 3, an overlapping A is refused, B on release removes the last, B + D-pad Up opens the menu and removes
    nothing, the menu closed opens Ready with the layout."""
    out = {'problems': []}
    st = to_placing(ctx, out)
    if st['n']:
        st = clear_layout(ctx)
    ground = p1_of(st)[1]
    out['start'] = {'cursor': st['cursor'], 'p1': st['p1'], 'text': st['text']}
    if st['text'] != 'Place targets 0/10   A place  B undo  X start':
        out['problems'].append('standing text %r' % st['text'])
    if st['cursor'] is None or abs(st['cursor'][1] - (ground + 15.0)) > 1.0:
        out['problems'].append('the cursor starts at %s, not 15 above the ground (%.1f)' % (st['cursor'], ground))
    out['shot_start'] = str(ctx.shot('btt-E26-placing-0'))
    placed = put_layout(ctx, st, out)
    st = latest(ctx)
    if st['text'] != 'Place targets 3/10   A place  B undo  X start':
        out['problems'].append('after three A: %r' % st['text'])
    out['shot_three'] = str(ctx.shot('btt-E26-placing-3'))
    # An overlapping target is refused (the cursor is on the third).
    mark = hs.diag_mark(ctx)
    ctx.play('btt-a')
    refused = wait_state(ctx, mark, lambda s: s['text'] == 'Too close to another target', 6, 'overlap refused')
    if refused['n'] != 3:
        out['problems'].append('an overlapping A changed the count to %d' % refused['n'])
    # B on release removes the last.
    mark = hs.diag_mark(ctx)
    ctx.play('btt-b')
    wait_state(ctx, mark, lambda s: s['n'] == 2, 6, 'B removes the last target')
    out['shot_undo'] = str(ctx.shot('btt-E26-after-undo'))
    # B + D-pad Up opens the menu and removes nothing.
    mark = hs.diag_mark(ctx)
    steps.open_menu(ctx)
    time.sleep(1.5)
    out['shot_menu'] = str(ctx.shot('btt-E26-menu-open'))
    changed = [s for _, s in since(ctx, mark) if s['n'] != 2]
    if changed:
        out['problems'].append('B + D-pad Up changed the layout: %s' % changed[:2])
    mark = hs.diag_mark(ctx)
    close_menu(ctx)
    ready = wait_state(ctx, mark, lambda s: s['phase'] == 'Ready', 10, 'Ready after the menu closed with a layout')
    out['ready'] = ready
    if ready['n'] != 2:
        out['problems'].append('Ready with %d targets, not the 2 left' % ready['n'])
    save(ctx, 'e26', out)
    settle(ctx)
    if out['problems']:
        raise RuntimeError('; '.join(out['problems']))
    return {'placed': len(placed), 'ready_text': ready['text']}


def e28(ctx):
    """Row 28: X with no targets is refused; X with 3 goes to Ready at 0:00.00; the clock waits for the first input."""
    out = {'problems': []}
    st = to_placing(ctx, out)
    st = clear_layout(ctx) if st['n'] else st
    mark = hs.diag_mark(ctx)
    ctx.play('x-press')
    wait_state(ctx, mark, lambda s: s['text'] == 'Place at least one target', 6, 'X with no targets is refused')
    if latest(ctx)['phase'] != 'Placing':
        out['problems'].append('X with no targets left Placing')
    put_layout(ctx, latest(ctx), out)
    mark = hs.diag_mark(ctx)
    ctx.play('x-press')
    ready = wait_state(ctx, mark, lambda s: s['phase'] == 'Ready', 8, 'X with three targets starts Ready')
    if ready['text'] != 'Targets 0/3   0:00.00':
        out['problems'].append('Ready text %r' % ready['text'])
    out['shot_ready'] = str(ctx.shot('btt-E28-ready'))
    # The clock waits: no Running line in 6 s of wall time (3 s of game time at 30 fps or better).
    mark = hs.diag_mark(ctx)
    time.sleep(6.0)
    running = [s for _, s in since(ctx, mark) if s['phase'] == 'Running']
    if running:
        out['problems'].append('the clock started with no input: %s' % running[0])
    # The first input starts it.
    mark = hs.diag_mark(ctx)
    ctx.play('btt-a')
    first = wait_state(ctx, mark, lambda s: s['phase'] == 'Running', 6, 'the first input starts the clock')
    time.sleep(1.5)
    out['shot_running'] = str(ctx.shot('btt-E28-running'))
    clocks = [clock_of(s['text']) for _, s in since(ctx, mark) if s['phase'] == 'Running' and clock_of(s['text']) is not None]
    out['first_running'] = first
    out['running_lines'] = len(clocks)
    if len(clocks) < 3:
        out.setdefault('warnings', []).append('fewer than 3 Running lines after the first input (%d): the diag line may not print per tick' % len(clocks))
    mark = hs.diag_mark(ctx)
    ctx.play(steps.RESET_CENTER)
    wait_state(ctx, mark, lambda s: s['phase'] == 'Ready' and s['n'] == 3, 10, 'L+R+A returns to Ready with every target')
    save(ctx, 'e28', out)
    settle(ctx)
    if out['problems']:
        raise RuntimeError('; '.join(out['problems']))
    return {'ready_text': ready['text'], 'running_lines': len(clocks)}


def run_targets(ctx, targets, cap_s=90):
    """One run with P1: walk to within reach of each target (P1's x from the per-tick lines) and attack until the count rises.
    The Done state and the attempts made."""
    t0, run_mark = time.time(), hs.diag_mark(ctx)
    attempts, ref, last_x = [], hs._state(ctx).get('btt_p1'), None
    # Left to right: P1 walks right, and a hit may break two at once (the count then jumps past k).
    for k, (tx, _) in enumerate(sorted(targets, key=lambda t: t[0])):
        for attempt in range(10):
            ctx.check_stop()
            if time.time() - t0 > cap_s:
                raise RuntimeError('run longer than %d s at target %d; attempts %s' % (cap_s, k + 1, attempts))
            st = latest(ctx)
            if st['phase'] == 'Done' or (count_of(st['text']) or (0, 0))[0] > k:
                break
            # In Ready no line has P1's position since the reset put it back: it is where the layout was made.
            here = ref[0] if st['phase'] == 'Ready' and ref else p1_of(st)[0]
            gap = tx - here
            # btt-4: walk while it moves P1 (a platform's edge stops a walk), then attack from where it stands.
            if gap > 12 and here != last_x:
                last_x = here
                ctx.play('btt-walk-r8')
                time.sleep(0.3)
                continue
            if gap < 4:
                ctx.play('btt-walk-l8')
                continue
            move = ATTACKS[attempt % len(ATTACKS)]
            mark = hs.diag_mark(ctx)
            ctx.play(move)
            attempts.append((k + 1, move, round(gap, 1)))
            try:
                wait_state(ctx, mark, lambda s: s['phase'] == 'Done' or (count_of(s['text']) or (0, 0))[0] > k, 2.5, 'break %d' % (k + 1))
                break
            except RuntimeError:
                continue
    done = wait_state(ctx, run_mark, lambda s: s['phase'] == 'Done', 12, 'the run ends')
    return done, attempts, run_mark


# The diag prints a Running line on a break and on every tenth clock tick (`runtime::features::btt::pass`, `DIAG % 10`): that is
# the log's resolution. Ten ticks show as at most 17 hundredths (a tick is 5/3 of a hundredth, rounded down when shown).
SAMPLE_TICKS = 10
SAMPLE_HUNDREDTHS = -(-SAMPLE_TICKS * 5 // 3)


def clock_stop_problems(states, first_run):
    """SPEC row 29, from a run's `[Btt] phase=` states in order (its Running lines, its Done line, the Ready after): (a) the
    last sampled Running clock is not later than the Done time; (b) Done comes within one sampling interval of that sample
    (the log cannot place the stop any closer); (c) no Running line after Done, so the clock stopped; (d) on a session's
    first run of the layout the best is that run's time. The problems found (none: the check holds)."""
    done_at = next((k for k, s in enumerate(states) if s['phase'] == 'Done'), None)
    if done_at is None:
        return ['no Done line']
    found = _DONE.search(states[done_at]['text'])
    if not found:
        return ['Done text %r' % states[done_at]['text']]
    took, best = hundredths(*found.group(2, 3, 4)), hundredths(*found.group(5, 6, 7))
    running = [clock_of(s['text']) for s in states[:done_at] if s['phase'] == 'Running' and clock_of(s['text']) is not None]
    problems = []
    if not running:
        problems.append('no Running line before Done')
    elif running[-1] > took:
        problems.append('(a) the last Running clock %d is after Done %d' % (running[-1], took))
    elif took - running[-1] > SAMPLE_HUNDREDTHS:
        problems.append('(b) Done %d is more than one sampling interval (%d) after the last Running clock %d' % (took, SAMPLE_HUNDREDTHS, running[-1]))
    after = [s['text'] for s in states[done_at + 1:] if s['phase'] == 'Running']
    if after:
        problems.append('(c) Running after Done: %s' % after[:2])
    if first_run and best != took:
        problems.append('(d) the first run\'s best %d is not its time %d' % (best, took))
    return problems

def e29(ctx):
    """Row 29: a 3-target run; the clock stops on the last break; Ready again with every target back; a second run shows
    the fastest."""
    out = {'problems': []}
    targets = hs._state(ctx).get('btt_layout')
    st = latest(ctx)
    if not targets or st is None or st['phase'] != 'Ready' or st['n'] != 3:
        raise RuntimeError('btt-E28 did not leave Ready with its 3-target layout (%s, %s)' % (st, targets))
    runs = []
    for number in (1, 2):
        done, attempts, run_mark = run_targets(ctx, targets)
        found = _DONE.search(done['text'])
        if not found:
            raise RuntimeError('Done text %r' % done['text'])
        took, best = hundredths(*found.group(2, 3, 4)), hundredths(*found.group(5, 6, 7))
        if found.group(1) != str(len(targets)):
            out['problems'].append('run %d: Done counts %s targets, not %d' % (number, found.group(1), len(targets)))
        out['shot_done%d' % number] = str(ctx.shot('btt-E29-done-%d' % number))
        runs.append({'took': took, 'best': best, 'text': done['text'], 'attempts': attempts})
        mark = hs.diag_mark(ctx)
        ready = wait_state(ctx, mark, lambda s: s['phase'] == 'Ready', 8, 'Ready again after the 3 s notice')
        # The clock stopped on the last break (SPEC row 29), checked from the run's own lines (`clock_stop_problems`).
        states = [s for _, s in since(ctx, run_mark)]
        out['problems'].extend('run %d: %s' % (number, why) for why in clock_stop_problems(states, number == 1))
        if ready['n'] != 3 or ready['text'] != 'Targets 0/3   0:00.00':
            out['problems'].append('run %d: Ready again shows n=%d %r' % (number, ready['n'], ready['text']))
        ctx.play(steps.RESET_CENTER)  # from Ready the reset changes no [Btt] line: P1 only goes back
        time.sleep(2.5)
        if latest(ctx)['phase'] != 'Ready':
            out['problems'].append('run %d: not Ready after L+R+A: %s' % (number, latest(ctx)))
    out['runs'] = runs
    if runs[1]['best'] != min(runs[0]['took'], runs[1]['took']):
        out['problems'].append('the second run\'s best %d is not the fastest of %d and %d' % (runs[1]['best'], runs[0]['took'], runs[1]['took']))
    save(ctx, 'e29', out)
    settle(ctx)
    if out['problems']:
        raise RuntimeError('; '.join(out['problems']))
    return {'times': [r['took'] for r in runs], 'best': runs[1]['best']}


def stage_of(ctx, key):
    stage = hs._state(ctx).get('stage_' + key)
    if stage is None:
        raise RuntimeError('no stage id remembered for %s: btt-st-%s did not run' % (key, key))
    return stage


def e41(ctx, key='bf'):
    """Row 41 (btt-5): ZL + Down twice on Battlefield makes two NEW files (`Layout N · Battlefield`, the lowest free N), each with
    the layout's targets to a tenth; nothing else changes; the newer is chosen, first of the five; the Saved layouts list shows
    this stage's files newest first (the hot five, then by name), none of another stage's."""
    out = {'problems': []}
    stage, name = stage_of(ctx, key), STAGE_NAMES[key]
    st = to_placing(ctx, out)
    if st['n'] != 3:
        if st['n']:
            clear_layout(ctx)
        put_layout(ctx, latest(ctx), out)
    want = sorted(round(float(c), 1) for t in hs._state(ctx).get('btt_layout') or [] for c in t)
    first = save_new(ctx, stage, name)
    time.sleep(0.5)
    second = save_new(ctx, stage, name)
    out['files'] = [first, second]
    for _, doc in (first, second):
        if want and sorted(round(c, 1) for t in doc['targets'] for c in t) != want:
            out['problems'].append('%s holds %s, the layout is %s' % (doc['name'], doc['targets'], want))
    if not second[1]['used'] > first[1]['used']:
        out['problems'].append('the second save is not the newer use: %s' % [first[1]['used'], second[1]['used']])
    mine = layouts(ctx, stage)
    chosen = 'Place targets 3/10   1/%d %s: 3' % (min(len(mine), 5), second[1]['name'])
    wait_state(ctx, hs.diag_mark(ctx) - 60, lambda s: s['text'] == chosen, 6, 'the new file chosen, first of the five')
    out['shot_saved'] = str(ctx.shot('btt-E41-saved'))
    listing = open_list(ctx)
    out['list'] = listing
    out['shot_list'] = str(ctx.shot('btt-E41-list'))
    if listing['labels'] != list_order(mine) or listing['labels'][:2] != [second[1]['name'], first[1]['name']]:
        out['problems'].append('the list %s, want %s (newest first)' % (listing['labels'], list_order(mine)))
    others = {d['name'] for _, d in layouts(ctx)} - {d['name'] for _, d in mine}
    if others & set(listing['labels']):
        out['problems'].append('another stage\'s layouts are listed: %s' % (others & set(listing['labels'])))
    leave_list(ctx)
    save(ctx, 'e41', out)
    settle(ctx)
    if out['problems']:
        raise RuntimeError('; '.join(out['problems']))
    return {'stage': stage, 'new': [first[0], second[0]], 'list': listing['labels']}


def e43(ctx, key='bf'):
    """Row 43 (btt-5), the Saved layouts screen: A on the newest opens its own screen (Load, Rename, Duplicate, Delete); Rename
    takes the keyboard's name into the SAME file (top of the list); Duplicate makes a new file `Layout N · <stage>` with the
    same targets at the top; Delete asks once (Keep it, Delete it) and only then removes the file."""
    out = {'problems': []}
    stage, name = stage_of(ctx, key), STAGE_NAMES[key]
    mine = layouts(ctx, stage)
    if len(mine) < 2:
        raise RuntimeError('btt-E41 left %d files on this stage' % len(mine))
    newest_id, newest = mine[0]
    listing = open_list(ctx)
    if listing['labels'][:1] != [newest['name']]:
        out['problems'].append('the list opens on %s, not the newest %s' % (listing['labels'][:1], newest['name']))
    # 1. Rename: the keyboard's name, the same file.
    own = own_row(ctx, 1)
    out['own'] = own
    out['keyboard'] = cov.keyboard_type(ctx, 'Ledge trap')
    files = wait_layouts(ctx, lambda ls: dict(ls).get(newest_id, {}).get('name') == 'Ledge trap', 8, 'the rename')
    if len(files) != len(layouts(ctx)) or len([k for k, _ in files if k == newest_id]) != 1:
        out['problems'].append('the rename made or lost a file: %s' % [k for k, _ in files])
    renamed = dict(layouts(ctx, stage))[newest_id]
    if renamed['used'] != max(d['used'] for _, d in layouts(ctx)):
        out['problems'].append('the rename is not the newest use (%s)' % renamed['used'])
    time.sleep(1.0)
    steps.press(ctx, 'a', 1.0)  # back on the Saved layouts row: the list again
    listing = view(ctx)
    out['list_renamed'] = listing['labels']
    if listing['labels'][:1] != ['Ledge trap']:
        out['problems'].append('after the rename the list starts %s' % listing['labels'][:1])
    # 2. Duplicate: a new file with the same targets, at the top.
    before = dict(layouts(ctx))
    own_row(ctx, 2)
    got = wait_layouts(ctx, lambda ls: any(k not in before for k, _ in ls), 8, 'the duplicate')
    copy_id, copy = [(k, d) for k, d in got if k not in before][0]
    want = lowest_free([d['name'] for _, d in before.items() if d.get('stage') == stage], name)
    if (copy['name'], copy['targets'], copy['stage']) != (want, renamed['targets'], stage):
        out['problems'].append('the copy %s, want %r with %s' % (copy, want, renamed['targets']))
    listing = view(ctx)
    out['list_duplicated'] = listing['labels']
    out['shot_duplicated'] = str(ctx.shot('btt-E43-duplicated'))
    if listing['labels'][:2] != [want, 'Ledge trap']:
        out['problems'].append('after Duplicate the list starts %s' % listing['labels'][:2])
    # 3. Delete the copy (the top row): the question first, the file still there; then Delete it.
    own_row(ctx, 3)
    question = view(ctx)
    out['question'] = question
    out['shot_question'] = str(ctx.shot('btt-E43-question'))
    if question['labels'] != ASK_ROWS or copy_id not in dict(layouts(ctx)):
        out['problems'].append('Delete did not ask first: %s, file there %s' % (question['labels'], copy_id in dict(layouts(ctx))))
    steps.press(ctx, 'right', 0.4)
    steps.press(ctx, 'a', 1.0)
    wait_layouts(ctx, lambda ls: copy_id not in dict(ls), 6, 'the delete')
    listing = view(ctx)
    out['list_deleted'] = listing['labels']
    if want in listing['labels'] or listing['labels'][:1] != ['Ledge trap']:
        out['problems'].append('after Delete the list is %s' % listing['labels'])
    out['shot_deleted'] = str(ctx.shot('btt-E43-deleted'))
    leave_list(ctx)
    save(ctx, 'e43', out)
    settle(ctx)
    if out['problems']:
        raise RuntimeError('; '.join(out['problems']))
    return {'renamed': newest_id, 'copy': copy_id, 'list': listing['labels']}


def mean_diff(path_a, path_b):
    from PIL import Image
    with Image.open(path_a) as a, Image.open(path_b) as b:
        x, y = a.convert('RGB'), b.convert('RGB').resize(a.size)
        pa, pb = x.tobytes(), y.tobytes()
        return sum(abs(i - j) for i, j in zip(pa, pb)) / max(len(pa), 1)


def e31(ctx, key='bf'):
    """Row 31: the camera shows the whole stage in Placing, Ready and Running and is the game's own again with the menu open
    and with the mode Off: stills plus the `[Btt] camera` lines. The stills are for review; the lines and the picture change
    are checked."""
    out = {'problems': [], 'tag': key}
    # 1. Off: the game's camera.
    mark = hs.diag_mark(ctx)
    was = latest(ctx)
    set_row(ctx, False)
    if was and was['phase'] != 'Off':  # the camera goes back only if the mode had it
        try:
            wait_state(ctx, mark, lambda s: s['phase'] == 'Off', 10, 'the mode Off')
            out['off'] = wait_camera(ctx, mark, False, 1, 8, 'mode Off')
        except RuntimeError as error:
            out['problems'].append(str(error))
    time.sleep(2.5)
    out['shot_off'] = str(ctx.shot('btt-E31-%s-off-normal' % key))
    # 2. Placing.
    mark = hs.diag_mark(ctx)
    st = to_placing(ctx, out)
    try:
        out['placing'] = wait_camera(ctx, mark, True, 4, 10, 'Placing')
    except RuntimeError as error:
        out['problems'].append(str(error))
    time.sleep(3.0)
    out['shot_placing'] = str(ctx.shot('btt-E31-%s-placing' % key))
    # 3. Ready (the layout the stage already has, or three targets near P1).
    if st['n'] == 0:
        st = latest(ctx)
        put_layout(ctx, st, out)
    mark = hs.diag_mark(ctx)
    ctx.play('x-press')
    wait_state(ctx, mark, lambda s: s['phase'] == 'Ready', 8, 'Ready')
    time.sleep(2.0)
    out['shot_ready'] = str(ctx.shot('btt-E31-%s-ready' % key))
    # 4. Running.
    mark = hs.diag_mark(ctx)
    ctx.play('btt-a')
    wait_state(ctx, mark, lambda s: s['phase'] == 'Running', 6, 'Running')
    time.sleep(1.0)
    out['shot_running'] = str(ctx.shot('btt-E31-%s-running' % key))
    # 5. The plugin's menu open: the game's own camera; closed again: the fixed one.
    mark = hs.diag_mark(ctx)
    steps.open_menu(ctx)
    try:
        out['menu'] = wait_camera(ctx, mark, False, 1, 8, 'menu open')
    except RuntimeError as error:
        out['problems'].append(str(error))
    time.sleep(2.0)
    out['shot_menu'] = str(ctx.shot('btt-E31-%s-menu-open' % key))
    mark = hs.diag_mark(ctx)
    close_menu(ctx)
    try:
        out['closed'] = wait_camera(ctx, mark, True, 4, 10, 'menu closed')
    except RuntimeError as error:
        out['problems'].append(str(error))
    time.sleep(2.5)
    out['shot_closed'] = str(ctx.shot('btt-E31-%s-menu-closed' % key))
    # The picture changed with the camera: Placing against Off, menu open against Running.
    for a, b, tag in (('shot_off', 'shot_placing', 'off-vs-placing'), ('shot_running', 'shot_menu', 'running-vs-menu')):
        try:
            out['diff_' + tag] = round(mean_diff(out[a], out[b]), 2)
            if out['diff_' + tag] < 8.0:
                out['problems'].append('%s: the stills differ by only %.1f (the camera did not move?)' % (tag, out['diff_' + tag]))
        except Exception as error:  # noqa: BLE001 - the diff is a check on the evidence, not on the product
            out['problems'].append('%s: %r' % (tag, error))
    mark = hs.diag_mark(ctx)
    ctx.play(steps.RESET_CENTER)
    save(ctx, 'e31-' + key, out)
    settle(ctx)
    if out['problems']:
        raise RuntimeError('; '.join(out['problems']))
    return {'diffs': {k: v for k, v in out.items() if k.startswith('diff_')}}


def e42(ctx, key='bbf'):
    """Row 42 (btt-5): a layout file this boot did not write (the seeded `layouts/seeded-big-battlefield.json`) is read when the
    stage opens: ZL + Right names it, 1/1 with its 3 targets; ZL + Up loads them into Placing (its file is used, so it stays
    the newest); X opens Ready with the 3."""
    out = {'problems': []}
    stage = stage_of(ctx, key)
    seeded = dict(layouts(ctx, stage)).get(SEEDED_ID)
    if seeded is None:
        raise RuntimeError('the fixture %s is not in the profile (instances[].files with "to")' % (layout_dir(ctx) / SEEDED_ID))
    st = to_placing(ctx, out)
    if st['n']:
        clear_layout(ctx)
    shown = zl_step(ctx)
    count = min(len(layouts(ctx, stage)), 5)
    if (shown.get('name'), shown.get('index'), shown.get('count'), shown.get('targets')) != (SEEDED_NAME, 1, count, 3):
        out['problems'].append('ZL + Right shows %s, want 1/%d %s: 3' % (shown, count, SEEDED_NAME))
    out['shot_chosen'] = str(ctx.shot('btt-E42-chosen'))
    mark = hs.diag_mark(ctx)
    ctx.play('chord-load')
    loaded = wait_state(ctx, mark, lambda s: s['text'] == 'Loaded: ' + SEEDED_NAME and s['n'] == 3, 6, 'ZL + Up loads it')
    out['loaded'] = loaded
    time.sleep(1.0)
    out['shot_loaded'] = str(ctx.shot('btt-E42-loaded'))
    wait_layouts(ctx, lambda ls: dict(ls).get(SEEDED_ID, {}).get('used', 0) > seeded.get('used', 0), 6, 'the load is a use')
    mark = hs.diag_mark(ctx)
    ctx.play('x-press')
    wait_state(ctx, mark, lambda s: s['phase'] == 'Ready' and s['n'] == 3, 8, 'the loaded layout starts Ready')
    time.sleep(1.5)
    out['shot_ready'] = str(ctx.shot('btt-E42-ready'))
    save(ctx, 'e42', out)
    settle(ctx)
    if out['problems']:
        raise RuntimeError('; '.join(out['problems']))
    return {'chosen': shown, 'n': loaded['n']}


def x_stages(ctx, first='bf', second='bbf'):
    """Saving and loading on two stages (the operator's question; btt-5's convention): on `second` (Big Battlefield) the list
    and the chords show only its own layouts; a Save there makes `Layout N · Big Battlefield` and no `first` file changes; back
    on `first` (Battlefield) ZL + Right steps to the OLDER of its layouts and ZL + Up loads it, which makes it the newest (the
    `[Btt] layouts` line names it first); no `second` file changes."""
    out = {'problems': []}
    a, b = stage_of(ctx, first), stage_of(ctx, second)
    bytes_of = lambda stage: {k: (layout_dir(ctx) / ('%s.json' % k)).read_bytes() for k, _ in layouts(ctx, stage)}  # noqa: E731
    # 1. The second stage: its own list.
    change_stage(ctx, second)
    files_a = bytes_of(a)
    listing = open_list(ctx)
    out['list_second'] = listing['labels']
    out['shot_list_second'] = str(ctx.shot('btt-X-stages-1-list'))
    if listing['labels'] != list_order(layouts(ctx, b)):
        out['problems'].append('step 1: the list is %s, the stage\'s files %s' % (listing['labels'], list_order(layouts(ctx, b))))
    if {d['name'] for _, d in layouts(ctx, a)} & set(listing['labels']):
        out['problems'].append('step 1: the first stage\'s layouts are listed')
    leave_list(ctx)
    # 2. Two targets there, saved: a new file of its own.
    st = to_placing(ctx, out, 'xs2')
    if st['n']:
        clear_layout(ctx)
    st = latest(ctx)
    cursor = st['cursor']
    for k, (x, y) in enumerate(sorted(layout_near(st)[:2], key=lambda t: abs(t[0] - st['cursor'][0])), 1):
        cursor = place_at(ctx, cursor, x, y, k)['cursor']
    made = save_new(ctx, b, STAGE_NAMES[second])
    out['saved_second'] = made
    if bytes_of(a) != files_a:
        out['problems'].append('step 2: a first-stage file changed')
    files_b = bytes_of(b)
    # 3. Back on the first stage: the older layout, loaded.
    change_stage(ctx, first)
    st = to_placing(ctx, out, 'xs3')
    if st['n']:
        clear_layout(ctx)
    mine = layouts(ctx, a)
    if len(mine) < 2:
        raise RuntimeError('step 3: %d layouts on the first stage' % len(mine))
    older = mine[1]
    steps_seen = [zl_step(ctx), zl_step(ctx)]
    out['steps'] = steps_seen
    if [s.get('name') for s in steps_seen] != [mine[0][1]['name'], older[1]['name']]:
        out['problems'].append('step 3: ZL + Right showed %s, want %s then %s' % (steps_seen, mine[0][1]['name'], older[1]['name']))
    mark = hs.diag_mark(ctx)
    ctx.play('chord-load')
    n = len(older[1]['targets'])
    loaded = wait_state(ctx, mark, lambda s: s['text'] == 'Loaded: ' + older[1]['name'] and s['n'] == n, 6, 'ZL + Up loads the older')
    out['loaded'] = loaded['text']
    try:
        out['listed'] = wait_listed(ctx, mark, lambda l: l['stage'] == a and l['first'] == older[1]['name'], 6, 'the load moves it first')
    except RuntimeError as error:
        out['problems'].append(str(error)[:300])
    time.sleep(1.0)
    out['shot_loaded'] = str(ctx.shot('btt-X-stages-3-loaded'))
    if bytes_of(b) != files_b:
        out['problems'].append('step 3: a second-stage file changed')
    save(ctx, 'x_stages', out)
    settle(ctx)
    if out['problems']:
        raise RuntimeError('; '.join(out['problems']))
    return {'second': made[1]['name'], 'older': older[1]['name']}


def x_saves(ctx, key='bf'):
    """Repeated saves and the chords on the hot five (btt-5): saves until the stage has 7 files (each a NEW file); the writer
    lists 5 hot; ZL + Right steps 1/5 .. 5/5 through the five newest and wraps, ZL + Left goes back; the two oldest stay as
    files outside the five; ZL + Up on 3/5 loads it and it becomes 1/5; ZL + Y twice deletes it and the five refill from the
    rest. No temporary file is left beside the layouts."""
    out = {'problems': []}
    stage, name = stage_of(ctx, key), STAGE_NAMES[key]
    st = to_placing(ctx, out)
    if st['n'] != 3:
        if st['n']:
            clear_layout(ctx)
        put_layout(ctx, latest(ctx), out)
    made = []
    while len(layouts(ctx, stage)) < 7:
        made.append(save_new(ctx, stage, name)[1]['name'])
        time.sleep(0.4)
    out['made'] = made
    mine = layouts(ctx, stage)
    hot = [d['name'] for _, d in mine[:5]]
    # The last Save chose its new file, 1/5: Right steps 2/5 .. 5/5 and wraps to 1/5, 2/5; Left goes back to 1/5.
    seen = [zl_step(ctx) for _ in range(6)] + [zl_step(ctx, False)]
    out['steps'] = seen
    want = [(i % 5 + 1, hot[i % 5]) for i in range(1, 7)] + [(1, hot[0])]
    if [(s.get('index'), s.get('name')) for s in seen] != want or any(s.get('count') != 5 for s in seen):
        out['problems'].append('the chords stepped %s, want %s' % ([(s.get('index'), s.get('name')) for s in seen], want))
    if len(layouts(ctx, stage)) != len(mine):
        out['problems'].append('stepping changed the files')
    # Load the third (two Rights from 1/5): it becomes the newest, 1/5.
    for _ in range(2):
        zl_step(ctx)
    third = hot[2]
    mark = hs.diag_mark(ctx)
    ctx.play('chord-load')
    wait_state(ctx, mark, lambda s: s['text'] == 'Loaded: ' + third, 6, 'ZL + Up loads 3/5')
    try:
        wait_state(ctx, mark, lambda s: (picked(s['text']) or {}).get('index') == 1 and (picked(s['text']) or {}).get('name') == third, 8,
                   'the loaded layout chosen, 1/5')
    except RuntimeError as error:
        out['problems'].append(str(error)[:300])
    # Delete it: ZL + Y asks, the second deletes; the five refill (7 - 1 = 6 files, 5 hot).
    before = len(layouts(ctx, stage))
    mark = hs.diag_mark(ctx)
    ctx.play('btt-zl-y2')
    wait_state(ctx, mark, lambda s: s['text'] == 'Deleted: ' + third, 6, 'ZL + Y twice deletes')
    texts = [s['text'] for _, s in since(ctx, mark)]
    if 'ZL + Y again to delete ' + third not in texts:
        out['problems'].append('no question before the delete: %s' % texts[:4])
    wait_layouts(ctx, lambda ls: len([1 for _, d in ls if d.get('stage') == stage]) == before - 1, 6, 'the file deleted')
    try:
        out['listed'] = wait_listed(ctx, mark, lambda l: l['stage'] == stage and l['files'] == before - 1 and l['hot'] == 5, 6, 'the five refill')
    except RuntimeError as error:
        out['problems'].append(str(error)[:300])
    if third in {d['name'] for _, d in layouts(ctx, stage)}:
        out['problems'].append('%s is still a file' % third)
    if stray(ctx):
        out['problems'].append('temporary files left: %s' % stray(ctx))
    out['shot_end'] = str(ctx.shot('btt-X-saves-end'))
    save(ctx, 'x_saves', out)
    settle(ctx)
    if out['problems']:
        raise RuntimeError('; '.join(out['problems']))
    return {'files': len(layouts(ctx, stage)), 'hot': hot, 'deleted': third}


def btt_off(ctx):
    """The last row: the mode Off, the menu closed, the fighters reset, so the boot ends where the others do."""
    set_row(ctx, False)
    steps.open_menu(ctx)
    close_menu(ctx)
    ctx.play(steps.RESET_CENTER)
    time.sleep(1.0)
    st = latest(ctx)
    settle(ctx)
    return {'last_state': st}


def btt_recover(ctx):
    """Recovery for a failed row: the mode Off by the live setting (no input reaches a Placing view), the menu backed out of
    (B closes it), the fighters reset."""
    try:
        set_row(ctx, False)
    except RuntimeError:
        pass
    for _ in range(4):
        steps.press(ctx, 'b', 0.3)
    ctx.play(steps.RESET_CENTER)
    time.sleep(0.8)


# --- the GIF takes: prelude hooks (not recorded) and action hooks (inside a `gif` step) ----------------------------

def g1_prep(ctx):
    """Before the clip: Battlefield, the mode Off, the menu open on Fighters & Stages with Place targets focused."""
    # btt-4: the session keeps Battlefield's layout, so the clip would open with the earlier rows' targets: clear it first.
    st = to_placing(ctx, {}, 'g1')
    if st['n']:
        clear_layout(ctx)
    set_row(ctx, False)
    ctx.play(steps.RESET_CENTER)
    time.sleep(1.5)
    steps.open_menu(ctx)
    steps.goto_page(ctx, 'fighters_stages')
    if not steps._focus_row(ctx, 'Place targets'):
        raise RuntimeError('the Place targets row was not focused')
    return {'shot': str(ctx.shot('btt-G1-before'))}


def g1_action(ctx):
    """In the clip: A on Place targets (the menu closes and the camera zooms out), three targets placed, the notice."""
    out = {}
    mark = hs.diag_mark(ctx)
    steps.press(ctx, 'a', 0.8)
    st = wait_state(ctx, mark, lambda s: s['phase'] == 'Placing', 12, 'Placing')
    time.sleep(1.0)
    put_layout(ctx, st, out)
    time.sleep(1.5)
    return out


def g2_prep(ctx):
    """Before the clip: Ready with three targets near P1 (Placing, three A, X)."""
    out = {}
    st = to_placing(ctx, out, 'g2')
    if st['n']:
        st = clear_layout(ctx)
    put_layout(ctx, st, out)
    ctx.play('x-press')
    wait_state(ctx, hs.diag_mark(ctx) - 20, lambda s: s['phase'] == 'Ready', 8, 'Ready')
    return out


def g2_action(ctx):
    """In the clip: the run (the first input starts the clock) through to the Done notice."""
    targets = hs._state(ctx).get('btt_layout')
    done, attempts, _ = run_targets(ctx, targets, cap_s=40)
    time.sleep(2.5)
    return {'done': done['text'], 'attempts': attempts}


def g3_prep(ctx):
    """Before the clip: Placing on Battlefield with three targets."""
    out = {}
    st = to_placing(ctx, out, 'g3')
    if st['n']:
        st = clear_layout(ctx)
    put_layout(ctx, st, out)
    return out


def g3_action(ctx):
    """In the clip (btt-5): ZL + Down saves a new file (Saved: Layout N · Battlefield, chosen 1/k), Y twice clears, ZL + Up loads
    the chosen file back (Loaded: ..., the three targets return)."""
    out = {}
    stage, name = stage_of(ctx, 'bf'), STAGE_NAMES['bf']
    time.sleep(0.8)
    out['saved'] = save_new(ctx, stage, name)[1]['name']
    time.sleep(1.5)
    mark = hs.diag_mark(ctx)
    ctx.play('btt-y2')
    wait_state(ctx, mark, lambda s: s['n'] == 0, 6, 'cleared')
    time.sleep(1.0)
    mark = hs.diag_mark(ctx)
    ctx.play('chord-load')
    wait_state(ctx, mark, lambda s: s['text'] == 'Loaded: ' + out['saved'] and s['n'] == 3, 6, 'loaded')
    time.sleep(1.5)
    return out