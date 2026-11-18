"""rc.15 practice menu rows (menu-games-devin, 2026-11-18) on the exact gate-built DIAG.

Gates read the DIAG's own lines: `[Ui] menu opened page=`, `[Ui] press … screen=… row=…`, `[Ui] menu closed`,
`[Practice] start|stop game=`, `[Eggs] notice=`, `[Btt] phase=` and `[TrainingRooms] notice_draw`. The exact build is
traced with Eden `trace.*` (stub off before `game.load`, `trace.configure`, the gate's own diag ELF via `symbols.load`):
TR's `pane::set_text` is recorded, so the menu's drawn words ("Running", "Stop", "10 targets") are guest evidence.
"""
import hashlib
import json
import re
import time
from pathlib import Path

import routes

ROOT = Path(__file__).parent
UI = re.compile(r'\[Ui\] press (?P<button>\w+) page=(?P<page>\S+) cursor=(?P<cursor>\d+) depth=(?P<depth>\d+) '
                r'screen=(?P<screen>.*?) row=(?:Some\("(?P<row>[^"]*)"\)|None)')
DRAW = r'\[TrainingRooms\] notice_draw header="(?P<header>{header})" body="(?P<body>{body})" '
OPENED = re.compile(r'\[Ui\] menu opened page=(?P<page>\S+) cursor=\d+ depth=(?P<depth>\d+) screen=(?P<screen>.*)')
GROUPS = {'Tech practice': 21, 'Eggs-ercise': 22, 'Break the Targets': 23, 'Mash out': 24}


def save(ctx, name, value):
    (Path(ctx.row_dir) / (name + '.json')).write_text(json.dumps(value, indent=2, default=str) + '\n')
    return value


def pins():
    return json.loads((ROOT / 'build-pins.json').read_text())


def trace_setup(ctx):
    """Before the single game.load: stub off (read back), muted, trace output, the diag ELF, the set_text point."""
    if (ctx.call('status') or {}).get('running'):
        raise RuntimeError('trace_setup runs before game.load')
    ctx.call('settings.set', {'key': 'core.use_gdbstub', 'value': False})
    stub = ctx.call('settings.get', {'key': 'core.use_gdbstub'}) or {}
    if str(stub.get('value')).lower() != 'false':
        raise RuntimeError(f'GDB stub still on: {stub}')
    volume = ctx.call('settings.get', {'key': 'core.volume'}) or {}
    if str(volume.get('value')) != '0':
        raise RuntimeError(f'not muted: {volume}')
    ctx.call('settings.save', {})
    pin = pins()
    for key, path in (('diag_sha256', pin['diag_nro']), ('elf_sha256', pin['diag_elf'])):
        if hashlib.sha256(Path(path).read_bytes()).hexdigest() != pin[key]:
            raise RuntimeError(f'pinned {key} differs: {path}')
    folder = Path(ctx.row_dir).parent / 'native-trace'
    folder.mkdir(exist_ok=True)
    configured = ctx.call('trace.configure', {'run_directory': str(folder), 'label': 'practice-menu-' + pin['head'][:8]})
    (Path(ctx.row_dir).parent / 'trace-path.json').write_text(json.dumps(configured) + '\n')
    symbols = ctx.call('symbols.load', {'module': 'libtraining_rooms.nro', 'elf': pin['diag_elf']})
    point = {'target': 'libtraining_rooms.nro!' + pin['set_text_symbol'], 'mode': 'records',
             'registers': ['x0', 'x1', 'x2', 'pc'], 'condition': 'x2 > 0',
             'captures': [{'expression': 'x0', 'type': 'bytes', 'size': 64},
                          {'expression': 'x1', 'type': 'bytes', 'size': 320}],
             'max_hits': 0, 'rate_limit': 1000}
    armed = ctx.call('trace.arm', point)
    return save(ctx, 'trace-setup', {'stub': stub, 'volume': volume, 'configured': configured, 'symbols': symbols,
                                     'point': point, 'armed': armed})


def tracefile(ctx):
    return Path(json.loads((Path(ctx.row_dir).parent / 'trace-path.json').read_text())['path'])


def trace_offset(ctx):
    path = tracefile(ctx)
    return path.stat().st_size if path.exists() else 0


def traced(ctx, offset, wanted):
    """The recorded set_text texts since `offset` that contain `wanted` (the exact build's own drawn words)."""
    found, path = [], tracefile(ctx)
    if not path.exists():
        return found
    with path.open('rb') as stream:
        stream.seek(offset)
        for raw in stream:
            try:
                record = json.loads(raw)
            except json.JSONDecodeError:
                continue
            caps, regs = record.get('captures') or [], record.get('registers') or {}
            if len(caps) != 2 or any(c.get('error') for c in caps) or 'x2' not in regs:
                continue
            text = bytes.fromhex(caps[1].get('hex', ''))[:2 * int(regs['x2'], 0)].decode('utf-16-le', 'replace')
            if wanted in text:
                found.append({'frame': record.get('frame'), 'text': text})
    return found


def cursor(ctx):
    return routes.LogCursor(routes.instance_path(ctx, 'diag_log'))


def waits(ctx, button, pattern, cap=4):
    log = cursor(ctx)
    try:
        if button:
            ctx.press(button)
        return log.wait(pattern, cap, ctx)
    finally:
        log.close()


def press(ctx, button):
    return UI.search(waits(ctx, button, UI.pattern, 3).group(0)).groupdict()


def absent(ctx, pattern, window):
    """No line matching `pattern` within `window` seconds (a negative gate, bounded)."""
    log = cursor(ctx)
    try:
        log.wait(pattern, window, ctx)
    except RuntimeError:
        return True
    finally:
        log.close()
    raise RuntimeError(f'unexpected line: {pattern}')


