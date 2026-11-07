"""Build eden/recording-redesign/rows.json (card 10a2, 2026-11-07; card 10c adds rr-7's runtime rows).

Copies the worked examples' patterns (eden/phase23/rows-iv/make_run.py, rows-hb-ab, rows-A):
- every non-library segment is a RECIPE the runner records on its first play (playbook
  docs/MACROS.md, "Test runs": recipe inputs are the only single inputs the runner ever sends);
  no .edenmacro.json is written by hand and no segment carries a sha256 before it is recorded;
- the route segments and gates that already exist come from the shared runner library
  (/home/intpa/ssbu-mod-playbook/tools/eden-runner/library/index.json) by name;
- every row is one project step hook (steps.py:<function>, "check": true), the rows-iv shape;
- the screen chain is one label thread through the whole grouped session: the first row boots
  instance xps-a (boot -> training), every later group's first row re-establishes its own
  instance at Training (the label carried from the previous group's last row);
- playbook a37664e ("Test runs", rule 8): every row after a host's first carries a start gate and
  the plan carries one recover list, so a failed row does not stop the run (card 10b).

Run: /usr/bin/python3 -I eden/recording-redesign/make_rows.py   (from the repository root)

Recipe syntax (runner.py docstring and record()): {"buttons": "a", "hold": 150, "gap": 0.6},
{"buttons": "b", "with_stick": [0, -1]}, {"stick": [x, y, ms]}; "frames": N and "gap_frames": N
hold for N game frames of the recording where timing matters. Button names are input.button's
vocabulary (the runs' actions.jsonl shows zl, zr; the rc2d base steps.py shows up, down, left,
right, a, b). Frame counts below are the recipe's best effort from the described behaviour, not
measured timings: every recipe carries note "timing to be checked when recorded", and the facts
no source gave are listed in README.md, Open questions.
"""
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent

