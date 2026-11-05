"""Regenerate rows-hb-ab/rows.json (make_rows.py) and append the ab4 rows, each starting where the previous row ends."""
import json
import subprocess
from pathlib import Path

ROWS = Path('/home/intpa/smash-training-lobby/tmp-p23/hitbox-orbs/eden/phase23/rows-hb-ab')
subprocess.run(['/usr/bin/python3', '-I', str(ROWS / 'make_rows.py')], check=True)
p = ROWS / 'rows.json'
j = json.loads(p.read_text())
# The segments the ab3 rows added by hand.
import hashlib  # noqa: E402
for name in ('hb-zl-a', 'hb-zl-shieldgrab'):
    f = ROWS / 'segments' / f'{name}.edenmacro.json'
    j['segments'][name] = {'file': f'segments/{f.name}', 'sha256': hashlib.sha256(f.read_bytes()).hexdigest(), 'cap': 20}


def add(rid, hook, args, end=None):
    prev = j['rows'][-1]['end']
    j['rows'].append({'id': rid, 'start': prev, 'end': end or prev, 'budget_min': 3,
                      'steps': [{'run': f'hb_steps.py:{hook}', 'check': True, 'args': args}]})


add('30-prepare-vs', 'prepare_vs', {'cpu': 63, 'cpu_name': 'Ridley'})
add('31-freeze', 'menu_route', {'seq': ['hb-plus', 'hb-down', 'hb-down', 'hb-down', 'hb-down', 'hb-down', 'hb-a', 'hb-right',
                                        'hb-b', 'hb-b'], 'tag': 'frozen', 'settle': 0.8})
add('32-benefit', 'benefit', {'tag': 'benefit'})
p.write_text(json.dumps(j, indent=2) + '\n')
print([r['id'] for r in j['rows']])
