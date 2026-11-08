#!/usr/bin/env python3
"""Build the docs-GIF fixtures and docs-gif-rows.json from the committed rows.json and fixtures.

python3 -I make_docs_gifs.py [id,id,...]   writes fixtures/{strip,play-chord,loop2,mismatch,jabs3}/ and
                                          docs-gif-rows.json; listed ids get one "-retry" row each

One XPS boot of the diag NRO records every docs GIF in docs/MEDIA.md that a fixture can reach. Each row copies
its fixture, sets up outside the clip, then records the documented inputs with the runner's own `gif` step
(preset docs: full frame, 1280 wide, at most the MEDIA.md length). No row waits for a diag line: the clip is
reviewed on its contact sheet.
"""
import copy
import json
import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
FIX = HERE / 'fixtures'
PIKACHU = 8  # fighter kind; Mario is 0


def load(path):
    return json.loads(path.read_text())


def dump(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + '\n')


def fixture(name, base, index_edit=None, files=None):
    """Copy fixture `base` to `name`, then apply index and file edits."""
    dest = FIX / name
    if dest.exists():
        shutil.rmtree(dest)
    shutil.copytree(FIX / base, dest)
    index = load(dest / 'recording-index.json')
    if index_edit:
        index_edit(index)
    dump(dest / 'recording-index.json', index)
    for rel, value in (files or {}).items():
        dump(dest / rel, value)


def build_fixtures():
    # Frame strip: nothing active, the bound save puts P1 at x=-30 and the CPU at x=0, both 0%.
    def strip(index):
        index['hot']['macro'] = []
        index['active'] = {'player': None, 'cpu': None}
        index['bound_save'] = 'jab-start'
    fixture('strip', 'play-warp', strip)
    shutil.rmtree(FIX / 'strip' / 'macros')
    (FIX / 'strip' / 'macros').mkdir()

    # Play chord: the jab macro (start save jab-start) active for the CPU. Loop 5 keeps it playing
    # long enough for the second ZR+Up to stop it on camera. Loops are 1, 2, 3, 5 or 10
    # (model::library::LoopCount); the first pass wrote 4 and the index did not load.
    def chord(index):
        index['active'] = {'player': None, 'cpu': 'jab'}
        index['loop'] = {'player': 1, 'cpu': 5}
        index['bound_save'] = 'jab-start'
    fixture('play-chord', 'play-warp', chord)

    # Loop and chain: a jab alone barely moves the CPU, so the first pass showed no visible "continue
    # from here". This macro walks left (toward P1 after the warp) for 30 frames, then jabs.
    walk = copy.deepcopy(load(FIX / 'play-warp' / 'macros' / 'jab.json'))
    idle = copy.deepcopy(walk['frames'][1])
    step = dict(idle, lstick_x=-70)
    walk['frames'] = ([copy.deepcopy(step) for _ in range(30)] + [copy.deepcopy(walk['frames'][0])]
                      + [copy.deepcopy(idle) for _ in range(24)])
    walk['name'] = 'Walk jab'

    def loop2(index):
        index['active'] = {'player': None, 'cpu': 'jab'}
        index['loop'] = {'player': 1, 'cpu': 2}
        index['bound_save'] = 'jab-start'
    fixture('loop2', 'play-warp', loop2, files={'macros/jab.json': walk})
    fixture('chain2', 'chain', files={'macros/jab.json': walk})

    # Fighter Mismatch: the chain fixture with macro 1 recorded on Pikachu. The CPU is Mario.
    jab = load(FIX / 'chain' / 'macros' / 'jab.json')
    jab['start']['fighter_kind'] = PIKACHU
    fixture('mismatch', 'chain', files={'macros/jab.json': jab})

    # Make combo / crop: one macro of three jabs, 14 frames apart, from the jab start save.
    three = copy.deepcopy(load(FIX / 'play-warp' / 'macros' / 'jab.json'))
    idle = copy.deepcopy(three['frames'][1])
    press = copy.deepcopy(three['frames'][0])
    frames = [copy.deepcopy(idle) for _ in range(70)]
    for at in (0, 14, 28):
        frames[at] = copy.deepcopy(press)
    three['frames'] = frames
    three['name'] = 'Jab x3'

    def jabs3(index):
        index['hot']['macro'] = ['jabs3']
        index['active'] = {'player': None, 'cpu': None}
        index['bound_save'] = 'jab-start'
    fixture('jabs3', 'play-warp', jabs3, files={'macros/jabs3.json': three})
    (FIX / 'jabs3' / 'macros' / 'jab.json').unlink()


