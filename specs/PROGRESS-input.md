# PROGRESS — area `input` (recording/playback, input display, input delay, button chords, notifications)

Spec writer: Claude (Fable 5.1), started 2026-11-01. Read-only research; no code, no builds, nothing outside `specs/`.

## Sources read

- Upstream `97cbe9b9`: `src/training/{input_record,input_log,input_delay,frame_counter,mod,mash,ledge}.rs`, `src/training/ui/{input_log,display,damage,notifications,mod,menu}.rs`, `src/common/{input,button_config,offsets,mod,menu}.rs`, `training_mod_consts/src/{options,lib,files}.rs`, `src/training/save_states.rs` (load/record/playback paths).
- Upstream `077a8e43`: `src/training/mod.rs` once-per-frame change only.
- Fork stage M5i: the same files plus `src/training/{practice,lobby,reset,named_macros}.rs`, `src/training/ui/{native_menu,layout_registry}.rs`, `training_mod_consts/src/menu_shortcuts.rs`, `training_mod_macros/src/{recording,lib}.rs`, `training_mod_tui/src/macro_panel.rs` (slot-origin helpers), docs `named-macros.md`, `reference/named-macros.md`, `menu-shortcuts.md`, `input-display-performance.md`, `input-display-benchmark.md`, `input-display-followup.md`, `REVISION29_DRAW_DECOUPLING.md` (anchors), `REVISION32_SMALL_PROFILE.md`.
- Playbook `docs/CHECKLIST.md` (hook safety, evidence table wording).

## Status

- [x] Reading complete (2026-11-01).
- [x] SPEC-input-01-recording-and-playback.md (written 2026-11-01; 17 open questions)
- [x] SPEC-input-02-input-display.md (written 2026-11-01; 5 open questions; fork renderer folded in as required behaviour with measured numbers)
- [x] SPEC-input-03-input-delay.md (written 2026-11-01; 4 open questions)
- [x] SPEC-input-04-button-chords.md (written 2026-11-01; 5 open questions; fork Plus decision folded in, INI shortcuts stated as interface)
- [x] SPEC-input-05-notifications-and-damage-display.md (written 2026-11-01; 6 open questions)
- [x] INDEX-input.md (written 2026-11-01)

## Done

All six files written. Nothing committed; nothing outside `specs/` touched. Final report returned to the lead.
