"""Build eden/recording-redesign/rows.json and its authored chord macros (card 10a, 2026-11-07).

Copies the worked examples' patterns (eden/phase23/rows-iv/make_run.py, rows-hb-ab, rows-A):
- chords are authored same-frame macros (playbook docs/MACROS.md rule 14), written to segments/;
- the route segments and gates that already exist come from the shared runner library
  (/home/intpa/ssbu-mod-playbook/tools/eden-runner/library/index.json) by name;
- every row is one project step hook (steps.py:<function>, "check": true), the rows-iv shape;
- the screen chain is one label thread through the whole grouped session: the first row boots
  instance xps-a (boot -> training), every later group's first row re-establishes its own
  instance at Training (the label carried from the previous group's last row).

Run: /usr/bin/python3 -I eden/recording-redesign/make_rows.py   (from the repository root)

The macro event encoding is make_run.py's: events are (frame, buttons, sticks) with the HID
NpadButton bit per button, confirmed in that file for A, B, L, R and D-pad Up; ZL, ZR and the
other D-pad directions follow the same standard HID layout (ZL=0x100, ZR=0x200, Left=0x1000,
Right=0x4000, Down=0x8000). Frame numbers in the movement/combo macros are authored inputs for
the session to tune, not measured facts (README, Open questions).
"""
import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SEG = HERE / 'segments'

# HID NpadButton bits (make_run.py confirms A/B/L/R/Up; the rest is the same standard layout).
A, B, X, Y = 0x1, 0x2, 0x4, 0x8
L, R, ZL, ZR = 0x40, 0x80, 0x100, 0x200
PLUS, MINUS = 0x400, 0x800
DP_LEFT, DP_UP, DP_RIGHT, DP_DOWN = 0x1000, 0x2000, 0x4000, 0x8000
Z = (0, 0, 0, 0)
MOTION = [{"accel": [0, 0, -1], "delta_us": 1000, "gyro": [0, 0, 0]}] * 2


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def macro(name, events, duration):
    """events: (frame, buttons, (lx, ly[, rx, ry])). Same-frame chord macros per MACROS rule 14."""
    out = {"bindings": {"gamepad": "", "keyboard": ""}, "duration_frames": duration, "events": [
        {"buttons": b, "frame": f, "motion": MOTION, "sticks": list(s), "triggers": [0, 0]}
        for f, b, s in events], "format": "eden-input-macro", "game_id": "01006A800016E000",
        "markers": [], "name": name, "parent": {"marker": "", "name": ""}, "player": 0, "version": 1}
    path = SEG / (name + '.edenmacro.json')
    path.write_text(json.dumps(out) + '\n')
    return name, {"file": "segments/%s.edenmacro.json" % name, "sha256": sha(path), "cap": 30}


