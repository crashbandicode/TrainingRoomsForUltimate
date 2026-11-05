"""Append or replace one row in the live ab3 rows.json; its start is the previous row's end (labels only).
Usage: add_row.py <id> <hook> '<args json>' [end-label]"""
import json
import sys
from pathlib import Path

p = Path('/home/intpa/eden-runs/tr-p23-hitbox-ab3/rows/rows.json')
j = json.loads(p.read_text())
rid, hook, args = sys.argv[1], sys.argv[2], json.loads(sys.argv[3])
j['rows'] = [r for r in j['rows'] if r['id'] != rid]
prev = j['rows'][-1]['end']
end = sys.argv[4] if len(sys.argv) > 4 else prev
j['rows'].append({'id': rid, 'start': prev, 'end': end, 'budget_min': 3,
                  'steps': [{'run': f'hb_steps.py:{hook}', 'check': True, 'args': args}]})
p.write_text(json.dumps(j, indent=2) + '\n')
print(rid, prev, '->', end)
