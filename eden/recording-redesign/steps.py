"""rc.5 grouped-session step hooks (card 10a2, 2026-11-07): one function per row of rows.json.

Copies the rows-iv pattern (iv_steps.py): every row is one {"run": "steps.py:<row>", "check": true}
step. The hook reaches the row's start state, performs the row's documented inputs, waits for the
row's expected diag line or screen, saves its evidence into the row folder, and raises with the
captured data when the expectation is not met. Chords and the movement/attack inputs are recipe
segments (recorded from their recipe on first play; MACROS, "Test runs"); the C-stick is the one
input a recipe cannot express and is pressed by its row hook (README, Open questions). Diag lines
come from the stage-1 report's Design section: screen=recording-popup, screen=recording-page,
playback, load, drill, notice, migrate (plus the recording/pick/mismatch/rename/crop lines the
RR-6/RR-7 Eden tables name).

Facts a source did not give (menu cursor routes inside the new screens, the published frame-data
pages, the hook-stats line format) are marked in README.md, Open questions; the hooks bounded-try
the documented inputs and raise with what they saw instead of passing on an unverified screen.
"""
import hashlib
import json
import re
import struct
import time
from pathlib import Path

# The factory chord macros (segments/, authored same-frame per docs/MACROS.md rule 14).
SAVE, LOAD = 'chord-save', 'chord-load'
RECORD, PLAY_CPU, PLAY_P1, POPUP = 'chord-record', 'chord-play-cpu', 'chord-play-p1', 'chord-popup'
MENU_OPEN, MENU_CLOSE, X = 'menu-open', 'menu-close', 'x-press'
RESET_WALK, RESET_CENTER, JAB, FSMASH = 'reset-walk', 'reset-center', 'jab', 'fsmash'


def save(ctx, name, value):
    (ctx.row_dir / (name + '.json')).write_text(json.dumps(value, indent=2, default=str) + '\n')
    return value


def press(ctx, button, gap=0.4):
    ctx.press(button, ms=150)
    time.sleep(gap)


def settle(ctx, cap=20):
    ctx.gate('training-hud-unpaused')


def open_menu(ctx):
    ctx.play(MENU_OPEN)
    time.sleep(1.4)


def close_menu(ctx):
    ctx.play(MENU_CLOSE)
    time.sleep(0.3)
    # The close chord is one B. A portrait grid or a confirm sits under the page, so two more
    # B presses back out before the HUD check. Leaving the menu up makes the next gate miss.
    press(ctx, 'b', 0.35)
    press(ctx, 'b', 0.35)
    time.sleep(0.4)
    settle(ctx)


def _save_files(ctx):
    folder = Path(ctx.instance['diag_log']).parent / 'saves'
    if not folder.is_dir():
        return {}
    return {path.name: path.read_bytes() for path in folder.glob('*.json')}


def _restore_saves(ctx, backup):
    """Put back a save file this row deleted. The macro keeps the start_save name."""
    folder = Path(ctx.instance['diag_log']).parent / 'saves'
    folder.mkdir(parents=True, exist_ok=True)
    restored = []
    for name, data in backup.items():
        target = folder / name
        if not target.exists():
            target.write_bytes(data)
            restored.append(name)
    return restored


def _shield_index(ctx):
    """First set byte of menu.shield_state. None is 0, Hold is 2. Written on menu close."""
    row = _menu_values(ctx).get('shield_state') or []
    for index, value in enumerate(row):
        if value:
            return index
    return 0


def _menu_values(ctx):
    path = Path(ctx.instance['diag_log']).parent / 'settings.json'
    if not path.is_file():
        return {}
    try:
        menu = json.loads(path.read_text()).get('menu') or {}
    except json.JSONDecodeError:
        return {}
    return menu if isinstance(menu, dict) else {}


def _focus_row(ctx, title):
    """Up until the diag line names `title`. Up wraps, so one lap reaches any focusable row."""
    path = Path(gate_source(ctx))
    offset = path.stat().st_size if path.is_file() else 0
    seen = []
    for _ in range(24):
        press(ctx, 'up', 0.28)
        offset, match = _await_ui(ctx, offset, cap=1.2)
        if match is None:
            return False
        if match.group('row') == title:
            return True
        cursor = int(match.group('cursor'))
        if cursor in seen:
            return False
        seen.append(cursor)
    return False


def _display_wrong(ctx):
    menu = _menu_values(ctx)
    want = []
    if menu.get('hud') == [0, 1]:
        want.append('HUD')
    if menu.get('hitbox_vis') == [1, 0]:
        want.append('Hitbox Visualization')
    return want


def _apply_display(ctx, want):
    """One A per row, then B. settings.json is written when the menu closes, so a second A
    before that close flips the row back."""
    open_menu(ctx)
    goto_page(ctx, 'display_controls')
    for title in want:
        if not _focus_row(ctx, title):
            press(ctx, 'b', 0.4)
            raise RuntimeError('recover could not focus %s' % title)
        press(ctx, 'a', 0.45)
    press(ctx, 'b', 0.45)


def _restore_display(ctx):
    """Put the native HUD back and clear hitboxes when a row toggled them.

    HUD off hides the native zero the start gate matches. Hitboxes on can cover that digit.
    A is the toggle. Left and Right do not change it. A release build emits no row line, so
    this leaves the page instead of pressing A on an unknown row.
    """
    want = _display_wrong(ctx)
    if not want:
        return
    _apply_display(ctx, want)
    deadline = time.time() + 2.0
    while time.time() < deadline and _display_wrong(ctx):
        time.sleep(0.1)
    want = _display_wrong(ctx)
    if not want:
        return
    _apply_display(ctx, want)
    deadline = time.time() + 2.0
    while time.time() < deadline and _display_wrong(ctx):
        time.sleep(0.1)
    want = _display_wrong(ctx)
    if want:
        raise RuntimeError('recover left %s' % ', '.join(want))


def recover_training(ctx):
    """Leave a menu, pop-up, crop screen, or prompt, then show the unpaused Training HUD.

    B cancels a mismatch prompt and backs out of a crop screen, a file, a picker, and the
    page. Eight presses cover a stack of those. The training reset then clears a hit flash
    so the native zero is undimmed. The runner checks that gate on this same instance.
    """
    for _ in range(8):
        press(ctx, 'b', 0.28)
    _restore_display(ctx)
    ctx.play(RESET_CENTER)
    time.sleep(0.6)


def gate_source(ctx):
    """The file a scripted wait reads.

    The diag window declares `gates` (practice-gates.log, which the diag build does not
    rotate). The release window does not write that file, so a wait there reads
    practice-diagnostics.log.
    """
    gates = ctx.instance.get('gates')
    if gates:
        return str(gates)
    return str(ctx.instance['diag_log'])


# Developer stays out of the ring unless developer.ini turns it on. Six R presses are one
# past a full trip of this ring, so they do not stop on Recording.
PAGES = ('cpu', 'saves', 'recording', 'fighters_stages', 'display_controls')
_UI_PRESS = re.compile(
    r'\[Ui\] press (?P<button>\w+) page=(?P<page>\S+) cursor=(?P<cursor>\d+)'
    r' depth=(?P<depth>\d+) .*? row=(?:Some\("(?P<row>[^"]*)"\)|None)'
    r' recording=(?P<recording>[^\n]*)'
)


def _menu_page(ctx):
    path = Path(ctx.instance['diag_log']).parent / 'settings.json'
    page = 'cpu'
    if path.is_file():
        try:
            page = json.loads(path.read_text()).get('last_page') or 'cpu'
        except json.JSONDecodeError:
            page = 'cpu'
    return page if page in PAGES else 'cpu'


def _ui_since(ctx, offset):
    path = Path(gate_source(ctx))
    if not path.is_file():
        return offset, []
    data = path.read_bytes()
    if len(data) < offset:
        offset = 0
    # A short read can end in the middle of a line. Advance only through the last complete match
    # so the next poll still sees that line.
    chunk = data[offset:].decode('utf-8', 'replace')
    found = list(_UI_PRESS.finditer(chunk))
    if not found:
        return offset, []
    return offset + found[-1].end(), found


def _last_ui(ctx):
    """The newest [Ui] press line, or None when this window has not written one."""
    path = Path(gate_source(ctx))
    if not path.is_file() or path.stat().st_size == 0:
        return None
    found = list(_UI_PRESS.finditer(path.read_bytes()[-16000:].decode('utf-8', 'replace')))
    return found[-1] if found else None


def _await_ui(ctx, offset, cap=2.0):
    deadline = time.time() + cap
    while time.time() < deadline:
        offset, found = _ui_since(ctx, offset)
        if found:
            return offset, found[-1]
        time.sleep(0.1)
    return offset, None


def goto_page(ctx, page):
    """R until the diag line names `page`.

    The menu reopens on last_page, and that file can lag the page actually open. A missing
    line is not arrival: a fixed hop count then walks off the page and the next A hits
    whatever row that page remembered. One R reveals the page after the press, and further
    R presses stop when the line names the target.
    """
    if page not in PAGES:
        raise RuntimeError('unknown menu page %s' % page)
    path = Path(gate_source(ctx))
    offset = path.stat().st_size if path.is_file() else 0
    landed = None
    misses = 0
    for _ in range(len(PAGES) + 3):
        if landed == page:
            return offset
        press(ctx, 'r', 0.45)
        offset, match = _await_ui(ctx, offset, cap=1.5)
        if match is None:
            misses += 1
            if misses >= 3:
                break
            continue
        misses = 0
        landed = match.group('page')
    if landed != page:
        raise RuntimeError('menu stayed on %s, wanted %s' % (landed, page))
    return offset


def _jump_top(ctx):
    """Put the cursor on the first focusable row of this page.

    ZL+Up stays put once it is already in the first section, so it does not reach that
    section's first row. Up wraps. The diag line's cursor falls until it is 0, which is
    the first focusable entry. A press that the log does not answer is sent again.
    A release NRO does not emit the line.
    """
    path = Path(gate_source(ctx))
    offset = path.stat().st_size if path.is_file() else 0
    previous = None
    misses = 0
    for _ in range(20):
        press(ctx, 'up', 0.3)
        offset, match = _await_ui(ctx, offset, cap=1.2)
        if match is None:
            misses += 1
            if misses >= 3:
                return False
            continue
        misses = 0
        cursor = int(match.group('cursor'))
        if cursor == 0:
            return True
        if previous is not None and cursor > previous:
            press(ctx, 'down', 0.3)
            return True
        previous = cursor
    return True


def _file_title(match):
    return '' if match is None else (match.group('recording') or '').strip()


def _gate_offset(ctx):
    path = Path(gate_source(ctx))
    return path.stat().st_size if path.is_file() else 0


def _page_field(line, name):
    match = re.search(r'(?:^|\s)%s=(\S+)' % re.escape(name), line)
    return match.group(1) if match else ''


