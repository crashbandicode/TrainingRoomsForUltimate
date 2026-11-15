"""spots-4: which percent the game's knockback function takes. For every hit spots-3 recorded (a game slot and a received knockback), the spread of
received / formula on the game's own slot under each candidate percent; the tightest is the game's.

`python3 -I percent_variants.py <boot out folder>...` (replay_table.hits_of reads each boot's receipts). The candidates are what the hit handler
(13.0.5 main+0x402e94) could add up from the defender's damage record: `+0x0` (the percent before the hit, or already after it) and `+0x8` (the
hit's damage before the 1-on-1 ratio, `gained / 1.2`).
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent / 'rows'))
import replay_table  # noqa: E402
import spots3_oracle as o  # noqa: E402

VARIANTS = {
    'A before + gained/1.2 (+0x0 the percent before, + +0x8)': lambda h: h['before'] + h['gained'] / o.TARGET_MUL,
    'B before + gained (the percent after the hit)': lambda h: h['before'] + h['gained'],
    'C before + gained + gained/1.2 (+0x0 already after the hit)': lambda h: h['before'] + h['gained'] + h['gained'] / o.TARGET_MUL,
    'D gained/1.2 (+0x0 not added)': lambda h: h['gained'] / o.TARGET_MUL,
}


def spread(hits, percent):
    ratios = []
    for h in hits:
        slot = h['powers'][h['game_slot']]
        if slot['damage'] <= 0 or h['launched'] <= 0:
            continue
        k = o.launched_by(slot, h['gained'] / o.TARGET_MUL, percent(h), h['weight'] if h['weight'] > 0 else 100.0, h.get('attacker', 0.0))
        ratios.append(h['launched'] / k)
    return ratios


def main():
    hits = [h for folder in sys.argv[1:] for _, h in replay_table.hits_of(folder) if h.get('gained', 0) > 0 and h.get('game_slot') is not None]
    for name, percent in VARIANTS.items():
        ratios = spread(hits, percent)
        print('%-62s n=%d  %.4f..%.4f  width %.4f' % (name, len(ratios), min(ratios), max(ratios), max(ratios) - min(ratios)))


if __name__ == '__main__':
    main()
