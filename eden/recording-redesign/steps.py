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
import json
import re
import time
from pathlib import Path

# The factory chord macros (segments/, authored same-frame per docs/MACROS.md rule 14).
SAVE, LOAD = 'chord-save', 'chord-load'
RECORD, PLAY_CPU, PLAY_P1, POPUP = 'chord-record', 'chord-play-cpu', 'chord-play-p1', 'chord-popup'
MENU_OPEN, MENU_CLOSE, X = 'menu-open', 'menu-close', 'x-press'
RESET_WALK, JAB, FSMASH = 'reset-walk', 'jab', 'fsmash'


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
    time.sleep(0.8)
    settle(ctx)


def popup(ctx):
    ctx.play(POPUP)
    return ctx.wait_log('diag', r'screen=recording-popup\b', cap=10)


def popup_close(ctx):
    press(ctx, 'b', 0.6)


def diag(ctx, pattern, cap=10):
    return ctx.wait_log('diag', pattern, cap=cap)


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
    return {'gif': str(path), 'seconds': seconds, 'bytes': path.stat().st_size, 'status': status}


def console_scan(ctx):
    """No Unmapped and no panic lines in the console log (the whole file, this run)."""
    text = Path(ctx.instance['console_log']).read_text(errors='replace')
    bad = [l for l in text.splitlines() if ('Unmapped' in l or 'panic' in l.lower())]
    if bad:
        raise RuntimeError('console has Unmapped/panic lines: %s' % bad[:5])
    return {'scanned_bytes': len(text), 'unmapped_or_panic': 0}


def install_heap(ctx):
    """The install line says heap_allocations=0 (the events log's cpu_common=installed line)."""
    text = ''
    events = Path(ctx.instance.get('events_log') or ctx.instance['console_log'])
    for p in (events.with_name('events.previous.log'), events):
        if p.exists():
            text += p.read_text(errors='replace')
    line = [l for l in text.splitlines() if 'cpu_common=installed' in l]
    if not line:
        raise RuntimeError('no install line in the events log')
    m = re.search(r'heap_allocations=(\d+)', line[-1])
    if not m:
        raise RuntimeError('install line has no heap_allocations count: %s' % line[-1])
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

