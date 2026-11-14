#!/usr/bin/env python3
"""Build the rc.14 2e (a-full, A1) rows package (2026-11-14) from the option-GIF takes framework, read-only, the way
eden/rc14-cycle-fix/mk-rows.py does: the boot rows, gates and segment pins (make_packages.py), the helpers, cycle-fix's
cyclefix.py (its setup and knob) and afull.py from this folder.

    /usr/bin/python3 -I mk-rows.py <out folder>        # writes <out>/rows/"""
import hashlib
import json
import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
LIB = Path('/home/intpa/smash-training-lobby/artifacts/phase23/option-gifs-20261113/takes/_lib')
sys.path.insert(0, str(LIB))
import make_packages as mp  # noqa: E402

HELPERS = mp.HELPERS + ['gna_p2_fs.py']
SEGMENTS = mp.SEGMENTS + ['p2-fs-p1-cycle', 'p2-fs-cpu-cycle', 'p2-fs-p1-hat']
CF = HERE.parent / 'rc14-cycle-fix' / 'cyclefix.py'
M = 'afull.py'
ROWS = [
    ('af-setup', 'cyclefix.py', 'cf_setup', 8, 'Live values (RF 8, the console lanes), a P1 lap and a CPU lap, the notice settled.'),
    ('af-keep', M, 'af_keep', 5, 'r1: a forced Reload (direct pick of Bowser) keeps the CPU window of two or more members.'),
    ('af-attach', M, 'af_attach', 8, 'r2/r3: the kept window warm-attached again, then two CPU laps; helper reloads of a '
                                     'kept kind recorded; no StallWatch or Unmapped.'),
    ('af-mii', M, 'af_mii', 6, 'r4: the Mii Gunner window kept across the forced Reload, then warm-attached again.'),
    ('af-kirby', M, 'af_kirby', 6, 'r6: Kirby alone in his window, kept by the forced Reload, then the warm return at cell 0.'),
    ('af-kirby-copy', M, 'af_kirby_copy', 5, 'r7: Kirby with a hat: the gated Reload destroys his window alone; (a\') keeps '
                                            'the other groups.'),
    ('af-pair', M, 'af_pair', 8, 'r5: the Ice Climbers pair window kept across the forced Reload, then its warm return (RF 10).'),
    ('af-restore', M, 'af_restore', 3, 'The remembered values back, Mario / Mario.'),
]


def row(rid, module, fn, budget, expect):
    return {'id': rid, 'start': 'training', 'end': 'training', 'start_gate': 'training-hud-unpaused',
            'group': 'rc14-a-full', 'budget_min': budget, 'steps': [{'run': '%s:%s' % (module, fn), 'check': True}],
            'expect': expect}


def plan():
    segs = mp.lib_segments()
    for name in SEGMENTS:
        path = LIB / 'segments' / (name + '.edenmacro.json')
        segs[name] = {'file': 'segments/%s.edenmacro.json' % name,
                      'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'cap': 90 if name.startswith('p2-fs-') else 30}
    setup = list(mp.BOOT_SETUP) + [
        {'state': 'Training, Ready Fighters 8, P1 original > Pikachu > Kirby > Palutena, CPU original > Mii Gunner > '
                  'Pichu > Diddy Kong > King K. Rool, every Loading row at the card value (its live values)',
         'via': 'hook', 'ref': 'cyclefix.py:cf_setup'},
        {'state': 'A1 on for a row: diag-cycle.req `reload window` (`reload mii window` for the Mii row)',
         'via': 'hook', 'ref': 'afull.py:_keep_across'}]
    setup[0] = dict(setup[0], state=setup[0]['state'] + '; TrainingRooms NRO swapped in by prep_run.py --tr-nro')
    return {'game_id': '01006A800016E000', 'start': 'frontend-ready', 'early_stop': False, 'on_failure': 'continue',
            'scope': 'rc.14 2e (a-full, A1): one boot, the boot rows, the af-* rows, z-boot; the a-full diag through '
                     'prep_run --tr-nro.',
            'setup': setup, 'segments': segs, 'gates': mp.gates(),
            'recover': [{'run': 'steps.py:recover_training', 'check': True}], 'gif_presets': {},
            'rows': mp.boot_rows() + [row(*r) for r in ROWS] + [mp.end_row()]}


def main(out):
    rows = Path(out) / 'rows'
    if rows.exists():
        shutil.rmtree(rows)
    (rows / 'segments').mkdir(parents=True)
    for name in HELPERS:
        shutil.copyfile(LIB / name, rows / name)
    shutil.copyfile(CF, rows / 'cyclefix.py')
    shutil.copyfile(HERE / M, rows / M)
    for name in SEGMENTS:
        shutil.copyfile(LIB / 'segments' / (name + '.edenmacro.json'), rows / 'segments' / (name + '.edenmacro.json'))
    (rows / 'rows.json').write_text(json.dumps(plan(), indent=1) + '\n')
    print(rows, sorted(p.name for p in rows.iterdir()))


if __name__ == '__main__':
    main(sys.argv[1])