# --- authored same-frame macros -------------------------------------------------------------------
# Chords (factory defaults, RR-5 design table / card): one frame, then release.
CHORDS = [
    ('chord-save', [(0, ZL | DP_DOWN, Z), (6, 0, Z)], 24),        # Save, ZL+Down [6,9]
    ('chord-load', [(0, ZL | DP_UP, Z), (6, 0, Z)], 24),          # Load, ZL+Up [6,8]
    ('chord-record', [(0, ZR | DP_DOWN, Z), (6, 0, Z)], 24),      # Record, ZR+Down [7,9]
    ('chord-play-cpu', [(0, ZR | DP_UP, Z), (6, 0, Z)], 24),      # Play on CPU, ZR+Up [7,8]
    ('chord-play-p1', [(0, ZR | DP_LEFT, Z), (6, 0, Z)], 24),     # Play on Player 1, ZR+Left [7,10]
    ('chord-popup', [(0, ZR | DP_RIGHT, Z), (6, 0, Z)], 24),      # Recording pop-up, ZR+Right [7,11]
    ('menu-open', [(0, B | DP_UP, Z), (8, 0, Z)], 60),            # Open menu, B+Up (permanent chord)
    ('menu-close', [(0, B, Z), (6, 0, Z), (40, B, Z), (46, 0, Z)], 80),  # rows-iv iv-menu-close
    ('x-press', [(0, X, Z), (6, 0, Z)], 24),                      # cycle the loop, cursor in MACROS
    # Movement and attacks (frames authored; the session tunes them against the described behaviour).
    ('reset-walk', [(0, L | R | A, Z), (4, 0, Z), (70, 0, (1, 0, 0, 0)), (190, 0, Z)], 205),  # rows-iv iv-reset-walk
    ('jab', [(0, A, Z), (2, 0, Z)], 60),                                                        # rows-iv iv-jab
    ('walk-right', [(0, 0, (0.4, 0, 0, 0)), (120, 0, Z)], 140),     # below the dash threshold, then neutral
    ('dash-dance', [(0, 0, (1, 0, 0, 0)), (14, 0, (-1, 0, 0, 0)), (28, 0, (1, 0, 0, 0)),
                    (42, 0, (-1, 0, 0, 0)), (56, 0, (1, 0, 0, 0)), (70, 0, (-1, 0, 0, 0)),
                    (84, 0, (1, 0, 0, 0)), (98, 0, (-1, 0, 0, 0)), (112, 0, Z)], 130),
    ('cstick-smash', [(0, 0, (0, 0, 1, 0)), (8, 0, Z)], 60),        # C-stick right, no A
    ('tilted-a', [(0, 0, (0, 0.6, 0, 0)), (2, A, (0, 0.6, 0, 0)), (4, 0, (0, 0.6, 0, 0)),
                  (12, 0, Z)], 60),                                  # rows-iv combo() shape, left stick tilt
    ('fsmash', [(0, A, Z), (2, 0, Z), (10, 0, (1, 0, 0, 0)), (14, A, (1, 0, 0, 0)),
                (16, 0, (1, 0, 0, 0)), (24, 0, Z)], 90),
    ('nair-loop-clean', [(0, X, Z), (3, 0, Z), (16, A, Z), (18, 0, Z), (44, 0, (0, -1, 0, 0)),
                         (50, 0, Z), (90, X, Z), (93, 0, Z), (106, A, Z), (108, 0, Z),
                         (134, 0, (0, -1, 0, 0)), (140, 0, Z)], 180),
    ('nair-loop-full-hop', [(0, X, Z), (14, 0, Z), (30, A, Z), (32, 0, Z), (60, 0, (0, -1, 0, 0)),
                            (66, 0, Z), (110, X, Z), (124, 0, Z), (140, A, Z), (142, 0, Z),
                            (170, 0, (0, -1, 0, 0)), (176, 0, Z)], 220),
    ('nair-loop-no-fastfall', [(0, X, Z), (3, 0, Z), (16, A, Z), (18, 0, Z), (90, X, Z),
                               (93, 0, Z), (106, A, Z), (108, 0, Z)], 180),
    ('dash-attack', [(0, 0, (1, 0, 0, 0)), (6, A, (1, 0, 0, 0)), (8, 0, (1, 0, 0, 0)),
                     (16, 0, Z)], 90),
]

# Library segments and gates used by the boot route (proven routes; runner library index.json).
LIBRARY_SEGMENTS = ('attract-cancel', 'main-stage', 'stage-css', 'enter-training')
LIBRARY_GATES = ('installed', 'main', 'stage', 'css', 'ready', 'training-hud-unpaused')

INSTANCE_SETUP = ('xps-b', 'ally-a', 'surface-a', 'surface-b', 'yoga-a', 'yoga-b')


def R(row_id, hook, host, timed, expect, source, evidence, media=None, start='training', end='training', args=None):
    return {'id': row_id, 'hook': hook, 'host': host, 'timed': timed, 'start': start, 'end': end,
            'expect': expect, 'source': source, 'evidence': evidence, 'media': media, 'args': args}


STAGE1 = 'LANE-GROK-RECORDING-REDESIGN-REPORT.md'
PLAN = 'EDEN-SESSION-PLAN-rc5.md'
MEDIA = 'docs/MEDIA.md (rc.5 captures still needed)'

