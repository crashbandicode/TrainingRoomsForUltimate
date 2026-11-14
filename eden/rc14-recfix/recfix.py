"""rc.14 recfix Eden rows (recfix lane, 2026-11-14): `t-recfix` (setup, run, restore: the Start save switch and the P1 CPU
Control refusal of `rc14/recfix`) and `t-recfix-slot` (slot_setup, slot_run, slot_restore: Playback Slot 1 on Mash).
NOT RUN: written for integration's final pass; the lane booted no Eden. Start: Training, P1 Mario and CPU Mario (g-training).
Steps follow gna_p2_rec / gna_p2_l4b (their helpers, same package folder).

  R1 Start save On (the default): Record a walk and a jab from the reset spot (Record writes the start save), walk P1
     away, Play (ZR + D-pad Left): exactly one `screen=playback role=player warp=1`.
  R2 The macro's own screen, Start save, Left: Off. The macro file now says `start_save_on: false` (it is omitted while
     On, so absent means On), still there after the menu closed; Play from the same away spot: the macro plays
     (P1 active frames) and there is NO `warp=1`.
  R3 Right: On again. The file is On (key absent), Play from the away spot warps once.
  R4 CPU > Player 1 control > P1 CPU Control On (the card has no pilot plugin): the AI drives P1 (two `state pos` dumps
     differ). Record (ZR + D-pad Down) is refused: `notice_draw ... P1 is controlled by an AI: recording is off`, no
     `screen=recording role=player recording=1`, the macro count is unchanged. Play on Player 1 (ZR + Left) is refused:
     `... Play on Player 1 is off`, no `screen=playback role=player`.
  R5 P1 CPU Control Off again: Record works (recording=1 then 0, one more macro), so R4 was the refusal and not a
     broken chord.

  Row t-recfix-slot, Defect 2 (the Playback Slot on Mash did not play on the Ally, rc.13 `2a667381`; rc.12 and rc.13
  `7d7b74c9` passed it on Linux):
  S1 The same fighters on both sides are asserted first (state pos `kind=`): the Ally's third run, whose own capture shows a
     Meta Knight CPU, said Mario. S2 `rec.hitstun_setup`: a jump and a neutral air recorded on P1, Playback slot Right on its
     screen, the menu closed, Mash = Playback Slot 1 by live reload AFTER the menu (the docs lane's order). S3 P1's forward
     smash: `playback_mash slot=0` within 5 s, with the diag's `cpu_event command_query ... queued=` lines kept, so a miss
     says where it stopped: `queued=false latched=false` means Mash never reached the queue, `queued=true` and no
     `playback_mash` means the glue after the queue (recorder.rs `mash_start`).
"""
import re
import time

import cov
import fx
import hotslots as hs
import steps
import gna
import gna_p2_rec as rec
import fx_rc11
from gna import _begin, _close, _finish, _mark, _reset

PR = r'\[PracticeRecording\] '
POS = re.compile(r'state pos entry=(\d+) kind=(-?\d+) x=(-?[\d.]+) y=(-?[\d.]+) lr=(-?[\d.]+) dmg=(-?[\d.]+)')
OFF_ON = {'off': [0, 1], 'on': [1, 0]}             # a toggle's list is [On, Off] (fx.py: [1, 0] is On)


def _macro_file(ctx, macro_id):
    return cov.read_json(cov.card(ctx) / 'macros' / (macro_id + '.json')) or {}


def _switch_on(ctx, macro_id):
    """`start_save_on` is omitted from the file while On."""
    return _macro_file(ctx, macro_id).get('start_save_on', True)


def _p1_x(ctx):
    """P1's x from one diag-pos.req dump (states.rs `state pos`)."""
    tail = cov.diag(ctx)
    (cov.card(ctx) / 'diag-pos.req').write_text('1\n')
    xs = []
    deadline = time.time() + 6
    while time.time() < deadline and not xs:
        xs = [float(m.group(3)) for m in POS.finditer(tail.text()) if int(m.group(1)) == 0]
        time.sleep(0.2)
    if not xs:
        raise RuntimeError('harness: no state pos dump for Player 1')
    return xs[-1]


def _away(ctx):
    _reset(ctx, 1.2)
    fx_rc11._walk(ctx, -0.6, 1.4)
    time.sleep(0.6)


