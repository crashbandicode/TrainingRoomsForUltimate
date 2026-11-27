# Clip editor fighter switch and roster loading

The shipped editor waits for the roster’s outcome when switching fighters. B can cancel until the roster commits the change.

## Design history

| 2026-11-27 record | Detail |
| --- | --- |
| Defect report | dup-audit, clip 2, 00:38:37Z. |
| Scope and handover | tr-primary-2 at 00:40:25Z and 00:54:26Z. |
| First revision | docs-tutorials, `artifacts/phase23/docs-tutorials/toggle/DESIGN.md`. |
| Revision 2 | preview-vis-sol, reviewed by rv-hit-streak. |
| Worktree / branch | `/home/intpa/tr-fixes-20261113/toggle-wait`, `fix/toggle-load-wait`. |
| Base | `rc-next` at `cc71537cb7405690c468ecc9e42d776bca8176bd`. |
| Playbook | `c8d9959`. |
| Revision 2 decision | At 01:02:26Z, option 1 assigned drop reporting to the roster. |

## Why the old editor timed out

| Evidence at `cc71537c` | Consequence |
| --- | --- |
| Visible editor published `REPLAY_EDITOR` through `macros::sync_owner`. | `Owners::holds_gameplay()` kept the roster’s input gate closed. |
| `menu::captures_input()` remained true. | An admitted request waited up to 180 fighter frames, then failed (`character_roster.rs:1453–1473`). |
| `clip_panel.rs::tick_fighter_load` counted 600 worker passes. | The editor could stop waiting before the roster answered. |
| `cancel_fighter` cleared only editor fields. | The roster retained its request and could commit after editor close. |

The original diagnosis came from source inspection.

## Why revision 1 was superseded

| Mii-owner sequence | Why a quiet roster was ambiguous |
| --- | --- |
| Complete the load | Clear the track and release its lock. |
| Publish the observed pair afterwards | The roster may already be idle, and publication itself can return without writing. |
| Revision 1 inference | An unchanged observed pair plus an idle roster could falsely report a drop. |
| Revision 2 | The roster explicitly marks a drop. Success uses the game’s live fighter kind. |

The original trace cites `mii_ordinary_player.rs:605–610` and `character_roster.rs:1683–1698`.

## Input ownership and outcomes

| State or event | Shipped behavior |
| --- | --- |
| Editor visible, no fighter switch | Hold input. A visible browser also holds input. |
| Switch wanted, asked or loading | Publish `REPLAY_LOADING`. The panel remains visible and Player 1 remains neutral. |
| Loading owner | B reaches the editor. The editor releases the roster’s gameplay gate. |
| Ticket storage | The existing `CLIP_REPLY` word holds the role, target and state. |
| Asked | The worker posted a request. A superseded ticket never reaches the roster. |
| Admitted or refused | The game frame reports the roster’s admission decision. |
| Dropped | The roster marks a matching request when it gives up. |
| Applied | An admitted or committed request can settle when idle with the requested live kind. |
| Fighter observation | Both roles use the live fighter. A delayed or refused pair publication cannot hide success. |
| Unresolved | Continue displaying Loading. No editor counter or timeout decides the result. |

This removed `tick_fighter_load`, `fighter_load_passes` and the old timeout message, “The fighter did not load; the editor keeps the current one”.

## B and commitment

| When B arrives | Outcome |
| --- | --- |
| Wanted, before posting | Cancel locally. The roster has received nothing. |
| Posted, before admission | Mark the ticket withdrawn. The game frame skips the request. |
| Admitted, still queued | Post withdrawal. The game frame calls `character_roster::withdraw_direct`. |
| Owner has committed | Continue Loading without the B prompt until the roster applies or drops the request. |
| Mii owner has taken the request | The editor follows that committed load. |

## Ticket boundaries

| Boundary | Rule |
| --- | --- |
| Role departs | Invalidate its switch even before admission. |
| Mii track or bank fails | Only an admitted or committed switch can belong to that owner. |
| Mailbox overflows | Mark the exact evicted clip ticket. Ignore unrelated tickets and legacy messages. |
| Terminal mark and matching fighter coexist | Consume the mark before testing the observed fighter for success. |
| Mailbox drain precedes fighter callback | Arbitrate B at the irreversible handoff on the existing ticket, without another lock. |
| Handoff sites | Accepted Mii handoff, already-active direct selection, and generic direct reload about to start. |
| Native reload clears recording state | Only the committed role and target can identify the editor’s own install. |
| Editor’s own reload | Preserve its clip draft. Clear other transient UI. |
| Actual scene or entry departure | Clear normally. |
| Toggle waits for scrub release | Save the entry generation and recheck it at admission, even after delayed posting. |

## Evidence limits

| Scope | Recorded evidence |
| --- | --- |
| Roster wait | Its existing 180-fighter-frame bound was outside the editor change. |
| Older Switch path | `service_reload` retained its existing 600-pass wait. |
| Original revision | Host tests only. The handoff assigned subsequent Eden rows to dup-audit. |

## Requirement trace

| Requirement clause (00:40:25Z and 01:02:26Z decisions) | Test |
| --- | --- |
| Wait for the live fighter kind or an explicit refusal | `an_outcome_after_600_passes_still_ends_the_load` |
| Keep Loading visible beyond 600 worker passes | `an_outcome_after_600_passes_still_ends_the_load` |
| Allow B to withdraw before commitment | `b_before_commit_withdraws_the_request` |
| Follow the roster after commitment | `b_after_commit_follows_the_roster` |
| Show an admission refusal | `a_refused_switch_shows_the_refusal` |
| Keep the strip in sync with an unfinished load | `the_strip_stays_loading_until_the_roster_answers` |
| Cover the CPU chord (ZL+R) and switching back | `the_cpu_toggle_and_the_way_back_wait_on_the_roster` |
| Propagate direct-request drops for the correct fighter | `clip_switch_reports_the_rosters_answer_on_the_next_worker_pass` (the fixture's dropped and other-fighter checks) |
| Use the live kind across a Mii completion gap | `a_mii_owners_completion_gap_ends_on_the_new_fighter` |
| Use the game’s fighter even if publication fails | `a_refused_publish_still_shows_the_fighter_the_game_has` |
| Handle scene departure and entry destruction | `clip_switch_reports_the_rosters_answer_on_the_next_worker_pass` (the fixture's role-wide mark on a committed load) |
| Release the roster gate while the editor remains visible | `the_editor_does_not_hold_the_rosters_gate_during_a_fighter_load` |