# (row id, hook suffix, MEDIA.md length in s, what must be visible)
CLIPS = [
    ('frame-strip-lanes', 'lanes', 6, 'P1 gold startup, red active, blue recovery; CPU light-grey hitlag then purple hitstun; a run of 7+ shows its length'),
    ('frame-strip-chips', 'chips', 4, 'one green attack chip with an up-right arrow and a white tick down to P1\'s lane; no second chip'),
    ('frame-strip-cstick', 'cstick', 4, 'one attack chip with a right arrow; the left stick stays neutral'),
    ('frame-strip-stick-row', 'stick_row', 6, 'a row under P1\'s lane with a direction only while held, blanks while neutral, scrolling with the lanes'),
    ('frame-strip-freeze', 'freeze', 5, 'the lanes stop while both stand, then move again on the second jab'),
    ('frame-strip-freeze-stick', 'freeze_stick', 5, 'the strip keeps scrolling while the stick changes direction with no button'),
    ('frame-strip-advantage-hit', 'adv_hit', 5, 'ADVANTAGE is a signed number with on hit under it'),
    ('frame-strip-link', 'link', 5, 'LINK 0 or below with TRUE combo in green, and a green window Nf bar above P1\'s lane'),
    ('frame-strip-drop', 'drop', 6, 'LINK a positive number with DROP in red'),
    ('frame-strip-input', 'input', 6, 'INPUT shows Nf late / on time / buffered Nf, then N spare or Nf past'),
    ('frame-strip-last', 'last', 7, 'LAST shows pills newest first, TRUE or DROP with a number, moves like Jab > Jab'),
    ('frame-strip-exchange', 'exchange', 8, 'the late second hit does not become a large DROP; ADVANTAGE is the new exchange, on hit'),
    ('frame-strip-advantage-shield', 'adv_shield', 6, 'ADVANTAGE reads on shield; LINK does not become TRUE combo or DROP'),
    ('play-chord', 'play_chord', 8, 'one warp to the start save, the CPU plays the jab, a second ZR+Up stops it'),
    ('loop-no-warp', 'loop', 8, 'the first pass warps once; the second pass does not warp and continues in place'),
    ('chain-play', 'chain', 9, 'one warp to macro 1\'s start save, then recording 1 and recording 2, no warp between'),
    ('fighter-mismatch', 'mismatch', 7, 'the Fighter Mismatch rows with the cursor on Keep; Keep plays the chain on Mario; no second prompt'),
    ('macro-screen', 'macro_screen', 6, 'breadcrumb Recording (Player view) > Save states & Macros > name; Start save orange floppy On; one Set active row'),
    ('make-combo', 'make_combo', 8, 'the macro plays once on P1 from its start save, a new Combo N appears, the drilled combo stays the drill'),
    ('crop-combo', 'crop_combo', 7, 'the handle screen is open, a handle moves, A writes a new combo, the original stays'),
]


MENU_CLIPS = {'macro_screen', 'make_combo', 'crop_combo'}
# The one retry per GIF, after the first XPS pass's contact sheets (LANE-OPUS-RC5-GIFS-REPORT.md).
RETRIES = set(sys.argv[1].split(',')) if len(sys.argv) > 1 and sys.argv[1] else set()