def _set_switch(ctx, out, tag, button, macro_id):
    """The macro's screen (the just-recorded macro is the list's first), Start save, one Left or Right, the menu closed."""
    offset, title = steps.open_library_item(ctx, True)
    out[tag + '_file_title'] = title
    steps.down_to_file_row(ctx, offset, title, 'Start save')
    time.sleep(0.5)
    steps.press(ctx, button, 1.0)
    out[tag + '_shot'] = str(ctx.shot(tag))
    _close(ctx)
    time.sleep(0.8)


def setup(ctx):
    out = {'live': gna._base(ctx, recording_crop=OFF_ON['on'], p1_cpu_control=OFF_ON['off'])}
    _reset(ctx, 1.4)
    rec._ensure_notice_seen(ctx)

    def moves():
        fx_rc11._walk(ctx, 0.6, 0.45)
        rec._jab(ctx)

    macro_id, doc, out['macro'] = rec._record(ctx, moves, 0.4)
    out['macro_id'] = macro_id
    hs._remember(ctx, 'recfix_macro', macro_id)
    out['start_save'] = (doc or {}).get('start_save')
    if not out['start_save']:
        raise RuntimeError('harness: the recorded macro has no start save: %s' % sorted((doc or {}).keys()))
    time.sleep(1.2)
    return out


def run(ctx):
    t0, out = _begin(ctx)
    problems = []
    macro_id = hs._state(ctx).get('recfix_macro')
    # R1: On, the default.
    _mark(t0, out, 'R1')
    _away(ctx)
    rec._play(ctx, t0, out, 'r1_play', 4.0)
    if len(out['r1_play_warp']) != 1:
        problems.append('R1 Start save On: Play should warp once: %s' % out['r1_play_warp'])
    # R2: Left on the macro's screen: Off, in the file, and Play does not warp.
    _mark(t0, out, 'R2')
    _set_switch(ctx, out, 'r2', 'left', macro_id)
    out['r2_file_on'] = _switch_on(ctx, macro_id)
    if out['r2_file_on'] is not False:
        problems.append('R2: the macro file does not say start_save_on false after Off: %s' % out['r2_file_on'])
    _away(ctx)
    out['r2_x_before'] = _p1_x(ctx)
    rec._play(ctx, t0, out, 'r2_play', 4.0)
    if out['r2_play_warp']:
        problems.append('R2 Start save Off: Play warped: %s' % out['r2_play_warp'])
    if not out['r2_play_active_ms']:
        problems.append('R2 Start save Off: the macro did not play (no P1 active frames)')
    # R3: Right: On again.
    _mark(t0, out, 'R3')
    _set_switch(ctx, out, 'r3', 'right', macro_id)
    out['r3_file_on'] = _switch_on(ctx, macro_id)
    if out['r3_file_on'] is not True:
        problems.append('R3: the macro file is not On after Right: %s' % out['r3_file_on'])
    _away(ctx)
    rec._play(ctx, t0, out, 'r3_play', 4.0)
    if len(out['r3_play_warp']) != 1:
        problems.append('R3 Start save On again: Play should warp once: %s' % out['r3_play_warp'])
    # R4: P1 CPU Control On: Record and Play on Player 1 are refused.
    _mark(t0, out, 'R4')
    _reset(ctx, 1.2)
    macros_before = rec._macro_ids(ctx)
    cov._page_row(ctx, 'cpu', 'P1 CPU Control')
    steps.press(ctx, 'a', 1.0)
    out['r4_row_shot'] = str(ctx.shot('r4-row-on'))
    steps.close_menu(ctx)
    time.sleep(2.0)
    xs = []
    for _ in range(4):
        xs.append(_p1_x(ctx))
        time.sleep(1.2)
    out['r4_p1_x'] = xs
    if max(xs) - min(xs) < 0.5:
        problems.append('harness: the P1 AI did not move Player 1 (%s): P1 CPU Control was not driving' % xs)
    seq = hs.diag_mark(ctx)
    ctx.play(rec.RECORD)
    time.sleep(2.5)
    lines = [t for _, _, t in hs.diag_since(ctx, seq)]
    out['r4_record_lines'] = [t[:200] for t in lines if 'notice_draw' in t or 'screen=recording' in t][:6]
    out['r4_record_shot'] = str(ctx.shot('r4-record-refused'))
    if not any('P1 is controlled by an AI: recording is off' in t for t in lines):
        problems.append('R4: Record with P1 CPU Control On showed no "P1 is controlled by an AI: recording is off"')
    if any(re.search(PR + r'screen=recording role=player recording=1', t) for t in lines):
        problems.append('R4: Record started while P1 CPU Control played Player 1')
    if rec._macro_ids(ctx) != macros_before:
        problems.append('R4: a macro was written while P1 CPU Control played Player 1')
    seq = hs.diag_mark(ctx)
    ctx.play(rec.PLAY_P1)
    time.sleep(2.5)
    lines = [t for _, _, t in hs.diag_since(ctx, seq)]
    out['r4_play_lines'] = [t[:200] for t in lines if 'notice_draw' in t or 'screen=playback' in t][:6]
    out['r4_play_shot'] = str(ctx.shot('r4-play-refused'))
    if not any('P1 is controlled by an AI: Play on Player 1 is off' in t for t in lines):
        problems.append('R4: Play on Player 1 with P1 CPU Control On showed no "P1 is controlled by an AI: Play on Player 1 is off"')
    if any(re.search(r'screen=playback role=player', t) for t in lines):
        problems.append('R4: Play on Player 1 ran while P1 CPU Control played Player 1')
    # R5: Off again: the same chord records.
    _mark(t0, out, 'R5')
    cov._page_row(ctx, 'cpu', 'P1 CPU Control')
    steps.press(ctx, 'a', 1.0)
    steps.close_menu(ctx)
    time.sleep(2.0)
    _reset(ctx, 1.2)
    try:
        new_id, _, out['r5_record'] = rec._record(ctx, lambda: rec._jab(ctx), 0.3)
        out['r5_new_macro'] = new_id
    except RuntimeError as error:
        problems.append('R5: with P1 CPU Control Off Record wrote no macro: %s' % str(error)[:200])
    _mark(t0, out, 'done')
    return _finish(ctx, 'rc14-recfix', out, problems)