def _last_page_line(ctx):
    """Newest Recording-page line for the saves/macros group. The library index is not the page cursor."""
    path = Path(gate_source(ctx))
    if not path.is_file():
        return ''
    tail = path.read_bytes()[-65536:].decode('utf-8', 'replace')
    line = ''
    for row in tail.splitlines():
        if 'screen=recording-page' in row and 'group=saves-macros' in row:
            line = row
    return line


def _await_page_line(ctx, previous, cap=1.5):
    deadline = time.time() + cap
    while time.time() < deadline:
        line = _last_page_line(ctx)
        if line and line != previous:
            return line
        time.sleep(0.1)
    return _last_page_line(ctx)


def _library_index_zero(ctx):
    """Leave the library upward, then re-enter it.

    Up leaves the library only from its first item. The page cursor wraps, and Down
    returns to that same first item. Right copies the source column's index, so this
    has to happen on the saves column before the swap onto macros.
    """
    offset = _gate_offset(ctx)
    left = False
    for _ in range(30):
        press(ctx, 'up', 0.28)
        offset, match = _await_ui(ctx, offset, cap=1.0)
        if match is None or match.group('page') != 'recording':
            continue
        if int(match.group('cursor')) != 0:
            left = True
            break
    if not left:
        raise RuntimeError('library did not leave its first item')
    press(ctx, 'down', 0.35)
    offset, match = _await_ui(ctx, offset, cap=1.2)
    if match is None or match.group('page') != 'recording' or int(match.group('cursor')) != 0:
        raise RuntimeError('library did not re-enter at its first item')


def _macros_have_file(line):
    if _page_field(line, 'header') != 'off':
        return False
    if _page_field(line, 'floppy') == 'on':
        return True
    return _page_field(line, 'hot') not in ('', '0') or _page_field(line, 'hairline') not in ('', '0')


def _await_title(ctx, offset, cap=1.5):
    deadline = time.time() + cap
    while time.time() < deadline:
        offset, match = _await_ui(ctx, offset, cap=0.4)
        title = _file_title(match)
        if title:
            return offset, title
        if match is None:
            time.sleep(0.05)
    return offset, ''


def open_library_item(ctx, macros, downs=0):
    """Open the first save, or a macro file. Menu must end closed by the caller.

    Right swaps columns and copies the source index. The macro column lists files
    first and "+ New chain" last, so a saves index that is not 0 lands on New chain.
    A there opens the chain picker. B finishes that picker and opens the Name chain
    keyboard; another B creates the chain. Neither B backs out, so this route never
    presses B on an unopened file. It resets the saves index to 0, swaps, and presses
    A only once the macro column shows a file.
    """
    open_menu(ctx)
    goto_page(ctx, 'recording')
    _jump_top(ctx)
    if macros:
        before = _last_page_line(ctx)
        press(ctx, 'right', 0.4)
        landed = _await_page_line(ctx, before)
        if _page_field(landed, 'header') != 'on':
            before = landed
            press(ctx, 'right', 0.4)
            landed = _await_page_line(ctx, before)
        if _page_field(landed, 'header') != 'on':
            raise RuntimeError('saves column not shown (%s)' % landed[-180:])
        _library_index_zero(ctx)
        before = _last_page_line(ctx)
        press(ctx, 'right', 0.45)
        line = _await_page_line(ctx, before)
        if not _macros_have_file(line):
            raise RuntimeError('macro column has no file (%s)' % line[-180:])
        for _ in range(downs):
            press(ctx, 'down', 0.35)
        offset = _gate_offset(ctx)
        press(ctx, 'a', 0.55)
        offset, title = _await_title(ctx, offset)
        if not title:
            raise RuntimeError('macro file not opened (%s)' % line[-180:])
        return offset, title
    offset = _gate_offset(ctx)
    press(ctx, 'a', 0.6)
    offset, match = _await_ui(ctx, offset)
    return offset, _file_title(match)


def down_to_file_row(ctx, offset, current, title):
    """Move the open file screen onto `title`. The diag line names that row after each press."""
    for _ in range(8):
        if current == title:
            return offset
        press(ctx, 'down', 0.35)
        offset, match = _await_ui(ctx, offset)
        current = _file_title(match)
    raise RuntimeError('file row %s not reached (last %s)' % (title, current))


def popup(ctx):
    ctx.play(POPUP)
    try:
        return ctx.wait_log(gate_source(ctx), r'screen=recording-popup\b', cap=10)
    except RuntimeError:
        popup_close(ctx)
        raise


def show_popup(ctx):
    """Open the pop-up on a release NRO. The screen= line is diag-only, so this does not wait for it."""
    ctx.play(POPUP)
    time.sleep(0.8)


def popup_close(ctx):
    # A can open the file screen under the list. One B returns to the list and leaves it up,
    # and the next row's training-HUD recovery then stops the run.
    press(ctx, 'b', 0.45)
    press(ctx, 'b', 0.45)


# Screens that sit on top of Training. A missed gate backs out of these only.
# Gameplay lines (playback, a FrameStrip) are not in this set: B there is Neutral Special.
_OVERLAY_SCREENS = {
    'recording-page', 'recording-popup', 'rename', 'crop', 'mismatch',
    'notice', 'make-combo', 'migrate',
}


def _ui_press_seen(ctx):
    """False on the release NRO: its practice-gates.log has no [Ui] press lines."""
    path = Path(gate_source(ctx))
    if not path.is_file():
        return False
    return b'[Ui] press' in path.read_bytes()


def _overlay_open(ctx):
    """True when the gates log's latest screen= is a menu, pop-up, or keyboard."""
    path = Path(gate_source(ctx))
    if not path.is_file() or path.stat().st_size == 0:
        return False
    screen = ''
    for row in path.read_bytes()[-8192:].decode('utf-8', 'replace').splitlines():
        found = re.search(r'screen=(\S+)', row)
        if found:
            screen = found.group(1)
    return screen in _OVERLAY_SCREENS


def diag(ctx, pattern, cap=10):
    try:
        return ctx.wait_log(gate_source(ctx), pattern, cap=cap)
    except RuntimeError:
        # Back out only when a menu, pop-up, or keyboard is actually up. B during gameplay
        # is Neutral Special and the next row inherits the damage.
        if _overlay_open(ctx):
            for _ in range(4):
                press(ctx, 'b', 0.3)
        raise


def gif(ctx, name, seconds, do):
    """Record the row's GIF (Eden's own recorder, 1280 wide, game speed) around the documented inputs."""
    path = ctx.row_dir / name
    ctx.call('recording.start', {'path': str(path), 'fps': 15, 'width': 1280,
                                 'duration_ms': int(seconds * 1000)})
    time.sleep(0.3)
    do()
    status = None
    for _ in range(40):
        status = ctx.call('recording.status')
        if isinstance(status, dict) and status.get('state') not in ('recording', 'finalizing'):
            break
        time.sleep(0.5)
    if not path.exists():
        raise RuntimeError('no GIF written: %s' % status)
    screen = GIF_GATES.get(name)
    line = None
    if screen:
        line = diag(ctx, screen, cap=12)
    return {'gif': str(path), 'seconds': seconds, 'bytes': path.stat().st_size,
            'status': status, 'screen': line}


# A media row fails unless this line was written while its clip ran. The line is the screen the
# docs say must be visible. Frame-strip clips use the strip's own diag line.
GIF_GATES = {
    'rc5-record-start-save.gif': r'screen=recording role=player recording=1',
    'rc5-play-chord.gif': r'screen=playback role=',
    'rc5-loop-no-warp.gif': r'screen=recording-popup role=player loop=2',
    'rc5-chain-play.gif': r'screen=playback .*warp=1',
    'rc5-popup.gif': r'screen=recording-popup',
    'rc5-recording-page.gif': r'screen=recording-page',
    'rc5-macro-screen.gif': r'screen=recording-page',
    'rc5-fighter-mismatch.gif': r'screen=mismatch ask=macro1',
    'rc5-make-combo.gif': r'screen=make-combo source=\S+',
    'rc5-crop-combo.gif': r'screen=crop source=\S+ combo=1',
    'rc5-grade-not-true.gif': r'\[FrameStrip\] link verdict=',
    'rc5-sequence-drill.gif': r'\[Ui\] press .*Drill',
    'rc5-frame-strip-lanes.gif': r'\[FrameStrip\] p1 ',
    'rc5-frame-strip-chips.gif': r'\[FrameStrip\] p1 ',
    'rc5-frame-strip-stick-row.gif': r'\[FrameStrip\] p1 ',
    'rc5-frame-strip-cstick.gif': r'\[FrameStrip\] p1 ',
    'rc5-frame-strip-freeze.gif': r'\[FrameStrip\] p1 ',
    'rc5-frame-strip-freeze-stick.gif': r'\[FrameStrip\] p1 ',
    'rc5-frame-strip-advantage-hit.gif': r'\[FrameStrip\] advantage value=-?\d+ on_shield=false',
    'rc5-frame-strip-advantage-shield.gif': r'\[FrameStrip\] advantage value=-?\d+ on_shield=true',
    'rc5-frame-strip-link.gif': r'\[FrameStrip\] link verdict=',
    'rc5-frame-strip-drop.gif': r'\[FrameStrip\] link verdict=',
    'rc5-frame-strip-input.gif': r'\[FrameStrip\] p1 ',
    'rc5-frame-strip-last.gif': r'\[FrameStrip\] link verdict=',
    'rc5-frame-strip-exchange.gif': r'\[FrameStrip\] link verdict=',
}


def console_scan(ctx):
    """No Unmapped and no panic lines in the console log (the whole file, this run)."""
    text = Path(ctx.instance['console_log']).read_text(errors='replace')
    bad = [l for l in text.splitlines() if ('Unmapped' in l or 'panic' in l.lower())]
    if bad:
        raise RuntimeError('console has Unmapped/panic lines: %s' % bad[:5])
    return {'scanned_bytes': len(text), 'unmapped_or_panic': 0}


def install_heap(ctx):
    """TrainingRooms' own install line, not the later cpu_common_load line and not the console.

    practice-diagnostics.log and events.log sit in the same folder
    (user/sdmc/ultimate/TrainingRooms/). The install line is
    plugin=training_rooms/… result=installed heap_allocations=0. The following
    cpu_common_load line also contains cpu_common=installed and has no heap count.
    """
    diag = Path(ctx.instance['diag_log'])
    events = diag.with_name('events.log')
    text = ''
    for p in (events.with_name('events.previous.log'), events):
        if p.exists():
            text += p.read_text(errors='replace')
    line = [l for l in text.splitlines()
            if 'plugin=training_rooms/' in l and 'result=installed' in l]
    if not line:
        raise RuntimeError('no TrainingRooms result=installed line in %s' % events)
    m = re.search(r'heap_allocations=(\d+)', line[-1])
    if not m:
        raise RuntimeError('install line has no heap_allocations count: %s' % line[-1][:240])
    if m.group(1) != '0':
        raise RuntimeError('install line heap_allocations=%s' % m.group(1))
    return line[-1]


