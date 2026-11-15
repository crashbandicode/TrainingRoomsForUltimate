"""Build the hide-cpu-tag boot's rows folder from integration-3's eggs plan (read-only source): its hooks, segments and
gates, with this lane's rows. usage: build_plan.py <out rows dir>"""
import json
import shutil
import sys
from pathlib import Path

SRC = Path('/home/intpa/smash-training-lobby/artifacts/phase23/rc14-int3/eden/plans/eggs/rows')
HERE = Path(__file__).resolve().parent
out = Path(sys.argv[1])
if out.exists():
    shutil.rmtree(out)
shutil.copytree(SRC, out, ignore=shutil.ignore_patterns('__pycache__'))
shutil.copy2(HERE / 'hidetag.py', out / 'hidetag.py')
plan = json.loads((SRC / 'rows.json').read_text())
by_id = {row['id']: row for row in plan['rows']}
rows = [by_id[i] for i in ('g-gdb-on', 'g-load', 'g-gdb-attached', 'g-training')]
rows.append({
    'id': 'ht-tag', 'start': 'training', 'end': 'training', 'start_gate': 'training-hud-unpaused',
    'group': 'rc14-hide-cpu-tag', 'budget_min': 10, 'on_failure': 'continue',
    'steps': [{'run': 'hidetag.py:tag_row', 'check': True}],
    'expect': 'Hide CPU On: the CPU, its CPU tag and its HUD panel are gone, P1 tag and panel stay (h04/h05); L+R+A (h07) '
              'and Cycle CPU (h08) keep all three hidden; Off brings body, tag and panel back (h09/h10); the shield pass '
              'the same (s2/s3). Lines: tag off/on and panel off/on for entry 1, no Player 1 panel write, no flash.',
})
rows.append({
    'id': 'ht-eggs', 'start': 'training', 'end': 'training', 'start_gate': 'training-hud-unpaused',
    'group': 'rc14-hide-cpu-tag', 'budget_min': 8, 'on_failure': 'continue',
    'steps': [{'run': 'eggshide.py:setup', 'check': True}, {'run': 'hidetag.py:eggs_row', 'check': True}],
    'expect': 'Eggs-ercise, CPU during a game Hide: the CPU, its tag and its panel are gone while the game runs (H1) and '
              'back after Off (H2); Show keeps them (H3); Target CPU never hidden (H4); Hide CPU On wins (H5), back after '
              'the restore. Lines as in ht-tag.',
})
rows.append(by_id['z-boot'])
plan['rows'] = rows
plan['scope'] = ('rc.14 hide-cpu-tag (11-15): a hidden CPU\'s tag and HUD panel follow its body. One instance, Training, '
                 'muted, GDB stub on. No card; nothing pushed.')
plan['setup'] = plan['setup'] + [{
    'state': 'Hide CPU and the practice game rows set by the diag live settings reload',
    'via': 'hook', 'ref': 'hidetag.py (wraps integration-3 hidecpu.h_shots and eggshide.run)'}]
(out / 'rows.json').write_text(json.dumps(plan, indent=1) + '\n')
print('rows', [r['id'] for r in rows], 'budget', sum(r.get('budget_min', 0) for r in rows))
