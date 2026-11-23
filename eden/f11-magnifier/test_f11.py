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


    def test_a_ready_sample_needs_the_selected_cpu_with_x_and_y(self):
        good = {"kind": 32, "x": -199.9928, "y": -30.619871, "dmg": 100.0}
        self.assertIsNone(f11.ready_body_problem(good, 32))
        self.assertIn("not the selected CPU", f11.ready_body_problem({**good, "kind": 0}, 32))
        self.assertIn("not the selected CPU", f11.ready_body_problem(None, 32))
        for axis in ("x", "y"):
            for bad in (None, float("nan"), float("inf"), "1.0", True):
                self.assertIn(f"no valid {axis}", f11.ready_body_problem({**good, axis: bad}, 32), (axis, bad))
            missing = {key: value for key, value in good.items() if key != axis}
            self.assertIn(f"no valid {axis}", f11.ready_body_problem(missing, 32))


    def test_rest_is_two_equal_grounded_reads(self):
        standing = {"x": 36.0, "y": 0.010259508, "situation": 0}
        self.assertTrue(f11.at_rest(dict(standing), standing))
        airborne = {"x": -10.52223, "y": 15.195398, "situation": 2}  # the first f11-t3 of f11-proof-1
        self.assertFalse(f11.at_rest(dict(airborne), airborne))
        self.assertFalse(f11.at_rest({**standing, "x": 36.01}, standing))
        self.assertFalse(f11.at_rest({**standing, "y": None}, standing))
        self.assertFalse(f11.at_rest(standing, {}))


    # The bridge's own lines, as recorded (wall times dropped).
    YOGA_SWAP = [  # early install 2 boot, Yoga, f11-t1: Donkey Kong held, B, Zero Suit Samus back at the held point
        "[TrainingRooms] cpu_event status entry=1 partner=false status=0 prev=14 situation=0 motion_frame=1 ev=4029 frame=37414",
        "[Bridge] fighter entry=1 gone ev=4030 frame=37415",
        "[TrainingRooms] cpu_event role entry=1 partner=false kind=32 lobby=false accessor=0x121c5cf0c0 ev=4032 frame=37416",
        "[Bridge] fighter entry=1 kind=32 x=-149.9928 y=-100.58487 lr=-1 dmg=0 status=14 situation=2 stock=0 dead=0 item=-1 ev=4036 frame=37417",
    ]
    XPS_ROOM = [  # early install 2 boot, XPS room, f11-t1: the partner put back, then running off by itself
        "[TrainingRooms] cpu_event status entry=4 partner=false status=0 prev=14 situation=0 motion_frame=1 ev=3849 frame=81329",
        "[Bridge] fighter entry=4 kind=32 x=-36.048977 y=0.010259508 lr=1 dmg=0 status=7 situation=0 stock=0 dead=2 item=-1 ev=3852 frame=81330",
        "[Bridge] fighter entry=4 kind=32 x=0.09103739 y=0.010259508 lr=1 dmg=0 status=0 situation=0 stock=0 dead=2 item=-1 ev=3857 frame=81359",
    ]

    def test_the_put_back_is_judged_at_the_first_line_after_its_event(self):
        origin = {"kind": 32, "x": -36.048977, "y": 0.010259508, "situation": 0}
        back = f11.put_back(self.XPS_ROOM, 4)
        self.assertEqual((back["x"], back["status"]), (-36.048977, 7))
        self.assertIsNone(f11.put_back_problem(back, origin, 32))
        # The line a second later is where the partner ran to; it is not the put-back.
        self.assertIsNotNone(f11.back_problem(f11.fighter_lines(self.XPS_ROOM, 4)[-1], origin))
        self.assertIsNone(f11.put_back(self.XPS_ROOM, 1))
        self.assertIsNone(f11.put_back(self.XPS_ROOM[:1], 4))
        self.assertIn("no fighter line", f11.put_back_problem(None, origin, 32))

    def test_the_recorded_swap_back_is_the_red_take(self):
        origin = {"kind": 32, "x": 36.0, "y": 0.010259508, "situation": 0}
        back = f11.put_back(self.YOGA_SWAP, 1)
        # The drill's fighter (kind 1) was gone before it wrote a line: the first line is the original, off the stage.
        self.assertIn("not the held fighter 1", f11.put_back_problem(back, origin, 1))
        self.assertIn("after B", f11.put_back_problem(back, origin, 32))

    def test_the_floor_attach_candidates(self):
        candidates = {"floor_end": -79.9856, "ledge": -85.9856}
        # The room's Juggler reading fits both candidates, so that x cannot decide.
        self.assertEqual(f11._word(f11.attach(0.09103739, -79.9856)), "0x3dba7000")
        self.assertEqual(f11._word(f11.attach(0.09103739, -85.9856)), "0x3dba7000")
        self.assertEqual(f11.attach_verdict(0.09103739, 0.091033936, candidates)[0], "SETUP")
        # A whole x of 45 gives the two candidates different words (0x42340000 and 0x4233fffe).
        self.assertEqual((f11._word(f11.attach(45.0, -79.9856)), f11._word(f11.attach(45.0, -85.9856))),
                         ("0x42340000", "0x4233fffe"))
        name, detail = f11.attach_verdict(45.0, f11.attach(45.0, -85.9856), candidates)
        self.assertEqual((name, detail["read_is_the_placed_word"]), ("ledge", False))
        # The floor end leaves 45 as placed: the read names it, and says it is also the placed word.
        name, detail = f11.attach_verdict(45.0, 45.0, candidates)
        self.assertEqual((name, detail["read_is_the_placed_word"]), ("floor_end", True))
        self.assertEqual(f11.attach_verdict(45.0, 45.5, candidates)[0], "none")
        # Left of 42.0144 the candidates agree, as for every origin recorded so far.
        self.assertEqual(f11.attach_verdict(-40.0, -40.0, candidates)[0], "SETUP")


if __name__ == "__main__":
    unittest.main()
