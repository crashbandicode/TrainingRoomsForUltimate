# Package 3 (save states) progress

Branch `stream/pkg3`, worktree `TrainingRoomsForUltimate-pkg3`. Not pushed. Updated 2026-11-02.

## Milestones

| Milestone | State | Evidence |
| --- | --- | --- |
| S3-1 save/load core, JSON saves, setup library | done; Eden pass (s3-01) | `docs/test-provenance/package-3.md`, `eden/EDEN-PACKAGE-3.md` |
| S3-2 charges, buffs | host pass; Eden: Samus charge and Bob-omb snapshot (s3-02), Cloud Limit buff (s3-04, after the s3-03 crash fix) | same |
| S3-3 items, fighter-specific restore (Steve, Olimar, Sora, Pokemon Trainer, Kirby) | host pass; Eden pass: Steve (s3-01), Pokemon Trainer (s3-05, s3-07), Kirby hat (s3-06); Olimar and Sora not Eden-run | same |
| S3-5 first-boot import (SPEC-core-08) | host pass (13 filesystem tests); Eden pass: s3-09 (all files, staging skipped, `developer.ini` untouched) and s3-10 (N6: 5 snapshots), `check-migration.py verify` PASS | same |
| S3-4 Switch builds, batched Eden rows, compliance | done: builds, Eden s3-03 to s3-10, `docs/compliance-package-3.md` | below |

## State of the code