def hook_stats(ctx):
    """The hook-stats snapshot: no steady-state hook allocations after the whole session.

    The snapshot's line format is not given in any file this card names (README, Open questions);
    the hook copies the diag lines that name hook_stats into the row folder and fails on any
    steady-state counter that reads nonzero.
    """
    text = ''
    diag_log = Path(ctx.instance.get('diag_log') or ctx.instance['console_log'])
    for p in (diag_log.with_name('practice-diagnostics.previous.log'), diag_log):
        if p.exists():
            text += p.read_text(errors='replace')
    lines = [l for l in text.splitlines() if 'hook_stats' in l or 'hook-stats' in l]
    if lines:
        (ctx.row_dir / 'hook-stats-snapshot.log').write_text('\n'.join(lines[-50:]) + '\n')
        bad = [l for l in lines if re.search(r'steady[_ ]state.*alloc(ations)?=(\d+)', l)
               and l.split('alloc')[-1].strip().startswith(('=1', '=2', '=3', '=4', '=5', '=6',
                                                            '=7', '=8', '=9'))]
        if bad:
            raise RuntimeError('steady-state hook allocations: %s' % bad[:3])
    return {'lines': len(lines), 'snapshot': 'hook-stats-snapshot.log' if lines else None}


# --- the boot route (every group's first row; rows-iv base steps.py pattern) ----------------------

STEP_CSS = re.compile(r'step screen=css dir=(\w+) from=(\S+) to=(\S+)')
SETTLED_CSS = re.compile(r'settled screen=css at=(\d+) hovered=(\d+)')


def css_chord(ctx, buttons, hold_ms=180):
    """L3 first, then the D-pad, both held, then released. One tile (TILE-CURSOR.md)."""
    lease = hold_ms + 500
    for button in buttons:
        ctx.call('input.button', {'button': button, 'state': 'press', 'lease_ms': lease})
    time.sleep(hold_ms / 1000)
    for button in reversed(buttons):
        ctx.call('input.button', {'button': button, 'state': 'release'})
    ctx.call('input.reset')


def css_one_step(path, offset):
    deadline = time.time() + 4
    step = None
    settled = None
    collected = []
    while time.time() < deadline:
        data = path.read_bytes()
        if len(data) < offset:
            offset = 0
        lines = [line for line in data[offset:].decode('utf-8', 'replace').splitlines() if line.strip()]
        offset = len(data)
        collected.extend(lines)
        for line in lines:
            found = STEP_CSS.search(line)
            if found:
                step = found
            stopped = SETTLED_CSS.search(line)
            if stopped:
                settled = stopped
        if step and (step.group(3) == 'none' or settled):
            return offset, step, settled, collected
        time.sleep(0.1)
    return offset, step, settled, collected


def css_to_mario(ctx):
    """Up to the edge, then left to the edge. Surface recover-enter-training-b settled at tile 0 this way.

    The analog stick does not move this cursor (surface setup, XPS rr-migrate). One A on tile 0
    readies the match while the CPU coin is still Random (yoga and surface). settings.json did not
    override that coin. The re-run still requires settled screen=css at=0 hovered=0 before A.
    """
    path = Path(ctx.instance['tile_log'])
    offset = path.stat().st_size
    trace = []
    for direction in ('up', 'left'):
        reached = False
        for _ in range(16):
            ctx.check_stop()
            css_chord(ctx, ['lstick', direction], 180)
            offset, step, settled, lines = css_one_step(path, offset)
            trace.append({'direction': direction,
                          'step': None if step is None else step.group(0),
                          'settled': None if settled is None else settled.group(0)})
            if step is None:
                raise RuntimeError('no css step after %s; lines=%s' % (direction, lines[-4:]))
            if step.group(1) != direction:
                raise RuntimeError('step dir %s after pressing %s: %s' % (step.group(1), direction, step.group(0)))
            if step.group(3) == 'none':
                reached = True
                break
            if settled and settled.group(1) != settled.group(2):
                raise RuntimeError('hovered and settled tiles differ: %s' % settled.group(0))
        if not reached:
            raise RuntimeError('%s did not reach the edge in 16 steps' % direction)
    ctx.shot('top-left')
    last = next((item['settled'] for item in reversed(trace) if item['settled']), None)
    if last != 'settled screen=css at=0 hovered=0':
        raise RuntimeError('tile cursor did not settle on Mario: %s' % last)
    return {'last_settled': last, 'steps': [item['step'] for item in trace]}


def wait_loaded(ctx, cap=180):
    """game.load returns while shaders are still building. The next gate then sees not_ready."""
    deadline = time.time() + cap
    while time.time() < deadline:
        ctx.check_stop()
        state = ctx.call('status') or {}
        shaders = state.get('shaders_building')
        if state.get('loading_complete') and not shaders and (state.get('game_fps') or 0) >= 30:
            return
        time.sleep(1)
    raise RuntimeError('game still loading after %ss' % cap)


def _pick_fighter(ctx, downs, neighbor):
    """A portrait on Fighters & Stages. The grid opens on Mario. One Right is the next fighter.

    ZL+Up lands on Player Swapping. One Down is Player Character. Two Downs is CPU Character.
    The page comes from last_page. Three R from CPU misses it once the menu has been opened elsewhere.
    """
    open_menu(ctx)
    goto_page(ctx, 'fighters_stages')
    _jump_top(ctx)
    for _ in range(downs):
        press(ctx, 'down', 0.35)
    press(ctx, 'a', 0.8)
    time.sleep(0.5)
    if neighbor:
        press(ctx, 'right', 0.4)
    press(ctx, 'a', 0.6)
    close_menu(ctx)


def cpu_mario(ctx):
    """The Training CPU comes from the menu, not the CSS coin.

    One A on tile 0 readies this CSS while the coin is still Random, and a Random coin does not
    follow CPU Shield. The portrait grid opens on Mario. A selects it.
    """
    _pick_fighter(ctx, 2, False)
    return {'cpu': 'mario', 'route': 'menu CPU Character'}


def cpu_neighbor(ctx):
    """The fighter beside Mario, so a Mario recording is a different fighter."""
    _pick_fighter(ctx, 2, True)
    return {'cpu': 'neighbor', 'route': 'menu CPU Character right'}


def player_mario(ctx):
    _pick_fighter(ctx, 1, False)
    return {'player': 'mario'}


def player_neighbor(ctx):
    """P1 beside Mario, so a Mario recording mismatches Play on P1."""
    _pick_fighter(ctx, 1, True)
    return {'player': 'neighbor'}


def boot_training(ctx, release=False):
    """Bring this instance to Training with P1 on Mario.

    `release`: the release NRO writes no [Ui] press lines, so the CPU is left as the CSS set it.

    The CSS walk is L3 + D-pad, the route rc5-surface-1 used after input.stick failed, and the
    same walk XPS rc5-xps-04 settled on tile 0. enter-training is played from its file: the library
    segment's start gate is the animated ready banner, which does not settle. One more A dismisses
    a fresh profile's fighter tip, then the HUD gate is checked again.
    """
    status = ctx.call('status') or {}
    if status.get('running') and status.get('loading_complete'):
        try:
            ctx.gate('training-hud-unpaused')
        except RuntimeError:
            pass
        else:
            # Already in Training. A later miss inside cpu_mario must not fall through into the
            # main-menu route: that route's Start presses leave the match.
            # The release NRO writes no [Ui] press lines, so goto_page cannot see the page.
            if _ui_press_seen(ctx):
                return {'already': 'training', 'cpu': cpu_mario(ctx)}
            return {'already': 'training', 'cpu': 'left-as-set', 'ui_log': 'empty'}
    else:
        ctx.call('game.load', {'path': ctx.instance['game']}, job_cap=180)
        wait_loaded(ctx)
    # The plan's main gate carries "recover": "attract-cancel" with a short cap and several
    # recover_times (MACROS "Idle game"): Start leaves the movie for the title, then the title
    # or the demo for the main menu.
    ctx.gate('main')
    ctx.shot('main-menu')
    ctx.play('main-stage')
    ctx.gate('stage')
    ctx.play('stage-css')
    ctx.gate('css')
    picked = css_to_mario(ctx)
    css_chord(ctx, ['a'], 150)
    time.sleep(0.8)
    seg = ctx.runner.plan['segments']['enter-training']
    ctx.call('macro.play', {'path': seg['file']})
    ctx.runner.wait_macro(ctx.on, seg.get('cap', 40))
    tip = None
    for _ in range(4):
        try:
            ctx.gate('training-hud-unpaused')
            tip = 'hud'
            break
        except RuntimeError as error:
            tip = str(error)
            css_chord(ctx, ['a'], 120)
            time.sleep(1.2)
    if tip != 'hud':
        raise RuntimeError('fighter tip still up after four confirms: %s' % tip)
    cpu = 'left-as-set' if release else cpu_mario(ctx)
    return {'css_tile': '0', 'pick': picked, 'cpu': cpu}


def fresh_helper(ctx):
    """A fresh profile stores Combo helper On. Nobody opens Options."""
    path = Path(ctx.instance['settings'])
    if not path.is_file():
        raise RuntimeError('no settings file at %s' % path)
    data = json.loads(path.read_text())
    defaults = (data.get('defaults_menu') or {}).get('combo_helper')
    menu = (data.get('menu') or {}).get('combo_helper')
    if defaults != [1, 0] or menu not in (None, [1, 0]):
        raise RuntimeError('fresh combo_helper is not On [1, 0]: defaults=%s menu=%s' % (defaults, menu))
    return save(ctx, 'fresh-helper', {'defaults': defaults, 'menu': menu})


# --- xps-a: migration boot, runtime rows ----------------------------------------------------------

def rr_migrate(ctx):
    """The migration boot: fresh index, one live legacy slot; the worker tick migrates once."""
    boot_training(ctx)
    line = diag(ctx, r'screen=migrate once=1', cap=120)
    ctx.shot('migrate')
    out = save(ctx, 'migrate', {'line': line})
    if 'once=1' not in line:
        raise RuntimeError('migrate line did not say once=1: %s' % line)
    return out


def media_migration(ctx):
    """The migrated slot plays on the release NRO. The file name is the check, not a diag line."""
    folder = Path(ctx.instance['diag_log']).parent / 'macros'
    names = sorted(p.name for p in folder.glob('*')) if folder.is_dir() else []
    dated = [name for name in names if name.startswith('Slot 1')]
    if not dated:
        raise RuntimeError('no Slot 1 file in %s (found %s)' % (folder, names[:8]))
    def do():
        show_popup(ctx)
        press(ctx, 'r', 0.5)
        press(ctx, 'a', 0.6)
        popup_close(ctx)
        time.sleep(0.5)
        ctx.play(PLAY_CPU)
        time.sleep(2.5)
    out = gif(ctx, 'rc5-migration.gif', 8, do)
    ctx.shot('migration-after')
    return save(ctx, 'migration', {**out, 'slot_files': dated})