ROWS = [
    # --- xps-a: the migration boot (legacy slots fixture pinned before boot), then the runtime rows ---
    R('rr-migrate', 'rr_migrate', 'xps', False,
      "diag line screen=migrate once=1 at the boot's worker tick; the migrated slot's name carries the boot date; a second tick does not rewrite the tree",
      'RR-7 §Eden rows for rr-10 (row rr-migrate); %s A13 and migration table' % STAGE1,
      ['diag screen=migrate once=1', 'macro file names Slot N (YYYY-MM-DD)', 'index migrated:true'],
      start='boot'),
    R('media-rc5-migration', 'media_migration', 'xps', False,
      "The name uses Slot 1 and the date. Play does not warp. Empty slots did not become files.",
      MEDIA + ', row media/rc5-migration.gif',
      ['media/rc5-migration.gif', 'playback diag line warp=0'], media=('media/rc5-migration.gif', 8)),
    R('rr-record', 'rr_record', 'xps', False,
      "diag screen=recording role=player recording=1 on ZR+Down, then screen=recording role=cpu recording=0 on the second ZR+Down after the pop-up role moved to CPU",
      'RR-7 §Eden rows for rr-10 (rows rr-record); %s line 166' % STAGE1,
      ['diag screen=recording role=player recording=1', 'diag screen=recording role=cpu recording=0']),
    R('rr-notice', 'rr_notice', 'xps', False,
      "screen=notice kind=chord-once on the first Record or Play after boot (once), and screen=notice kind=macro-start-save-missing only when the macro's start-save file was deleted",
      'RR-7 §Eden rows for rr-10 (rows rr-notice); %s A15' % STAGE1,
      ['diag screen=notice kind=chord-once', 'diag screen=notice kind=macro-start-save-missing']),
    R('rr-mismatch', 'rr_mismatch', 'xps', False,
      "screen=mismatch ask=macro1 once at the chain's start; Switch names macro 1's fighter; no second prompt when the later macro plays",
      'RR-7 §Eden rows for rr-10 (row rr-mismatch); %s line 326' % STAGE1,
      ['diag screen=mismatch ask=macro1', 'prompt rows Keep current fighter / Switch to recorded fighter']),
    R('rr-rename', 'rr_rename', 'xps', False,
      "screen=rename header=Name macro after A on Rename, then the titled keyboard offer; with Online on, the offer is not made",
      'RR-7 §Eden rows for rr-10 (row rr-rename)', ['diag screen=rename header=Name macro']),
    R('rr-crop', 'rr_crop', 'xps', False,
      "screen=crop source=<id> combo=0 after A on Crop, then the new file write with the original bytes unchanged",
      'RR-7 §Eden rows for rr-10 (row rr-crop)', ['diag screen=crop source= combo=0', 'original combo bytes unchanged']),
    R('rr-load', 'rr_load', 'xps', False,
      "Load (ZL+Up) warps both fighters to the bound save and starts nothing: no playback diag line, the active macros stay idle",
      'RR-4 requirement trace (A5, load_starts_no_playback); %s line 182' % STAGE1,
      ['no playback/load diag line after ZL+Up', 'both fighters at the saved spot']),
    R('rr-chain', 'rr_chain', 'xps', False,
      "the chain warps once to macro 1's start save, then plays macro 1 then macro 2 with no warp between them; the inner macros' start saves are not used",
      'RR-4 requirement trace (A6, a_chain_warps_to_macro_1_start_save_only); %s line 202' % STAGE1,
      ['one warp at chain start', 'no warp before macro 2']),
    R('rr-loop', 'rr_loop', 'xps', False,
      "with the loop at 2 and the cursor in MACROS, X shows 2; the first pass warps once and the second pass does not warp",
      'RR-4 requirement trace (A7, the_loop_loads_the_start_save_once); RR-6 §Eden rows for rr-10 (row rr-loop)',
      ['diag screen=recording-popup role=player loop=2', 'second pass continues without a warp']),

    # --- xps-b: the menu rows (no timing) ---
    R('rr-chords', 'rr_chords', 'xps', False,
      "the eight factory chords keep today's defaults: Save ZL+Down [6,9], Load ZL+Up [6,8], Record ZR+Down [7,9], Play on CPU ZR+Up [7,8], Play on Player 1 ZR+Left [7,10], Recording pop-up ZR+Right [7,11], Cycle Player ZL+Right [6,11], Cycle CPU ZL+Left [6,10]",
      'RR-5 Design factory-row table; %s A14' % STAGE1,
      ['chord table screenshot', 'factory arrays [6,9] [6,8] [7,9] [7,8] [7,10] [7,11] [6,11] [6,10]']),
    R('rr-picker', 'rr_picker', 'xps', False,
      "the numbered picker works as Character Cycles: A gives the next number, R removes and later numbers close up, A on a numbered tile does nothing, R on the last remaining number does nothing, B is done; the chain picker lists recordings only",
      'RR-3 requirement trace (A1, A9); %s lines 197, 203' % STAGE1,
      ['picker screenshot sequence', 'chain picker lists no chain tile']),
    R('rr-hot', 'rr_hot', 'xps', False,
      "screen=recording-popup hot=5 hairline=5: five macros in the green outline, most recently used first, the hairline after the fifth",
      'RR-6 §Eden rows for rr-10 (row rr-hot); %s A10' % STAGE1,
      ['diag screen=recording-popup hot=5 hairline=5']),
    R('rr-icons', 'rr_icons', 'xps', False,
      "screen=recording-popup floppy=off header=on: the row's floppy is grey (126,132,144) with the switch Off, the SAVE STATES header floppy stays orange (242,152,72)",
      'RR-6 §Eden rows for rr-10 (row rr-icons); RR-8 Names rr-6 binds',
      ['diag screen=recording-popup floppy=off header=on']),
    R('rr-page', 'rr_page', 'xps', False,
      "screen=recording-page role=cpu group=saves-macros after one ZR on the Recording page; L and R still change the menu page; the retired rows (Record Slot, Trigger, Duration, Source, Randomize slots) are gone",
      'RR-6 §Eden rows for rr-10 (row rr-page); %s lines 243-249, 316-319' % STAGE1,
      ['diag screen=recording-page role=cpu group=saves-macros', 'retired rows absent from the page']),
    R('rr-popup', 'rr_popup', 'xps', False,
      "screen=recording-popup role=player tab=saves on ZR+Right with the menu closed; B returns to the match and the bound save is unchanged",
      'RR-6 §Eden rows for rr-10 (row rr-popup); %s line 167' % STAGE1,
      ['diag screen=recording-popup role=player tab=saves', 'B closes without changing the selection']),
    R('rr-active', 'rr_active', 'xps', False,
      "screen=recording-popup role=cpu active=pressure after L or R to CPU: each role shows its own active macro, green only in that role's view",
      'RR-6 §Eden rows for rr-10 (row rr-active); %s A8' % STAGE1,
      ['diag screen=recording-popup role=cpu active=pressure']),

    # --- ally-a: the recording-feature GIFs (no timing measurement) ---
    R('media-rc5-record-start-save', 'media_record_start_save', 'ally', False,
      "A new macro at the top of the hot five, an orange floppy, and a start save named with the macro.",
      MEDIA + ', row media/rc5-record-start-save.gif',
      ['media/rc5-record-start-save.gif'], media=('media/rc5-record-start-save.gif', 8)),
    R('media-rc5-play-chord', 'media_play_chord', 'ally', False,
      "One warp to the start save, then the CPU plays the jab. A second press of ZR+Up stops it.",
      MEDIA + ', row media/rc5-play-chord.gif',
      ['media/rc5-play-chord.gif'], media=('media/rc5-play-chord.gif', 8)),
    R('media-rc5-loop-no-warp', 'media_loop_no_warp', 'ally', False,
      "The first pass warps once. The second pass does not warp. It continues from where the first pass ended.",
      MEDIA + ', row media/rc5-loop-no-warp.gif',
      ['media/rc5-loop-no-warp.gif'], media=('media/rc5-loop-no-warp.gif', 8)),
    R('media-rc5-chain-play', 'media_chain_play', 'ally', False,
      "One warp to macro 1's start save, then recording 1, then recording 2, with no warp between them.",
      MEDIA + ', row media/rc5-chain-play.gif',
      ['media/rc5-chain-play.gif'], media=('media/rc5-chain-play.gif', 9)),
    R('media-rc5-popup', 'media_popup', 'ally', False,
      "The window over the match. Player | CPU at the top. Two stacked sections. Green fill on the bound save. White cursor. B closes it and the bound save is unchanged.",
      MEDIA + ', row media/rc5-popup.gif',
      ['media/rc5-popup.gif'], media=('media/rc5-popup.gif', 8)),
    R('media-rc5-recording-page', 'media_recording_page', 'ally', False,
      "Player view shows saves and macros, and Combos & Sequences in the left column. CPU view hides Combos & Sequences. The bound save is green in both.",
      MEDIA + ', row media/rc5-recording-page.gif',
      ['media/rc5-recording-page.gif'], media=('media/rc5-recording-page.gif', 8)),
    R('media-rc5-macro-screen', 'media_macro_screen', 'ally', False,
      "Breadcrumb with the role and the name. Start save with the floppy, switch On. Set active is one row for that role.",
      MEDIA + ', row media/rc5-macro-screen.gif',
      ['media/rc5-macro-screen.gif'], media=('media/rc5-macro-screen.gif', 6)),
    R('media-rc5-fighter-mismatch', 'media_fighter_mismatch', 'ally', False,
      "One Fighter Mismatch prompt, title Fighter Mismatch, rows Keep current fighter and Switch to recorded fighter. It names the first macro's fighter. No second prompt when the later macro plays.",
      MEDIA + ', row media/rc5-fighter-mismatch.gif',
      ['media/rc5-fighter-mismatch.gif'], media=('media/rc5-fighter-mismatch.gif', 7)),
    R('media-rc5-make-combo', 'media_make_combo', 'ally', False,
      "The macro plays once on Player 1 from its start save. A new combo appears. The drilled combo is still the drill when it finishes.",
      MEDIA + ', row media/rc5-make-combo.gif; RR-7 Make combo on a chain/macro',
      ['media/rc5-make-combo.gif'], media=('media/rc5-make-combo.gif', 8)),
    R('media-rc5-crop-combo', 'media_crop_combo', 'ally', False,
      "A new combo of the kept links. The original combo is still in the list.",
      MEDIA + ', row media/rc5-crop-combo.gif',
      ['media/rc5-crop-combo.gif'], media=('media/rc5-crop-combo.gif', 7)),
    R('media-rc5-save-warp', 'media_save_warp', 'ally', False,
      "Both fighters return to the saved spot. No recording starts. The CPU does not play inputs.",
      MEDIA + ', row media/rc5-save-warp.gif',
      ['media/rc5-save-warp.gif'], media=('media/rc5-save-warp.gif', 6)),

    # --- surface-a: timed strip rows, the advantage cases, and the graded GIFs ---
    R('strip-walk', 'strip_walk', 'surface', True,
      "The stick row shows that direction on the frames it was held, and is blank on the neutral frames. No chip unless a button went down.",
      'RR-11 §Eden rows for the grouped session (row Walk)',
      ['stick row screenshot', 'no chip without a button press']),
    R('strip-dash-dance', 'strip_dash_dance', 'surface', True,
      "The row alternates left and right with the lanes. The strip does not freeze while the binned direction keeps changing.",
      'RR-11 §Eden rows for the grouped session (row Dash-dance)',
      ['stick row screenshot']),
    R('strip-cstick-smash', 'strip_cstick_smash', 'surface', True,
      "The chip arrow matches the C-stick. The row matches the left stick.",
      'RR-11 §Eden rows for the grouped session (row C-stick smash)',
      ['chip arrow screenshot']),
    R('strip-tilted-a', 'strip_tilted_a', 'surface', True,
      "The chip arrow matches the left stick. The row matches the left stick.",
      'RR-11 §Eden rows for the grouped session (row Tilted A)',
      ['chip arrow screenshot']),
    R('adv-jab-shield', 'adv_jab_shield', 'surface', True,
      "The strip's ADVANTAGE readout for the jab on the CPU's shield, compared with the published frame-data number for the move; on a mismatch, the first-actionable-frame tracepoints say which side is off",
      PLAN + ' (frame advantage, standard cases)',
      ['advantage readout value', 'published: to be filled by the session (jab on shield)']),
    R('adv-tilt-shield', 'adv_tilt_shield', 'surface', True,
      "The strip's ADVANTAGE readout for a grounded tilt on the CPU's shield, compared with the published frame-data number for the move",
      PLAN + ' (frame advantage, standard cases)',
      ['advantage readout value', 'published: to be filled by the session (grounded tilt on shield)']),
    R('adv-smash-shield', 'adv_smash_shield', 'surface', True,
      "The strip's ADVANTAGE readout for a smash attack on the CPU's shield, compared with the published frame-data number for the move",
      PLAN + ' (frame advantage, standard cases)',
      ['advantage readout value', 'published: to be filled by the session (smash attack on shield)']),
    R('adv-aerial-shield', 'adv_aerial_shield', 'surface', True,
      "The strip's ADVANTAGE readout for an aerial on the CPU's shield that lands after the aerial, compared with the published frame-data number for the move",
      PLAN + ' (frame advantage, standard cases)',
      ['advantage readout value', 'published: to be filled by the session (aerial on shield, landing after)']),
    R('adv-move-hit', 'adv_move_hit', 'surface', True,
      "The strip's ADVANTAGE readout for one move on hit with a CPU that is not mashing, compared with the published frame-data number for the move",
      PLAN + ' (frame advantage, standard cases)',
      ['advantage readout value', 'published: to be filled by the session (move on hit, CPU not mashing)']),
    R('media-rc5-grade-not-true', 'media_grade_not_true', 'surface', True,
      "Attempt 1 still counts (the bar does not require TRUE). Attempt 2 drops, and the bar says the move was wrong.",
      MEDIA + ', row media/rc5-grade-not-true.gif; RR-12 §Grade bar faults',
      ['media/rc5-grade-not-true.gif'], media=('media/rc5-grade-not-true.gif', 9)),
    R('media-rc5-sequence-drill', 'media_sequence_drill', 'surface', True,
      "The bar names the sequence and moves from combo 1 to combo 2. After the drop it is back on combo 1, streak 0.",
      MEDIA + ', row media/rc5-sequence-drill.gif; %s A4' % STAGE1,
      ['media/rc5-sequence-drill.gif'], media=('media/rc5-sequence-drill.gif', 9)),

    # --- surface-b: the frame-strip GIFs (timed captures) ---
    R('media-rc5-frame-strip-lanes', 'media_strip_lanes', 'surface', True,
      "P1 gold startup, red active, blue recovery. CPU light-grey hitlag, then purple hitstun. A run of 7 or more shows its length. Newest frame on the right.",
      MEDIA + ', row media/rc5-frame-strip-lanes.gif',
      ['media/rc5-frame-strip-lanes.gif'], media=('media/rc5-frame-strip-lanes.gif', 6)),
    R('media-rc5-frame-strip-chips', 'media_strip_chips', 'surface', True,
      "One green attack chip, an up-right arrow on it, and a white tick down to P1's lane. No second chip.",
      MEDIA + ', row media/rc5-frame-strip-chips.gif',
      ['media/rc5-frame-strip-chips.gif'], media=('media/rc5-frame-strip-chips.gif', 4)),
    R('media-rc5-frame-strip-freeze', 'media_strip_freeze', 'surface', True,
      "The lanes stop moving while both stand, then move again on the second jab.",
      MEDIA + ', row media/rc5-frame-strip-freeze.gif',
      ['media/rc5-frame-strip-freeze.gif'], media=('media/rc5-frame-strip-freeze.gif', 5)),
    R('media-rc5-frame-strip-advantage-hit', 'media_strip_advantage_hit', 'surface', True,
      "ADVANTAGE is a signed number, not a dash, and the line under it reads on hit. Plus is green. Minus is red.",
      MEDIA + ', row media/rc5-frame-strip-advantage-hit.gif',
      ['media/rc5-frame-strip-advantage-hit.gif'], media=('media/rc5-frame-strip-advantage-hit.gif', 5)),
    R('media-rc5-frame-strip-advantage-shield', 'media_strip_advantage_shield', 'surface', True,
      "ADVANTAGE reads on shield. LINK does not become TRUE combo or DROP for that shield hit.",
      MEDIA + ', row media/rc5-frame-strip-advantage-shield.gif',
      ['media/rc5-frame-strip-advantage-shield.gif'], media=('media/rc5-frame-strip-advantage-shield.gif', 6)),
    R('media-rc5-frame-strip-link', 'media_strip_link', 'surface', True,
      "LINK shows a number of 0 or below and the words TRUE combo, in green. A green bar above P1's lane reads window Nf.",
      MEDIA + ', row media/rc5-frame-strip-link.gif',
      ['media/rc5-frame-strip-link.gif'], media=('media/rc5-frame-strip-link.gif', 5)),
    R('media-rc5-frame-strip-drop', 'media_strip_drop', 'surface', True,
      "LINK shows a positive number and the word DROP, in red.",
      MEDIA + ', row media/rc5-frame-strip-drop.gif',
      ['media/rc5-frame-strip-drop.gif'], media=('media/rc5-frame-strip-drop.gif', 6)),
    R('media-rc5-frame-strip-input', 'media_strip_input', 'surface', True,
      "INPUT shows Nf late or on time or buffered Nf, and then either N spare or Nf past.",
      MEDIA + ', row media/rc5-frame-strip-input.gif',
      ['media/rc5-frame-strip-input.gif'], media=('media/rc5-frame-strip-input.gif', 6)),
    R('media-rc5-frame-strip-last', 'media_strip_last', 'surface', True,
      "The LAST row shows up to four pills, newest first. A pill reads TRUE or DROP with its number, and the moves read like Jab > Jab.",
      MEDIA + ', row media/rc5-frame-strip-last.gif',
      ['media/rc5-frame-strip-last.gif'], media=('media/rc5-frame-strip-last.gif', 7)),
    R('media-rc5-frame-strip-exchange', 'media_strip_exchange', 'surface', True,
      "The second hit does not turn LINK into a large DROP of the first exchange. After both can act, ADVANTAGE is the new exchange, on hit.",
      MEDIA + ', row media/rc5-frame-strip-exchange.gif',
      ['media/rc5-frame-strip-exchange.gif'], media=('media/rc5-frame-strip-exchange.gif', 8)),

    # --- yoga-a: the rr-11 stick-row GIFs and the combo rows ---
    R('media-rc5-frame-strip-cstick', 'media_strip_cstick', 'yoga', True,
      "One attack chip with a right arrow. The left stick never left neutral.",
      MEDIA + ', row media/rc5-frame-strip-cstick.gif',
      ['media/rc5-frame-strip-cstick.gif'], media=('media/rc5-frame-strip-cstick.gif', 4)),
    R('media-rc5-frame-strip-stick-row', 'media_strip_stick_row', 'yoga', True,
      "A row under P1's lane. A direction only while the stick is held. Blank cells while neutral. The row scrolls with the lanes.",
      MEDIA + ', row media/rc5-frame-strip-stick-row.gif',
      ['media/rc5-frame-strip-stick-row.gif'], media=('media/rc5-frame-strip-stick-row.gif', 6)),
    R('media-rc5-frame-strip-freeze-stick', 'media_strip_freeze_stick', 'yoga', True,
      "The strip keeps scrolling while the stick changes direction. It does not freeze.",
      MEDIA + ', row media/rc5-frame-strip-freeze-stick.gif',
      ['media/rc5-frame-strip-freeze-stick.gif'], media=('media/rc5-frame-strip-freeze-stick.gif', 5)),
    R('combo-nair-loop-clean', 'combo_nair_loop_clean', 'yoga', True,
      "Clean. Events are jump press, jump release, attack press, fast-fall. Drift may show and does not change the verdict.",
      'RR-12 §Eden rows (row Nair loop, drilled clean)',
      ['bar verdict clean', 'events jump press, jump release, attack press, fast-fall']),
    R('combo-nair-loop-full-hop', 'combo_nair_loop_full_hop', 'yoga', True,
      "Drop. The bar names the late jump release.",
      'RR-12 §Eden rows (row Same loop with a full hop)',
      ['bar verdict drop', 'bar names the late jump release']),
    R('combo-nair-loop-no-fastfall', 'combo_nair_loop_no_fastfall', 'yoga', True,
      "Drop. The bar names a missing fast-fall.",
      'RR-12 §Eden rows (row Same loop without the fast-fall)',
      ['bar verdict drop', 'bar names a missing fast-fall']),
    R('combo-dash-attack', 'combo_dash_attack', 'yoga', True,
      "Clean when the dash flick and the attack press are both there, in order, facing-relative. A missing cross drops on the dash.",
      'RR-12 §Eden rows (row Dash-attack combo)',
      ['bar verdict clean on the dash flick + attack press']),

    # --- yoga-b: the grade-bar and drill rows (combo grading, timed) ---
    R('rr-grade-bar', 'rr_grade_bar', 'yoga', True,
      "a non-TRUE link inside the tolerance still counts; a missing press, a press outside the tolerance and a gap past 20 frames drop; a drop restarts the sequence at combo 1 with streak 0 and best kept; the bar still draws TRUE",
      'RR-2 requirement trace (A2, A3, A4); %s lines 147-149, 198' % STAGE1,
      ['bar screenshots of the four cases', 'streak 0 after the drop']),
    R('rr-drill', 'rr_drill', 'yoga', True,
      "Drilling a combo or the sequence replaces the drill (A, no chord); Make combo plays the file once on Player 1 from its start save, adds the new combo to the hot five, and leaves the drill unchanged",
      'RR-2 requirement trace (A11); %s lines 178, 191, 200, 259, 265, 267' % STAGE1,
      ['drill screenshots', 'Make combo leaves the drill']),

    # --- the final row: the heap and crash gates on every instance ---
    R('heap-and-crash-check', 'heap_crash', 'xps', False,
      "on every instance: the install line says heap_allocations=0, the hook-stats snapshot shows no steady-state hook allocations after the whole session, and the console log has no Unmapped and no panic lines",
      PLAN + ' (gates on every instance); playbook CHECKLIST (no heap in hooks)',
      ['install line heap_allocations=0 on all 7 instances', 'hook-stats snapshot', 'console Unmapped/panic scan']),
]