# --- recipes (recorded on first play; no file is written by hand) ---------------------------------
# Chords (factory defaults, RR-5 design table / card): the runner's recipe presses the action's
# buttons together (each held on its lease through the hold, released together).
# Timed gameplay recipes use the thresholds rr-12 measured in the stock 13.0.5 params
# (dash_stick_x 0.85, dive_cont_value -0.66, jump_stick_y 0.7); the hold and gap frame counts are
# the recipe's best effort, to be checked when recorded.
NOTE = 'timing to be checked when recorded'
RECIPES = {
    'chord-save': [{"buttons": "zl,down", "frames": 2, "gap_frames": 10}],          # Save, ZL+Down [6,9]
    'chord-load': [{"buttons": "zl,up", "frames": 2, "gap_frames": 10}],            # Load, ZL+Up [6,8]
    'chord-record': [{"buttons": "zr,down", "frames": 2, "gap_frames": 10}],        # Record, ZR+Down [7,9]
    'chord-play-cpu': [{"buttons": "zr,up", "frames": 2, "gap_frames": 10}],        # Play on CPU, ZR+Up [7,8]
    'chord-play-p1': [{"buttons": "zr,left", "frames": 2, "gap_frames": 10}],       # Play on Player 1, ZR+Left [7,10]
    'chord-popup': [{"buttons": "zr,right", "frames": 2, "gap_frames": 10}],        # Recording pop-up, ZR+Right [7,11]
    'menu-open': [{"buttons": "b,up", "frames": 2, "gap_frames": 10}],              # Open menu, B+Up (permanent chord)
    'menu-close': [{"buttons": "b", "frames": 2, "gap_frames": 6},                  # B, then B again to leave
                   {"buttons": "b", "frames": 2, "gap_frames": 34}],
    'x-press': [{"buttons": "x", "frames": 2, "gap_frames": 10}],                   # cycle the loop, cursor in MACROS
    # Training's position reset (L+R+A in one action), settle, then walk into the CPU (rows-iv's
    # iv-reset-walk shape).
    # Position reset only. The old reset-walk then held right for 120 frames, so a "before"
    # shot was already dashing and a following jab often missed (surface strip-walk, LAST, exchange).
    'reset-center': [{"buttons": "l,r,a", "frames": 4, "gap_frames": 70}],
    'reset-walk': [{"buttons": "l,r,a", "frames": 4, "gap_frames": 66},
                   {"stick": [1, 0, 0], "frames": 120, "gap_frames": 14}],
    'jab': [{"buttons": "a", "frames": 2, "gap_frames": 58}],
    # Two-hit link, then a third jab, then stand. Gaps are inside Mario jab's cancel window so LAST
    # gets pills (docs/MEDIA.md last row; the first pass waited 0.5 s and the LAST row stayed empty).
    'jab-link': [{"buttons": "a", "frames": 2, "gap_frames": 14},
                 {"buttons": "a", "frames": 2, "gap_frames": 30}],
    'jab-link-3': [{"buttons": "a", "frames": 2, "gap_frames": 14},
                   {"buttons": "a", "frames": 2, "gap_frames": 14},
                   {"buttons": "a", "frames": 2, "gap_frames": 40}],
    # First jab, then a gap that lands the second jab after the CPU can act and inside 20 frames of
    # that (MEDIA drop / input). 42 frames from the press: hit ~f6, stun ends ~f25, second press ~f42.
    'jab-drop': [{"buttons": "a", "frames": 2, "gap_frames": 42},
                 {"buttons": "a", "frames": 2, "gap_frames": 30}],
    # A full second of neutral after the first jab (more than 20 frames after the CPU can act), then
    # the second jab is a new exchange (MEDIA exchange / freeze).
    'exchange-jabs': [{"buttons": "a", "frames": 2, "gap_frames": 90},
                      {"buttons": "a", "frames": 2, "gap_frames": 40}],
    'freeze-jabs': [{"buttons": "a", "frames": 2, "gap_frames": 90},
                    {"buttons": "a", "frames": 2, "gap_frames": 20}],
    # Stick held through the A press, past both thresholds, so the chip is one green up-right attack
    # chip. "hold" not "frames" on the with_stick action: record() asks for lease_ms 10200 when a
    # framed with_stick press is used (FRAME_LEASE_MS + 200), and Eden rejects that.
    'chips-attack': [{"stick": [0.95, 0.95, 0], "frames": 6, "gap_frames": 0},
                     {"buttons": "a", "with_stick": [0.95, 0.95], "hold": 80, "gap": 0.5}],
    'lanes-smash': [{"stick": [1, 0, 0], "frames": 8, "gap_frames": 0},
                    {"buttons": "a", "with_stick": [1, 0], "hold": 80, "gap": 1.2}],
    # A walk: the stick held below the dash threshold (rr-12: dash_stick_x 0.85), then neutral.
    'walk-right': [{"stick": [0.4, 0, 0], "frames": 30, "gap_frames": 10}],
    # A dash-dance: alternate left and right without pausing (rr-11, row Dash-dance).
    'dash-dance': [{"stick": [1, 0, 0], "frames": 12, "gap_frames": 0},
                   {"stick": [-1, 0, 0], "frames": 12, "gap_frames": 0},
                   {"stick": [1, 0, 0], "frames": 12, "gap_frames": 0},
                   {"stick": [-1, 0, 0], "frames": 12, "gap_frames": 0},
                   {"stick": [1, 0, 0], "frames": 12, "gap_frames": 0},
                   {"stick": [-1, 0, 0], "frames": 12, "gap_frames": 0},
                   {"stick": [1, 0, 0], "frames": 12, "gap_frames": 0},
                   {"stick": [-1, 0, 0], "frames": 12, "gap_frames": 0},
                   {"stick": [0, 0, 0], "frames": 2, "gap_frames": 10}],
    # A forward smash: dash right (past the walk threshold), then A (docs/MEDIA.md, lanes row).
    'fsmash': [{"stick": [1, 0, 0], "frames": 6, "gap_frames": 0},
               {"buttons": "a", "with_stick": [1, 0], "hold": 60, "gap": 0.8}],
    # A tilted A: the left stick held at a tilt (below the dash threshold) through the A press
    # (rows-iv's combo shape: the tilt held, never a flick).
    'tilted-a': [{"stick": [0, 0.65, 0], "frames": 3, "gap_frames": 0},
                 {"buttons": "a", "with_stick": [0, 0.65], "hold": 60, "gap": 0.4}],
    # A short-hop nair loop, drilled clean (rr-12, row "Nair loop, drilled clean"): jump press,
    # jump release (short hop), attack press, fast-fall (the stick crossing -0.66), repeated.
    'nair-loop-clean': [{"buttons": "x", "frames": 3, "gap_frames": 0},
                        {"buttons": "a", "frames": 2, "gap_frames": 20},
                        {"stick": [0, -1, 0], "frames": 6, "gap_frames": 34},
                        {"buttons": "x", "frames": 3, "gap_frames": 0},
                        {"buttons": "a", "frames": 2, "gap_frames": 20},
                        {"stick": [0, -1, 0], "frames": 6, "gap_frames": 34},
                        {"stick": [0, 0, 0], "frames": 2, "gap_frames": 10}],
    # The same loop with the jump released later than the on-time tolerance (rr-12: Drop; the bar
    # names the late jump release).
    'nair-loop-full-hop': [{"buttons": "x", "frames": 14, "gap_frames": 0},
                           {"buttons": "a", "frames": 2, "gap_frames": 20},
                           {"stick": [0, -1, 0], "frames": 6, "gap_frames": 34},
                           {"buttons": "x", "frames": 14, "gap_frames": 0},
                           {"buttons": "a", "frames": 2, "gap_frames": 20},
                           {"stick": [0, -1, 0], "frames": 6, "gap_frames": 34},
                           {"stick": [0, 0, 0], "frames": 2, "gap_frames": 10}],
    # The same loop without the fast-fall (rr-12: the down input never crosses -0.66 in the air).
    'nair-loop-no-fastfall': [{"buttons": "x", "frames": 3, "gap_frames": 0},
                              {"buttons": "a", "frames": 2, "gap_frames": 20},
                              {"gap_frames": 40},
                              {"buttons": "x", "frames": 3, "gap_frames": 0},
                              {"buttons": "a", "frames": 2, "gap_frames": 20},
                              {"gap_frames": 40},
                              {"stick": [0, 0, 0], "frames": 2, "gap_frames": 10}],
    # A dash-attack combo (rr-12, row "Dash-attack combo"): the stick crosses 0.85, then the attack.
    'dash-attack': [{"stick": [1, 0, 0], "frames": 2, "gap_frames": 0},
                    {"buttons": "a", "with_stick": [1, 0], "hold": 50, "gap": 0.5}],
    # The C-stick smash: the runner's recipe actions drive the LEFT stick only (record()'s stick
    # action), so a recipe cannot express this input. The recipe below is a placeholder that must
    # not be played as-is; the row's hook presses the right stick directly and captures what the
    # control call answers (README, Open questions).
    'cstick-smash': [{"stick": [0, 0, 0], "frames": 2, "gap_frames": 40}],
}
# The one segment whose recipe is a placeholder gets its own note.
CSTICK_NOTE = ('the recipe actions drive the left stick only; the C-stick input is pressed by the '
               'row hook (README, Open questions); timing to be checked when recorded')