def rr_record(ctx):
    """Record follows the pop-up's role: ZR+Down starts on Player, again stops on CPU."""
    settle(ctx)
    popup(ctx)
    press(ctx, 'b', 0.6)              # close; the pop-up role stays Player
    before = time.time()
    ctx.play(RECORD)
    line = diag(ctx, r'screen=recording role=player recording=1', cap=10)
    # Record something small on camera (one jab), then move the pop-up role to CPU and stop.
    ctx.play(JAB)
    popup(ctx)
    press(ctx, 'r', 0.5)
    press(ctx, 'b', 0.6)
    ctx.play(RECORD)
    stop = diag(ctx, r'screen=recording role=cpu recording=0', cap=10)
    return save(ctx, 'record', {'start': line, 'stop': stop, 'seconds': round(time.time() - before, 1)})


def rr_record_cpu(ctx):
    """Record while possessing the CPU. The file must hold that input, and Play on CPU must start it."""
    settle(ctx)
    before = time.time()
    popup(ctx)
    press(ctx, 'r', 0.5)
    press(ctx, 'b', 0.6)
    ctx.play(RECORD)
    start = diag(ctx, r'screen=recording role=cpu recording=1', cap=10)
    ctx.play(JAB)
    ctx.play(RECORD)
    stop = diag(ctx, r'screen=recording role=cpu recording=0', cap=10)
    folder = Path(ctx.instance['diag_log']).parent / 'macros'
    written = [p for p in folder.glob('*.json') if p.stat().st_mtime >= before - 1] if folder.is_dir() else []
    if not written:
        raise RuntimeError('Record on CPU wrote no macro in %s' % folder)
    path = max(written, key=lambda item: item.stat().st_mtime)
    frames = json.loads(path.read_text()).get('frames') or []
    pressed = [frame.get('buttons') for frame in frames if frame.get('buttons') not in (None, 32768)]
    if not pressed:
        raise RuntimeError(
            'CPU-role file %s holds only idle buttons 32768; the jab made while possessing the CPU was not stored'
            % path.name)
    ctx.play(PLAY_CPU)
    time.sleep(1.5)
    shot = ctx.shot('cpu-playing')
    return save(ctx, 'record-cpu', {
        'start': start, 'stop': stop, 'file': path.name, 'pressed': pressed[:8], 'shot': str(shot),
    })


def _open_file_row(ctx, title, downs=0):
    """Open a macro file and stop on `title`. The caller presses A for that row.

    `downs` picks a later macro. A chain file has no Crop row. Close that file and open
    the next library item. B is safe here because the file screen is already open;
    B on the New-chain picker would create a chain instead.
    """
    offset, current = open_library_item(ctx, True, downs)
    last = current
    for _ in range(6):
        try:
            return down_to_file_row(ctx, offset, current, title)
        except RuntimeError:
            last = current
            press(ctx, 'b', 0.4)
            press(ctx, 'down', 0.35)
            offset = _gate_offset(ctx)
            press(ctx, 'a', 0.55)
            offset, current = _await_title(ctx, offset)
            if not current:
                break
    raise RuntimeError('file row %s not reached (last %s)' % (title, last))


def rr_make_combo(ctx):
    """Make combo plays the macro under the cursor and saves Combo N."""
    settle(ctx)
    _open_file_row(ctx, 'Make combo')
    press(ctx, 'a', 0.6)
    line = diag(ctx, r'screen=make-combo source=\S+', cap=15)
    close_menu(ctx)
    return save(ctx, 'make-combo', {'line': line})


def rr_make_combo_chain(ctx):
    """Make combo on a chain plays that chain from macro 1's start save."""
    settle(ctx)
    popup(ctx)
    press(ctx, 'down', 0.35)
    press(ctx, 'a', 0.6)
    popup_close(ctx)
    _open_file_row(ctx, 'Make combo', 1)
    press(ctx, 'a', 0.6)
    line = diag(ctx, r'screen=make-combo source=\S+', cap=15)
    close_menu(ctx)
    return save(ctx, 'make-combo-chain', {'line': line})


def rr_crop_macro(ctx):
    """Crop a macro: the handle screen opens, A writes a new macro, the original stays."""
    settle(ctx)
    _open_file_row(ctx, 'Crop')
    press(ctx, 'a', 0.6)
    opened = diag(ctx, r'screen=crop source=\S+ combo=0', cap=10)
    press(ctx, 'r', 0.4)
    press(ctx, 'a', 0.6)
    saved = diag(ctx, r'screen=crop source=\S+ combo=0 saved=1', cap=10)
    close_menu(ctx)
    return save(ctx, 'crop-macro', {'open': opened, 'saved': saved})


def rr_crop_combo(ctx):
    """Crop a combo by its links. A writes a new combo.

    Combos & Sequences is the section after the macro library. Right swaps that group onto combos.
    There is no combo until Make combo has saved one; an empty column does not invent a file.
    """
    settle(ctx)
    open_menu(ctx)
    goto_page(ctx, 'recording')
    _jump_top(ctx)
    # css_chord presses ZL and Down on different frames, so the menu saw a plain Down.
    # chord-save is ZL+Down on one frame: a section jump while the menu is open.
    ctx.play(SAVE)
    time.sleep(0.35)
    press(ctx, 'right', 0.4)
    press(ctx, 'a', 0.6)
    opened = diag(ctx, r'screen=crop source=\S+ combo=1', cap=10)
    press(ctx, 'a', 0.6)
    saved = diag(ctx, r'screen=crop source=\S+ combo=1 saved=1', cap=10)
    close_menu(ctx)
    return save(ctx, 'crop-combo', {'open': opened, 'saved': saved})


def rr_mismatch_macro(ctx):
    """Play a macro recorded on another fighter. Keep plays. B is not Keep.

    Both fighters are Mario until this row moves P1 one portrait to the right. The recorded file
    stays Mario, so Play on P1 is the mismatch. P1 is put back on Mario either way.
    """
    settle(ctx)
    player_neighbor(ctx)
    try:
        popup(ctx)
        press(ctx, 'a', 0.6)
        popup_close(ctx)
        ctx.play(PLAY_P1)
        line = diag(ctx, r'screen=mismatch ask=macro ', cap=12)
        ctx.shot('mismatch-macro')
        press(ctx, 'a', 0.6)
        return save(ctx, 'mismatch-macro', {'line': line})
    finally:
        player_mario(ctx)


def rr_mismatch_chain(ctx):
    """A chain asks once, for macro 1. Keep plays on the current fighter.

    The CPU moves off Mario first. A chain is not invented: if the active item is a single macro
    the line says ask=macro and this row's ask=macro1 pattern fails.
    """
    settle(ctx)
    cpu_neighbor(ctx)
    try:
        popup(ctx)
        press(ctx, 'r', 0.5)
        press(ctx, 'a', 0.6)
        popup_close(ctx)
        ctx.play(PLAY_CPU)
        line = diag(ctx, r'screen=mismatch ask=macro1', cap=12)
        ctx.shot('mismatch-chain')
        press(ctx, 'a', 0.6)
        return save(ctx, 'mismatch-chain', {'line': line})
    finally:
        cpu_mario(ctx)


def rr_visuals(ctx):
    """The hot five keep their outline token, and the cursor is the white bar."""
    settle(ctx)
    line = popup(ctx)
    if 'hairline=' not in line and 'hot=' not in line:
        line = diag(ctx, r'hot=\d+', cap=8)
    ctx.shot('visuals')
    press(ctx, 'b', 0.5)
    return save(ctx, 'visuals', {'line': line})


def rr_notice(ctx):
    """Two notices: chord-once (first Record or Play after boot) and macro-start-save-missing."""
    settle(ctx)
    # chord-once: the first chord after boot prints it once; the boot already used chords, so the
    # bounded probe records whether the line exists at all in this boot's diag (README open fact).
    ctx.play(JAB)
    time.sleep(0.5)
    chord_lines = []
    try:
        chord_lines.append(diag(ctx, r'screen=notice kind=chord-once', cap=4))
    except RuntimeError:
        pass
    # Delete removes the save file. Active on the macro file sets the player macro, and Play
    # then reports the missing start save. Delete is immediate; there is no confirm A.
    # The file is put back afterwards: this boot left macro-2 pointing at a missing macro-2-start.
    backup = _save_files(ctx)
    offset, current = open_library_item(ctx, False)
    down_to_file_row(ctx, offset, current, 'Delete')
    press(ctx, 'a', 0.6)
    close_menu(ctx)
    offset, current = open_library_item(ctx, True)
    down_to_file_row(ctx, offset, current, 'Set active')
    press(ctx, 'a', 0.6)
    close_menu(ctx)
    ctx.play(PLAY_P1)
    missing = diag(ctx, r'screen=notice kind=macro-start-save-missing', cap=10)
    restored = _restore_saves(ctx, backup)
    ctx.shot('notice')
    return save(ctx, 'notice', {'chord_once': chord_lines, 'missing': missing, 'restored': restored})


def rr_mismatch(ctx):
    """A chain whose macro 1 fighter differs: one prompt at the chain start naming macro 1.

    The CPU moves off Mario first. No chain is written here. A single macro prompts ask=macro,
    and this row keeps the ask=macro1 pattern.
    """
    settle(ctx)
    cpu_neighbor(ctx)
    try:
        popup(ctx)
        press(ctx, 'r', 0.5)              # CPU view
        press(ctx, 'a', 0.6)              # set the chain active for the CPU
        popup_close(ctx)
        time.sleep(0.5)
        ctx.play(PLAY_CPU)
        line = diag(ctx, r'screen=mismatch ask=macro1', cap=15)
        shot = ctx.shot('mismatch-prompt')
        press(ctx, 'b', 0.8)              # Keep current fighter (the GIF's choice)
        time.sleep(2.0)
        return save(ctx, 'mismatch', {'line': line, 'shot': str(shot)})
    finally:
        cpu_mario(ctx)


def rr_rename(ctx):
    """A on Rename opens the titled keyboard offer (Online off; with Online on it is not made)."""
    settle(ctx)
    _open_file_row(ctx, 'Rename')
    press(ctx, 'a', 0.6)
    line = diag(ctx, r'screen=rename header=Name macro', cap=10)
    ctx.shot('rename-offer')
    press(ctx, 'b', 0.8)              # leave the keyboard without renaming
    close_menu(ctx)
    return save(ctx, 'rename', {'line': line})