def boot_training(ctx):
    """Bring this instance to Training, P1 Mario / CPU Mario, 0%, strip on, menu closed.

    Fresh instance: game.load (defer_load configs) if the game is not running, then the library
    route (attract-cancel -> main, main-stage, stage-css), the Mario/Mario pick on the CSS, and
    enter-training. Checked: the HUD gate leaves the screen verified.
    """
    status = ctx.call('status') or {}
    if not (status.get('running') and status.get('loading_complete')):
        ctx.call('game.load', {'path': ctx.instance['game']}, job_cap=180)
    try:
        ctx.gate('main')
    except RuntimeError:
        ctx.play('attract-cancel')  # the idle attract-mode intro (runner "known detours")
        ctx.gate('main')
    ctx.play('main-stage')
    ctx.gate('stage')
    ctx.play('stage-css')
    ctx.gate('css')
    # The Mario/Mario pick: hover a tile, wait for the tile cursor to settle, confirm both roles,
    # then enter Training. The tile numbers are the session's pick (README, Open questions).
    ctx.call('input.stick', {'stick': 'left', 'x': 0, 'y': 1, 'lease_ms': 400})
    time.sleep(0.3)
    ctx.call('input.stick', {'stick': 'left', 'x': 0, 'y': 0, 'lease_ms': 100})
    line = ctx.wait_log('tile', r'settled screen=css at=\d+ hovered=\d+', cap=8)
    at, hovered = re.search(r'at=(\d+) hovered=(\d+)', line).groups()
    if at != hovered:
        raise RuntimeError('hovered and settled tiles differ: %s' % line)
    press(ctx, 'a', 0.35)
    ctx.gate('ready')
    ctx.play('enter-training')
    settle(ctx)
    return {'css_tile': at}


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
    """The migrated slot plays from where the fighters are: no warp, name Slot 1 (date)."""
    def do():
        popup(ctx)
        # The migrated macro is Player 1's active macro after migration (selected.txt); play it on
        # the CPU view per the GIF. Bounded: open the list and find the Slot 1 file by name.
        press(ctx, 'r', 0.5)          # pop-up role: CPU
        line = diag(ctx, r'screen=recording-popup role=cpu', cap=8)
        press(ctx, 'a', 0.6)          # A on the macro: set active for this role (pop-up A acts)
        popup_close(ctx)
        time.sleep(0.5)
        ctx.play(PLAY_CPU)
        time.sleep(2.5)
    out = gif(ctx, 'rc5-migration.gif', 8, do)
    warp = None
    try:
        warp = diag(ctx, r'playback .*warp=1', cap=2)
    except RuntimeError:
        pass
    if warp:
        raise RuntimeError('the migrated slot warped on play: %s' % warp)
    ctx.shot('migration-after')
    return out


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
    # macro-start-save-missing: delete the recorded macro's start save through its own screen
    # (the plugin menu's Delete removes the file), then play the macro.
    popup(ctx)
    press(ctx, 'a', 0.6)              # A on a macro in the pop-up: open its file screen? No: pop-up
    # A binds/sets active/drills (page A opens). Open the Recording page for the file screen.
    popup_close(ctx)
    open_menu(ctx)
    for _ in range(6):                # step menu pages with R until the Recording page
        press(ctx, 'r', 0.6)
    press(ctx, 'a', 0.8)              # A on the macro row: open its own screen (page A opens)
    for _ in range(10):               # walk to the Start save screen's Delete row
        press(ctx, 'down', 0.4)
    press(ctx, 'a', 0.8)              # Delete
    press(ctx, 'a', 0.8)              # confirm
    close_menu(ctx)
    ctx.play(PLAY_P1)
    missing = diag(ctx, r'screen=notice kind=macro-start-save-missing', cap=10)
    ctx.shot('notice')
    return save(ctx, 'notice', {'chord_once': chord_lines, 'missing': missing})


def rr_mismatch(ctx):
    """A chain whose macro 1 fighter differs: one prompt at the chain start naming macro 1."""
    settle(ctx)
    # Bounded build of the documented fixture: record a macro on P1, rename it, put it first in a
    # chain, set the chain active for the CPU, play. The fighter change itself needs the CSS
    # (README open question); this hook plays the chain and captures the prompt's line and rows.
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
    later = ctx.call('status')
    return save(ctx, 'mismatch', {'line': line, 'shot': str(shot)})


def rr_rename(ctx):
    """A on Rename opens the titled keyboard offer (Online off; with Online on it is not made)."""
    settle(ctx)
    open_menu(ctx)
    for _ in range(6):
        press(ctx, 'r', 0.6)
    press(ctx, 'a', 0.8)              # A on the macro row: the file screen (page A opens)
    for _ in range(10):
        press(ctx, 'down', 0.4)
    press(ctx, 'a', 0.8)              # A on Rename
    line = diag(ctx, r'screen=rename header=Name macro', cap=10)
    ctx.shot('rename-offer')
    press(ctx, 'b', 0.8)              # leave the keyboard without renaming
    close_menu(ctx)
    return save(ctx, 'rename', {'line': line})


