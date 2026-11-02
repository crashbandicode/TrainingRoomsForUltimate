# Integrator 3: merges into work/implementation and the RC checklist (2026-11-02)

Lead `0caa72b8`; integrator-3 = Claude Opus 5.5 high (`claude-opus-5-5`, effort high from the session transcript; one hidden
watcher, model/effort only). Work tree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-work`, branch `work/implementation`,
target `cargo-targets/trainingrooms-work`. Nothing pushed, tagged or written to a card.

Playbook followed: `ae87443` (`git -C ~/ssbu-mod-playbook pull --ff-only`: already up to date; README.md, every file in docs/,
tools/PROVENANCE.md read in full, and the tools run: `tools/build/nro_size.py`, `tools/check/compliance.py`).

## Commits

| Commit | What |
| --- | --- |
| `27d84f4` | Merge stream/pkg2 `29b603c` (CommonParams shield multiplier, stage table, install line rendered whole = ui-screens `8658812`, Ally scripts as one set, `dismiss-fw.ps1` removed) |
| `a6d1e77` | Merge stream/pkg6 `98d6f5b` (clean) |
| `2bd47c2` | One wrapper per native (lead ruling): seven addresses wrapped twice, `state_natives` copies deleted, `tests/native_wrappers.rs` |
| `bf836c8` | Diag builds: two closures that need no `mut` |
| `a968c05` | Merge stream/sync `dc01e5a` (hos-lock kernel-arbiter locks, `published::Live`, the menu-close hang's root fix). `compat/menu.rs`: sync's file; package 6's preload canonicalisation and roster re-plan pasted into sync's `before_publish` / `after_publish` as ui-screens sent them (no lock edit); `apply_snapshot` and `CURRENT` gone. Needs one merged-tree Eden boot before the push (menu-close cycles, a menu commit reaching the preload engine, a P1 hot swap, hook allocations 0, no `Unmapped`): asked pkg6 to run its s6 rows on this NRO |
| `73831b8` | Diag probes behind features `texture-probe` / `seam-drill`, wired to their callers; `run_buff` loses the `fighter` parameter that 1076ddc left unused (gates queued) |

## Gate evidence

| Head | cargo test | clippy -D warnings | --no-default-features | Switch release | Switch diag |
| --- | --- | --- | --- | --- | --- |
| `27d84f4` | 61 binaries, 7,697 passed, 0 failed | exit 0 | exit 0 | `99282821…` 2,879,488 B resident | `2174f5b1…` 3,100,672 B |
| `bf836c8` | 63 binaries, 7,826 passed, 0 failed (on `2bd47c2`; `bf836c8` changes diag code + one test attribute, its wrapper test re-run) | exit 0 | exit 0 | `bbe6ffceb32ff61ea2a040f0bd05bd168f6be28baa9ee61e1acc09ed421e5be1` 2,940,928 B | `6a54682d40f279095bf61cf9c1acbc7ed1c846e8992bd0fe0e46a89d1fe9fb4d` 3,162,112 B |
| `a968c05` (incl. `73831b8`) | open (queued on the Surface and the Yoga; every host held by an Eden boot at 12:0xZ) | open | open | open | open |

Each cargo call: `systemd-run --user --scope -p MemoryMax=6G /usr/bin/timeout 1800`, `CARGO_TERM_COLOR=never`, one call each; Switch
builds with the campaign rustup home (docs/BUILD.md). Logs kept under `/tmp/integrator3/` (test.log, test2.log, clippy*.log,
nodef*.log, sw-*.log). Surface claims and releases: ledger lines 10:53/10:55Z, 10:59/11:01Z, 11:18/11:22Z, 11:28/11:30Z.

## Checklist (docs/CHECKLIST.md) for the integrated tree

`compliance.py <work tree> --nro <release NRO of bf836c8>`: machine rows below; the four failing rows are named exactly.

### Implementation
| Item | Tag | Result | Evidence |
| --- | --- | --- | --- |
| crate-type cdylib only | M | pass | compliance: `Cargo.toml crate-type = ['cdylib']` |
| release profile s/lto/cgu 1/abort; skyline =0.6.0; toolchain 1.98.1 | M | pass | compliance rows (three) |
| pure logic in model with host tests; runtime only under cfg(switch) | M | pass | compliance: `src/lib.rs`; 7,826 host tests |
| gates, one cargo call each | M | pass for `bf836c8`; open for `73831b8` | table above |
| no heap in hooks: counting allocator, `heap_allocations=0` in the install line | M | machine half pass; runtime half open | compliance allocator row; `tests/hook_alloc.rs`; no Eden boot of the integrated NRO yet (the line must read `heap_allocations=0`) |
| fixed buffers, writer thread on a static stack, stack-built paths | M | open → integrator-3 after sync (lead ruling: static buffer sized by a const from the site tables, host worst-case test, `result=line_overflow` never silent) | hooks: fixed buffers (`tests/hook_alloc.rs`); the install line is rendered into a `String` on the boot thread, outside hook scope (`install_line::render`, lead's direction, = ui-screens `8658812`) because a fixed buffer cut it twice (2,048 B, 8,192 B); the RC decides whether that counts as the fixed-buffer rule |
| root cause or operator-approved workaround, listed with date; shared causes checked for one abstraction | M | open (list) | this integrator added no workaround; shared-cause abstraction applied: one wrapper per native (`2bd47c2`), one native-call grant layer (R2-1). The operator-approved workaround list across streams is the lead's to compile |

### Hook safety
| Item | Tag | Result | Evidence |
| --- | --- | --- | --- |
| version guard, `result=unsupported_version` | M | pass (code + host tests) | `runtime/boot.rs` step 1, `model::version` tests |
| 64-byte guard on every hooked, called and relied-on site | M | pass; compliance row fails on a **recorded checker limit** (lead ruling 2026-11-02) | compliance: `884 guard(s) in src/, sizes [0, 64]`; the single "0" is `Site::EMPTY`'s initializer `guard: [0; GUARD_BYTES]` (`src/model/seams/native_seams/mod.rs:229`, `GUARD_BYTES = 64`), which the checker's regex does not read as an array of 64. Evidence that every real guard is the 64 bytes of 13.0.5: the host tests against the owned ELF (`core01_sites_match_owned_elf`, `cpu_common_guards_match_the_owned_module`, `tests/state_sites.rs`, `tests/cpu_sites.rs`), all passing in the gate runs above |
| card scan of each site over a dated card copy | M | open | per stream (e.g. pkg2: 2026-11-01 10:19 MTP preflight copy, 16 plugins); no single scan of every offset of the integrated tables yet |
| hook sharing / chaining; both stand-in modes and both load orders in Eden | C (applies: the CPU pilot and other card plugins share sites) | open | the drill is now buildable (`--features seam-drill`, `73831b8`); no Eden run of it on the integrated tree |
| hook after the prologue, or call instead | M | open (RC review) | each row's `Placement` in the site tables; `Entry` placements are leaf thunks whose 16-byte far jump fits (row notes), to be reviewed as a list |
| no diag code or strings in the shipped NRO | M | pass | compliance: `libtraining_rooms-release.nro: no diag markers`; probes behind `diag`, `texture-probe`, `seam-drill` |
| shipped NRO writes its install line and nothing per frame | M | open | needs a release boot of the integrated NRO with `events.log` read |

### Online (the CPU behaviours and package 6's lobby practice run in online lobbies)
| Item | Tag | Result | Evidence |
| --- | --- | --- | --- |
| read-only rules reviewed hook by hook | C (applies) | open | pkg2 reviewed its hooks (`IMPLEMENTATION-PROGRESS-pkg2.md`); the integrated set has no single review |
| replay A/B with the desync detector | C (applies) | open | not run |
| two consoles past the changed path, `desync_seen=0` at the same frame | C (applies) | open | not run |

### Eden (the integrator ran no Eden; these are the per-stream runs' and the RC run's)
| Item | Tag | Result | Evidence |
| --- | --- | --- | --- |
| claim before Up, release after Down | M | open for the RC run | integrator compile claims only (lines above) |
| one Eden per host, MemAvailable >= 10 GiB, not profile a / :61 | M | open for the RC run | |
| hard-linked profile copy, only the copy changed; identical files linked, written files copied | M | open for the RC run | |
| one target per source copy, cargo home shared, `df -h /` >= 40 GB, products deleted with a manifest | M | partial | own target `trainingrooms-work`; df 197 GB / 188 GB at the builds; deletion manifest open (the target is still in use) |
| waits gated on a log line or screenshot | M | open for the RC run | |
| failed precondition aborted and retried under the next label | M | open for the RC run | |
| shortest ruleset | M | open for the RC run | |
| profile restored, temp removed, host released | M | open for the RC run | |
| console.log searched (panic/abort/crash/Unmapped); run.json, memory.peak kept | M | open for the RC run | |
| every local command under the memory and time guard | M | partial | every cargo call guarded; read-only git/rg/ssh helpers ran unguarded |
| outputs kept under the run's label | M | open for the RC run | |

### Evidence and docs
| Item | Tag | Result | Evidence |
| --- | --- | --- | --- |
| README: paragraph, full-screen GIF, install path, build commands, license, AI disclaimer | M | **fail** (compliance) → pkg7 (RC row, lead ruling) | `README has a GIF, build commands and a license line`: fail; disclaimer row passes |
| a GIF under docs/media | M | **fail** (compliance) → pkg7 (RC row) | `docs/media/*.gif: none` |
| dated evidence doc | M | pass | compliance lists the dated docs |
| numbers from committed, tested scripts | C | open | compliance `agent`: `tools/` has no `test_*.py`; applies wherever an evidence doc cites log-derived numbers |
| no operator quotes, no claims beyond the tests | M | pass for this file | |
| handoff file current | M | **fail** (compliance: `HANDOFF.md` missing) → pkg7 (RC row) | package 7 writes the plugin repo's `HANDOFF.md` (the playbook's name) with the illustrated summary; the campaign handoff `TRAININGROOMS-HANDOFF.md` stays the lead's |

### Release and card (no operator order)
| Item | Tag | Result | Evidence |
| --- | --- | --- | --- |
| version bump, annotated tag, Forgejo release, assets verified | C | not authorized | no order |
| rebuild of the tag byte-identical | C | not authorized | (docs/BUILD.md shows the build is reproducible) |
| install with preflight, rollback copy, readbacks, receipts | C | not authorized | |
| the user connects/disconnects; no mount/unmount/eject/DBI by the agent | M | pass | compliance: none in tracked scripts or the playbook's tools/switch; the integrator never touched a card |
| nothing pushed, tagged or written to the card without the order | M | pass | `git status` / no push by the integrator |

### Git and routing
| Item | Tag | Result | Evidence |
| --- | --- | --- | --- |
| Forgejo remote; commit messages from a UTF-8 file | M | pass | compliance remote row; every commit `git commit -F <file>` |
| model/effort gate and watcher | M | pass | `claude-opus-5-5` / high from the transcript; watcher in `/tmp/integrator3-watch` |
| every spawned worker read the playbook | M | N/A | the integrator spawned no worker |

## Scripts merged or written, against the playbook's tools

| Script | Status |
| --- | --- |
| `eden/ally/ally_conf.py` (new, `27d84f4`) | Not covered by the playbook: one place for the Windows Ally rig's root and per-stream folders (the playbook's eden-solo/eden-pair drive Linux Eden on the Yoga/Surface; the Ally runs the Windows Eden port under Job objects). |
| `eden/ally/ally_ops.py`, `ally_launch.py`, `ally_log.py`, `ally_route.py`, `ally_rows_mash.py`, `ally_fetch.py`, `wa_s2.py`, `wa_s5.py`, `ally-stage.ps1`, `ally-setup.ps1`, `ally-build.sh` (merged pkg2 + pkg5) | Not covered: Windows Eden on the Ally (schtasks launch, Job kill, MSI pwsh over ssh, WSL build host). Route segments follow MACROS.md's hybrid (macro + log/tile-cursor gates); the playbook's `step.ps1`/`gv.py` would replace `ally_ops.py` only on a Linux host. |
| `eden/ally/dismiss-fw.ps1` | Removed (answering the firewall prompt is operator-level). |
| Integrator helpers in `/tmp/integrator3/` (not committed): `ledger.py`, `surface_free.py`, `claim_when_free.py`, `status.py` | Not covered: the playbook's claim handling is inside `solo.ps1` for Eden runs; these write/read the same ledger for compile claims and the queue file. |
| `/tmp/integrator3/warns.py`, `classify.py`, `dupscan.py`, `resolve.py` | Not covered: Switch warning counts by kind/module, the duplicate-wrapper scan (now the committed test `tests/native_wrappers.rs`), merge-conflict resolution. |
| `tools/build/nro_size.py`, `tools/check/compliance.py` | Playbook tools, used as is. |

## Open after this record
- Gates of `73831b8` (queued), then `stream/sync` merged first when it has commits (no lock call site edited by the integrator;
  package 6 added no new `parking_lot` use: 22 files before and after).
- Dead-code pass plan sent to the lead (103 release / 122 diag warnings at `bf836c8`, by owner); nothing deleted that a running stream wires.
