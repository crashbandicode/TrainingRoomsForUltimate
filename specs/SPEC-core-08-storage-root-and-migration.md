# Storage root and one-time migration — behavioural specification

- **Spec id:** `SPEC-core-08`
- **Origin:** project decision (operator, 2026-11-01), not upstream behaviour; no upstream reading was needed. The file names it imports are the fork's (ours).
- **Status:** draft (the operator answered the open questions of §10 on 2026-11-01 and the policy below is updated accordingly; see §10)
- **Spec writer:** project lead (Fable 5.1), 2026-11-01
- **Fork reference:** `training_mod_consts/src/files.rs` (`TRAINING_MODPACK_ROOT`, `SETUP_LIBRARY_DIR`), `training_mod_consts/src/menu_json.rs`, `src/replay_store.rs`, `docs/named-macros.md`, `docs/setup-library.md`, `docs/mii-fighters.md`, `docs/menu-shortcuts.md`, `docs/cycle-preload.md` in stage M5i.
- **Decides:** SPEC-core-01 Q2 (migration of the user's folder), SPEC-core-04 Q3 (file location), SPEC-input-01 open question 15 and SPEC-state-01 open question 9 (storage root).

## 1. Purpose (one paragraph)

Everything TrainingRoomsForUltimate writes lives under one new directory,
`sd:/ultimate/TrainingRooms/`. The first time the plugin boots on a card that
has the fork's `sd:/ultimate/TrainingModpack/` folder, it imports the player's
settings, save states, Mii presets and macro library once, by copying, and
never touches the originals, so the fork and the new plugin can coexist on the
same card and a downgrade loses nothing.

## 2. User-visible behaviour

- All files are under `sd:/ultimate/TrainingRooms/`. The plugin never writes
  anywhere else on the card (the Skyline log excepted).
- First boot on a card with `sd:/ultimate/TrainingModpack/`: the import runs
  before settings are loaded, so the player's old menu choices, saves, Miis and
  named recordings are already there when Training opens. One notification on
  the first Training entry: "Imported your Training Modpack data" (or
  "Training Modpack data: nothing to import"). No dialog, no question.
- The originals in `TrainingModpack/` are left byte for byte as they were.
- The install line carries `migration=none|done|partial|skipped` and, in the
  diagnostics file, one line per file (`[Migration] copied … bytes=… sha256=…`
  or `skipped reason=…`).
- The import runs once. `sd:/ultimate/TrainingRooms/migration.json` is the
  marker; while it exists nothing is imported again, even if the player deletes
  files under `TrainingRooms/`. Deleting the marker re-runs the import on the
  next boot (documented in the README). There is no menu row for it (S1 item 5,
  decided 2026-11-01).
- **Draft 9 additions (decided 2026-11-01).** The import also (a) copies the
  fork's `training_modpack_menu.conf` (its settings file, JSON content) under the
  new name `settings.json` (S1 item 1; corrected 2026-11-02);
  (b) copies `button_config.ini` and `replay_capture.ini` (S1 item 2);
  (c) copies `replay_captures/` **in full, with no size cap** (S1 item 3, N8),
  because it holds the companions the Replay Window crops macros from; it does
  this **without blocking the boot** (§5); (d) converts the dropped *Item on
  load* setting once (N6, §4 *Item-on-load conversion*). TOML-only cards are
  still not converted (S1 item 4).
- `developer.ini` (SPEC-core-01 §2) is new in the clean room: the fork has no
  such file, so the import neither reads nor creates it.

## 3. Inputs

- Existence and listing of `sd:/ultimate/TrainingModpack/` and the files and
  folders in §4.
- Existence of `sd:/ultimate/TrainingRooms/migration.json`.
- Existence of each destination file (a destination that already exists is
  never overwritten).

## 4. Outputs and native state touched

No game state is touched. File system only, at boot, on a plugin thread with an
explicit stack (SPEC-core-01 §threads).

Imported (copied; the operator's list, 2026-11-01):

| Source (`sd:/ultimate/TrainingModpack/`) | Destination (`sd:/ultimate/TrainingRooms/`) | What it holds | Rule |
| --- | --- | --- | --- |
| `training_modpack_menu.conf` | `settings.json` (S1 item 1, decided 2026-11-01; source name corrected 2026-11-02; the loader also accepts the old name once) | menu settings and saved defaults (fork JSON envelope, SPEC-core-04); retired keys `update_policy`, `menu_open_start_press`, `character_item` ignored on load | copy if destination absent |
| `button_config.ini` | `button_config.ini` (S1 item 2, decided 2026-11-01) | the menu shortcuts (SPEC-input-04 §7); the file of record that the menu now edits | copy if destination absent |
| `replay_capture.ini` | `replay_capture.ini` (S1 item 2) | the replay-capture on/off choice (`enabled=on/off`) | copy if destination absent |
| `replay_captures/` (every file) | `replay_captures/` (S1 item 3, N8; decided 2026-11-01) | the match captures (`.smd`) the Replay Window crops macros from, in full, no size cap | copy each file if absent; **background, non-blocking** (§5); the originals stay |
| `save_states.json` | `save_states.json` | the five slots per role, item snapshots, Mii identity, setup bindings (SPEC-state-01 §7) | copy if destination absent |
| `setups/` (every file) | `setups/` | the setup library's portable setup files (fork, carried over) | copy each file if absent; part of "save states" in the operator's list |
| `mii-presets.json` | `mii-presets.json` | replay-discovered Miis (fork, carried over) | copy if destination absent |
| `macros/` (every file) | `macros/` | the named macro library and exported record slots (fork, carried over) | copy each file if absent |

Not migrated (and why):

| Source | Why not |
| --- | --- |
| `training_modpack.json` | the fork's updater record (`{"update":{"last_update_version":…}}`, 72 B on the card), not settings; the clean room has no updater (SPEC-core-04 Q4). Corrected 2026-11-02: an earlier draft named this file as the settings source |
| `training_modpack_menu_defaults.conf`, `training_modpack_menu.json`, `*.toml` | legacy formats of upstream and of the fork before its JSON port; the fork's `tools/port_save_states.py` converted them at the C116 upgrade; a card that has only these never ran the JSON fork (open question 4). (`training_modpack_menu.conf` is not in this row: it is the settings file imported above.) || `cycle_preload.ini` | a cache of menu choices the fork rewrites at boot; the imported settings file is authoritative and the plugin regenerates it |
| `practice-diagnostics.log`, `practice-diagnostics.previous.log`, `version.txt` | fork diagnostics and version stamp |
| `dev.json`, `layout.arc` | development-only inputs of the fork (`dev_config`, `layout_arc_from_file`) |
| the legacy root `sd:/TrainingModpack/` | upstream moved off it years ago; the fork migrated it into `sd:/ultimate/TrainingModpack/` already; not read |

Rules for every copy:

1. Copy, never move, rename or delete anything under `TrainingModpack/`.
2. If the destination exists, keep it and record `skipped reason=exists`
   (a hand-copied folder or a second plugin version wins).
3. Copy through a fixed 64 KiB buffer; a file larger than 8 MiB is skipped
   with `reason=too_large`. **Exception (N8):** files under `replay_captures/`
   have no size cap. Each copy is written to `<name>.part` and renamed to its
   final name only when complete, so a copy interrupted by a power-off or by
   quitting the game never leaves a truncated file that the exists rule
   (rule 2) would keep; a leftover `.part` is overwritten on the next run.
4. Any read or write error skips that one file (`reason=io:<code>`) and the
   import continues; the result is `partial`.
5. Write `migration.json` last: `{ "source": "sd:/ultimate/TrainingModpack",
   "at": <tick>, "plugin": "<version>", "result": "done|partial|none",
   "files": [ { "from", "to", "bytes", "sha256", "result" } … ],
   "skipped": [ { "path", "reason" } … ] }`. If the marker cannot be written,
   the import is reported `skipped reason=marker` and will run again next boot
   (the per-file "destination exists" rule makes that harmless).
6. If `TrainingModpack/` does not exist: write the marker with `result: none`
   and no files; the notification says nothing was imported.

**Item-on-load conversion (N6, decided 2026-11-01; menu question 6).** The
*Item on load (older saves)* row and its setting `character_item` are dropped
(SPEC-core-04 §3.1; SPEC-state-04 §2). So that older saves keep their item,
the import reads `character_item` from the source settings file once, after
that file and `save_states.json` are copied, and, for each **player-role**
save-state record in the copied `save_states.json` that has no `held_item`
snapshot, writes one from the old setting: the setting's index resolves to an
item by the rule of SPEC-state-04 §5.2 steps 1–2 (Player 1st–8th: the
player record's own `fighter_kind`; CPU 1st–8th: the `fighter_kind` of the CPU
record in the same slot), and a record whose resolved fighter kind has no entry
for that variation, a never-saved record (`fighter_kind` -1), or a setting of
`None`, gets no snapshot (it stays absent, which
now means no item). Only the copy under `TrainingRooms/` is changed; the
fork's original file is untouched. The conversion runs once with the import
(the marker blocks a second run), logs one line per converted record, and
never overwrites an existing snapshot (`held_item` of any state, including
`empty` and `unsupported`).

## 5. Timing and frame rules

Boot, after the version guard passes and before the settings load
(SPEC-core-01 boot order), so the imported settings are the ones loaded. It
never runs on the game thread and nothing of it runs per frame. The import has
two parts (draft 9, N8):

1. **Blocking part (bounded).** The small files: `settings.json`, both INI
   files, `save_states.json` (with the item-on-load conversion), `setups/`,
   `mii-presets.json`, `macros/`, each at most 8 MiB. It finishes before the
   settings load, so the imported settings and saves are there when Training
   opens.
2. **Background part (non-blocking).** `replay_captures/` is copied on its own
   plugin thread with an explicit stack (64 KiB, SPEC-core-01 threads), started
   after the blocking part and never joined by the boot: the boot, the settings
   load and the first Training frame do not wait for it, and no frame hook
   waits for it. The Replay Window lists the capture files that are complete
   when it opens (a `.part` file is not listed). `migration.json` is written
   after the last file of both parts; if the game quits first, the next boot
   runs the import again and the exists rule (rule 2) skips what was copied.
   The install line says `migration=done|partial|skipped captures=running|done`
   (the diagnostics file carries the final `[Migration]` line when the
   background copy ends).

## 6. Edge cases and interactions

1. **Both plugins installed** (the fork's `libtraining_modpack.nro` and this
   plugin): each reads and writes only its own root; the import copies, so
   the fork keeps working. The card-level conflict of two plugins hooking the
   same sites is SPEC-core-01's concern, not this spec's.
2. **Downgrade to the fork:** the fork reads `TrainingModpack/`, untouched.
3. **Partial earlier import** (marker missing because the write failed):
   re-runs; every destination that exists is kept.
4. **Player edits `TrainingRooms/` files afterwards:** nothing is re-imported.
5. **Corrupt source file:** copied as is; the reader's corrupt-file policy
   (SPEC-core-04 Q2) applies when it is loaded; the import does not validate
   content, only that the copy's bytes match (sha256 recorded).
6. **Names inside imported JSON that refer to the old root** (none are known:
   the fork stores relative names in the setup and macro libraries; if any
   absolute `TrainingModpack/` path is found in a fixture, the importer leaves
   it and the loader resolves relative to its own root).
7. **Lobby practice, hot swap, Miis:** unaffected; the Mii presets file is the
   only Mii data on the card (saved Smash Miis live in the game's own save).

## 7. Persistence

`sd:/ultimate/TrainingRooms/migration.json` (marker and receipt). Everything
else is the owning spec's file, now under the new root.

## 8. Resource budget

| Metric | Fork today | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes | none (the fork has no importer) | small: one policy function plus the copy loop | `nro_size.py` with the feature off/on |
| per-frame work | — | 0 | diag counters |
| heap allocations in hooks | — | 0 (boot thread, fixed buffer, stack-built paths) | counting allocator, install line |
| boot time | — | copy time of the listed files (a few hundred KB typically); logged | `[Migration]` timestamps |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Plan = f(source listing, destination listing, marker): exact copy/skip list for the §4 tables | `migration::plan_copies_listed_files_only`, `migration::plan_skips_existing_and_excluded` | — | |
| Marker present → empty plan | `migration::marker_blocks_rerun` | CORE08-E03 | |
| Source absent → `none`, marker written | `migration::no_source_writes_none_marker` | CORE08-E04 | |
| Receipt JSON shape and sha256 per file | `migration::receipt_shape` (fixtures from `training_mod_consts/tests/fixtures/ported-json`) | — | |
| First boot on a profile with a populated `TrainingModpack/`: files copied, originals' hashes unchanged, settings loaded from the copy, install line `migration=done`, notification shown | — | CORE08-E01 | |
| Second boot: no copies, `migration=skipped reason=marker_present` not logged as an error | — | CORE08-E02 | |
| One unreadable source file → `partial`, the rest copied | `migration::io_error_is_partial` | CORE08-E05 (file made unreadable in the profile) | |
| Oversized file → `too_large` (not under `replay_captures/`) | `migration::too_large_skipped` | — | |
| **S1 items 1–2** The settings file is copied as `settings.json`; both INI files are copied; the loader accepts the old name once | `migration::settings_renamed_on_copy`, `migration::ini_files_copied`, `migration::loader_accepts_old_name_once` | CORE08-E06 | |
| **N6** Item-on-load conversion: a player-role record without `held_item` gets a snapshot resolved from `character_item` against its own `fighter_kind`; `None`, no entry for that kind, an existing snapshot (any state) and CPU-role records are left alone; only the copy changes; runs once | `migration::item_on_load_converts_player_records`, `migration::item_on_load_never_overwrites_snapshot`, `migration::item_on_load_none_and_no_entry_leave_absent`, `migration::item_on_load_runs_once` | CORE08-E07 profile with a fork save file that has no snapshot and `character_item` = Player 1st Var.: after the first boot the save restores the item; the original files' hashes are unchanged | |
| **N6** After the import the settings file has no `character_item`, the menu has no *Item on load* row, and the retired key is ignored on load | `migration::character_item_ignored_after_import` (with `core04_character_item_retired`) | CORE08-E07 | |
| **N8** `replay_captures/` is copied in full with no size cap (a file above 8 MiB is copied); originals stay; a `.part` file is renamed only when complete and a leftover `.part` is overwritten next run | `migration::captures_have_no_size_cap`, `migration::captures_part_then_rename`, `migration::captures_originals_untouched` | CORE08-E08 | |
| **N8** The capture copy does not block: the boot, the settings load and the first Training frame complete before it ends; the Replay Window lists only complete files; the marker is written after the last file | `migration::plan_splits_blocking_and_background`, `migration::marker_after_background_part` | CORE08-E09 profile with a large `replay_captures/` (several hundred MiB): Training opens and the menu works while the copy runs; the install line says `captures=running`, the diagnostics file later `done` | |

## 10. Open questions

1. **File name of the settings file** under the new root: keep the fork's
   `training_modpack_menu.conf` (corrected 2026-11-02; an earlier draft said
   `training_modpack.json`, which is the updater record) or rename it to `settings.json` at import
   (cleaner; one rename in the loader). Recommendation: rename; the loader
   also accepts the old name once. **Decided 2026-11-01 (S1 item 1): as recommended;** rename (the file is copied under the name `settings.json`).
2. **Import `button_config.ini` and `replay_capture.ini`** as well? They are
   user configuration the operator's list did not name. Recommendation: yes,
   same rules. **Decided 2026-11-01 (S1 item 2): as recommended;** both are imported.
3. **`replay_captures/`**: not imported as proposed. Alternative: import when
   the folder is under a size cap (for example 64 MiB) so the Replay Window
   keeps its history. Recommendation: not imported; document how to copy it by
   hand. **Decided 2026-11-01 (S1 item 3, N8): not as recommended;** `replay_captures/` is imported in full, with no size cap, because it holds the companions the Replay Window crops macros from; the copy does not block the boot (§5), and the originals stay.
4. **TOML-only cards** (the fork before its JSON port): the importer does not
   convert. Recommendation: document the fork's `tools/port_save_states.py`
   as the offline path; do not carry a TOML reader into the clean room. **Decided 2026-11-01 (S1 item 4): as recommended;** not converted.
5. **Re-run by deleting the marker**: acceptable as the documented way, or
   should the menu offer "Import Training Modpack data again"? Recommendation:
   the marker only; no menu row. **Decided 2026-11-01 (S1 item 5): as recommended;** the marker only, no menu row.
