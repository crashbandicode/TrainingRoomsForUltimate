"""spots-3: replay a boot's hits under the record-slot attribution (the primary's approval ~03:20Z), from the rows' receipts.

`python3 -I replay_table.py <run out folder>` reads every receipt the spot rows wrote (gutcheck.json, rage.json, charge.json, sweep-*.json,
attempts.json, stills.json) and, for each hit that added damage and has the defender's record line (the game's slot), prints: the slot,
its class against the frame's hitboxes (rows/spots3_oracle.py), the label the build under test showed, the label the record-slot rule
gives (the slot's class when its damage ratio lies in 0.5..1.6 and its received/formula knockback in the window) and whether that equals
the slot's class. The knockback ratio spread of the game's own slot ends the output (the window: the spread plus a margin of 0.03).
"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / 'rows'))
import spots3_oracle as o  # noqa: E402

K_LOW, K_HIGH = 0.5, 1.6


def hits_of(folder):
    seen = set()
    for path in sorted(Path(folder).rglob('*.json')):
        if path.name in ('summary.json', 'plan.json') or path.parent.name.startswith('post-run'):
            continue
        try:
            data = json.loads(path.read_text())
        except (ValueError, UnicodeDecodeError):
            continue
        if not isinstance(data, dict):
            continue
        hits = data.get('hits') or [h for s in data.get('swings', []) + data.get('attempts', []) + data.get('stills', []) for h in s.get('hits', [])]
        for h in hits:
            if not isinstance(h, dict) or 'powers' not in h:
                continue
            key = (path.parent.name, h.get('frame'), h.get('gained'), h.get('launched'), h.get('walk'), h.get('round'), h.get('cpu'), h.get('hold'))
            if key not in seen:
                seen.add(key)
                yield path.parent.name, h


def own_ratio(h):
    s = h['powers'][h['game_slot']]
    if s['damage'] <= 0 or h['launched'] <= 0:
        return None
    k = o.knockback(s, h['gained'] / o.TARGET_MUL, h['before'] + h['gained'], h['weight'] if h['weight'] > 0 else 100.0)
    return h['launched'] / k if k > 0 else None


def replay(h, window):
    slot = h['game_slot']
    s = h['powers'][slot]
    cls = o.classes_of(h['powers'], h['mul'] or 1.0, h['before'], h['weight']).get(slot)
    r = own_ratio(h)
    ok = s['damage'] > 0 and (h['mul'] or 0) > 0 and K_LOW <= h['gained'] / (s['damage'] * h['mul']) <= K_HIGH and r is not None and window[0] <= r <= window[1]
    return cls, (cls if ok else None), r


def main():
    rows = [(row, h) for row, h in hits_of(sys.argv[1]) if h.get('gained', 0) > 0 and h.get('game_slot') is not None]
    ratios = [r for r in (own_ratio(h) for _, h in rows) if r]
    window = (round(min(ratios) - 0.03, 2), round(max(ratios) + 0.03, 2)) if ratios else (0.8, 1.2)
    print('| row | frame | gained | received | game slot | slot class | shown (old rule) | record-slot rule | equal | ratio |')
    print('| --- | ---: | ---: | ---: | ---: | --- | --- | --- | --- | ---: |')
    agree = late = 0
    for row, h in rows:
        cls, new, r = replay(h, window)
        shown = h.get('label')
        # Late comes from the action's history, which a single hit's receipt does not hold: a Late label the old rule showed stays Late when
        # the slot's frame class is not Sweet or Sour.
        if shown == 'Late' and cls is None:
            new, late = 'Late', late + 1
        equal = new == (cls if shown != 'Late' else 'Late')
        agree += equal
        print('| %s | %s | %.3f | %.3f | %d | %s | %s | %s | %s | %s |' % (row, h.get('frame'), h['gained'], h['launched'], h['game_slot'], cls, shown, new,
                                                                     'yes' if equal else 'NO', '%.4f' % r if r else '-'))
    print('\n%d hits with a record, %d equal to the slot class (%d Late kept from history); ratio spread %.4f..%.4f, window %s' % (
        len(rows), agree, late, min(ratios), max(ratios), window))


if __name__ == '__main__':
    main()
