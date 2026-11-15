"""rc.14 cpu-target-flight: the CPU target's flight, read from the diag build's `[Eggs] cpu` lines (pure; host-tested by
test_cfcheck.py).

One line per event (a placement afresh, a landing, a counted hit) and every 6th held tick:

  [Eggs] cpu damage=<d> score=<n> move=<None | Some(([x, y], Hold | Fresh(b) | Land))> want=<None | Some([x, y])>
         at=[x, y] egg=<None | Some([x, y])> age=<n> fly=[v, vx, g] status=<s> fall=<id> wait=<id> stop=<b>

`want` is where the last pass put the CPU, `at` where the game's frame left it (its `top` joint, read before this pass
moves it), `egg` the target's spawn point, `age` its ticks since, `fly` its launch, sideways speed and gravity, `fall` and
`wait` the ids of FIGHTER_STATUS_KIND_FALL and _WAIT. `model` is eggs.rs `Run::egg_at` with `cpu_pass`'s lift, so a flight
is checked against the model from the lines alone."""
import math
import re

NUM = r'(-?[\d.]+(?:e-?\d+)?|NaN|-?inf)'
PAIR = r'\[%s, %s\]' % (NUM, NUM)
LINE = re.compile(
    r'\[Eggs\] cpu damage=%s score=(\d+) move=(None|Some\(\(%s, (Hold|Fresh\((?:true|false)\)|Land)\)\)) '
    r'want=(None|Some\(%s\)) at=%s egg=(None|Some\(%s\)) age=(\d+) fly=\[%s, %s, %s\] status=(-?\d+) fall=(-?\d+) '
    r'wait=(-?\d+) stop=(true|false)' % (NUM, PAIR, PAIR, PAIR, PAIR, NUM, NUM, NUM))
BOUNCE, LIFT = 0.6, 1.0
# A hit "in the air": the CPU's feet this far above the target's ground or more; "on the ground": within REST of it.
AIRBORNE, REST = 2.0, 0.5


def parse(text):
    """One line as a dict, or None."""
    m = LINE.search(text)
    if not m:
        return None
    g = m.groups()
    f = float
    return {
        'damage': f(g[0]), 'score': int(g[1]),
        'next': [f(g[3]), f(g[4])] if g[2] != 'None' else None, 'put': g[5] or 'None',
        'want': [f(g[7]), f(g[8])] if g[6] != 'None' else None,
        'at': [f(g[9]), f(g[10])],
        'egg': [f(g[12]), f(g[13])] if g[11] != 'None' else None,
        'age': int(g[14]), 'fly': [f(g[15]), f(g[16]), f(g[17])],
        'status': int(g[18]), 'fall': int(g[19]), 'wait': int(g[20]), 'stop': g[21] == 'true',
    }


def height(r):
    """The CPU's feet above the target's ground in a parsed line (None without a target)."""
    return r['at'][1] - r['egg'][1] if r and r['egg'] else None


def model(egg, fly, age, lift=LIFT):
    """eggs.rs `egg_at` at `age`, raised to `lift` above the spawn while the flight lasts (`cpu_pass`); and whether it flies."""
    (x, y), (v, vx, g), t = egg, fly, float(age)
    up = 2.0 * v / g if g else 0.0
    vv, tt = (v, t) if t <= up else (v * BOUNCE, min(t - up, up * BOUNCE))
    flown = min(t, up * (1.0 + BOUNCE))
    h = max(vv * tt - g * tt * tt / 2.0, 0.0)
    aloft = v > 0.0 and t < up * (1.0 + BOUNCE)
    return [x + vx * flown, max(y + h, y + lift) if aloft else y + h], aloft