def none_since(ctx, log, pattern):
    """No line matching `pattern` since `log` was opened (its earlier waits read past older lines only)."""
    try:
        found = log.wait(pattern, 0.5, ctx)
    except RuntimeError:
        return True
    raise RuntimeError(f'unexpected line: {found.group(0)[:200]}')


def open_menu(ctx):
    """Open the menu; returns its first state. It reopens where it was (`Place`): the page and any sub-screen."""
    # Boot 1 (r01 after a session clear, r06 right after a close): one chord was not taken and no opening line came, so the
    # chord is played again, at most three times, each gated on the opening line.
    for attempt in range(3):
        log = cursor(ctx)
        try:
            routes.play(ctx, 'tr-menu-open')
            try:
                return OPENED.search(log.wait(OPENED.pattern, 6, ctx).group(0)).groupdict()
            except RuntimeError:
                if attempt == 2:
                    raise
                time.sleep(1.0)
        finally:
            log.close()


def to_list(ctx):
    """From any menu state to the Practice games list on Fighters & Stages (depth 0)."""
    state = open_menu(ctx)
    page, depth = state['page'], int(state['depth'])
    for _ in range(depth):
        s = press(ctx, 'b')
        page, depth = s['page'], int(s['depth'])
    for _ in range(6):
        if page == 'fighters_stages':
            break
        page = press(ctx, 'r')['page']
    else:
        raise RuntimeError('Fighters & Stages page not reached')
    return state


def focus(ctx, title, depth='0'):
    seen = set()
    for _ in range(60):
        state = press(ctx, 'up')
        if state['depth'] != depth:
            raise RuntimeError(f'left depth {depth}: {state}')
        if state['row'] == title:
            return state
        if state['cursor'] + state['screen'] in seen:
            raise RuntimeError('row absent: ' + title)
        seen.add(state['cursor'] + state['screen'])
    raise RuntimeError('row search exhausted: ' + title)


def open_game(ctx, game):
    to_list(ctx)
    focus(ctx, game)
    state = press(ctx, 'a')
    if f'Rows {{ group: {GROUPS[game]}, cursor: 0 }}' not in state['screen'] or state['row'] != 'Start':
        raise RuntimeError(f'{game} sub-screen did not open on Start: {state}')
    return state


def focus_sub(ctx, game, title):
    seen = set()
    for _ in range(40):
        state = press(ctx, 'down')
        if f'group: {GROUPS[game]}' not in state['screen']:
            raise RuntimeError(f'left the {game} sub-screen: {state}')
        if state['row'] == title:
            return state
        if state['screen'] in seen:
            raise RuntimeError('sub row absent: ' + title)
        seen.add(state['screen'])
    raise RuntimeError('sub row search exhausted: ' + title)


def close_all(ctx):
    for _ in range(4):
        log = cursor(ctx)
        try:
            ctx.press('b')
            try:
                log.wait(r'\[Ui\] menu closed ', 1.5, ctx)
            except RuntimeError:
                continue
            routes.gate(ctx, 'training')
            return
        finally:
            log.close()
    raise RuntimeError('the menu did not close after four B')


def shot(ctx, name, offset=None, words=()):
    time.sleep(0.6)
    path = ctx.shot(name)
    time.sleep(0.4)
    drawn = {w: traced(ctx, offset, w)[:3] for w in words} if offset is not None else {}
    for word, hits in drawn.items():
        if not hits:
            raise RuntimeError(f'{name}: "{word}" was not drawn')
    return {'shot': str(path), 'drawn': drawn}


def start(ctx, game, expect):
    """A on Start (the cursor is on it): the menu closes, `[Practice] start`, then the game's own line."""
    log = cursor(ctx)
    try:
        ctx.press('a')
        # The close starts the game before it writes its own closed line (worker.rs `close`).
        started = log.wait(r'\[Practice\] start game=(\w+)', 4, ctx).group(1)
        log.wait(r'\[Ui\] menu closed ', 4, ctx)
        line = log.wait(expect, 20, ctx).group(0)
    finally:
        log.close()
    if started != {'Eggs-ercise': 'Eggs', 'Break the Targets': 'Btt', 'Tech practice': 'Tech', 'Mash out': 'Mashout'}[game]:
        raise RuntimeError(f'started {started}')
    return {'started': started, 'line': line[:200]}


# ---- rows ---------------------------------------------------------------------------------------------------------

def old_settings(ctx):
    """The boot loaded the pinned pre-change settings.json: every practice key is as written, nothing started at boot,
    and the menu shows the stored modes (10 targets, 3 minutes) with no Off."""
    fixture = json.loads((ROOT / 'settings-c212bc50.json').read_text())['menu']
    live = json.loads(Path(ctx.instance['settings']).read_text())['menu']
    keys = ['eggsercise', 'eggsercise_break', 'btt', 'btt_projectiles', 'tech_practice', 'practice_cpu']
    diff = {k: (fixture[k], live.get(k)) for k in keys if fixture[k] != live.get(k)}
    if diff:
        raise RuntimeError(f'settings changed: {diff}')
    boot = routes.instance_path(ctx, 'diag_log').read_text(errors='replace')
    # A retry: later rows have receipts (the diag log rotates, so its lines cannot tell).
    retry = (Path(ctx.row_dir).parent / 'r02-eggs-start.json').exists()
    if not retry and re.search(r'\[Eggs\] notice=|\[Btt\] phase=', boot):
        raise RuntimeError('a game ran at boot')
    # A retry in the same boot runs after later rows started games: the boot check is attempt 1's (its receipt failed
    # later, at the first menu-open gate, after this check).
    save(ctx, 'boot-check', {'retry': retry, 'checked': not retry})
    offset = trace_offset(ctx)
    to_list(ctx)
    focus(ctx, 'Eggs-ercise')
    out = {'list': shot(ctx, 'games-list', offset, ('Eggs-ercise', 'Break the Targets', 'Tech practice'))}
    offset = trace_offset(ctx)
    open_game(ctx, 'Eggs-ercise')
    out['eggs'] = shot(ctx, 'eggs-sub-screen', offset, ('Start', '10 targets'))
    offset = trace_offset(ctx)
    press(ctx, 'b')
    open_game(ctx, 'Tech practice')
    out['tech'] = shot(ctx, 'tech-sub-screen', offset, ('Start', '3 minutes'))
    close_all(ctx)
    return save(ctx, 'old-settings', {'fixture': {k: fixture[k] for k in keys}, **out})