def rr_crop(ctx):
    """A on Crop: screen=crop source=<id> combo=0, then the new file write, originals unchanged."""
    settle(ctx)
    _open_file_row(ctx, 'Crop')
    press(ctx, 'a', 0.6)
    line = diag(ctx, r'screen=crop source=\S+ combo=0', cap=10)
    ctx.shot('crop')
    press(ctx, 'b', 0.8)
    close_menu(ctx)
    return save(ctx, 'crop', {'line': line})


def rr_load(ctx):
    """Save (ZL+Down) writes the bound save; Load (ZL+Up) warps both fighters and starts nothing."""
    settle(ctx)
    ctx.play(RESET_WALK)
    time.sleep(0.5)
    ctx.play(SAVE)
    time.sleep(1.0)
    ctx.play(RESET_WALK)              # move both fighters away from the saved spot
    time.sleep(0.5)
    before = ctx.shot('before-load')
    ctx.play(LOAD)
    time.sleep(1.5)
    after = ctx.shot('after-load')
    quiet = []
    try:
        quiet = diag(ctx, r'playback |screen=recording\b', cap=2)
    except RuntimeError:
        pass
    if quiet:
        raise RuntimeError('Load started something: %s' % quiet)
    return save(ctx, 'load', {'before': str(before), 'after': str(after)})


def rr_chain(ctx):
    """The chain warps once to macro 1's start save, plays 1 then 2, no warp between them."""
    settle(ctx)
    popup(ctx)
    press(ctx, 'r', 0.5)              # CPU view
    press(ctx, 'a', 0.6)              # set the chain active for the CPU
    popup_close(ctx)
    time.sleep(0.5)
    warps = []
    ctx.play(PLAY_CPU)
    warps.append(diag(ctx, r'playback .*warp=1', cap=15))
    time.sleep(3.0)
    try:
        extra = diag(ctx, r'playback .*warp=1', cap=4)
        warps.append(extra)
    except RuntimeError:
        pass
    if len(warps) != 1:
        raise RuntimeError('the chain warped %d times, expected 1' % len(warps))
    ctx.shot('chain-played')
    return save(ctx, 'chain', {'warps': warps})


def rr_loop(ctx):
    """X cycles the loop with the cursor in MACROS; pass 2 does not warp."""
    settle(ctx)
    popup(ctx)
    ctx.play(X)                       # X once with the cursor in MACROS: loop 1 -> 2
    line = diag(ctx, r'screen=recording-popup role=player loop=2', cap=8)
    press(ctx, 'r', 0.5)              # CPU view
    press(ctx, 'a', 0.6)              # set the looped macro active for the CPU
    popup_close(ctx)
    time.sleep(0.5)
    ctx.play(PLAY_CPU)
    first = diag(ctx, r'playback .*warp=1', cap=15)
    time.sleep(4.0)
    second = []
    try:
        second = diag(ctx, r'playback .*warp=1', cap=4)
    except RuntimeError:
        pass
    if second:
        raise RuntimeError('the second pass warped: %s' % second)
    ctx.shot('loop-pass-2')
    return save(ctx, 'loop', {'x': line, 'first_warp': first})


def rr_play(ctx):
    """RR-7 row rr-play, both cases: Play on Player 1 warps once; Play on CPU with no start save does not.

    Case 1 needs Player 1's active macro to have a start save with the switch on; case 2 needs the
    CPU's active macro to have no start save. Which macro sits under the pop-up cursor when the row
    runs is not in any source (README, Open questions): the hook sets the cursor's macro active for
    each role, waits for the exact diag line RR-7 gives, and raises with the captured data when the
    warp behaviour differs.
    """
    settle(ctx)

    def warp_count():
        path = Path(ctx.instance.get('diag_log') or ctx.instance['console_log'])
        if not path.exists():
            return 0
        return len(re.findall(r'playback .*warp=1', path.read_text(errors='replace')))

    # Case 1 (RR-7): Play on Player 1 ([7,10]), start save, switch on: warp=1, warps stay 1.
    popup(ctx)
    press(ctx, 'a', 0.6)              # A on the macro: set it active for Player 1
    popup_close(ctx)
    time.sleep(0.5)
    before = warp_count()
    ctx.play(PLAY_P1)
    player = diag(ctx, r'screen=playback role=player warp=1', cap=15)
    time.sleep(4.0)                   # the loop passes play out; the warps count must stay 1
    warps = warp_count() - before
    if warps != 1:
        raise RuntimeError('the play warped %d times across the loop, expected 1' % warps)
    ctx.shot('play-player')
    # Case 2 (RR-7): Play on CPU (the old Input Playback chord), no start save: warp=0, no warp.
    popup(ctx)
    press(ctx, 'r', 0.5)              # CPU view
    press(ctx, 'a', 0.6)              # set the CPU's active macro (no start save, Open questions)
    popup_close(ctx)
    time.sleep(0.5)
    ctx.play(PLAY_CPU)
    cpu = diag(ctx, r'screen=playback role=cpu warp=0', cap=15)
    ctx.shot('play-cpu-no-warp')
    return save(ctx, 'play', {'player': player, 'warps': warps, 'cpu': cpu})


def rr_mash(ctx):
    """RR-7 row rr-mash: a role is playing with the Mash interrupt on; the mash stops the playback.

    "The mash that already calls interrupt_playback" is the CPU's own mash (the menu's Mash row);
    the exact page route is not in any source (README, Open questions), so the hook sets a mash
    action bounded, plays the CPU's macro, puts the CPU into hitstun so its mash trigger fires, and
    waits for screen=playback mash=1 (playing false after it).
    """
    settle(ctx)
    open_menu(ctx)
    for _ in range(6):
        press(ctx, 'r', 0.6)
    press(ctx, 'a', 0.8)              # the CPU page's Mash row (README, Open questions)
    press(ctx, 'a', 0.8)              # set a mash action
    close_menu(ctx)
    popup(ctx)
    press(ctx, 'r', 0.5)              # CPU view
    press(ctx, 'a', 0.6)              # set the macro active for the CPU
    popup_close(ctx)
    time.sleep(0.5)
    ctx.play(PLAY_CPU)
    time.sleep(1.0)
    ctx.play(JAB)                     # the CPU enters hitstun; its mash trigger fires
    line = diag(ctx, r'screen=playback mash=1', cap=15)
    ctx.shot('mash-interrupt')
    return save(ctx, 'mash', {'line': line})


def rr_combo(ctx):
    """RR-7 row rr-combo: the Connections panel lists each link's events, then the drift line.

    The panel line (jump · release · down flick · A, then drift ->) is the published frame, not a
    diag print (RR-7), so the evidence is the panel screenshot. The combo whose links store events,
    including a down flick and a forward drift, is the session's fixture (README, Open questions).
    """
    settle(ctx)
    open_menu(ctx)
    for _ in range(6):
        press(ctx, 'r', 0.6)
    for _ in range(3):
        press(ctx, 'down', 0.4)       # to the Combos & Sequences group
    press(ctx, 'a', 0.8)              # open the combo
    time.sleep(0.8)
    shot = ctx.shot('combo-panel')
    close_menu(ctx)
    return save(ctx, 'combo', {'shot': str(shot)})


# --- xps-b: the menu rows ------------------------------------------------------------------------

def rr_chords(ctx):
    """The eight factory chords keep today's defaults (the Shortcuts page shows the table)."""
    settle(ctx)
    open_menu(ctx)
    shot = ctx.shot('shortcuts')
    close_menu(ctx)
    # Each chord recipe exists and plays (recorded from its recipe on the first play).
    played = {}
    for name, key in ((SAVE, 'ZL+Down [6,9]'), (LOAD, 'ZL+Up [6,8]'), (RECORD, 'ZR+Down [7,9]'),
                      (PLAY_CPU, 'ZR+Up [7,8]'), (PLAY_P1, 'ZR+Left [7,10]'),
                      (POPUP, 'ZR+Right [7,11]')):
        ctx.play(name)
        time.sleep(0.6)
        played[key] = 'played'
    popup_close(ctx)
    if any(value != 'played' for value in played.values()):
        raise RuntimeError('a factory chord did not play: %s' % played)
    return save(ctx, 'chords', {'shortcuts_shot': str(shot), 'played': played})


def rr_picker(ctx):
    """The numbered picker matches Character Cycles: A, R, A-on-numbered, R-on-last, B."""
    settle(ctx)
    open_menu(ctx)
    for _ in range(6):
        press(ctx, 'r', 0.6)
    press(ctx, 'a', 0.8)              # open a chain's Edit chain: the numbered picker
    ctx.shot('picker-open')
    press(ctx, 'a', 0.6)              # A on a tile: next number
    ctx.shot('picker-numbered')
    press(ctx, 'r', 0.6)              # R removes; later numbers close up
    ctx.shot('picker-removed')
    press(ctx, 'b', 0.6)              # B is done
    close_menu(ctx)
    return save(ctx, 'picker', {'shots': 'picker-*.png'})


def rr_hot(ctx):
    """Six or more macros: hot=5, hairline after the fifth."""
    settle(ctx)
    popup(ctx)
    line = diag(ctx, r'screen=recording-popup hot=5 hairline=5', cap=10)
    shot = ctx.shot('hot-five')
    popup_close(ctx)
    return save(ctx, 'hot', {'line': line, 'shot': str(shot)})


def rr_icons(ctx):
    """The floppy is grey with the start-save switch Off; the header floppy stays orange."""
    settle(ctx)
    popup(ctx)
    line = diag(ctx, r'screen=recording-popup floppy=off header=on', cap=10)
    shot = ctx.shot('icons')
    popup_close(ctx)
    return save(ctx, 'icons', {'line': line, 'shot': str(shot)})


def rr_page(ctx):
    """ZR switches the Recording page's role; L and R still change the menu page."""
    settle(ctx)
    open_menu(ctx)
    for _ in range(6):
        press(ctx, 'r', 0.6)
    line = diag(ctx, r'screen=recording-page role=player', cap=10)
    press(ctx, 'zr', 0.6)             # ZR once: Player -> CPU
    cpu = diag(ctx, r'screen=recording-page role=cpu group=saves-macros', cap=10)
    ctx.shot('page-cpu')
    press(ctx, 'l', 0.6)              # L still changes the menu page
    press(ctx, 'b', 0.6)              # B closes the menu
    close_menu(ctx)
    return save(ctx, 'page', {'player': line, 'cpu': cpu})


def rr_popup(ctx):
    """ZR+Right opens the pop-up with the menu closed; B returns to the match."""
    settle(ctx)
    before = ctx.shot('before-popup')
    line = popup(ctx)
    shot = ctx.shot('popup-open')
    popup_close(ctx)
    after = ctx.shot('after-popup')
    return save(ctx, 'popup', {'line': line, 'open': str(shot), 'before': str(before),
                               'after': str(after)})


