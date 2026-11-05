"""Build the Eden run folder for the frame strip's one boot (SPEC-input-06 §8): config.json, rows.json, the authored
same-frame macros, the hooks. Based on the Surface's rc2d run (lifecycle config, base hooks, references).

Usage: make_run.py <staging dir> (the rows folder is <staging dir>/rows; eden_test.py new --rows takes it)
"""
import hashlib, json, shutil, sys
from pathlib import Path

RUN = Path(sys.argv[1])
SRC = Path('/home/intpa/eden-runs/tr-p23-surface-rc2d')
LABEL = RUN.name
ROWS = RUN / 'rows'
SEG = ROWS / 'segments'


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


MOTION = [{"accel": [0, 0, -1], "delta_us": 1000, "gyro": [0, 0, 0]}] * 2


def macro(name, events, duration):
    """events: (frame, buttons, (lx, ly))."""
    out = {"bindings": {"gamepad": "", "keyboard": ""}, "duration_frames": duration, "events": [
        {"buttons": b, "frame": f, "motion": MOTION, "sticks": [lx, ly, 0, 0], "triggers": [0, 0]}
        for f, b, (lx, ly) in events], "format": "eden-input-macro", "game_id": "01006A800016E000",
        "markers": [], "name": name, "parent": {"marker": "", "name": ""}, "player": 0, "version": 1}
    path = SEG / (name + '.edenmacro.json')
    path.write_text(json.dumps(out) + '\n')
    return name, {"file": "segments/%s.edenmacro.json" % name, "sha256": sha(path), "cap": 20}


A, L, R, B, DUP = 0x1, 0x40, 0x80, 0x2, 0x2000
Z = (0, 0)


def combo(n):
    """Jab on frame 0, then up tilt: the stick tilted up (0.6, never a flick) from n-5, A again on n."""
    return macro('iv-combo-%02d' % n, [(0, A, Z), (2, 0, Z), (n - 5, 0, (0, 0.6)), (n, A, (0, 0.6)),
                                         (n + 2, 0, (0, 0.6)), (n + 8, 0, Z)], n + 70)


def jab_jab(n):
    """Jab, then A again on frame n (the jab's own follow-up, or a fresh jab once it has ended).""" 
    return macro('iv-jj-%02d' % n, [(0, A, Z), (2, 0, Z), (n, A, Z), (n + 2, 0, Z)], n + 70)


def jab_dtilt(n):
    """Jab, then down tilt: the stick tilted down (0.6) from n-5, A again on n.""" 
    return macro('iv-jd-%02d' % n, [(0, A, Z), (2, 0, Z), (n - 5, 0, (0, -0.6)), (n, A, (0, -0.6)),
                                       (n + 2, 0, (0, -0.6)), (n + 8, 0, Z)], n + 70)


def main():
    for d in (RUN / 'inputs', SEG, ROWS / 'refs'):
        d.mkdir(parents=True, exist_ok=True)
    shutil.copy(SRC / 'inputs/qt-b.ini', RUN / 'inputs/qt-b.ini')
    shutil.copy('/home/intpa/smash-training-lobby/tmp-p23/surface-tmp/rows-rc2/rc2-settings.json',
                RUN / 'inputs/iv-settings.json')
    for f in (SRC / 'rows/refs').iterdir():
        shutil.copy(f, ROWS / 'refs' / f.name)
    shutil.copy(SRC / 'rows/steps.py', ROWS / 'steps.py')
    shutil.copy(SRC / 'rows/p4t_boot.py', ROWS / 'p4t_boot.py')
    shutil.copy(Path(__file__).with_name('iv_steps.py'), ROWS / 'iv_steps.py')
    base = json.loads((SRC / 'rows/rows.json').read_text())
    segments = {k: v for k, v in base['segments'].items() if k in ('main-stage', 'stage-css', 'attract-cancel', 'title-confirm')}
    segments.update(dict([
        # L+R+A in one frame (Training's position reset), a second to settle, then walk into the CPU.
        macro('iv-reset-walk', [(0, L | R | A, Z), (4, 0, Z), (70, 0, (1, 0)), (190, 0, Z)], 205),
        macro('iv-jab', [(0, A, Z), (2, 0, Z)], 60),
        macro('iv-menu-open', [(0, B | DUP, Z), (8, 0, Z)], 14),
        macro('iv-menu-close', [(0, B, Z), (6, 0, Z), (40, B, Z), (46, 0, Z)], 80),
    ] + [combo(n) for n in (10, 14, 18, 22, 26, 32, 40)] + [jab_jab(n) for n in (6, 9, 12, 16, 22, 30)]
      + [jab_dtilt(n) for n in (10, 16, 22, 30)]))
    rows = {
        "game_id": base['game_id'],
        "start": "boot",
        "scope": "SPEC-input-06 frame strip, one boot on the diag build of p23/input-viz: hits, links (true, drop), "
                 "buffered and late presses, shield, hidden under the pause and the menu, Off, and the docs GIF.",
        "setup": [
            {"state": "Eden frontend up, game not yet loaded (defer-load); row 00-load calls game.load",
             "via": "fixture", "ref": "config defer_load; steps.py:preload"},
            {"state": "Settings: the Surface's rc2 fixture, whose old input_display [0,1,0,0] must migrate to the frame strip",
             "via": "fixture", "ref": "files: surface-tmp/rows-rc2/rc2-settings.json pinned to TrainingRooms/settings.json (carried from rc2d)"},
            {"state": "Training, HUD settled, plugin menu closed (P1 Steve, CPU as picked)",
             "via": "segment", "ref": "main-stage + stage-css library segments, steps.py:css_pick_and_enter"},
            {"state": "Fighters close: Training's L+R+A reset then a walk into the CPU",
             "via": "segment", "ref": "iv-reset-walk (authored same-frame macro)"},
            {"state": "CPU shield Infinite for the shield row, the strip Off for the Off row",
             "via": "fixture", "ref": "iv_steps.py:push (diag-settings-next.json, diag_settings_reload ack)"},
        ],
        "segments": segments,
        "gates": base['gates'],
        "rows": [
            base['rows'][0], base['rows'][1], base['rows'][2], base['rows'][3],
            {"id": "04-iv-ready", "start": "Training-settled", "end": "Training-settled",
             "steps": [{"run": "iv_steps.py:ready", "check": True}]},
            {"id": "05-iv-links", "start": "Training-settled", "end": "Training-settled",
             "steps": [{"run": "iv_steps.py:links", "check": True}]},
            {"id": "06-iv-shield", "start": "Training-settled", "end": "Training-settled",
             "steps": [{"run": "iv_steps.py:shield", "check": True}]},
            {"id": "07-iv-hide", "start": "Training-settled", "end": "Training-settled",
             "steps": [{"run": "iv_steps.py:hide", "check": True}]},
            {"id": "08-iv-off", "start": "Training-settled", "end": "Training-settled",
             "steps": [{"run": "iv_steps.py:off", "check": True}]},
            {"id": "09-iv-gif", "start": "Training-settled", "end": "Training-settled",
             "steps": [{"run": "iv_steps.py:gif", "check": True}]},
        ],
    }
    (ROWS / 'rows.json').write_text(json.dumps(rows, indent=2) + '\n')
    print('rows', sha(ROWS / 'rows.json'))


if __name__ == '__main__':
    main()
