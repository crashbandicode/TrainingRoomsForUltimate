# Implementation progress, stream `wire` (cross-stream wiring of packages 2-5)

Branch `stream/wire` (worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-wire`), base `c5863bd` of `work/implementation`. Implementer
Claude Sonnet 5.5 (high) under lead `0caa72b8`, 2026-11-02. Playbook `ae87443` read in full. Host gates and Switch builds on the Surface (compile-only
claims in the Yoga ledger, owner `Claude 0caa72b8/wire`), target `/home/intpa/smash-training-lobby/cargo-targets/trainingrooms-wire`. Eden: Yoga, work
root `/home/intpa/trainingrooms-eden-wire`, profile `w1`, display `:92`, labels `w-NN`, rig `eden/tr_eden_w.py`, `eden/route_w.py`, `eden/rows_w.py`.

## Items (the lead's list)

| Item | State | Commit |
| --- | --- | --- |
| 1 one `work_get_int` row (pkg2's) | done, host + Switch | `5cfc546` |
| 2 0-byte guards | nothing to fix: the five "0-byte guards" `compliance.py` counts are type declarations (`guard: [u8; GUARD_BYTES]` in `sites.rs`, `sites_pkg2.rs`'s `CommonSite`, three in the carried `native_seams/mod.rs`) matched by its regex; every row has 64 bytes (826 of 826). Checker contract not changed; asking the lead | none |
| 3 chords -> pkg3 | done (the Save and Load chords are pulled in `states::on_fighter_frame`, SPEC-input-04 section 2) | `e75ede2` |
| 4 pkg3 -> pkg4, loading fact | done; `states::is_loading` is authoritative (the load machine's own flag; `cpu.save_loading` had no producer); `is_killing` removed; pkg3's duplicate recording trigger removed (pkg4's `on_state_restored` decides, with the ledge exclusion) | `1275816`, `e75ede2` |
| 5 HUD draw | done (`features/hud_draw.rs`, registered after `ui::install`); notice slot and input log panes written by name | `e75ede2` |
| 6 outbox, writer | done (writer always started; settings, shortcuts, replay capture via the writer thread) | `e75ede2` |
| 7 scene | `set_lobby_active` wired in `cpu::set_lobby`; `is_training` used by the save-state hooks and the HUD draw; `is_native_training` by the reset check, hazards, reset hub, stale toggle | `e75ede2` |
| 8 inboxes | `CPU_PLAYBACK_REQUEST`, `CPU_STOP_PLAYBACK`, `FRAME_ADVANTAGE_*` removed: direct calls (`recorder::request_playback`, `recorder::interrupt_playback`, `notices::post_frame_advantage`) | `1275816`, `e75ede2` |
| 9 x3 | same object (holder; Controller at +0x10); pkg5 read the style at +0x60, the game reads +0x9c: fixed | `3a682e9` |
| 10 shared natives | `game::natives::{fighter_entry, entry_body, fighter_accessor, module_accessor, is_fighter, is_grounded, is_operation_cpu, lr, is_stop, set_int, set_color_rgb}`; rows `utility_get_category`, `utility_get_kind` | `7eeec7c` |
| 11 pkg4 accessors | `recording_or_playback`, `replay_editor_occupied`, `replay_capture_loading` | `6271629` |
| 12 notices API | `upsert`, `clear`, `is_queued`, `take_delivered` (standing notices, host-tested) | `d60b88c` |
| 13 mii_preferences, `ui::clear_session_state` | done | `79680f8`, `3257e01` |

Commits 10-13 for pkg6: `7eeec7c`, `6271629`, `d60b88c`, `79680f8`, `3257e01` (merge up to `3257e01`).

## Gates

Surface, worktree HEAD `3a682e9`: `cargo test --no-fail-fast` all pass, `cargo clippy --all-targets -- -D warnings` 0, `cargo build --no-default-features` ok,
Switch release and diag build. Release `c31d2c0d4cf9463dd050ef53587de79fffcbe617780c888372f9927840c1e82c` (file 729,088 B, resident 786,432 B); diag
`72cf357068f64d23285ae6337fcb651bbf83606c9c1ed25eda253442a6eba9b8` (778,240 B file, 933,888 B resident). Release warnings: only the pkg6-facing API of items 10-12 that has no
consumer in this tree (`cpu::situation_ground`, `game::natives::{module_accessor, is_fighter, is_grounded}`, `notices::{upsert, clear, is_queued, take_delivered}`,
`recorder::recording_or_playback`). `compliance.py`: no diag strings in the release NRO; machine rows still failing are the project-level ones (README GIF,
HANDOFF.md, dirty tree) and the guard-size row (see item 2).

## Eden w-01 (batched, one boot)

Waits for the Yoga (queue line 5; the fork's `m5qo1` online round holds both hosts). Rows: `eden/rows_w.py` (install line and HUD draw registered, input log and notice
on screen, frame advantage notice, Save/Load chords, recorder stop on a load and restore, Recording Trigger = Save State Load, Playback Mash Interrupt, shortcut edit and
replay-capture toggle through the writer thread with R5 delta 0, a real menu close writing `settings.json`).

Next commands: `ssh memento-claw-yoga-ubuntu`, claim per the queue file, then from the Surface worktree:
`ssh … "cd /home/intpa/trainingrooms-eden-wire; python3 -I tr_eden_w.py up w-01 --nro inbox/diag/libtraining_rooms.nro --nro inbox/libssbu_tile_cursor.nro --nro inbox/pilot/libssbu_cpu_pilot.nro --settings inbox/boot-w.json"`,
`python3 -I eden/route_w.py w-01`, `python3 -I eden/rows_w.py w-01`, `tr_eden_w.py scan w-01`, `tr_eden_w.py down w-01`, `tr_eden_w.py restore`.

## Build log note (2026-11-02)

This stream built on the Surface at 06:27-06:28Z, 06:35-06:36Z and 06:39-06:40Z (host gates and Switch release/diag builds, each under a compile-only claim and RELEASED line in the Yoga ledger). None of them was at ~06:48Z; the fork's m5qo1 claim opened at 06:41Z, after the last one. At 06:35Z the foreign note (06:33Z) asking for no Surface cargo applied only to its own open claim, which did not exist yet. No build of this stream has run since 06:40Z; from here on no NRO/Switch build or full clippy runs on a host with an open claim of any owner.