- The model (`src/model/state/`) is pure and host-tested: records and JSON (fork order, absent options as `null`,
  new optional `copy_charge` only when present), the load machine, placement rules, charges (own table and
  Kirby's copied table), buffs, items, Steve, Olimar line, Pokemon Trainer decide and wait, Kirby acceptance,
  setup library, N6 conversion.
- The runtime (`src/runtime/features/states.rs`, `state_effects.rs`, `state_restore.rs`, `state_recorder.rs`,
  `import.rs`) calls the game only through guarded site rows (`src/model/sites_pkg3.rs`, generated from the owned
  ELF by `tools/gen_sites_pkg3.py`): 114 rows, 31 replace hooks, one inline hook (Pokemon decide).
- Persistence follows review R1 (atomic replace, copy-aside of a corrupt file, fs-backed two-boot tests).
- Hooks allocate nothing; the diag build logs `state_hook_counts` (now with `se_silenced`, `se_passed`,
  `switch_fx`, `decide`, `copy_hook`).

## Builds (Yoga, `cargo skyline build --release`)

Built at 13be87c in `/home/intpa/trainingrooms-build-pkg3/dist/{release,diag}`:

| Build | sha256 | Resident |
| --- | --- | --- |
| release | `aefbcad56be10d8fa16c39b3ae2be7e9a2a874b3984235f1788489d4a2685948` | 421888 B (412 KiB) |
| diag | `e39444a8d39d98f0942bcacfa3f401e5cd8a93370bcf78a0f283d28f5c4a90f2` | 548864 B (536 KiB) |

Gates at 13be87c (full host tests on the Surface, clippy and no-default-features on the Yoga, all green): `cargo test --no-fail-fast` all pass (29 binaries); `cargo clippy --all-targets -- -D warnings`
and `cargo build --no-default-features` clean (run on the Yoga); runtime code is `cfg(target_os = "switch")`
and is compiled only by the Switch build (four dead-code warnings for `request_save`, `request_load`,
`is_loading`, `is_killing`, which package 4's chord engine consumes).

## Process slip (2026-11-02)

At 06:48Z `s3_eden.py up s3-08` wrote a CLAIMED line on the Yoga while the foreign claim `m5qo1` (b81ec740/tm-m5q-online)
was open. Cause: the harness decided "free" from the owner's last line, and the foreign worker had written a
RELEASED line for its R-D1 claim that said "Yoga continues under claim m5qo1". The up failed on the GPU lock, the
harness wrote the matching RELEASED, nothing else was touched (the label s3-08 is consumed). Fixes: the harness
ignores a RELEASED line that says "continues under", and the retry wrapper refuses while any other owner's claim
on the host lacks a later RELEASED. It is now the general rule in TRAININGROOMS-STREAM-RULES.md.

## Known gaps and decisions

- Chord engine and the recorder are package 4's: `states::request_save/load` and `state_recorder` adapters log
  only until the merge.
- Kirby's copied-kind source is `INT_COPY_CHARA`; the diag dump compares the module offset `+0x17398`.
- The decide descriptor layout is a native check; a rejection in normal play is a defect to root-cause.
- Kirby and Sora silences apply only while loading (spec open question 4).
- Held-item capture limits, the setup-library UI glue (packages 4 and 6), and Eden rows for fighters other than
  Steve, Samus, Cloud, Pokemon Trainer and Kirby are not run.

## Playbook record (retroactive audit, 2026-11-02, pbaudit, playbook ae87443)

Stream `pkg3` (package 3, save states; transcript `agent-ab8ec5011401d2954.jsonl`, claude-sonnet-5-5 high). The progress file above records no playbook commit and no checklist; `docs/compliance-package-3.md` holds a partial checklist (agent rows only, written 2026-11-02 06:38Z). Code in scope: `src/model/state/`, `src/runtime/features/{states,state_effects,state_restore,state_recorder}.rs`, `src/runtime/import.rs`, `src/model/sites_pkg3.rs`, `tools/gen_sites_pkg3.py`, `tests/state*.rs`, `tests/migration*.rs`, `eden/{s3_eden,route_s3,s3d}.py`.

### Reading evidence (transcript audit)

**Read: yes, all nine files, early, at playbook `009c9ac` (not `ae87443`).** Transcript (UTC, 2026-11-02): 04:18:00 `git log`/`git pull --ff-only` printed `009c9ac compliance.py: read src/ recursively` and the file sizes; 04:18:04-04:18:08 `Read` of `README.md`, `docs/CHECKLIST.md`, `docs/WORKFLOW.md`, `docs/EDEN.md`, `docs/MACROS.md`, `docs/HOOK-SHARING.md`, `docs/TILE-CURSOR.md`, `docs/GIFS.md`, `tools/PROVENANCE.md` (sizes 10,037 / 17,485 / 8,723 B for CHECKLIST / WORKFLOW / PROVENANCE: the pre-`ae87443` versions; `ae87443` added `swap-ums.py` text and a CHECKLIST line, nothing that changes a pkg3 row). It also read the stream rules (04:17:51, which already contained "Playbook read in full" in its First list), `MY_MODEL.md` (04:17:52), ran `scan-offsets.py` (04:38:41), read `compliance.py` (06:00:15) and ran it (06:37:39, `/tmp/pkg3-compliance.md`). The brief only pointed to the rules file for the playbook; it named no clone path or commit.

### Checklist (`docs/CHECKLIST.md`, M and C items applied to this stream's code as merged in `bf836c8`)

Result vocabulary: `pass`, `fail`, `N/A: reason`, `not authorized`, and `open` (no evidence yet: not a pass). Tree-level rows cite the audit's compliance run: `python3 -I ~/ssbu-mod-playbook/tools/check/compliance.py <pbaudit worktree at bf836c8> --nro cargo-targets/integ3-nro/libtraining_rooms-release.nro` (report: `docs/pbaudit/compliance-bf836c8.md`). The audit did not rerun gates or Eden; evidence quoted from the stream's own files is marked as such, evidence marked "audit" was checked by the audit itself.

| # | Requirement | Tag | Result | Evidence |
| --- | --- | --- | --- | --- |
| 1 | Implementation: `crate-type = ["cdylib"]` only | M | pass | tree-level: `docs/pbaudit/compliance-bf836c8.md` row 1 (`Cargo.toml crate-type = ['cdylib']`) |
| 2 | Implementation: release profile `opt-level "s"`, lto, cgu 1, panic abort; `skyline = "=0.6.0"`; toolchain 1.98.1 | M | pass | tree-level: `docs/pbaudit/compliance-bf836c8.md` rows 2, 3, 5 (profile as required, `skyline = "=0.6.0"`, toolchain 1.98.1) |
| 3 | Implementation: pure logic in `model` with host tests; `runtime` only under `cfg(target_os = "switch")` | M | pass | `docs/pbaudit/compliance-bf836c8.md` (`src/lib.rs` carries `target_os = "switch"`); pkg3 host tests `tests/state01_json.rs`, `state01_machine.rs`, `state01_setup.rs`, `state02_charges.rs`, `state03_buffs.rs`, `state04_convert.rs`, `state04_items.rs`, `state05_restore.rs`, `state06_07.rs`, `state_sites.rs`, `migration.rs`, `migration_run.rs` |
| 4 | Implementation: gates, one cargo call each (test, clippy -D warnings, no-default-features, skyline release, diag build) | M | pass | stream record at `13be87c` (not re-run by the audit): `cargo test --no-fail-fast` 29 binaries ok (Surface); clippy `-D warnings` and `--no-default-features` clean (Yoga, at `4fdf560` and `13be87c`); Switch release `aefbcad5...` (412 KiB resident) and diag `e39444a8...` (536 KiB) on the Yoga (`IMPLEMENTATION-PROGRESS-pkg3.md` "Builds"; `docs/compliance-package-3.md` row 1). The later stream commit `8cf2a7c` is not gated in the record. Integrated tree at `bf836c8`: integrator-3 (7,826 tests, clippy 0, no-default-features ok) |
| 5 | Implementation: no heap in hooks (counting allocator, `heap_allocations=0` in the install line) | M | pass | `eden/EDEN-PACKAGE-3.md`: s3-05 found `heap_allocations=1` (a `Vec` in `decide_index` inside the hook), fixed at the root in `13be87c`; s3-06 and s3-07 `heap_allocations=0`; s3-01 install line `heap_allocations=0`. Detector tests: `tests/hook_alloc.rs` (r1fix R5) |
| 6 | Implementation: fixed buffers, writer thread on a static stack, stack-built paths | M | open | save-state file writes run on `tr-state-writer` (`src/runtime/features/states.rs:1425`) and the import on `tr-import` (`src/runtime/import.rs:26`): `std::thread::Builder` with `stack_size(128 * 1024)`, i.e. a heap-backed std thread, not the exemplar static stack; the stream says hooks allocate nothing but does not claim or show the row. The exemplar repos (gsp-visualizer, ssbu-gnw-judge-log) are not on this host, so the audit could not compare |
| 7 | Implementation: root cause, or each workaround (guard, skip, restriction, tolerance, lint `allow`, feature gate, setting off) approved and listed with its date | M | fail | root-cause fixes shown: s3-03 crash (wrong argument type, `1076ddc`), s3-05 heap (`13be87c`), s3-08 harness claim bug. But `#[allow(clippy::derivable_impls)]` at `src/model/store.rs:138` entered with `e162674` ("Package 3: shared generic record Store, carried from the fork's library.rs") and is neither listed as a workaround nor approved (audit: `git blame`). No operator-approved workaround list exists in the stream files |
| 8 | Hook safety: version guard (`result=unsupported_version`) | M | pass | tree-level: `docs/pbaudit/compliance-bf836c8.md` row "13.0.5 in model" (`src/model/guard.rs`, `install_line.rs`, `archive_recipe.rs`); `tests/core01.rs:70` asserts `result=unsupported_version` |
| 9 | Hook safety: 64-byte guard on every hooked, called and relied-on site | M | fail | tree-level checker limit (see row text); pkg3 evidence: `tests/state_sites.rs` compares each of the 114 `sites_pkg3.rs` rows with the owned 13.0.5 ELF; `tools/gen_sites_pkg3.py` generates them; the install line reports per-site outcomes in Eden (s3-03: `state_pokemon_decide=stock/near`, `state_kirby_opff=stock/far`, `state_effect_req_on_joint_hook=stock/far`) |
| 10 | Hook safety: card scan (`scan-offsets.py`) of the site over a dated card copy | M | pass | audit (transcript 04:38:41Z): `tools/build/scan-offsets.py` over `/home/intpa/ssbu-cpu-pilot-install/switch-mtp-v0101-v021/pilot/after/.../skyline/plugins` (copy dated 2026-11-01 10:19, 16 plugins) for 13 pkg3 offsets (`208de30 2281a20 1ff0ab0 ...`): only `libtraining_modpack.nro` (the fork this plugin replaces) references them. Caveat: `docs/compliance-package-3.md` marks this row "N/A, no card work" and contradicts its own scan; 13 of the 114 rows were scanned, the called-function rows were not |
| 11 | Hook safety: sharing / chaining when another plugin may hook the site (both stand-in modes, both load orders in Eden) | C | N/A: scan shows only the fork at the scanned sites | the fork is replaced by this plugin; the CPU pilot has no reference to the 13 offsets. Eden s3-02..s3-07 ran with the pilot and the tile cursor and `result=installed` (coexistence); stand-in modes were not run |
| 12 | Hook safety: hook after the prologue, or call the function instead | M | open | 31 replace hooks at function entries plus one inline hook (`state_pokemon_decide`, `main+0x34cec44`) per `IMPLEMENTATION-PROGRESS-pkg3.md`; `docs/compliance-package-3.md` calls the row "pass" without a per-row placement review; integrator-3 lists it open (RC review) |
| 13 | Hook safety: no diag code or strings in the shipped NRO | M | pass | tree-level: `docs/pbaudit/compliance-bf836c8.md` "libtraining_rooms-release.nro: no diag markers" (sha256 bbe6ffceb32ff61e, `cargo-targets/integ3-nro/libtraining_rooms-release.nro`) |
| 14 | Hook safety: the shipped NRO writes its install line and nothing per frame | M | pass | save-state slot file, setups and the import's copies are the plugin's data files (a data-product plugin, as the row allows); the shipped build has no diag markers; s3-09 and s3-10 ran the release NRO and `check-migration.py verify` passed (`eden/EDEN-PACKAGE-3.md`) |
| 15 | Online: read-only rules reviewed hook by hook | C | N/A: Training only (stream's claim) | `docs/compliance-package-3.md`: every hook is gated by `is_training_mode`. The audit did not re-verify the gate hook by hook; if the lobby-practice scene counts as online play, this row reopens |
| 16 | Online: replay A/B with the desync detector identical | C | N/A: Training only (stream's claim) | as row 15 |
| 17 | Online: two consoles past the changed path, `desync_seen=0` at the same `frame=` | C | N/A: Training only (stream's claim) | as row 15 |
| 18 | Eden: claim written before Up and released after Down, also after a failed run | M | fail | Yoga ledger `/tmp/deluxe-surface-1007/OWNER-COORDINATION.md` (copied read-only by the audit): owner `0caa72b8/pkg3-eden` 10 CLAIMED / 10 RELEASED, none open. But s3-08's first attempt wrote CLAIMED while the foreign claim `m5qo1` was open (recorded in `IMPLEMENTATION-PROGRESS-pkg3.md` "Process slip"; the GPU lock refused, no Eden ran, the label was consumed); the harness fix and the general rule followed |
| 19 | Eden: one Eden per host, MemAvailable >= 10 GiB, never profile `a`, display `:61` or another agent's runs | M | pass | `docs/compliance-package-3.md`: harness `s3_eden.py up` asserts one Eden, MemAvailable (about 22 GiB seen), own profile `t3`, display `:97`; the s3-08 slip above is the only breach |
| 20 | Eden: profile is a copy with hard-linked game content; identical files linked, written files copied | M | pass | `eden/s3_eden.py prep` copies profile `b` with `os.link` for read-only content (code read by the audit); `eden/EDEN-PACKAGE-3.md` header. A `du` check is not recorded |
| 21 | Disk: one `CARGO_TARGET_DIR` per source copy, cargo home shared, `df` >= 40 GB, finished products deleted with a manifest | M | open | own targets: Surface `cargo-targets/trainingrooms-pkg3` (no longer present at audit), Yoga `/home/intpa/trainingrooms-build-pkg3` (still present, 2026-11-02); `df` 277 GB before Yoga builds, 40 GB asserted by `up` (`docs/compliance-package-3.md`). No deletion manifest recorded for the finished stream |
| 22 | Eden: every wait gated on a log line or screenshot check | M | pass | `eden/route_s3.py` gates on the install line and reference screenshots (15 gate references, 10 short sleeps); `s3d.py wait` on log lines (`docs/compliance-package-3.md`) |
| 23 | Eden: a run whose precondition line never comes is aborted and retried under the next label | M | pass | s3-03 crashed (Cloud Limit load); the fix was built and the row rerun as s3-04; s3-08 refused by the GPU lock was not counted (`eden/EDEN-PACKAGE-3.md`) |
| 24 | Eden: shortest ruleset (1 stock, 1:00, items/hazards/FS meter off) | M | N/A: Training mode | all pkg3 rows run in Training (no match ruleset); s3-09/s3-10 boot to the title only |
| 25 | Eden: profile restored after the run (set-aside NROs back, temp files removed, host released) | M | open | `down` after every run is recorded; no `restore` of the profile (`s3_eden.py restore`) is in the transcript; the profile keeps `*.before-s3-NN` archives by design (`docs/compliance-package-3.md`) |
| 26 | Eden: `console.log` searched for panic/abort/crash/Unmapped (fail below 0x1000 or in the plugin's window); `run.json`, `memory.peak` kept | M | pass | audit: scan of the retained Yoga `runs/s3-01..s3-10` `console.log`: 0 matching lines except s3-03 (1 line: the crash, `unmapped address 0x0`, fixed in `1076ddc` and rerun clean as s3-04); `run.json` and `memory.peak` present for every run (peaks 4.9-6.7 GB) |
| 27 | Every local command under the memory and time guard | M | pass | cargo through `/tmp/pkg3-cargo.ps1` (contains `systemd-run ... MemoryMax=6G` and `timeout`; audit read it) and the Yoga builds under `MemoryMax=8G timeout 3000` (`docs/compliance-package-3.md`) |
| 28 | Each run's outputs kept under its label | M | pass | `eden/runs/s3-NN/` on the Yoga (`/home/intpa/trainingrooms-eden-p3/runs/`): every label has `console.log`, `run.json`, `memory.peak` (audit listing; s3-08 was never booted) |
| 29 | Docs: README (paragraph, full-screen GIF, install path, build commands, license, AI disclaimer) | M | fail | tree-level: `docs/pbaudit/compliance-bf836c8.md` rows "README has a GIF, build commands and a license line" and "a GIF under docs/media" fail (the AI-disclaimer row passes); assigned to package 7 by the lead (integrator-3 record), not this stream's |
| 30 | Docs: a dated evidence doc (run labels, build, what was done, what the log showed) | M | pass | `eden/EDEN-PACKAGE-3.md`: labels, NRO sha256 and source commit per run, results and log lines |
| 31 | Docs: numbers cited from logs come from a committed, host-tested script | C | N/A: no log-derived statistics | the Eden doc quotes log lines and the checker output; `eden/check-migration.py` is not in the repo at `bf836c8` (the doc cites it as the verifier of s3-09/s3-10) - open sub-item: the verifier script should be committed with a test |
| 32 | Docs: no operator quotes, no claims beyond the tests | M | pass | rows not run (Olimar, Sora, Samus-hat charge, ledge raise) are listed as host-tested or unrun in `eden/EDEN-PACKAGE-3.md` and the progress file |
| 33 | Docs: handoff file current (branch, commit, evidence, live state, next commands) | M | fail | tree-level: `docs/pbaudit/compliance-bf836c8.md` `HANDOFF.md` missing; the stream state is in this file and the campaign handoff is `TRAININGROOMS-HANDOFF.md`, neither is the plugin repo's `HANDOFF.md`; package 7 per integrator-3 |
| 34 | Release: minor bump, annotated tag, Forgejo release, assets verified | C | not authorized | no operator order |
| 35 | Release: rebuild of the tag byte-identical to the released NRO | C | not authorized | no operator order |
| 36 | Card: DBI MTP preflight, rollback copy, readbacks, receipts | C | not authorized | no operator order |
| 37 | Card: the user connects and disconnects; the agent never mounts, unmounts, ejects or remounts, nor starts or stops DBI | M | pass | `docs/pbaudit/compliance-bf836c8.md` row "no script mounts, unmounts or ejects the card" passes; transcript shows no `gio`/`udisksctl`/DBI/`tools/switch` command; ledger lines say "No physical Switch writes" |
| 38 | Without the order: nothing pushed, tagged or written to the card | M | pass | transcript shows no `git push` or `git tag` and no card write (`rg` over the stream's PowerShell commands) |
| 39 | Git: Forgejo, never GitHub; commit messages from a UTF-8 file | M | fail | remote is Forgejo (compliance pass). Commit messages: transcript heuristic 17 `git commit -F` and 4 single-line `git commit -m` (the checklist asks for a UTF-8 file) |
| 40 | Routing: model/effort gate and per-thread watcher | M | fail | read `MY_MODEL.md` at 04:17:52Z (model/effort from the transcript). The only watcher in the transcript polls the Yoga queue/ledger, not model/effort (`docs/compliance-package-3.md` row "Model/effort gate and watcher" says so) |
| 41 | Routing: every spawned worker read this playbook itself | M | N/A | the stream spawned no worker (no Agent call in its transcript) |

Counts: 20 pass, 7 fail (#7, #9, #18, #29, #33, #39, #40), 4 open (#6, #12, #21, #25), 7 N/A, 3 not authorized.