def eggs_start(ctx):
    open_game(ctx, 'Eggs-ercise')
    out = start(ctx, 'Eggs-ercise', r'\[Eggs\] notice="Eggs 0/10')
    out['shot'] = shot(ctx, 'eggs-running')
    return save(ctx, 'eggs-start', out)


def running_and_b(ctx):
    """The list shows Running (green), the sub-screen's first row reads Stop; B leaves the run going."""
    offset = trace_offset(ctx)
    state = open_menu(ctx)
    if 'group: 22, cursor: 0' not in state['screen']:
        raise RuntimeError(f'did not reopen on the Eggs-ercise sub-screen: {state}')
    out = {'reopened': state, 'stop': shot(ctx, 'eggs-stop-row', offset, ('Stop', 'Running'))}
    offset = trace_offset(ctx)
    press(ctx, 'b')
    out['list'] = shot(ctx, 'games-list-running', offset, ('Running',))
    log = cursor(ctx)
    try:
        close_all(ctx)
        first = log.wait(r'\[Eggs\] notice="(Eggs [^"]*)"', 10, ctx).group(1)
    finally:
        log.close()
    absent(ctx, r'\[Practice\] (start|stop)|\[Eggs\] notice="Eggs 0/10   0:00\.0', 2.5)
    out['after_close'] = first
    return save(ctx, 'running-and-b', out)


def lra_restart(ctx):
    log = cursor(ctx)
    try:
        routes.play(ctx, 'reset-lra')
        line = log.wait(r'\[Eggs\] notice="Eggs 0/10   0:00\.0', 15, ctx).group(0)
    finally:
        log.close()
    return save(ctx, 'lra-restart', {'line': line[:160], 'shot': shot(ctx, 'eggs-after-lra')})


def stop_keeps_open(ctx):
    offset = trace_offset(ctx)
    open_game(ctx, 'Eggs-ercise')
    log = cursor(ctx)
    try:
        ctx.press('a')
        stopped = log.wait(r'\[Practice\] stop game=Some\(Eggs\)', 4, ctx).group(0)
    finally:
        log.close()
    absent(ctx, r'\[Ui\] menu closed ', 2.0)
    out = {'stopped': stopped, 'shot': shot(ctx, 'eggs-stopped-start-row', offset, ('Start',))}
    close_all(ctx)
    absent(ctx, r'\[Eggs\] notice=', 2.5)
    return save(ctx, 'stop-keeps-open', out)


def start_other_stops_first(ctx):
    open_game(ctx, 'Eggs-ercise')
    out = {}
    # An earlier attempt may have left Eggs-ercise running: its first row is then Stop, so A stops it (the menu stays
    # open) and the next A starts it.
    log = cursor(ctx)
    try:
        ctx.press('a')
        first = log.wait(r'\[Practice\] (start|stop) game=(\S+)', 4, ctx)
    finally:
        log.close()
    if first.group(1) == 'stop':
        out['stopped_first'] = first.group(0)
        out['eggs'] = start(ctx, 'Eggs-ercise', r'\[Eggs\] notice="Eggs 0/10')
    else:
        out['eggs'] = {'started': first.group(2), 'line': waits(ctx, None, r'\[Eggs\] notice="Eggs 0/10', 20).group(0)[:200]}
        routes.gate(ctx, 'training')
    open_game(ctx, 'Break the Targets')
    out['btt'] = start(ctx, 'Break the Targets', r'\[Btt\] phase=(Placing|Ready) ')
    absent(ctx, r'\[Eggs\] notice=', 2.5)
    out['shot'] = shot(ctx, 'btt-started-eggs-stopped')
    return save(ctx, 'start-other-stops-first', out)


def tech_views(ctx):
    """Tech point and Tech wizard still open their views (and start Tech practice, which stops Break the Targets)."""
    out = {}
    open_game(ctx, 'Tech practice')
    focus_sub(ctx, 'Tech practice', 'Tech point')
    log = cursor(ctx)
    try:
        ctx.press('a')
        log.wait(r'\[Practice\] start game=Tech', 4, ctx)
        out['point'] = log.wait(DRAW.format(header='Tech Practice', body=r'Choose a surface[^"]*'), 60, ctx).group(0)[:200]
    finally:
        log.close()
    out['point_shot'] = shot(ctx, 'tech-point-view')
    state = open_menu(ctx)
    if 'group: 21' not in state['screen']:
        raise RuntimeError(f'did not reopen on the Tech sub-screen: {state}')
    focus_sub(ctx, 'Tech practice', 'Tech wizard')
    log = cursor(ctx)
    try:
        ctx.press('a')
        log.wait(r'\[Practice\] start game=Tech', 4, ctx)
        out['wizard'] = log.wait(DRAW.format(header='Tech Practice', body=r'[^"]*Move the cursor onto a surface[^"]*'), 60, ctx).group(0)[:240]
    finally:
        log.close()
    out['wizard_shot'] = shot(ctx, 'tech-wizard-view')
    # B in the wizard's first step goes back to the menu; then Stop ends Tech practice.
    waits(ctx, 'b', r'\[Ui\] menu opened ', 10)
    log = cursor(ctx)
    try:
        for _ in range(3):
            s = press(ctx, 'b')
            if s['depth'] == '0':
                break
        focus(ctx, 'Tech practice')
        press(ctx, 'a')
        ctx.press('a')
        out['stop'] = log.wait(r'\[Practice\] stop game=Some\(Tech\)', 4, ctx).group(0)
    finally:
        log.close()
    close_all(ctx)
    return save(ctx, 'tech-views', out)