# Library segments and gates used by the boot route (proven routes; runner library index.json).
LIBRARY_SEGMENTS = ('attract-cancel', 'main-stage', 'stage-css', 'enter-training')
LIBRARY_GATES = ('installed', 'main', 'stage', 'css', 'ready', 'training-hud-unpaused')

# Recovery (card 10b; playbook a37664e, "Test runs" rule 8: the run continues past a failed row
# when the next row has a start gate and a recover list). One plan-level list fits every in-match
# row, because they all start at the same screen (Training, menu closed): B closes the menu or the
# pop-up that a failed row may have left open (the recipe's two B presses; with both already closed
# the fighter's special plays, which the Load after it undoes), then Load (ZL+Up) warps both
# fighters to the bound save and starts nothing (RR-4 A5), and nothing else plays. The start gate
# that proves that screen is the library gate training-hud-unpaused; recover_to checks it after
# the list. The setup rows start somewhere else (their own instance, not a match screen), so each
# carries its own recover list: the boot hook their steps already run, on that instance.
RECOVER = [{'play': 'menu-close'}, {'play': 'chord-load'}]


def R(row_id, hook, host, timed, expect, source, evidence, media=None, start='training', end='training', args=None,
      gates=None, trace=None, build=None):
    # gates: plan gate names appended to the row's steps after the hook (a log gate on the diag
    # source is the card's preferred form for a runtime row's exact diag line). trace: RR-7's
    # tracepoint note, carried in the row because RR-7 names functions only and trace.arm needs a
    # resolved address (README, Open questions). build: "diag" or "release" for the confirming re-run.
    return {'id': row_id, 'hook': hook, 'host': host, 'timed': timed, 'start': start, 'end': end,
            'expect': expect, 'source': source, 'evidence': evidence, 'media': media, 'args': args,
            'gates': gates, 'trace': trace, 'build': build}


