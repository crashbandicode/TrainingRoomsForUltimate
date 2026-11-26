#!/usr/bin/python3
"""Host tests for the provenance regeneration check."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


class ProvenanceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        base = Path(self.temp.name)
        self.root = base / "root"
        self.provenance = self.root / "docs" / "provenance"
        self.provenance.mkdir(parents=True)
        self.stages = base / "stages"
        self.stage = self.stages / "s1" / "stage"
        self.stage.mkdir(parents=True)
        self.upstream = base / "upstream"
        self.upstream.mkdir()
        for folder, name, contents in (
            (self.stage, "new.txt", "new\n"),
            (self.stage, "identical.txt", "same\n"),
            (self.upstream, "identical.txt", "same\n"),
            (self.stage, "changed.txt", "fork\n"),
            (self.upstream, "changed.txt", "upstream\n"),
            (self.upstream, "dropped.txt", "dropped\n"),
        ):
            (folder / name).write_text(contents, encoding="utf-8")
        for package in ("pkg4", "pkg6"):
            self.manifest(package, "s1")
        (self.provenance / "provenance-files.json").write_text(
            json.dumps({"upstream": str(self.upstream)}), encoding="utf-8")
        bindir = base / "bin"
        bindir.mkdir()
        rg = bindir / "rg"
        rg.write_text(
            "#!/usr/bin/python3\n"
            "import pathlib, sys\n"
            "if len(sys.argv) != 3 or sys.argv[1] != '--files': sys.exit(2)\n"
            "for path in sorted(pathlib.Path(sys.argv[2]).rglob('*')):\n"
            "    if path.is_file(): print(path.resolve())\n",
            encoding="utf-8",
        )
        rg.chmod(0o755)
        self.env = {**os.environ, "TR_CARRY_STAGES": str(self.stages),
                    "PATH": str(bindir) + os.pathsep + os.environ.get("PATH", "")}
        out = self.run_check("--write")
        self.assertEqual(out.returncode, 0, out.stdout + out.stderr)
        self.assertTrue(out.stdout.rstrip().endswith("wrote 4 provenance files from stage s1"))

    def manifest(self, package, stage):
        (self.provenance / f"{package}-carry-manifest.json").write_text(
            json.dumps({"stage": stage}), encoding="utf-8")

    def run_check(self, *args):
        tool = Path(__file__).resolve().with_name("check_provenance.py")
        return subprocess.run([sys.executable, "-I", str(tool), "--root", str(self.root), *args],
                              env=self.env, capture_output=True, text=True)

    def test_the_four_files_pass_when_they_are_what_the_tools_write(self):
        before = {p: p.read_bytes() for folder in (self.stage, self.upstream) for p in folder.rglob("*") if p.is_file()}
        out = self.run_check()
        self.assertEqual(out.returncode, 0, out.stdout + out.stderr)
        self.assertEqual(out.stdout.strip(), "4 provenance files regenerate from stage s1")
        self.assertEqual(before, {p: p.read_bytes() for p in before})
        scan = json.loads((self.provenance / "provenance-files.json").read_text(encoding="utf-8"))
        self.assertEqual(scan["summary"], {"ours-new": 1, "upstream-identical": 1, "mixed": 1, "dropped-by-fork": 1})

    def test_every_changed_file_is_named(self):
        for name, old, new in (
            ("provenance-files.md", "| Path |", "| Poth |"),
            ("feature-groups.json", '"UNMAPPED"', '"UNMAPPEd"'),
        ):
            path = self.provenance / name
            text = path.read_text(encoding="utf-8")
            self.assertIn(old, text)
            path.write_text(text.replace(old, new, 1), encoding="utf-8")
        out = self.run_check()
        self.assertEqual(out.returncode, 1, out.stdout + out.stderr)
        self.assertIn("DIFFERS docs/provenance/provenance-files.md: line 1", out.stdout)
        self.assertIn("DIFFERS docs/provenance/feature-groups.json: line 2", out.stdout)
        self.assertTrue(out.stdout.rstrip().endswith("2 problem(s)"))

    def test_manifests_that_name_different_stages_fail(self):
        self.manifest("pkg4", "s2")
        out = self.run_check()
        self.assertEqual(out.returncode, 1, out.stdout + out.stderr)
        self.assertEqual(out.stdout.strip(), "FAILS: the carry manifests name different stages (s2, s1)")

    def test_a_missing_stage_fails(self):
        for package in ("pkg4", "pkg6"):
            self.manifest(package, "missing")
        out = self.run_check()
        self.assertEqual(out.returncode, 1, out.stdout + out.stderr)
        self.assertEqual(out.stdout.strip(), f"FAILS: {self.stages / 'missing' / 'stage'} is not on this machine")
        for package in ("pkg4", "pkg6"):
            self.manifest(package, "s1")
        path = self.provenance / "provenance-files.json"
        scan = json.loads(path.read_text(encoding="utf-8"))
        scan["upstream"] = str(self.upstream / "missing")
        path.write_text(json.dumps(scan), encoding="utf-8")
        out = self.run_check()
        self.assertEqual(out.returncode, 1, out.stdout + out.stderr)
        self.assertEqual(out.stdout.strip(), f"FAILS: {self.upstream / 'missing'} is not on this machine")

    def test_write_then_check_passes_after_the_stage_changes(self):
        (self.stage / "added.txt").write_text("added\n", encoding="utf-8")
        out = self.run_check()
        self.assertEqual(out.returncode, 1, out.stdout + out.stderr)
        self.assertIn("DIFFERS docs/provenance/provenance-files.json:", out.stdout)
        out = self.run_check("--write")
        self.assertEqual(out.returncode, 0, out.stdout + out.stderr)
        self.assertTrue(out.stdout.rstrip().endswith("wrote 4 provenance files from stage s1"))
        out = self.run_check()
        self.assertEqual(out.returncode, 0, out.stdout + out.stderr)
        self.assertEqual(out.stdout.strip(), "4 provenance files regenerate from stage s1")


if __name__ == "__main__":
    unittest.main()
