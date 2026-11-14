#!/usr/bin/env python3
"""Build the rc.14 cycle-fix rows package (2026-11-14) from the option-GIF takes framework, read-only: its boot rows,
gates and segment pins (make_packages.py), the helpers beside rows.json, and cyclefix.py from this folder.

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
M = 'cyclefix.py'
ROWS = [
    ('cf-setup', 'cf_setup', 8, 'Live values (RF 8, the console lanes), a P1 lap and a CPU lap, the notice settled.'),
    ('cf-rf8-laps', 'cf_rf8_laps', 14, 'Five patient CPU laps and two fast ones at Ready Fighters 8: after the first lap '
                                       'no Reload, no teardown, the same cold fighter each lap, the rest warm.'),
    ('cf-rf9', 'cf_rf9', 6, 'Ready Fighters 9: the lap fits, all warm; headroom with King K. Rool on screen.'),
    ('cf-k1-setup', 'cf_k1_setup', 5, 'k = 1 with the CPU short: P1 Mario > Pikachu, the CPU lap unchanged, Ready Fighters 6.'),
    ('cf-k1-laps', 'cf_k1_laps', 12, 'Five patient and two fast CPU laps: one cold load per lap, the same fighter, no Reload.'),
    ('cf-k1-fits', 'cf_k1_fits', 5, 'Ready Fighters 7: the lap fits, all warm; the headroom for the memory comparison.'),
    ('cf-restore', 'cf_restore', 3, 'The remembered values back, Mario / Mario.'),
]
# Phase 2 (2a-2d): the same setup, then the direct picks, the forced Reload, the rc.12 Mii crash's fail-first row last.
BY_ID = {r[0]: r for r in ROWS}
ROWS2 = [BY_ID['cf-setup'], BY_ID['cf-k1-setup'], BY_ID['cf-k1-laps'], BY_ID['cf-k1-fits'],
         ('cf-direct', 'cf_direct', 5, 'Direct CPU Character picks (King K. Rool, then Bowser) take no Reload; Bowser, '
                                       'outside the cycle, is a demand group.'),
         ('cf-k2', 'cf_k2', 8, 'k = 2: a 6-member CPU lap at Ready Fighters 8 loads the same two cold fighters each lap.'),
         ('cf-forced-reload', 'cf_forced_reload', 4, 'A forced target-first Reload keeps the Built ordinary groups.'),
         ('cf-mii-failfirst', 'cf_mii_failfirst', 4, 'The rc.12 crash: a kept Mii group warm-attached after the Reload.'),
         BY_ID['cf-restore']]


def row(rid, fn, budget, expect):
    return {'id': rid, 'start': 'training', 'end': 'training', 'start_gate': 'training-hud-unpaused',
            'group': 'rc14-cycle-fix', 'budget_min': budget, 'steps': [{'run': '%s:%s' % (M, fn), 'check': True}],
            'expect': expect}


def plan(rows):
    segs = mp.lib_segments()
    for name in SEGMENTS:
        path = LIB / 'segments' / (name + '.edenmacro.json')
        segs[name] = {'file': 'segments/%s.edenmacro.json' % name,
                      'sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'cap': 90 if name.startswith('p2-fs-') else 30}
    setup = list(mp.BOOT_SETUP) + [
        {'state': 'Training, Ready Fighters 8, P1 original > Pikachu > Kirby > Palutena, CPU original > Mii Gunner > '
                  'Pichu > Diddy Kong > King K. Rool, every Loading row at the card value (its live values)',
         'via': 'hook', 'ref': 'cyclefix.py:cf_setup'}]
    setup[0] = dict(setup[0], state=setup[0]['state'] + '; TrainingRooms NRO swapped in by prep_run.py --tr-nro')
    return {'game_id': '01006A800016E000', 'start': 'frontend-ready', 'early_stop': False, 'on_failure': 'continue',
            'scope': 'rc.14 cycle fix (Design A): one boot, the boot rows, the cf-* rows, z-boot; the fix diag through '
                     'prep_run --tr-nro.',
            'setup': setup, 'segments': segs, 'gates': mp.gates(),
            'recover': [{'run': 'steps.py:recover_training', 'check': True}], 'gif_presets': {},
            'rows': mp.boot_rows() + [row(*r) for r in rows] + [mp.end_row()]}


def main(out, phase='1'):
    rows = Path(out) / 'rows'
    if rows.exists():
        shutil.rmtree(rows)
    (rows / 'segments').mkdir(parents=True)
    for name in HELPERS:
        shutil.copyfile(LIB / name, rows / name)
    shutil.copyfile(HERE / M, rows / M)
    for name in SEGMENTS:
        shutil.copyfile(LIB / 'segments' / (name + '.edenmacro.json'), rows / 'segments' / (name + '.edenmacro.json'))
    (rows / 'rows.json').write_text(json.dumps(plan(ROWS2 if phase == '2' else ROWS), indent=1) + '\n')
    print(rows, sorted(p.name for p in rows.iterdir()))


if __name__ == '__main__':
    main(*sys.argv[1:3])
