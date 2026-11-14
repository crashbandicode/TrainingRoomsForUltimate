#!/usr/bin/env python3
"""Build the rc.14 2e follow-up (a1-enable) rows package (2026-11-14), read-only, the way eden/rc14-a-full/mk-rows.py does:
the boot rows, gates and segment pins (make_packages.py), the helpers, cycle-fix's cyclefix.py, a-full's afull.py and a1.py
from this folder.

    /usr/bin/python3 -I mk-rows.py <out folder> [row,ids,to,skip]        # writes <out>/rows/"""
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
AF = HERE.parent / 'rc14-a-full' / 'afull.py'
M = 'a1.py'
ROWS = [
    ('a1-setup', 'a1.py', 'a1_setup', 8, 'Live values (Ready Fighters 8, the console lanes), a P1 lap and a CPU lap, the notice settled; A1 on (knob window).'),
    ('a1-regress', M, 'a1_regress', 9, 'A forced Reload keeps the CPU window; the warm attach back; three CPU laps: cold release follows, bodies stay '
                                       'within Ready Fighters + 1, no declined press, no Reload or teardown, at most Ready Fighters after.'),
    ('a1-kirby', M, 'a1_kirby', 7, 'r6: Kirby alone in his window, kept by the forced Reload, then the warm return at cell 0.'),
    ('a1-kirby-copy', M, 'a1_kirby_copy', 6, "r7: Ready Fighters 10, the P1 lane settled, Kirby with a hat: the press to Palutena takes the gated "
                                             "Reload; Kirby's window and Palutena's group go; the other Built groups are kept."),
    ('a1-mii', M, 'a1_mii', 7, 'Ready Fighters 10: the Mii Gunner window (not the cold fighter) kept across the forced Reload, then warm-attached again.'),
    ('a1-pair', M, 'a1_pair', 9, 'Ready Fighters 10, CPU Solo Popo Off: the Ice Climbers pair window (Popo and Nana) kept, then its warm return.'),
    ('a1-restore', M, 'a1_restore', 3, 'The knob off, the remembered values back, Mario / Mario.'),
]


def row(rid, module, fn, budget, expect):
    return {'id': rid, 'start': 'training', 'end': 'training', 'start_gate': 'training-hud-unpaused',
            'group': 'rc14-a1-enable', 'budget_min': budget, 'steps': [{'run': '%s:%s' % (module, fn), 'check': True}],
            'expect': expect}


def plan(skip=()):
    segs = mp.lib_segments()
    for name in SEGMENTS:
        path = LIB / 'segments' / (name + '.edenmacro.json')
        segs[name] = {'file': 'segments/%s.edenmacro.json' % name,
                      'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'cap': 90 if name.startswith('p2-fs-') else 30}
    setup = list(mp.BOOT_SETUP) + [
        {'state': 'Training, Ready Fighters 8, P1 original > Pikachu > Kirby > Palutena, CPU original > Mii Gunner > '
                  'Pichu > Diddy Kong > King K. Rool, every Loading row at the card value (its live values)',
         'via': 'hook', 'ref': 'cyclefix.py:cf_setup'},
        {'state': 'A1 on for the boot: diag-cycle.req `window`; `reload window` (`reload mii window`) for a forced Reload',
         'via': 'hook', 'ref': 'a1.py:_keep'}]
    setup[0] = dict(setup[0], state=setup[0]['state'] + '; TrainingRooms NRO swapped in by prep_run.py --tr-nro')
    return {'game_id': '01006A800016E000', 'start': 'frontend-ready', 'early_stop': False, 'on_failure': 'continue',
            'scope': 'rc.14 2e follow-up (a1-enable): one boot, the boot rows, the a1-* rows, z-boot; the be759456 diag '
                     'through prep_run --tr-nro.',
            'setup': setup, 'segments': segs, 'gates': mp.gates(),
            'recover': [{'run': 'steps.py:recover_training', 'check': True}], 'gif_presets': {},
            'rows': mp.boot_rows() + [row(*r) for r in ROWS if r[0] not in skip] + [mp.end_row()]}


def main(out, skip=()):
    rows = Path(out) / 'rows'
    if rows.exists():
        shutil.rmtree(rows)
    (rows / 'segments').mkdir(parents=True)
    for name in HELPERS:
        shutil.copyfile(LIB / name, rows / name)
    shutil.copyfile(CF, rows / 'cyclefix.py')
    shutil.copyfile(AF, rows / 'afull.py')
    shutil.copyfile(HERE / M, rows / M)
    for name in SEGMENTS:
        shutil.copyfile(LIB / 'segments' / (name + '.edenmacro.json'), rows / 'segments' / (name + '.edenmacro.json'))
    (rows / 'rows.json').write_text(json.dumps(plan(skip), indent=1) + '\n')
    print(rows, sorted(p.name for p in rows.iterdir()))


if __name__ == '__main__':
    main(sys.argv[1], tuple(sys.argv[2].split(',')) if len(sys.argv) > 2 else ())