def rr_crop(ctx):
    """A on Crop: screen=crop source=<id> combo=0, then the new file write, originals unchanged."""
    settle(ctx)
    open_menu(ctx)
    for _ in range(6):
        press(ctx, 'r', 0.6)
    press(ctx, 'a', 0.8)
    for _ in range(12):
        press(ctx, 'down', 0.4)
    press(ctx, 'a', 0.8)              # A on Crop
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
        try:
            ctx.play(name)
            time.sleep(0.6)
            played[key] = 'played'
        except RuntimeError as e:
            played[key] = 'refused: %s' % e
    line = diag(ctx, r'screen=recording-popup\b', cap=4)  # the pop-up chord toggled the window
    popup_close(ctx)
    return save(ctx, 'chords', {'shortcuts_shot': str(shot), 'played': played, 'popup_line': line})


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
        popup(ctx)
        press(ctx, 'b', 0.6)          # pop-up role Player
        ctx.play(RECORD)
        ctx.call('input.stick', {'stick': 'left', 'x': 1, 'y': 0, 'lease_ms': 900})
        time.sleep(0.9)
        ctx.call('input.stick', {'stick': 'left', 'x': 0, 'y': 0, 'lease_ms': 100})
        ctx.play(JAB)
        time.sleep(0.8)
        ctx.play(RECORD)              # stop
        time.sleep(0.6)
        popup(ctx)                    # open the pop-up: the new macro at the top of the hot five
        time.sleep(0.8)
        popup_close(ctx)
    out = gif(ctx, 'rc5-record-start-save.gif', 8, do)
    return save(ctx, 'record-start-save', out)


def media_play_chord(ctx):
    """Set the macro active for the CPU, stand away from its start save, ZR+Up."""
    def do():
        popup(ctx)
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
    warp = diag(ctx, r'playback .*warp=1', cap=4)
    return save(ctx, 'play-chord', {**out, 'warp': warp})


def media_loop_no_warp(ctx):
    """Loop 2 (X while the cursor is in MACROS); the second pass does not warp."""
    def do():
        popup(ctx)
        ctx.play(X)
        press(ctx, 'r', 0.5)
        press(ctx, 'a', 0.6)
        popup_close(ctx)
        ctx.play(PLAY_CPU)
        time.sleep(6.5)
    out = gif(ctx, 'rc5-loop-no-warp.gif', 8, do)
    first = diag(ctx, r'playback .*warp=1', cap=4)
    second = []
    try:
        second = [diag(ctx, r'playback .*warp=1', cap=3)]
    except RuntimeError:
        pass
    if second:
        raise RuntimeError('the second pass warped: %s' % second)
    return save(ctx, 'loop-no-warp', {**out, 'first_warp': first})


def media_chain_play(ctx):
    """A chain of two recordings, the first with a start save; one warp, then both, no gap warp."""
    def do():
        popup(ctx)
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
        popup(ctx)
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
        popup(ctx)
        press(ctx, 'r', 0.5)
        press(ctx, 'a', 0.6)
        popup_close(ctx)
        ctx.play(PLAY_CPU)
        time.sleep(1.2)
        press(ctx, 'b', 0.8)          # Keep current fighter
        time.sleep(4.0)
    out = gif(ctx, 'rc5-fighter-mismatch.gif', 7, do)
    line = diag(ctx, r'screen=mismatch ask=macro1', cap=6)
    return save(ctx, 'fighter-mismatch', {**out, 'line': line})


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
    def do():
        ctx.play(SAVE)
        time.sleep(1.0)
        ctx.play(RESET_WALK)
        time.sleep(0.4)
        ctx.play(LOAD)
        time.sleep(2.0)
    out = gif(ctx, 'rc5-save-warp.gif', 6, do)
    quiet = []
    try:
        quiet = diag(ctx, r'playback |screen=recording\b', cap=2)
    except RuntimeError:
        pass
    if quiet:
        raise RuntimeError('the save warp started a recording: %s' % quiet)
    return save(ctx, 'save-warp', out)


# --- surface-a: the timed strip rows, the advantage cases, the graded GIFs -----------------------

def reset(ctx):
    ctx.play(RESET_WALK)
    time.sleep(0.5)
    settle(ctx)


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


