import importlib.util
from pathlib import Path
import sys
import unittest
import tempfile
import json

TOOLS = Path(__file__).resolve().parents[1] / 'tools'
sys.path.insert(0, str(TOOLS))
spec = importlib.util.spec_from_file_location('f6_matched_table', TOOLS / 'f6_matched_table.py')
table = importlib.util.module_from_spec(spec)
spec.loader.exec_module(table)


def stall(ms, gap, owner=0):
    return (f'{ms} #7 [BuildSplit] stall owner={owner} gap_us={gap} loader=0 outer=0 '
            'installed=1 changing=0 count=1 kinds=[6] flags=0x0')


class MatchedTableTests(unittest.TestCase):
    def test_missing_cache_is_explicit_and_error_invalidates_complete_pair(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)
            self.assertFalse(table.cache_evidence(path)['complete'])
            for phase in ('before', 'after'):
                (path / f'cache-{phase}.json').write_text(json.dumps({'utc': phase, 'roots': {
                    name: {} for name in ('user/shader','user/cache','xdg-cache')}}))
            self.assertTrue(table.cache_evidence(path)['complete'])
            (path / 'cache-error.txt').write_text('inventory failed')
            self.assertFalse(table.cache_evidence(path)['complete'])

    def test_absent_is_unknown(self):
        row = table.window([], 100, 300)
        self.assertIsNone(row['endpoint_sample_max_us'])
        self.assertIsNone(row['contained_sample_max_us'])

    def test_other_role_and_outside_window_do_not_raise_max(self):
        row = table.window([stall(200, 70000), stall(200, 100000, 1), stall(400, 900000)], 100, 300)
        self.assertEqual(row['contained_sample_max_us'], 70000)
        self.assertEqual(len(row['samples']), 1)

    def test_boundary_crossing_kept_but_not_contained(self):
        row = table.window([stall(120, 90000), stall(300, 80000)], 100, 300)
        self.assertEqual(row['endpoint_sample_max_us'], 90000)
        self.assertIsNone(row['contained_sample_max_us'])

    def test_exact_millisecond_boundary_is_conservative(self):
        row = table.window([stall(170, 70000)], 100, 300)
        self.assertIsNone(row['contained_sample_max_us'])

    def test_first_and_repeat_are_press_three_and_eleven(self):
        transactions = [dict(phase='bootstrap', role=0, press=1, target=45,
                             submit='100 submit', ready='110 ready', request_to_ready_ms=10)]
        for press in range(1, 12):
            transactions.append(dict(phase='lap', role=0, press=press, target=6 if press in (3,11) else 1,
                                     submit=f'{1000*press} submit', ready=f'{1000*press+press} ready',
                                     request_to_ready_ms=press))
        transactions.extend([dict(phase='lap', role=1, press=1, target=1)]*5)
        proof=dict(transactions=transactions, label='synthetic', identities={}, native_baseline=True)
        row=table.summarize(proof, [])
        self.assertEqual(row['first_kirby']['request_to_ready_ms'], 3)
        self.assertEqual(row['repeat_kirby']['request_to_ready_ms'], 11)
        self.assertEqual(row['cold_kirby_builds'], [])
        transactions[3]['target']=1
        with self.assertRaises(AssertionError):
            table.summarize(proof, [])


if __name__ == '__main__':
    unittest.main()
