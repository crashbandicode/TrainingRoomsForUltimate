"""Host tests of the F11 rows' pure checks, on the samples flown in f11-probe-1 (XPS, 2026-11-23).
Run: /usr/bin/python3 -I -B test_f11.py  (needs the playbook runner's folder for `routes`; see the path below)."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, "/home/intpa/tile-fix-g4/runner-216a6b2/tools/eden-runner")
sys.path.insert(0, str(Path(__file__).resolve().parent))
import f11  # noqa: E402

GRAVITY = 0.12  # Zero Suit Samus (kind 32), the CPU of f11-probe-1
FLOWN = [  # (published point, bridge sample) while held
    (("-159.9928", "-69.49987"), {"x": -159.9928, "y": -69.61987}),
    (("-199.9928", "-30.49987"), {"x": -199.9928, "y": -30.619871}),
    (("-229.9928", "59.50013"), {"x": -229.9928, "y": 59.38013}),
    (("0", "130"), {"x": 0.0, "y": 129.88}),
    (("-100", "60"), {"x": -100.0, "y": 59.88}),
]


class Checks(unittest.TestCase):
    def test_the_flown_held_samples_are_the_picked_point_less_one_gravity_step(self):
        for point, body in FLOWN:
            self.assertIsNone(f11.held_problem(body, point, GRAVITY), (point, body))

    def test_a_body_off_the_point_by_less_than_any_old_tolerance_fails(self):
        point, body = FLOWN[0]
        for moved in ({"x": body["x"] + 0.01, "y": body["y"]}, {"x": body["x"], "y": body["y"] - 0.01},
                      {"x": body["x"], "y": float(point[1])}):  # the last: the point itself, no gravity step
            self.assertIn("is not the held point", f11.held_problem(moved, point, GRAVITY))

    def test_another_fighters_gravity_fails(self):
        point, body = FLOWN[0]
        self.assertIsNotNone(f11.held_problem(body, point, 0.125))

    def test_a_missing_read_fails(self):
        point, body = FLOWN[0]
        for broken_body, broken_point in (({"x": None, "y": body["y"]}, point), ({}, point), (body, None),
                                          (body, (point[0], None))):
            self.assertIn("a read is missing", f11.held_problem(broken_body, broken_point, GRAVITY))

    def test_back_means_the_same_words_and_situation(self):
        origin = {"x": 36.0, "y": 0.010259508, "situation": 0}
        self.assertIsNone(f11.back_problem(dict(origin), origin))
        for moved in ({**origin, "x": 36.01}, {**origin, "y": 0.02}, {**origin, "situation": 2}):
            self.assertIn("after B", f11.back_problem(moved, origin))
        for broken in ({**origin, "x": None}, {"x": 36.0, "y": 0.010259508}, {**origin, "situation": None}):
            self.assertIsNotNone(f11.back_problem(broken, origin))


if __name__ == "__main__":
    unittest.main()
