"""Exercise F6 row failures without Eden or a claim."""
import argparse
import importlib.util
import json
from pathlib import Path
import tempfile
import types
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("f6_rows", Path(__file__).resolve().parents[1] / "eden/f6_rows.py")
rows = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rows)


class Rows(unittest.TestCase):
    def run_rows(self, native, complete, prefetch=True):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "route.py").touch()
            diag = root / "practice-diagnostics.log"
            diag.write_text("[GroupSwap] prefetched\n" if prefetch else "")
            presses = []

            def press(buttons, hold):
                presses.append(buttons)
                if complete:
                    with diag.open("a") as stream:
                        stream.write("[CharacterRoster] queued owner=0\n[CharacterRoster] complete owner=0 kind=0x6\n")

            session = types.SimpleNamespace(press=press, t=types.SimpleNamespace(shot=lambda name: root / (name + ".png")))
            modules = {"params": types.SimpleNamespace(load=lambda: types.SimpleNamespace(data_dir=root)),
                       "route": types.SimpleNamespace(session=lambda *_: session)}
            args = argparse.Namespace(harness=str(root), label="f6-99", native_reload=native,
                                      settle=15, p1=2, cpu=0, gap=6)
            ticks = iter(range(0, 10000, 5))
            error = None
            with patch.dict("sys.modules", modules), patch.object(rows, "HERE", root), \
                 patch.object(rows.time, "time", side_effect=lambda: next(ticks)), \
                 patch.object(rows.time, "sleep"):
                try:
                    rows.laps(args)
                except SystemExit as exc:
                    error = str(exc)
            receipt = root / "runs/f6-99/laps.jsonl"
            records = [json.loads(line) for line in receipt.read_text().splitlines()] if receipt.exists() else []
            return presses, records, error

    def test_missing_prefetch_sends_no_input(self):
        presses, records, error = self.run_rows(False, False, prefetch=False)
        self.assertEqual((presses, records), ([], []))
        self.assertIn("no prefetched group", error)

    def test_missing_completion_retains_failure_and_stops(self):
        presses, records, error = self.run_rows(False, False)
        self.assertEqual(len(presses), 1)
        self.assertEqual(len(records), 1)
        self.assertIsNone(records[0]["done"])
        self.assertIn("GATE FAIL", error)

    def test_native_reload_requires_its_own_completion_without_prefetch(self):
        presses, records, error = self.run_rows(True, True, prefetch=False)
        self.assertIsNone(error)
        self.assertEqual(len(presses), 2)
        self.assertEqual(len(records), 2)
        self.assertTrue(all("[CharacterRoster] complete owner=0" in row["done"] for row in records))


if __name__ == "__main__":
    unittest.main()
