# Guard the Edge and Juggler player start

## Measured cause

On the installed card build `51d19bbd` (NRO `280f3a0d`), boot `fg2-g1-yoga-33` recorded 91 releases of Guard the Edge with Player 1's position and status on every frame (trace SHA-256 `4c8b475c29ed8e94fddb93d9c01114237a6f2f6c7d04b299818d1fc5c49131cb`, review passed 2026-11-22 05:47:51Z).

- The drill writes Player 1's start once per attempt. In all 91 attempts Player 1 was on the written point one update later; in 90 of them in Wait (attempt 1, written while idle, was on the point in another status).
- In 21 attempts the write found Player 1 in a knockback status (Damage, Damage air, Damage fly or Damage fly roll). In all 21 Player 1 then moved off the point along the floor, still in Wait: 18 on the first update after the write, 3 after 2, 8 and 11 updates. The largest step in one update was 3.01 units.
- In the other 70 attempts the write found Player 1 in another status, and it was on the point at the release.

The speed that outlives the write is not identified. The three late starts had not moved for the three updates before the write; that they were in hit freeze is an inference, the trace has no hit-freeze flag.

## Placement and bound

Before the start write, the drill reads Player 1. If it is in the game's hit freeze or not in Wait, the start is watched and the written point is kept for the attempt.

After a watched write, on every placement update until Ready, whether or not the CPU has reached Fall yet, the drill writes that same point again while Player 1 is in hit freeze, or is in Wait and not on its start. On its start means the written x and standing on the floor the write found: the first floor under Player 1's feet has the written height at that x. Another x is off the start whatever the floor read says. When the floor cannot be read, nothing is shown: a Player 1 that is not in hit freeze is not written, and Ready may start as before (tr-primary-2 ledger, 2026-11-22 07:20:07Z). Hit freeze is written again whatever the floor read says. Any other status is left alone.

Ready starts on the first update on which the CPU is in Fall and nothing was written. The rewrites of one attempt are counted against `Drill::ready_frames()` (60 in Guard the Edge, 45 in Juggler); after that many the drill goes on as before. Rule and bound were approved in the tr-primary-2 ledger at 2026-11-22 05:45:52Z.

A start write that finds Player 1 idle is not watched and follows the existing path on the same frame as before. Nothing holds Player 1 through Ready. No hook site and no native site is added; every native used is an existing granted row.

## Requirement trace

| Operator clause | Design | Test |
| --- | --- | --- |
| Player 1 starts where the wizard shows it, also when the previous attempt ended with a hit | A start written during a hit is watched and written again, on every placement update, until Player 1 stands idle on it | `hit_player_start_is_rewritten_until_it_holds`, `player_start_is_judged_on_every_update_before_ready`, `edge_juggle_settles_a_hit_player_before_ready` |
| Player 1's own movement before the release stays the player's; no hold through Ready | Only hit freeze, or Wait observed off the start, is rewritten; an unreadable floor shows nothing; the Ready hold still covers the CPU only | `hit_player_start_is_rewritten_until_it_holds`, `player_start_is_judged_on_every_update_before_ready`, `edge_juggle_settles_a_hit_player_before_ready` |
| The drill never stalls | At most one ready period of rewrites, from the existing constants | `hit_player_start_is_rewritten_until_it_holds`, `player_start_is_judged_on_every_update_before_ready` |
| A remembered CPU start the player never moved follows the current fighter's default | The wizard records an edit when a stick move changes the CPU point; choosing an area or resetting with X assigns a default and clears it. A plain Start recomputes the current fighter's default only for a start saved without an edit | `unmoved_remembered_start_follows_the_current_fighter`, `a_moved_start_stays_as_set_even_back_on_its_default`, `edge_juggle_remembers_whether_the_start_was_moved` |
| A start the player moved stays exactly as set | A recorded edit keeps the saved point on every later Start, also when the player moved it back onto the default | `a_moved_start_stays_as_set_even_back_on_its_default`, `edge_juggle_remembers_whether_the_start_was_moved` |
| The CPU start is readable in the wizard | Use the whole CPU mark and show its coordinates on the Area step without changing the sealed notice pages | `edge_wizard_marks_its_cpu_start_readably`, `edge_wizard_notice_keeps_the_pages_the_sealed_rows_read` |
| An off-picture CPU start has a pointer at the picture border | Project the start and draw a five-dot chevron at the canvas edge | `a_start_outside_the_picture_gets_a_pointer_at_the_border`, `edge_wizard_marks_its_cpu_start_readably` |
| The pointer shows only on the steps whose notice shows the CPU coordinates | `Wizard::shows_cpu` names Area, CPU and Preview, and the picture pass draws the pointer only then. Percent and Player keep their text and get no pointer | `the_pointer_steps_are_the_ones_that_show_the_cpu_coordinates`, `edge_wizard_points_only_where_the_notice_shows_the_cpu`, `edge_wizard_notice_keeps_the_pages_the_sealed_rows_read` |

