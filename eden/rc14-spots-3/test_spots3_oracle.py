"""Host tests of the spots-3 Eden oracle (the record-slot rule), kb_table.py and replay_table.py, on synthetic diag lines in the build's
exact format. `python3 -I test_spots3_oracle.py`."""
import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE / 'rows'))
sys.path.insert(0, str(HERE))
import kb_table  # noqa: E402
import replay_table  # noqa: E402
import spots3_oracle as o  # noqa: E402

HIT, GUST = (16.38, 56, 0, 90), (11.466, 50, 0, 80)  # Gut Check's counter against Marth's forward tilt (spots3-bade7ea6-1)
JAB = ((3.0, 15, 0, 30), (3.0, 12, 0, 30), (5.0, 12, 0, 30), (5.0, 12, 0, 30))  # Marth's jab: the 5s are the tip


def powers(*hits):
    slots = list(hits) + [(0.0, 0, 0, 0)] * (8 - len(hits))
    return ', '.join('Hit { damage: %.3f, growth: %d, fixed: %d, base: %d }' % h for h in slots)


def stop(frame, gained, launched, before, label=None, hits=(HIT, GUST), attacker=0.0, recorded=None, weight=98.0):
    mark = 'Some(Mark { frame: %d, lane: 1, spot: %s })' % (frame, label) if label else 'None'
    rec = 'Some(%d)' % recorded if recorded is not None else 'None'
    return ('[Spots] stop frame=%d gained=%.3f launched=%.3f mul=1.0000 before=%.3f weight=%.2f attacker=%.3f recorded=%s powers=[%s] newest_mark=%s'
            % (frame, gained, launched, before, weight, attacker, rec, powers(*hits), mark))


def record(damage, slot, kb):
    return '[SpotsKB] who=cpu stop=true damage=%.3f thunk=%.3f rec10=%.3f rec50=0x00%02x0000 slot=%d attacker=0x10 rec160=0.000' % (damage, kb, kb, slot, slot)


def received(hit, gained, before, r=1.0, weight=98.0):
    h = {'damage': hit[0], 'growth': hit[1], 'fixed': hit[2], 'base': hit[3]}
    return r * o.knockback(h, gained / o.TARGET_MUL, before + gained, weight)


class Oracle(unittest.TestCase):
    def one(self, pre, line):
        hits = o.parse([(0, 0, t) for t in list(pre) + [line]])
        self.assertEqual(len(hits), 1)
        return hits[0]

    def hit(self, slot, label, gained, before=20.0, hits=(HIT, GUST), weight=98.0):
        kb = received(hits[slot], gained, before, weight=weight)
        return self.one([record(before + gained, slot, kb)], stop(5, gained, kb, before, label, hits, recorded=slot, weight=weight))

    def test_the_gust_is_sour_and_the_hit_sweet(self):
        self.assertIsNone(o.verdict(self.hit(1, 'Sour', 11.466 * 1.26)))
        self.assertIsNone(o.verdict(self.hit(0, 'Sweet', 16.38 * 1.26)))

    def test_a_wrong_or_missing_label_is_caught(self):
        self.assertIn('gives Sour', o.verdict(self.hit(1, 'Sweet', 11.466 * 1.26)))
        self.assertIn('gives Sour', o.verdict(self.hit(1, None, 11.466 * 1.26)))

    def test_the_stale_tip_is_sweet(self):
        # Eden: Marth's jab tip on K. Rool at 12.7%, gained 4.082 (a 5 x 0.816 stale hit): the game's slot 2, the tip, Sweet.
        hit = self.hit(2, 'Sweet', 4.082, before=12.718, hits=JAB, weight=133.0)
        self.assertIsNone(o.verdict(hit))
        self.assertEqual((hit['game_slot'], hit['expected']), (2, 'Sweet'))

    def test_the_plugins_slot_must_be_the_record_lines(self):
        kb = received(GUST, 14.4, 20.0)
        hit = self.one([record(34.4, 1, kb)], stop(5, 14.4, kb, 20.0, 'Sour', recorded=0))
        self.assertIn('read slot 0', o.verdict(hit))

    def test_late_is_accepted_and_the_damage_oracle_kept(self):
        hit = self.hit(1, 'Late', 11.466 * 1.26)
        self.assertIsNone(o.verdict(hit, damage_classes={'Sour'}))
        self.assertEqual(hit['classes_damage_oracle'], ['Sour'])


class Tables(unittest.TestCase):
    def test_the_window_comes_from_the_game_slot_ratios(self):
        lines = []
        for frame, (which, slot, r) in enumerate(((GUST, 1, 1.0), (HIT, 0, 1.03), (GUST, 1, 0.95)), start=1):
            g = which[0] * 1.26
            kb = received(which, g, 20.0, r)
            lines += [record(20.0 + g, slot, kb), stop(frame, g, kb, 20.0, None, attacker=60.0, recorded=slot)]
        rows = kb_table.table(lines)
        self.assertEqual([r['game_slot'] for r in rows], [1, 0, 1])
        self.assertEqual([round(r['own'], 2) for r in rows], [1.0, 1.03, 0.95])
        self.assertEqual(kb_table.window(rows), {'n': 3, 'min': 0.95, 'max': 1.03, 'window': (0.92, 1.06)})
        self.assertIsNone(kb_table.window([]))

    def test_the_replay_gives_the_slot_class(self):
        g = 4.082
        hit = {'powers': [{'damage': d, 'growth': gr, 'fixed': f, 'base': b} for d, gr, f, b in JAB], 'gained': g, 'before': 12.718, 'weight': 133.0,
               'mul': 1.0, 'game_slot': 2, 'launched': received(JAB[2], g, 12.718, weight=133.0)}
        cls, new, r = replay_table.replay(hit, (0.9, 1.05))
        self.assertEqual((cls, new, round(r, 3)), ('Sweet', 'Sweet', 1.0))
        self.assertEqual(replay_table.replay(hit, (1.1, 1.2))[1], None, 'outside the window: no label')


if __name__ == '__main__':
    unittest.main()
