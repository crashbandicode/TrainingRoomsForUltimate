#!/usr/bin/env python3
"""spots-2: build the Eden rows folder of the probe / final boots from integration-3's `plans/spots/rows` (the checklist-adapted spots rig).

    python3 -I port_spots2_rows.py <plan> <out rows dir>        plan: probe | final

Copies integration-3's rows folder, applies the spots-2 edits to `spots_rows.py` and `make_spots_segments.py` (the charged-smash segments, the
`mul=` field of the `[Spots] stop` line, the oracle's multiplier, the `sp_charge_probe` hook), regenerates the segments and writes `rows.json`:
`probe` is the boot chain plus `sp-charge-probe`; `final` keeps integration-3's spots rows (the falcon knee included) and adds `sp-charge`.
"""
import json
import shutil
import subprocess
import sys
from pathlib import Path

INT3 = Path('/home/intpa/smash-training-lobby/artifacts/phase23/rc14-int3/eden/plans/spots/rows')
HERE = Path(__file__).resolve().parent


def edit(path, pairs):
    text = path.read_text()
    for old, new in pairs:
        assert old in text, (path.name, old[:80])
        text = text.replace(old, new, 1)
    path.write_text(text)


def port(out):
    if out.exists():
        shutil.rmtree(out)
    shutil.copytree(INT3, out, ignore=shutil.ignore_patterns('__pycache__'))
    rows = out / 'spots_rows.py'
    edit(rows, [
        ("STOP = re.compile(r'\\[Spots\\] stop frame=(\\d+) gained=(\\S+) powers=\\[([^\\]]*)\\] newest_mark=(?:None|Some\\(Mark \\{ frame: (\\d+), lane: (\\d), spot: (\\w+) \\}\\))')",
         "STOP = re.compile(r'\\[Spots\\] stop frame=(\\d+) gained=(\\S+)(?: mul=(\\S+))? powers=\\[([^\\]]*)\\] newest_mark=(?:None|Some\\(Mark \\{ frame: (\\d+), lane: (\\d), spot: (\\w+) \\}\\))')"),
        ("def _classes(gained, powers, earlier=0.0):\n    \"\"\"The oracle: the classes of the slots that could have dealt `gained` (ratio in the window), from the SPEC's rules.\"\"\"",
         "def _classes(gained, powers, earlier=0.0, mul=1.0):\n    \"\"\"The oracle: the classes of the slots that could have dealt `gained` (damage x multiplier x ratio in the window), from the SPEC's rules.\n    `mul` is the attacker's smash charge multiplier (spots-2); 1.0 for an uncharged hit.\"\"\""),
        ("    return {one(p) for p in live if K_LOW <= gained / p <= K_HIGH}", "    return {one(p) for p in live if K_LOW <= gained / (p * mul) <= K_HIGH}"),
        ("        powers = [float(v) for v in m.group(3).split(',')]\n        out.append({'frame': int(m.group(1)), 'gained': gained, 'powers': powers,\n                    'mark': (int(m.group(4)), int(m.group(5)), m.group(6)) if m.group(6) else None})",
         "        powers = [float(v) for v in m.group(4).split(',')]\n        out.append({'frame': int(m.group(1)), 'gained': gained, 'powers': powers, 'mul': float(m.group(3)) if m.group(3) else 1.0,\n                    'mark': (int(m.group(5)), int(m.group(6)), m.group(7)) if m.group(7) else None})"),
        ("    classes = _classes(hit['gained'], hit['powers'])", "    classes = _classes(hit['gained'], hit['powers'], mul=hit.get('mul_oracle', hit['mul']))"),
    ])
    rows.write_text(rows.read_text().rstrip() + '\n' + (HERE / 'sp_charge_hooks.py').read_text())
    # make_btt_segments.py is in integration-3's rows folder; make_spots_segments.py (here) carries the charged-smash and early forward-air segments.
    shutil.copy(HERE / 'make_spots_segments.py', out / 'make_spots_segments.py')
    shutil.copy(HERE / 'charge_fit.py', out / 'charge_fit.py')
    subprocess.run([sys.executable, '-I', 'make_spots_segments.py'], cwd=out, check=True)


