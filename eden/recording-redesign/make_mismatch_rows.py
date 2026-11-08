#!/usr/bin/env python3
"""mismatch-rows.json: the docs-GIF boot, then the Fighter Mismatch proof rows (mismatchproof.py) and its docs GIF."""
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent

plan = json.loads((HERE / 'docs-gif-rows.json').read_text())
boot = [row for row in plan['rows'] if row['id'] == 'docs-boot']
# Playbook 8218f34: an ROI-only reference with scale "fit" names the frame it was cut from (the library refs are 1280x720).
for gate in plan['gates'].values():
    if gate.get('scale') == 'fit':
        gate['frame'] = [1280, 720]


def row(row_id, steps, expect):
    return {'id': row_id, 'start': 'training', 'end': 'training', 'start_gate': 'training-hud-unpaused',
            'steps': steps, 'expect': expect, 'budget_min': 2}


plan['rows'] = boot + [
    row('mm-keep', [{'run': 'mismatchproof.py:keep', 'check': True}, {'run': 'docsgif.py:after', 'check': True}],
        'The question is drawn over the match with the cursor on Keep; A logs answer=keep and the chain plays on Mario.'),
    row('gif-fighter-mismatch', [
        {'run': 'mismatchproof.py:gif_setup', 'check': True},
        {'gif': {'name': 'rc5-fighter-mismatch', 'preset': 'docs', 'fps': 15, 'length': 7,
                 'steps': [{'run': 'mismatchproof.py:gif_clip'}]}},
        {'run': 'docsgif.py:after', 'check': True},
    ], 'docs/MEDIA.md: the Fighter Mismatch rows open, cursor on Keep; Keep plays the chain on Mario; no second prompt.'),
    row('mm-switch', [{'run': 'mismatchproof.py:switch', 'check': True},
                      {'run': 'mismatchproof.py:cpu_back_to_mario', 'check': True}],
        'Down then A logs answer=switch; the CPU becomes the recorded fighter.'),
]
plan['scope'] = ('rc.5 Fighter Mismatch proof on the XPS diag window (p23/rc5-mismatch-fix): the question is visible, '
                 'Keep and Switch each work, then the rc5-fighter-mismatch docs GIF.')
(HERE / 'mismatch-rows.json').write_text(json.dumps(plan, indent=2) + '\n')
print([r['id'] for r in plan['rows']])
