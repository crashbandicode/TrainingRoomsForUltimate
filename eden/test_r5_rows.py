"""Acceptance-driver regressions; execute the actual row functions with log fixtures."""
import ast
import json
import re
import tempfile
import types
import unittest
from pathlib import Path

SOURCE = Path(__file__).with_name("rows_s6.py")
TREE = ast.parse(SOURCE.read_text())
FUNCTIONS = ast.Module(body=[n for n in TREE.body if isinstance(n, ast.FunctionDef)
                            and n.name in ("r_r5", "r_r5_soak")], type_ignores=[])
CODE = compile(FUNCTIONS, str(SOURCE), "exec")


def samples(count=19, gap=10000, allocation=None, preload=True, stalled=False):
    return [f"{1000 + i * gap} #1 hook_stats allocations={int(allocation is not None and i >= allocation)} "
            f"delta={int(i == allocation)} hook_calls=100 "
            f"hooks=ui_pass_calculate:{100 + i}," +
            (f"roster_preload:{100 if stalled else 100 + i}" if preload else "cpu_frame:10") +
            f" allocs=roster_preload:{int(allocation is not None and i >= allocation)}"
            for i in range(count)]


class Rows(unittest.TestCase):
    def run_row(self, name, lines, plan="slots=2 fighters=true hats=true"):
        rows = []
        ticks = iter(range(1000))
        with tempfile.TemporaryDirectory(prefix="pkg6-r5-fixture-") as directory:
            scope = dict(re=re, json=json, OUT=Path(directory),
                         time=types.SimpleNamespace(monotonic=lambda: next(ticks), sleep=lambda _: None),
                         reload=lambda *_: (0, "diag_settings_reload"),
                         wait_for=lambda *_: "[CyclePreload] applied " + plan,
                         row=lambda title, ok, evidence: rows.append((title, bool(ok))),
                         after=lambda _: lines, read_diag=lambda: lines,
                         stamp_of=lambda line: int(line.split()[0]))
            exec(CODE, scope)
            scope[name]()
        return rows

    def test_accepts_live_zero_soak(self):
        self.assertTrue(all(ok for _, ok in self.run_row("r_r5_soak", samples())))

    def test_rejects_incomplete_or_inactive_soak(self):
        for lines in (samples(count=2), [], samples(preload=False), samples(stalled=True),
                      samples(count=7, gap=30000), samples(allocation=0), samples(allocation=10)):
            with self.subTest(lines=len(lines)):
                self.assertTrue(any(not ok for _, ok in self.run_row("r_r5_soak", lines)))

    def test_requires_fighters_and_hats(self):
        self.assertFalse(self.run_row("r_r5_soak", samples(), "slots=2 fighters=true hats=false")[0][1])

    def test_full_scan_never_skips_first_allocation_sample(self):
        self.assertFalse(self.run_row("r_r5", samples(allocation=0))[0][1])
        self.assertTrue(self.run_row("r_r5", samples())[0][1])


if __name__ == "__main__":
    unittest.main()
