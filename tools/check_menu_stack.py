"""Regression: the recording worker must fit the existing menu stack, even with placement enabled."""
import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import stack_depth


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("elf", type=Path)
    parser.add_argument("--table", type=Path, default=Path(__file__).resolve().parents[1] / "src/model/thread_stacks.rs")
    parser.add_argument("--objdump", default=stack_depth.OBJDUMP)
    args = parser.parse_args()
    stack_depth.OBJDUMP = args.objdump
    menu, = (row for row in stack_depth.table_rows(args.table.read_text()) if row[1] == "tr-menu")
    if menu[2] > 90_112 or menu[3] > 57_152:
        sys.exit("tr-menu's allocation or bound was raised")
    lines, failed = stack_depth.check_table(stack_depth.analyse(args.elf), [menu])
    print(lines[-1])
    return int(bool(failed) or "no shim" in lines[-1])


if __name__ == "__main__":
    sys.exit(main())
