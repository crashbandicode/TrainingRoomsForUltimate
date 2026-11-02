# deluxe stream: progress and evidence

2026-11-02. Branch `stream/deluxe` from `27d84f4`, worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-deluxe`,
target `/home/intpa/smash-training-lobby/cargo-targets/trainingrooms-deluxe`, Eden labels `dx-NN`. Playbook `ae87443`.

Scope: carry the MIT Deluxe plugin SDK (decision L1), check its ABI against the card's Deluxe host 0.10.1, enable the
Replay capture row on the Deluxe Plugins page (menu question 7) with the menu row, the INI and the Deluxe row in sync.

## 1. Carry (licence evidence)

`CARRY-OVER.md`, section "deluxe": `deluxe-plugin-abi` 5.0.0 and `deluxe-plugin-sdk` 5.0.0, MIT, rev
`20dd1cbf5201d2b23e653b1bd9798122981f53da`, five files copied unchanged under `third_party/deluxe-plugin-sdk/` with their sha256;
nothing outside `sdk/` was materialised or read. `deluxe-sdk = ["dep:deluxe-plugin-sdk"]` and back in `default`
(`tests/pkg6_features.rs`: the default closure equals the M5o release closure again).

## 2. ABI against the card's Deluxe 0.10.1

Card copy: the 2026-11-01 MTP install folder `/home/intpa/ssbu-cpu-pilot-install/switch-mtp-v0101-v021/deluxe/`
(`operator-approval.json`: Deluxe v0.10.1, `new` sha256 `a8dcda242b890f09f3fdb2bd767dda2b85dbb1cd519f8e95c8fb6b57e41a5a59`); the
2026-11-02 c1162q install `verify/` copy of the card has the same sha256. Build string in the NRO:
`0.10.1-03e9d42ad2ef-…`.

Method: the NRO image wrapped as an ELF (segments at their NRO offsets, `.dynsym` from `DT_SYMTAB`; script
`/tmp/dx-rea-scripts/nro2elf.py`, not committed), `llvm-objdump -T` / `-d`, and the `R_AARCH64_RELATIVE` relocations of the
table (`/tmp/dx-rea-scripts/table.py`). (REA's `open_binary` refused the raw NRO format.)

| Fact | Evidence (offsets in the NRO image) |
| --- | --- |
| Export `ssbu_online_deluxe_plugin_api_v1` | `.dynsym`: `0x365a4`, size 0xc: `nop; adr x0, 0x1155d8; ret` |
| Export `ssbu_online_deluxe_online_context_v1` | `.dynsym`: `0x36520` |
| Table at `0x1155d8` | `struct_size = 0x20` (32), table `abi_version = 1`; relocations: `register = 0x1fdb0`, `host_status = 0x1fd98`, `host_build = 0x1fd6c` |
| `register` checks | `cbz x0` → -1; `ldr w8,[x0]; cmp w8,#0x48; b.ne` → -2 (`PluginDesc` must be 72 B); `ldr w8,[x20,#4]; sub w8,w8,#2; cmp w8,#3; b.hi` → -2, so **descriptor versions 2..=5 accepted**; `ldr w8,[x20,#0x24]; cmp w8,#1; b.hi` → -3 (flags: only `OWN_PAGE`); null `current`/`request` (`+0x30`/`+0x38`) → -6 |

SDK 5.0.0 (`third_party/.../sdk/src/lib.rs`): `host()` accepts only `table_matches(32, 1)`; `register_with` sends a 72-byte
`PluginDesc` with `abi_version = ABI_VERSION = 4`, calls `register` once and never retries (`REGISTER_ERR_ABI` →
`RegisterError::Abi`). The fork's `negotiated_descriptor_version` and its version-1 fallback do not exist in 5.0.0; the host
refuses version 1 anyway (`abi_version - 2` wraps above 3).

**Finding:** SDK 5.0.0 registers with the card's 0.10.1 host directly (table v1 matches; descriptor v4 is inside 2..=5). No
fallback is needed or coded. A host older than Deluxe 0.5.3 answers `REGISTER_ERR_ABI`: the plugin runs on without the row
(install line `reason=host_older_than_0_5_3`). Spec and host agree; nothing to escalate.

Host tests (`src/model/deluxe_row.rs`, `engine::deluxe_row`): a fake `register` with exactly the checks above; version 4 registers
in one call with no persistence path; an older host (2..=3) answers Abi once with no retry; a table of another size or table
version is incompatible and never called; no host off the console; the host's callbacks read and change the one preference,
Pending during a record, L+R+X (the 0.10.1 second action: a request for the shown choice) writes nothing; every register error
maps to a named install-line reason.

## 3. The row (Q7)

- `engine::deluxe_row` (pure): `SPEC` (`training_rooms` / "Training Rooms", one option `replay_capture` / "Replay capture",
  Off/On, tones Neutral/Ok, `no_persist`, `config_path: None`), `CaptureRow` (handler over package 4's `CaptureToggle`),
  `RowState` (install-line reasons).
- `runtime::deluxe_row`: `register()` once from `macros::install` after `replay_store::init` (INI seed) and before
  `live_replay::init` (a registered row keeps the adapter resident while the INI says off); only when the layer's guarded closure
  passed (else `capture_layer_off`). Install line: `deluxe_row=installed` or `deluxe_row=not_installed reason=<…>`; diag line
  `[TrainingRooms] deluxe_row registered handle=… descriptor=4 host_status=… host_build=…`.
- Sync: one preference, one writer. The menu row (`writer::request_replay_capture`) now also changes package 4's toggle; the
  Deluxe row changes the toggle and calls `writer::persist_replay_capture` (an atomic flag); the writer thread writes the INI
  from `writer::replay_capture()`, which is the toggle's preference once the adapter is ready, so the file ends at the newest
  value whichever row changed it. The host is not asked to persist (it would be a second writer of the file).
- Draw-thread callbacks: atomics only (toggle word, writer flag); no lock across game calls, no I/O, no allocation. Boot
  registration allocates once (the SDK's string copies and handler box), outside any hook.

## Seams in other streams' files (for the lead)

- `src/runtime/macros/mod.rs` (package 4): one call, `crate::runtime::deluxe_row::register()`, between `prepare_buffers` and
  `live_replay::init`; one doc line.
- `src/runtime/features/writer.rs` (package 4): `request_replay_capture` also requests the toggle; new `persist_replay_capture`;
  `replay_capture()` reads the toggle when ready; the slot became an atomic "write due" flag read at write time.
- pkg4 carry manifest: `capture_plugin.rs` superseded; `replay_store.rs` six edits (CARRY-OVER "deluxe").
- `src/runtime/boot.rs`: one `FeatureReport` (`deluxe_row`).

## Open items

- Without a Deluxe host and with `replay_capture.ini` off, the capture adapter does not install at boot (package 4's
  `live_replay::init`: hooks only when the INI says on or a host registered), so turning the TrainingRooms menu row On writes the
  INI but takes effect at the next boot (the toggle answers Unavailable until then). Q7 says the row can be changed in game; the
  fork's own reason for the host case ("a later On cannot install Skyline hooks") applies to the menu row too. Root fix belongs
  in package 4's `live_replay::init` (adapter resident whenever the layer installed). Not changed here (package 4's file).
- The TrainingRooms menu reads the preference when it opens (`ui::worker::env`); a change made on the Deluxe page while the
  TrainingRooms menu is open shows at its next opening.

## Gates

(filled in at the gate)
