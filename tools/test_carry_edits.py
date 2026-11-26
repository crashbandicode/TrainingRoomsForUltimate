#!/usr/bin/python3
"""Host tests for carry_edits.py: python3 -I tools/test_carry_edits.py"""
import contextlib
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import carry_edits

with tempfile.TemporaryDirectory() as directory:
    Path(directory, "manifest.json").write_text(json.dumps({"stage": "test", "modules": {}, "files": []}), encoding="utf-8")
    with patch.object(sys, "argv", ["carry_pkg4.py", "verify", "--dest-only", "--root", directory,
                                    "--manifest", "manifest.json"]), contextlib.redirect_stdout(io.StringIO()):
        import carry_pkg4


def lines(count):
    return [f"line {i}\n" for i in range(count)]


class CarryEditsTests(unittest.TestCase):
    def replay(self, base, wanted, edits):
        self.assertEqual(carry_pkg4.apply_edits({"src": "x", "edits": edits}, base.encode("utf-8")),
                         wanted.encode("utf-8"))

    def test_one_changed_block(self):
        base = lines(30)
        wanted = base.copy()
        wanted[13:16] = ["changed 1\n", "changed 2\n", "changed 3\n"]
        edits = carry_edits.derive("".join(base), "".join(wanted), "product")
        self.assertEqual(len(edits), 1)
        self.replay("".join(base), "".join(wanted), edits)

    def test_two_blocks_far_apart(self):
        base = lines(40)
        wanted = base.copy()
        wanted[5] = "early change\n"
        wanted[32] = "later change\n"
        edits = carry_edits.derive("".join(base), "".join(wanted), "product")
        self.assertEqual(len(edits), 2)
        self.assertIn("later change\n", edits[0]["new"])
        self.assertIn("early change\n", edits[1]["new"])
        self.replay("".join(base), "".join(wanted), edits)

    def test_repeated_text_grows_context_upward(self):
        repeated = [f"repeated {i}\n" for i in range(9)]
        base = ["first header\n", *repeated, *lines(15), "second header\n", *repeated]
        wanted = base.copy()
        wanted[-5] = "changed\n"
        self.assertEqual("".join(base).count("".join(repeated[1:8])), 2)
        edits = carry_edits.derive("".join(base), "".join(wanted), "product")
        self.assertEqual(len(edits), 1)
        self.assertTrue(edits[0]["old"].startswith("second header\n"))
        self.replay("".join(base), "".join(wanted), edits)

    def test_pure_insertion(self):
        base = lines(20)
        wanted = base[:10] + ["inserted\n"] + base[10:]
        edits = carry_edits.derive("".join(base), "".join(wanted), "insert")
        self.assertEqual(len(edits), 1)
        self.replay("".join(base), "".join(wanted), edits)

    def test_pure_deletion(self):
        base = lines(20)
        wanted = base[:10] + base[11:]
        edits = carry_edits.derive("".join(base), "".join(wanted), "delete")
        self.assertEqual(len(edits), 1)
        self.replay("".join(base), "".join(wanted), edits)

    def test_first_line_changed(self):
        base = lines(20)
        wanted = base.copy()
        wanted[0] = "first changed\n"
        edits = carry_edits.derive("".join(base), "".join(wanted), "first")
        self.assertEqual(len(edits), 1)
        self.assertTrue(edits[0]["old"].startswith("line 0\n"))
        self.replay("".join(base), "".join(wanted), edits)

    def test_no_unique_context_is_refused(self):
        base = "same\n" * 20
        wanted = "same\n" * 10 + "inserted\n" + "same\n" * 10
        for count in range(1, 14):
            self.assertGreater(base.count("same\n" * count), 1)
        with self.assertRaisesRegex(SystemExit, "no unique context for the block at base line"):
            carry_edits.derive(base, wanted, "insert")

    def test_reasons_split_at_the_tests_marker(self):
        base = lines(45)
        base[20] = "# tests begin\n"
        wanted = base.copy()
        wanted[5] = "product change\n"
        wanted[35] = "test change\n"
        edits = carry_edits.derive("".join(base), "".join(wanted), "product", "tests", "# tests begin")
        self.assertEqual([edit["reason"] for edit in edits], ["tests", "product"])
        self.replay("".join(base), "".join(wanted), edits)

    def test_crlf_is_kept(self):
        base = "".join(lines(20)).replace("\n", "\r\n")
        wanted = base.replace("line 10\r\n", "changed\r\n")
        edits = carry_edits.derive(base, wanted, "crlf")
        self.assertIn("\r\n", edits[0]["old"])
        self.replay(base, wanted, edits)

    def test_manifest_is_written_as_the_carry_tool_writes_it(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            base = "".join(lines(20))
            wanted = base.replace("line 10\n", "changed\n")
            (root / "base").write_text(base, encoding="utf-8")
            (root / "wanted").write_text(wanted, encoding="utf-8")
            old = {"reason": "existing", "old": "old", "new": "new"}
            manifest = {"files": [{"dest": "wanted", "edits": [old]}]}
            path = root / "pkg4-carry-manifest.json"
            path.write_text(json.dumps(manifest), encoding="utf-8")
            args = ["wanted", str(root / "base"), "--reason", "new reason", "--root", directory,
                    "--manifest", "pkg4-carry-manifest.json"]
            with contextlib.redirect_stdout(io.StringIO()):
                carry_edits.main(args)
            text = path.read_text(encoding="utf-8")
            result = json.loads(text)
            self.assertEqual(text, json.dumps(result, indent=1) + "\n")
            self.assertEqual(result["files"][0]["edits"][0], old)
            new = result["files"][0]["edits"][1:]
            self.assertEqual(len(new), 1)
            self.replay(base, wanted, new)
            before = path.read_bytes()
            with contextlib.redirect_stdout(io.StringIO()):
                carry_edits.main(args + ["--dry-run"])
            self.assertEqual(path.read_bytes(), before)

    def test_identical_files_change_nothing(self):
        base = "".join(lines(10))
        edits = carry_edits.derive(base, base, "same")
        self.assertEqual(edits, [])
        self.replay(base, base, edits)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "base").write_text(base, encoding="utf-8")
            (root / "wanted").write_text(base, encoding="utf-8")
            manifest = root / "manifest.json"
            manifest.write_text('{"files": []}', encoding="utf-8")
            before = manifest.read_bytes()
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                carry_edits.main(["wanted", str(root / "base"), "--reason", "same", "--root", directory,
                                  "--manifest", "manifest.json"])
            self.assertEqual(output.getvalue().strip(), "no change")
            self.assertEqual(manifest.read_bytes(), before)


if __name__ == "__main__":
    unittest.main()
