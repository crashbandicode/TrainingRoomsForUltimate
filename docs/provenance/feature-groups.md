| Feature | Disposition | Files | ours-new | mixed | upstream-identical | other | fork-only lines | retained upstream lines |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Diagnostics, probes and traces (diag feature; shipped NRO carries only the install line) | ours: carry over (diag feature only) | 34 | 34 | 0 | 0 | 0 | 14004 | 0 |
| Seam guards (byte-guarded native sites, version guard, authoritative rows) | ours: carry over | 20 | 20 | 0 | 0 | 0 | 8803 | 0 |
| Mii fighters (saved Miis, native editor bridge, HUD portraits, ordinary/CPU Mii, LDN qualification) | ours: carry over | 132 | 132 | 0 | 0 | 0 | 36481 | 0 |
| Hot-swap engine (windows, groups, swap bank, identity view, ledger, memory admission, coordinator) | ours: carry over | 188 | 188 | 0 | 0 | 0 | 40606 | 0 |
| Kirby hats (hat orders, hat cycle, copy packages) | ours: carry over | 6 | 6 | 0 | 0 | 0 | 836 | 0 |
| Cycle preload (fighter/hat preloading, capacity, admission grace, preload metrics, group swap config) | ours: carry over | 12 | 12 | 0 | 0 | 0 | 6049 | 0 |
| CPU-as-P1 handoff (cpu_pilot; defers to libssbu_cpu_pilot when present) | ours: carry over | 12 | 12 | 0 | 0 | 0 | 3089 | 0 |
| Practice stages, lobby stages and Training stage rotation | ours: carry over | 141 | 141 | 0 | 0 | 0 | 6607 | 0 |
| Lobby practice (waiting-room practice, waiting UI patch, automation helper) | ours: carry over | 47 | 47 | 0 | 0 | 0 | 9144 | 0 |
| Named macros, replay import, record slots, clip editor, live replay | ours: carry over | 67 | 67 | 0 | 0 | 0 | 16558 | 0 |
| Setup library and JSON saves (named setups, save_states.json, training_modpack.json, port tool) | ours: carry over | 30 | 30 | 0 | 0 | 0 | 5356 | 0 |
| Character items (menu icons from game assets, turnip face, item choice) | ours: carry over | 46 | 46 | 0 | 0 | 0 | 2785 | 0 |
| Menu shortcuts INI and fork menu additions | ours: carry over | 2 | 2 | 0 | 0 | 0 | 259 | 0 |
| Input display renderer optimisation (fork fix on an upstream feature) | ours: carry over as the required behaviour (spec SPEC-input-02) | 14 | 14 | 0 | 0 | 0 | 2755 | 0 |
| Fork release/size docs | ours: carry over (history) | 3 | 3 | 0 | 0 | 0 | 214 | 0 |
| Upstream artwork and layout (unlicensed assets) | upstream: replace (redraw) — never copy | 54 | 0 | 0 | 53 | 1 | 0 | 947519 |
| Upstream documentation | upstream: drop (write our own README) | 2 | 0 | 1 | 1 | 0 | 110 | 476 |
| Build configuration | mixed: new Cargo workspace per playbook (fork feature list is ours) | 8 | 2 | 4 | 2 | 0 | 1993 | 119 |
| Upstream: CPU behaviour (mash, followup, triggers, overrides, defensive, ledge, tech, shield, aerial/throw timing, frame advantage) | upstream: clean-room spec (SPEC-cpu-*) | 16 | 0 | 5 | 11 | 0 | 110 | 3039 |
| Upstream: save states, reset, charges, buffs, items, fighter-specific restore, stage hazards | upstream: clean-room spec (SPEC-state-*) | 11 | 0 | 11 | 0 | 0 | 720 | 4156 |
| Upstream: input recording/playback, input display, input delay, button chords, notifications | upstream: clean-room spec (SPEC-input-*) | 9 | 0 | 6 | 3 | 0 | 1112 | 1875 |
| Upstream: boot, offsets, logging, menu system, persistence, layout archive, hitbox visualizer, dev config | upstream: clean-room spec (SPEC-core-*) | 35 | 0 | 25 | 10 | 0 | 4431 | 17428 |

## Mixed files by feature (fork-only ranges are stage line numbers)

### Upstream documentation

| File | stage lines | retained upstream | fork-only | fork-only ranges |
| --- | ---: | ---: | ---: | --- |
| `README.md` | 584 | 474 | 110 | 1-108, 150, 278 |

