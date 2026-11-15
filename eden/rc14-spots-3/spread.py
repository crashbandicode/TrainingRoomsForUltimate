"""spots-4: the spread of received / formula knockback on the game's own slot, under the game's percent (before + gained / 1.2), over every hit
spots-3 recorded, and the knockback window it supports (the spread plus a margin of 0.03 each side, to two decimals).

`python3 -I spread.py <boot out folder>...` reads each boot's receipts (replay_table.hits_of) and adds the three hits of the fully staled jab that
`runs/spots3-bade7ea6-1/rage-stand-lines.txt` holds (the log had rotated; the lines are transcribed below, the file is the source). Prints the
spread per source and overall, and the window.
"""
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent / 'rows'))
import replay_table  # noqa: E402
import spots3_oracle as o  # noqa: E402

MARGIN = 0.03

# rage-stand-lines.txt: the CPU Mario's jab on P1 Marth (weight 90), mashed every frame it could: gained 1.239 of 2.2 (0.563, the staling floor).
JAB = [{'damage': 2.2, 'growth': 30, 'fixed': 0, 'base': 20}, {'damage': 2.2, 'growth': 30, 'fixed': 0, 'base': 20},
       {'damage': 2.2, 'growth': 20, 'fixed': 0, 'base': 15}, {'damage': 2.2, 'growth': 20, 'fixed': 0, 'base': 15}]
STAND = [  # frame, slot, before, gained, launched
    (1050, 1, 29.362, 1.239, 27.987),
    (1092, 2, 30.601, 1.239, 20.395),
    (1134, 2, 31.841, 1.239, 20.465),
]


def stand_ratios():
    return [replay_table.own_ratio({'powers': JAB, 'game_slot': slot, 'launched': launched, 'gained': gained, 'before': before, 'weight': 90.0,
                                    'attacker': 0.0}) for _, slot, before, gained, launched in STAND]


def window(low, high, margin=MARGIN):
    """The window: the spread widened by `margin`, rounded to two decimals (the receipts carry three decimals, so the ratios are good to about 1e-3)."""
    return round(low - margin, 2), round(high + margin, 2)


def main():
    sources = {}
    for folder in sys.argv[1:]:
        ratios = [r for r in (replay_table.own_ratio(h) for _, h in replay_table.hits_of(folder) if h.get('gained', 0) > 0 and h.get('game_slot') is not None) if r]
        sources[Path(folder).name] = ratios
    sources['rage-stand-lines.txt (staled jab)'] = stand_ratios()
    everything = []
    for name, ratios in sources.items():
        print('%-36s %3d hits  %.4f..%.4f' % (name, len(ratios), min(ratios), max(ratios)))
        everything += ratios
    low, high = min(everything), max(everything)
    print('%-36s %3d hits  %.4f..%.4f' % ('all', len(everything), low, high))
    print('window (spread, margin %.2f each side, 2 decimals): %.2f..%.2f; the oracle\'s shipped window %.2f..%.2f leaves %.4f below the spread and %.4f above' % (
        MARGIN, *window(low, high), o.KB_LOW, o.KB_HIGH, low - o.KB_LOW, o.KB_HIGH - high))


if __name__ == '__main__':
    main()
