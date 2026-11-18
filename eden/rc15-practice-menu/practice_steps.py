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
GROUPS = {'Tech practice': 21, 'Eggs-ercise': 22, 'Break the Targets': 23}


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
    if started != {'Eggs-ercise': 'Eggs', 'Break the Targets': 'Btt', 'Tech practice': 'Tech'}[game]:
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
    routes.gate(ctx, 'training')
    return {'recovered': True}


def console_scan(ctx):
    text = routes.instance_path(ctx, 'console_log').read_text(errors='replace')
    findings = [line for line in text.splitlines() if re.search(r'panic|abort|crash|Unmapped', line, re.I)]
    counters = ctx.call('trace.counters', {})
    unmapped = ctx.call('trace.unmapped_access', {})
    save(ctx, 'console-scan', {'findings': findings, 'trace_counters': counters, 'unmapped': unmapped})
    if findings:
        raise RuntimeError(f'{len(findings)} console findings need review')
    return {'findings': 0}
