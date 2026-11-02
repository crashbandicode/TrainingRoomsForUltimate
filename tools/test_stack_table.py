#!/usr/bin/python3
"""Host tests of tools/stack_table.py on synthetic log lines: python3 -I tools/test_stack_table.py"""
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import stack_table as st  # noqa: E402

LINES = [
    "1 #1 [ThreadStack] spawn name=tr-heartbeat base=0x10dd820000 size=0x10000 top=0x10dd830000 sp=0x10dd82ffa0 kind=0xb table=0x10000",
    "2 #2 [ThreadStack] high_water name=tr-heartbeat used=4096 size=65536 free=61440 low=false",
    "3 #3 [ThreadStack] high_water name=tr-heartbeat used=9000 size=65536 free=56536 low=false",
    "4 #4 [ThreadStack] high_water name=tr-menu used=250000 size=262144 free=12144 low=true",
    "5 #5 [ThreadStack] exit name=tr-settings-load used=7000 size=262144 free=255144 low=false",
    "6 #6 [HookStack] site=native_service calls=10 min_free=300000 stack=0x80000 base=0x1000000",
    "7 #7 [HookStack] site=native_service calls=50 min_free=250000 stack=0x80000 base=0x1000000",
    "8 #8 [HookStack] site=ui_archive calls=9 min_free=90000 stack=0x20000 base=0x2000000",
    "9 #9 [TrainingRooms] something else",
]


class StackTable(unittest.TestCase):
    def test_threads_take_the_largest_use_and_the_least_free(self):
        threads, _, _ = st.collect(LINES)
        self.assertEqual(threads["tr-heartbeat"]["used"], 9000)
        self.assertEqual(threads["tr-heartbeat"]["free"], 56536)
        self.assertFalse(threads["tr-heartbeat"]["low"])
        self.assertTrue(threads["tr-menu"]["low"], "a low=true line is kept")
        self.assertEqual(threads["tr-settings-load"]["used"], 7000, "exit lines count")

    def test_hooks_take_the_least_room_and_its_region(self):
        _, hooks, _ = st.collect(LINES)
        self.assertEqual(hooks["native_service"]["min_free"], 250000)
        self.assertEqual(hooks["native_service"]["calls"], 50)
        self.assertEqual(hooks["ui_archive"]["stack"], 0x20000)

    def test_spawns_and_tables(self):
        _, _, spawns = st.collect(LINES)
        self.assertEqual(spawns, [("tr-heartbeat", 0x10dd820000, 0x10000, 0x10dd830000)])
        text = st.tables(*st.collect(LINES)[:2])
        self.assertIn("| tr-menu | 262144 | 250000 | 12144 | yes |", text)
        # hooks sorted by the least room first
        self.assertLess(text.index("ui_archive"), text.index("native_service"))


if __name__ == "__main__":
    unittest.main()
