#!/usr/bin/env python3
"""Build the rc.14 a-full-2 rows package (2026-11-14), read-only, the way eden/rc14-a1-enable's mk-rows.py does: the boot
rows, gates and segment pins (make_packages.py), the helpers, cycle-fix's cyclefix.py, a-full's afull.py and afull2.py
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
M = 'afull2.py'
ROWS = [
    ('a2-setup', M, 'a2_setup', 8, 'Live values (Ready Fighters 8, the console lanes), a P1 lap and a CPU lap, the notice settled; knob off.'),
    ('a2-k2', M, 'a2_k2', 10, "S1 / Gap 2: cycle-fix's cf-k2 (k = 2, Ready Fighters 6): no 'no free group slot', no Reload."),
    ('a2-direct', M, 'a2_direct', 4, "S1 / Gap 2: cycle-fix's cf-direct: a pick outside the cycle is served by a demand group."),
    ('a2-r6', M, 'a2_r6', 7, 'S1 / A1: the console P1 lane, Kirby alone, kept by the forced Reload (no slot refusal), warm return.'),
    ('a2-strip', M, 'a2_strip', 8, 'K-strip k2 + k5: Kirby with a hat, the press strips and parks him, the target warm; back to Kirby warm.'),
    ('a2-strip-same', M, 'a2_strip_same', 5, 'K-strip k3: a Pikachu hat, the press onto the Built Pikachu group.'),
    ('a2-strip-move', M, 'a2_strip_move', 5, 'K-strip k4: the press inside his copy special (B held).'),
    ('a2-restore', M, 'a2_restore', 3, "The knob off, cycle-fix's restore."),
    # Reloads out of native windows last: cycle-fix 23:47Z, the pins `settle(None)` drops on such a Reload (a1's crash).
    ('a2-pt', M, 'a2_pt', 10, 'Limit 2 PT p1-p4: the PT window kept by `reload window pt`, articles retired; warm back, rebuilt; laps; counts.'),
    ('a2-aegis-probe', M, 'a2_aegis_probe', 6, 'Probe a0 (diag only): a Pyra window across a forced Reload, the holder before and after the start.'),
]


def row(rid, module, fn, budget, expect):
    return {'id': rid, 'start': 'training', 'end': 'training', 'start_gate': 'training-hud-unpaused',
            'group': 'rc14-a-full-2', 'budget_min': budget, 'steps': [{'run': '%s:%s' % (module, fn), 'check': True}],
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
        {'state': 'Per row: diag-cycle.req `window` / `reload window` / `window strip` / `reload window pt` / `reload`, '
                  'the lanes and Ready Fighters by live settings; Kirby hats by the Kirby Hats shortcut',
         'via': 'hook', 'ref': 'afull2.py'}]
    setup[0] = dict(setup[0], state=setup[0]['state'] + '; TrainingRooms NRO swapped in by prep_run.py --tr-nro')
    return {'game_id': '01006A800016E000', 'start': 'frontend-ready', 'early_stop': False, 'on_failure': 'continue',
            'scope': 'rc.14 a-full-2: one boot, the boot rows, the a2-* rows (S1, K-strip, PT, probe a0), z-boot.',
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
