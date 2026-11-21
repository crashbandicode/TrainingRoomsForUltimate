# Overlay frame sync

## Native order

The fighter pass runs in the stock phase-1 `BattleObjectManager::process_begin` task at `main+0x3ac660`. The stock HitModule updates its world shapes in phase 4 via `main+0x3ad3e0 → 0x3a8798 → 0x469dc0 → 0x468220`. The phase-8 `BattleObjectManager::process_end` uses the normal and alternate ModelModule paths at `main+0x3a8be0` and `main+0x3a8c50`. Both object walks complete before the native return. The hook on `main+0x3ade50` calls the stock task first.

## Sampling and placement

The phase-1 caller queues fighter identities. At the phase-8 return, one sample validates those identities, rebuilds fighter and article geometry, and snapshots the practice pictures. It uses the existing joint transform, wind area geometry, and angle calculation. The layout callback at `main+0x375efa8` uses the render camera to project that sample. A skipped publication cannot supply a previous frame's combat or target geometry. A held simulation preserves the last completed pose while the camera can still update.

The layout tick reads the live input-driven editor state and passes it through the same projection and pane writer. BTT placement shows cursor movement and edited targets before the next simulation step. After X confirms and A starts the run, the committed layout remains visible until phase 8 supplies the first active-target sample. Holding the board during the HUD read makes that handoff use one snapshot. The egg-zone wizard, Tech placement and Guard the Edge placement likewise read live editor state. BTT targets in play and moving Eggs-ercise discs come from the phase-8 sample. Egg and BTT camera feedback, practice gameplay, the Perfect recovery height probe, input and speed keep their original callbacks.

The same phase-8 publication carries fighter anchors independently of the legend setting. The compact strip's `fighter_screen_anchors(layout_name)` reader yields a fighter ID, its sampled top joint and that point projected into the current HUD canvas. It requires a matching active scene, a current snapshot and a valid render projection.

## Overlay store ownership

Both mutexes are private to `src/runtime/features/hitbox.rs`. The complete set of their callers is below. The coordinates are from the stock 13.0.5 image.

| Path | Source | Acquisition and lifetime |
| --- | --- | --- |
| Fighter initialization | `src/runtime/features/reset_hub.rs:34–40`, `hitbox.rs:1147–1162` | Marks the exact accessor's cached set for retirement atomically. Still tries to clear its board pictures and anchor immediately. |
| ATTACK and CATCH animation commands | `hitbox.rs:1527–1575` | Take the inherited `STORE` or `ARTICLES` lock. Decode the command and read status first. Release the guard before calling the original. |
| Four wind-area animation commands | `hitbox.rs:1583–1627` | Take the inherited `ARTICLES` lock to track the article. Release the guard before calling the original. |
| Rebound reset | `hitbox.rs:1633–1654` | Call the original and read status first. Take the inherited `STORE` lock to clear cached spheres. The board clear uses try-lock. |
| Phase-8 overlay sampling | `hitbox.rs:1376–1450,1656–1662` | Run the original task first, then try `BOARD`, `STORE` and `ARTICLES`. |

The reset hub patches Fighter vtable slot `+0x30` at `main+0x4f80240` and runs the original `main+0x60dfa0` before retiring the cache. Stock virtual dispatch `main+0x656f88..0x656f94` reaches that slot from the task at `main+0x354a7dc` via `main+0x135dd70`, `main+0x14de860` and `main+0x656134`. Historical stock traces recorded 192 init calls, including 46 off main. In the Yoga Mii reset receipt, sequence 4099 reaches the exact slot on guest thread 113. The original init uses the same non-null accessor at `main+0x60f7c4..0x60f7c8`. This rules out keeping init's inherited Store acquisition under a single-thread assumption.

A reset publishes one atomic retirement bit against the accessor registered for each cache slot. A repeated reset with no intervening cache access is idempotent. A slot eviction forgets the old records and binds the replacement key. A reset for the old accessor cannot mark a replacement with a different key. The consumer checks the marks under its existing Store guard before sampling, queuing a grab or clearing rebound. It discards the retired records without issuing a native command. The part of init visible to HUD still uses the existing independent board try-lock.

The layout worker at `main+0x375efa8` calls `draw_orbs` and `fighter_screen_anchors`; they only read `BOARD` for combat geometry. No UI callback or plugin-owned worker reaches `STORE` or `ARTICLES`. The early Perfect recovery measurement uses invocation-local scratch and does not acquire either lock. Diagnostic signature and census reads borrow the Store guard phase 8 already acquired. The lock regression follows the real init, phase-8 and early Perfect roots and checks that it detects an injected blocking acquisition on each.

Stock `main+0x135ca00` invokes the registered phase callbacks synchronously through `main+0x137b4d0` or `main+0x137b5a0`. Exact RED observes phase 1, HitModule phase 4 and phase 8 on guest thread 80, with layout on guest thread 99. Historical records also put the observed ATTACK calls and two wind variants on thread 80. The stock task roots `main+0x353d514` and `main+0x37ccad8` can dispatch both wind scripts and worker init. They do not identify a unique thread. CATCH, rebound and the wind exports also have independent indirect entry from stock fighter modules. Their full scheduling ownership remains unproven. The 20:40Z thread-only acceptance condition is therefore still open for the remaining inherited locks.

## Requirement trace

| Operator clause | Design | Test |
| --- | --- | --- |
| Correct the previous-frame drawing across overlays | Phase-8 snapshot for attacks, grabs, hurtboxes, wind and angles, article geometry, and practice discs | `overlay_board_exposes_only_the_completed_simulation_frame` |
| Preserve the defined shapes and their draw rules | Use the shared world placement and HUD projection | `a_capsules_two_ends_are_one_orb`, `an_articles_wind_area_is_drawn_and_keeps_it_tracked`, `a_live_targets_hurtbox_is_its_picture_disc` |
| Keep editor input visible while the simulation is held | The same layout writer draws live cursor and target placement, the committed layout until the first running sample, and the active targets afterward | `paused_editor_polls_update_pictures_without_advancing_the_world_sample` |
| Check the new native site and avoid blocking the game hook | Exact 64-byte guard and bounded acquisition | `core06_rows_match_the_owned_elf`, `overlay_sampling_and_perfect_height_do_not_wait_for_overlay_data` |
| Keep the worker init path off the cache lock | Retire the exact accessor through the registered slot | `fighter_init_on_a_worker_retires_only_its_cached_set`, `overlay_sampling_and_perfect_height_do_not_wait_for_overlay_data` |

## Exact runtime evidence

RED uses the existing release of `3448686c` with the stock Training step input. GREEN is part of the integrated candidate’s combined Eden session. The artifact ledger records the NRO identities, native and plugin traces, stills, and card scan.
