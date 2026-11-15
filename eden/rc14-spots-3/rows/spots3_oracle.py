"""spots-3's oracle, pure (no Eden): the diag lines parsed and the spot rules restated from their public sources, apart from the plugin's
code: SmashWiki's knockback formula, the ranking by knockback then damage, the damage window (1.0..1.6) and the received-knockback window.

  [Spots] stop frame=<n> gained=<d> launched=<kb> mul=<charge> before=<percent> weight=<w> attacker=<percent>
          powers=[Hit { damage: .., growth: .., fixed: .., base: .. }, ..] newest_mark=<None | Some(Mark {..})>
  [SpotsKB] who=<p1|cpu> stop=<bool> damage=<percent> thunk=<kb> rec10=<kb> rec50=<word> slot=<attack slot> ...
"""
import re

K_LOW, K_HIGH, KB_LOW, KB_HIGH, DEALT, TARGET_MUL, KB_EPS, EPS = 0.5, 1.6, 0.90, 1.05, 1.26, 1.2, 0.5, 0.05
STOP = re.compile(r'\[Spots\] stop frame=(\d+) gained=(\S+) launched=(\S+) mul=(\S+) before=(\S+) weight=(\S+) attacker=(\S+)(?: recorded=(None|Some\(\d+\)))? '
                  r'powers=\[(.*)\] newest_mark=(?:None|Some\(Mark \{ frame: (\d+), lane: (\d), spot: (\w+) \}\))')
HIT = re.compile(r'Hit \{ damage: ([^,]+), growth: (-?\d+), fixed: (-?\d+), base: (-?\d+) \}')
KB = re.compile(r'\[SpotsKB\] who=(\w+) stop=(\w+) damage=(\S+) thunk=(\S+) rec10=(\S+) rec50=(\S+) slot=(\d+)')


def _f(text):
    try:
        return float(text)
    except ValueError:
        return 0.0


def knockback(hit, d, p, w):
    """SmashWiki, Knockback: ((p/10 + p*d/20) * 200/(w+100) * 1.4 + 18) * growth/100 + base; a fixed knockback stands for d at p = 10."""
    if hit['fixed']:
        p, d = 10.0, float(hit['fixed'])
    return ((p / 10 + p * d / 20) * 200 / (w + 100) * 1.4 + 18) * hit['growth'] / 100 + hit['base']


def power(hit, mul, before, weight):
    d = hit['damage'] * mul
    return (knockback(hit, d, before + d * DEALT, weight if weight > 0 else 100.0), hit['damage'])


def below(a, b):
    return a[0] < b[0] if abs(a[0] - b[0]) > KB_EPS else a[1] < b[1] - EPS


def classes_of(powers, mul, before, weight):
    """Each live slot's class on this frame without the action's history (Late is not decided here)."""
    live = {i: power(h, mul, before, weight) for i, h in enumerate(powers) if h['damage'] > 0}
    out = {}
    for i, p in live.items():
        top = all(not below(p, q) for q in live.values())
        out[i] = 'Sour' if not top else ('Sweet' if any(below(q, p) for q in live.values()) else None)
    return out


def candidates(hit):
    """The slots that could have dealt the hit: the damage window, and the received-knockback window when it was read. Sets hit['kb_ratio']."""
    out, hit['kb_ratio'] = [], {}
    for i, h in enumerate(hit['powers']):
        if h['damage'] <= 0 or hit['mul'] <= 0 or not K_LOW <= hit['gained'] / (h['damage'] * hit['mul']) <= K_HIGH:
            continue
        k = knockback(h, hit['gained'] / TARGET_MUL, hit['before'] + hit['gained'], hit['weight'] if hit['weight'] > 0 else 100.0)
        hit['kb_ratio'][i] = round(hit['launched'] / k, 4) if k > 0 else None
        if hit['launched'] > 0 and hit['weight'] > 0 and not (k > 0 and KB_LOW <= hit['launched'] / k <= KB_HIGH):
            continue
        out.append(i)
    return out


