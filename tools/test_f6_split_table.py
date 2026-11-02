#!/usr/bin/env python3
"""Host tests of tools/f6_split_table.py on synthetic log lines in the plugin's formats (`model::build_split` Display, the carried
`[GroupSwap] swap` line). Run: python3 -I tools/test_f6_split_table.py"""
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import f6_split_table as t  # noqa: E402

LOG = """\
100 [GroupSwap] build owner=0 group=0 kinds=[2d, 2c, 1c] mode=async reason=-
101 [BuildSplit] setup owner=0 group=0 kinds=[2d, 2c, 1c] build_of_boot=1 carried_setup_us=77000 plugin=900/9 unprepare_17f26c0=400/1 roster_17e4f50=300/1 log=200/2 memory_svc=100/1 models_654dc0=1000/1 contexts_655320=500/1 control_64f820=73600/1 allocate_3ae5f0=3000/3 setup_us=77000 total_us=80000 largest=control_64f820
150 [BuildSplit] queue owner=0 group=0 free_tasks=20/1 allocate_3ae5f0=3000/3 task_392e290=60/3 setup_us=20 total_us=3100 largest=allocate_3ae5f0
102 [BuildSplit] settle owner=0 group=0 settle_of_boot=1 install_cells=10/1 free_tasks=50/1 verify_ack=900/1 setup_us=0 total_us=960 largest=verify_ack
103 [GroupSwap] swap owner=0 kind=0x2d request_to_ready_us=2300000 bodies=[..] max_tick_gap_us=100211 ticks=138
104 [BuildSplit] gaps owner=0 kind=0x2d ticks=138 top=[t1:100211:setup, t90:20000:settle, t3:17000:poll]
200 [BuildSplit] setup owner=0 group=0 kinds=[6, 2d, 2c] build_of_boot=2 carried_setup_us=13000 plugin=800/9 control_64f820=10000/1 models_654dc0=1200/1 setup_us=12000 total_us=13000 largest=control_64f820
201 [GroupSwap] swap owner=0 kind=0x6 request_to_ready_us=2500000 bodies=[..] max_tick_gap_us=99000 ticks=150
202 [BuildSplit] gaps owner=0 kind=0x6 ticks=150 top=[t140:99000:settle, t1:28000:setup]
300 [BuildSplit] setup owner=1 group=0 kinds=[8, f] build_of_boot=3 carried_setup_us=14000 control_64f820=12000/1 setup_us=12000 total_us=14000 largest=control_64f820
"""


class SplitTable(unittest.TestCase):
    def setUp(self):
        self.builds = t.parse(LOG.splitlines())

    def test_each_build_collects_its_settlement_swap_and_gaps(self):
        b = self.builds[0]
        self.assertEqual((b["owner"], b["kinds"], b["build_of_boot"], b["carried_setup_us"]), (0, [0x2d, 0x2c, 0x1c], 1, 77000))
        self.assertEqual(b["steps"]["control_64f820"], (73600, 1))
        self.assertEqual(b["largest"], "control_64f820")
        self.assertEqual(b["settle"]["steps"]["verify_ack"], (900, 1))
        self.assertEqual((b["max_tick_gap_us"], b["request_to_ready_us"]), (100211, 2300000))
        self.assertEqual(b["gaps"][0], {"tick": 1, "us": 100211, "after": "setup"})
        self.assertEqual(b["queue"]["largest"], "allocate_3ae5f0")
        self.assertEqual(b["queue"]["total_us"], 3100)
        k = self.builds[1]
        self.assertIsNone(k["queue"])
        self.assertEqual(k["kinds"], [6, 0x2d, 0x2c])
        self.assertIsNone(k["settle"])
        self.assertEqual(k["gaps"][0]["after"], "settle")

    def test_a_build_without_its_swap_line_keeps_none(self):
        self.assertEqual(len(self.builds), 3)
        self.assertIsNone(self.builds[2]["max_tick_gap_us"])
        self.assertEqual(self.builds[2]["gaps"], [])

    def test_compare_puts_the_first_use_step_first(self):
        rows = t.compare(self.builds)
        self.assertEqual(rows[0], ("control_64f820", 73600, 11000))
        # a step the later builds never ran has median 0, not a guess
        self.assertIn(("unprepare_17f26c0", 400, 0), rows)

    def test_compare_without_a_first_build_is_empty(self):
        self.assertEqual(t.compare(self.builds[1:]), [])

    def test_table_has_one_row_per_build_and_the_comparison(self):
        text = t.table(self.builds)
        self.assertEqual(sum(1 for line in text.splitlines() if line.startswith("| 1 |") or line.startswith("| 2 |") or line.startswith("| 3 |")), 3)
        self.assertIn("| control_64f820 | 73600 | 11000 |", text)
        self.assertIn("[2d, 2c, 1c]", text)


if __name__ == "__main__":
    unittest.main()
