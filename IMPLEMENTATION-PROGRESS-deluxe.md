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

## Merge and F8 (game thread never waits)

`work/implementation` 566780a (stream/sync: `hos-lock`, parking_lot gone) merged as `14016fe` (lead's request); conflicts only in
`Cargo.toml` / `CARRY-OVER.md` (both kept) and `Cargo.lock` (theirs plus the two SDK path crates). F8 audit of this stream's
paths (SPEC-core-01 §4.3):

| Path | Thread | Lock / wait | Treatment |
| --- | --- | --- | --- |
| `deluxe_row::register` (SDK `register`, `host()` `OnceLock` init) | boot (`main`, before any hook runs) | none contended: first and only init of the lookup | boot only |
| `CaptureRow::current` / `request` | Deluxe overlay draw thread | `CaptureToggle` compare-exchange loop (lock-free, no wait) | no lock |
| `writer::persist_replay_capture` | draw thread / menu worker | one atomic store | no lock |
| `writer::request_replay_capture`, `replay_capture()` | menu worker thread | toggle CAS + atomics | no lock |
| INI write | writer thread | atomic swap of the due flag, file I/O on the writer thread only | off the game thread |
| capture adapter (package 4) | game thread | toggle CAS (`is_armed`, `begin_record`, `end_record`) | unchanged, lock-free |

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

## Playbook

Playbook followed: `ae87443` (`git -C ~/ssbu-mod-playbook pull --ff-only`: already up to date, 2026-11-02 ~11:4xZ). Read
completely before work: README.md, docs/{WORKFLOW,EDEN,MACROS,TILE-CURSOR,GIFS,HOOK-SHARING,CHECKLIST}.md,
tools/PROVENANCE.md; tools read before running them: build/nro_size.py, build/scan-offsets.py, check/compliance.py,
eden-solo/{gv.py,solo.ps1,step.ps1}, eden-pair/solo_surface.ps1.

### Checklist (docs/CHECKLIST.md; M mandatory, C conditional)

| Item | Tag | State | Evidence |
| --- | --- | --- | --- |
| cdylib only | M | pass | `Cargo.toml` `crate-type = ["cdylib"]`; the SDK is an `rlib` dependency (LTO applies) |
| Release profile s/lto/cgu 1/abort; skyline =0.6.0; toolchain 1.98.1 | M | pass | `Cargo.toml`, `rust-toolchain.toml` unchanged |
| Pure logic in model with host tests; runtime only for the Switch | M | pass | `src/model/deluxe_row.rs` (9 tests), `src/runtime/deluxe_row.rs` under `runtime` (`cfg(target_os = "switch")`) |
| Gates: test, clippy -D warnings, no-default-features, release, diag | M | partial | host gates 2026-11-02 11:23–11:27Z: test 7706 passed 0 failed, clippy 0, no-default-features ok (`/tmp/dx-work/{test,clippy,nodefault}.log`); release + diag: **open** (queued) |
| No heap in hooks (`heap_allocations=0`) | M | open | no hook added; draw-thread callbacks are atomics only; Eden install line pending |
| Fixed buffers, writer thread, stack paths | M | pass | the INI write stays on the existing writer thread (`features::writer`), the row only sets an atomic flag |
| Root cause or approved workaround | M | pass | no workaround; host persistence dropped for one writer (R5), the fork's 2.0.0 file superseded (CARRY-OVER "deluxe"); the menu-row adapter gap is reported, not worked around (Open items) |
| Version guard 13.0.5 | M | pass | unchanged boot guard (`runtime::boot` step 1); registration runs after it |
| 64-byte guard per site | M | N/A | no hooked, called or relied-on game site is added: the host is reached through its exported symbol (`nn::ro::LookupSymbol`) |
| Card scan for the site | M | N/A | no game offset is used; the card's Deluxe NRO was analysed for its export and `register` instead (section 2) |
| Hook sharing / chaining, stand-ins, load orders | C | N/A | no hook; this is HOOK-SHARING's provider pattern (exported registration API, plugin runs without the provider) |
| Hook after the prologue / call instead | M | N/A | no hook |
| No diag code/strings in the shipped NRO | M | open | `compliance.py --nro <release>` after the release build |
| Shipped NRO writes its install line and nothing per frame | M | pass (code) | the row adds an install-line field and one boot `diagnostic` line (`[TrainingRooms]`, shipped tag, once) |
| Online: read-only rules hook by hook | C | open → pkg7 | the Deluxe row is also reachable online; package 7 owns the online rows |
| Online: replay A/B with the desync detector | C | open → pkg7 | |
| Online: two consoles past the changed path | C | open → pkg7 | no LDN here (brief) |
| Eden: claim before Up, release after Down | M | open | dx-01/dx-02 queued (Surface) |
| Eden: one per host, ≥ 10 GiB, not profile a / :61 | M | open | harness asserts both (`eden/dx_eden.py` `up`/`launch`), display :94 |
| Eden: hard-linked profile copy, only the copy changed | M | open | `dx_eden.py prep` (gv.py's copy rule) |
| Copies: identical linked, written-in-place copied | M | open | same |
| Disk: one target per copy, shared cargo home, df ≥ 40 GB, products deleted with manifest | M | partial | Surface target `cargo-targets/trainingrooms-deluxe`, Yoga target `/home/intpa/trainingrooms-build-deluxe/target`; df 178 GB (Surface) / 248 GB (Yoga); deletion at stream end: open |
| Eden: waits gated on a log line or screenshot | M | open | `route_dx.py` (gated route), row waits on the diag lines |
| Eden: failed precondition aborted and retried | M | open | |
| Ruleset short | M | N/A | Training mode only, no match to finish |
| Eden: profile restored, host released | M | open | `dx_eden.py restore` / `down` |
| console.log scan, run.json, memory.peak kept | M | open | `dx_eden.py scan` / `collect` |
| Every local command under memory/time guard | M | pass | cargo under `systemd-run --scope -p MemoryMax=6G timeout 1800` (`/tmp/dx-work/gate.ps1`); Eden under `eden-run.py` (8 GiB, bounded) |
| Each run's outputs kept under its label | M | open | `eden/runs/dx-NN/` |
| README GIF / license / disclaimer | M | N/A here | the repo README is the lead's; one third-party line is a lead decision (CARRY-OVER "deluxe" records the licence) |
| Dated evidence doc | M | partial | this file and `eden/EDEN-DELUXE.md` (with the runs) |
| Numbers from tested scripts | C | N/A | no numbers computed from logs are cited |
| No operator quotes, no claims beyond tests | M | pass | |
| Handoff current | M | pass | this file (branch, commit, evidence, next steps); the lead keeps TRAININGROOMS-HANDOFF.md |
| Release: bump, tag, assets | C | N/A | no release ordered |
| Release: rebuild byte-identical | C | N/A | no release |
| Card: MTP preflight, rollback, readbacks | C | N/A | no card work (hard limit); the card copy used is the 2026-11-01 install's local copy |
| Card: never mounted/ejected | M | pass | the card was not touched |
| Nothing pushed/tagged/written to the card | M | pass | commits only on `stream/deluxe` |
| Forgejo, push only on order, messages from a file | M | pass | commit `166d3cd` from `/tmp/dx-work/commit1.txt`; no push |
| Model/effort gate and watcher | M | pass | own transcript `agent-a458ac493fbd2f302.jsonl`: claude-opus-5-5 / high; watcher `/tmp/deluxe-watcher-a458ac/` (owner pid 1069629) |
| Every worker read the playbook | M | N/A | no subagent spawned |

### Scripts written here (not playbook tools)

| Script | Why the playbook does not cover it |
| --- | --- |
| `/tmp/dx-rea-scripts/nro2elf.py`, `table.py` | ABI reading of the host NRO (export, table, relocations): REA's `open_binary` refuses NRO; the playbook has no NRO disassembly tool (`nro_size.py` reads the header only) |
| `/tmp/dx-work/manifest_edit.py`, `manifest_reasons.py`, `manifest_show.py` | one-off edits of package 4's carry manifest; the repo's `tools/carry_pkg4.py` then derives the files (repo tooling, not a playbook concern) |
| `/tmp/dx-work/gate.ps1` | one cargo call under the playbook's memory/time guard with this copy's target (the playbook gives the rule, not a script) |
| `/tmp/dx-work/switch.ps1` | the Surface Switch recipe of docs/BUILD.md; **replaced by** the Yoga `build.ps1` copy (`/tmp/dx-work/build-deluxe.ps1`, the stream rules' recipe) while the Surface is held |
| `/tmp/dx-work/surface_free.ps1`, `host_free.ps1` | read the shared ledger for open claims per host; `solo.ps1`'s check is tied to its own owner and the Yoga, and the stream rules require all owners per host |
| `/tmp/dx-work/mk_dx.py` → `eden/dx_eden.py`, `eden/route_dx.py` | Eden driver: the repo's harness lineage is the playbook's `gv.py` + `solo.ps1` adapted (pkg6's `s6_eden.py`); `gv.py`/`solo.ps1` hard-code gsp-visualizer's root, profile `y`, display `:68`, owner, labels `e\d+` and LAN Online on, and `solo_surface.ps1` is elite-rematch's remote-Surface wrapper. Same commands (`up`/`down`/`shot`/`press`/`stick`/`call`/`macro`), claims per EDEN.md, tile cursor per TILE-CURSOR.md, macro segments per MACROS.md |

## Gates

Host gates: above. Switch release + diag, `nro_size.py`, `compliance.py --nro`: open (queued for a compile slot).