STAGE1 = 'LANE-GROK-RECORDING-REDESIGN-REPORT.md'
PLAN = 'EDEN-SESSION-PLAN-rc5.md'
MEDIA = 'docs/MEDIA.md (rc.5 captures still needed)'

ROWS = [
    # --- xps-a: the migration boot (legacy slots fixture pinned before boot), then the runtime rows ---
    R('rr-migrate', 'rr_migrate', 'xps', False,
      "diag line screen=migrate once=1 at the boot's worker tick; the migrated slot's name carries the boot date; a second tick does not rewrite the tree",
      'RR-7 §Eden rows for rr-10 (row rr-migrate); %s A13 and migration table' % STAGE1,
      ['diag screen=migrate once=1', 'macro file names Slot N (YYYY-MM-DD)', 'index migrated:true'],
      start='boot', gates=('diag-migrate',),
      trace='RR-7: migrate_at_boot; capture the slot name containing the boot date; a second tick does not rewrite the tree. Symbols only (README, Open questions).'),
    R('media-rc5-migration', 'media_migration', 'xps', False,
      "The name uses Slot 1 and the date. Play does not warp. Empty slots did not become files.",
      MEDIA + ', row media/rc5-migration.gif',
      ['media/rc5-migration.gif', 'playback diag line warp=0'], media=('media/rc5-migration.gif', 8)),
    R('rr-record', 'rr_record', 'xps', False,
      "diag screen=recording role=player recording=1 on ZR+Down, then screen=recording role=cpu recording=0 on the second ZR+Down after the pop-up role moved to CPU",
      'RR-7 §Eden rows for rr-10 (rows rr-record); %s line 166' % STAGE1,
      ['diag screen=recording role=player recording=1', 'diag screen=recording role=cpu recording=0'],
      gates=('diag-record-player', 'diag-record-cpu'),
      trace="RR-7: recording_host::on_chords when the record bit is set; capture the capture role and whether recording is on, and that the stop wrote the macro only when frames were stored. Symbols only (README, Open questions)."),
    R('rr-notice', 'rr_notice', 'xps', False,
      "screen=notice kind=chord-once on the first Record or Play after boot (once), and screen=notice kind=macro-start-save-missing only when the macro's start-save file was deleted",
      'RR-7 §Eden rows for rr-10 (rows rr-notice); %s A15' % STAGE1,
      ['diag screen=notice kind=chord-once', 'diag screen=notice kind=macro-start-save-missing'],
      gates=('diag-notice-missing',),
      trace="RR-7: start_save_notice for Deleted (a missing id with the switch off does not print this line); the chord-once flag is read and set by the worker, a second chord does not print it. Symbols only (README, Open questions)."),
    R('rr-mismatch', 'rr_mismatch', 'xps', False,
      "screen=mismatch ask=macro1 once at the chain's start; Switch names macro 1's fighter; no second prompt when the later macro plays",
      'RR-7 §Eden rows for rr-10 (row rr-mismatch); %s line 326' % STAGE1,
      ['diag screen=mismatch ask=macro1', 'prompt rows Keep current fighter / Switch to recorded fighter'],
      gates=('diag-mismatch',),
      trace='RR-7: chain_mismatch; capture step 0 only, and that Switch names macro 1\'s fighter. Symbols only (README, Open questions).'),
    R('rr-rename', 'rr_rename', 'xps', False,
      "screen=rename header=Name macro after A on Rename, then the titled keyboard offer; with Online on, the offer is not made",
      'RR-7 §Eden rows for rr-10 (row rr-rename)', ['diag screen=rename header=Name macro'],
      gates=('diag-rename',),
      trace='RR-7: Effect::Keyboard, then the titled keyboard offer; online captures the offer not made. Symbols only (README, Open questions).'),
    R('rr-crop', 'rr_crop', 'xps', False,
      "screen=crop source=<id> combo=0 after A on Crop, then the new file write with the original bytes unchanged",
      'RR-7 §Eden rows for rr-10 (row rr-crop)', ['diag screen=crop source= combo=0', 'original combo bytes unchanged'],
      gates=('diag-crop',),
      trace='RR-7: Effect::Crop, then the new file write; capture the original bytes unchanged (a combo crops with combo=1). Symbols only (README, Open questions).'),
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
    # --- rr-7's runtime rows (card 10c; RR-7 §Eden rows for rr-10) ------------------------------------
    R('rr-play', 'rr_play', 'xps', False,
      "screen=playback role=player warp=1 on Play on Player 1 ([7,10]) when Player 1's active macro has a start save and the switch is on, with the warps count staying 1 across the loop; then screen=playback role=cpu warp=0 on Play on CPU (the old Input Playback chord) when the CPU's active macro has no start save",
      'RR-7 §Eden rows for rr-10 (row rr-play, both cases)',
      ['diag screen=playback role=player warp=1', 'diag screen=playback role=cpu warp=0',
       'warps stay 1 across the loop'],
      gates=('diag-play-player', 'diag-play-cpu'),
      trace="RR-7: the worker drain into Playback::play_chord; capture Effect.warp and warps staying 1 across the loop; the CPU case is the same drain with Role::Cpu, capture no warp. Symbols only (README, Open questions)."),
    R('rr-mash', 'rr_mash', 'xps', False,
      "with a role playing and the Mash interrupt on, the mash that already calls interrupt_playback prints screen=playback mash=1 and stops the playing macro (playing false)",
      'RR-7 §Eden rows for rr-10 (row rr-mash)',
      ['diag screen=playback mash=1', 'playing false after the mash'],
      gates=('diag-mash',),
      trace='RR-7: recording_host::note_mash, then Playback::mash; capture playing false. Symbols only (README, Open questions).'),
    R('rr-combo', 'rr_combo', 'xps', False,
      "the combo's Connections panel lists each link's events on a second line - jump · release · ↓ flick · A - then drift → on the next line; the published frame, not a new diag print",
      'RR-7 §Eden rows for rr-10 (row rr-combo)',
      ['panel line jump · release · ↓ flick · A', 'drift → line under it', 'panel screenshot'],
      trace='RR-7: recording_menu::combo_link_lines reads the combo file; the draw hook paints the frame the UI worker already published. The panel line is the published frame, not a diag print, so this row has no log gate.'),

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
      ['trace return of tr_combo_link_v1 with the clean flag', 'events jump press, jump release, attack press, fast-fall'],
      gates=('trace-combo-link',), build='release',
      trace='Release: tr_combo_link_v1 on_return (src/runtime/features/combo.rs). x0 index, x1 link, x2 flags (bit 1 clean), x3 offset. The diag "drill " line is not printed by the release NRO.'),
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
      ['trace return of tr_combo_attempt_v1 for the four cases', 'streak 0 after the drop'],
      gates=('trace-combo-attempt',), build='release',
      trace='Release: tr_combo_attempt_v1 on_return. x0 outcome (1 completed, 2 stopped, 3 cleared), x1 true links, x2 total, x3 first bad. A screenshot alone does not pass the row.'),
    R('rr-drill', 'rr_drill', 'yoga', True,
      "Drilling a combo or the sequence replaces the drill (A, no chord); Make combo plays the file once on Player 1 from its start save, adds the new combo to the hot five, and leaves the drill unchanged",
      'RR-2 requirement trace (A11); %s lines 178, 191, 200, 259, 265, 267' % STAGE1,
      ['diag [Ui] press on row Drill', 'diag [Ui] press on row Make combo', 'the drill id is unchanged after Make combo'],
      gates=('diag-drill-press', 'diag-make-combo-press'), build='diag'),

    # --- the final row: the heap and crash gates on every instance ---
    R('f9-body-trace', 'f9_body_trace', 'surface', False,
      "placeholder: trace f9_body_of's return. LANE-GROK-RC5-FIX-F9-REPORT.md was not present; the arm spec is TASK-GROK-rc5-fix-f9.md (entry, [entry+0x4160], the state word, slot, on_return)",
      'TASK-GROK-rc5-fix-f9.md tracepoint spec; LANE-GROK-RC5-FIX-F9-REPORT.md absent at row-fix time',
      ['trace_jsonl return of f9_body_of'],
      gates=('trace-f9-body',), build='release',
      trace='Placeholder. Symbol training_rooms::runtime::roster::character_sources::f9_body_of (the task names character_roster::sources::f9_body_of; the source file is character_sources.rs). on_return, captures of entry, [entry+0x4160], the state word, and slot. Not a diag print.'),
    R('heap-and-crash-check', 'heap_crash', 'surface', False,
      "on every re-run instance, release and diag: TrainingRooms' own events.log line plugin=training_rooms/ result=installed heap_allocations=0, the hook-stats snapshot shows no steady-state hook allocations, and the console log has no Unmapped and no panic lines",
      PLAN + ' (gates on every instance); playbook CHECKLIST (no heap in hooks)',
      ['TrainingRooms events.log result=installed heap_allocations=0 on surface-a, surface-b, yoga-a, yoga-b',
       'hook-stats snapshot', 'console Unmapped/panic scan'],
      build='release'),
]


