"""Reject wrong-owner, failed-read, stale-ID and non-entry trace observations."""
import importlib.util
import json
from pathlib import Path
import re
import unittest

spec = importlib.util.spec_from_file_location('steps', Path(__file__).with_name('steps.py'))
steps = importlib.util.module_from_spec(spec)
spec.loader.exec_module(steps)


class TraceTests(unittest.TestCase):
    def record(self):
        return {'event': 'entry', 'point_id': '17', 'captures': [
            {'expression': e, 'error': '', 'value': str(v)}
            for e, v in [('x0+0xe0', 1), ('x1+0x18', 74), ('x2', 1)]]}

    def match(self, record):
        pattern = steps.trace_pattern('17', [('x0+0xe0', 1), ('x1+0x18', 74), ('x2', 1)])
        return re.search(pattern, json.dumps(record))

    def test_accepts_actual_id_and_typed_native_fields(self):
        self.assertIsNotNone(self.match(self.record()))

    def test_rejects_wrong_role_target_and_flag(self):
        for i in range(3):
            r = self.record();r['captures'][i]['value'] = '0'
            self.assertIsNone(self.match(r))

    def test_rejects_failed_memory_read(self):
        r = self.record();r['captures'][0]['error'] = 'unreadable'
        self.assertIsNone(self.match(r))

    def test_rejects_other_point_and_return(self):
        for key, value in [('point_id', '1'), ('event', 'return')]:
            r = self.record();r[key] = value
            self.assertIsNone(self.match(r))

    def test_resolution_refuses_pending_point(self):
        class Context:
            def call(self, method):
                return {'points': [{'target': steps.INSTALL, 'id': '17', 'state': 'pending'}]}
        with self.assertRaises(RuntimeError):steps._point(Context(), steps.INSTALL)


if __name__ == '__main__':unittest.main()