CLOCK = r'\[Btt\] phase=Running n=\d+ .*?text="[^"]*?(\d+):(\d\d)\.(\d\d)'


def btt_clock(ctx, cap=10):
    found = waits(ctx, None, CLOCK, cap)
    return int(found.group(1)) * 60 + int(found.group(2)) + int(found.group(3)) / 100, found.group(0)[:200]


def btt_running_course(ctx):
    """Start Break the Targets; with no target, place one; X; the first input starts the clock (`[Btt] phase=Running`)."""
    open_game(ctx, 'Break the Targets')
    out = {}
    # An earlier attempt may have left it running (its first row is then Stop): A stops it, the menu stays open, and the
    # next A starts it. The expected line carries the target count (boot 2: the match alone had no `n=`).
    ready = r'\[Btt\] phase=(Placing|Ready) n=\d+ '
    log = cursor(ctx)
    try:
        ctx.press('a')
        first = log.wait(r'\[Practice\] (start|stop) game=(\S+)', 4, ctx)
        if first.group(1) == 'start':
            out['start'] = {'started': first.group(2), 'line': log.wait(ready, 20, ctx).group(0)}
    finally:
        log.close()
    if first.group(1) == 'stop':
        out['stopped_first'] = first.group(0)
        out['start'] = start(ctx, 'Break the Targets', ready)
    log = cursor(ctx)
    try:
        if 'phase=Placing' in out['start']['line']:
            if ' n=0 ' in out['start']['line']:
                time.sleep(1.0)
                ctx.press('a')
                log.wait(r'\[Btt\] phase=Placing n=1 ', 10, ctx)
            time.sleep(0.5)
            ctx.press('x')
            log.wait(r'\[Btt\] phase=Ready ', 15, ctx)
        time.sleep(1.0)
        ctx.press('a')
        out['running'] = log.wait(r'\[Btt\] phase=Running ', 10, ctx).group(0)[:200]
    finally:
        log.close()
    return out


def btt_close_keeps_clock(ctx):
    """Primary 15:30Z (Q1 for Break the Targets): opening and closing the menu neither restarts the run nor resets its
    clock; only Start and Stop act."""
    out = btt_running_course(ctx)
    time.sleep(2.5)
    before, out['before'] = btt_clock(ctx)
    offset = trace_offset(ctx)
    to_list(ctx)
    focus(ctx, 'Break the Targets')
    out['list'] = shot(ctx, 'btt-list-running', offset, ('Running',))
    log = cursor(ctx)
    try:
        close_all(ctx)
        time.sleep(3.0)
        none_since(ctx, log, r'\[Practice\] (start|stop)|\[Btt\] phase=(Ready|Placing|Off) ')
    finally:
        log.close()
    after, out['after'] = btt_clock(ctx)
    if after < before:
        raise RuntimeError(f'the clock went back: {before} -> {after}')
    out['clock'] = {'before_open': before, 'after_close': after}
    out['shot'] = shot(ctx, 'btt-running-after-close')
    return save(ctx, 'btt-close-keeps-clock', out)


def btt_load_while_running(ctx):
    """A saved layout loaded while Break the Targets runs replaces the course and goes back to Ready with it (the Load's
    own effect, in the game pass); the close after it starts nothing."""
    folder = Path(ctx.instance['settings']).parent / 'layouts'
    folder.mkdir(exist_ok=True)
    (folder / 'practice-menu-two.json').write_text(json.dumps(
        {'schema_version': 1, 'name': 'Practice two', 'stage': 0, 'used': 9,
         'targets': [[-30.0, 30.0], [30.0, 30.0]]}) + '\n')
    try:
        out = {'running_before': btt_clock(ctx, 3)[1]}
    except RuntimeError:
        # A retry finds the course at Ready (attempt 1's Load): the first input starts the clock again.
        out = {'running_before': waits(ctx, 'a', r'\[Btt\] phase=Running [^\n]*', 10).group(0)[:200]}
    open_game(ctx, 'Break the Targets')
    focus_sub(ctx, 'Break the Targets', 'Saved layouts')
    waits(ctx, 'a', r'\[Ui\] press A .*screen=Some\(External\(Layouts\)\)', 4)
    # The Layouts screens are the external editor's: no `[Ui] press` lines, so its tile moves take short settles
    # (MACROS: a fixed wait only where no state exists); the newest-used layout is the first entry.
    time.sleep(0.6)
    ctx.press('a')
    time.sleep(0.6)
    # Three cursors from the Load press: the game pass may write Ready before the screen's reread (boot 2 attempt 2).
    log, ready, starts = cursor(ctx), cursor(ctx), cursor(ctx)
    try:
        ctx.press('a')
        out['reread'] = log.wait(r'\[Btt\] layouts stage=0 [^\n]*', 15, ctx).group(0)[:200]
        out['loaded_shot'] = shot(ctx, 'btt-layout-loaded-in-menu')
        close_all(ctx)
        # The game pass takes the layout (with the menu open or after the close): Ready with both targets up.
        out['ready'] = ready.wait(r'\[Btt\] phase=Ready n=2 [^\n]*', 15, ctx).group(0)[:200]
        time.sleep(1.0)
        none_since(ctx, starts, r'\[Practice\] (start|stop)')
    finally:
        log.close()
        ready.close()
        starts.close()
    out['after_close'] = 'no [Practice] start or stop line from the Load to 1 s after Ready'
    # Stop: the only way it ends.
    open_game(ctx, 'Break the Targets')
    out['stop'] = waits(ctx, 'a', r'\[Practice\] stop game=Some\(Btt\)', 4).group(0)
    close_all(ctx)
    return save(ctx, 'btt-load-while-running', out)