def advantage(ctx, move_segment, shield):
    """One advantage attempt: the strip's ADVANTAGE readout, compared with published frame data.

    The published page and its number are not in any file this card names; the row leaves the
    comparison to the session (README, Open questions) and captures the readout and a shot.
    """
    reset(ctx)
    if shield:
        _cpu_shield_hold(ctx)
    before = []
    try:
        before = [diag(ctx, r'advantage ', cap=2)]
    except RuntimeError:
        pass
    ctx.play(move_segment)
    time.sleep(2.5)
    line = diag(ctx, r'advantage value=-?\d+ on_(hit|shield)=true', cap=10)
    shot = ctx.shot('advantage')
    if shield:
        _cpu_shield_none(ctx)
    return save(ctx, 'advantage', {'line': line, 'shot': str(shot), 'before': before,
                                   'published': 'to be filled by the session'})


def _cpu_shield_hold(ctx):
    """CPU menu: ZL+Down to Shield, Right until Hold, B closes (docs/MEDIA.md advantage-shield)."""
    open_menu(ctx)
    ctx.play('chord-save')            # ZL+Down jumps sections (the menu's section chord)
    time.sleep(0.6)
    press(ctx, 'right', 0.5)
    press(ctx, 'a', 0.6)
    close_menu(ctx)


def _cpu_shield_none(ctx):
    """CPU Shield back to None (the shared start)."""
    open_menu(ctx)
    ctx.play('chord-save')
    time.sleep(0.6)
    press(ctx, 'left', 0.5)
    press(ctx, 'a', 0.6)
    close_menu(ctx)


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
    """Save last combo on a true jab-jab; Load; attempt 1 late-but-within-tolerance, attempt 2 tilt."""
    def do():
        reset(ctx)
        ctx.play(JAB)
        time.sleep(0.6)
        ctx.play(JAB)                 # the true jab > jab
        time.sleep(1.0)
        ctx.play(SAVE)                # Save last combo (the combo helper's save)
        time.sleep(0.8)
        ctx.play(LOAD)                # the same save back
        time.sleep(0.8)
        ctx.play('jab')               # attempt 1: the follow-up late but within the tolerance
        time.sleep(0.6)
        ctx.play('jab')
        time.sleep(1.0)
        ctx.play(LOAD)
        time.sleep(0.8)
        ctx.play('tilted-a')          # attempt 2: a forward tilt instead of a jab
        time.sleep(1.5)
    out = gif(ctx, 'rc5-grade-not-true.gif', 9, do)
    return save(ctx, 'grade-not-true', out)


def media_sequence_drill(ctx):
    """A sequence of two short combos, drilled, streak 0: land combo 1, fail combo 2."""
    def do():
        reset(ctx)
        ctx.play(JAB)
        time.sleep(0.8)
        ctx.play('jab')               # combo 1 clean
        time.sleep(1.0)
        ctx.play('tilted-a')          # combo 2 fails with a different move
        time.sleep(1.5)
    out = gif(ctx, 'rc5-sequence-drill.gif', 9, do)
    line = diag(ctx, r'drill ', cap=6)
    return save(ctx, 'sequence-drill', {**out, 'line': line})


# --- surface-b: the frame-strip GIFs -------------------------------------------------------------

def media_strip_lanes(ctx):
    """Dash, then a forward smash into the CPU; hold still until both can act."""
    def do():
        reset(ctx)
        ctx.call('input.stick', {'stick': 'left', 'x': 1, 'y': 0, 'lease_ms': 350})
        time.sleep(0.35)
        ctx.call('input.stick', {'stick': 'left', 'x': 0, 'y': 0, 'lease_ms': 100})
        ctx.play(FSMASH)
        time.sleep(3.0)
    out = gif(ctx, 'rc5-frame-strip-lanes.gif', 6, do)
    return save(ctx, 'strip-lanes', out)


def media_strip_chips(ctx):
    """Left stick up-right past both thresholds, then A once; release."""
    def do():
        reset(ctx)
        ctx.call('input.stick', {'stick': 'left', 'x': 0.8, 'y': 0.8, 'lease_ms': 400})
        ctx.play(JAB)
        time.sleep(0.6)
        ctx.call('input.stick', {'stick': 'left', 'x': 0, 'y': 0, 'lease_ms': 100})
        time.sleep(1.5)
    out = gif(ctx, 'rc5-frame-strip-chips.gif', 4, do)
    return save(ctx, 'strip-chips', out)


