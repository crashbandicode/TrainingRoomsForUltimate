"""rc.14 spots-3 (spots by knockback): Eden row hooks on top of the spots lanes' `spots_rows.py`, with the knockback oracle of `spots3_oracle.py`
patched in (the old damage oracle kept beside it in every receipt: the primary's ruling 4, ~02:40Z). New rows: `kr_gutcheck` (King K. Rool's
Gut Check: the gust labelled Sour), `sp_rage` (a raged hit for the knockback window), `sp_late` (a Late label).
"""
import copy
import json
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import hotslots as hs  # noqa: E402
import spots3_oracle as o  # noqa: E402
import spots_rows as sr  # noqa: E402
import steps  # noqa: E402

KROOL, MARIO, MARTH, FALCON = 66, 0, 21, 11
sr.KINDS[KROOL] = 0x45  # trio.rs: (0x45, "King K. Rool") at CHOICES index 66
JAB, FTILT, ALWAYS = 18, 19, 16  # options.rs ACTIONS / MASH_TRIGGERS

# The damage oracle of spots / spots-2, kept on the shared module once: the runner may load this file again while spots_rows stays imported.
if not hasattr(sr, '_classes_damage'):
    sr._classes_damage = sr._classes
OLD_CLASSES = sr._classes_damage
_LAST = {'classes': set()}


def _hits(lines):
    return o.parse(lines)


def _verdict(hit):
    why = o.verdict(hit, OLD_CLASSES(hit['gained'], [h['damage'] for h in hit['powers']], mul=hit['mul'] or 1.0))
    _LAST['classes'] = {hit['expected']} - {None}
    return why


sr.STOP, sr._hits, sr._verdict = o.STOP, _hits, _verdict
# spots-2's charge row asks `_classes(...)` right after `_verdict(hit)` for the same hit: answer with that verdict's knockback classes.
sr._classes = lambda gained, powers, earlier=0.0, mul=1.0: _LAST['classes']


def _reload(ctx, p1, cpu, labels, mash=None, frame_data=False, items=None, extra=None):
    """spots_rows' reload (the same rows) with the CPU's Mash and Mash Triggers (`mash`: (action, trigger), or None for the factory's nothing)
    and Hitbox Visualization's six items (`items`; default the three boxes and Frame data colours as `frame_data`); `extra`: more menu rows."""
    data = copy.deepcopy(sr.mem._base(ctx))
    menu = data.setdefault('menu', {})
    menu.update(extra or {})
    on, off = [1, 0], [0, 1]
    menu['hitbox_vis'] = list(items) if items else [1, 1, 1, int(frame_data), 0, 0]
    menu['spots_strip'] = on if labels else off
    menu['spots_combo'] = off
    menu['trio_practice'] = on
    menu['input_display_strip'] = [0, 1]
    menu['combo_helper'] = on
    menu['mash_state'] = [1 if mash and i == mash[0] else 0 for i in range(30)]
    menu['mash_triggers'] = [1 if i == mash[1] else 0 for i in range(17)] if mash else [1 if i in (0, 1, 3, 7) else 0 for i in range(17)]
    for key, who in (('player_character', p1), ('cpu_character', cpu)):
        menu[key] = sr._onehot(87, who)
    sr._LAST.update(p1=p1, cpu=cpu)
    root = sr.mem._tr(ctx)
    tmp = root / 'diag-settings-next.tmp'
    tmp.write_text(json.dumps(data, indent=2) + '\n')
    seq = hs.diag_mark(ctx)
    tmp.replace(root / 'diag-settings-next.json')
    if not hs.wait_diag(ctx, seq, r'diag_settings_reload', 15):
        raise RuntimeError('no diag_settings_reload line within 15 s')


def sp_setup(ctx, p1, cpu, frame_data=False, labels=True, vis=True, combo=False):
    return sr.sp_setup(ctx, p1, cpu, frame_data=frame_data, labels=labels, vis=vis, combo=combo)


def sp_sweep(ctx, attack, walks, min_hits=1):
    return sr.sp_sweep(ctx, attack, walks, min_hits=min_hits)


def sp_charge(ctx, holds=(0, 30, 66), walks=(8, 28), rounds=1):
    return sr.sp_charge(ctx, holds=holds, walks=walks, rounds=rounds)


def sp_late(ctx, walks=(24, 28, 32, 36, 44)):
    """Captain Falcon's forward air at the spacings where only its late hits land: a Late label, every label agreeing with the oracle."""
    sr.sp_setup(ctx, FALCON, MARIO, frame_data=False, labels=True)
    out = sr.sp_sweep(ctx, 'sp-fair', walks, min_hits=1)
    if 'Late' not in out['classes_labelled']:
        raise RuntimeError('no Late label at the spacings %s: %s' % (walks, out))
    return out




CAP_BEFORE = 995.0


