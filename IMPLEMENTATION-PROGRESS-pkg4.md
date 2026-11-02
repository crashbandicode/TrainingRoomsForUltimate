# Implementation progress, package 4 (stream `pkg4`)

Branch `stream/pkg4` (worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-pkg4`), base 473da26, merged
`work/implementation` fa8b77d (pkg2 core) and 4895ea3 (shared Store). Implementer: Claude Sonnet 5.5 (high) under lead `0caa72b8`.
Playbook followed: `009c9ac`. Target `/home/intpa/smash-training-lobby/cargo-targets/trainingrooms-pkg4`; Yoga build folder
`/home/intpa/trainingrooms-build-pkg4`; Eden labels `s4-NN`, work root `/home/intpa/trainingrooms-eden-p4`, display `:94`.

## Done

- S4-1 recorder: `model/recording.rs` (state machine, slots, lockout/standby/recording/playback, crop, loop, ledge buffer, mirror,
  hitstun timing, term table, mash start/interrupt, triggers, named-layer contract), runtime `features/recorder.rs`.
- S4-2 chords and notices: `model/chords.rs`, `model/menu_shortcuts.rs` (carried, N7 writer), `model/ini_files.rs` (fail-closed reads,
  transactional writes), `model/notify.rs`, `model/damage_display.rs`; runtime `features/{chords,notices}.rs`; Combo Display copy and
  the stale-moves toggle hooks.
- S4-3 input display: `model/input_display.rs` (logger, rules, icons, fade, render plan), runtime `features/input_display.rs`.
- Sites: `model/sites_pkg4.rs` (12 rows, generated from the ELF by `tools/gen_sites_pkg4.py`); hook at the AI control function's epilogue
  `main+0x2dbc38` (the pilot hooks its entry `0x2da180`), caller-side poll hooks shared with Input Delay.
- Q7 carried `capture_settings.rs`; the menu env default follows the INI (off when absent).
- Gates at HEAD: `cargo test` all pass, `cargo clippy --all-targets -- -D warnings` zero, `cargo build --no-default-features` clean, Switch release
  and diag builds warning-free on the Yoga.

## Waiting / not built

- S4-4 carry of the named-macro / replay / clip layer waits for pkg6's C3 (`training_mod_mii`); rulings in the lead's note of 2026-11-02
  (vendor `smash-match-data`, keep `deluxe-sdk` feature, serde derive pinned like pkg6).
- Pane writes (input display, notice slot, damage helper) wait for package 5's layout and anchors.
- Mash-triggered playback / interrupt: needs pkg2's per-frame mash trigger evaluation. Save-state triggers: pkg3's restore completion.
- Status names (SPEC-input-02 question 1): numeric ids; parked as an operator decision.

## Next commands

`git -C /home/intpa/smash-training-lobby/TrainingRoomsForUltimate-pkg4 log --oneline -5`; Eden: `python3 -I eden/route4.py s4-01` then
`rows4.py s4-01 A` (diag NRO + pilot + tile cursor, `eden/settings` boot A), then s4-02 / B (reversed load order). Results go to
`eden/EDEN-PACKAGE-4.md`.
