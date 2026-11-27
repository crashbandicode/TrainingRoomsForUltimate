# Toggle fix: the task's checklist (playbook `c8d9959`, `docs/CHECKLIST.md`)

2026-11-27. Branch `fix/toggle-load-wait` from `rc-next` `cc71537c`. Filled in at READY; evidence goes beside each row
in `toggle/COMPLIANCE.md`.

## Implementation

- **M** cdylib only; release profile; toolchain: unchanged by this branch (machine rows).
- **M** Pure logic in the model with host tests: the editor's states in `clip_panel.rs`; the ticket in
  `roster_requests.rs`, tested through `tests/support/clip_admission.rs`.
- **M** Gates: test, clippy, no-default-features, release, diag (the Switch builds deny warnings).
- **M** No heap in hooks, fixed buffers: no hook is added; the game-frame code touches one atomic word.
- **M** Root cause fixed; no workaround: the gate and the roster's outcome. No bound of mine. The two older bounds
  found are named to tr-primary-2 and not changed.
- **M** Every user-facing refusal cites an approved restriction: the refusal line is the existing
  `clip-fighter-refused`; the deleted "did not load" line had no row. No new refusal text.
- **M** Design report with a requirement-trace table before the code: `toggle/DESIGN.md`.

## Hook safety

- **M** Version guard, 64-byte guards, card scan, hook after the prologue: N/A, no hook or site is added or moved.
- **C** Hook sharing: N/A, no site.
- **M** No game hook blocks on a plugin lock: the new game-frame code runs in `roster_requests::service` (the lobby
  update on MainThread) and takes the role state lock the roster already takes there; the worker side reads atomics.
  The lock tests must pass in the gate.
- **M** No diag code or strings in the shipped NRO; nothing per frame written: no log line is added to the release
  build.

## Online

- **C** N/A as a new path: the clip editor runs in Training only. Not reviewed or run online by this lane.

## Eden

- All rows N/A for this lane: no Eden run. dup-audit flies one boot after the fold (toggle rows for Player 1 and the
  CPU, there and back). The evidence rule for that boot: trace the build under test, no diag build to log.

## Evidence and docs

- **M** Dated evidence doc: `toggle/DESIGN.md`, the gate receipts under `docs-tutorials/remote/toggle-*`, the READY post.
- **M** No operator quotes in shipped files; no claims beyond the tests.
- **C** A recorded decision is touched: SM3 A (`docs/DECISIONS.md`, 2026-11-26): "B cancels" and "One load at a time".
  The fix keeps both; B after the roster has begun follows the roster, which the order of 00:40:25Z rules.
- **M** Handoff file current: `docs-tutorials/HANDOFF.md`.

## Release and card

- **M** Nothing pushed, tagged or written to the card.

## Git and routing

- **M** Commits from a UTF-8 file, as crashbandicode; no push.
- **M** Model gate and watcher: as on P0, P5b and P6 (the pair is ruled in policy; no watcher armed; stated).
- **M** A spawned worker reads the playbook itself, builds its own checklist and runs compliance before its READY.
