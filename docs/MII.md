# Miis: what came over from the fork

The Mii code is the fork's own (M5q stage, `parked-reuse175/refactor/stages/m5q/stage`), ported directly. The pkg6
carry records (`docs/provenance/pkg6-carry-manifest.json`, `pkg6-carry-ledger.md`, checked by
`python3 -I tools/carry_pkg6.py verify`) name the fork file behind every carried file and every exact-text change.
This page is the parity checklist of 2026-11-05 (branch `p23/mii-editor`). "Fork" paths are relative to the M5q
stage; "Here" paths to this repository.

The native editor ran on the fork's builds. In this repository the rows below have host tests, and the rc.6 Eden rows
that ran them are listed under [Eden, rc.6](#eden-rc6).

## Checklist

| # | Behaviour | Fork | Here | Status |
| --- | --- | --- | --- | --- |
| 1 | **Add New** opens Smash's own Mii Fighter editor from Training, Quickplay or Local Wireless waiting practice | `src/mii_editor_bridge/mod.rs` `request`, `editor_origin.rs` `admit` | `src/runtime/mii/mii_editor_bridge/mod.rs:146` `request`, `editor_origin.rs:14`; `NATIVE_EDITOR_AVAILABLE = true` (`src/model/mii_root/mii_menu.rs:12`) | present |
| 2 | Add New starts from a copy of the role's Player choice, else the first saved Mii of the class, else the game's default | `training_mod_tui/src/mii_menu.rs:337-352`, `native_default.rs` | `mii_menu.rs:367` `editor_seed`, `native_default.rs` | present |
| 3 | Add New with a Player/CPU choice made in the same opening: the choice stays saved, its reload is dropped, the editor opens alone | `training_mod_tui/src/containers/app.rs:923-931`, `src/common/menu.rs:483-491` | `src/editors/mii.rs` `close_for`, `src/runtime/mii/menu_handoff.rs` (editor request before the catalog close) | **ported 2026-11-05** |
| 4 | **Save** makes a new native Smash Mii: only a fresh draft reaches the game's allocator (a new slot); a saved ID never enters, so no saved Mii or protected original is overwritten; an identical configuration reuses the saved one and says so: the notice "Same as ‹saved name›, nothing new saved" names the matched saved Mii (operator decision C, 2026-11-08; a rename alone is identical, row 12); refused when the catalog is incomplete or holds 99 files | `src/mii_editor_bridge/mod.rs:592-645` `edit_decide` | `mii_editor_bridge/mod.rs:634` `edit_decide`; post-save check `save_ready_for_return` `:723`, `catalog_read` `:747` | present |
| 5 | **Name**: the editor's Name row uses the shared inline keyboard (10 UTF-16 units; empty Enter cancels; one keyboard lease shared with the macro keyboard) | `src/mii_editor_bridge/name_keyboard.rs` | `mii_editor_bridge/name_keyboard.rs:153`, `src/model/mii/inline_keyboard.rs` | present |
| 6 | **Interrupted editor**: a match starting in Quickplay/Local Wireless while editing keeps the confirmed draft and the exact screen (Settings row, Name text and caret, Headgear/Outfit/Colour/Voice cursors, Specials picks, Test), in memory only; returning to the same lobby resumes it without saving | `connected.rs`, `resume_adapter.rs`, `ui_resume.rs`, `*_resume.rs` | the same files under `src/runtime/mii/mii_editor_bridge/` (`connected.rs:298/341` → `resume_adapter.rs:413` `try_auto_return` → `mod.rs:221` `resume_checkpoint`); `src/model/mii_root/resume.rs` | present |
| 7 | **Leaving**: B / Exit Without Saving / Cancel discards the draft and creates nothing; the copied Mii never changes; Training (or the lobby's practice fighter and Ready card) comes back | `mod.rs` `editor_parent`, `training_state`; `connected.rs` `owner.reenter` | `mod.rs:503` `editor_parent`, `:345` `training_state`; `connected.rs` | present |
| 8 | Local Wireless waiting practice greys Headgear, Outfit, Colour, Voice and Random in the editor | `settings_resume.rs:128-133` | `settings_resume.rs:129-134` | present |
| 9 | **Edit an existing Mii**: the fork has none (a saved Mii is only a template; the game's own saved-Mii page is cancelled) | `mii_menu.rs:337`, `editor_row.rs:1` | same | same (absent in both) |
| 10 | **Restore** in the fork means resuming an interrupted editor (row 6) and re-entering practice after Exit Without Saving (row 7); there is no "revert a Mii" | `resume_adapter.rs:281-311`, `connected.rs:239-246` | same | present |
| 11 | **Replay Miis**: with replay capture on, Save Replay stores the match's Mii data; entering Training or waiting practice imports it | `src/live_replay.rs:339-371`, `src/mii_replay.rs:146-245` | `src/runtime/macros/live_replay.rs:505` `copy_record`, `src/runtime/mii/mii_replay.rs` | present |
| 12 | Matching: two Miis are the same when class, look (CharInfo without its create ID and name), the four specials, colour, voice, outfit and hat match; names never count. Identical to a saved native Mii: that one is used and nothing is stored. Otherwise added once to the modpack library as "Replay Brawler/Swordfighter/Gunner"; the same capture again adds no copy | `training_mod_mii/src/inventory.rs:5-16`, `draft.rs:119-131,170-177` | `src/model/mii/inventory.rs`, `src/model/mii/draft.rs:119` `insert` | present |
| 13 | Replay Miis are modpack-only (`sd:/ultimate/TrainingRooms/mii-presets.json`, up to 256, never written to Smash's Mii list); a replay Mii is listed after the saved ones, and one equal to a saved Mii is hidden | `draft_store.rs:6-7`, `inventory.rs:298-309` | `src/model/mii/draft_store.rs:6-7`, `inventory.rs` | present (the folder is TrainingRooms') |
| 14 | Per-role (Player/CPU) choice per class page (Brawler, Swordfighter, Gunner); the choice applies on close (reload now if that class is on screen, else on a later cycle); the role's character-select Mii is greyed in its own list | `mii_menu.rs`, `selection.rs:186-252` | `src/model/mii_root/mii_menu.rs`, `src/editors/mii.rs`, `src/model/mii/selection.rs` | present (one list per class, ZR switches the role: the decided menu) |
| 15 | A portrait pick in Player/CPU Character drops that role's Mii reload from the same opening | `app.rs:522-531` | `src/editors/mii.rs` `close_for` (`picked`), `src/runtime/ui/worker.rs` `begin_close` | **ported 2026-11-05** |
| 16 | Stored Mii choices are read one by one: a bad entry or an unknown field leaves the other choices | `training_mod_consts/src/menu_json.rs:17-37` | `src/model/compat/menu.rs` `mii_preferences_of` | **ported 2026-11-05** |
| 17 | Mii tiles in Character Cycles, the portrait picks and Kirby Hats: a class with no admitted Mii for that role is greyed and refuses A ("… - unavailable"); admitted Mii tiles take their cycle number; the role's original Mii takes the grey 0 only without a numbered place. **P-CYCLE (2026-11-05):** the CPU cycle's numbers are its walkers' own list (`engine::swap::cycle_members`: All and the ranks, with the Mii classes the CPU Mii owner admits; All includes them, numbered), so a greyed class never cycles and every cycled class is numbered; the 0 is the runtime's slot 0 (the scene's original), not the saved pick | `app.rs:563-685,745-800`, `src/training/ui/roster_grid.rs:197-206,304-308` | `src/editors/roster.rs` `MiiTiles`, `roster_badges`; `src/editors/mii.rs` `tiles`; `src/model/swap/cycle_members.rs`; `src/runtime/ui/view.rs` (greyed tint) | **ported 2026-11-05**; fixed by P-CYCLE |
| 18 | Cycles load the chosen Mii; preloaded helpers and the body bank carry it | `training_mod_consts/src/mii_cycle.rs`, `mii_managed_helper.rs` | `src/model/swap/mii_cycle.rs`, `src/runtime/mii/mii_managed_helper.rs` | present |
| 19 | HUD portraits show the chosen Mii (Training, the Local Wireless CPU icon, the Quickplay CPU); the Ready card keeps the real-match fighter | `mii_hud_portrait.rs` | `src/runtime/mii/mii_hud_portrait.rs` | present |
| 20 | The exact Mii is kept across L+R+A, save-state loads, swaps, stage changes and a same-lobby return | `character_lifecycle.rs`, `mii_ordinary_*` | same files under `src/runtime/` | present |
| 21 | With no Mii catalog for an opening the fork showed no Mii pages | `app.rs:287-301` | the pages stay and say "Mii lists open in Training and in waiting practice." (`menu_app::no_catalog`) | differs (the decided menu keeps its rows) |

## Kept different on purpose, or not carried

- **Replay Window retry and exact-Mii named macros: ported 2026-11-05**, with the rest of the RC's frozen recording
  workflows (operator: "yes port enable the fork's Replay Window retry / exact-Mii named macros", then "Switch CPU to
  recorded fighter, Setup library, Edit placement"). The RC flag `ADVANCED_RECORDING_WORKFLOWS` and its gates are
  gone: the fork's code runs as it did.
  - **Replay window** (Recording › Replays › Replay window): choosing a capture's Mii track retries a missed import (the
    track's target is "not resolved" until the window resolves the recorded Mii,
    `engine::macro_ui::replay_target::clip_target`), then Switch loads that exact Mii on the CPU. An ordinary track's
    Switch loads its fighter. rc.6: the capture's import runs while the window waits for it (`Owners::REPLAY_LOADING`,
    `src/model/input_delay.rs`); the window's own Fighter Mismatch question is drawn (cov-rest `66a9a53`, `2891eaa`).
  - **Recorded macros:** since rc.5 every macro is a named file ([RECORDING.md](RECORDING.md)). A CPU macro recorded on
    an exact Mii asks Fighter Mismatch, and Switch loads that exact Mii (cov-rest `cbe961a`); one recorded on another
    ordinary fighter loads that fighter. Keep plays on the current CPU.
  - **A Mii without its own ID and name** (one from a replay keeps CharInfo bytes `0x00..0x26` zero): the CPU install
    writes the saved native Mii with the same look (row 12), or else the Mii with an ID made from its look and the name
    "New Mii". The game's own Mii setup binds no Mii resource for a CharInfo without them, and the body then read null
    memory (cov-rest `f2298ee`, `src/model/mii_install.rs` `installable`).
  - **Edit placement** and the **Setup library** (Saves › Setup library) are back. Edit placement edits the active
    macro of the role in view ([RECORDING.md](RECORDING.md), Replays).
- **Editor trace file** (`mii-editor-last.json`): diagnostic builds only; the fork wrote it in every build.
- **Menu input gate:** the editor also waits while the game's pause or the native waiting-room menu is open (the
  shared `captures_input`), not only for the plugin menu.
- **Capture commit:** a committed replay capture goes to the menu worker through a 16-slot mailbox (no heap in the
  game's hooks); the fork wrote it directly.
- **Not carried:** the fork's diagnostic Mii probes and observers (listed in the manifest's `not_carried`); the two
  Lua helper tests (`lua_helpers/tests/*.lua`; they need a Lua 5.3 interpreter, not a cargo test); and the fork test
  `training_mod_mii/tests/source_provenance.rs`, whose subjects are runtime files that a host test could only include
  from a test module, which the size counter would then count as test code.
- **Fork tests carried 2026-11-05:** `replay185_headgear` (`tests/mii_replay185_headgear.rs`), `ldn_backend` and
  `quickplay_backend` (lib test modules in `src/model/mii_fork_tests/`).

## Guards

Every game function the Mii code hooks or calls has an exact 64-byte row in `src/model/seams/native_seams/rows_a2.rs`
(group `mii_editor_bridge` and the Mii groups), checked against the owned 13.0.5 ELF by
`compared_rows_match_the_owned_elf`. The editor installs only when every row of its preflight passes
(`install::run`). The port added `mii_editor_bridge.called_376d990`, the Colour selector the resume path calls, which
had no guard in the fork.

## Mii groups and seats in rc.6

- **Async build:** a Mii group is constructed on the task worker like an ordinary group, not by the synchronous loop
  on the game thread (`async_build_eligible`, `src/model/swap/body_bank.rs:753`). The 2026-11-02 restriction stays for
  native groups (Pokémon Trainer, Aegis) and the Ice Climbers pair.
- **Warm exit:** leaving a Mii group for the window it displaced is a warm exchange; the Mii switch's publication is
  put back in the settle tick (`plan_mii_exit`, `src/runtime/roster/native_body_bank.rs:989`). TODO-PROOF: hot lane,
  the exit to any built group, and Mii groups built in the background (`p23/rc6-hot-slots-mii`).
- **No. of CPUs seats:** a seat member keeps its exact-Mii configuration, so a Mii seat loads through the Mii helper
  (`seat_load`, `src/model/parked_entries.rs:257`), and the seats park at one CPU whatever the plan lists
  (`parked_seat`, `:269`). A promoted seat parked again gets its Mii proof re-recorded only when its identity is
  unchanged.
- **Portrait first:** a Mii press waits while that role's HUD portrait renders on a helper, and a seat load holds back
  while a portrait waits for one (`Blocker::Portrait`, `src/model/swap/source_load.rs:150`;
  `seat_load_waits`, `src/model/parked_entries.rs:310`). The press runs after the render; it is not dropped.

## Eden, rc.6

Each run is a diag build of the named commit, 2026-11-08. Evidence is under `artifacts/phase23/rc6-eden/<run>/` and
`artifacts/phase23/rc6-item3-re/`.

| Rows above | Eden row | Run (build) | Result |
| --- | --- | --- | --- |
| 1, 2, 4, 7 | `cr-mii-exit`, `cr-mii-identical`, `cr-mii-changed`: no save on exit, the saved Mii reused, one new `mii_<id>.bin` | `covrest-16` (`1f6285c`) | pass |
| 4 | `cr-mii-rename-same`: a rename-only Save reuses the saved Mii and shows "Same as ‹name›, nothing new saved" | `covrest-07` (`f5a73c0`) | pass |
| 5 | `cr-mii-name-test`: the Name keyboard and Test | `covrest-07` (`f5a73c0`) | pass |
| 6 | `p00`–`p07` and the resume rows: a match starts while the editor is open in a Local Wireless room; the editor comes back with the typed name | `covrest-pair-y3` (`8e50b22`) | pass |
| 11, 12 | `cov-replay-window`: the capture's Mii track asks, and Switch loads the recorded Mii | `covrest-yoga-04` (`cbe961a`) | pass |
| Recorded macros | `cov-named-mii`: a CPU macro recorded on an exact Mii, Switch loads it, 0 Unmapped | `covrest-yoga-04` (`cbe961a`) | pass |
| 14, 17, 18 | `cr-cpu-cycle-list`, `cr-player-cycle-list`: the Mii classes load in both cycles | `covrest-16` (`1f6285c`) | pass |
| 18, seats | `cr-f9-raise-again`, `cr-f9-mii-cycles`: a Mii seat loads the saved Mii; the classes stay in both cycles after a raise and a drop | `covrest-09` (`f98813b`), `covrest-14` (`0c4b48b`) | pass |
| 20 | `cr-p2-lra-mii`: Player 2 controls a Mii CPU, after L+R+A too | `covrest-16` (`1f6285c`) | pass |
| Async build, warm exit | `me-p1-lap`, `me-cpu-lap`: every press applied, one Mii build per role, the exit 50–114 ms | `rc6-hot-xps6` (`d6a485f`) | pass |

The `covrest-yoga-04` and `covrest-pair-y3` builds carry cov-rest commits that reach the rc.6 candidate with the
cov-rest merge (TODO-PROOF: final pass).
