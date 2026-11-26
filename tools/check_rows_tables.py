#!/usr/bin/python3
"""Compare generated seam tables with the committed tables."""
import argparse
import itertools
import pathlib
import subprocess
import sys
import tempfile


def main():
    root = pathlib.Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser()
    parser.add_argument("elf")
    parser.add_argument("--tables", type=pathlib.Path, default=root / "data" / "tables")
    args = parser.parse_args()
    generators = sorted((root / "tools").glob("gen_rows_*.py"))
    problems = 0
    with tempfile.TemporaryDirectory() as directory:
        for generator in generators:
            name = generator.stem.removeprefix("gen_") + ".ROWS.tsv"
            output = pathlib.Path(directory) / name
            table = args.tables / name
            result = subprocess.run(
                [sys.executable, "-I", str(generator), args.elf, str(output)],
                capture_output=True, text=True,
            )
            generated = result.returncode == 0 and output.is_file()
            if not generated:
                lines = (result.stdout + result.stderr).splitlines()
                print(f"FAILS {generator.name}: {lines[-1] if lines else 'no output'}")
                problems += 1
            if not table.is_file():
                print(f"MISSING {name}")
                problems += 1
            elif generated:
                actual, expected = output.read_bytes(), table.read_bytes()
                if actual != expected:
                    for line, (a, b) in enumerate(itertools.zip_longest(
                            actual.splitlines(keepends=True), expected.splitlines(keepends=True)), 1):
                        if a != b:
                            print(f"DIFFERS {name}: line {line}")
                            break
                    problems += 1
    if problems:
        print(f"{problems} problem(s)")
        return 1
    print(f"{len(generators)} tables regenerate from the ELF")
    return 0


if __name__ == "__main__":
    sys.exit(main())
