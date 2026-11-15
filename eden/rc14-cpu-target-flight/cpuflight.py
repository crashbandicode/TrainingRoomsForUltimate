"""rc.14 cpu-target-flight Eden rows (the operator 11-15 ~02:20Z: the CPU target flies like the discs), one boot on the diag
build of the branch, worker A's eggs harness (eggsrows.py, eggsfly.py, hotslots.py, gna.py), P1 Fox from reset-center.

The diag build writes one `[Eggs] cpu` line per event of the CPU target (a placement afresh, a landing, a counted hit) and
every 6th held tick (cfcheck.py reads them: where TR put it, where the game's frame left it, the flight's tick, launch,
status against the FALL and WAIT ids). Rows:
  flight_normal / flight_slow / flight_fast: Target CPU, Free practice: stills in flight and at rest; the flight against
    the model (cfcheck.problems: drift and path error <= 0.5, apex, landing tick, FALL in flight, WAIT on the ground).
  groundhit: the CPU launched beside Fox (diag-egg.req), a jab after it lands counts with the CPU on the ground.
  airhit: Slow, Breaks on Aerials: the CPU launched beside Fox, full-hop nairs until one counts with the CPU airborne; the
    next spot is placed afresh in the air and flies.
  done10: 10 targets: ten launches beside Fox, each jabbed at rest: "Done! 10".
  reset: L+R+A mid-flight: a new first target is placed afresh in the air and flies to rest by the model."""
import re
import threading
import time

import hotslots as hs
from gna import _begin, _finish, _mark

import cfcheck
import eggsfly as ef
import eggsrows as er

TEN, FREE = 2, 4
ANY, AERIALS = 0, 1
SPEED = {'normal': 0, 'slow': 1, 'fast': 2}
FACTOR = {'normal': 1.0, 'slow': 0.5, 'fast': 1.5}
FRESH_AIR = r'\[Eggs\] cpu .*move=Some\(\(\[[^\]]*\], Fresh\(true\)\)\)'
LAND = r'\[Eggs\] cpu .*move=Some\(\(\[[^\]]*\], Land\)\)'
KEEP = re.compile(r'\[Eggs\] cpu |\[Eggs\] notice=|diag_egg_request|\[Ui\] menu (opened|closed)')


def _set(ctx, mode, rule, speed):
    er.fx._live_set(ctx, eggsercise=ef._onehot(mode, 5), eggsercise_break=ef._onehot(rule, 3),
                    eggsercise_speed=ef._onehot(speed, 3), eggsercise_target=ef._onehot(1, 2))


class Reader:
    """Every kept line after `mark`, read by a thread every 0.2 s (the 10-target notice turns the log over in seconds)."""

    def __init__(self, ctx, mark):
        self.ctx, self.mark, self.kept, self.stop = ctx, mark, {}, threading.Event()
        self.thread = threading.Thread(target=self._poll, daemon=True)
        self.thread.start()

    def _poll(self):
        while not self.stop.is_set():
            self.collect()
            self.stop.wait(0.2)

    def collect(self):
        self.kept.update({s: t for s, _, t in hs.diag_since(self.ctx, self.mark) if KEEP.search(t)})

    def lines(self):
        self.collect()
        return [self.kept[s] for s in sorted(self.kept)]

    def find(self, pattern, cap, after=0):
        """(index, line) of the first kept line from the `after`th on matching `pattern`, within `cap` s; (None, None)."""
        regex, deadline = re.compile(pattern), time.time() + cap
        while time.time() < deadline:
            self.ctx.check_stop()
            lines = self.lines()
            hit = next((i for i in range(after, len(lines)) if regex.search(lines[i])), None)
            if hit is not None:
                return hit, lines[hit]
            time.sleep(0.15)
        return None, None

    def wait(self, pattern, cap, after=0):
        return self.find(pattern, cap, after)[1]

    def closed(self):
        """The index after the last `[Ui] menu closed` line (0 without one): where a run started by the close begins."""
        lines = self.lines()
        return next((i + 1 for i in range(len(lines) - 1, -1, -1) if '[Ui] menu closed' in lines[i]), 0)

    def close(self):
        self.stop.set()
        self.thread.join()
        return self.lines()


def _records(lines):
    return [r for r in (cfcheck.parse(t) for t in lines) if r]


def _reset(ctx):
    ctx.call('macro.play', {'path': str(er.SEG / 'reset-center.edenmacro.json')})
    ctx.runner.wait_macro(ctx.on, 20)
    ctx.call('input.reset')


def _start(ctx, mode, rule, speed, reset=True):
    """reset-center, the rows, the real menu opened and closed (the close starts the run); a reader from before the close."""
    if reset:
        _reset(ctx)
        time.sleep(1.0)
    _set(ctx, mode, rule, speed)
    reader = Reader(ctx, hs.diag_mark(ctx))
    er.steps.open_menu(ctx)
    er._close(ctx)
    return reader


def _press(ctx, button, ms=60):
    ctx.call('input.button', {'button': button, 'state': 'tap', 'duration_ms': ms, 'lease_ms': ms + 300})


