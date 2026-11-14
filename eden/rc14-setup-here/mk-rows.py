"""save-here lane: the stage's rows.json (worker A's card stage: g-gdb-on, g-load, g-gdb-attached, g-training,
t-hbprobe, z-boot) with t-hbprobe replaced by t-setup-here (setuphere.py)."""
import copy
import json
import os
import sys

dst, tag, commit = sys.argv[1], sys.argv[2], sys.argv[3]
d = json.load(open(dst))
rows = []
for r in d['rows']:
    if r['id'] != 't-hbprobe':
        rows.append(r)
        continue
    t = copy.deepcopy(r)
    t['id'], t['group'], t['budget_min'] = 't-setup-here', 'rc14-setup-here', 15
    t['steps'] = [{'run': 'setuphere.py:setup', 'check': True}, {'run': 'setuphere.py:run', 'check': True},
                  {'run': 'setuphere.py:restore', 'check': True}]
    t['expect'] = ('rc.14 SL4: P1 Mario vs CPU Mario, fixture cov-replay (bound save jab-start). S1 Load; S2 fsmash and a walk, '
                   'diag-pos.req reads both spots; S3 Saves > Save setup here..., "Here spot": a new setups/ file with the live '
                   'spots, CPU damage and stage, save_states.json and jab-start.json unchanged; S4 Load still goes to P1 x -5; '
                   'S5 Setup library A on Here spot: both placed at its spots; S6 Save States Off: nothing kept, the help line '
                   'shows "Save States is Off (Saves page)"; S7 Cycle CPU to the Mii Gunner: "Here mii" keeps kind 0x4a with '
                   "the exact Mii Save keeps; A restores it.")
    rows.append(t)
if [r['id'] for r in rows] != ['g-gdb-on', 'g-load', 'g-gdb-attached', 'g-training', 't-setup-here', 'z-boot']:
    raise SystemExit('unexpected rows: %s' % [r['id'] for r in rows])
d['rows'] = rows
d['setup'] = [s for s in d['setup'] if 'p2-staling-sbs' not in s['state']] + [
    {'state': '[t-setup-here] bound save jab-start (P1 x -5, CPU x 5) and the setup RC5 Mario corners on the card',
     'via': 'fixture', 'ref': 'fixtures/cov-replay (cov.prep)'},
    {'state': '[t-setup-here] the CPU a Mii Gunner', 'via': 'segment', 'ref': 'p2-fs-cpu-cycle (the card cpu_roster cycle)'},
    {'state': '[t-setup-here] Save States Off for S6', 'via': 'fixture', 'ref': 'diag-settings-next.json (fx._live_set)'}]
base =os.path.dirname(os.path.abspath(dst))
missing = [k for k, v in d['segments'].items() if not os.path.exists(os.path.join(base, v['file']))]
for need in ('chord-load', 'chord-save', 'fsmash', 'p2-fs-cpu-cycle', 'menu-open', 'menu-close', 'reset-center'):
    if need not in d['segments'] or need in missing:
        raise SystemExit('segment %s missing from the stage' % need)
d['scope'] = ('save-here lane, rc.14 SL4 Save setup here (the primary 2026-11-14, XPS slot 1) on the branch diag %s of %s '
              '(rc14/setup-here); worker A card stage boot rows; one instance, GDB stub on, muted.' % (tag, commit))
json.dump(d, open(dst, 'w'), indent=1)
print([r['id'] for r in rows], 'segments without a file:', missing)
