#!/usr/bin/env python3
"""a-full-2: same-boot retry rows for a run's rows.json. New ids (`<id>-<suffix>`) so the runner runs them although earlier
receipts exist; the failed originals named in --drop leave the list (their receipts stay in out/), so a retry runs only
the new rows. Usage:
    add-retry-rows.py <rows.json> <suffix> <id>[,<id>...] [--drop <id>[,<id>...]]"""
import json
import sys
from pathlib import Path

path, suffix, ids = Path(sys.argv[1]), sys.argv[2], sys.argv[3].split(',')
drop = sys.argv[sys.argv.index('--drop') + 1].split(',') if '--drop' in sys.argv else []
plan = json.loads(path.read_text())
rows = {r['id']: r for r in plan['rows']}
plan['rows'] = [r for r in plan['rows'] if r['id'] not in drop]
for rid in ids:
    row = dict(rows[rid])
    row['id'] = '%s-%s' % (rid, suffix)
    row['expect'] = '(same-boot retry, fixed hook) ' + row.get('expect', '')
    if row['id'] not in rows:
        plan['rows'].append(row)
path.write_text(json.dumps(plan, indent=1) + '\n')
print([r['id'] for r in plan['rows']])