def _launch_beside(ctx, reader, dx):
    """diag-egg.req: the CPU target placed afresh `dx` in front of Fox on the ground and launched: the index after its
    Fresh line (or after the request) and the line."""
    n = len(reader.lines())
    if not er._place(ctx, dx, 0.0):
        return n, None
    i, fresh = reader.find(FRESH_AIR, 4, after=n)
    return (n if i is None else i + 1), fresh


def _off(ctx):
    """Eggs-ercise Off: the run stops on the next pass, so the next row's reset restarts nothing."""
    er.fx._live_set(ctx, eggsercise=ef._onehot(0, 5))


def _score_line(reader, score, after):
    return reader.wait(r'\[Eggs\] cpu damage=\S+ score=%d move=None' % score, 3, after=after)


def setup(ctx):
    _, out = _begin(ctx)
    ok = er._fighter(ctx, er.FOX, out, 'p1_fox')
    out['menu'] = {k: er._menu(ctx).get(k) for k in ('eggsercise', 'eggsercise_target', 'eggsercise_speed', 'lra_reset')}
    if not ok:
        raise RuntimeError('cpuflight setup: no HUD line for P1 Fox')
    return out


def _row(name):
    """A row body: it gets (ctx, t0, out, problems) and fills them; the run is turned Off after it whatever happens."""
    def wrap(body):
        def run(ctx):
            t0, out = _begin(ctx)
            problems = []
            try:
                body(ctx, t0, out, problems)
            finally:
                _off(ctx)
            out['problems'] = problems
            return _finish(ctx, name, out, problems)
        run.__name__ = body.__name__
        return run
    return wrap


def _flight(name):
    def body(ctx, t0, out, problems):
        reader = _start(ctx, FREE, ANY, SPEED[name])
        start = reader.closed()
        _, fresh = reader.find(FRESH_AIR, 20, after=start)
        _mark(t0, out, 'fresh')
        rest_s = 1.6 / FACTOR[name]
        shots = []
        for k in range(8):
            shots.append(str(ctx.shot('cf-%s-fly-%d' % (name, k))))
            time.sleep(rest_s / 9)
        land = reader.wait(LAND, rest_s + 6, after=start)
        time.sleep(0.8)
        shots.append(str(ctx.shot('cf-%s-rest' % name)))
        time.sleep(0.5)
        lines = reader.close()[start:]
        rec = cfcheck.flight(_records(lines))
        bad = cfcheck.problems(rec)
        if not fresh:
            bad.insert(0, 'no placement afresh in the air within 20 s of the menu close')
        out.update({'speed': name, 'fresh': fresh and fresh[:300], 'land': land and land[:300], 'flight': rec,
                    'shots': shots, 'lines': [t[:400] for t in lines if '[Eggs] cpu ' in t][:80]})
        problems.extend('%s: %s' % (name, b) for b in bad)
    body.__name__ = 'flight_' + name
    return _row('cf-' + name)(body)


flight_normal, flight_slow, flight_fast = _flight('normal'), _flight('slow'), _flight('fast')


def _jab_landed(ctx, reader, score, tries, shots=None):
    """Launch the CPU 7 in front of Fox, wait for its landing, jab; until the count reads `score` (at most `tries`). The
    receipts of the tries, the counted hit's line and the index after which it was looked for."""
    log = []
    for k in range(tries):
        after, fresh = _launch_beside(ctx, reader, 7.0)
        land = reader.wait(LAND, 5, after=after) if fresh else None
        n = len(reader.lines())
        time.sleep(0.3)
        rec = {'fresh': fresh and fresh[:300], 'land': land and land[:200]}
        if shots:
            rec['before'] = str(ctx.shot('%s-%d-before' % (shots, k)))
        _press(ctx, 'a')
        hit = _score_line(reader, score, n)
        rec['hit'] = hit and hit[:400]
        if shots:
            rec['after'] = str(ctx.shot('%s-%d-after' % (shots, k)))
        log.append(rec)
        if hit:
            return log, hit, n
        time.sleep(1.0)
    return log, None, len(reader.lines())


@_row('cf-groundhit')
def groundhit(ctx, t0, out, problems):
    """Normal, Any attack: the CPU launched 7 in front of Fox; after it lands, a jab counts with the CPU on the ground; the
    next spot is placed afresh in the air."""
    reader = _start(ctx, FREE, ANY, SPEED['normal'])
    out['first'] = reader.wait(FRESH_AIR, 20, after=reader.closed())
    _mark(t0, out, 'tries')
    out['tries'], hit, n = _jab_landed(ctx, reader, 1, 3, shots='cf-ground')
    rec = cfcheck.parse(hit) if hit else None
    out['hit'] = rec
    if not rec:
        problems.append('no counted jab on the landed CPU in %d tries' % len(out['tries']))
    elif abs(cfcheck.height(rec)) > cfcheck.REST:
        problems.append('the CPU was not on the ground at the hit: %.2f above it' % cfcheck.height(rec))
    nxt = reader.wait(FRESH_AIR, 4, after=n) if rec else None
    out['next'] = nxt and nxt[:300]
    if rec and not nxt:
        problems.append('no next spot placed afresh in the air after the hit')
    out['lines'] = [t[:400] for t in reader.close() if '[Eggs] cpu ' in t][-60:]


