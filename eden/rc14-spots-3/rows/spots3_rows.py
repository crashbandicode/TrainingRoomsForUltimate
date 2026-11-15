"""rc.14 spots-3 (spots by knockback): Eden row hooks on top of the spots lanes' `spots_rows.py`, with the knockback oracle of `spots3_oracle.py`
patched in (the old damage oracle kept beside it in every receipt: the primary's ruling 4, ~02:40Z). New rows: `kr_gutcheck` (King K. Rool's
Gut Check: the gust labelled Sour), `sp_rage` (a raged hit for the knockback window), `sp_late` (a Late label).
"""
import copy
import json
import re
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


def _reload(ctx, p1, cpu, labels, mash=None, frame_data=False, items=None):
    """spots_rows' reload (the same rows) with the CPU's Mash and Mash Triggers (`mash`: (action, trigger), or None for the factory's nothing)
    and Hitbox Visualization's six items (`items`; default the three boxes and Frame data colours as `frame_data`)."""
    data = copy.deepcopy(sr.mem._base(ctx))
    menu = data.setdefault('menu', {})
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


WIND = re.compile(r'\[SpotsWind\] slot=(\d+) no_reaction=(\S+) target_status=(\S+) hitstun=(\w+)')


def _swing_lines(ctx, walk, attack, wait):
    """Reset, walk, attack; the diag lines of the swing."""
    ctx.play('reset-center')
    time.sleep(2.0)
    ctx.play(sr._walk(walk))
    seq = hs.diag_mark(ctx)
    ctx.play(attack)
    time.sleep(wait)
    return hs.diag_since(ctx, seq)


def kr_gutcheck(ctx, setups=((MARTH, FTILT),), walks=(76, 88), rounds=4):
    """King K. Rool's Gut Check: P1 K. Rool walks in and does down special while the CPU mashes an attack (Always), so the CPU's hit triggers the
    counter, whose hits land on the CPU. Every label is checked by the record-slot oracle. The row passes when a counter hit on the CPU from its
    weaker hitbox (the gust: the game's slot whose class is Sour) is labelled Sour. Also kept: each hit's `[SpotsWind]` line (the recorded slot's
    no-reaction byte, ATTACK arg 26, and the target's status and hitstun), and one 1/4-speed clip with Hitbox Visualization on Windboxes only."""
    landed, bad, wind = [], [], []
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
                lines = _swing_lines(ctx, walk, 'sp-downb', 3.5)
                wind += [{'cpu': cpu, 'walk': walk, 'line': t[t.index('[SpotsWind]'):]} for _, _, t in lines if '[SpotsWind]' in t]
                for hit in _hits(lines):
                    why = _verdict(hit)
                    hit.update(cpu=cpu, action=action, walk=walk, round=rnd)
                    landed.append(hit)
                    if why:
                        bad.append((cpu, walk, why))
    gust = [h for h in landed if h['label'] == 'Sour' and h['mark'] and h['mark'][1] == 1 and h.get('game_slot_class') == 'Sour']
    # The operator's question (via the primary, ~03:25Z): is the gust a windbox? Windboxes only, 1/4 speed, through one counter.
    clip = None
    try:
        _reload(ctx, KROOL, MARTH, True, mash=(FTILT, ALWAYS), items=[0, 0, 0, 0, 0, 1])
        time.sleep(3.0)
        sr._speed_hold(ctx, ['left', 'left', 'left'])
        clip = sr._gif(ctx, 'kr-windboxes-only', 'sp-downb', 68 * 4, seconds=14)  # sp-wf272: 68 game frames of walk at 1/4 speed
        clip['shot'] = str(ctx.shot('kr-windboxes-only-after'))
    except Exception as exc:  # the clip is evidence for the question, not the row's verdict
        clip = {'error': repr(exc)}
    finally:
        sr._speed_hold(ctx, ['right', 'right', 'right'])
        # Leave Marth vs Mario, so the next row's start gate (the Training HUD) sees the fighters it was captured with.
        sr.sp_setup(ctx, MARTH, MARIO, frame_data=False, labels=True)
    out = {'hits': landed, 'bad': bad, 'gust_sour': len(gust), 'wind': wind, 'clip': clip,
           'labels': sorted({h['label'] or 'none' for h in landed if h['gained'] > 0})}
    steps.save(ctx, 'gutcheck', out)
    if bad:
        raise RuntimeError('labels the oracle disagrees with: %s' % bad[:6])
    if not gust:
        raise RuntimeError('no counter hit from the weaker slot labelled Sour (NOT REPRODUCED or wrong): labels %s, %d hits' % (out['labels'], len(landed)))
    return {'gust_sour': len(gust), 'labels': out['labels'], 'hits': len(landed), 'wind': [w['line'] for w in wind][:8]}


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