# ---- tr-review-astra-3 16:37Z findings (boot 3) ------------------------------------------------------------------

TECH_GRADE = (r'\[TrainingRooms\] notice_draw header="Tech Practice" body="(?!Choose a surface|Tech the selected|Finding)'
              r'[^"]* · (?P<n>\d+)/(?P<of>\d+) · streak \d+')


def start_game(ctx, game, expect, cap=20):
    """The game's sub-screen, A on its first row: Start, or Stop then Start when an earlier attempt left it running."""
    open_game(ctx, game)
    out = {}
    log = cursor(ctx)
    try:
        ctx.press('a')
        first = log.wait(r'\[Practice\] (start|stop) game=(\S+)', 4, ctx)
        if first.group(1) == 'start':
            out = {'started': first.group(2), 'line': log.wait(expect, cap, ctx).group(0)[:200]}
    finally:
        log.close()
    if first.group(1) == 'stop':
        out = {'stopped_first': first.group(0), **start(ctx, game, expect)}
    return out


def stop_game(ctx, game, code):
    open_game(ctx, game)
    line = waits(ctx, 'a', r'\[Practice\] stop game=Some\(' + code + r'\)', 4).group(0)
    close_all(ctx)
    return line


def set_row(ctx, game, title, presses, button):
    """In the game's sub-screen (open), focus `title` and press Left or Right `presses` times (each gated on its line)."""
    focus_sub(ctx, game, title)
    return [press(ctx, button) for _ in range(presses)][-1]


TECH_LEFT = (r'\[TrainingRooms\] notice_draw header="Tech Practice" body="[^"]* · (?P<m>\d+):(?P<s>\d\d) left · '
             r'streak \d+')


def tech_left(line):
    return int(line.group('m')) * 60 + int(line.group('s'))


def tech_menu_visit_keeps_clock(ctx):
    """Finding 1, on the session clock (boot 3: every attempt there graded "unavailable (not counted)", so the count
    could not move): a 1 minute session runs, the menu is visited and closed with B, and the time left goes on from
    where it was; a fresh session would read 1:00 again."""
    open_game(ctx, 'Tech practice')
    stored = json.loads(Path(ctx.instance['settings']).read_text())['menu']['tech_practice']
    shown = max(next(i for i, v in enumerate(stored) if v) - 1, 0)
    out = {'stored_mode': stored}
    if shown != 2:
        out['mode_row'] = set_row(ctx, 'Tech practice', 'Tech practice', (shown - 2) % 5, 'left')
        back = press(ctx, 'up')
        if back['row'] != 'Start':
            raise RuntimeError(f'not back on Start: {back}')
    log = cursor(ctx)
    try:
        ctx.press('a')
        first = log.wait(r'\[Practice\] (start|stop) game=(\S+)', 4, ctx)
        if first.group(1) == 'stop':
            out['stopped_first'] = first.group(0)
            time.sleep(0.5)
            ctx.press('a')
            log.wait(r'\[Practice\] start game=Tech', 4, ctx)
        log.wait(r'\[Ui\] menu closed ', 4, ctx)
        deadline = time.monotonic() + 60
        while True:
            line = log.wait(TECH_LEFT, max(1.0, deadline - time.monotonic()), ctx)
            if tech_left(line) <= 48 or time.monotonic() > deadline:
                break
    finally:
        log.close()
    before = tech_left(line)
    if before > 48:
        raise RuntimeError(f'the session clock did not run: {line.group(0)[-120:]}')
    out['before'] = line.group(0)[-160:]
    out['before_shot'] = shot(ctx, 'tech-clock-before-visit')
    open_menu(ctx)
    time.sleep(2.0)
    log, starts = cursor(ctx), cursor(ctx)
    try:
        close_plain(ctx)
        after = log.wait(TECH_LEFT, 30, ctx)
        none_since(ctx, starts, r'\[Practice\] (start|stop)')
    finally:
        log.close()
        starts.close()
    left = tech_left(after)
    if left > before:
        raise RuntimeError(f'the session clock went back: {before} s left before the visit, {left} s after')
    out.update(after=after.group(0)[-160:], seconds_left=[before, left], after_shot=shot(ctx, 'tech-clock-after-visit'))
    open_game(ctx, 'Tech practice')
    out['stop'] = waits(ctx, 'a', r'\[Practice\] stop game=Some\(Tech\)', 4).group(0)
    close_all(ctx)
    return save(ctx, 'tech-menu-visit-keeps-clock', out)


def tech_grades_without_visit(ctx):
    """Control for boot 3 (every attempt there graded "unavailable" after the first): a 20-attempt session with no menu
    visit; every grade notice for up to 90 s is kept. PASS when one counted grade (n >= 1) is seen."""
    open_game(ctx, 'Tech practice')
    stored = json.loads(Path(ctx.instance['settings']).read_text())['menu']['tech_practice']
    shown = max(next(i for i, v in enumerate(stored) if v) - 1, 0)
    out = {'stored_mode': stored}
    if shown:
        out['mode_row'] = set_row(ctx, 'Tech practice', 'Tech practice', shown, 'left')
        if press(ctx, 'up')['row'] != 'Start':
            raise RuntimeError('not back on Start')
    log, bodies = cursor(ctx), []
    try:
        ctx.press('a')
        first = log.wait(r'\[Practice\] (start|stop) game=(\S+)', 4, ctx)
        if first.group(1) == 'stop':
            time.sleep(0.5)
            ctx.press('a')
            log.wait(r'\[Practice\] start game=Tech', 4, ctx)
        log.wait(r'\[Ui\] menu closed ', 4, ctx)
        deadline = time.monotonic() + 90
        while time.monotonic() < deadline:
            try:
                line = log.wait(TECH_GRADE, max(1.0, deadline - time.monotonic()), ctx)
            except RuntimeError:
                break
            body = line.group(0).split('body="', 1)[1]
            if not bodies or bodies[-1] != body:
                bodies.append(body)
            if int(line.group('n')) >= 2:
                break
    finally:
        log.close()
    out['grades'] = bodies
    out['shot'] = shot(ctx, 'tech-grades-without-visit')
    open_game(ctx, 'Tech practice')
    out['stop'] = waits(ctx, 'a', r'\[Practice\] stop game=Some\(Tech\)', 4).group(0)
    close_plain(ctx)
    save(ctx, 'tech-grades-without-visit', out)
    if not any(re.search(r' · [1-9]\d*/20 · ', b) for b in bodies):
        raise RuntimeError(f'no counted grade without a menu visit: {bodies[:6]}')
    return out


