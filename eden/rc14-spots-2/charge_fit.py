#!/usr/bin/env python3
"""spots-2: fit the damage a smash hit added against the AttackModule floats the diag build's `[SpotsProbe]` lines carry.

    python3 -I charge_fit.py <charge-probe.json | practice-diagnostics.log>... [--json out.json]

Input is a row's `charge-probe.json` (swings with their kept lines) or a raw diag log. For each swing:
* `hold`: the frames P1 stayed in the smash-hold status (0x2e), read off the probe's frame counter (first line of 0x2e to the first
  line of 0x2f);
* `fields`: P1's raw module floats on the first probe line of the active status (and every distinct set seen while active);
* `gained`, `powers`: the `[Spots] stop` line of the hit (the damage the defender gained and the attacker's active slot damages).
Then, per charge level, the product of the candidate floats and `gained / (power x product)`: the right divisor leaves the same
residual (1.2 for 1v1 times the move's freshness, 1.05 fresh down to about 0.89 stalest) at every charge.
"""
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

PROBE = re.compile(r'\[SpotsProbe\] frame=(\d+) who=(\w+) status=(0x[0-9a-f]+) stop=(\w+) active=(\w+) mf=(\S+) damage=(\S+) other_damage=(\S+) '
                   r'powers=\[([^\]]*)\] fields: (.*)$')
STOP = re.compile(r'\[Spots\] stop frame=(\d+) gained=(\S+)(?: mul=(\S+))? powers=\[([^\]]*)\]')
FIELD = re.compile(r'(0x[0-9a-f]+)=(\S+)')


def parse_fields(text):
    return {int(k, 16): float(v) for k, v in FIELD.findall(text)}


def parse_line(text):
    m = PROBE.search(text)
    if m:
        return {'kind': 'probe', 'frame': int(m.group(1)), 'who': m.group(2), 'status': int(m.group(3), 16), 'stop': m.group(4) == 'true',
                'active': m.group(5) == 'true', 'mf': float(m.group(6)), 'damage': float(m.group(7)), 'other': float(m.group(8)),
                'powers': [float(v) for v in m.group(9).split(',')], 'fields': parse_fields(m.group(10))}
    m = STOP.search(text)
    if m:
        gained = m.group(2)
        return {'kind': 'stop', 'frame': int(m.group(1)), 'gained': float(gained) if gained not in ('NaN', 'inf') else 0.0,
                'mul': float(m.group(3)) if m.group(3) else None, 'powers': [float(v) for v in m.group(4).split(',')]}
    return None


def swing_summary(lines):
    """One swing's facts from its parsed lines (see the module notes)."""
    probes = [p for p in lines if p['kind'] == 'probe' and p['who'] == 'p1']
    stops = [p for p in lines if p['kind'] == 'stop']
    hold_first = next((p['frame'] for p in probes if p['status'] == 0x2e), None)
    attack_first = next((p['frame'] for p in probes if p['status'] == 0x2f), None)
    hold = attack_first - hold_first if hold_first is not None and attack_first is not None else 0
    active = [p for p in probes if p['active']]
    distinct = []
    for p in active:
        if p['fields'] not in distinct:
            distinct.append(p['fields'])
    return {'hold': hold, 'fields_active': distinct, 'fields_first_active': active[0]['fields'] if active else None,
            'stops': [{'frame': s['frame'], 'gained': s['gained'], 'mul': s['mul'], 'powers': s['powers']} for s in stops]}


def read_input(path):
    path = Path(path)
    if path.suffix == '.json':
        data = json.loads(path.read_text())
        return [(s['hold'], s['walk'], s['round'], [p for p in map(parse_line, s['lines']) if p]) for s in data['swings']]
    parsed = [p for p in map(parse_line, path.read_text(errors='replace').splitlines()) if p]
    return [(None, None, 0, parsed)]


def field_table(swings):
    """For every module offset: the distinct values seen on active frames per hold level."""
    table = defaultdict(lambda: defaultdict(set))
    for hold, _, _, lines in swings:
        s = swing_summary(lines)
        for fields in s['fields_active']:
            for off, value in fields.items():
                table[off][s['hold']].add(value)
    return table


def main(argv):
    out_json = None
    if '--json' in argv:
        i = argv.index('--json')
        out_json = argv[i + 1]
        argv = argv[:i] + argv[i + 2:]
    swings = [s for a in argv for s in read_input(a)]
    result = {'swings': [], 'fields': {}}
    for hold_planned, walk, rnd, lines in swings:
        s = swing_summary(lines)
        s.update(planned_hold=hold_planned, walk=walk, round=rnd)
        result['swings'].append(s)
        print('hold planned=%s measured=%s walk=%s round=%s' % (hold_planned, s['hold'], walk, rnd))
        for stop in s['stops']:
            print('   hit frame=%d gained=%.3f mul=%s powers=%s' % (stop['frame'], stop['gained'], stop['mul'], stop['powers']))
        if s['fields_first_active']:
            print('   fields on the first active frame: ' + ' '.join('%#x=%g' % kv for kv in sorted(s['fields_first_active'].items())))
            e0 = s['fields_first_active'].get(0xe0)
            print('   0xe0=%g; 1 + min(hold, 60) / 150 = %g' % (e0, 1 + min(s['hold'], 60) / 150))
    table = field_table(swings)
    print('\nOffsets whose active-frame values differ between charge levels:')
    for off in sorted(table):
        per_hold = {h: sorted(v) for h, v in table[off].items()}
        flat = {tuple(v) for v in per_hold.values()}
        if len(flat) > 1:
            print('  %#x: %s' % (off, {h: per_hold[h] for h in sorted(per_hold)}))
            result['fields'][hex(off)] = {str(h): per_hold[h] for h in sorted(per_hold)}
    if out_json:
        Path(out_json).write_text(json.dumps(result, indent=1) + '\n')


if __name__ == '__main__':
    main(sys.argv[1:])