def rows_json(out, plan):
    data = json.loads((out / 'rows.json').read_text())
    entries = json.loads((out / 'segments-spots.json').read_text())
    data['segments'].update(entries)
    chain = [r for r in data['rows'] if r['id'] in ('g-gdb-on', 'q00-load', 'g-gdb-attached', 'm-main', 'p-route')]
    assert len(chain) == 5
    marth, mario = 21, 0
    charge = {
        'id': 'sp-charge-probe' if plan == 'probe' else 'sp-charge', 'start': 'training', 'end': 'training', 'start_gate': 'training-hud-unpaused',
        'group': 'rc14-spots-2', 'on_failure': 'continue', 'budget_min': 14,
        'steps': [{'run': 'spots_rows.py:sp_setup', 'on': 'diag', 'check': True, 'args': {'p1': marth, 'cpu': mario, 'frame_data': False, 'labels': True}},
                  {'run': 'spots_rows.py:%s' % ('sp_charge_probe' if plan == 'probe' else 'sp_charge'), 'on': 'diag', 'check': True, 'args': {'holds': [0, 30, 66], 'walks': [8, 28], 'rounds': 2}}],
        'expect': 'Marth vs Mario, spot labels On: forward smash held 0, 30 and 66 extra C-stick frames at a tip (8) and a blade (28) spacing, twice. The diag '
                  'build\'s [SpotsProbe] lines (raw AttackModule floats), [Spots] stop and the strip lines of every swing are kept for charge_fit.py.',
    }
    if plan == 'probe':
        data['rows'] = chain + [charge]
        data['scope'] = ('rc.14 spots-2 probe (the primary 2026-11-14, plan approved): the diag build of rc14/spots-2 with the [SpotsProbe] line, the card set and boot '
                         'chain of the integration-3 spots plan, Training only, muted (volume 0), the GDB stub on before game.load. Settings change through the diag '
                         'reload file. No card; nothing pushed.')
    else:
        edge = next(r for r in data['rows'] if r['id'] == 'sp-falcon-edge')
        edge['expect'] = edge['expect'] + ' (rerun on the spots-2 build; the sp-knee row before it sets Falcon up and tries the early forward air.)'
        knee = {'id': 'sp-knee', 'start': 'training', 'end': 'training', 'start_gate': 'training-hud-unpaused', 'group': 'rc14-spots-2', 'on_failure': 'continue',
                'budget_min': 8, 'steps': [{'run': 'spots_rows.py:sp_knee', 'on': 'diag', 'check': True}],
                'expect': 'Captain Falcon vs Mario: the forward air with the stick 4, 5 and 6 frames after the short hop, at six spacings; every label agrees with the damage '
                          'oracle and the knee\'s Sweet and Sour hits each land at least once.'}
        slow = {'id': 'sp-knee-slow', 'start': 'training', 'end': 'training', 'start_gate': 'training-hud-unpaused', 'group': 'rc14-spots-2', 'on_failure': 'continue',
                'budget_min': 4, 'steps': [{'run': 'spots_rows.py:sp_knee_slow', 'on': 'diag', 'check': True}],
                'expect': 'Captain Falcon vs Mario at Game speed 1/4 (how the first boot landed the knee): sp-fair at five spacings around 112; every label agrees with the '
                          'damage oracle and the knee\'s Sweet and Sour hits each land at least once.'}
        data['rows'] = chain + [charge, slow, knee, edge]
    data['setup'] = data['setup'] + [{'state': 'the charged smashes', 'via': 'segment',
                                      'ref': 'segments/sp-csmash-h30, sp-csmash-h66 (make_spots_segments.py, sha256 pinned in rows.json)'}]
    (out / 'rows.json').write_text(json.dumps(data, indent=1) + '\n')
    print(plan, len(data['rows']), 'rows', len(data['segments']), 'segments')


if __name__ == '__main__':
    plan, out = sys.argv[1], Path(sys.argv[2])
    assert plan in ('probe', 'final')
    port(out)
    rows_json(out, plan)
