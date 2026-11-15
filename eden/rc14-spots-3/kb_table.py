"""spots-3: the received/slot knockback table from a boot's diag log, for the KB_LOW..KB_HIGH window (the primary's ruling 1, ~02:40Z).

`python3 -I kb_table.py <practice-diagnostics.log>...` reads every `[Spots] stop` line (with the `[SpotsKB]` record line before it), and for
each hit that added damage prints: the attacker's percent, the defender's percent before and its weight, the damage gained, the knockback
received (DamageModule::reaction), the game's attack slot (record +0x52) and, for every slot in the damage window, received / that slot's
knockback by the formula (rows/spots3_rows.py). The last lines give the spread of the ratio for the game's own slot, and the window it implies
with a margin of 0.03 on each side.
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / 'rows'))
import spots3_oracle as o  # noqa: E402


def table(lines):
    rows = []
    for hit in o.parse([(0, 0, line) for line in lines]):
        if hit['gained'] <= 0:
            continue
        o.candidates(hit)
        slot = hit['record']['slot'] if hit.get('record') else None
        rows.append({'frame': hit['frame'], 'attacker': hit['attacker'], 'before': hit['before'], 'weight': hit['weight'], 'gained': hit['gained'],
                     'launched': hit['launched'], 'game_slot': slot, 'ratios': hit.get('kb_ratio', {}),
                     'own': hit.get('kb_ratio', {}).get(slot) if slot is not None else None})
    return rows


def window(rows, margin=0.03):
    own = [r['own'] for r in rows if r['own']]
    if not own:
        return None
    return {'n': len(own), 'min': min(own), 'max': max(own), 'window': (round(min(own) - margin, 3), round(max(own) + margin, 3))}


def main():
    lines = []
    for path in sys.argv[1:]:
        lines += Path(path).read_text(errors='replace').splitlines()
    rows = table(lines)
    print('| frame | attacker % | before % | weight | gained | received | game slot | received / slot knockback |')
    print('| ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |')
    for r in rows:
        ratios = ', '.join('%d: %.3f' % (k, v) for k, v in sorted(r['ratios'].items()) if v is not None)
        print('| %d | %.1f | %.1f | %.1f | %.3f | %.3f | %s | %s |' % (r['frame'], r['attacker'], r['before'], r['weight'], r['gained'], r['launched'],
                                                                   r['game_slot'], ratios))
    print('own-slot ratio spread:', window(rows))


if __name__ == '__main__':
    main()
