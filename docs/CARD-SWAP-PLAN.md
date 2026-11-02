# Card swap: Training Modpack → TrainingRooms (plan)

2026-11-02, the lead. Order of the operator (relayed by the primary): once the clean room is done, the primary removes
the Training Modpack fork from the Switch card and installs the clean-room plugin in its place, and the relevant fixes
go into both trees. This page plans the swap and its data migration. Nothing here is executed by the clean-room lead:
the card, DBI and USB belong to the primary, on the operator's order.

## When the swap can happen ("done")

A release candidate of `libtraining_rooms.nro` with:

1. the full playbook gate: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo build --no-default-features`,
   `cargo skyline build --release` and the diag build, each in one call, with outputs kept;
2. every feature's Eden rows, batched per boot, with the `Unmapped` scan; the first-boot import rows below;
3. the online rows the playbook requires (lobby practice and swaps run online): the read-only review per hook, a replay
   A/B with the desync detector, and a two-console run with the detector on both (`desync_seen=0` at the same `frame=`);
4. `tools/check/compliance.py` with every `agent` row filled;
5. an illustrated install summary (what changes on the card, screens of the menu, the install line, the rollback).

The lead tells the primary when the candidate reaches this point. The NRO is never installed alongside the fork's:
both hook the same sites (`DECISIONS.md` B1).

## What moves, and how (per `specs/SPEC-core-08`, decisions S1, N6, N8)

| Fork file under `sd:/ultimate/TrainingModpack/` | TrainingRooms (`sd:/ultimate/TrainingRooms/`) | How |
| --- | --- | --- |
| `training_modpack_menu.conf` (the fork's settings, JSON content; 41,649 B on the card) | `settings.json` | copied and renamed at first boot; setting ids unchanged; *Item on load* is converted once into each save's item snapshot (N6) and dropped. Corrected 2026-11-02: an earlier draft named `training_modpack.json`, which is the updater record |
| `save_states.json` (live) | `save_states.json` | copied at first boot; the file is the one on the card at that moment, so the operator can keep using the fork until the swap |
| `setups/`, `mii-presets.json`, `macros/` | same names | copied (the `switch-c1162-firstboot` card listing shows no `setups/` folder: that copy is then empty) |
| `button_config.ini`, `replay_capture.ini` | same names | copied (S1 item 2); the menu now edits `button_config.ini` (N7) |
| `replay_captures/` | `replay_captures/` | copied in full, no size cap, on a background thread after boot (N8); the Replay Window lists a capture once its copy is complete |
| `cycle_preload.ini` | regenerated | not copied |
| `training_modpack.json` (updater record, 72 B) | – | not copied; the clean room has no updater |
| `*.toml` (the listing shows `training_modpack.toml` and a legacy `save_states.toml` next to the live `save_states.json`), `training_modpack_menu_defaults.conf`, `training_modpack_menu.json`, logs, `dev.json`, `layout.arc`, `version.txt` | – | not copied (named in `SPEC-core-08` §4 *Not migrated*) |
| `practice-trace*.bin` and `practice-trace*.map.json` (one `.bin` is 1.5 GB), `console-*.bin`, `console-*.log`, `native-unlink.bin`, the `handoff-*` folders, `backups/`, `install/`, `cpu_pilot/` (the pilot's own folder), `mii-editor-last.json` | – | not copied: fork trace, debug and tooling output. `SPEC-core-08` does not name them; its import list is a whitelist (§4 *Imported*), so they are skipped |
| – | `developer.ini` (`developer_page=on`) | written by the install for the operator's card only (N1) |
| – | `migration.json` | written by the import when it finishes (marker) |

Rules: the fork's folder is never changed; an existing destination file is never overwritten; the import starts only
when `migration.json` is absent, so the pre-installed `developer.ini` must not count as "already imported" (a
requirement for `SPEC-core-08` and its host test). Deleting `migration.json` re-runs the import (S1 item 5).

## Swap steps (primary, on the operator's order)

1. **Preflight** (`upgrade2-*`/`remove-mtp.py` style receipts): local copies of every plugin, Skyline's `exefs/`, and the
   whole `ultimate/TrainingModpack/` folder; free space on the card ≥ the size of `replay_captures/` plus 64 MiB.
2. **Remove** `libtraining_modpack.nro` with `remove-mtp.py` (archived on the card); its data folder stays.
3. **Deploy** `libtraining_rooms.nro` with `deploy-mtp.py`, then `ultimate/TrainingRooms/developer.ini`.
4. **First boot** on the console: the install line shows `result=installed` and the import counts.
5. **Readback**: copy `ultimate/TrainingRooms/` back; `migration.json`'s counts and hashes match the preflight copy.
6. **Rollback** if needed: remove `libtraining_rooms.nro`, restore the fork's NRO from the card archive. The fork's
   data is as it was at the swap; anything recorded after the swap is only under `TrainingRooms/`.

The companion plugins and Skyline stay as they are unless `SPEC-core-01`'s install line shows the clean room needs a
change; any such change is its own transaction, loader file last (playbook WORKFLOW §7).

## What the clean room needs from the card (read-only copies, by the primary)

1. The whole `ultimate/TrainingModpack/` folder as it is now, with a listing of `replay_captures/` (names, sizes):
   the import fixture for host tests and the Eden first-boot rows. A fresh copy again just before the swap.
2. The card's plugin set (the MTP preflight's local copies): the hook-site scan (`scan-offsets.py`) and the online
   rows' plugin set.
3. Skyline's `exefs/` (the patched `subsdk9` in use with C116.2) and the ARCropolis version, for the boot rows.
4. A listing of the mods folders (stage, effect, fighter mods; the Final Destination replacement): the run-time
   asset reads (`SPEC-core-07` §4.5–4.6, Q16).
5. Free space on the card.