### Build configuration

| File | stage lines | retained upstream | fork-only | fork-only ranges |
| --- | ---: | ---: | ---: | --- |
| `Cargo.toml` | 240 | 60 | 180 | 3, 14-16, 37, 42-53, 68-72, 82-122, 124-240 |
| `rust-toolchain.toml` | 4 | 1 | 3 | 2-4 |
| `training_mod_consts/Cargo.toml` | 46 | 23 | 23 | 7, 21-28, 31-40, 43-46 |
| `training_mod_tui/Cargo.toml` | 25 | 18 | 7 | 10, 15, 19-23 |

### Upstream: CPU behaviour (mash, followup, triggers, overrides, defensive, ledge, tech, shield, aerial/throw timing, frame advantage)

| File | stage lines | retained upstream | fork-only | fork-only ranges |
| --- | ---: | ---: | ---: | --- |
| `src/training/combo.rs` | 169 | 145 | 24 | 21-26, 88, 91-107 |
| `src/training/ledge.rs` | 221 | 210 | 11 | 49-50, 106-108, 187-192 |
| `src/training/mash.rs` | 656 | 639 | 17 | 66, 107, 197-211 |
| `src/training/shield.rs` | 457 | 439 | 18 | 34-43, 168-170, 173, 190-193 |
| `src/training/tech.rs` | 777 | 737 | 40 | 12, 19-25, 355-372, 384-389, 415, 437, 753, 771-775 |

### Upstream: save states, reset, charges, buffs, items, fighter-specific restore, stage hazards

| File | stage lines | retained upstream | fork-only | fork-only ranges |
| --- | ---: | ---: | ---: | --- |
| `src/hazard_manager/mod.rs` | 162 | 134 | 28 | 6-8, 35-43, 97-100, 102-104, 115-116, 149-155 |
| `src/training/buff.rs` | 449 | 429 | 20 | 9, 19-26, 30-35, 37, 45, 53, 65, 73 |
| `src/training/character_specific/items.rs` | 555 | 515 | 40 | 480-488, 492-504, 536-553 |
| `src/training/character_specific/kirby.rs` | 608 | 600 | 8 | 2, 5, 17, 19, 22, 37, 606-607 |
| `src/training/character_specific/mod.rs` | 84 | 74 | 10 | 67-76 |
| `src/training/character_specific/pikmin.rs` | 216 | 210 | 6 | 2-3, 34, 36, 51-52 |
| `src/training/character_specific/ptrainer.rs` | 175 | 153 | 22 | 1, 5, 143, 145, 153-168, 173-174 |
| `src/training/character_specific/steve.rs` | 187 | 184 | 3 | 4-6 |
| `src/training/charge.rs` | 1114 | 1087 | 27 | 2, 10, 19-23, 26-29, 432-439, 441-444, 454-456, 459 |
| `src/training/reset.rs` | 54 | 31 | 23 | 12-34 |
| `src/training/save_states.rs` | 1272 | 739 | 533 | 2, 25, 95-102, 116-118, 171-172, 175-179, 183-504, 508-510, 515-516, 525-529, 538-541, 547-550, … (+30 more) |

### Upstream: input recording/playback, input display, input delay, button chords, notifications

| File | stage lines | retained upstream | fork-only | fork-only ranges |
| --- | ---: | ---: | ---: | --- |
| `src/common/button_config.rs` | 484 | 236 | 248 | 3, 12-71, 226-227, 230, 234, 240-241, 253-254, 257-287, 292-296, 300-304, 307, 309-346, … (+7 more) |
| `src/training/input_delay.rs` | 27 | 23 | 4 | 10-13 |
| `src/training/input_log.rs` | 384 | 374 | 10 | 177, 187, 211, 312-318 |
| `src/training/input_record.rs` | 1040 | 532 | 508 | 12-14, 74-88, 110, 189-199, 223-255, 263-264, 267-301, 303-317, 320, 322-330, 337-338, 358, … (+35 more) |
| `src/training/ui/display.rs` | 71 | 70 | 1 | 28 |
| `src/training/ui/input_log.rs` | 432 | 91 | 341 | 2, 17-82, 85-217, 265-285, 288-294, 302, 304-305, 309-363, 365-369, 375-385, 390-412, 415-429, … (+1 more) |

