#!/usr/bin/python3
"""Host tests of tools/svc_clobber_check.py on synthetic disassembly (no ELF needed): python3 -I tools/test_svc_clobber_check.py"""
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import svc_clobber_check as sc  # noqa: E402

# rc.8 R6 (the docs lane's review of p23/rc8-waste, finding 1): at opt-level "z" the machine outliner moves caller code
# into OUTLINED_FUNCTION_*; a path that reaches one must go through its body and come back, not end as at a call.
DISASM = """
0000000000001000 <reads_after_the_outline>:
    1000: svc #0x34
    1004: bl 0x9000 <OUTLINED_FUNCTION_1>
    1008: ldr w0, [x10]
    100c: ret

0000000000002000 <the_outline_reads>:
    2000: svc #0x34
    2004: bl 0x9100 <OUTLINED_FUNCTION_2>
    2008: ret

0000000000003000 <the_outline_writes_first>:
    3000: svc #0x34
    3004: bl 0x9200 <OUTLINED_FUNCTION_3>
    3008: ldr w0, [x10]
    300c: ret

0000000000004000 <a_real_call_ends_the_path>:
    4000: svc #0x34
    4004: bl 0x5000 <callee>
    4008: ldr w0, [x10]
    400c: ret

0000000000005000 <callee>:
    5000: ret

0000000000006000 <a_tail_outline_reads>:
    6000: svc #0x34
    6004: b 0x9100 <OUTLINED_FUNCTION_2>

0000000000009000 <OUTLINED_FUNCTION_1>:
    9000: mov x0, x1
    9004: ret

0000000000009100 <OUTLINED_FUNCTION_2>:
    9100: add x0, x9, #0x1
    9104: ret

0000000000009200 <OUTLINED_FUNCTION_3>:
    9200: mov x10, x1
    9204: ret
"""


class OutlinedPaths(unittest.TestCase):
    def test_outlined_bodies_are_followed_and_the_caller_resumes_after_them(self):
        sites, violations = sc.check_funcs(sc.parse_text(DISASM))
        self.assertEqual(len(sites), 5)
        found = sorted((name, at, bad) for name, _, at, _, _, bad in violations)
        self.assertEqual(found, [
            ("a_tail_outline_reads", 0x9100, [9]),
            ("reads_after_the_outline", 0x1008, [10]),
            ("the_outline_reads", 0x9100, [9]),
        ])


if __name__ == "__main__":
    unittest.main()
