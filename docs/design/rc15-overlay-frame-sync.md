# Overlay frame sync

## Native order

The fighter pass runs in the stock phase-1 `BattleObjectManager::process_begin` task at `main+0x3ac660`. The stock HitModule updates its world shapes in phase 4 via `main+0x3ad3e0 → 0x3a8798 → 0x469dc0 → 0x468220`. The phase-8 `BattleObjectManager::process_end` uses the normal and alternate ModelModule paths at `main+0x3a8be0` and `main+0x3a8c50`. Both object walks complete before the native return. The hook on `main+0x3ade50` calls the stock task first.

## Sampling and placement

The phase-1 caller queues fighter identities. At the phase-8 return, one sample validates those identities, rebuilds fighter and article geometry, and snapshots the practice pictures. It uses the existing joint transform, wind area geometry, and angle calculation. The layout callback at `main+0x375efa8` uses the render camera to project that sample. A skipped publication cannot supply a previous frame's combat or target geometry. A held simulation preserves the last completed pose while the camera can still update.

The Egg and BTT camera feedback stays on the layout tick. Practice gameplay, the Perfect recovery height probe, input, and speed keep their original callbacks. The egg-zone wizard's immediate editor guides are authored world lines and use the camera when the editor draws. They do not sample a scaled fighter or article pose.

## Requirement trace

| Operator clause | Design | Test |
| --- | --- | --- |
| Correct the previous-frame drawing across overlays | Phase-8 snapshot for attacks, grabs, hurtboxes, wind and angles, article geometry, and practice discs | `overlay_board_exposes_only_the_completed_simulation_frame` |
| Preserve the defined shapes and their draw rules | Use the shared world placement and HUD projection | `a_capsules_two_ends_are_one_orb`, `an_articles_wind_area_is_drawn_and_keeps_it_tracked`, `a_live_targets_hurtbox_is_its_picture_disc` |
| Check the new native site and avoid blocking the game hook | Exact 64-byte guard and bounded acquisition | `core06_rows_match_the_owned_elf` |

## Exact runtime evidence

RED uses the existing release of `3448686c` with the stock Training step input. GREEN uses the gated release of this change. The artifact ledger records the NRO identities, native and plugin traces, stills, and card scan.