def main():
    SEG.mkdir(parents=True, exist_ok=True)
    segments = {name: {'library': name} for name in LIBRARY_SEGMENTS}
    for name, events, duration in CHORDS:
        segments[name] = macro(name, events, duration)[1]
    gates = {name: {'library': name} for name in LIBRARY_GATES}
    gates['main']['recover'] = 'attract-cancel'
    gates['main']['recover_times'] = 3

    rows = []
    # Instance keys: the operator's capacity is surface 2, yoga 2, ally 1, xps 2. Each host's rows
    # are contiguous in ROWS; the host's instances split at the counts below (xps-a then xps-b,
    # surface-a then surface-b, yoga-a then yoga-b).
    SPLIT = {'xps': 10, 'surface': 11, 'yoga': 7}
    INSTANCES = {'xps': ('xps-a', 'xps-b'), 'surface': ('surface-a', 'surface-b'),
                 'yoga': ('yoga-a', 'yoga-b'), 'ally': ('ally-a',)}
    counts, previous_key = {}, None
    # Group 1 boots instance xps-a itself (the migration boot), so its first row starts on boot.
    for i, spec in enumerate(ROWS):
        counts[spec['host']] = counts.get(spec['host'], 0) + 1
        keys = INSTANCES[spec['host']]
        key = keys[0] if (counts[spec['host']] <= SPLIT.get(spec['host'], 0) or len(keys) == 1) else keys[1]
        steps = [{'run': 'steps.py:%s' % spec['hook'], 'check': True, 'on': key}]
        if spec['args']:
            steps[0]['args'] = spec['args']
        row = {'id': spec['id'], 'start': spec['start'], 'end': spec['end'], 'steps': steps,
               'evidence': spec['evidence'], 'source': spec['source'], 'host': spec['host'],
               'expect': spec['expect'], 'timed': spec['timed']}
        if spec['media']:
            row['media'] = {'gif': spec['media'][0], 'seconds': spec['media'][1]}
        # Before each group's first row (except the very first), re-establish that instance at
        # Training: the installed gate, then the boot hook (game.load if needed, the library route,
        # the Mario/Mario pick, enter Training).
        if spec['id'] not in ('rr-migrate', 'heap-and-crash-check') and i > 0 and previous_key != key:
            rows.append({'id': 'setup-' + key, 'start': 'training', 'end': 'training',
                         'steps': [{'gate': 'installed', 'on': key},
                                   {'run': 'steps.py:boot_training', 'check': True, 'on': key}],
                         'evidence': ['install line cpu_common=installed', 'Training HUD settled'],
                         'source': 'rows-iv/rows-hb-ab boot route (library segments, steps.py hook)',
                         'host': spec['host'], 'expect': 'the instance is back at Training, HUD unpaused',
                         'timed': False})
        previous_key = key
        rows.append(row)
    # The heap row gains one hook step per instance, all in its last row.
    heap = rows[-1]
    heap['steps'] = [{'run': 'steps.py:heap_crash', 'check': True, 'on': key}
                     for key in ('xps-a', 'xps-b', 'ally-a', 'surface-a', 'surface-b', 'yoga-a', 'yoga-b')]

    plan = {
        'game_id': '01006A800016E000',
        'start': 'boot',
        'scope': 'rc.5 grouped Eden session (card 10a): the recording-redesign rows from the stage-1 report, '
                 'the rc.5 frame-advantage cases and heap/crash gates (EDEN-SESSION-PLAN-rc5), and every GIF '
                 'docs/MEDIA.md still needs, across the operator\'s 7 instances (surface 2, yoga 2, ally 1, xps 2). '
                 'Timed rows (frame advantage, strip timing, combo grading) run on the Surface and the Yoga only. '
                 'Instance keys: xps-a, xps-b, ally-a, surface-a, surface-b, yoga-a, yoga-b.',
        'setup': [
            {'state': 'Instance xps-a boots with the migration fixture: a non-empty recording in slot 1, the other recording slots empty, legacy save slots live',
             'via': 'fixture', 'ref': 'RR-7 Eden row rr-migrate start state; media-rc5-migration start'},
            {'state': 'Every instance: the rc.5 release NRO with all slices merged, plugins installed, Eden muted (core.volume=0 persisted)',
             'via': 'fixture', 'ref': 'EDEN-SESSION-PLAN-rc5; operator instruction 2026-10-09'},
            {'state': 'Training, Battlefield, P1 Mario and CPU Mario at 0%, Input Display Frame strip, Hitbox Visualization Off, CPU Shield None and DI/SDI cleared unless the row says otherwise, menu closed',
             'via': 'segment', 'ref': 'library segments attract-cancel, main-stage, stage-css, enter-training; steps.py:boot_training'},
            {'state': 'Shared menu facts: chords at their factory defaults (Save ZL+Down, Load ZL+Up, Record ZR+Down, Play on Player 1 ZR+Left, Play on CPU ZR+Up, Recording pop-up ZR+Right)',
             'via': 'fixture', 'ref': 'docs/MEDIA.md shared start; RR-5 factory-row table'},
        ],
        'segments': segments,
        'gates': gates,
        'rows': rows,
    }
    (HERE / 'rows.json').write_text(json.dumps(plan, indent=2) + '\n')
    print('rows', len(rows), 'segments', len(segments), sha(HERE / 'rows.json')[:12])


if __name__ == '__main__':
    sys.exit(main())