def flight(records):
    """The first flight in `records` (dicts from `parse`, in log order): from the first placement afresh to its rest.
    Returns the facts a row checks: the model's apex and rest tick, the apex seen, the landing's tick, the largest drift
    (game frame vs the last placement) and path error (the game's position vs the model), the statuses seen in flight and
    at rest, and the FALL and WAIT ids."""
    start = next((i for i, r in enumerate(records) if r['put'].startswith('Fresh') and r['egg']), None)
    if start is None:
        return {'error': 'no placement afresh'}
    first = records[start]
    egg, fly = first['egg'], first['fly']
    v, _, g = fly
    rec = {'egg': egg, 'fly': fly, 'fresh': first['put'], 'fall': first['fall'], 'wait': first['wait'],
           'apex_model': round(v * v / (2 * g), 2) if g else 0.0, 'rest_model': round(2 * v / g * (1 + BOUNCE), 1) if g else 0.0}
    drift, error, gap, top, land, samples, rest_height = 0.0, 0.0, 0.0, None, None, 0, 0.0
    seen = {'air': set(), 'rest': set()}
    for r in records[start + 1:]:
        if r['egg'] != egg or r['put'].startswith('Fresh'):
            break  # the next target, or a placement afresh after a hit
        if r['put'] == 'Land' and land is None:
            land = r['age']
        if r['next'] is not None:
            gap = max(gap, math.dist(model(egg, fly, r['age'])[0], r['next']))
        if r['want'] is None or r['put'] != 'Hold':
            continue
        samples += 1
        drift = max(drift, math.dist(r['want'], r['at']))
        # The game's position against the model at the last pass's tick (the age before this pass, or this one if the CPU's
        # pass ran first in the frame).
        error = max(error, min(math.dist(model(egg, fly, a)[0], r['at']) for a in (r['age'] - 1, r['age'])))
        top = max(top if top is not None else -1e9, height(r))
        # The pose: a status request takes effect on the game's next frame, so the 3 ticks after a placement or a landing
        # are left out.
        aloft = model(egg, fly, r['age'])[1]
        settled = r['age'] >= first['age'] + 3 if aloft else land is not None and r['age'] >= land + 3
        if settled:
            seen['air' if aloft else 'rest'].add(r['status'])
            if not aloft:
                rest_height = max(rest_height, abs(height(r)))
    rec.update({'samples': samples, 'land_age': land, 'apex_seen': round(top, 2) if top is not None else None,
                'drift': round(drift, 3), 'path_error': round(error, 3), 'model_gap': round(gap, 4),
                'air': sorted(seen['air']), 'rest': sorted(seen['rest']), 'rest_height': round(rest_height, 3)})
    return rec


def problems(rec, tolerance=0.5, apex_slack=0.8, land_slack=2):
    """Why a flight fails, or [] when it follows the model."""
    if rec.get('error'):
        return [rec['error']]
    out = []
    if rec['fresh'] != 'Fresh(true)':
        out.append('the first placement was %s, not in the air' % rec['fresh'])
    if rec['model_gap'] > 1e-2:
        out.append('a placement %.3f off the model' % rec['model_gap'])
    if rec['drift'] > tolerance:
        out.append('the game moved the CPU %.3f from where it was put' % rec['drift'])
    if rec['path_error'] > tolerance:
        out.append('the CPU was %.3f off the model path' % rec['path_error'])
    if rec['apex_seen'] is None or abs(rec['apex_seen'] - rec['apex_model']) > apex_slack:
        out.append('apex %s, the model %s' % (rec['apex_seen'], rec['apex_model']))
    if rec['land_age'] is None or abs(rec['land_age'] - rec['rest_model']) > land_slack:
        out.append('landed at tick %s, the model %s' % (rec['land_age'], rec['rest_model']))
    if rec['fall'] < 0 or rec['air'] != [rec['fall']]:
        out.append('in flight the statuses %s, want FALL (%s) alone' % (rec['air'], rec['fall']))
    if rec['wait'] < 0 or rec['rest'] != [rec['wait']] or rec['rest_height'] > REST:
        out.append('at rest the statuses %s %.2f above the ground, want WAIT (%s) on it' % (rec['rest'], rec['rest_height'],
                                                                                          rec['wait']))
    return out
