# r1fix stream: the five findings of the Astra review of package 1

Stream `r1fix` (claude-sonnet-5-5, high), lead `0caa72b8`. Worktree
`/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-r1fix`, branch `stream/r1fix`, base `4895ea3`
(pkg2 core `fa8b77d` + the shared Store). Review: `artifacts/trainingrooms-reviews/R1-astra-pkg1/REVIEW.md`
(locations there are at `473da26`).

## Commits (one per finding; test, clippy -D warnings and `build --no-default-features` green after each)

| Finding | Commit |
| --- | --- |
| R4 developer.ini fails closed | `2b7b00a` |
| R3 delay 0 | `949a373` |
| R1 settings recovery transaction | `a449e33` |
| R5 diag hook observation | `aa69d2b` |
| R2 native pause owner | `009f6c8` |
| docs and Eden harness | see `git log` |

Spec text (SPEC-input-03 section 4 native pause row, section 5 step 0, 5.1, 6.7, 9, question 3) is on
`review/menu-proposal` in `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate`, commit `251fec2`
(docs only, not pushed).

## Where things are

- R1 `src/model/settings_file.rs` (runtime keeps `boot::load_settings`); `Store::write_synced` and
  `replace_file` are now `pub` for the reuse.
- R2 (revised by the lead ruling): the producer is `InputAdapter::pause_flag` (`src/model/input_adapter.rs`), fed from the UI tick (`runtime/ui/tick.rs`) by `PAUSE_ACTIVE`; the first attempt (scene `StatePause` hooks) was removed after Eden showed the Training menu does not run it.
  `src/model/input_adapter.rs` (what the hooks do), `src/runtime/features/pause_owner.rs` (two inline hooks),
  `boot.rs` installs both or neither; `published::input_owners()` was removed (nothing read it any more).
- R3 `DelayQueue::step` step 0.
- R4 `src/model/developer_ini.rs` (`load`, `read`, `parse`, `Reason`); install line `developer=...`.
- R5 `src/model/hook_stats.rs` (allocator, `HookScope::enter(HookId)`, `Observer`), diag line from the
  heartbeat thread in `src/runtime/diag_log.rs`; gate `eden/hook_stats.py`.
- Install line buffer raised to 4096 bytes (`install.rs`): 49 site tokens no longer risk the 2048 cut.

## Gates, builds, Eden

Host gates: `$env:CARGO_TARGET_DIR='/home/intpa/smash-training-lobby/cargo-targets/trainingrooms-r1fix'`, one cargo call
each under `systemd-run --user --scope -p MemoryMax=6G /usr/bin/timeout 1800`. Switch builds on the Surface with
`/tmp/r2/sk.ps1`-style launcher (docs/BUILD.md: campaign rustup home, no override); NROs in `dist/` (gitignored).
Eden: Yoga, harness `/home/intpa/trainingrooms-eden-r1` (deployed from `eden/` with `tr_eden.r1fix.config.json`
as `tr_eden.config.json`), profile `r1`, display `:97`, labels `r1-NN`, results in `eden/EDEN-PACKAGE-1.md`
("R1-fix" section).

## Open

- Resident size grew by 16 KiB over `4895ea3` (233,472 to 249,856 B release): see EDEN-PACKAGE-1.md R1-fix section.
