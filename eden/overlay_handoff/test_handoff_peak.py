"""Host tests of handoff_peak.py on synthetic traces: python3 -I eden/overlay_handoff/test_handoff_peak.py"""
import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import handoff_peak  # noqa: E402

POINTS = {'hand_off': '1', 'catch': '2', 'rebound': '3', 'wind': '4', 'wind_arg10': '5', 'wind_rad': '6',
          'wind_rad_arg9': '7', 'attack': '8'}


def trace(rows, extra=()):
    """A JSONL file of (point, thread) entries numbered from 10, plus raw extra lines; its path."""
    handle = tempfile.NamedTemporaryFile('w', suffix='.jsonl', delete=False, encoding='utf-8')
    with handle:
        for offset, (point, thread) in enumerate(rows):
            if point is None:
                continue
            handle.write(json.dumps({'event': 'entry', 'point_id': point, 'sequence': str(10 + offset),
                                     'thread_id': thread, 'frame': '1'}) + '\n')
        for line in extra:
            handle.write(line + '\n')
    return handle.name


class Peak(unittest.TestCase):
    def run_main(self, path, *more):
        return handoff_peak.main([path, '--frame', '0', *[f'--point={n}={p}' for n, p in POINTS.items()], *more])

    def test_only_complete_spans_count_and_each_point_is_counted_in_its_span(self):
        rows = [('1', '80'), ('0', '80'),                       # before the first frame entry: left out
                ('2', '80'), ('1', '80'), ('1', '81'), ('0', '80'),   # span 1: catch 1, hand_off 2
                ('0', '80'),                                    # span 2: nothing
                ('3', '80'), ('4', '80'), ('8', '80'), ('1', '80'), ('1', '80'), ('1', '80'), ('0', '80'),  # span 3
                ('1', '80')]                                    # after the last frame entry: left out
        entries, other, missing, duplicates = handoff_peak.read(trace(rows))
        self.assertEqual((other, missing, duplicates), ([], 0, 0))
        report = handoff_peak.measure(entries, '0', POINTS)
        self.assertEqual(report['frames'], 3)
        self.assertEqual(report['frame_threads'], ['80'])
        hand = report['points']['hand_off']
        self.assertEqual((hand['total'], hand['max'], hand['p50'], hand['threads']), (5, 3, 2, ['80', '81']))
        self.assertEqual(hand['max_at_sequence'], 16)
        self.assertEqual(report['points']['six']['total'], 3)
        self.assertEqual(report['points']['six']['max'], 2)
        self.assertEqual(report['points']['attack']['total'], 1)

    def test_the_percentile_is_the_nearest_rank(self):
        rows = [('0', '80')]
        for n in range(1000):                                   # 999 spans of one hand-off, one span of seven
            rows += [('1', '80')] * (7 if n == 500 else 1) + [('0', '80')]
        report = handoff_peak.measure(handoff_peak.read(trace(rows))[0], '0', POINTS)
        hand = report['points']['hand_off']
        self.assertEqual((report['frames'], hand['max'], hand['p99_9'], hand['p50']), (1000, 7, 1, 1))
        rows += [('1', '80')] * 7 + [('0', '80')]               # two of 1001: the 99.9th is the high one
        hand = handoff_peak.measure(handoff_peak.read(trace(rows))[0], '0', POINTS)['points']['hand_off']
        self.assertEqual(hand['p99_9'], 7)

    def test_a_missing_sequence_or_a_foreign_line_is_loss_and_fails(self):
        rows = [('0', '80'), ('1', '80'), ('0', '80')]
        self.assertEqual(self.run_main(trace(rows)), 0)
        gap = [('0', '80'), (None, None), ('1', '80'), ('0', '80')]
        self.assertEqual(handoff_peak.read(trace(gap))[2], 1)
        self.assertEqual(self.run_main(trace(gap)), 2)
        self.assertEqual(self.run_main(trace(gap), '--allow-loss'), 0)
        lossy = trace(rows, extra=['{"event":"loss","dropped":3}', 'not json'])
        self.assertEqual(len(handoff_peak.read(lossy)[1]), 2)
        self.assertEqual(self.run_main(lossy), 2)

    def test_a_return_record_is_a_record_but_not_an_entry(self):
        path = trace([('0', '80'), ('1', '80'), ('0', '80')],
                     extra=[json.dumps({'event': 'return', 'point_id': '1', 'sequence': '13', 'thread_id': '80'})])
        entries, other, missing, duplicates = handoff_peak.read(path)
        self.assertEqual((len(entries), other, missing, duplicates), (3, [], 0, 0))


if __name__ == '__main__':
    unittest.main()
