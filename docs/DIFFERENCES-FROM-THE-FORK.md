# Differences from the Training Modpack fork

TrainingRooms re-specifies the features of this project's earlier Training Modpack fork (stage M5q) and keeps its
behaviour unless an operator decision changed it ([DECISIONS.md](DECISIONS.md), [MENU-DRAFT.md](../MENU-DRAFT.md)).
This page lists only the changes, each checked against the code of 0.2.0-rc.2 (`p23/rc2-pause-helper` `913d06b`,
2026-11-05). Clips are Eden footage of the rc.1 release NRO; see [MEDIA.md](MEDIA.md).

## Install and files

| Change | Where in the code |
| --- | --- |
| The plugin is `libtraining_rooms.nro`; its data lives in `sd:/ultimate/TrainingRooms/`. It does not detect the fork's `libtraining_modpack.nro`: never install both. | `Cargo.toml:2`, `src/model/storage.rs:10` |
| First boot on a card with the fork's folder imports its settings (`training_modpack_menu.conf` → `settings.json`), `button_config.ini`, `replay_capture.ini` and `replay_captures/` by copying. `migration.json` marks it done; delete it to import again. | `src/model/migration.rs:42-62` |
| A settings file that does not parse is kept as `settings.invalid-N.json`, and each bad field falls back to its default; the other fields load. | `src/model/settings_file.rs:5-13` |
| A panic is written to the diagnostics file and the Skyline log; there is no pop-up. | `src/runtime/boot.rs:100-104` |
| If another mod replaces the Training HUD layout, the plugin still injects its own. | `src/runtime/ui/archive.rs:183-189` |

## Menu

![The five pages](media/menu-tour.gif)

| Change | Where in the code |
| --- | --- |
| Five pages (CPU, Saves, Recording, Fighters & Stages, Display & Controls), switched with L/R. A sixth Developer page appears only with `developer_page=on` in `developer.ini`; developer-only rows live there. | `src/model/menu/table.rs:10-22`, `src/model/developer_ini.rs:24` |
| The menu opens on the last page used (CPU the first time). | `src/model/menu/mod.rs:261-262` |
| Dropdown rows change in place with Left/Right; A opens the list. | `src/model/menu/mod.rs:813-817` |
| The seven delay rows offer a checklist and a min–max range; using one greys the other, Y clears. | `src/model/menu/table.rs:201-203` |
| Direction grids carry a weight per chip: each A adds one. | `src/model/menu/mod.rs:1165-1166` |
| Grids page more densely: 5×8 docked, 4×4 handheld. | `src/model/menu/grid.rs:73-74` |
| Fighter grids show the game's own stock icons, loaded at run time through the game's mod-aware loader, so modded icons show. | `src/runtime/roster/character_roster.rs:2168-2174` |
| The DI and SDI chips show the game's own DI line and SDI streak (the streak drawn ×2.0). | `src/model/effect_chips.rs:7-16` |
| Menu shortcuts 1 and 2 are edited in the menu with a chord editor, apply at once and are saved to `button_config.ini`. B + D-pad Up always opens the menu. | `src/model/menu/table.rs:394-399` |
| A *Replay capture* row writes `replay_capture.ini`; the Online Deluxe overlay's row shows the same value. | `src/model/menu/table.rs:332`, `src/runtime/deluxe_row.rs:31` |

![Fighter grid with cycle badges](media/cycle-badges.gif)

![A new menu shortcut applies at once](media/menu-shortcut.gif)

![Replay capture, menu and Deluxe overlay](media/replay-capture.gif)

## CPU and practice behaviour

