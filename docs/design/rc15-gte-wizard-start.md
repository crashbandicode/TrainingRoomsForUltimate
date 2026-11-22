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

## Limits and proof

- That the rewrite keeps Player 1 on its start is not proven. The measuring boot never rewrote. The proof is an Eden run on the built head with Player 1 read at every release.
- The three late starts are covered only if the game reports hit freeze for Player 1 while it stands in Wait on the written point. The query can be true in Wait, but its value in those three attempts was not recorded. If it is false there, Ready starts before the slide and the start is missed as before.
- A slide step that takes Player 1 past the ledge or into a teeter leaves Wait and is not rewritten. In `fcd2-guard-1` one update moved Player 1 6.4 units (attempt 62) and 7.9 units (attempt 38) to a Fall off the left ledge.
- A Player 1 that stands at the written x on another floor is missed when the floor under it cannot be read, as before the fix. Every measured displacement changed x; this case was never observed.
- The x comparison is exact. On a floor where the game moves a grounded fighter's x after the write, the drill would rewrite until the bound. Only Final Destination was measured.