def cap_states(base, percent=CAP_BEFORE):
    """The boot's save_states.json (`base`, parsed) with the CPU's slot 0 at `percent`: what sp_cap loads. Stage it as the boot's
    save_states.json; other rows are unchanged while Saves > CPU damage on load stays Default (0%)."""
    out = copy.deepcopy(base)
    out['cpu'][0]['percent'] = percent
    return out


def sp_cap(ctx, walks=(8, 28), tries=2):
    """Past the game's damage cap (999%; spots-3 (b), the primary ~04:45Z): Saves > CPU damage on load = Save State, Load (slot 0 holds
    the CPU at 995%, `cap_states`), then P1 Marth's forward smash (13 or 18, x1.26, more than the 4% gap). The percent's rise is cut at
    999, so the plugin takes the hit's damage from the defender's record (+0x8, `rec08` x 1.2). PASS: a labelled hit past the cap, its
    gained equal to rec08 x 1.2, and no label the oracle disagrees with. Precondition: no save state was loaded earlier in the boot (the
    plugin reads save_states.json once) and the staged file is `cap_states` of the boot's."""
    landed, bad = [], []
    try:
        _reload(ctx, MARTH, MARIO, True, extra={'save_damage_cpu': [0, 1, 0]})
        time.sleep(4.0)
        for _ in range(tries):
            for walk in walks:
                ctx.play('p23-state-load')
                time.sleep(2.0)
                seq = hs.diag_mark(ctx)
                if walk:
                    ctx.play(sr._walk(walk))
                ctx.play('sp-csmash')
                time.sleep(3.5)
                for hit in _hits(hs.diag_since(ctx, seq)):
                    why = _verdict(hit)
                    hit.update(walk=walk)
                    landed.append(hit)
                    if why:
                        bad.append((walk, why))
    finally:
        _reload(ctx, MARTH, MARIO, True)
    capped = [h for h in landed if h.get('capped') and h['label']]
    out = {'hits': landed, 'bad': bad, 'capped_labelled': len(capped), 'shot': str(ctx.shot('sp-cap'))}
    steps.save(ctx, 'cap', out)
    if bad:
        raise RuntimeError('labels the oracle disagrees with: %s' % bad[:6])
    if not capped:
        raise RuntimeError('no labelled hit past the damage cap: %d hits, befores %s' % (len(landed), sorted({round(h['before']) for h in landed})))
    return {'capped_labelled': len(capped), 'hits': len(landed), 'labels': sorted({h['label'] for h in capped})}


def _swing_lines(ctx, walk, attack, wait):
    """Reset, walk, attack; the diag lines of the swing."""
    ctx.play('reset-center')
    time.sleep(2.0)
    ctx.play(sr._walk(walk))
    seq = hs.diag_mark(ctx)
    ctx.play(attack)
    time.sleep(wait)
    return hs.diag_since(ctx, seq)


def kr_gutcheck(ctx, setups=((MARTH, FTILT), (MARTH, JAB)), walks=(64, 76, 88), rounds=2):
    """King K. Rool's Gut Check: P1 K. Rool walks in and does down special while the CPU mashes an attack (Always), so the CPU's hit triggers the
    counter, whose hits land on the CPU. Every label is checked by the record-slot oracle; the row passes when a counter hit on the CPU is
    labelled and no label disagrees. The gust (the counter's weaker hitbox) connected once, in spots3-bade7ea6-1 (Sour, the game's slot 1); it
    did not recur in 16 tries of spots3-bedc9a69-1, so the row reports it when it comes and does not require it (the primary, ~04:15Z). Gut
    Check has no windbox (the wind-audit lane, from the game data)."""
    landed, bad = [], []
    try:
        for cpu, action in setups:
            seq = hs.diag_mark(ctx)
            _reload(ctx, KROOL, cpu, True, mash=(action, ALWAYS))
            time.sleep(28.0)
            census = sr._census(ctx, seq, sr.KINDS[KROOL], cap=30)
            steps.save(ctx, 'setup-%d-%d' % (cpu, action), {'cpu': cpu, 'action': action, 'census': census, 'shot': str(ctx.shot('kr-setup-%d-%d' % (cpu, action)))})
            if not census:
                bad.append((cpu, action, 'the census never named K. Rool'))
                continue
            for rnd in range(rounds):
                for walk in walks:
                    for hit in _hits(_swing_lines(ctx, walk, 'sp-downb', 3.5)):
                        why = _verdict(hit)
                        hit.update(cpu=cpu, action=action, walk=walk, round=rnd)
                        landed.append(hit)
                        if why:
                            bad.append((cpu, walk, why))
    finally:
        # Leave Marth vs Mario, so the next row's start gate (the Training HUD) sees the fighters it was captured with.
        sr.sp_setup(ctx, MARTH, MARIO, frame_data=False, labels=True)
    counters = [h for h in landed if h['label'] and h['mark'] and h['mark'][1] == 1]
    gust = [h for h in counters if h['label'] == 'Sour' and h.get('game_slot_class') == 'Sour']
    out = {'hits': landed, 'bad': bad, 'counters': len(counters), 'gust_sour': len(gust),
           'labels': sorted({h['label'] or 'none' for h in landed if h['gained'] > 0})}
    steps.save(ctx, 'gutcheck', out)
    if bad:
        raise RuntimeError('labels the oracle disagrees with: %s' % bad[:6])
    if not counters:
        raise RuntimeError('no labelled counter hit on the CPU: labels %s, %d hits' % (out['labels'], len(landed)))
    return {'counters': len(counters), 'gust_sour': len(gust), 'labels': out['labels'], 'hits': len(landed)}