@_row('cf-airhit')
def airhit(ctx, t0, out, problems):
    """Slow, Breaks on Aerials: the CPU launched 7 in front of Fox; full-hop nairs at varied delays until one counts with the
    CPU airborne; the next spot is placed afresh in the air and flies."""
    reader = _start(ctx, FREE, AERIALS, SPEED['slow'])
    out['first'] = reader.wait(FRESH_AIR, 20, after=reader.closed())
    tries, hit, n = [], None, 0
    for k, delay in enumerate((0.0, 0.3, 0.6, 0.15, 0.45, 0.8, 0.05, 0.35)):
        _mark(t0, out, 'try-%d' % k)
        n, fresh = _launch_beside(ctx, reader, 7.0)
        time.sleep(delay)
        _press(ctx, 'x', 150)  # held 150 ms: a full hop
        time.sleep(0.1)
        _press(ctx, 'a')  # no direction: neutral air
        shots = []
        for j in range(4):
            time.sleep(0.12)
            shots.append(str(ctx.shot('cf-air-%d-%d' % (k, j))))
        hit = _score_line(reader, 1, n)
        tries.append({'delay': delay, 'fresh': fresh and fresh[:300], 'hit': hit and hit[:400], 'shots': shots})
        if hit:
            break
        time.sleep(1.6)  # Fox lands
    out['tries'] = tries
    rec = cfcheck.parse(hit) if hit else None
    out['hit'] = rec
    if not rec:
        problems.append('no counted nair on the flying CPU in %d tries' % len(tries))
        return
    if cfcheck.height(rec) < cfcheck.AIRBORNE:
        problems.append('the CPU was not airborne at the hit: %.2f above the ground' % cfcheck.height(rec))
    i, nxt = reader.find(FRESH_AIR, 6, after=n)
    out['next'] = nxt and nxt[:300]
    time.sleep(1.2 / FACTOR['slow'])
    out['next_shot'] = str(ctx.shot('cf-air-next-flying'))
    lines = reader.close()
    after = _records(lines[i:]) if i is not None else []
    rise = [r['at'][1] - r['egg'][1] for r in after[1:] if r['egg'] and r['put'] == 'Hold']
    out['next_rise'] = [round(h, 2) for h in rise[:12]]
    if not nxt or not rise or max(rise) < 5.0:
        problems.append('the next spot did not fly: %s' % out['next_rise'])
    out['lines'] = [t[:400] for t in lines if '[Eggs] cpu ' in t][-60:]


@_row('cf-done10')
def done10(ctx, t0, out, problems):
    """10 targets, Any attack, Normal: ten launches beside Fox, each jabbed after it lands: "Done! 10"."""
    reader = _start(ctx, TEN, ANY, SPEED['normal'])
    out['first'] = reader.wait(FRESH_AIR, 20, after=reader.closed())
    scores = []
    for k in range(1, 11):
        _mark(t0, out, 'target-%d' % k)
        _, hit, _ = _jab_landed(ctx, reader, k, 3)
        scores.append(bool(hit))
        if not hit:
            problems.append('target %d: no counted jab' % k)
            break
    done = reader.wait(r'\[Eggs\] notice="Done! 10 in', 5)
    out.update({'scores': scores, 'done': done and done[:200], 'shot': str(ctx.shot('cf-done10'))})
    if not done:
        problems.append('no "Done! 10" notice')
    reader.close()


@_row('cf-reset')
def reset(ctx, t0, out, problems):
    """Free practice, Normal: L+R+A while the CPU flies; a new first target is placed afresh in the air and flies to rest by
    the model."""
    reader = _start(ctx, FREE, ANY, SPEED['normal'])
    out['first'] = reader.wait(FRESH_AIR, 20, after=reader.closed())
    time.sleep(0.35)
    out['mid'] = str(ctx.shot('cf-reset-midflight'))
    n = len(reader.lines())
    _mark(t0, out, 'reset')
    _reset(ctx)
    restart = reader.wait(r'\[Eggs\] notice="Eggs 0"', 6, after=n)
    fresh = reader.wait(FRESH_AIR, 20, after=n)
    land = reader.wait(LAND, 8, after=n) if fresh else None
    time.sleep(0.6)
    out['rest'] = str(ctx.shot('cf-reset-rest'))
    lines = reader.close()[n:]
    rec = cfcheck.flight(_records(lines))
    problems.extend(cfcheck.problems(rec))
    if not restart:
        problems.insert(0, 'no "Eggs 0" notice after L+R+A')
    out.update({'restart': restart and restart[:200], 'fresh': fresh and fresh[:300], 'land': land and land[:300],
                'flight': rec, 'lines': [t[:400] for t in lines if '[Eggs] cpu ' in t][:60]})


def restore(ctx):
    er.fx._live_set(ctx, eggsercise=ef._onehot(0, 5), eggsercise_target=ef._onehot(0, 2), eggsercise_speed=ef._onehot(0, 3),
                    eggsercise_break=ef._onehot(0, 3))
    return {}