def parse(lines):
    """The [Spots] hits of `lines` ((_, _, text) tuples), each with the defender's last [SpotsKB] record line before it whose percent is the
    hit's percent after (the probe runs before the strip's step)."""
    out, last_kb = [], []
    for _, _, text in lines:
        k = KB.search(text)
        if k:
            last_kb.append({'who': k.group(1), 'stop': k.group(2) == 'true', 'damage': _f(k.group(3)), 'thunk': _f(k.group(4)),
                            'rec10': _f(k.group(5)), 'slot': int(k.group(7)), 'text': text[text.index('[SpotsKB]'):]})
            continue
        m = STOP.search(text)
        if not m:
            continue
        powers = [{'damage': _f(a), 'growth': int(b), 'fixed': int(c), 'base': int(d)} for a, b, c, d in HIT.findall(m.group(9))]
        recorded = m.group(8)
        hit = {'frame': int(m.group(1)), 'gained': _f(m.group(2)), 'launched': _f(m.group(3)), 'mul': _f(m.group(4)),
               'before': _f(m.group(5)), 'weight': _f(m.group(6)), 'attacker': _f(m.group(7)), 'powers': powers,
               'recorded': int(recorded[5:-1]) if recorded and recorded.startswith('Some') else None,
               'mark': (int(m.group(10)), int(m.group(11)), m.group(12)) if m.group(12) else None}
        after = hit['before'] + hit['gained']
        hit['record'] = next((r for r in reversed(last_kb) if r['stop'] and abs(r['damage'] - after) < 0.01), None)
        out.append(hit)
    return out


def expected(hit, slot, classes):
    """The record-slot rule: the class of `slot` when its damage ratio lies in K_LOW..K_HIGH and its received/formula knockback in the window."""
    if slot is None or slot >= len(hit['powers']) or hit['powers'][slot]['damage'] <= 0 or hit['mul'] <= 0 or hit['gained'] <= 0:
        return None
    s = hit['powers'][slot]
    k = knockback(s, hit['gained'] / TARGET_MUL, hit['before'] + hit['gained'], hit['weight'] if hit['weight'] > 0 else 100.0)
    fits = K_LOW <= hit['gained'] / (s['damage'] * hit['mul']) <= K_HIGH and k > 0 and KB_LOW <= hit['launched'] / k <= KB_HIGH
    return classes.get(slot) if fits else None


def verdict(hit, damage_classes=None):
    """None when the label agrees with the record-slot rule on the game's own slot (the defender's `[SpotsKB]` record line), else why not. A
    label shows only for the hit's own frame; Late depends on the action's history and is accepted. The plugin's `recorded=` must name the
    record line's slot. `damage_classes`, the old damage oracle (spots / spots-2), is kept beside it (`classes_damage_oracle`)."""
    label = hit['mark'][2] if hit['mark'] and hit['mark'][0] == hit['frame'] else None
    hit['label'] = label
    classes = classes_of(hit['powers'], hit['mul'], hit['before'], hit['weight'])
    cands = candidates(hit) if hit['gained'] > 0 else []
    hit['candidates'] = cands
    hit['candidate_classes'] = {classes[i] for i in cands}
    hit['classes'] = sorted(c or 'none' for c in hit['candidate_classes'])
    if damage_classes is not None:
        hit['classes_damage_oracle'] = sorted(c or 'none' for c in damage_classes)
    record = hit.get('record')
    game_slot = record['slot'] if record else None
    hit['game_slot'] = game_slot
    hit['game_slot_class'] = classes.get(game_slot) if game_slot is not None else None
    hit['expected'] = expected(hit, game_slot, classes)
    hit['oracles_differ'] = damage_classes is not None and sorted(c or 'none' for c in {hit['expected']}) != hit.get('classes_damage_oracle')
    if hit['gained'] <= 0:
        return None if label is None else 'a label with no damage gained'
    if hit.get('recorded') is not None and game_slot is not None and hit['recorded'] != game_slot:
        return 'the plugin read slot %s, the record line says %s' % (hit['recorded'], game_slot)
    if label == 'Late':
        return None
    if label != hit['expected']:
        return 'label %s but the game slot %s gives %s' % (label, game_slot, hit['expected'])
    return None


def equal_damage_pair(hit):
    """Two live hitboxes of the same damage and different launch numbers (Gut Check's hit and gust)."""
    live = [h for h in hit['powers'] if h['damage'] > 0]
    return any(abs(a['damage'] - b['damage']) < EPS and (a['growth'], a['fixed'], a['base']) != (b['growth'], b['fixed'], b['base'])
               for a in live for b in live)