def main():
    segments = {name: {'library': name} for name in LIBRARY_SEGMENTS}
    for name, recipe in RECIPES.items():
        segments[name] = {'file': 'segments/%s.edenmacro.json' % name, 'cap': 30, 'recipe': recipe,
                          'note': CSTICK_NOTE if name == 'cstick-smash' else NOTE}
    gates = {name: {'library': name} for name in LIBRARY_GATES}
    gates['main']['recover'] = 'attract-cancel'
    gates['main']['recover_times'] = 3
    # Card 10c: each runtime row's exact diag line (RR-7 §Eden rows for rr-10) as a log gate on the
    # diag source. The row's run hook waits for the same line and raises without it; the gate step
    # after the hook re-checks the line in this row's own log lines (the runner scopes log gates to
    # the row). rr-combo has no gate: RR-7 says the Connections panel line is the published frame,
    # not a diag print.
    diag_gates = {
        'diag-migrate': {'kind': 'log', 'source': 'diag', 'pattern': r'screen=migrate once=1', 'cap': 10},
        'diag-record-player': {'kind': 'log', 'source': 'diag', 'pattern': r'screen=recording role=player recording=1', 'cap': 10},
        'diag-record-cpu': {'kind': 'log', 'source': 'diag', 'pattern': r'screen=recording role=cpu recording=0', 'cap': 10},
        'diag-notice-missing': {'kind': 'log', 'source': 'diag', 'pattern': r'screen=notice kind=macro-start-save-missing', 'cap': 10},
        'diag-mismatch': {'kind': 'log', 'source': 'diag', 'pattern': r'screen=mismatch ask=macro1', 'cap': 10},
        'diag-rename': {'kind': 'log', 'source': 'diag', 'pattern': r'screen=rename header=Name macro', 'cap': 10},
        'diag-crop': {'kind': 'log', 'source': 'diag', 'pattern': r'screen=crop source=\S+ combo=0', 'cap': 10},
        'diag-play-player': {'kind': 'log', 'source': 'diag', 'pattern': r'screen=playback role=player warp=1', 'cap': 10},
        'diag-play-cpu': {'kind': 'log', 'source': 'diag', 'pattern': r'screen=playback role=cpu warp=0', 'cap': 10},
        'diag-mash': {'kind': 'log', 'source': 'diag', 'pattern': r'screen=playback mash=1', 'cap': 10},
        'diag-adv-shield': {'kind': 'log', 'source': 'diag', 'pattern': r'\[FrameStrip\] advantage value=-?\d+ on_shield=true', 'cap': 12},
        'diag-adv-hit': {'kind': 'log', 'source': 'diag', 'pattern': r'\[FrameStrip\] advantage value=-?\d+ on_shield=false', 'cap': 12},
        'diag-drill-press': {'kind': 'log', 'source': 'diag', 'pattern': r'\[Ui\] press .*Drill', 'cap': 12},
        'diag-make-combo-press': {'kind': 'log', 'source': 'diag', 'pattern': r'\[Ui\] press .*Make combo', 'cap': 12},
        # Release still exports these (combo.rs, no_mangle, not cfg(diag)). sample_frame's advantage
        # print is cfg(diag) and the function returns (), so advantage rows stay on the diag build.
        'trace-combo-link': {'kind': 'log', 'source': 'trace_jsonl',
                             'pattern': r'(?:"event":"return".*tr_combo_link_v1|tr_combo_link_v1.*"event":"return")',
                             'cap': 15},
        'trace-combo-attempt': {'kind': 'log', 'source': 'trace_jsonl',
                                'pattern': r'(?:"event":"return".*tr_combo_attempt_v1|tr_combo_attempt_v1.*"event":"return")',
                                'cap': 15},
        'trace-f9-body': {'kind': 'log', 'source': 'trace_jsonl',
                          'pattern': r'(?:"event":"return".*f9_body_of|f9_body_of.*"event":"return")',
                          'cap': 30},
    }
    gates.update(diag_gates)
    # Reference frames are 1280x674. A full-window grab is 1280x720; Ally's grab was 2560x1440.
    # runner-fix2 accepts "scale": "fit". Tolerances and reference hashes stay as they are.
    for name in ('main', 'stage', 'css', 'ready', 'training-hud-unpaused'):
        gates[name]['scale'] = 'fit'

    # Confirming re-run: Surface 2 + Yoga 2. surface-a and yoga-a are release. surface-b and yoga-b
    # are diag. Ally and XPS never reached Training, so their rows move onto these four instances.
    SURFACE_B = ['rr-migrate', 'rr-record', 'rr-notice', 'rr-mismatch', 'rr-rename', 'rr-crop',
                 'rr-play', 'rr-mash', 'adv-jab-shield', 'adv-tilt-shield', 'adv-smash-shield',
                 'adv-aerial-shield', 'adv-move-hit']
    SURFACE_A = ['strip-walk', 'strip-dash-dance', 'strip-cstick-smash', 'strip-tilted-a',
                 'media-rc5-frame-strip-lanes', 'media-rc5-frame-strip-chips', 'media-rc5-frame-strip-freeze',
                 'media-rc5-frame-strip-advantage-hit', 'media-rc5-frame-strip-advantage-shield',
                 'media-rc5-frame-strip-link', 'media-rc5-frame-strip-drop', 'media-rc5-frame-strip-input',
                 'media-rc5-frame-strip-last', 'media-rc5-frame-strip-exchange',
                 'media-rc5-grade-not-true', 'media-rc5-sequence-drill', 'f9-body-trace']
    YOGA_A = ['media-rc5-migration', 'media-rc5-record-start-save', 'media-rc5-play-chord',
              'media-rc5-loop-no-warp', 'media-rc5-chain-play', 'media-rc5-popup',
              'media-rc5-recording-page', 'media-rc5-macro-screen', 'media-rc5-fighter-mismatch',
              'media-rc5-make-combo', 'media-rc5-crop-combo', 'media-rc5-save-warp',
              'media-rc5-frame-strip-cstick', 'media-rc5-frame-strip-stick-row',
              'media-rc5-frame-strip-freeze-stick', 'rr-chords', 'rr-picker', 'rr-combo',
              'combo-nair-loop-clean', 'combo-nair-loop-full-hop', 'combo-nair-loop-no-fastfall',
              'combo-dash-attack', 'rr-grade-bar']
    YOGA_B = ['rr-load', 'rr-chain', 'rr-loop', 'rr-hot', 'rr-icons', 'rr-page', 'rr-popup',
              'rr-active', 'rr-drill']
    INSTANCE = {}
    for name in SURFACE_B:
        INSTANCE[name] = ('surface-b', 'diag')
    for name in SURFACE_A:
        INSTANCE[name] = ('surface-a', 'release')
    for name in YOGA_A:
        INSTANCE[name] = ('yoga-a', 'release')
    for name in YOGA_B:
        INSTANCE[name] = ('yoga-b', 'diag')
    INSTANCE['heap-and-crash-check'] = ('surface-a', 'release')
    ORDER = SURFACE_B + SURFACE_A + YOGA_A + YOGA_B + ['heap-and-crash-check']
    by_id = {spec['id']: spec for spec in ROWS}
    missing = [name for name in ORDER if name not in by_id]
    extra = [spec['id'] for spec in ROWS if spec['id'] not in INSTANCE]
    if missing or extra:
        raise SystemExit('row assignment mismatch missing=%s extra=%s' % (missing, extra))
    ADV_GATE = {
        'adv-jab-shield': 'diag-adv-shield', 'adv-tilt-shield': 'diag-adv-shield',
        'adv-smash-shield': 'diag-adv-shield', 'adv-aerial-shield': 'diag-adv-shield',
        'adv-move-hit': 'diag-adv-hit',
    }
    COMBO_TRACE = {'combo-nair-loop-full-hop', 'combo-nair-loop-no-fastfall', 'combo-dash-attack'}

    rows = []
    previous_key = None
    for spec in (by_id[name] for name in ORDER):
        key, build = INSTANCE[spec['id']]
        host = 'surface' if key.startswith('surface') else 'yoga'
        steps = [{'run': 'steps.py:%s' % spec['hook'], 'check': True, 'on': key}]
        if spec['args']:
            steps[0]['args'] = spec['args']
        gates_for = list(spec['gates'] or ())
        if spec['id'] in ADV_GATE and ADV_GATE[spec['id']] not in gates_for:
            gates_for.append(ADV_GATE[spec['id']])
        if spec['id'] in COMBO_TRACE and 'trace-combo-link' not in gates_for:
            gates_for.append('trace-combo-link')
        row = {'id': spec['id'], 'start': spec['start'], 'end': spec['end'], 'steps': steps,
               'evidence': spec['evidence'], 'source': spec['source'], 'host': host,
               'expect': spec['expect'], 'timed': spec['timed'],
               'build': spec.get('build') or build, 'on': key}
        if spec.get('trace'):
            row['trace'] = spec['trace']
        elif spec['id'] in COMBO_TRACE:
            row['trace'] = ('Release: tr_combo_link_v1 on_return. x0 index, x1 link, x2 flags '
                            '(bit 1 clean, bit 2 wrong move, bit 3 missing).')
        if gates_for:
            row['steps'] = row['steps'] + [{'gate': name, 'on': key} for name in gates_for]
        if spec['id'] != 'rr-migrate':
            row['start_gate'] = 'training-hud-unpaused'
        if spec['media']:
            row['media'] = {'gif': spec['media'][0], 'seconds': spec['media'][1]}
        if spec['id'] not in ('rr-migrate', 'heap-and-crash-check') and previous_key != key:
            rows.append({'id': 'setup-' + key, 'start': 'training', 'end': 'training',
                         'steps': [{'gate': 'installed', 'on': key},
                                   {'run': 'steps.py:boot_training', 'check': True, 'on': key}],
                         'evidence': ['TrainingRooms events.log result=installed', 'Training HUD settled'],
                         'source': 'rows-iv/rows-hb-ab boot route (library segments, L3+D-pad, steps.py hook)',
                         'host': host, 'expect': 'the instance is at Training, HUD unpaused, P1 on Mario',
                         'timed': False, 'build': build, 'on': key,
                         'start_gate': 'training-hud-unpaused',
                         'recover': [{'run': 'steps.py:boot_training', 'check': True, 'on': key}]})
        previous_key = key
        rows.append(row)
    heap = rows[-1]
    heap['steps'] = [{'run': 'steps.py:heap_crash', 'check': True, 'on': key}
                     for key in ('surface-a', 'surface-b', 'yoga-a', 'yoga-b')]
    heap['build'] = 'release-and-diag'

    plan = {
        'game_id': '01006A800016E000',
        'start': 'boot',
        'scope': 'rc.5 confirming re-run on Surface and Yoga, two instances each. surface-a and yoga-a load the '
                 'release NRO. surface-b and yoga-b load the diag NRO from the same commit. Ally and XPS rows moved '
                 'here. Diag-line gates run on the diag instances. tr_combo_link_v1, tr_combo_attempt_v1 and '
                 'f9_body_of are trace gates on release. Heap and crash run on all four instances. Media stays on '
                 'release. Timed rows stay on the Surface and the Yoga.',
        'setup': [
            {'state': 'surface-b (diag) and yoga-a (release) boot with a non-empty recording in slot 1 and the other recording slots empty',
             'via': 'fixture', 'ref': 'RR-7 Eden row rr-migrate; media-rc5-migration'},
            {'state': 'surface-a and yoga-a load the release NRO; surface-b and yoga-b load the diag NRO of the same commit. Eden muted (core.volume=0). Both windows of a host leave the intro before timed rows',
             'via': 'fixture', 'ref': 'EDEN-SESSION-PLAN-rc5; operator 2026-10-09 and 2026-10-26'},
            {'state': 'Training, Battlefield, P1 Mario and CPU Mario at 0%, Input Display Frame strip, Hitbox Visualization Off, CPU Shield None and DI/SDI cleared unless the row says otherwise, menu closed',
             'via': 'segment', 'ref': 'library segments attract-cancel, main-stage, stage-css, enter-training; steps.py:boot_training'},
            {'state': 'Shared menu facts: chords at their factory defaults (Save ZL+Down, Load ZL+Up, Record ZR+Down, Play on Player 1 ZR+Left, Play on CPU ZR+Up, Recording pop-up ZR+Right)',
             'via': 'fixture', 'ref': 'docs/MEDIA.md shared start; RR-5 factory-row table'},
        ],
        'segments': segments,
        'gates': gates,
        'recover': RECOVER,
        'rows': rows,
    }
    (HERE / 'rows.json').write_text(json.dumps(plan, indent=2) + '\n')
    print('rows', len(rows), 'segments', len(segments))


if __name__ == '__main__':
    sys.exit(main())