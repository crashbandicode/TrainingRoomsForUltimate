"""recfix lane: a stage's rows.json (worker A's card stage: g-gdb-on, g-load, g-gdb-attached, g-training, t-hbprobe,
z-boot) with t-hbprobe replaced by t-recfix (recfix.py). Same shape as eden/rc14-setup-here/mk-rows.py.
Usage: python3 mk-rows.py <stage rows.json> <diag nro tag> <commit>"""
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
    t['id'], t['group'], t['budget_min'] = 't-recfix', 'rc14-recfix', 10
    t['steps'] = [{'run': 'recfix.py:setup', 'check': True}, {'run': 'recfix.py:run', 'check': True},
                  {'run': 'recfix.py:restore', 'check': True}]
    t['expect'] = ('rc.14 recfix: R1 Start save On, Play warps once; R2 the macro screen Start save Left: the file says '
                   'start_save_on false, Play plays with no warp=1; R3 Right: On, Play warps once; R4 P1 CPU Control On (no '
                   'pilot plugin): Record shows "P1 is controlled by an AI: recording is off" and writes nothing, Play on '
                   'Player 1 shows "...Play on Player 1 is off" and does not play; R5 Off again: Record writes a macro.')
    rows.append(t)
if [r['id'] for r in rows] != ['g-gdb-on', 'g-load', 'g-gdb-attached', 'g-training', 't-recfix', 'z-boot']:
    raise SystemExit('unexpected rows: %s' % [r['id'] for r in rows])
d['rows'] = rows
d['setup'] = [s for s in d['setup'] if 'p2-staling-sbs' not in s['state']] + [
    {'state': '[t-recfix] a macro recorded from the reset spot (its start save is made by Record)', 'via': 'record',
     'ref': 'recfix.py:setup (gna_p2_rec._record, one bounded recording)'},
    {'state': '[t-recfix] P1 CPU Control On, then Off', 'via': 'hook', 'ref': 'recfix.py:run (cov._page_row, A on the row)'}]
base = os.path.dirname(os.path.abspath(dst))
missing = [k for k, v in d['segments'].items() if not os.path.exists(os.path.join(base, v['file']))]
for need in ('menu-open', 'menu-close', 'reset-center'):
    if need not in d['segments'] or need in missing:
        raise SystemExit('segment %s missing from the stage' % need)
d['scope'] = ('recfix lane, rc.14 Start save switch and P1 CPU Control refusal (the primary 2026-11-14) on the integration '
              'diag %s of %s; one instance, GDB stub on, muted. No pilot plugin pinned.' % (tag, commit))
json.dump(d, open(dst, 'w'), indent=1)
print([r['id'] for r in rows], 'segments without a file:', missing)
