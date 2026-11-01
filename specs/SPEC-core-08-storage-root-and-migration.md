# Storage root and one-time migration — behavioural specification

- **Spec id:** `SPEC-core-08`
- **Origin:** project decision (operator, 2026-11-01), not upstream behaviour; no upstream reading was needed. The file names it imports are the fork's (ours).
- **Status:** draft (policy proposed for the operator's confirmation; open questions in §10)
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
  next boot (documented in the README).

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
| `training_modpack.json` | `training_modpack.json` (open question 1 proposes `settings.json`) | menu settings and saved defaults (fork JSON envelope, SPEC-core-04); retired keys `update_policy`, `menu_open_start_press` ignored on load | copy if destination absent |
| `save_states.json` | `save_states.json` | the five slots per role, item snapshots, Mii identity, setup bindings (SPEC-state-01 §7) | copy if destination absent |
| `setups/` (every file) | `setups/` | the setup library's portable setup files (fork, carried over) | copy each file if absent; part of "save states" in the operator's list |
| `mii-presets.json` | `mii-presets.json` | replay-discovered Miis (fork, carried over) | copy if destination absent |
| `macros/` (every file) | `macros/` | the named macro library and exported record slots (fork, carried over) | copy each file if absent |

Not migrated (and why):

| Source | Why not |
| --- | --- |
| `training_modpack_menu.conf`, `training_modpack_menu_defaults.conf`, `training_modpack_menu.json`, `*.toml` | legacy formats of upstream and of the fork before its JSON port; the fork's `tools/port_save_states.py` converted them at the C116 upgrade; a card that has only these never ran the JSON fork (open question 4) |
| `button_config.ini`, `replay_capture.ini` | user configuration, but not in the operator's list; the plugin creates fresh defaults (Minus; capture on). Recommendation: add both to the import (open question 2) |
| `cycle_preload.ini` | a cache of menu choices the fork rewrites at boot; the imported `training_modpack.json` is authoritative and the plugin regenerates it |
| `replay_captures/` | match captures (`.smd`), large and re-capturable; the Replay Window starts empty (open question 3) |
| `practice-diagnostics.log`, `practice-diagnostics.previous.log`, `version.txt` | fork diagnostics and version stamp |
| `dev.json`, `layout.arc` | development-only inputs of the fork (`dev_config`, `layout_arc_from_file`) |
| the legacy root `sd:/TrainingModpack/` | upstream moved off it years ago; the fork migrated it into `sd:/ultimate/TrainingModpack/` already; not read |

Rules for every copy:

1. Copy, never move, rename or delete anything under `TrainingModpack/`.
2. If the destination exists, keep it and record `skipped reason=exists`
   (a hand-copied folder or a second plugin version wins).
3. Copy through a fixed 64 KiB buffer; a file larger than 8 MiB is skipped
   with `reason=too_large` (none of the imported kinds should be; captures,
   which can be, are not imported).
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

## 5. Timing and frame rules

Boot, after the version guard passes and before the settings load
(SPEC-core-01 boot order), so the imported settings are the ones loaded. The
import is bounded: at most the files listed above, each at most 8 MiB. It
never runs on the game thread and nothing of it runs per frame.

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
| Oversized file → `too_large` | `migration::too_large_skipped` | — | |

## 10. Open questions

1. **File name of the settings file** under the new root: keep the fork's
   `training_modpack.json` (zero change to the carried-over JSON code and the
   fork's `port_save_states.py`) or rename it to `settings.json` at import
   (cleaner; one rename in the loader). Recommendation: rename; the loader
   also accepts the old name once.
2. **Import `button_config.ini` and `replay_capture.ini`** as well? They are
   user configuration the operator's list did not name. Recommendation: yes,
   same rules.
3. **`replay_captures/`**: not imported as proposed. Alternative: import when
   the folder is under a size cap (for example 64 MiB) so the Replay Window
   keeps its history. Recommendation: not imported; document how to copy it by
   hand.
4. **TOML-only cards** (the fork before its JSON port): the importer does not
   convert. Recommendation: document the fork's `tools/port_save_states.py`
   as the offline path; do not carry a TOML reader into the clean room.
5. **Re-run by deleting the marker**: acceptable as the documented way, or
   should the menu offer "Import Training Modpack data again"? Recommendation:
   the marker only; no menu row.