def build_rows():
    base = load(HERE / 'rows.json')
    accept = load(HERE / 'accept-rows.json')
    segments = {}
    for source in (base['segments'], accept['segments']):
        for name, spec in source.items():
            segments.setdefault(name, spec)
    for name, spec in accept['segments'].items():  # the acceptance set carries the recorded sha256s
        if spec.get('sha256'):
            segments[name] = spec
    gates = {
        'main': {'library': 'main', 'recover': 'attract-cancel', 'recover_times': 4, 'scale': 'fit', 'cap': 45},
        'stage': {'library': 'stage', 'scale': 'fit'},
        'css': {'library': 'css', 'scale': 'fit'},
        'training-hud-unpaused': {'library': 'training-hud-unpaused', 'scale': 'fit'},
    }
    rows = [{
        'id': 'docs-boot', 'start': 'boot', 'end': 'training',
        'steps': [{'gate': 'main'}, {'run': 'docsgif.py:boot', 'check': True}],
        'expect': 'The first gate leaves the intro with attract-cancel; the library route enters Training on Mario.',
    }]
    segments['jab-drop2'] = {
        'file': 'segments/jab-drop2.edenmacro.json', 'cap': 30,
        'recipe': [{'buttons': 'a', 'frames': 2, 'gap_frames': 32}, {'buttons': 'a', 'frames': 2, 'gap_frames': 40}],
        'note': 'the second jab after the CPU is free and inside the 20-frame exchange (jab-drop was 42 frames)'}
    clips = [(row_id, hook, seconds, visible, '') for row_id, hook, seconds, visible in CLIPS]
    clips += [(row_id, hook, seconds, visible, '-retry')
              for row_id, hook, seconds, visible in CLIPS if row_id in RETRIES]
    for row_id, hook, seconds, visible, suffix in clips:
        rows.append({
            'id': 'gif-' + row_id + suffix, 'start': 'training', 'end': 'training',
            'start_gate': 'training-hud-unpaused',
            'steps': [
                {'run': 'docsgif.py:setup_' + hook, 'check': True},
                {'gif': {'name': 'rc5-' + row_id, 'preset': 'docs', 'fps': 15, 'length': seconds,
                         'steps': [{'run': 'docsgif.py:clip_' + hook}]}},
                {'run': 'docsgif.py:' + ('after_menu' if hook in MENU_CLIPS else 'after'), 'check': True},
            ],
            'expect': visible, 'media': 'docs/media/rc5-%s.gif' % row_id, 'budget_min': 2,
        })
    plan = {
        'game_id': base['game_id'], 'start': 'boot', 'early_stop': False, 'on_failure': 'continue',
        'scope': 'rc.5 docs GIFs on the XPS diag window (NRO 6bbf2ecd). One recording per GIF; the contact sheet '
                 'decides. No row waits for a change-event diag line.',
        'setup': [
            {'state': 'One XPS instance, diag NRO, volume 0, Res1X (eden_settings core.resolution_setup 3)',
             'via': 'fixture', 'ref': 'config.json eden_settings and pinned settings.json (hitbox off, frame strip)'},
            {'state': 'Training, P1 Mario, CPU Mario', 'via': 'segment',
             'ref': 'library attract-cancel (recover on gate main), main-stage, stage-css, enter-training; steps.py:boot_training'},
            {'state': 'Each GIF row copies its own recording fixture', 'via': 'fixture',
             'ref': 'eden/recording-redesign/fixtures/{strip,play-chord,loop2,chain,mismatch,play-warp,jabs3}'},
        ],
        'segments': segments, 'gates': gates,
        'recover': [{'run': 'steps.py:recover_training', 'check': True}],
        'rows': rows,
    }
    dump(HERE / 'docs-gif-rows.json', plan)


if __name__ == '__main__':
    build_fixtures()
    build_rows()
