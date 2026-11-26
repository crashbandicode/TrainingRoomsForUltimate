#!/usr/bin/python3
"""Host tests for the package carry tools."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class CarryToolsTests(unittest.TestCase):
    def fixture(self, directory, package):
        base = Path(directory)
        root = base / "root"
        root.mkdir()
        stages = base / "stages"
        files = ["one.txt", "two.txt", "three.txt"]
        for stage in ("s1", "s2"):
            folder = stages / stage / "stage"
            folder.mkdir(parents=True)
            for name in files:
                (folder / name).write_text(f"source {name}\n", encoding="utf-8")
        entries = [
            {"src": name, "dest": f"dest/{name}", "crate": "root",
             "src_sha256": hashlib.sha256((stages / "s1" / "stage" / name).read_bytes()).hexdigest(),
             "dest_sha256": ""}
            for name in files
        ]
        manifest = root / "manifest.json"
        manifest.write_text(json.dumps({"stage": "old", "modules": {}, "files": entries}), encoding="utf-8")
        out = self.run_tool(package, root, stages, "carry", "--stage", "s1")
        self.assertEqual(out.returncode, 0, out.stdout + out.stderr)
        return root, stages, manifest, entries

    def run_tool(self, package, root, stages, *args):
        tool = Path(__file__).resolve().with_name(f"carry_{package}.py")
        return subprocess.run([sys.executable, "-I", str(tool), *args, "--root", str(root), "--manifest", "manifest.json"],
                              env={**os.environ, "TR_CARRY_STAGES": str(stages)}, capture_output=True, text=True)

    def snapshot(self, root, manifest, entries):
        return manifest.read_bytes(), [
            (root / entry["dest"]).read_bytes() if (root / entry["dest"]).exists() else None
            for entry in entries
        ]

    def test_carry_writes_nothing_when_a_source_changed(self):
        for package in ("pkg4", "pkg6"):
            with self.subTest(package=package), tempfile.TemporaryDirectory() as directory:
                root, stages, manifest, entries = self.fixture(directory, package)
                (root / entries[0]["dest"]).write_bytes(b"first destination must survive\n")
                (stages / "s2" / "stage" / entries[1]["src"]).write_bytes(b"changed source\n")
                data = json.loads(manifest.read_text(encoding="utf-8"))
                data["files"][2]["removed"] = "fixture"
                manifest.write_text(json.dumps(data), encoding="utf-8")
                before = self.snapshot(root, manifest, entries)
                out = self.run_tool(package, root, stages, "carry", "--stage", "s2")
                self.assertEqual(out.returncode, 1, out.stdout + out.stderr)
                self.assertEqual(self.snapshot(root, manifest, entries), before)
                self.assertTrue(out.stdout.rstrip().endswith("nothing written"), out.stdout + out.stderr)

    def test_carry_writes_nothing_when_an_edit_does_not_apply(self):
        for package in ("pkg4", "pkg6"):
            with self.subTest(package=package), tempfile.TemporaryDirectory() as directory:
                root, stages, manifest, entries = self.fixture(directory, package)
                data = json.loads(manifest.read_text(encoding="utf-8"))
                data["files"][1]["edits"] = [{"old": "absent", "new": "replacement", "reason": "fixture"}]
                manifest.write_text(json.dumps(data), encoding="utf-8")
                (root / entries[0]["dest"]).write_bytes(b"first destination must survive\n")
                before = self.snapshot(root, manifest, entries)
                out = self.run_tool(package, root, stages, "carry", "--stage", "s1")
                self.assertEqual(out.returncode, 1, out.stdout + out.stderr)
                self.assertEqual(self.snapshot(root, manifest, entries), before)
                self.assertTrue(out.stdout.rstrip().endswith("nothing written"), out.stdout + out.stderr)

    def test_carry_writes_the_stage_name_when_all_is_well(self):
        for package in ("pkg4", "pkg6"):
            with self.subTest(package=package), tempfile.TemporaryDirectory() as directory:
                root, stages, manifest, _ = self.fixture(directory, package)
                data = json.loads(manifest.read_text(encoding="utf-8"))
                data["stage"] = "old"
                manifest.write_text(json.dumps(data), encoding="utf-8")
                out = self.run_tool(package, root, stages, "carry", "--stage", "s1")
                self.assertEqual(out.returncode, 0, out.stdout + out.stderr)
                self.assertEqual(json.loads(manifest.read_text(encoding="utf-8"))["stage"], "s1")

    def test_verify_lists_every_failing_file(self):
        for package in ("pkg4", "pkg6"):
            with self.subTest(package=package), tempfile.TemporaryDirectory() as directory:
                root, stages, manifest, entries = self.fixture(directory, package)
                data = json.loads(manifest.read_text(encoding="utf-8"))
                for entry in data["files"][:2]:
                    entry["edits"] = [{"old": "absent", "new": "replacement", "reason": "fixture"}]
                manifest.write_text(json.dumps(data), encoding="utf-8")
                out = self.run_tool(package, root, stages, "verify", "--stage", "s1")
                self.assertEqual(out.returncode, 1, out.stdout + out.stderr)
                for entry in entries[:2]:
                    self.assertIn(f"EDIT FAILS {entry['src']}:", out.stdout)


if __name__ == "__main__":
    unittest.main()
