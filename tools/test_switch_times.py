#!/usr/bin/env python3
"""Host tests of tools/switch_times.py: python3 -I tools/test_switch_times.py"""
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import switch_times as st  # noqa: E402

# The rc.6 console play test (artifacts/console-rc6-playtest-20261110/practice-diagnostics.log), P1 lines shortened.
CONSOLE = """706945 #324 [CharacterRoster] request handed to the ordinary Mii owner role=Player selection=Direct(TrioFighter(6))
707197 #326 [BodyBank] layout owner=0 target=0x6 cells=[6, 8, 1c] active=0 form=single
707197 #327 [MiiOrdinaryPlayer] submit ticket=Ticket { epoch: 2, entry: 1, sequence: 9 } outgoing=0x1c target=0x6 index=0 bank_build=true
707197 #329 [BodyBank] build owner=0 entry=0x1a8c25c000 members=Members { kinds: [6, 8, 28], count: 3, pair_lead: false, special: None }
714550 #333 [BodyBank] ready owner=0 index=0 kind=0x6 previous=Building bodies=[]
714550 #336 [MiiOrdinaryPlayer] ready ticket=Ticket { epoch: 2, entry: 1, sequence: 9 } target=0x6 index=0
767162 #415 [CharacterRoster] request handed to the ordinary Mii owner role=Player selection=Cycle
767163 #416 [ProcessMemory] window owner=0 target=0x36 members=[36, 1c, 8] admitted=0 of supplements=[1c, 8] admission=Refuse
767163 #417 [MiiOrdinaryPlayer] submit ticket=Ticket { epoch: 2, entry: 1, sequence: 10 } outgoing=0x6 target=0x36 index=0 bank_build=false
770552 #421 [BodyBank] canonical replacement owner=0; cache retired
770553 #424 [MiiOrdinaryPlayer] ready ticket=Ticket { epoch: 2, entry: 1, sequence: 10 } target=0x36 index=0
778753 #426 [CharacterRoster] request handed to the ordinary Mii owner role=Player selection=Cycle
778753 #430 [BodyBank] build owner=0 entry=0x1a8c25c000 members=Members { kinds: [6, 28, 8], count: 3, pair_lead: false, special: None }
786682 #434 [BodyBank] ready owner=0 index=1 kind=0x1c previous=Building bodies=[]
793949 #439 [CharacterRoster] request handed to the ordinary Mii owner role=Player selection=Cycle
793949 #440 [BodyBank] reuse owner=0 from=1 to=2 kind=0x8
794001 #442 [BodyBank] ready owner=0 index=2 kind=0x8 previous=Changing bodies=[]
"""

# The same cycle as the target-first build logs it (synthetic): the Reload builds G&W (0x1c) alone, a group builds behind it, and
# the next switch is a warm exchange into that group.
AFTER = """1000 #10 [CharacterRoster] request handed to the ordinary Mii owner role=Player selection=Cycle
1000 #11 [BodyBank] target first owner=0 target=0x1c lane=[8, 6, 36]; the other members build as groups
1000 #12 [BodyBank] build owner=0 entry=0x1 members=Members { kinds: [28, 4294967295, 4294967295], count: 1, pair_lead: false, special: None }
3100 #13 [BodyBank] ready owner=0 index=0 kind=0x1c previous=Building bodies=[]
3300 #14 [GroupSwap] plan owner=0 group=0 kinds=[6, 8] special=None bodies=2
5200 #15 [GroupSwap] background built owner=0 group=0 ok=true reason=built kinds=[6, 8] bodies=[] us=1800000
6000 #16 [CharacterRoster] request handed to the ordinary Mii owner role=Cpu selection=Cycle
6100 #17 [BodyBank] ready owner=1 index=0 kind=0x2 previous=Changing bodies=[]
7000 #18 [CharacterRoster] request handed to the ordinary Mii owner role=Player selection=Cycle
7000 #19 [GroupSwap] request owner=0 kind=0x8 group=0 member=1 via=2 from=0 state=Built form=single
7040 #20 [BodyBank] ready owner=0 index=1 kind=0x8 previous=Changing bodies=[]
7040 #21 [GroupSwap] swap owner=0 kind=0x8 request_to_ready_us=40000 bodies=[] max_tick_gap_us=17 ticks=3
"""


