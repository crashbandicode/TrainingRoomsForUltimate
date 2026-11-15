"""Host tests of cfcheck.py: synthetic `[Eggs] cpu` lines written by the model, clean and with planted faults.

  /usr/bin/python3 -I eden/rc14-cpu-target-flight/test_cfcheck.py"""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import cfcheck  # noqa: E402

FALL, WAIT, LANDING, DAMAGE = 0x1d, 0x0, 0x17, 0x47


def line(r):
    pair = lambda p: '[%r, %r]' % (p[0], p[1])  # noqa: E731
    move = 'None' if r['next'] is None else 'Some((%s, %s))' % (pair(r['next']), r['put'])
    return ('[Eggs] cpu damage=%r score=%d move=%s want=%s at=%s egg=%s age=%d fly=[%r, %r, %r] status=%d fall=%d wait=%d '
            'stop=%s' % (r['damage'], r['score'], move, 'None' if r['want'] is None else 'Some(%s)' % pair(r['want']),
                         pair(r['at']), 'Some(%s)' % pair(r['egg']), r['age'], *r['fly'], r['status'], FALL, WAIT,
                         'true' if r['stop'] else 'false'))


def synth(speed=1.0, egg=(-12.5, 0.0), drift=0.0, sag_at=None, land_late=0, status_at=None, every=6):
    """The lines a CPU pass writes for one flight: Fresh at age 0, Hold every `every` ticks, Land at the rest tick.
    `status_at` = (age, status) plants a status on one sample."""
    v, g = 1.44 * speed, 0.048 * speed * speed
    fly = [v, 0.05 * speed, g]
    rest = int(2 * v / g * 1.6 + 0.999)
    out, last = [], None
    for age in range(0, rest + 40):
        nxt, flying = cfcheck.model(egg, fly, age)
        put = 'Fresh(true)' if age == 0 else 'Land' if age == rest + land_late else 'Hold'
        at = list(last) if last else [egg[0], egg[1]]
        at[1] -= drift  # the game's own gravity between two passes
        if sag_at is not None and age == sag_at:
            at[1] -= 3.0
        status = FALL if flying and age >= 2 else WAIT
        if status_at and status_at[0] == age:
            status = status_at[1]
        if put != 'Hold' or age % every == 0:
            out.append(line({'damage': 0.0, 'score': 0, 'next': nxt, 'put': put, 'want': last, 'at': at, 'egg': list(egg),
                             'age': age, 'fly': fly, 'status': status, 'stop': False}))
        last = nxt
    return out


def check(lines):
    recs = [cfcheck.parse(t) for t in lines]
    assert all(recs), [t for t, r in zip(lines, recs) if not r][:2]
    rec = cfcheck.flight(recs)
    return rec, cfcheck.problems(rec)


class Flight(unittest.TestCase):
    def test_parse_rust_debug(self):
        t = ('12345 [Eggs] cpu damage=0 score=3 move=Some(([-12.5, 1.0], Fresh(true))) want=None at=[-30.0, 0.0] '
             'egg=Some([-12.5, 0.0]) age=0 fly=[1.44, 0.03, 0.048] status=0 fall=29 wait=0 stop=false')
        r = cfcheck.parse(t)
        self.assertEqual((r['put'], r['next'], r['want'], r['egg'], r['fly'][2], r['score'], r['fall'], r['wait']),
                         ('Fresh(true)', [-12.5, 1.0], None, [-12.5, 0.0], 0.048, 3, 29, 0))
        hit = cfcheck.parse('[Eggs] cpu damage=9.5 score=1 move=None want=Some([1.0, 2e-3]) at=[1.0, 12.0] '
                            'egg=Some([1.0, 0.0]) age=21 fly=[0.72, 0.0, 0.012] status=71 fall=29 wait=0 stop=true')
        self.assertEqual((hit['next'], hit['put'], hit['want'], hit['stop'], cfcheck.height(hit)),
                         (None, 'None', [1.0, 0.002], True, 12.0))
        self.assertIsNotNone(cfcheck.parse('[Eggs] cpu damage=0 score=0 move=None want=None at=[NaN, NaN] egg=None age=0 '
                                           'fly=[0.0, 0.0, 0.0] status=-1 fall=-1 wait=-1 stop=false'))

    def test_clean_flights_pass_at_every_speed(self):
        for speed in (1.0, 0.5, 1.5):
            rec, bad = check(synth(speed))
            self.assertEqual(bad, [], (speed, rec))
            self.assertAlmostEqual(rec['apex_model'], 21.6, places=1)
            self.assertEqual((rec['air'], rec['rest']), ([FALL], [WAIT]))

    def test_game_gravity_within_tolerance(self):
        self.assertEqual(check(synth(drift=0.2))[1], [])

    def test_planted_faults_are_flagged(self):
        cases = {
            'a fall the model does not have': synth(sag_at=42),
            'a landing late': synth(land_late=5),
            'a landing mid-flight': synth(status_at=(30, LANDING)),
            'a tumble status in flight': synth(status_at=(36, DAMAGE)),
            'still in the air after the landing': synth(status_at=(120, FALL)),
            'the game pulling it down every frame': synth(drift=0.8),
        }
        for name, lines in cases.items():
            self.assertNotEqual(check(lines)[1], [], name)

    def test_no_flight(self):
        self.assertEqual(cfcheck.problems(cfcheck.flight([])), ['no placement afresh'])


if __name__ == '__main__':
    unittest.main()