## Cursor mark and remembered start

The CPU start uses the whole marker. The Area step shows its coordinates without the word CPU. The Percent and Player steps are unchanged because the sealed rows' readers (`guard_rows.py:37–64`) tell the pages apart by that word and by the word start. A five-dot chevron at the picture border points toward a start outside the picture, on the steps whose notice shows the CPU coordinates: Area, CPU and Preview (tr-primary-2 ledger, 2026-11-22 08:49:08Z). On Percent and Player the mark is drawn when it is in the picture, and nothing points to it when it is not.

The wizard records whether the player edited the CPU start. A stick move that changes the point is an edit, also when a later move brings the point back onto the default. Choosing an area and X assign a default and clear the edit. On a plain Start, a start saved without an edit follows the current fighter's default for its area, and a percent-dependent default follows the current percent. A start saved with an edit, and Juggler's saved start, stay exactly where they were. Set up… always opens on the current fighter's defaults.

The pointer's projection for off-screen points and its look remain unverified until the combined Eden proof.

## 2026-11-23: the CPU waits at its start (fix round, item F11)

The operator's order of 2026-11-23 replaces the border chevron of the two pointer rows above with the game's own off-screen magnifier.

The game shows that magnifier only for a fighter that is itself out of the camera's view: its per-fighter cursor update (`main+0x1556dc0`..`0x1557010` in 13.0.5) asks the fighter's camera module first, and only then `FLAG_LOUPE` or `FLAG_FORCE_LOUPE`. No picker moved a body, so a CPU start outside the picker's Fixed camera had nothing of the game's at it.

While the Guard the Edge or Juggler picker is up, the CPU now waits at its picked start: placed awake with a Fall request for `PREVIEW_SETTLE` frames, then held asleep there, with the placement the drill's own start uses (`model::crosshair::CpuPreview`, `placement::preview`). In view the game draws the CPU under the crosshair; out of view it draws its magnifier with the direction pointer. The chevron stays where the body cannot be held (constants or grants missing) and while the game HUD is off, which hides the magnifier. Leaving the picker without starting puts the CPU back where it stood, except while a save state loads. A started drill wakes it and the existing placement runs unchanged. `tr.practice` reports `wizard.cpu` as `in_view` or `magnifier`; the second is the game's `FLAG_IS_LOUPE` read back.

The recording start picker keeps its chevron: its spots are clamped to the camera's range. *Changed 2026-11-25: the recording start picker and Change placement are one wizard that holds both fighters at their spots; see "Change placement: the crosshair, on the save itself" in `docs/DECISIONS.md`.*

| Requirement | Design | Test |
| --- | --- | --- |
| An off-picture CPU start shows the game's magnifier of the CPU instead of the chevron | The picker holds the CPU at the picked point; `Marks::shown` leaves the chevron out while it does | `the_dotted_pointer_is_not_drawn_while_the_cpu_stands_at_its_point`, `the_drill_picker_holds_the_cpu_at_the_picked_point`, `the_picker_places_the_cpu_awake_then_holds_it_asleep` |
| In the picture the crosshair shows as before | `Marks::shown` changes only the pointer | `the_dotted_pointer_is_not_drawn_while_the_cpu_stands_at_its_point` |
| The direction stays shown | The game's magnifier carries its pointer; without a held body the chevron remains | `the_dotted_pointer_stays_when_the_cpu_cannot_be_held`, `a_start_outside_the_picture_gets_a_pointer_at_the_border` |
| Leaving the picker leaves the CPU where it was | `Engine::wake_cpu` ends the preview and writes the origin back | `leaving_the_picker_puts_the_cpu_back_where_it_stood`, `a_new_cpu_body_starts_its_own_preview_from_where_it_stood` |
| The bridge can tell which is shown | `crosshair::cpu_view` in `tr.practice` | `the_practice_state_says_how_the_cpu_point_is_shown`, `the_practice_snapshot_carries_the_cpu_view` |

