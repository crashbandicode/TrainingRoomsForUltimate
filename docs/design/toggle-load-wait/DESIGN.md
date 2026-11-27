# Toggle fix: the fighter toggle waits on the roster, not on a count (revision 2)

2026-11-27, lane preview-vis-sol (taken over from docs-tutorials, whose revision 1 is
`artifacts/phase23/docs-tutorials/toggle/DESIGN.md`). Worktree `/home/intpa/tr-fixes-20261113/toggle-wait`, branch
`fix/toggle-load-wait` from `rc-next` `cc71537cb7405690c468ecc9e42d776bca8176bd`. Playbook `c8d9959`, read completely.
Orders: tr-primary-2, inbox 2026-11-27 00:40:25Z (the defect), 00:54:26Z (the handover), 01:02:26Z (option 1).
Finding: dup-audit, inbox 00:38:37Z, clip 2. Reviewer: rv-hit-streak.

## Root cause (read in the code at `cc71537c`; nothing run in a game by this lane)

1. `clip_panel.rs` `tick_fighter_load` ends the editor's wait after 600 worker passes. Nobody ruled that bound.
2. `cancel_fighter` clears only the editor's fields. The roster keeps the request.
3. The cause under both: while the toggle waits, the editor is visible, so `macros::sync_owner` publishes the owner
   `REPLAY_EDITOR`, which `Owners::holds_gameplay()` counts. The roster keeps an admitted request while
   `menu::captures_input()` for up to 180 fighter frames and then rejects it (`character_roster.rs:1453-1473`). So an
   accepted request commits only once the editor closes.

## What changed from revision 1

Revision 1 inferred "dropped" from silence: the roster not busy and the role's observed pair unchanged. That is wrong
for a load a Mii owner runs: its completion clears its track, releases its lock and only then publishes the observed
pair, and that publication can return without writing (`mii_ordinary_player.rs:605-610`,
`character_roster.rs:1683-1698`). tr-primary-2 ruled option 1 at 01:02:26Z: the roster marks a drop itself and nothing
is inferred from silence.

## Design

### 1. The gate

`macros::sync_owner`: `held = (panel_visible() && !fighter_load()) || browser_visible()`,
`loading = replay_capture::loading() || fighter_load()`. While a switch is wanted, asked or loading, the editor
publishes `REPLAY_LOADING`: it stays on screen with its "Loading ..." line, Player 1 stays neutral, B still reaches it,
and the roster's gates no longer wait for it.

### 2. The outcome

The one ticket word `CLIP_REPLY` in `roster_requests.rs` (no new static) carries the ticket and its state: asked,
admitted, refused, dropped, withdrawn, committed.

- **Admission** (game frame): the roster's own answer, as today. A request whose ticket is no longer current is not
  sent.
- **Dropped**: every place that gives up a user's direct request calls `roster_requests::note_drop(player)`, which
  marks the current ticket of that role. The mark sites are named in the READY.
- **Applied**: the roster is not busy and the kind the game has for that role is the asked one. The game's kind, for
  both roles: the CPU's is already the live fighter's (`facts::fact_cpu_kind`); Player 1's becomes the live fighter's
  too, in place of the roster's observed pair, which lags in a Mii owner's completion and never changes when its
  publication is refused.
- Anything else stays "Loading ...". No count is kept anywhere. `tick_fighter_load`, `fighter_load_passes` and the
  line "The fighter did not load; the editor keeps the current one" go.

### 3. B

- Wanted, not yet asked: the model cancels at once; the roster never heard of it.
- Asked: the worker posts a withdraw. The game frame answers: not yet sent, so never sent (withdrawn); waiting in the
  roster with no load begun, so removed (`character_roster::withdraw_direct`, withdrawn); otherwise committed: the
  editor keeps "Loading ...", without "B cancels", and follows the roster to applied or dropped.

## Limits, stated

- The roster's own wait of 180 fighter frames and the older Switch path's 600-pass wait (`service_reload`) are older
  bounds outside this change; named to tr-primary-2 by docs-tutorials, not touched.
- A request a Mii owner took cannot be withdrawn here; B then follows the roster.
- Host tests only. dup-audit flies the Eden rows after the fold.

### Reviewer correction: the ticket at its boundaries

A role's departure invalidates even a switch not yet admitted. A failed Mii track or bank does not own an unadmitted
switch. Mailbox overflow reports the exact slot it evicted; a different ticket or a legacy message cannot refuse
the current switch. A terminal mark is consumed before a matching observed fighter can report success.

The game frame drains the mailbox before the native fighter callback. Sampling a pending request there would leave
one frame in which the strip could still promise cancellation after commitment. Instead, an accepted Mii handoff,
an already-active direct selection, or a generic direct reload about to start arbitrates against B on the existing
ticket. B wins before commitment; after it, the worker follows the roster without a B prompt. No lock is added.

## Requirement trace

The clauses are tr-primary-2's orders of 2026-11-27 00:40:25Z and 01:02:26Z.

| Order clause | Design | Test |
| --- | --- | --- |
| "the editor waits on the roster's own outcome: applied (the seen kind equals the requested one), or refused" | the ticket's states; no counter | `an_outcome_after_600_passes_still_ends_the_load` |
| "an outcome that arrives after more than 600 passes: the editor keeps "Loading…" and then shows the new fighter" | the same | `an_outcome_after_600_passes_still_ends_the_load` |
| "B cancels. A cancel withdraws the roster request if it hasn't committed" | withdraw before the roster hears it, or `withdraw_direct` | `b_before_commit_withdraws_the_request` |
| "If it has, the editor follows what the roster did" | committed, then applied or dropped | `b_after_commit_follows_the_roster` |
| "a refusal: the refusal message" | refused at admission | `a_refused_switch_shows_the_refusal` |
| "the editor's line always matches the game: no "kept" message while a load is still under way" | "Loading ..." until the roster's outcome | `the_strip_stays_loading_until_the_roster_answers` |
| "Cover the CPU (ZL+R) path and the way back too" | both roles, both directions | `the_cpu_toggle_and_the_way_back_wait_on_the_roster` |
| "Every place that gives up a user's direct request marks it, and the ticket reads that mark" | `note_drop` at each site | `clip_switch_reports_the_rosters_answer_on_the_next_worker_pass` (the fixture's dropped and other-fighter checks) |
| "the Mii-owner completion gap (not busy, pair not yet published): stays Loading and then shows the new fighter" | applied reads the game's kind | `a_mii_owners_completion_gap_ends_on_the_new_fighter` |
| "a refused publish" | the same | `a_refused_publish_still_shows_the_fighter_the_game_has` |
| "a scene change or entry destruction" | marked drops | `clip_switch_reports_the_rosters_answer_on_the_next_worker_pass` (the fixture's role-wide mark on a committed load) |
| root cause: the visible editor held the roster's gate | `sync_owner`, `fighter_load()` | `the_editor_does_not_hold_the_rosters_gate_during_a_fighter_load` |
