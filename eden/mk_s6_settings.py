"""Boot settings for the package-6 set A/B boot (third successor): Player Swapping on, the P1 and CPU cycle orders the rows step
through, preload off at boot (the preload row turns it on through the diag reload, so the menu-commit path is exercised).
Usage: mk_s6_settings.py <out.json>"""
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
TRIO = HERE.parent / "src/model/swap/trio.rs"
CATALOG = [int(k, 16) for k in re.findall(r"^\s*\(0x([0-9a-f]+), \"", TRIO.read_text(), re.M)]


def roster(*kinds):
    ranks = [0] * len(CATALOG)
    for rank, kind in enumerate(kinds, 1):
        ranks[CATALOG.index(kind)] = rank
    return [0] + ranks


MENU = {
    "trio_practice": [1, 0],
    # P1: Kirby, Ice Climbers, Pokemon Trainer (Squirtle), Palutena; CPU: Pikachu, Bowser.
    "player_roster": roster(0x06, 0x4b, 0x24, 0x36),
    "cpu_roster": roster(0x08, 0x0f),
}

if __name__ == "__main__":
    Path(sys.argv[1]).write_text(json.dumps({"menu": MENU}), encoding="utf-8")
    print(len(CATALOG), "catalog entries")