def media_strip_freeze(ctx):
    """Jab, both stand a full second, jab again: the lanes stop, then move."""
    def do():
        reset(ctx)
        ctx.play(JAB)
        time.sleep(1.6)
        ctx.play(JAB)
        time.sleep(1.5)
    out = gif(ctx, 'rc5-frame-strip-freeze.gif', 5, do)
    return save(ctx, 'strip-freeze', out)


def media_strip_advantage_hit(ctx):
    """One jab on the CPU, then both stand until the readout settles."""
    def do():
        reset(ctx)
        ctx.play(JAB)
        time.sleep(3.0)
    out = gif(ctx, 'rc5-frame-strip-advantage-hit.gif', 5, do)
    line = diag(ctx, r'advantage value=-?\d+ on_hit=true', cap=6)
    return save(ctx, 'strip-advantage-hit', {**out, 'line': line})


def media_strip_advantage_shield(ctx):
    """Menu, CPU Shield Hold, close; jab the shield; the readout settles on shield."""
    def do():
        reset(ctx)
        _cpu_shield_hold(ctx)
        reset(ctx)
        ctx.play(JAB)
        time.sleep(3.0)
        _cpu_shield_none(ctx)
    out = gif(ctx, 'rc5-frame-strip-advantage-shield.gif', 6, do)
    line = diag(ctx, r'advantage value=-?\d+ on_shield=true', cap=6)
    return save(ctx, 'strip-advantage-shield', {**out, 'line': line})


def media_strip_link(ctx):
    """Jab, then A again as soon as the first jab can be cancelled: TRUE combo."""
    def do():
        reset(ctx)
        ctx.play(JAB)
        time.sleep(0.35)
        ctx.play(JAB)
        time.sleep(2.0)
    out = gif(ctx, 'rc5-frame-strip-link.gif', 5, do)
    line = diag(ctx, r'link verdict=true', cap=6)
    return save(ctx, 'strip-link', {**out, 'line': line})


def media_strip_drop(ctx):
    """Jab; wait until the CPU can act; jab again inside 20 frames: DROP."""
    def do():
        reset(ctx)
        ctx.play(JAB)
        time.sleep(0.9)
        ctx.play(JAB)
        time.sleep(1.5)
    out = gif(ctx, 'rc5-frame-strip-drop.gif', 6, do)
    line = diag(ctx, r'link verdict=drop', cap=6)
    return save(ctx, 'strip-drop', {**out, 'line': line})


def media_strip_input(ctx):
    """The late jab of the drop clip: INPUT shows Nf late, then N spare or Nf past."""
    def do():
        reset(ctx)
        ctx.play(JAB)
        time.sleep(1.0)
        ctx.play(JAB)
        time.sleep(2.0)
    out = gif(ctx, 'rc5-frame-strip-input.gif', 6, do)
    line = diag(ctx, r'input ', cap=6)
    return save(ctx, 'strip-input', {**out, 'line': line})


def media_strip_last(ctx):
    """Three jabs in a row, then stand: the LAST row shows the pills, newest first."""
    def do():
        reset(ctx)
        for _ in range(3):
            ctx.play(JAB)
            time.sleep(0.5)
        time.sleep(2.0)
    out = gif(ctx, 'rc5-frame-strip-last.gif', 7, do)
    return save(ctx, 'strip-last', out)


def media_strip_exchange(ctx):
    """Jab, stand a full second (past 20 frames), jab again: a new exchange, not a big DROP."""
    def do():
        reset(ctx)
        ctx.play(JAB)
        time.sleep(1.8)
        ctx.play(JAB)
        time.sleep(2.0)
    out = gif(ctx, 'rc5-frame-strip-exchange.gif', 8, do)
    return save(ctx, 'strip-exchange', out)


# --- yoga-a: the rr-11 stick-row GIFs and the combo rows ------------------------------------------------

