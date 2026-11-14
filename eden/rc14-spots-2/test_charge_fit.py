#!/usr/bin/env python3
"""Host tests of charge_fit.py: python3 -I test_charge_fit.py (synthetic lines in the diag build's format)."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import charge_fit as cf  # noqa: E402

FIELDS = '0xe0=%s 0xe4=0.0 0xf0=1.0 0xf4=1.0 0x210=1.0'


def probe(frame, status, active, mul, powers='13.0, 13.0, 13.0, 18.0, 0.0, 0.0, 0.0, 0.0', who='p1'):
    return ('[SpotsProbe] frame=%d who=%s status=%#x stop=false active=%s mf=1.0 damage=0.000 other_damage=0.000 powers=[%s] fields: %s'
            % (frame, who, status, str(active).lower(), powers, FIELDS % mul))


def stop(frame, gained, mul=None):
    extra = ' mul=%s' % mul if mul is not None else ''
    return '[Spots] stop frame=%d gained=%s%s powers=[13.0, 13.0, 13.0, 18.0, 0.0, 0.0, 0.0, 0.0] newest_mark=None' % (frame, gained, extra)


class Parse(unittest.TestCase):
    def test_a_probe_line_has_its_fields(self):
        p = cf.parse_line(probe(10, 0x2f, True, 1.4))
        self.assertEqual((p['kind'], p['frame'], p['who'], p['status'], p['active']), ('probe', 10, 'p1', 0x2f, True))
        self.assertEqual(p['fields'][0xe0], 1.4)
        self.assertEqual(p['powers'][3], 18.0)

    def test_a_stop_line_with_and_without_the_multiplier(self):
        self.assertEqual(cf.parse_line(stop(5, '22.680'))['gained'], 22.68)
        self.assertEqual(cf.parse_line(stop(5, '22.930', '1.4000'))['gained'], 22.93)
        self.assertIsNone(cf.parse_line('[FrameStrip] p1 status=0x2f'))


class Summary(unittest.TestCase):
    def lines(self, hold, mul):
        out = [probe(100, 0x2d, False, 1.0)]
        out += [probe(106, 0x2e, False, 1.0)]
        out += [probe(106 + hold, 0x2f, False, mul), probe(110 + hold, 0x2f, True, mul)]
        out += [stop(111 + hold, 13.0 * mul * 1.26, mul)]
        return [cf.parse_line(x) for x in out]

    def test_the_hold_is_read_off_the_frame_counter(self):
        self.assertEqual(cf.swing_summary(self.lines(30, 1.2))['hold'], 30)
        self.assertEqual(cf.swing_summary(self.lines(0, 1.0))['hold'], 0)

    def test_only_the_charged_offset_differs_between_levels(self):
        swings = [(0, 8, 0, self.lines(0, 1.0)), (66, 8, 0, self.lines(66, 1.4))]
        table = cf.field_table(swings)
        differing = [off for off in table if len({tuple(sorted(v)) for v in table[off].values()}) > 1]
        self.assertEqual(differing, [0xe0])


if __name__ == '__main__':
    unittest.main()