class SwitchTimes(unittest.TestCase):
    def lines(self, text):
        handle = tempfile.NamedTemporaryFile("w", suffix=".log", delete=False)
        handle.write(text)
        handle.close()
        self.addCleanup(os.unlink, handle.name)
        return st.read_lines([handle.name])

    def test_switch_times_reports_each_switch_and_its_path(self):
        rows = st.switches(self.lines(CONSOLE))
        self.assertEqual([(r["kind"], r["path"], r["ms"]) for r in rows],
                         [(0x6, "window 3", 7605), (0x36, "canonical", 3391), (0x1c, "window 3", 7929),
                          (0x8, "in-window", 52)])
        after = st.switches(self.lines(AFTER))
        self.assertEqual([(r["kind"], r["path"], r["ms"]) for r in after], [(0x1c, "target-first", 2100), (0x8, "group built", 40)])
        self.assertIn("| 1 | 0 | 1000 | 0x1c | target-first | 2100 |", st.table(after))

    def test_switch_times_counts_background_groups_built(self):
        rows = st.switches(self.lines(AFTER))
        self.assertEqual(rows[0]["background"], [{"ms": 5200, "group": 0, "kinds": [6, 8]}])
        self.assertEqual(rows[1]["background"], [])
        self.assertIn("group 0 0x6/0x8 at +2100 ms", st.table(rows))
        # The CPU's switch is its own role's; owner 1 sees it alone.
        cpu = st.switches(self.lines(AFTER), owner=1)
        self.assertEqual([(r["kind"], r["ms"]) for r in cpu], [(0x2, 100)])

    def test_logs_merge_by_sequence_and_a_switch_without_ready_is_reported(self):
        head, tail = CONSOLE.split("767162 ", 1)
        a = tempfile.NamedTemporaryFile("w", suffix=".log", delete=False)
        b = tempfile.NamedTemporaryFile("w", suffix=".log", delete=False)
        a.write("767162 " + tail); b.write(head + "767162 " + tail.splitlines()[0] + "\n")
        a.close(); b.close()
        self.addCleanup(os.unlink, a.name); self.addCleanup(os.unlink, b.name)
        rows = st.switches(st.read_lines([a.name, b.name]))
        self.assertEqual(len(rows), 4)
        open_row = st.switches(self.lines("1 #1 [CharacterRoster] request handed to the ordinary Mii owner role=Player selection=Cycle\n"))
        self.assertEqual((open_row[0]["ms"], open_row[0]["path"]), (None, None))
        self.assertIn("not ready", st.table(open_row))

    def test_a_previous_log_of_another_boot_is_its_own_session(self):
        # The console's `.previous` copy was an earlier boot: its sequence overlaps the current log's at other ticks.
        older = tempfile.NamedTemporaryFile("w", suffix=".log", delete=False)
        older.write("9000000 #415 [CharacterRoster] request handed to the ordinary Mii owner role=Player selection=Cycle\n"
                    "9000060 #416 [BodyBank] reuse owner=0 from=0 to=1 kind=0x8\n"
                    "9000090 #417 [BodyBank] ready owner=0 index=1 kind=0x8 previous=Changing bodies=[]\n")
        current = tempfile.NamedTemporaryFile("w", suffix=".log", delete=False)
        current.write(CONSOLE)
        older.close(); current.close()
        self.addCleanup(os.unlink, older.name); self.addCleanup(os.unlink, current.name)
        self.assertEqual(len(st.sessions([older.name, current.name])), 2)
        with self.assertRaises(ValueError):
            st.read_lines([older.name, current.name])
        rows = st.switches_by_session([older.name, current.name])
        self.assertEqual([(r["session"], r["kind"], r["ms"]) for r in rows],
                         [(0, 0x8, 90), (1, 0x6, 7605), (1, 0x36, 3391), (1, 0x1c, 7929), (1, 0x8, 52)])


if __name__ == "__main__":
    unittest.main()
