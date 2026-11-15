"""Build the wind-articles boot's rows folder from integration-3's eggs plan (read-only source: its boot rows, hooks, segments,
the hb-legend rig's hbl-seg and z-boot), with this lane's rows. usage: build_plan.py <out rows dir>"""
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
shutil.copy2(HERE / 'windart.py', out / 'windart.py')
plan = json.loads((SRC / 'rows.json').read_text())
by_id = {row['id']: row for row in plan['rows']}
rows = [by_id[i] for i in ('g-gdb-on', 'g-load', 'g-gdb-attached', 'g-training')]


def row(rid, fn, budget, expect):
    return {'id': rid, 'start': 'training', 'end': 'training', 'start_gate': 'training-hud-unpaused', 'group': 'rc14-wind-articles',
            'budget_min': budget, 'on_failure': 'continue',
            'steps': [{'run': 'windart.py:setup', 'check': True}, {'run': 'windart.py:%s' % fn, 'check': True},
                      {'run': 'windart.py:restore', 'check': True}],
            'expect': expect}


rows.append(row('w-krool', 'krool', 6, "Windboxes alone, 25 % stills: King K. Rool's Blunderbuss held (the vacuum's deep blue windbox in "
                "front of the gun) and Crownerang (a small deep blue windbox on his arm as he throws). Check: stills and a drawn-pill line."))
rows.append(row('w-articles', 'articles', 9, "Windboxes alone, 25 % stills: Samus's bomb, Ness's PK Fire pillar and Palutena's Explosive "
                "Flame each show the see-through deep blue wind area of the ARTICLE (tracked by the AREA_WIND_2ND* hooks). Check per take: "
                "a `[Legend] object ... areas=<n>` line with n > 0 and a drawn-pill line."))
rows.append(row('w-items', 'items', 9, "Settle rows: Sheik's Burst Grenade and R.O.B.'s Gyro are items; the item scan tracks them and the "
                "probe lines record their live slots (nr, kb) and wind areas. Check: stills and the item tracked; the lines are the finding."))
rows.append(by_id['z-boot'])
# Added mid-boot in windart-y1 (rows and hooks only): the vacuum's windbox lives only until the CPU is pulled in and caught, between the
# 25 % stills; at 10 % a still catches it.
rows.append(row('w-krool-v', 'krool_vacuum_slow', 4, "Retake at 10 %: King K. Rool's Blunderbuss vacuum windbox (deep blue, in front "
                "of the gun) in a still; it lives only until the CPU is pulled in and caught."))
plan['rows'] = rows
plan['scope'] = ('rc.14 wind-articles (11-15): articles\' and items\' wind areas under Windboxes, K. Rool\'s windboxes, Burst Grenade '
                 'and Gyro settled. One instance, Training, muted, GDB stub on. No card; nothing pushed.')
plan['setup'] = plan['setup'] + [{
    'state': 'Windboxes alone through the diag live settings reload; P1 King K. Rool, Samus, Ness, Palutena, Sheik and R.O.B. in turn',
    'via': 'hook', 'ref': 'windart.py (hb-legend rig: hblegend._fighter, hblfinal._burst; hbl-seg move segments)'}]
(out / 'rows.json').write_text(json.dumps(plan, indent=1) + '\n')
print('rows', [r['id'] for r in rows], 'budget', sum(r.get('budget_min', 0) for r in rows))
