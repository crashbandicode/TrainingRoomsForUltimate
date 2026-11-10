#!/usr/bin/python3
"""Host tests of tools/stack_marks.py on synthetic logs and a synthetic table: python3 -I tools/test_stack_marks.py"""
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import stack_depth  # noqa: E402
import stack_marks as sm  # noqa: E402

TABLE = """
    row(ThreadId::Menu, c"tr-menu", 56 * KIB, 32_432, 30_848, "Eden st-02", "ui::worker::run"),
    row(ThreadId::Archive, c"tr-archive", 28 * KIB, 2_672, 7_840, "Eden st-02", "ui::archive::prepare::{closure#2}"),
"""
# final-surface-cov-4e336da4's tr-menu marks, before the combo crop overflowed its 56 KiB.
LOG = """1178460428 #49 [ThreadStack] high_water name=tr-menu used=34640 size=57344 free=22704 low=false
1178802081 #1627 [ThreadStack] high_water name=tr-menu used=51600 size=57344 free=5744 low=true
1178832496 #1743 [ThreadStack] high_water name=tr-menu used=54960 size=57344 free=2384 low=true
1178832500 #1744 [ThreadStack] high_water name=tr-archive used=6000 size=28672 free=22672 low=false
"""


class StackMarks(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.root = Path(self.dir.name)
        (self.root / "run" / "out").mkdir(parents=True)
        (self.root / "run" / "out" / "practice-diagnostics.log").write_text(LOG)
        self.rows = stack_depth.table_rows(TABLE)

    def tearDown(self):
        self.dir.cleanup()

    def test_the_largest_mark_of_each_thread_is_kept_from_a_folder(self):
        found = sm.marks([self.root / "run"])
        self.assertEqual(found["tr-menu"][0], 54960)
        self.assertEqual(found["tr-archive"][0], 6000)

    def test_the_cov_crop_marks_fail_against_the_stale_row(self):
        lines, bad = sm.check(sm.marks([self.root]), self.rows)
        self.assertEqual(bad, 1, lines)
        self.assertIn("past the recorded measured 30848", "\n".join(lines))

    def test_a_mark_within_its_row_passes(self):
        rows = stack_depth.table_rows(TABLE.replace("32_432, 30_848", "50_208, 57_648").replace("56 * KIB", "88 * KIB"))
        lines, bad = sm.check(sm.marks([self.root]), rows)
        self.assertEqual(bad, 0, lines)

    def test_a_thread_without_a_row_is_held_to_its_own_stack(self):
        (self.root / "boot.log").write_text("1 #24 [ThreadStack] high_water name=boot used=33200 size=65536 free=32336 low=false\n")
        _, bad = sm.check(sm.marks([self.root]), self.rows)
        self.assertEqual(bad, 1, "only tr-menu's growth")
        (self.root / "boot.log").write_text("1 #24 [ThreadStack] high_water name=boot used=60000 size=65536 free=5536 low=true\n")
        _, bad = sm.check(sm.marks([self.root]), self.rows)
        self.assertEqual(bad, 2, "a rowless stack with less than the SDK reserve left fails")

    def test_the_command_exits_1_on_growth_and_0_without_lines(self):
        table = self.root / "thread_stacks.rs"
        table.write_text(TABLE)
        self.assertEqual(sm.main([str(self.root / "run"), "--table", str(table)]), 1)
        (self.root / "empty").mkdir()
        self.assertEqual(sm.main([str(self.root / "empty"), "--table", str(table)]), 0)


if __name__ == "__main__":
    unittest.main()