def tech_menu_visit_keeps_session(ctx):
    """Finding 1 (row r20): the session clock check (`tech_menu_visit_keeps_clock`). Boot 3 graded every attempt
    "unavailable (not counted)", so the attempt count could not move there; the count variant stays below."""
    return tech_menu_visit_keeps_clock(ctx)


def tech_menu_visit_keeps_count(ctx):
    """Finding 1: a graded Tech session, a menu visit and B: the next grade continues the count (a fresh session would
    show 1/20 again). Mode set to 20 attempts in the sub-screen first (the fixture stores 3 minutes)."""
    open_game(ctx, 'Tech practice')
    # The presses to 20 attempts (the first shown mode) come from the stored row: shown = stored - 1, a stored Off is
    # the first mode (boot 3: a fixed count from the fixture's 3 minutes went wrong on a retry).
    stored = json.loads(Path(ctx.instance['settings']).read_text())['menu']['tech_practice']
    shown = max(next(i for i, v in enumerate(stored) if v) - 1, 0)
    out = {'stored_mode': stored}
    if shown:
        out['mode_row'] = set_row(ctx, 'Tech practice', 'Tech practice', shown, 'left')
    # Boot 4: on Surface Any a fresh session drew a spot its launches never reached (every attempt abandoned, "Grading
    # unavailable", before any menu visit). Floor gives contact; Surface is 2 rows below the mode row.
    surface = json.loads(Path(ctx.instance['settings']).read_text())['menu']['tech_surface']
    at = next(i for i, v in enumerate(surface) if v)
    if at != 1:
        out['surface_row'] = set_row(ctx, 'Tech practice', 'Surface', (1 - at) % 5, 'right')
    for _ in range(6):
        back = press(ctx, 'up')
        if back['row'] == 'Start':
            break
    if back['row'] != 'Start':
        raise RuntimeError(f'not back on Start: {back}')
    log = cursor(ctx)
    try:
        ctx.press('a')
        first = log.wait(r'\[Practice\] (start|stop) game=(\S+)', 4, ctx)
        if first.group(1) == 'stop':
            # An earlier attempt left a session on (its first row reads Stop): A stopped it; A starts a fresh one.
            out['stopped_first'] = first.group(0)
            time.sleep(0.5)
            ctx.press('a')
            log.wait(r'\[Practice\] start game=Tech', 4, ctx)
        log.wait(r'\[Ui\] menu closed ', 4, ctx)
        # The first counted grade (an uncounted "Grading unavailable · 0/20" can come first; boot 3 retry 2).
        deadline = time.monotonic() + 150
        while True:
            grade = log.wait(TECH_GRADE, max(1.0, deadline - time.monotonic()), ctx)
            if int(grade.group('n')) >= 1 or time.monotonic() > deadline:
                break
    finally:
        log.close()
    before, total = int(grade.group('n')), int(grade.group('of'))
    if total != 20 or before < 1:
        raise RuntimeError(f'expected a graded 20-attempt session: {grade.group(0)[:200]}')
    out['before'] = grade.group(0)[:220]
    out['before_shot'] = shot(ctx, 'tech-graded-before-visit')
    open_menu(ctx)
    time.sleep(1.0)
    # Boot 3: the Training HUD gate cannot pass while Tech practice launches the fighter, so the close is gated on the
    # DIAG's closed line only. Every grade after B is read: none may go below the count before the visit (a fresh
    # session would show 0 or 1/20 again), and one must go past it.
    log, starts = cursor(ctx), cursor(ctx)
    seen = []
    try:
        close_plain(ctx)
        deadline = time.monotonic() + 150
        while time.monotonic() < deadline:
            line = log.wait(TECH_GRADE, max(1.0, deadline - time.monotonic()), ctx)
            n = int(line.group('n'))
            seen.append(line.group(0)[-120:])
            if n < before:
                raise RuntimeError(f'the session restarted: {before}/20 before the visit, {n}/20 after: {seen}')
            if n > before:
                break
        none_since(ctx, starts, r'\[Practice\] (start|stop)')
    finally:
        log.close()
        starts.close()
    out.update(after=seen, counts=[before, n], after_shot=shot(ctx, 'tech-graded-after-visit'))
    open_game(ctx, 'Tech practice')
    out['stop'] = waits(ctx, 'a', r'\[Practice\] stop game=Some\(Tech\)', 4).group(0)
    close_all(ctx)
    return save(ctx, 'tech-menu-visit-keeps-session', out)


def close_plain(ctx):
    """B until the DIAG says the menu closed (at most four), with no HUD gate."""
    for _ in range(4):
        log = cursor(ctx)
        try:
            ctx.press('b')
            try:
                log.wait(r'\[Ui\] menu closed ', 1.5, ctx)
                return
            except RuntimeError:
                continue
        finally:
            log.close()
    raise RuntimeError('the menu did not close after four B')


