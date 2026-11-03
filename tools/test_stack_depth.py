#!/usr/bin/python3
"""Host tests of tools/stack_depth.py on synthetic disassembly (no ELF needed): python3 -I tools/test_stack_depth.py"""
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import stack_depth as sd  # noqa: E402

DISASM = """
0000000000001000 <small>:
    1000:      	stp	x29, x30, [sp, #-0x20]!
    1004:      	mov	x29, sp
    1008:      	bl	0x3000 <leaf>
    100c:      	ldp	x29, x30, [sp], #0x20
    1010:      	ret

0000000000002000 <big_probe>:
    2000:      	stp	x29, x30, [sp, #-0x60]!
    2004:      	stp	x28, x27, [sp, #0x10]
    2008:      	mov	x29, sp
    200c:      	sub	x9, sp, #0xe, lsl #12
    2010:      	sub	x9, x9, #0x940
    2014:      	sub	sp, sp, #0x1, lsl #12
    2018:      	cmp	sp, x9
    201c:      	b.le	0x2028 <big_probe+0x28>
    2020:      	str	xzr, [sp, #0x400]
    2024:      	b	0x2014 <big_probe+0x14>
    2028:      	mov	sp, x9
    202c:      	ldr	xzr, [sp]
    2030:      	bl	0x1000 <small>
    2034:      	blr	x8
    2038:      	b	0x4000 <tail>

0000000000003000 <leaf>:
    3000:      	sub	sp, sp, #0x40
    3004:      	add	sp, sp, #0x40
    3008:      	ret

0000000000004000 <tail>:
    4000:      	sub	sp, sp, #0x1, lsl #12
    4004:      	sub	sp, sp, #0x10
    4008:      	bl	0x5000 <memcpy@plt>
    400c:      	ret

0000000000005000 <memcpy@plt>:
    5000:      	br	x17

0000000000006000 <rec>:
    6000:      	stp	x29, x30, [sp, #-0x10]!
    6004:      	bl	0x6000 <rec>
    6008:      	ret

0000000000007000 <std::sys::thread::switch::Thread>::new::thread_start>:
    7000:      	sub	sp, sp, #0x70
    7004:      	blr	x8

0000000000008000 <<std::thread::lifecycle::spawn_unchecked<training_rooms::runtime::threads::spawn<training_rooms::runtime::compat::filesystem::init::{closure#0}, ()>::{closure#0}, ()>::{closure#1} as core::ops::function::FnOnce<()>>::call_once::{shim:vtable#0}>:
    8000:      	stp	x29, x30, [sp, #-0x10]!
    8004:      	bl	0x2000 <big_probe>
    8008:      	ret
"""


class StackDepth(unittest.TestCase):
    def setUp(self):
        self.funcs = sd.link(sd.parse(DISASM))

    def test_frames(self):
        f = self.funcs
        self.assertEqual(f[0x1000]["frame"], 0x20)
        # 0x60 saved, then the probe loop down to x9 = sp - 0xe940
        self.assertEqual(f[0x2000]["frame"], 0x60 + 0xe940)
        self.assertEqual(f[0x3000]["frame"], 0x40)
        self.assertEqual(f[0x4000]["frame"], 0x1010)

    def test_calls_tail_calls_imports_and_indirect(self):
        f = self.funcs[0x2000]
        self.assertEqual(f["calls"], {0x1000, 0x4000})
        self.assertEqual(f["indirect"], 1)
        self.assertEqual(self.funcs[0x4000]["calls"], set(), "an import is not followed")

    def test_deepest_path_and_recursion(self):
        total, path = sd.deepest(self.funcs, 0x2000)
        self.assertEqual(total, 0x60 + 0xe940 + 0x1010)
        self.assertEqual(path, [0x2000, 0x4000])
        self.assertEqual(sd.deepest(self.funcs, 0x6000)[0], 0x10)

    def test_table_check(self):
        text = '''row(ThreadId::TraceWriter, "tr-trace-writer", 64 * KIB, 0, 0, "", "compat::filesystem::init::{closure#0}"),
                  row(ThreadId::Probe, "tr-probe", 256 * KIB, 2_928, 0, "", "ui::probe::install::{closure#0}"),'''
        rows = sd.table_rows(text)
        self.assertEqual(rows[0], ("TraceWriter", "tr-trace-writer", 65536, 0, 0, "compat::filesystem::init::{closure#0}", 0))
        lines, bad = sd.check_table(self.funcs, rows)
        # 0x70 + 0x10 + 0xe9a0 + 0x1010 = 64,000 B static: the 64 KiB row fails (and its table_static of 0 too)
        self.assertEqual(bad, 1)
        self.assertIn("FAIL", lines[1])
        self.assertIn("no shim", lines[2])

    def test_required_matches_the_model(self):
        # model::thread_stacks::required: max * 1.25 + 16 KiB, whole pages
        self.assertEqual(sd.required(59_840, 0), 94_208)
        self.assertEqual(sd.required(0, 0), 16_384)

    def test_external_read_path_cannot_hide_behind_small_owned_frame_or_paint(self):
        text = '''row(ThreadId::Probe, "tr-probe", 24 * KIB, 3_440, 0, "", "ui::probe::install::{closure#0}").with_external(143_376),'''
        funcs = sd.link(sd.parse('''0000000000001000 <training_rooms::runtime::threads::imp::detached::<ui::probe::install::{closure#0}>>:
    1000: sub sp, sp, #0xd70
    1004: blr x8
    1008: ret
'''))
        rows = sd.table_rows(text)
        self.assertEqual(rows[0][-1], 143_376)
        lines, bad = sd.check_table(funcs, rows)
        self.assertEqual(bad, 1)
        self.assertIn("required= 200704 FAIL", lines[1])
        self.assertEqual(sd.required(3_440, 0), 24_576, "the old owned-only check admitted 24 KiB")
        fixed = text.replace("24 * KIB", "196 * KIB")
        self.assertEqual(sd.check_table(funcs, sd.table_rows(fixed))[1], 0)
        grown = fixed.replace("143_376", "147_472")
        self.assertEqual(sd.check_table(funcs, sd.table_rows(grown))[1], 1)


if __name__ == "__main__":
    unittest.main()
