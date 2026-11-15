#!/usr/bin/env python3
"""Build the rc.14 mem-headroom rows package (2026-11-15), read-only, the way eden/rc14-a-full-2's mk-rows.py does: the boot
rows, gates and segment pins (make_packages.py), the helpers, cycle-fix's cyclefix.py, a-full's afull.py and
memheadroom.py from this folder. One short boot: the boot rows, mh-pyra-pause, mh-restore, z-boot.

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
AF = HERE.parent / 'rc14-a-full' / 'afull.py'
M = 'memheadroom.py'
ROWS = [
    ('mh-pyra-pause', M, 'mh_pyra_pause', 8, "The operator's Loading settings (Ready Fighters 8, Bodies 3, Cycle Preload 5, Ready Hats 5), "
     "Kirby's hats filling the created helpers, Pyra / Mythra into the CPU lane held for memory; the give-back marks, "
     "'load after you pause'; one pause retires the idle helpers and the Pyra / Mythra group builds; census lines, no "
     'StallWatch, no Unmapped.'),
    ('mh-restore', M, 'mh_restore', 3, "The reserve 0, the knob off, cycle-fix's restore."),
]


def row(rid, module, fn, budget, expect):
    return {'id': rid, 'start': 'training', 'end': 'training', 'start_gate': 'training-hud-unpaused',
            'group': 'rc14-mem-headroom', 'budget_min': budget, 'steps': [{'run': '%s:%s' % (module, fn), 'check': True}],
            'expect': expect}


def plan():
    segs = mp.lib_segments()
    for name in SEGMENTS:
        path = LIB / 'segments' / (name + '.edenmacro.json')
        segs[name] = {'file': 'segments/%s.edenmacro.json' % name,
                      'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'cap': 90 if name.startswith('p2-fs-') else 30}
    setup = list(mp.BOOT_SETUP) + [
        {'state': "Training, the operator's Loading settings (Ready Fighters 8, Ready Fighter Bodies 3, Cycle Preload 5, Ready "
                  "Hats 5), P1 Pikachu > Kirby > Palutena with Kirby's hats, CPU Mii Gunner > (Pyra) > Diddy Kong",
         'via': 'hook', 'ref': 'memheadroom.py:mh_pyra_pause'},
        {'state': 'diag-memory-reserve.req when the process would hold the Pyra / Mythra group without the helpers\' room',
         'via': 'hook', 'ref': 'memheadroom.py:_reserve'}]
    setup[0] = dict(setup[0], state=setup[0]['state'] + '; TrainingRooms NRO swapped in by prep_run.py --tr-nro')
    return {'game_id': '01006A800016E000', 'start': 'frontend-ready', 'early_stop': False, 'on_failure': 'continue',
            'scope': 'rc.14 mem-headroom: one short boot, the boot rows, mh-pyra-pause, mh-restore, z-boot.',
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
    shutil.copyfile(AF, rows / 'afull.py')
    shutil.copyfile(HERE / M, rows / M)
    for name in SEGMENTS:
        shutil.copyfile(LIB / 'segments' / (name + '.edenmacro.json'), rows / 'segments' / (name + '.edenmacro.json'))
    (rows / 'rows.json').write_text(json.dumps(plan(), indent=1) + '\n')
    print(rows, sorted(p.name for p in rows.iterdir()))


if __name__ == '__main__':
    main(sys.argv[1])