def stop_running_tech(ctx):
    """Recovery: a running Tech practice keeps the HUD gate from passing. Its sub-screen draws Stop on its first row
    (traced set_text); A stops it there."""
    offset = trace_offset(ctx)
    open_game(ctx, 'Tech practice')
    time.sleep(0.8)
    if not traced(ctx, offset, 'Stop'):
        close_plain(ctx)
        return False
    waits(ctx, 'a', r'\[Practice\] stop game=Some\(Tech\)', 4)
    close_plain(ctx)
    return True


EGGS_N = r'\[Eggs\] notice="Eggs \d+/10 '
EGGS_MINUTE = r'\[Eggs\] notice="Eggs \d+ +\d:\d\d"'


def eggs_mode_applies_at_next_start(ctx):
    """Finding 2 (Eggs-ercise): the mode changed while 10 targets runs stays 10 targets after B; Stop and Start run the
    new mode (1 minute)."""
    out = {'start': start_game(ctx, 'Eggs-ercise', EGGS_N)}
    time.sleep(1.0)
    open_menu(ctx)
    out['mode_row'] = set_row(ctx, 'Eggs-ercise', 'Eggs-ercise', 1, 'left')
    log = cursor(ctx)
    try:
        close_all(ctx)
        out['after_b'] = log.wait(EGGS_N, 10, ctx).group(0)[:160]
        time.sleep(2.0)
        none_since(ctx, log, EGGS_MINUTE + r'|\[Practice\] (start|stop)')
    finally:
        log.close()
    out['after_b_shot'] = shot(ctx, 'eggs-mode-kept-after-b')
    open_game(ctx, 'Eggs-ercise')
    log = cursor(ctx)
    try:
        ctx.press('a')
        out['stop'] = log.wait(r'\[Practice\] stop game=Some\(Eggs\)', 4, ctx).group(0)
        time.sleep(0.5)
        ctx.press('a')
        log.wait(r'\[Practice\] start game=Eggs', 4, ctx)
        out['next_start'] = log.wait(EGGS_MINUTE, 15, ctx).group(0)[:160]
    finally:
        log.close()
    out['next_start_shot'] = shot(ctx, 'eggs-new-mode-at-next-start')
    out['stop_end'] = stop_game(ctx, 'Eggs-ercise', 'Eggs')
    return save(ctx, 'eggs-mode-applies-at-next-start', out)


def blaster(ctx, shots=3):
    for _ in range(shots):
        ctx.press('b')
        time.sleep(0.8)


def btt_projectiles_apply_at_next_start(ctx):
    """Finding 2 (Break the Targets): Projectiles Off at Start (the fixture), turned On while the run goes on, B: Fox's
    blaster still breaks nothing (no `[Btt] projectile` line). Stop and Start: the blaster breaks the target."""
    projectile = r'\[Btt\] projectile [^\n]*hits='
    # Boot 3: Fox stood where Tech practice left him (p1=[0.0, 47.2]). L+R+A first puts him back at the spawn, on the
    # ground at x -40 facing the CPU, so the target goes on the blaster's line in front of him.
    routes.play(ctx, 'reset-lra')
    time.sleep(2.0)
    routes.gate(ctx, 'training')
    out = {}
    # The rule must be Off at this Start: the fixture stores Off, a retry may find the On an earlier attempt set.
    stored = json.loads(Path(ctx.instance['settings']).read_text())['menu']['btt_projectiles']
    if stored == [0, 1]:
        open_game(ctx, 'Break the Targets')
        out['rule_off'] = set_row(ctx, 'Break the Targets', 'Projectiles break targets', 1, 'right')
        close_plain(ctx)
        time.sleep(1.0)
    out['start'] = start_game(ctx, 'Break the Targets', r'\[Btt\] phase=(Placing|Ready) n=\d+ ')
    if 'phase=Placing n=0 ' not in out['start']['line']:
        # An earlier attempt left a course: Place targets opens the placing view on it, and Y twice clears it.
        open_game(ctx, 'Break the Targets')
        focus_sub(ctx, 'Break the Targets', 'Place targets')
        log = cursor(ctx)
        try:
            ctx.press('a')
            log.wait(r'\[Btt\] phase=Placing n=\d+ ', 15, ctx)
            time.sleep(1.0)
            ctx.press('y')
            time.sleep(0.4)
            ctx.press('y')
            out['cleared'] = log.wait(r'\[Btt\] phase=Placing n=0 [^\n]*', 10, ctx).group(0)[:200]
        finally:
            log.close()
        out['start']['line'] = out['cleared']
        time.sleep(0.8)
    log = cursor(ctx)
    try:
        if 'phase=Placing n=0 ' in out['start']['line']:
            # One target at blaster height in front of Fox: the cursor starts mid-stage 15 above the ground; 15 left
            # and 8 down put it at x -15, about 7 above the ground.
            for button, count in (('left', 15), ('down', 8)):
                for _ in range(count):
                    ctx.press(button)
                    time.sleep(0.15)
            time.sleep(0.6)
            ctx.press('a')
            out['placed'] = log.wait(r'\[Btt\] phase=Placing n=1 [^\n]*', 10, ctx).group(0)[:200]
            time.sleep(0.5)
        ctx.press('x')
        out['ready'] = log.wait(r'\[Btt\] phase=Ready n=1 [^\n]*', 15, ctx).group(0)[:200]
        time.sleep(1.0)
        blaster(ctx)
        time.sleep(1.0)
        none_since(ctx, log, projectile)
    finally:
        log.close()
    out['off_shot'] = shot(ctx, 'btt-blaster-off-at-start')
    open_game(ctx, 'Break the Targets')
    out['rule_row'] = set_row(ctx, 'Break the Targets', 'Projectiles break targets', 1, 'right')
    log = cursor(ctx)
    try:
        close_all(ctx)
        time.sleep(1.0)
        blaster(ctx)
        time.sleep(1.0)
        none_since(ctx, log, projectile + r'|\[Practice\] (start|stop)|\[Btt\] phase=(Ready|Done) ')
    finally:
        log.close()
    out['on_after_b_shot'] = shot(ctx, 'btt-blaster-still-off-after-b')
    open_game(ctx, 'Break the Targets')
    log = cursor(ctx)
    try:
        ctx.press('a')
        out['stop'] = log.wait(r'\[Practice\] stop game=Some\(Btt\)', 4, ctx).group(0)
        time.sleep(0.5)
        ctx.press('a')
        log.wait(r'\[Practice\] start game=Btt', 4, ctx)
        out['ready_again'] = log.wait(r'\[Btt\] phase=Ready n=1 [^\n]*', 15, ctx).group(0)[:200]
        time.sleep(1.0)
        blaster(ctx)
        out['hit'] = log.wait(projectile, 10, ctx).group(0)[:220]
    finally:
        log.close()
    out['hit_shot'] = shot(ctx, 'btt-blaster-breaks-at-next-start')
    out['stop_end'] = stop_game(ctx, 'Break the Targets', 'Btt')
    return save(ctx, 'btt-projectiles-apply-at-next-start', out)