def restore(ctx):
    fx._live_set(ctx, p1_cpu_control=OFF_ON['off'])
    return {'p1_cpu_control': 'off'}


def _kinds(ctx):
    """Both bodies' fighter kinds from one diag-pos.req dump."""
    tail = cov.diag(ctx)
    (cov.card(ctx) / 'diag-pos.req').write_text('1\n')
    seen = {}
    deadline = time.time() + 6
    while len(seen) < 2 and time.time() < deadline:
        for m in POS.finditer(tail.text()):
            seen['player' if int(m.group(1)) == 0 else 'cpu'] = int(m.group(2))
        time.sleep(0.2)
    if len(seen) < 2:
        raise RuntimeError('harness: no state pos dump for both bodies')
    return seen


def slot_setup(ctx):
    kinds = _kinds(ctx)
    if kinds['player'] != kinds['cpu']:
        raise RuntimeError('harness: P1 and the CPU are different fighters (%s): the macro and the CPU must match' % kinds)
    out = rec.hitstun_setup(ctx)
    out['kinds'] = kinds
    return out


def slot_run(ctx):
    t0, out = _begin(ctx)
    problems = []
    out['kinds'] = _kinds(ctx)
    seq = hs.diag_mark(ctx)
    rec._smash(ctx, t0, out, 'smash')
    lines = [t for _, _, t in hs.diag_since(ctx, seq)]
    out['command_query'] = [t[:240] for t in lines if 'command_query' in t][:24]
    out['mash_lines'] = [t[:200] for t in lines if 'playback_mash' in t or 'Playback Slot is empty' in t][:6]
    out['smash_shot'] = str(ctx.shot('recfix-slot-played'))
    if not out.get('smash_start'):
        queued = [t for t in out['command_query'] if 'queued=true' in t]
        problems.append('the CPU did not play Playback Slot 1 after the hit (queue seen: %s)' % ('yes' if queued else 'no'))
    if out.get('smash_empty'):
        problems.append('the slot reads empty although the macro was assigned: %s' % out['smash_empty'])
    return _finish(ctx, 'rc14-recfix-slot', out, problems)


def slot_restore(ctx):
    fx._live_set(ctx, mash_state=[0] * 30)
    return {'mash_state': 'cleared'}
