"""Boot rows for rc.14 cpu-target-flight: author-2's p2-staling-sbs boot rows (GDB stub on, load, Training) around the
cpuflight.py rows, as worker A's mk-dg-rows.py built the Eggs-ercise boots.

  python3 -I mk-rows.py <p2-staling-sbs rows.json> <stage rows.json> <diag sha8> <commit>"""
import copy
import json
import os
import sys

src, dst, tag, commit = sys.argv[1:5]
d = json.load(open(src))
by = {r['id']: r for r in d['rows']}
take = by['t-p2-staling-sbs']


def row(rid, fn, expect, budget, setup=False):
    r = copy.deepcopy(take)
    r['id'], r['budget_min'], r['expect'], r['group'] = rid, budget, expect, 'rc14-cpu-target-flight'
    r['steps'] = ([{'run': 'cpuflight.py:setup', 'check': True}] if setup else []) + [{'run': 'cpuflight.py:' + fn, 'check': True}]
    return r


rows = [
    row('cf-normal', 'flight_normal', 'Target CPU, Free practice, Normal: P1 Fox; the CPU placed afresh in the air (FALL) at the '
        'first spot, stills in flight and at rest; its flight against the model from the [Eggs] cpu lines: drift and path '
        'error <= 0.5, apex 21.6 (or the view cap), landing at tick 96, WAIT on the ground after.', 4, setup=True),
    row('cf-slow', 'flight_slow', 'The same at Slow: apex at 60, landing at tick 192.', 3),
    row('cf-fast', 'flight_fast', 'The same at Fast: apex at 20, landing at tick 64.', 3),
    row('cf-groundhit', 'groundhit', 'Normal, Any attack: the CPU launched 7 in front of Fox (diag-egg.req); after it lands, a '
        'jab counts (score 1) with the CPU on the ground; the next spot is placed afresh in the air.', 3),
    row('cf-airhit', 'airhit', 'Slow, Breaks on Aerials: the CPU launched beside Fox; a full-hop nair counts with the CPU '
        'airborne (situation 2); the next spot is placed afresh in the air and rises.', 5),
    row('cf-done10', 'done10', '10 targets: ten launches beside Fox, each jabbed after it lands; the notice reads "Done! 10 in".',
        6),
    row('cf-reset', 'reset', 'L+R+A while the CPU flies: "Eggs 0" again, a new first target placed afresh in the air that '
        'flies to rest by the model.', 3),
]
d['rows'] = [by[i] for i in ('g-gdb-on', 'g-load', 'g-gdb-attached', 'g-training')] + rows + [by['z-boot']]
base = os.path.dirname(os.path.abspath(dst))
dropped = [k for k, v in d['segments'].items() if not os.path.exists(os.path.join(base, v['file']))]
d['segments'] = {k: v for k, v in d['segments'].items() if k not in dropped}
print('segments dropped (not in this stage):', dropped)
d['setup'] = list(d.get('setup', [])) + [
    {'state': 'Training, P1 Fox', 'via': 'hook', 'ref': 'cpuflight.py:setup (eggsrows._fighter, the player_character live set)'},
    {'state': 'Eggs-ercise rows (mode, Breaks on, Target speed, Target CPU) per row', 'via': 'hook',
     'ref': 'cpuflight.py:_set (fx._live_set), then the real menu opened and closed'},
    {'state': 'the CPU target launched beside Fox', 'via': 'hook', 'ref': 'eggsrows._place (diag-egg.req)'}]
d['scope'] = ('rc.14 lane cpu-target-flight (Opus; the operator 11-15 ~02:20Z: the CPU target flies like the discs) on the branch '
              'diag %s of %s (rc14/cpu-target-flight off rc14/integration 79ba5ba9); author-2 p2-staling-sbs boot rows; one '
              'instance, GDB stub on, muted.' % (tag, commit))
json.dump(d, open(dst, 'w'), indent=1)
print([r['id'] for r in d['rows']])
