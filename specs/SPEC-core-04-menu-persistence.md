# Menu persistence (settings file, defaults, load order, migration) — behavioural specification

- **Spec id:** `SPEC-core-04`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), spec-writer role, 2026-11-01
- **Upstream reference read (spec writer only):** `src/common/menu.rs` (load/commit), `src/common/filesystem.rs` (writer thread, JSON/TOML helpers), `training_mod_consts/src/files.rs` (paths), `training_mod_consts/src/config.rs` (updater config file), `training_mod_consts/src/lib.rs` (`MenuJsonStruct`, factory defaults), `training_mod_tui/src/containers/app.rs` (serialization shape), `src/lib.rs` (legacy folder migration, loader thread), `src/common/dev_config.rs` at commit `97cbe9b9` (unchanged in `077a8e43`); fork changes at stage M5i `src/common/menu.rs` `66-116, 118-125, 127-155`, `src/common/filesystem.rs` `12-37, 39-40, 55-67, 70-91, 102`, `training_mod_consts/src/files.rs` `5-10`, `training_mod_consts/src/config.rs` `6-7, 16-17, 20-25, 35, 37-39, 43-47, 63, 65, 79, 82, 88, 96, …`, `training_mod_consts/src/lib.rs` `296-308` (envelope), plus the fork-only `training_mod_consts/src/menu_json.rs`, from `docs/provenance/provenance-files.md`.
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

The player's menu choices and their saved defaults survive a restart: they are written to one JSON file on the SD card every time the menu closes and read back once at boot before the first Training frame. A missing file yields the factory settings; an unreadable file is reported and the factory settings are used; unknown keys are ignored so files from older or newer builds load. The fork moved every remaining config file of the plugin to JSON (TOML 0.5 could not emit values after tables), and this spec states the JSON contract that the clean room carries over.

## 2. User-visible behaviour

- Settings persist across game restarts; defaults saved with X persist too.
- No menu item. Deleting the settings file is the documented factory reset (upstream FAQ 24).
- Failures are silent to the player except through the diagnostics/Skyline log (upstream `warn`/`info` lines). Upstream deleted an unparsable file and recreated it at the next close; see §6 and Q2.
- Upstream also showed an error applet (code `0x70`, "Could not parse the menu response!") if the JSON produced by the menu itself failed to parse at close — a programming error path, never seen by players when the model is correct. The clean room logs instead (SPEC-core-01 Q1).

## 3. Inputs

- The commit payload from the menu at close (SPEC-core-02 §4): `{"menu": <settings>, "defaults_menu": <defaults>}` where each snapshot is a JSON object mapping a setting id to an array of small integers.
- The factory table (SPEC-core-02 §2.7) as the fallback for every setting.
- The file contents at boot.

### 3.1 Value encoding (the contract)

- Every setting is a JSON **array of unsigned 8-bit integers**, one element per option **in the option set's declaration order** (SPEC-core-02 §2.7), value `0` = not selected, `1` = selected, `2..5` = weight (weighted rows only).
  - `OnOff` is `[on, off]`: On = `[1,0]`, Off = `[0,1]`.
  - `BoolFlag` is `[true, false]`.
  - Button chords (`ButtonConfig`) are 16 elements in the order A, B, X, Y, L, R, ZL, ZR, DPad Up, DPad Down, DPad Left, DPad Right, Plus, Minus, LStick, RStick; the default save chord is ZL + DPad Down = `[0,0,0,0,0,0,1,0,0,1,0,0,0,0,0,0]`.
  - Sliders are two elements `[lower, upper]` (`DamagePercent`: default `[0, 150]`).