def rr_active(ctx):
    """L/R set the pop-up role; each view shows that role's active macro."""
    settle(ctx)
    popup(ctx)
    press(ctx, 'r', 0.6)              # L or R to CPU
    line = diag(ctx, r'screen=recording-popup role=cpu active=\S+', cap=10)
    shot = ctx.shot('active-cpu')
    popup_close(ctx)
    return save(ctx, 'active', {'line': line, 'shot': str(shot)})


# --- ally-a: the recording-feature GIFs ----------------------------------------------------------

def media_record_start_save(ctx):
    """Pop-up on Player, ZR+Down, walk forward and jab once, ZR+Down again, open the pop-up."""
    def do():
        show_popup(ctx)
        press(ctx, 'b', 0.6)          # pop-up role Player
        ctx.play(RECORD)
        ctx.call('input.stick', {'stick': 'left', 'x': 1, 'y': 0, 'lease_ms': 900})
        time.sleep(0.9)
        ctx.call('input.stick', {'stick': 'left', 'x': 0, 'y': 0, 'lease_ms': 100})
        ctx.play(JAB)
        time.sleep(0.8)
        ctx.play(RECORD)              # stop
        time.sleep(0.6)
        show_popup(ctx)                    # open the pop-up: the new macro at the top of the hot five
        time.sleep(0.8)
        popup_close(ctx)
    out = gif(ctx, 'rc5-record-start-save.gif', 8, do)
    return save(ctx, 'record-start-save', out)


def media_play_chord(ctx):
    """Set the macro active for the CPU, stand away from its start save, ZR+Up."""
    def do():
        show_popup(ctx)
        press(ctx, 'r', 0.5)
        press(ctx, 'a', 0.6)          # set active for the CPU
        popup_close(ctx)
        ctx.play(RESET_WALK)          # stand away from the start save
        time.sleep(0.4)
        ctx.play(PLAY_CPU)
        time.sleep(3.5)
        ctx.play(PLAY_CPU)            # a second press stops it
        time.sleep(1.0)
    out = gif(ctx, 'rc5-play-chord.gif', 8, do)
    return save(ctx, 'play-chord', out)


def media_loop_no_warp(ctx):
    """Loop 2 (X while the cursor is in MACROS); the second pass does not warp."""
    def do():
        show_popup(ctx)
        ctx.play(X)
        press(ctx, 'r', 0.5)
        press(ctx, 'a', 0.6)
        popup_close(ctx)
        ctx.play(PLAY_CPU)
        time.sleep(6.5)
    out = gif(ctx, 'rc5-loop-no-warp.gif', 8, do)
    return save(ctx, 'loop-no-warp', out)


def media_chain_play(ctx):
    """A chain of two recordings, the first with a start save; one warp, then both, no gap warp."""
    def do():
        show_popup(ctx)
        press(ctx, 'r', 0.5)
        press(ctx, 'a', 0.6)
        popup_close(ctx)
        ctx.play(PLAY_CPU)
        time.sleep(7.0)
    out = gif(ctx, 'rc5-chain-play.gif', 9, do)
    return save(ctx, 'chain-play', out)


def media_popup(ctx):
    """ZR+Right opens the pop-up over the match; Down onto a macro; B closes."""
    def do():
        show_popup(ctx)
        time.sleep(1.0)
        press(ctx, 'down', 0.5)       # down onto a macro
        time.sleep(0.6)
        press(ctx, 'b', 0.6)          # B closes, the bound save unchanged
        time.sleep(0.8)
    out = gif(ctx, 'rc5-popup.gif', 8, do)
    return save(ctx, 'popup-gif', out)


def media_recording_page(ctx):
    """Open the menu, go to Recording. ZR once, then ZR back."""
    def do():
        open_menu(ctx)
        for _ in range(6):
            press(ctx, 'r', 0.6)
        time.sleep(0.8)
        press(ctx, 'zr', 0.8)         # Player -> CPU
        time.sleep(1.0)
        press(ctx, 'zr', 0.8)         # back
        time.sleep(1.0)
        close_menu(ctx)
    out = gif(ctx, 'rc5-recording-page.gif', 8, do)
    return save(ctx, 'recording-page', out)


def media_macro_screen(ctx):
    """On the Recording page, A on a macro that has a start save."""
    def do():
        open_menu(ctx)
        for _ in range(6):
            press(ctx, 'r', 0.6)
        press(ctx, 'a', 0.8)          # A on the macro: its own screen
        time.sleep(1.2)
        close_menu(ctx)
    out = gif(ctx, 'rc5-macro-screen.gif', 6, do)
    return save(ctx, 'macro-screen', out)


def media_fighter_mismatch(ctx):
    """Play the mismatched chain on the CPU; Keep keeps the current fighter; no second prompt."""
    def do():
        show_popup(ctx)
        press(ctx, 'r', 0.5)
        press(ctx, 'a', 0.6)
        popup_close(ctx)
        ctx.play(PLAY_CPU)
        time.sleep(1.2)
        press(ctx, 'b', 0.8)          # Keep current fighter
        time.sleep(4.0)
    out = gif(ctx, 'rc5-fighter-mismatch.gif', 7, do)
    return save(ctx, 'fighter-mismatch', out)


def media_make_combo(ctx):
    """Drill some other combo first; on the macro's screen, Make combo."""
    def do():
        open_menu(ctx)
        for _ in range(6):
            press(ctx, 'r', 0.6)
        press(ctx, 'a', 0.8)          # the macro's screen
        for _ in range(12):
            press(ctx, 'down', 0.4)
        press(ctx, 'a', 0.8)          # Make combo: plays once on Player 1 from its start save
        time.sleep(4.0)
        close_menu(ctx)
    out = gif(ctx, 'rc5-make-combo.gif', 8, do)
    return save(ctx, 'make-combo', out)


def media_crop_combo(ctx):
    """Open a combo of at least two links. Crop. L and R set the range; A saves."""
    def do():
        open_menu(ctx)
        for _ in range(6):
            press(ctx, 'r', 0.6)
        for _ in range(3):
            press(ctx, 'down', 0.4)   # to the Combos & Sequences group
        press(ctx, 'a', 0.8)          # open a combo
        for _ in range(12):
            press(ctx, 'down', 0.4)
        press(ctx, 'a', 0.8)          # Crop
        time.sleep(0.8)
        press(ctx, 'a', 0.8)          # A saves the kept links
        time.sleep(1.5)
        close_menu(ctx)
    out = gif(ctx, 'rc5-crop-combo.gif', 7, do)
    return save(ctx, 'crop-combo', out)


def media_save_warp(ctx):
    """ZL+Down at centre, walk both to the ledge, ZL+Up: both return, nothing plays."""
    log = Path(ctx.instance['diag_log'])
    start = log.stat().st_size if log.exists() else 0
    def do():
        ctx.play(SAVE)
        time.sleep(1.0)
        ctx.play(RESET_WALK)
        time.sleep(0.4)
        ctx.play(LOAD)
        time.sleep(2.0)
    out = gif(ctx, 'rc5-save-warp.gif', 6, do)
    fresh = log.read_text(errors='replace')[start:] if log.exists() else ''
    if 'screen=playback' in fresh or 'screen=recording role=' in fresh:
        raise RuntimeError('Load started a recording or a playback: %s' % fresh[-400:])
    out['screen'] = 'load started nothing'
    return save(ctx, 'save-warp', out)


# --- surface-a: the timed strip rows, the advantage cases, the graded GIFs -----------------------

def reset(ctx):
    """Spawn-point reset. reset-walk's 120-frame hold made the strip show a dash before the row started."""
    center(ctx)


def strip_shot(ctx, name):
    ctx.shot(name)
    lines = []
    try:
        lines = [diag(ctx, r'\[FrameStrip\] ', cap=2)]
    except RuntimeError:
        pass
    return lines


def strip_walk(ctx):
    """Hold one direction through the walk, then neutral: the row shows the held frames."""
    reset(ctx)
    before = strip_shot(ctx, 'walk-before')
    ctx.play('walk-right')
    time.sleep(0.8)
    after = strip_shot(ctx, 'walk-after')
    return save(ctx, 'strip-walk', {'before': before, 'after': after})


def strip_dash_dance(ctx):
    """Alternate left and right without pausing: the row alternates, the strip does not freeze."""
    reset(ctx)
    ctx.play('dash-dance')
    time.sleep(1.5)
    return save(ctx, 'strip-dash-dance', {'shots': [str(ctx.shot('dash-dance'))]})


def strip_cstick_smash(ctx):
    """Smash with the C-stick while the left stick is tilted another way.

    The runner's recipe actions drive the left stick only, so the C-stick is pressed here directly;
    the call's answer is captured (README, Open questions).
    """
    reset(ctx)
    ctx.call('input.stick', {'stick': 'left', 'x': 0, 'y': 0.6, 'lease_ms': 1200})
    time.sleep(0.3)
    try:
        ctx.call('input.stick', {'stick': 'right', 'x': 1, 'y': 0, 'lease_ms': 300})
        time.sleep(1.2)
        ctx.call('input.stick', {'stick': 'right', 'x': 0, 'y': 0, 'lease_ms': 100})
    except RuntimeError as e:
        return save(ctx, 'strip-cstick-smash', {'right_stick': 'refused: %s' % e,
                                                'shot': str(ctx.shot('cstick-refused'))})
    ctx.call('input.stick', {'stick': 'left', 'x': 0, 'y': 0, 'lease_ms': 100})
    return save(ctx, 'strip-cstick-smash', {'right_stick': 'accepted', 'shots': [str(ctx.shot('cstick'))]})


def strip_tilted_a(ctx):
    """A with the left stick tilted, right stick held another way, no C-stick."""
    reset(ctx)
    ctx.call('input.stick', {'stick': 'right', 'x': 0, 'y': 0.8, 'lease_ms': 1200})
    time.sleep(0.3)
    ctx.play('tilted-a')
    time.sleep(1.5)
    ctx.call('input.stick', {'stick': 'right', 'x': 0, 'y': 0, 'lease_ms': 100})
    return save(ctx, 'strip-tilted-a', {'shots': [str(ctx.shot('tilted-a'))]})


# shield_state dropdown index. Factory is None (factory.rs). Options: None, Infinite, Hold, Constant
# (menu/options.rs SHIELD). Right cycles forward and wraps (menu/mod.rs cycle).
_SHIELD = {'mode': 'none'}
_SHIELD_INDEX = {'none': 0, 'infinite': 1, 'hold': 2, 'constant': 3}


