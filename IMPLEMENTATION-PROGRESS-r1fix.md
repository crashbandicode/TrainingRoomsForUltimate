# r1fix stream: the five findings of the Astra review of package 1

Stream `r1fix` (claude-sonnet-5-5, high), lead `0caa72b8`. Worktree
`/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-r1fix`, branch `stream/r1fix`, base `4895ea3`, merged with
`work/implementation` `c5863bd` (merge `ffc93e4`). Review: `artifacts/trainingrooms-reviews/R1-astra-pkg1/REVIEW.md`.

## Commits (one per finding; gates green after each)

| Finding | Commit |
| --- | --- |
| R4 developer.ini fails closed | `2b7b00a` |
| R3 delay 0 | `949a373` |
| R1 settings recovery transaction | `a449e33` |
| R5 diag hook observation | `aa69d2b` |
| R2 native pause owner (first attempt: scene `StatePause` hooks) | `009f6c8` |
| scene wrapper consumers (lead order) | `ba67245` |
| merge of work/implementation and reconciliation | `ffc93e4` |
| R2 by the lead ruling (UI flag producer; `StatePause` hooks removed) | `e2effe9` |

Spec text on `review/menu-proposal` (docs only, not pushed): `251fec2` (first), `c3c8a7b` (Eden correction), `9fe08d0` (the
`L_staying_help` flag as the authoritative source).

## Where things are

- R1 `src/model/settings_file.rs` (runtime keeps `boot::load_settings`); one transactional write: `Store::write_replace`.
- R2 `InputAdapter::pause_flag` / `pause_begin` / `pause_end` / `poll` in `src/model/input_adapter.rs`; the runtime calls
  them from `features/input_delay.rs` (poll, `pause_flag`) and from the UI tick (`runtime/ui/tick.rs`, the edge of
  `PAUSE_ACTIVE`). `published::input_owners()` is kept for the other packages.
- R3 `DelayQueue::step` step 0.
- R4 `src/model/developer_ini.rs` (`load`, `read`, `parse`, `Reason`); install line `developer=on|off reason=...`.
- R5 `src/model/hook_stats.rs` (allocator, `HookScope::enter()` unnamed / `enter_hook(HookId)` named, `Observer`), diag line
  from the heartbeat thread in `src/runtime/diag_log.rs`; gate `eden/hook_stats.py`.

## Gates, builds, Eden

Host gates: `$env:CARGO_TARGET_DIR='/home/intpa/smash-training-lobby/cargo-targets/trainingrooms-r1fix'`, one cargo call each
under `systemd-run --user --scope -p MemoryMax=6G /usr/bin/timeout 1800`. All test binaries pass except `tests/input_sites.rs`
`every_row_matches_the_owned_elf` (pre-existing on work: `work_get_int` is a row of both `sites_pkg2` and `sites_pkg4`; the
wire stream fixes it). Switch builds on the Surface with the campaign rustup home (docs/BUILD.md). NROs (gitignored `dist/`):
release `11f74916...` 778,240 B resident, diag `9c4ea584...` 933,888 B.
Eden: Yoga, `/home/intpa/trainingrooms-eden-r1`, profile `r1`, display `:98`, labels `r1-NN`, results in
`eden/EDEN-PACKAGE-1.md` ("R1-fix").

## Open

- Nothing from the review. Not done here: `tools/check/compliance.py` (no READY claimed).