MASH_NOTICE = r'\[Mashout\] notice gen=(?P<gen>\d+) text=(?P<text>[^\n]+)'


def mashout_start_stop(ctx):
    """Mash out, the fourth game (main 6f564b86 merged): its Practice games row opens its sub-screen; Start runs it (a
    notice); a menu visit with a State edit and B keeps the same run (same generation, no [Practice] line: the edit
    waits for the next Start); Stop ends it (an empty notice)."""
    out = {}
    open_game(ctx, 'Mash out')
    log = cursor(ctx)
    try:
        ctx.press('a')
        first = log.wait(r'\[Practice\] (start|stop) game=(\S+)', 4, ctx)
        if first.group(1) == 'stop':
            out['stopped_first'] = first.group(0)
            time.sleep(0.5)
            ctx.press('a')
            log.wait(r'\[Practice\] start game=Mashout', 4, ctx)
        log.wait(r'\[Ui\] menu closed ', 4, ctx)
        running = log.wait(MASH_NOTICE, 30, ctx)
    finally:
        log.close()
    gen = int(running.group('gen'))
    out['running'] = running.group(0)[:200]
    out['running_shot'] = shot(ctx, 'mashout-running')
    open_menu(ctx)
    out['state_row'] = set_row(ctx, 'Mash out', 'State', 1, 'right')
    log, starts = cursor(ctx), cursor(ctx)
    try:
        close_plain(ctx)
        time.sleep(6.0)
        # No Start or Stop, and no cancel (an empty notice): the run goes on with the State its Start latched.
        none_since(ctx, starts, r'\[Practice\] (start|stop)|\[Mashout\] notice gen=\d+ text=\s*$')
    finally:
        log.close()
        starts.close()
    out['after_b_shot'] = shot(ctx, 'mashout-kept-after-b')
    open_game(ctx, 'Mash out')
    log = cursor(ctx)
    try:
        ctx.press('a')
        out['stop'] = log.wait(r'\[Practice\] stop game=Some\(Mashout\)', 4, ctx).group(0)
        close_plain(ctx)
        out['stopped'] = log.wait(r'\[Mashout\] notice gen=\d+ text=\s*$', 10, ctx).group(0)[:160]
    finally:
        log.close()
    out['generation'] = gen
    return save(ctx, 'mashout-start-stop', out)


def gif(ctx, name, seconds, act):
    folder = Path(ctx.row_dir)
    path = folder / (name + '.gif')
    ctx.call('recording.start', {'path': str(path), 'fps': 15, 'width': 1280, 'duration_ms': int(seconds * 1000)})
    try:
        act()
    finally:
        for _ in range(int(seconds) + 20):
            status = ctx.call('recording.status', {}) or {}
            if status.get('state') in ('idle', 'finished', 'stopped') and path.exists():
                break
            time.sleep(1)
    return {'gif': str(path), 'status': status}


def docs_gifs(ctx):
    """Two docs clips (1280 wide, under 10 s): the list → Eggs-ercise → Start; then Running → Stop."""
    out = {}

    def flow():
        to_list(ctx)
        focus(ctx, 'Eggs-ercise')
        time.sleep(0.8)
        press(ctx, 'a')
        time.sleep(1.2)
        ctx.press('a')
        time.sleep(3.0)
    out['start'] = gif(ctx, 'practice-menu-start', 9.0, flow)

    def stop():
        open_menu(ctx)
        time.sleep(0.8)
        press(ctx, 'b')
        time.sleep(1.5)
        press(ctx, 'a')
        time.sleep(1.2)
        ctx.press('a')
        time.sleep(1.5)
        close_all(ctx)
        time.sleep(1.0)
    out['stop'] = gif(ctx, 'practice-menu-stop', 9.0, stop)
    return save(ctx, 'docs-gifs', out)


def recover(ctx):
    for _ in range(4):
        if routes.passes(ctx, 'training', 2):
            return {'recovered': True}
        ctx.press('b')
        time.sleep(0.5)
    # Boot 3: a Tech practice left running by a failed row keeps launching the fighter; stop it, then gate.
    stopped = stop_running_tech(ctx)
    time.sleep(2.0)
    routes.gate(ctx, 'training')
    return {'recovered': True, 'stopped_tech': stopped}


def console_scan(ctx):
    text = routes.instance_path(ctx, 'console_log').read_text(errors='replace')
    findings = [line for line in text.splitlines() if re.search(r'panic|abort|crash|Unmapped', line, re.I)]
    counters = ctx.call('trace.counters', {})
    unmapped = ctx.call('trace.unmapped_access', {})
    save(ctx, 'console-scan', {'findings': findings, 'trace_counters': counters, 'unmapped': unmapped})
    if findings:
        raise RuntimeError(f'{len(findings)} console findings need review')
    return {'findings': 0}