- The settings object contains one entry per setting id; ids are the stable keys of SPEC-core-02 §2.7 (`mash_state`, `hud`, …). Upstream wrote the two snapshots as flat objects (the struct's field names); the order of keys follows the struct, which the reader must not depend on.
- Upstream's encoding came from its `byteflags` crate (one byte per flag, serialized as a sequence). The clean room defines the same wire shape with its own types; a fixture file in the test tree pins it (Q1 on an exact byte-for-byte sample).

### 3.2 Reader rules (upstream `97cbe9b9`, carried by the fork)

- Unknown keys are ignored (the fork relies on this: a file that still has `update_policy` or `menu_open_start_press` loads).
- A **missing upstream key** is an error (serde's derived struct deserializer requires every field without a default); upstream then deleted the file. The fork gives its own added keys per-field defaults (older files load) and keeps the upstream fields required. The clean room gives **every** field a default (Q2).
- A wrong array length: elements beyond the option count are dropped by the model's bulk update; a shorter array leaves the tail off (SPEC-core-02 §5). Upstream's struct types rejected a wrong length at parse time for its own fields (fixed-size); the fork's roster import tests confirm "wrong length → error" for those fields.
- Duplicate keys: serde rejects a repeated known key; the fork additionally rejects any repeated key inside the two snapshots (`duplicate field` error).
- Values above 255 or negative: parse error.
- **Draft 9 fields (decided 2026-11-01).** New settings: `respawn_invincibility` (OnOff, default Off; SPEC-state-01), `settings_override_playback` (OnOff, default Off; SPEC-input-01), the Developer rows `dev_reload_config` and `dev_debug_hooks` (OnOff, default Off; SPEC-core-02 §2.8; `preload_samples` is the existing id), and an optional range per delay row (N3): `<delay id>_range`, a two-element array `[min, max]`, absent when no range is set (the checklist then applies; SPEC-core-02 §2.8). Every new field has a default, so a file without them loads (§3.2). Retired: `character_item` (N6, dropped row): accepted and ignored on load like `update_policy`, never written; the first-boot import has already converted it (SPEC-core-08 §4, SPEC-state-04 §7).

## 4. Outputs and native state touched

None in game memory. Files:

| Output | Path (upstream → clean room) | Format | When |
| --- | --- | --- | --- |
| Settings file | `sd:/ultimate/TrainingModpack/training_modpack_menu.conf` (JSON despite the extension) → `sd:/ultimate/TrainingRooms/settings.json` (Q3 for the name) | pretty-printed JSON `{"menu":{…},"defaults_menu":{…}}` (fork adds `"mii_preferences"` and `"stage_preferences"`, carry-over) | written on every menu close (queued to the writer thread, whole-file rewrite) |
| Published settings | the global settings value every feature reads (`MENU`) and the defaults (`DEFAULTS_MENU`) | in-memory copies | assigned at load and at every close **before** the file write is queued |
| Legacy menu paths (unused upstream constants) | `training_modpack_menu.json`, `training_modpack_menu_defaults.conf` | — | never read or written; do not carry |
| Updater config | `training_modpack.toml` → fork `training_modpack.json` (`{"update":{"last_update_version":"<RFC 3339 UTC>"}}`, created with the current time when absent, recreated when unparsable) | existed only for the dropped auto-update | drop (Q4) |
| Developer config | `dev.toml` → fork `dev.json`, an empty object; re-read while P1 holds L+R+A | development only | **ships (draft 9)** as the *Reload developer config* row of the hidden Developer page, gated by `developer.ini` (SPEC-core-01 §2 and Q5, decided 2026-11-01); the file is not part of the settings and is not imported |
| Legacy root | `sd:/TrainingModpack/*` moved into the data folder, old folder removed | — | SPEC-core-01 Q2 |
| Other constants | `version.txt`, `layout.arc` (dev override, SPEC-core-05), `UNPACK_PATH = sd:/` (updater) | — | not carried |

## 5. Timing and frame rules

- **Load**: at boot, after the data folder exists, on a dedicated thread with a 128 KiB stack that the boot sequence joins (the JSON parser's recursion overflowed the default thread stack upstream without a useful message). Order inside: if the file exists, read and parse; on success assign both snapshots to the globals; on failure log a warning (upstream then deleted the file); then serialize the (possibly factory) globals back into the menu model so the model starts in sync (`update_all_from_json` with the settings, and the defaults string kept for reset). The fork inserts its INI overlay (`cycle_preload.ini` takes precedence over the JSON values for its rows) between the parse and the model sync — fork carry-over.
- **Commit**: on the frame the menu closes (input hook, game thread): parse the model's combined payload into the two snapshots, assign the globals, queue the write. Features read the globals from the next frame. The fork additionally notifies interested features of the change (`settings_changed` callbacks for mash, lobby, roster, CPU pilot; fork carry-over).
- **Write**: a single writer thread receives `(path, contents)` jobs over a channel and writes the whole file (`std::fs::write`), logging success or failure. Upstream's thread had the default 2 MiB stack; the fork gives it 64 KiB and a name. The write is asynchronous; a crash within milliseconds of a close can lose the last commit (Q5).
- No per-frame file activity. Nothing is written when the menu closes because every controller was unplugged (SPEC-core-03 §2.3).

## 6. Edge cases and interactions

- **Missing file**: factory settings; the file is created at the first close.
- **Corrupt file** (not JSON, wrong types, missing required key, duplicate key): upstream deleted it on the spot and continued with factory settings. Recommendation (Q2): keep the file, rename it to `settings.invalid-<n>.json`, log, continue with factory settings; load every setting that parses and default the rest (per-field defaults).
- **File from a newer build** with extra keys: loads; extras are ignored and **kept on the next write** (lead decision 2026-11-02: unknown keys round-trip unchanged, so a newer build's file, or rows a package does not implement yet, survive a write by this build; only the retired keys `update_policy`, `menu_open_start_press` and `character_item` are dropped).
- **File from an older build** with fewer keys: loads only if every known key has a default (clean room) — upstream would have deleted it.
- **Upstream file in the user's old folder**: the key set and value shape are identical for all upstream rows, so an upstream `training_modpack_menu.conf` can be imported as-is (SPEC-core-01 Q2); `update_policy` and `menu_open_start_press` are ignored.
- **Fork envelope**: `mii_preferences` (per role/class identity selectors; invalid selectors are dropped individually) and `stage_preferences` (mode, direct stage, forms; invalid content is rejected and the file is re-read without it, keeping the numeric settings and reporting the reason) — fork's own, carry over with its tests. The fork also imports retired fixed-slot roster keys of its own older builds (`trio_fighter_1..3`, `player_roster_4..8`, `cpu_roster_1..8`, `*_roster_size` one-hot arrays) into ordered rosters once and never writes them again — fork's own.
- **Defaults semantics**: `defaults_menu` is what R + Yes restores; a factory reset is deleting the file. The two snapshots are independent; X copies the current settings over the defaults snapshot in the model, and both are written at close.
- **Save states, recordings, setup library, macros**: separate files (`save_states.toml` → fork `save_states.json`, `setups/`, `macros/`), owned by SPEC-state-* / SPEC-input-* / fork carry-over; this spec's writer thread is shared with them.
- **Concurrent writes**: all writes go through the one writer thread in order; a close while a previous write is pending queues a second whole-file write.
- **Thread failure**: if the writer thread cannot be created, upstream panicked (`expect`); the clean room logs and writes synchronously on the caller (as the diagnostics drain does, SPEC-core-01 §7).

## 7. Persistence

This spec *is* the persistence spec: file `sd:/ultimate/TrainingRooms/settings.json` (Q3), JSON as §3.1, pretty-printed, whole-file rewrite on close, read once at boot. Missing → factory; malformed → logged, preserved under another name, factory used (Q2).

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | JSON (serde_json) is already linked for the model; the TOML parser was removed by the fork (dependency `toml 0.5` gone) | no second serialization library | `tools/build/nro_size.py` |
| boot cost | one file read + parse on a joined 128 KiB thread | same | diag timing line |
| threads | one permanent writer thread, 64 KiB stack (fork; upstream 2 MiB) | 64 KiB or write synchronously from the close path (Q5) | install line (diag) |
| per-frame work | none | none | — |
| heap allocations in hooks | one JSON string + parse per close (game thread) | allocation only on close | counting allocator |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Wire shape: every setting an array in declaration order; OnOff `[1,0]`/`[0,1]`; sliders `[lower, upper]`; chord bit order | `core04_wire_shape_fixture` (fixture file pinned) | `core04-E1` read the file after a close | `docs/EDEN-RESULTS.md` |
| Factory file round trip; defaults snapshot independent of settings | `core04_roundtrip_factory`, `core04_defaults_independent` | `core04-E2` X then restart then reset | — |
| Unknown keys ignored on load and kept on rewrite; retired keys dropped | `core04_unknown_keys_ignored` | — | — |
| Missing keys take defaults (every field) | `core04_missing_keys_default` | `core04-E3` older file | — |
| Corrupt file: preserved under another name, factory used, logged | `core04_corrupt_file_policy` | `core04-E4` | `docs/EDEN-RESULTS.md` |
| Duplicate key rejected | `core04_duplicate_key_error` | — | — |
| Upstream `training_modpack_menu.conf` imports unchanged (incl. ignored `update_policy`, `menu_open_start_press`) | `core04_import_upstream_file` | `core04-E5` copy a real upstream file | — |
| Load publishes globals before the first Training frame; commit publishes before the write is queued | `core04_publish_order` (sequence) | `core04-E6` setting visible next frame | — |
| Writer thread stack/size; synchronous fallback | `core04_writer_fallback` | `core04-E7` | — |
| No write on unplug-close | `core04_no_write_on_unplug` | `core04-E8` | — |
| **Draft 9** New fields default when absent (`respawn_invincibility`, `settings_override_playback`, `dev_*`, `<delay id>_range`); the retired `character_item` key is accepted, ignored and not written back | `core04_draft9_fields_default`, `core04_character_item_retired` | `core04-E9` older fork file loads; the next close drops `character_item` | — |
| **S2** (decided) Corrupt file kept as `settings.invalid-<n>.json`, every field that parses is loaded, the rest default | `core04_corrupt_file_policy` (above), `core04_per_field_default_on_bad_field` | `core04-E4` | `docs/EDEN-RESULTS.md` |

## 10. Open questions

1. **Byte-exact sample.** The writer did not transcribe a real upstream settings file (none is checked into either tree). Recommendation: capture one from an Eden run of the fork into the test fixtures before implementation, so the clean room's fixture is validated against a real file rather than this description.
2. **Corrupt-file policy.** Upstream deletes; recommendation: preserve and rename, per-field defaults. Operator decision because it changes a user-visible recovery path. **Decided 2026-11-01 (S2): as recommended;** keep a renamed copy and load field by field.
3. **File name and location.** `settings.json` under `sd:/ultimate/TrainingRooms/`; or keep upstream's `.conf` name for drop-in import. Recommendation: new name, plus a one-time import of the upstream file (SPEC-core-01 Q2). **Decided 2026-11-01 (S1 item 1, SPEC-core-08): `settings.json`;** the import copies the fork's `training_modpack_menu.conf` under that name (corrected 2026-11-02; `training_modpack.json` is the updater record and is not imported), and the loader also accepts the old name once. (This spec's earlier working name `menu.json` is superseded.)
4. **Updater config file.** With auto-update dropped, `training_modpack.json` has no reader. Recommendation: do not create it.
5. **Asynchronous write.** Keep the writer thread (shared with save states and recordings) or write the small menu file synchronously at close. Recommendation: keep one writer thread with a 64 KiB stack for all files, flush on the panic path.
6. **Pretty vs compact JSON.** Upstream pretty-printed. Recommendation: pretty, for hand editing.