| Change | Where in the code |
| --- | --- |
| Air dodge and shield tilt *In*/*Out* are relative to the opponent on both sides. | `src/model/cpu/defence.rs:64-71` |
| *Respawn invincibility* is a toggle; Off (default) is the fork's instant respawn. It is greyed while *Auto-load on death* is On. | `src/model/factory.rs:144`, `src/model/menu/mod.rs:586` |
| Changing ledge or throw options in the menu re-rolls them at once, even for a CPU already on the ledge. | `src/model/cpu/state.rs:178-183` |
| During input playback the recording wins over DI, SDI, air dodge, tech and throw. *Settings override playback* (default Off) restores the fork's rule. | `src/model/cpu/playback.rs:29-31` |
| Ice Climbers keep one CPU state each for Popo and Nana. | `src/model/cpu/role.rs:32-52` |
| When the CPU pilot plugin drives Player 1, CPU settings apply to the CPU only, and Record is refused with a notice. | `src/model/cpu/role.rs:3-6`, `src/runtime/features/recorder.rs:551-562` |

![Respawn invincibility Off: instant respawn](media/respawn-off.gif)

![Record refused while the CPU pilot drives P1](media/record-refused.gif)

## Added in rc.2

Operator decisions of 2026-11-05 (`origin/review/menu-proposal` `deccd62`); the fork keeps its own behaviour.

| Change | Where in the code |
| --- | --- |
| A pause that opens while a preloaded helper fighter is still loading no longer crashes. At pause-open, before the game's pause check walks the fighter entries, each helper entry the plugin created that has no actor yet is retired through the game's own single-slot retire; the helper is then created again, reusing its owner (PAUSE-HELPER, option a). | `src/runtime/roster/native_source_capacity.rs:793` (`pause_state_exit`), `src/runtime/roster/character_sources.rs:276` (`pause_opening`), `src/runtime/roster/native_source_capacity.rs:829` (`retire_at`) |
| The Solo Popo "S" marks sit on the icon's lower corners, just above the fighter's name, never over it (SOLO-S-PLACEMENT). | `src/model/ui/panes.rs:220` (`roster_tile`) |
| Every Kirby hat request ends: it ages each game frame and, if the hat never arrives, ends with "Kirby hat could not be loaded" (HAT-REQUEST-END, rule 1). | `src/runtime/roster/kirby_hats.rs:390` (`tick`) |
| A swap away from Kirby ends that role's pending hat request (HAT-REQUEST-END, rule 2). | `src/runtime/roster/kirby_hats.rs:445` (`fighter_committed`) |

![Solo Popo marks on the Ice Climbers icon, Player (red) and CPU (blue)](decisions/solo-s-placement-rc2.png)

*Character Cycles, Player list: the red (Player) and blue (CPU) "S" marks on the Ice Climbers icon's lower corners,
clear of the name. Eden run `tr-p23-surface-rc2d`, 2026-11-05, rc.2 diag NRO `719324b6` (the final rc.2 code).*

## Added in rc.3

Operator decision of 2026-11-05 (HB-OWN-ORBS, option 1: "go with discs for all", `origin/review/menu-proposal` `5cb1a2a`).

| Change | Where in the code |
| --- | --- |
| Hitbox Visualization draws every fighter sphere (hitboxes, hurtboxes, grab boxes) as the plugin's own discs instead of the game's shared effect pool, which refused part of a large fighter's spheres (two Ridleys: 22 of 55 on every frame). The layout has 512 disc panes: up to 128 spheres for each of Training's four possible fighter objects (two fighters, each possibly Ice Climbers with Nana). Only while a gone fighter's last spheres linger (a few frames) can there be more; then hurtboxes are cut before hitboxes and grab boxes, and the log says so once. The discs sit under the HUD and the menus like the old spheres, are flat (no depth), stop at each sphere's radius (the old glow went past it) and trail the camera by one to two frames while it pans. Projectiles keep the game's effect spheres. | `src/model/hitbox_orbs.rs:28` (`OrbSink`), `src/runtime/features/hitbox.rs:171` (`publish_orbs`), `src/runtime/features/hitbox.rs:201` (`draw_orbs`), `tools/layout-builder/src/main.rs:67` (`orbs_first`) |

## Kept from M5q

These fork limitations are unchanged in rc.1: adding CPUs keeps the wrong-CPU limitation; cold fighter switches can
hitch; recording after swaps, first-frame hitbox refusals on large fighters and partial writes on power loss remain.
