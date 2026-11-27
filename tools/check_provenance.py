#!/usr/bin/python3
"""Regenerate and compare the provenance scan and feature groups."""
import argparse
import itertools
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

STAGES = os.environ.get("TR_CARRY_STAGES") or "/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/stages"
FILES = ("provenance-files.json", "provenance-files.txt", "feature-groups.txt", "feature-groups.json")


def first_differing_line(a, b):
    for line, (left, right) in enumerate(itertools.zip_longest(
            a.splitlines(keepends=True), b.splitlines(keepends=True)), 1):
        if left != right:
            return line


def main():
    tools = Path(__file__).resolve().parent
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=tools.parent)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    provenance = args.root / "docs" / "provenance"
    try:
        pkg4 = json.loads((provenance / "pkg4-carry-manifest.json").read_text(encoding="utf-8"))["stage"]
        pkg6 = json.loads((provenance / "pkg6-carry-manifest.json").read_text(encoding="utf-8"))["stage"]
        if pkg4 != pkg6:
            print(f"FAILS: the carry manifests name different stages ({pkg4}, {pkg6})")
            return 1
        stage = Path(STAGES) / pkg4 / "stage"
        upstream = Path(json.loads((provenance / FILES[0]).read_text(encoding="utf-8"))["upstream"])
        for folder in (stage, upstream):
            if not folder.is_dir():
                print(f"FAILS: {folder} is not on this machine")
                return 1
        with tempfile.TemporaryDirectory() as directory:
            generated = Path(directory)
            commands = [
                ("provenance_scan.py", stage, upstream, generated / FILES[0], generated / FILES[1]),
                ("feature_groups.py", generated / FILES[0], generated / FILES[2], generated / FILES[3]),
            ]
            for script, *arguments in commands:
                result = subprocess.run([sys.executable, "-I", str(tools / script), *map(str, arguments)],
                                        capture_output=True, text=True)
                if result.returncode:
                    details = (result.stdout + result.stderr).splitlines()
                    print(f"FAILS: {script}: {details[-1] if details else 'no output'}")
                    return 1
                if args.write:
                    print(result.stdout, end="")
            outputs = {name: (generated / name).read_bytes() for name in FILES}
            records = args.root / "tools" / "provenance-data"
            targets = {name: (records if name.endswith(".txt") else provenance) / name for name in FILES}
            if args.write:
                records.mkdir(parents=True, exist_ok=True)
                for name, data in outputs.items():
                    targets[name].write_bytes(data)
                print(f"wrote 4 provenance files from stage {pkg4}")
                return 0
            problems = 0
            for name, data in outputs.items():
                target = targets[name]
                if not target.is_file():
                    line = 1
                else:
                    line = first_differing_line(data, target.read_bytes())
                if line is not None:
                    print(f"DIFFERS {target.relative_to(args.root).as_posix()}: line {line}")
                    problems += 1
            if problems:
                print(f"{problems} problem(s)")
                return 1
        print(f"4 provenance files regenerate from stage {pkg4}")
        return 0
    except (OSError, ValueError, KeyError) as error:
        print(f"FAILS: {error}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
