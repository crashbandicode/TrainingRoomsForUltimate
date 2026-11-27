# Toggle fix: the fighter toggle waits on the roster, not on a count

2026-11-27, lane docs-tutorials. Worktree `/home/intpa/tr-fixes-20261113/toggle-wait`, branch `fix/toggle-load-wait`
from `rc-next` `cc71537cb7405690c468ecc9e42d776bca8176bd`. Playbook `c8d9959`, read completely before this file.
Order: tr-primary-2, inbox 2026-11-27 00:40:25Z. Finding: dup-audit, inbox 00:38:37Z, clip 2. Reviewer: rv-hit-streak.

## Root cause (read in the code at `cc71537c`; nothing run in a game by this lane)

1. `clip_panel.rs` `tick_fighter_load` ends the editor's wait after 600 worker passes. Nobody ruled that bound.
2. `cancel_fighter` clears only the editor's fields. The roster keeps the request.
3. The cause under both: while the toggle waits, the editor stays visible, so `macros::sync_owner` publishes the owner
   `REPLAY_EDITOR` (`runtime/macros/mod.rs:151`), which `Owners::holds_gameplay()` counts (`model/input_delay.rs:151`).
   The roster services its pending request on the fighter's frame (`character_roster.rs:1453`): while
   `menu::captures_input()` it keeps the request for up to 180 fighter frames, then drops it with "Character swap
   unavailable during this action". So an accepted request commits only once the editor closes. The older Switch path
   avoids this by parking the editor, which publishes `REPLAY_LOADING` (issue #68); part C did not.

## Design

### 1. The gate

- `replay_capture::fighter_load() -> bool`: the open editor has a switch wanted, asked or loading.
- `macros::sync_owner`: `held = (panel_visible() && !fighter_load()) || browser_visible()`,
  `loading = replay_capture::loading() || fighter_load()`. The editor stays on screen with its "Loading …" strip, Player 1
  stays neutral, and the roster's gates no longer wait for it. The pinned source test in `input_delay.rs` (`:348`)
  changes with it and gains a case for the load.

### 2. The outcome comes from the roster

The one ticket word `CLIP_REPLY` in `roster_requests.rs` (no new static) carries the ticket and a state:
`asked`, `admitted`, `refused`, `applied`, `dropped`, `withdrawn`, `committed`.

- **Admission** (game frame, as today): `request_player_kind` / `request_cpu_kind` returns true → `admitted`; false →
  `refused`. A request whose ticket is no longer the current one is not sent to the roster at all.
- **After admission**, each game frame, for the current ticket, from the roster's own state (a new reader
  `character_roster::clip_progress(player) -> (busy, observed)`; busy is what `notice_blocked()` reports, observed is
  the role's `(entry, kind)`):
  - not busy and the observed kind is the asked one → `applied`;
  - not busy and the role's entry and kind are exactly what they were at admission → `dropped` (the roster gave the
    request up);
  - not busy and another fighter is observed on a new entry → `dropped`;
  - otherwise still loading. No count is kept anywhere.
- **The editor** (worker) reads the state each pass. `applied` also needs the worker's own seen kind to be the asked
  one, so the strip matches the game. `refused` and `dropped` show the approved line "The roster refused the fighter
  switch" (`clip-fighter-refused`).
- `tick_fighter_load`, `fighter_load_passes` and "The fighter did not load; the editor keeps the current one" go.

### 3. B

- Wanted, not yet asked (the hold has not let go): the model cancels at once; the roster never heard of it.
- Asked or admitted: the worker posts a withdraw for the ticket. The game frame answers:
  - the request has not reached the roster yet → it is never sent → `withdrawn`;
  - it waits in the roster and no load has begun (`character_roster::withdraw_direct(player, kind)`: the role has no
    active load and its pending request is a user's Direct request for that kind) → removed → `withdrawn`;
  - anything else → `committed`: the editor keeps "Loading …", without "B cancels", and follows the roster to
    `applied` or `dropped`.
- The strip says nothing about keeping or cancelling while a load is under way.

## Files

| File | Carried | Change |
| --- | --- | --- |
| `src/model/macro_ui/clip_panel.rs` | pkg4 | the model's states; `note_fighter_outcome`; no pass counter |
| `src/runtime/macros/replay_capture.rs` | pkg4 | `fighter_load()`; `service_wanted_fighter` reads the outcome; B posts the withdraw |
| `src/runtime/macros/mod.rs` | no | `sync_owner` |
| `src/model/input_delay.rs` | no | its pinned test of `sync_owner` |
| `src/runtime/roster/roster_requests.rs` | no | ticket states, withdraw, outcome |
| `src/runtime/roster/character_roster.rs` | pkg6 | two small functions: `clip_progress`, `withdraw_direct` |
| `tests/support/clip_admission.rs`, `tests/roster_request_handoff.rs` | no | the red cases |

## Limits, stated

- The roster's own wait of 180 fighter frames (`request.age < 180`) and the older Switch path's 600-pass wait
  (`replay_capture.rs` `service_reload`) are older bounds outside this change; named to tr-primary-2, not touched.
- A request taken by a Mii owner cannot be withdrawn here; B then follows the roster.
- Host tests only. dup-audit flies the Eden boot after the fold.

## Requirement trace

The clauses are tr-primary-2's order of 2026-11-27 00:40:25Z.

| Order clause | Design | Test |
| --- | --- | --- |
| "the editor waits on the roster's own outcome: applied (the seen kind equals the requested one), or refused" | the ticket's states; no counter | `an_outcome_after_600_passes_still_ends_the_load` |
| "an outcome that arrives after more than 600 passes: the editor keeps "Loading…" and then shows the new fighter" | the same | `an_outcome_after_600_passes_still_ends_the_load` |
| "B cancels. A cancel withdraws the roster request if it hasn't committed" | withdraw before the roster hears it, or `withdraw_direct` | `b_before_commit_withdraws_the_request` |
| "If it has, the editor follows what the roster did" | `committed`, then `applied` or `dropped` | `b_after_commit_follows_the_roster` |
| "a refusal: the refusal message" | `refused` at admission, `dropped` after it | `a_refused_switch_shows_the_refusal`, `a_dropped_switch_shows_the_refusal` |
| "the editor's line always matches the game: no "kept" message while a load is still under way" | no line of the editor's own; "Loading …" until the roster's outcome | `the_strip_stays_loading_until_the_roster_answers` |
| "Cover the CPU (ZL+R) path and the way back too" | both roles, both directions, in the model and in the fixture | `the_cpu_toggle_and_the_way_back_wait_on_the_roster` |
| root cause: the visible editor held the roster's gate | `sync_owner`, `fighter_load()` | `the_editor_does_not_hold_the_rosters_gate_during_a_fighter_load` |
