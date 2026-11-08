"""One window's rows from accept-rows.json, for a one-instance boot.

  split_accept_rows.py accept-rows.json accept-release-rows.json release acc-boot,acc-heap,acc-strip,acc-helper,acc-rel-record
  split_accept_rows.py accept-rows.json accept-diag-rows.json diag acc-boot,acc-play-cpu,acc-loop,acc-grade
"""
import json
import sys
from pathlib import Path

src, dst, side = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3]
order = sys.argv[4].split(',')
plan = json.loads(src.read_text())
rows = {row['id']: row for row in plan['rows']}
out = []
for rid in order:
    row = json.loads(json.dumps(rows[rid]))
    row['steps'] = [s for s in row['steps'] if s.get('on') == side]
    row['on'] = side
    if rid == 'acc-boot':
        row['build'] = side
        row['expect'] = 'The %s window leaves the intro and sits in Training on Mario. The shot is the check that Start was taken.' % side
    out.append(row)
plan['rows'] = out
plan['scope'] = 'rc.5 %s window, one instance. Generated from accept-rows.json (%s).' % (side, ', '.join(order))
dst.write_text(json.dumps(plan, indent=2) + '\n')