def cpu_shield(ctx, mode):
    """Set CPU Shield from the menu. docs/MEDIA.md, the advantage-shield row.

    The CPU page opens on Reactions. Groups are Reactions, Defence, Ledge & Tech, Shield
    (menu/table.rs GROUPS). chord-save is ZL+Down on one frame, which is a section jump
    while this menu is open. Sequential presses are not: the confirming boot logged a plain
    Down and opened the Mash checklist, and shield_state stayed None. The jump stops when
    the UI line names the Shield row. Right cycles the dropdown. A would open the list, so
    it is not pressed. One B closes, and the row reads shield_state back from settings.json.
    """
    target = _SHIELD_INDEX[mode]
    # The file is the last committed value. The in-memory mode was left at None on the
    # confirming boot while this route opened the Mash checklist instead of Shield.
    current = _shield_index(ctx)
    open_menu(ctx)
    goto_page(ctx, 'cpu')
    last = None
    for _ in range(8):
        last = _last_ui(ctx)
        if (last and last.group('page') == 'cpu' and last.group('depth') == '0'
                and last.group('row') == 'Shield'):
            break
        if last and last.group('depth') != '0':
            press(ctx, 'b', 0.3)
            time.sleep(0.3)
            continue
        ctx.play(SAVE)
        time.sleep(0.4)
    else:
        seen = last.groupdict() if last else None
        raise RuntimeError('CPU menu never showed the Shield row: %s' % seen)
    delta = (target - current) % 4
    if delta <= 2:
        button, times = 'right', delta
    else:
        button, times = 'left', 4 - delta
    for _ in range(times):
        press(ctx, button, 0.35)
    press(ctx, 'b', 0.7)
    settle(ctx)
    found = current
    deadline = time.time() + 2.0
    while time.time() < deadline:
        found = _shield_index(ctx)
        if found == target:
            break
        time.sleep(0.2)
    if found != target:
        raise RuntimeError('shield_state index %s, wanted %s (%s)' % (found, target, mode))
    _SHIELD['mode'] = mode
    return {'shield_state': mode, 'shield_index': found,
            'right_presses': times if button == 'right' else 0}


def center(ctx):
    """Training reset to the spawn point, without the 120-frame walk that reset-walk adds."""
    ctx.play(RESET_CENTER)
    time.sleep(0.4)
    settle(ctx)


def _fighter_gap(path):
    """Pixels of empty stage between the two red bodies, or None when the shot has no pair.

    The confirming jab-shield shot is 1280x674 and the two shirts sit at x 293-440 and
    845-998, 405 px apart, which is why the jab whiffed. Portraits sit below this band.
    """
    from PIL import Image
    im = Image.open(path).convert('RGB')
    w, h = im.size
    px = im.load()
    y0, y1 = int(h * 0.28), int(h * 0.72)
    cols = []
    for x in range(w):
        red = 0
        for y in range(y0, y1, 2):
            r, g, b = px[x, y]
            if r > 140 and r > g + 40 and r > b + 40:
                red += 1
        cols.append(red)
    runs = []
    start = None
    for x, n in enumerate(cols):
        if n > 4:
            if start is None:
                start = x
        elif start is not None:
            if x - start > 20:
                runs.append((start, x - 1))
            start = None
    if start is not None and w - start > 20:
        runs.append((start, w - 1))
    if len(runs) < 2:
        return 0 if runs else None
    return runs[-1][0] - runs[0][1]


def _close_for_jab(ctx):
    """Walk P1 toward the other body until a jab can reach, then stop.

    Spawn leaves the pair hundreds of pixels apart. A full reset-walk paints a dash on the
    strip, so this holds a walk stick and remeasures. A step that opens the gap reverses.
    """
    direction = 1
    last = None
    gap = None
    for _ in range(8):
        gap = _fighter_gap(ctx.shot('gap'))
        if gap is None or gap <= 90:
            return gap
        if last is not None and gap > last + 20:
            direction *= -1
        last = gap
        ctx.call('input.stick', {'stick': 'left', 'x': 0.5 * direction, 'y': 0, 'lease_ms': 220})
        time.sleep(0.28)
        ctx.call('input.stick', {'stick': 'left', 'x': 0, 'y': 0, 'lease_ms': 100})
        time.sleep(0.2)
    gap = _fighter_gap(ctx.shot('gap'))
    if gap is None or gap <= 90:
        return gap
    raise RuntimeError('fighters still %s px apart; the move was not thrown' % gap)


def advantage(ctx, move_segment, shield):
    """One advantage attempt. The diag line is [FrameStrip] advantage value=N on_shield=true|false.

    The published page is still not in a named source. The CPU shields only for the on-shield rows.
    The on-hit row sets Shield back to None and does not enable mash.
    """
    shield_set = cpu_shield(ctx, 'hold' if shield else 'none')
    center(ctx)
    gap = _close_for_jab(ctx)
    ctx.play(move_segment)
    time.sleep(2.0)
    want = 'true' if shield else 'false'
    line = diag(ctx, r'\[FrameStrip\] advantage value=-?\d+ on_shield=%s' % want, cap=10)
    shot = ctx.shot('advantage')
    return save(ctx, 'advantage', {'line': line, 'shot': str(shot), 'shield': shield_set,
                                   'gap_px': gap, 'published': 'to be filled by the session'})


def adv_jab_shield(ctx):
    return advantage(ctx, JAB, True)


def adv_tilt_shield(ctx):
    return advantage(ctx, 'tilted-a', True)


def adv_smash_shield(ctx):
    return advantage(ctx, FSMASH, True)


def adv_aerial_shield(ctx):
    return advantage(ctx, 'nair-loop-clean', True)


def adv_move_hit(ctx):
    return advantage(ctx, FSMASH, False)


def media_grade_not_true(ctx):
    """A true jab-jab, then a late jab that still counts, then a forward tilt that drops.

    The clip is the bar. The release NRO does not print the verdict; rr-grade-bar traces it.
    """
    reset(ctx)
    def do():
        ctx.play('jab-link')          # the true jab > jab
        time.sleep(0.6)
        ctx.play(SAVE)
        time.sleep(0.4)
        ctx.play(LOAD)
        time.sleep(0.4)
        ctx.play('jab-drop')          # late, still a jab
        time.sleep(0.5)
        ctx.play(LOAD)
        time.sleep(0.4)
        ctx.play('tilted-a')          # the follow-up is a tilt
        time.sleep(0.8)
    out = gif(ctx, 'rc5-grade-not-true.gif', 9, do)
    return save(ctx, 'grade-not-true', out)


def media_sequence_drill(ctx):
    """Land the first short combo, then fail the second with a tilt. The bar is the clip."""
    reset(ctx)
    def do():
        ctx.play('jab-link')
        time.sleep(0.6)
        ctx.play('tilted-a')
        time.sleep(1.0)
    out = gif(ctx, 'rc5-sequence-drill.gif', 9, do)
    return save(ctx, 'sequence-drill', out)


# --- surface-b: the frame-strip GIFs -------------------------------------------------------------

def media_strip_lanes(ctx):
    """Dash, then a forward smash into the CPU, then hold still. The colours are the lanes."""
    reset(ctx)
    def do():
        ctx.play('lanes-smash')
        time.sleep(1.5)
    return save(ctx, 'strip-lanes', gif(ctx, 'rc5-frame-strip-lanes.gif', 6, do))


def media_strip_chips(ctx):
    """Up-right held through one A. The first pass pressed A after the stick lease, so no chip."""
    reset(ctx)
    def do():
        ctx.play('chips-attack')
        time.sleep(1.2)
    return save(ctx, 'strip-chips', gif(ctx, 'rc5-frame-strip-chips.gif', 4, do))


def media_strip_freeze(ctx):
    """Jab, 90 quiet frames, jab again. The lanes stop, then move."""
    reset(ctx)
    def do():
        ctx.play('freeze-jabs')
        time.sleep(0.6)
    return save(ctx, 'strip-freeze', gif(ctx, 'rc5-frame-strip-freeze.gif', 5, do))


def media_strip_advantage_hit(ctx):
    """One jab, then stand. The readout is on screen on the release NRO; the line is the diag row."""
    cpu_shield(ctx, 'none')
    reset(ctx)
    def do():
        ctx.play(JAB)
        time.sleep(2.5)
    return save(ctx, 'strip-advantage-hit', gif(ctx, 'rc5-frame-strip-advantage-hit.gif', 5, do))


def media_strip_advantage_shield(ctx):
    """Shield Hold is set before the clip, so the 6 s show the jab and the on-shield readout."""
    cpu_shield(ctx, 'hold')
    reset(ctx)
    def do():
        ctx.play(JAB)
        time.sleep(3.0)
    out = gif(ctx, 'rc5-frame-strip-advantage-shield.gif', 6, do)
    cpu_shield(ctx, 'none')
    return save(ctx, 'strip-advantage-shield', out)


def media_strip_link(ctx):
    """Second jab inside the cancel window, so LINK can read TRUE combo."""
    reset(ctx)
    def do():
        ctx.play('jab-link')
        time.sleep(1.2)
    return save(ctx, 'strip-link', gif(ctx, 'rc5-frame-strip-link.gif', 5, do))


def media_strip_drop(ctx):
    """Second jab after the CPU can act, inside 20 frames, so LINK can read DROP."""
    reset(ctx)
    def do():
        ctx.play('jab-drop')
        time.sleep(1.2)
    return save(ctx, 'strip-drop', gif(ctx, 'rc5-frame-strip-drop.gif', 6, do))


def media_strip_input(ctx):
    """The same late jab as the drop clip. INPUT is the late press, visible on the strip."""
    reset(ctx)
    def do():
        ctx.play('jab-drop')
        time.sleep(1.5)
    return save(ctx, 'strip-input', gif(ctx, 'rc5-frame-strip-input.gif', 6, do))


def media_strip_last(ctx):
    """Three jabs as two links, then stand, so LAST has pills. A 0.5 s gap left LAST empty."""
    reset(ctx)
    def do():
        ctx.play('jab-link-3')
        time.sleep(1.5)
    return save(ctx, 'strip-last', gif(ctx, 'rc5-frame-strip-last.gif', 7, do))


def media_strip_exchange(ctx):
    """90 quiet frames between jabs, past the 20-frame exchange, so the second hit is a new one."""
    reset(ctx)
    def do():
        ctx.play('exchange-jabs')
        time.sleep(1.2)
    return save(ctx, 'strip-exchange', gif(ctx, 'rc5-frame-strip-exchange.gif', 8, do))


# --- yoga-a: the rr-11 stick-row GIFs and the combo rows ------------------------------------------------

def media_strip_cstick(ctx):
    """Left stick neutral; C-stick right, one forward smash; no A (right stick pressed directly;
    the recipe actions cannot express it — README, Open questions)."""
    reset(ctx)
    def do():
        try:
            ctx.call('input.stick', {'stick': 'right', 'x': 1, 'y': 0, 'lease_ms': 300})
            time.sleep(1.2)
            ctx.call('input.stick', {'stick': 'right', 'x': 0, 'y': 0, 'lease_ms': 100})
        except RuntimeError as e:
            raise RuntimeError('the right-stick call was refused (the C-stick needs another route): %s' % e)
        time.sleep(1.0)
    out = gif(ctx, 'rc5-frame-strip-cstick.gif', 4, do)
    return save(ctx, 'strip-cstick', out)