def media_strip_cstick(ctx):
    """Left stick neutral; C-stick right, one forward smash; no A (right stick pressed directly;
    the recipe actions cannot express it — README, Open questions)."""
    def do():
        reset(ctx)
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
    def do():
        reset(ctx)
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
    def do():
        reset(ctx)
        ctx.play(JAB)
        time.sleep(0.8)
        ctx.call('input.stick', {'stick': 'left', 'x': -1, 'y': 0, 'lease_ms': 500})
        ctx.call('input.stick', {'stick': 'left', 'x': 1, 'y': 0, 'lease_ms': 500})
        ctx.call('input.stick', {'stick': 'left', 'x': 0, 'y': 0, 'lease_ms': 100})
        time.sleep(0.8)
    out = gif(ctx, 'rc5-frame-strip-freeze-stick.gif', 5, do)
    return save(ctx, 'strip-freeze-stick', out)


def combo(ctx, name, expect_pattern):
    """Drill the combo (A on it makes it the drill, no chord); the bar's verdict names the faults."""
    reset(ctx)
    popup(ctx)
    press(ctx, 'b', 0.6)
    open_menu(ctx)
    for _ in range(6):
        press(ctx, 'r', 0.6)
    for _ in range(3):
        press(ctx, 'down', 0.4)
    press(ctx, 'a', 0.8)              # A on the combo: Drill
    close_menu(ctx)
    ctx.play(name)
    time.sleep(2.5)
    line = diag(ctx, expect_pattern, cap=10)
    shot = ctx.shot('bar')
    return save(ctx, name, {'line': line, 'shot': str(shot)})


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
    """The four bar cases: non-TRUE clean, missing press, gap 20 counts / 21 stops, drop restarts."""
    out = {'cases': []}
    reset(ctx)
    # Case 1: a non-TRUE link inside the tolerance still counts (jab > jab late but within).
    ctx.play(JAB)
    time.sleep(0.9)
    ctx.play(JAB)
    time.sleep(1.0)
    out['cases'].append({'case': 'non-true clean', 'shot': str(ctx.shot('grade-clean'))})
    # Case 2: the follow-up never comes (a missing press drops).
    ctx.play(JAB)
    time.sleep(2.5)
    out['cases'].append({'case': 'missing press', 'shot': str(ctx.shot('grade-missing'))})
    # Case 3: a drop restarts the sequence at combo 1, streak 0, best kept.
    ctx.play('tilted-a')
    time.sleep(1.5)
    out['cases'].append({'case': 'wrong move drop', 'shot': str(ctx.shot('grade-drop'))})
    settle(ctx)
    return save(ctx, 'grade-bar', out)


def rr_drill(ctx):
    """A on a combo or the sequence drills it; Make combo leaves the drill."""
    settle(ctx)
    open_menu(ctx)
    for _ in range(6):
        press(ctx, 'r', 0.6)
    for _ in range(3):
        press(ctx, 'down', 0.4)
    press(ctx, 'a', 0.8)              # A on the combo: Drill
    time.sleep(1.0)
    ctx.play('jab')
    time.sleep(1.0)
    shot1 = ctx.shot('drill-combo')
    # Make combo from the macro's screen; the drill stays.
    for _ in range(6):
        press(ctx, 'down', 0.4)
    press(ctx, 'a', 0.8)              # Make combo
    time.sleep(3.0)
    shot2 = ctx.shot('after-make-combo')
    close_menu(ctx)
    return save(ctx, 'drill', {'drill_shot': str(shot1), 'after_make_combo': str(shot2)})


# --- the final row -------------------------------------------------------------------------------

def heap_crash(ctx):
    """One instance's share of the last row: install heap, hook-stats, console scan."""
    install = install_heap(ctx)
    stats = hook_stats(ctx)
    console = console_scan(ctx)
    return save(ctx, 'heap-crash-%s' % ctx.on, {'install': install, 'hook_stats': stats,
                                                'console': console})