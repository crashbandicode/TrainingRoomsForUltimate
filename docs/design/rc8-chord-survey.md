# Card chord survey for the rc.8 binds (R + D-pad Up / Down / Left / Right), 2026-11-11

The survey behind `docs/design/rc8-binds.md` §3.1 ("Not while L is held"), done on the primary's order of 20:01Z.
Everything was read-only:

- the configs under `/run/media/intpa/SWITCH SD/ultimate/` (the SD the user exposed; nothing was mounted, written or
  unmounted);
- the strings of the card's 19 plugin NROs in `atmosphere/contents/01006A800016E000/romfs/skyline/plugins/`
  (`nro_chords.py`, `nro_grep.py`, `nro_context.py`; kept with the README searches in the docs lane's
  `artifacts/phase23/feature-gifs-rc8/survey-tools/`);
- the plugins' READMEs on this machine.

## Overlaps with the rc.8 chords

| Plugin (card) | Chord | Overlaps | Source |
| --- | --- | --- | --- |
| Online Deluxe 0.10.4 (`libssbu_online_deluxe_rewrite.nro`) | **L + R + D-pad Down**: next window page | **Input display, R + D-pad Down**: every press flipped the strip (both `fx-record-refused` takes) | README "Hold L+R and tap D-pad Down"; NRO string "L+R+Down next page" |
| Online Deluxe 0.10.4 | **L + R + D-pad Up**: "Capture … L+R+Up (30 s)" on its Performance page | **Game speed, R + D-pad Up**: would open the speed pop-up | NRO string at `0xf201c` |
| Online Deluxe 0.10.4 | L + R + X: toggles the framed icon / profile | none (X) | README; NRO string |
| Online Deluxe 0.10.4 | ZR + D-pad Left/Right: edits a row; plain D-pad Up/Down: selects a row | none with R. ZR + Left/Right is TrainingRooms' own Play on Player 1 / Recording pop-up (rc.7 behaviour, not new) | README; NRO string |
| CPU pilot 0.2.3 (`libssbu_cpu_pilot.nro`) | its CPU Pilot page in Deluxe's window: L + R + Down, ZR + Left/Right, L + R + X | the same as Deluxe's | its README |
| Stages + Counterpick (`libelite_stages_counterpick.nro`, folder `elite-legal-stages-public`) | its own page in Deluxe's window (L + R + Down, ZR + Left/Right) | the same as Deluxe's | its README. The folder's `settings.toml` comment still says "in-game menu (L+R+Up)", but its NRO has no "L+R+Up" string (only Deluxe's has one), so the comment is stale. |
| Smash Replay to MP4 | the game's own L + R + A + Plus (quit playback) and X + D-pad Down (hide display) | none (no R + D-pad) | its README and inventory |
| QuickPlay cancel fix | no chord (`config.ini`: `hold_seconds` only; its 6 source files have no chord) | none | source |
| TrainingRooms itself | Minus (menu shortcut, card `button_config.ini`), B + D-pad Up (permanent), L + R + A (reset, option), ZL/ZR + D-pad (eight rows), R + D-pad Down/Up (rc.8) | none besides the two rc.8 rows' own | `button_config.ini`; `model::chords` |

## Nothing found

ARCropolis, css_preserve, elite_rematch, gsp_visualizer, imgui_smash, instant_back, memory_guard, nro_hook, param_hook,
smash_gamepad_overlay, ssbu_desync_detector, ssbu_quiet_elite_switch, ssbu_refresh_clock:

- no chord text in their NROs;
- no config with a chord;
- no README with a chord on this machine.

**Not covered:** a chord compiled as a button mask with no text. The sources of these plugins aren't on this machine.

## What option 2 covers, and what it doesn't

- **Covered:** L + R + D-pad Down and L + R + D-pad Up, whatever order the buttons come in, while L is held.
- **Not covered:** in Deluxe's window, letting go of L but keeping R held while tapping D-pad Up or Down to pick a row.
  That reaches R + Up or R + Down. Deluxe's README says rows are picked with a plain Up/Down.
