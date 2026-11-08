"""Host tests for fid_trace.py: python3 -I test_fid_trace.py"""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import fid_trace


def pack(buttons, x=0, y=0):
    return buttons | ((x & 0xff) << 32) | ((y & 0xff) << 40)


class Compare(unittest.TestCase):
    expected = [(0x8000, 0, -20, 0, 0), (0x8201, 0, 0, 0, 0), (0x8201, 0, 0, 0, 0), (0x8000, 0, -19, 0, 0),
                (0x8000, 31, 0, 0, 0)]

    def stream(self, plan):
        """plan: 'a<i>' applies expected[i], 't' is a consumer frame. Returns (applied, ticks)."""
        applied, ticks = [], []
        for seq, step in enumerate(plan):
            if step == 't':
                ticks.append((seq, seq))
            else:
                applied.append((seq, seq, self.expected[int(step[1:])]))
        return applied, ticks

    def test_lossless_run(self):
        applied, ticks = self.stream(['a0', 't', 'a1', 't', 'a2', 't', 'a3', 't', 'a4', 't'])
        report = fid_trace.compare(self.expected, applied, ticks)
        self.assertEqual((report['matched'], report['dropped'], report['ticks_without_frame']), (5, 0, 0))

    def test_drop_and_gap(self):
        # Frame 2 is overwritten before the AI takes it, and one AI frame gets nothing.
        applied, ticks = self.stream(['a0', 't', 'a1', 't', 't', 'a3', 't', 'a4', 't'])
        report = fid_trace.compare(self.expected, applied, ticks)
        self.assertEqual(report['dropped_first'], [2])
        self.assertEqual(report['ticks_without_frame'], 1)

    def test_unpack_signed(self):
        self.assertEqual(fid_trace.unpack(pack(0x8000, -70, 20)), (0x8000, -70, 20, 0, 0))

    def test_motion_names(self):
        self.assertEqual(fid_trace.motion_name(fid_trace.hash40('attack_11')), 'attack_11')
        self.assertEqual(fid_trace.motion_name(fid_trace.hash40('attack_s4_s')), 'attack_s4_s')

    def test_two_register_frame(self):
        lines = ['{"event":"entry","point_id":"4","sequence":"3","frame":"5","registers":{"x8":"0x8201","x9":"0xec3c"}}']
        recs = fid_trace.records(lines)
        self.assertEqual(fid_trace.applied(recs, '4', 'x8+x9'), [(3, 5, (0x8201, 0x3c, -20, 0, 0))])

    def test_records_skip_loss_lines(self):
        lines = ['{"schema":1,"event":"loss","records":"3"}',
                 '{"event":"entry","point_id":"2","sequence":"7","frame":"9","registers":{"x9":"0x8201"}}']
        recs = fid_trace.records(lines)
        self.assertEqual(fid_trace.applied(recs, '2', 'x9'), [(7, 9, (0x8201, 0, 0, 0, 0))])


if __name__ == '__main__':
    unittest.main()