Eden, 2026-11-23, XPS, card lineup `20261123-1532Z` with the bridge, build `5f9a9916` (run `f11-probe-1`, Final Destination, Player 1 Mario, CPU Zero Suit Samus): the Guard the Edge picker read `in_view` at (-160, -69), (-130, -30) and (-165, -30) and `magnifier` at (-200, -30), (-230, -30), (-230, 60) and (-150, -100), with the CPU on the picked point each time and back at its place after B. Every Juggler point was in view. Not run: a waiting room, another stage, a console.

## 2026-11-23: leaving the picker with the drill's own fighter (F11 follow-up)

A drill can have a CPU fighter of its own (**CPU fighter and mode**). The roster swaps it in for the picker and swaps the Training CPU back when the drill ends. It gives the incoming fighter the outgoing body's place, read when it takes the request.

B on the picker's first page ends the practice game in Player 1's controller poll. The practice CPU service asked for the original fighter at once, before the picker's put-back, which runs after Player 1's own frame. So the original came back at the picked start: off the stage for Guard the Edge, where it had to recover or was KO'd (early install 2 boot, Yoga, run `rc15-i2e-e1b5cf15-yoga-1`, row `f11-t1`: Zero Suit Samus came in at Donkey Kong's held point and was KO'd 164 frames after B). Before F11 no picker moved the body, so the roster handed the original place back.

The service now reads the slot through `picker::observable`. While nothing is selected and a left picker has not let go of the CPU (`edge_juggle::leaving_picker`: B's menu request is still to be taken, or the engine still holds the preview; a busy engine reads as held), the roster is not read and the original is not asked for.

B reopens the menu on Player 1's next poll, and the roster does not change the CPU under an open menu. So after B the drill's fighter stands at the CPU's place under the menu, and the Training CPU returns when the menu closes, as it does after Stop from the menu. The menu request is part of the rule because a request the roster finds already waiting under an open menu is dropped after 180 polls with a notice.

| Requirement | Design | Test |
| --- | --- | --- |
| After B the Training CPU is not brought back at the picker's held point | The original is not asked for until the picker has put the body back and let go | `the_original_is_not_asked_for_while_a_picker_holds_the_cpu`, `the_original_cpu_comes_back_after_the_picker_put_the_body_back` |
| A drill's own choice is still compared while its picker holds it | `picker::observable` asks about the hold only when nothing is selected | `a_selection_is_compared_whatever_the_picker_holds` |

Not changed: stopping a running drill that has its own fighter. The original takes over wherever the drill's fighter is, also at the off-stage start during Ready. Not run in Eden: this change (rows `f11-t4` and `f11-w4` in `eden/f11-magnifier/`).

## Limits and proof

- That the rewrite keeps Player 1 on its start is not proven. The measuring boot never rewrote. The proof is an Eden run on the built head with Player 1 read at every release.
- The three late starts are covered only if the game reports hit freeze for Player 1 while it stands in Wait on the written point. The query can be true in Wait, but its value in those three attempts was not recorded. If it is false there, Ready starts before the slide and the start is missed as before.
- A slide step that takes Player 1 past the ledge or into a teeter leaves Wait and is not rewritten. In `fcd2-guard-1` one update moved Player 1 6.4 units (attempt 62) and 7.9 units (attempt 38) to a Fall off the left ledge.
- A Player 1 that stands at the written x on another floor is missed when the floor under it cannot be read, as before the fix. Every measured displacement changed x; this case was never observed.
- The x comparison is exact. On a floor where the game moves a grounded fighter's x after the write, the drill would rewrite until the bound. Only Final Destination was measured.
