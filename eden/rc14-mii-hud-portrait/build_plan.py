"""Build the mii-hud-portrait boot's rows folder from integration-3's eggs plan (read-only source): its hooks, segments and
gates, with this lane's rows. usage: build_plan.py <out rows dir> <repro|proof>"""
import json
import shutil
import sys
from pathlib import Path

SRC = Path('/home/intpa/smash-training-lobby/artifacts/phase23/rc14-int3/eden/plans/eggs/rows')
HERE = Path(__file__).resolve().parent
out, mode = Path(sys.argv[1]), sys.argv[2]
if mode not in ('repro', 'proof'):
    raise SystemExit('mode is repro or proof')
if out.exists():
    shutil.rmtree(out)
shutil.copytree(SRC, out, ignore=shutil.ignore_patterns('__pycache__'))
shutil.copy2(HERE / 'mhp.py', out / 'mhp.py')
(out / 'MHP_MODE').write_text(mode + '\n')
plan = json.loads((SRC / 'rows.json').read_text())
by_id = {row['id']: row for row in plan['rows']}
rows = [by_id[i] for i in ('g-gdb-on', 'g-load', 'g-gdb-attached', 'g-training')]
EXPECT = {
    'repro': ('Hide CPU Off: a Mii Gunner bust on every path. Hide CPU On: the bust is grey (colourful 0) and a '
              '`cpu_event role|hide entry=2|3` line falls between the render\'s issue and its end.'),
    'proof': ('Every case, Hide CPU Off or On: a Mii Gunner bust after Off (colourful >= 0.05) and no helper CPU line '
              'in the render window.'),
}[mode]
rows.append({'id': 'mhp-setup', 'start': 'training', 'end': 'training', 'start_gate': 'training-hud-unpaused',
             'group': 'rc14-mii-hud-portrait', 'budget_min': 4, 'on_failure': 'continue',
             'steps': [{'run': 'mhp.py:setup', 'check': True}],
             'expect': 'P1 Mario, CPU Pichu, lanes Mario > Mii Gunner and Pichu > Mii Gunner, Mii groups On, Hide CPU Off.'})
for rid, fn, who, path in (('mhp-cpu-warm', 'cpu_warm', 'CPU Pichu', 'the kept Mii group (warm)'),
                           ('mhp-p1-warm', 'p1_warm', 'Player 1 Mario', 'the kept Mii group (warm)'),
                           ('mhp-cpu-reload', 'cpu_reload', 'CPU Pichu', 'a Reload (Group Swap Off)'),
                           ('mhp-p1-reload', 'p1_reload', 'Player 1 Mario', 'a Reload (Group Swap Off)')):
    rows.append({'id': rid, 'start': 'training', 'end': 'training', 'start_gate': 'training-hud-unpaused',
                 'group': 'rc14-mii-hud-portrait', 'budget_min': 5, 'on_failure': 'continue',
                 'steps': [{'run': 'mhp.py:%s' % fn, 'check': True}],
                 'expect': '%s -> Mii Gunner by %s, Hide CPU Off then On, one HUD still each. %s' % (who, path, EXPECT)})
rows.append({'id': 'mhp-restore', 'start': 'training', 'end': 'training', 'start_gate': 'training-hud-unpaused',
             'group': 'rc14-mii-hud-portrait', 'budget_min': 2, 'on_failure': 'continue',
             'steps': [{'run': 'mhp.py:restore', 'check': True}], 'expect': 'Mario / Mario, Hide CPU Off.'})
rows.append(by_id['z-boot'])
plan['rows'] = rows
plan['scope'] = ('rc.14 mii-hud-portrait (11-15, %s): the Mii HUD portrait after a swap to Mii Gunner, Hide CPU Off and On, '
                 'CPU and Player 1, warm / cold / reload. One instance, Training, muted, GDB stub on. No card; nothing '
                 'pushed.' % mode)
plan['setup'] = plan['setup'] + [{
    'state': 'P1 Mario, CPU Pichu, both lanes ending in Mii Gunner, Mii groups On, Hide CPU Off (diag live settings reload)',
    'via': 'hook', 'ref': 'mhp.py:setup (gna_p2_fs._picks)'}]
(out / 'rows.json').write_text(json.dumps(plan, indent=1) + '\n')
print('rows', [r['id'] for r in rows], 'budget', sum(r.get('budget_min', 0) for r in rows), 'mode', mode)