def media_strip_stick_row(ctx):
    """Hold left, neutral, down-right, then dash-dance about two seconds. No buttons."""
    reset(ctx)
    def do():
        ctx.call('input.stick', {'stick': 'left', 'x': -1, 'y': 0, 'lease_ms': 500})
        ctx.call('input.stick', {'stick': 'left', 'x': 0, 'y': 0, 'lease_ms': 250})
        ctx.call('input.stick', {'stick': 'left', 'x': 0.7, 'y': -0.7, 'lease_ms': 400})
        ctx.call('input.stick', {'stick': 'left', 'x': 0, 'y': 0, 'lease_ms': 100})
        ctx.play('dash-dance')
        time.sleep(1.0)
    out = gif(ctx, 'rc5-frame-strip-stick-row.gif', 6, do)
    return save(ctx, 'strip-stick-row', out)


def media_strip_freeze_stick(ctx):
    """After a jab, both can act: stick left then right, about two seconds; the strip keeps scrolling."""
    reset(ctx)
    def do():
        ctx.play(JAB)
        time.sleep(0.8)
        ctx.call('input.stick', {'stick': 'left', 'x': -1, 'y': 0, 'lease_ms': 500})
        ctx.call('input.stick', {'stick': 'left', 'x': 1, 'y': 0, 'lease_ms': 500})
        ctx.call('input.stick', {'stick': 'left', 'x': 0, 'y': 0, 'lease_ms': 100})
        time.sleep(0.8)
    out = gif(ctx, 'rc5-frame-strip-freeze-stick.gif', 5, do)
    return save(ctx, 'strip-freeze-stick', out)


def combo(ctx, name, expect_pattern):
    """Play the combo. The release gate is tr_combo_link_v1's return, not a drill print.

    expect_pattern is kept for the row's note. The trace line is the pass. Flags are in x2 when
    the re-run arms the entry registers: bit 1 clean, bit 2 wrong move, bit 3 a missing event.
    """
    reset(ctx)
    ctx.play(name)
    time.sleep(2.0)
    line = ctx.wait_log(
        'trace_jsonl',
        r'(?:"event":"return".*tr_combo_link_v1|tr_combo_link_v1.*"event":"return")',
        cap=12)
    shot = ctx.shot('bar')
    return save(ctx, name, {'line': line, 'shot': str(shot), 'case': expect_pattern})


def combo_nair_loop_clean(ctx):
    return combo(ctx, 'nair-loop-clean', r'drill .*(clean|1 of 1)')


def combo_nair_loop_full_hop(ctx):
    return combo(ctx, 'nair-loop-full-hop', r'drill .*(drop|late)')


def combo_nair_loop_no_fastfall(ctx):
    return combo(ctx, 'nair-loop-no-fastfall', r'drill .*(drop|missing)')


def combo_dash_attack(ctx):
    return combo(ctx, 'dash-attack', r'drill .*(clean|dash)')


# --- yoga-b: the grade-bar and drill rows --------------------------------------------------------

def rr_grade_bar(ctx):
    """Four cases. Each one must return from tr_combo_attempt_v1. A screenshot is not the gate."""
    out = {'cases': []}
    reset(ctx)
    ctx.play('jab-drop')              # a non-TRUE jab link inside the window
    time.sleep(1.0)
    out['cases'].append({'case': 'non-true link', 'shot': str(ctx.shot('grade-clean'))})
    ctx.play(JAB)                     # the follow-up never comes
    time.sleep(2.2)
    out['cases'].append({'case': 'missing press', 'shot': str(ctx.shot('grade-missing'))})
    ctx.play('jab-link')
    time.sleep(0.4)
    ctx.play('exchange-jabs')         # the gap past 20 frames ends the exchange
    time.sleep(0.8)
    out['cases'].append({'case': 'exchange gap', 'shot': str(ctx.shot('grade-gap'))})
    ctx.play('tilted-a')              # a different move drops and restarts at combo 1
    time.sleep(1.2)
    line = ctx.wait_log(
        'trace_jsonl',
        r'(?:"event":"return".*tr_combo_attempt_v1|tr_combo_attempt_v1.*"event":"return")',
        cap=12)
    out['cases'].append({'case': 'wrong move drop', 'shot': str(ctx.shot('grade-drop')), 'line': line})
    settle(ctx)
    return save(ctx, 'grade-bar', out)


def rr_drill(ctx):
    """A on Drill replaces the drill. Make combo must not replace it again.

    The diag UI line names the row under the cursor when A is pressed. The first pass passed
    this row with no line and no check.
    """
    settle(ctx)
    open_menu(ctx)
    for _ in range(8):
        press(ctx, 'r', 0.45)
    for _ in range(8):
        press(ctx, 'down', 0.3)
    press(ctx, 'a', 0.6)
    drill_line = diag(ctx, r'\[Ui\] press .*Drill', cap=8)
    time.sleep(0.4)
    ctx.play('jab')
    time.sleep(0.6)
    shot1 = ctx.shot('drill-combo')
    for _ in range(8):
        press(ctx, 'down', 0.3)
    press(ctx, 'a', 0.6)
    make_line = diag(ctx, r'\[Ui\] press .*Make combo', cap=8)
    time.sleep(1.5)
    shot2 = ctx.shot('after-make-combo')
    close_menu(ctx)
    if 'Drill' not in drill_line or 'Make combo' not in make_line:
        raise RuntimeError('drill presses were not the Drill and Make combo rows: %s / %s'
                           % (drill_line, make_line))
    return save(ctx, 'drill', {'drill': drill_line, 'make_combo': make_line,
                               'drill_shot': str(shot1), 'after_make_combo': str(shot2)})


def nro_build_id(path):
    """Build id Eden puts on `nro:<id>` for a note-less linkle ELF.

    SHA-256 of the page-padded PT_LOAD image (flags 5, 4, 6, image base 0), not the file hash.
    Matches core/debugger/trace_modules.cpp.
    """
    data = Path(path).read_bytes()
    if len(data) < 64 or data[:4] != b'\x7fELF' or data[4] != 2 or data[5] != 1:
        raise RuntimeError('not a little-endian ELF64: %s' % path)
    phoff, = struct.unpack_from('<Q', data, 32)
    phentsize, phnum = struct.unpack_from('<HH', data, 54)
    if phentsize != 56 or phnum < 3:
        raise RuntimeError('ELF program headers are not the linkle layout: %s' % path)

    def read(at, size):
        if at + size > len(data):
            raise RuntimeError('ELF field past end: %s' % path)
        return int.from_bytes(data[at:at + size], 'little')

    offsets, sizes, padded = [], [], []
    total = 0
    for i in range(3):
        ph = phoff + i * 56
        if read(ph, 4) != 1 or read(ph + 4, 4) != (5, 4, 6)[i] or read(ph + 16, 8) != total:
            raise RuntimeError('ELF load %s is not RX/R/RW at 0: %s' % (i, path))
        offsets.append(read(ph + 8, 8))
        sizes.append(read(ph + 32, 8))
        pad = (sizes[-1] + 0xfff) & ~0xfff
        if i < 2 or (phnum > 3 and read(ph + 56, 4) == 1):
            nxt = read(ph + 56 + 16, 8)
            if nxt < total or nxt - total < pad or nxt % 0x1000 != 0:
                raise RuntimeError('ELF load padding is not page-aligned: %s' % path)
            pad = nxt - total
        padded.append(pad)
        total += pad
    for i in range(3, phnum):
        ph = phoff + i * 56
        if read(ph, 4) == 1 and (i != 3 or read(ph + 32, 8) != 0 or read(ph + 16, 8) != total):
            raise RuntimeError('ELF has an unexpected extra load: %s' % path)
    image = bytearray(total)
    position = 0
    for i in range(3):
        start, size = offsets[i], sizes[i]
        image[position:position + size] = data[start:start + size]
        position += padded[i]
    return hashlib.sha256(image).hexdigest()


def plugin_elf(ctx):
    """The pinned training ELF. Lifecycle instances publish the trace file, not an elf key."""
    elf = ctx.instance.get('elf') or ctx.instance.get('so')
    if elf:
        return elf
    trace = ctx.instance.get('trace_jsonl')
    if trace:
        config = Path(trace).with_name('trace.json')
        if config.is_file():
            symbols = json.loads(config.read_text()).get('symbols') or []
            if symbols and symbols[0].get('elf'):
                return symbols[0]['elf']
    raise RuntimeError('instance has no elf path; keys=%s' % sorted(ctx.instance))


def plugin_module(ctx):
    """The training NRO Eden loaded, matched by the pinned ELF's build id.

    `modules.list` has many `nro:` entries. The first one is not this plugin.
    """
    elf = plugin_elf(ctx)
    build_id = nro_build_id(elf)
    want = 'nro:' + build_id
    listed = ctx.call('modules.list') or {}
    entries = listed.get('modules') if isinstance(listed, dict) else listed
    entries = entries or []
    names = []
    for entry in entries:
        if not isinstance(entry, dict):
            continue
        name = str(entry.get('name') or '')
        if name.startswith('nro:'):
            names.append(name)
        if name == want:
            return entry
    raise RuntimeError('modules.list has no %s; nro names: %s' % (want, names))


def f9_body_trace(ctx):
    """Arm f9_body_of on the module name this boot lists, then wait for its return."""
    entry = plugin_module(ctx)
    name = str(entry.get('name'))
    elf = plugin_elf(ctx)
    loaded = ctx.call('symbols.load', {'module': name, 'elf': elf})
    symbol = 'training_rooms::runtime::roster::character_roster::sources::f9_body_of'
    target = '%s!<%s>' % (name, symbol)
    point = ctx.call('trace.arm', {
        'target': target,
        'registers': ['x0', 'x1'],
        'captures': [
            {'expression': 'x0+0x4150', 'type': 'u64'},
            {'expression': 'x0+0x4160', 'type': 'u64'},
            {'expression': 'x0+0x591c', 'type': 'u8'},
        ],
        'on_return': True,
    })
    line = ctx.wait_log(
        'trace_jsonl',
        r'(?:"event":"return".*f9_body_of|f9_body_of.*"event":"return")',
        cap=30)
    ctx.shot('f9-body-trace')
    return save(ctx, 'f9-body-trace', {'line': line, 'module': name, 'symbols': loaded, 'point': point})


# --- the final row -------------------------------------------------------------------------------

def heap_crash(ctx):
    """One instance's share of the last row: install heap, hook-stats, console scan."""
    install = install_heap(ctx)
    stats = hook_stats(ctx)
    console = console_scan(ctx)
    return save(ctx, 'heap-crash-%s' % ctx.on, {'install': install, 'hook_stats': stats,
                                                'console': console})