def sp_stale_tip(ctx, stand_s=16.0):
    """The first boot's defect: the CPU Marth mashes Jab (Always) on P1 Mario standing in its reach, so its jab stales; every hit on P1 is checked by
    the record-slot oracle, and the row passes when a stale tip hit (the game's slot of the 5-damage tip, damage ratio under 1.0) is labelled Sweet."""
    sr.sp_setup(ctx, MARIO, MARTH, frame_data=False, labels=True)
    _reload(ctx, MARIO, MARTH, True, mash=(JAB, ALWAYS))
    time.sleep(2.0)
    ctx.play('reset-center')
    time.sleep(2.0)
    seq = hs.diag_mark(ctx)
    ctx.play(sr._walk(52))
    time.sleep(stand_s)
    landed, bad = [], []
    for hit in _hits(hs.diag_since(ctx, seq)):
        why = _verdict(hit)
        landed.append(hit)
        if why:
            bad.append(why)
    _reload(ctx, MARIO, MARTH, True, mash=None)
    stale_tips = [h for h in landed if h['label'] == 'Sweet' and h.get('game_slot') is not None and h['powers'][h['game_slot']]['damage'] >= 5.0
                  and h['gained'] / h['powers'][h['game_slot']]['damage'] < 1.0]
    out = {'hits': landed, 'bad': bad, 'stale_tips_sweet': len(stale_tips), 'shot': str(ctx.shot('sp-stale-tip'))}
    steps.save(ctx, 'stale-tip', out)
    if bad:
        raise RuntimeError('labels the oracle disagrees with: %s' % bad[:6])
    if not stale_tips:
        raise RuntimeError('no stale tip hit labelled Sweet: %s' % [(h['gained'], h.get('game_slot'), h['label']) for h in landed][:12])
    return {'stale_tips_sweet': len(stale_tips), 'hits': len(landed)}


def sp_rage(ctx, stand_s=30.0, tries=4):
    """A rage case for the knockback window (the primary's ruling 1): the CPU Mario jabs (Mash Jab, Always) while P1 Marth stands in its reach,
    so P1's percent climbs; then the CPU stops mashing and Marth forward-smashes from where it stands. Every hit keeps the attacker's percent
    and the received/slot ratios; the row passes when a hit landed with the attacker above 35% (rage) and every label agrees."""
    sr.sp_setup(ctx, MARTH, MARIO, frame_data=False, labels=True)
    _reload(ctx, MARTH, MARIO, True, mash=(JAB, ALWAYS))
    time.sleep(2.0)
    ctx.play('reset-center')
    time.sleep(2.0)
    ctx.play(sr._walk(52))
    time.sleep(stand_s)
    _reload(ctx, MARTH, MARIO, True, mash=None)
    time.sleep(2.0)
    landed, bad = [], []
    for k in range(tries):
        seq = hs.diag_mark(ctx)
        ctx.play('sp-csmash')
        time.sleep(3.0)
        for hit in _hits(hs.diag_since(ctx, seq)):
            why = _verdict(hit)
            landed.append(hit)
            if why:
                bad.append((k, why))
        if any(h['gained'] > 0 and h['attacker'] > 35.0 for h in landed):
            break
        ctx.play(sr._walk(8))
        time.sleep(0.5)
    out = {'hits': landed, 'bad': bad, 'shot': str(ctx.shot('sp-rage'))}
    steps.save(ctx, 'rage', out)
    if bad:
        raise RuntimeError('labels the oracle disagrees with: %s' % bad)
    raged = [h for h in landed if h['gained'] > 0 and h['attacker'] > 35.0]
    if not raged:
        raise RuntimeError('no hit with the attacker above 35%%: %s' % [(h['attacker'], h['gained']) for h in landed])
    return {'raged_hits': [(round(h['attacker'], 1), h.get('kb_ratio'), h.get('game_slot'), h['label']) for h in raged]}


def sp_off(ctx):
    return sr.sp_off(ctx)


def sp_recover(ctx):
    """After a failed row: close anything open, then Marth vs Mario with the factory CPU, the fighters the start gate (the Training HUD)
    was captured with (a K. Rool row left P1 K. Rool, and the gate refused it in both boots)."""
    out = sr.sp_recover(ctx)
    sr.sp_setup(ctx, MARTH, MARIO, frame_data=False, labels=True)
    return out