### Upstream: boot, offsets, logging, menu system, persistence, layout archive, hitbox visualizer, dev config

| File | stage lines | retained upstream | fork-only | fork-only ranges |
| --- | ---: | ---: | ---: | --- |
| `src/common/dev_config.rs` | 50 | 44 | 6 | 4, 11, 37, 40-41, 48 |
| `src/common/filesystem.rs` | 105 | 41 | 64 | 12-37, 39-40, 55-67, 70-91, 102 |
| `src/common/menu.rs` | 516 | 163 | 353 | 11, 15, 18-21, 23-33, 35-60, 71-79, 82-83, 95-97, 100-101, 107-115, 118-127, 130-134, … (+18 more) |
| `src/common/mod.rs` | 438 | 369 | 69 | 8, 16, 28, 31-35, 38, 45-54, 57-61, 69-72, 75-79, 86-91, 100-106, 108-111, … (+6 more) |
| `src/common/offsets.rs` | 455 | 328 | 127 | 5, 42-90, 94, 96-101, 124-125, 143-144, 157-158, 171-172, 182-183, 196-197, 206-207, 209-234, … (+15 more) |
| `src/hitbox_visualizer/mod.rs` | 368 | 350 | 18 | 224-225, 272, 328, 354-367 |
| `src/lib.rs` | 392 | 122 | 270 | 3-20, 24, 27, 37-40, 43-46, 48-113, 115-147, 150, 154-159, 162-174, 176-209, 214-269, … (+10 more) |
| `src/logging.rs` | 347 | 50 | 297 | 1-16, 20-284, 329-344 |
| `src/training/debug.rs` | 197 | 177 | 20 | 4, 8-15, 105-115 |
| `src/training/mod.rs` | 1378 | 823 | 555 | 7-8, 33, 47-54, 58, 60-112, 116, 167-179, 194-228, 230-235, 237-239, 244-246, 248-283, … (+54 more) |
| `src/training/ui/menu.rs` | 955 | 583 | 372 | 6, 66-142, 163-240, 270-278, 324-327, 329-347, 350, 356, 367, 384-398, 401-407, 414, … (+19 more) |
| `src/training/ui/mod.rs` | 392 | 166 | 226 | 7-8, 11, 14-16, 18, 21-23, 25, 27-102, 138-152, 154-182, 185-186, 195, 199-201, … (+13 more) |
| `training_mod_consts/src/config.rs` | 171 | 142 | 29 | 6-7, 16-17, 20-25, 35, 37-39, 43-47, 63, 65, 79, 82, 88, 96, … (+3 more) |
| `training_mod_consts/src/files.rs` | 18 | 12 | 6 | 5-10 |
| `training_mod_consts/src/lib.rs` | 1351 | 642 | 709 | 7-35, 40, 58-59, 120-122, 132-293, 299, 301, 303-307, 323, 384, 409-443, 445-457, … (+82 more) |
| `training_mod_consts/src/options.rs` | 1182 | 1102 | 80 | 42-45, 62-65, 74-76, 84-87, 227-228, 230-234, 313-353, 429-430, 432-436, 446, 1044-1052 |
| `training_mod_tui/src/containers/app.rs` | 1420 | 370 | 1050 | 5-6, 16, 52-67, 70-88, 98-109, 112-118, 120-364, 373-376, 404-406, 410-416, 418-427, 435-439, … (+33 more) |
| `training_mod_tui/src/containers/submenu.rs` | 212 | 159 | 53 | 22, 42, 53, 64-67, 75, 88, 101, 116, 135-169, 176-181, 210 |
| `training_mod_tui/src/containers/tab.rs` | 58 | 54 | 4 | 16-19 |
| `training_mod_tui/src/lib.rs` | 101 | 7 | 94 | 3-10, 16-101 |
| `training_mod_tui/src/structures/stateful_list.rs` | 130 | 124 | 6 | 15-20 |
| `training_mod_tui/src/structures/stateful_slider.rs` | 152 | 146 | 6 | 21-26 |
| `training_mod_tui/src/structures/stateful_table.rs` | 275 | 268 | 7 | 30-32, 39-40, 53-54 |
| `training_mod_tui/tests/test_stateful_slider.rs` | 288 | 282 | 6 | 5, 264, 267, 270, 273, 276 |
| `training_mod_tui/tests/test_submenu.rs` | 499 | 495 | 4 | 38, 42, 46, 54 |

