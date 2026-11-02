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
| `566780a` | Merge stream/sync `db12705` (slice 1's Eden evidence, docs only) |
| `b6c509a` | 97648b8 failed `tests/capabilities.rs` (ui / named_macros reached `ui_tex_find_handle`) and `tests/carry_pkg6.rs` (seam_install.rs edited outside the manifest), both from `73831b8`: texture_probe no longer calls `texture::install()` from the layout hook (boot installs the helper once); pkg6's `cd76fad` manifest edits applied unchanged |
| `a968c05` | Merge stream/sync `dc01e5a` (hos-lock kernel-arbiter locks, `published::Live`, the menu-close hang's root fix). `compat/menu.rs`: sync's file; package 6's preload canonicalisation and roster re-plan pasted into sync's `before_publish` / `after_publish` as ui-screens sent them (no lock edit); `apply_snapshot` and `CURRENT` gone. Needs one merged-tree Eden boot before the push (menu-close cycles, a menu commit reaching the preload engine, a P1 hot swap, hook allocations 0, no `Unmapped`): asked pkg6 to run its s6 rows on this NRO |
| `73831b8` | Diag probes behind features `texture-probe` / `seam-drill`, wired to their callers; `run_buff` loses the `fighter` parameter that 1076ddc left unused (gates queued) |

## Gate evidence

| Head | cargo test | clippy -D warnings | --no-default-features | Switch release | Switch diag |
| --- | --- | --- | --- | --- | --- |
| `27d84f4` | 61 binaries, 7,697 passed, 0 failed | exit 0 | exit 0 | `99282821…` 2,879,488 B resident | `2174f5b1…` 3,100,672 B |
| `bf836c8` | 63 binaries, 7,826 passed, 0 failed (on `2bd47c2`; `bf836c8` changes diag code + one test attribute, its wrapper test re-run) | exit 0 | exit 0 | `bbe6ffceb32ff61ea2a040f0bd05bd168f6be28baa9ee61e1acc09ed421e5be1` 2,940,928 B | `6a54682d40f279095bf61cf9c1acbc7ed1c846e8992bd0fe0e46a89d1fe9fb4d` 3,162,112 B |
| `566780a` (sync `db12705` + `b6c509a` + `73831b8`) | 64 binaries, 7,836 passed, 0 failed; hos-lock 16 passed | exit 0 | exit 0 | `4bf231a2f52bd5fbda5663afd3e437817dad1fec9131ab97ea667528502d4ad6` 2,961,408 B | `3d7a92152737456a630d4ce31fdc7ca0da308759b1bbc17ca891520926d45001` 3,194,880 B |

Each cargo call: `systemd-run --user --scope -p MemoryMax=6G /usr/bin/timeout 1800`, `CARGO_TERM_COLOR=never`, one call each; Switch
builds with the campaign rustup home (docs/BUILD.md). Logs kept under `/tmp/integrator3/` (test.log, test2.log, clippy*.log,
nodef*.log, sw-*.log). Surface claims and releases: ledger lines 10:53/10:55Z, 10:59/11:01Z, 11:18/11:22Z, 11:28/11:30Z.

## After 566780a (2026-11-02, 12:4x-13:3xZ)

| Commit | What |
| --- | --- |
| `50e9621`, `a9d843c` | Merge stream/stack aa18732 / f3b89c6 (static thread stacks from one table, trace-writer crash fix, boot sections out of the boot frame) |
| `ff1c62c`, `1c547e6` | Install line (lead ruling b2): fixed buffer of the computed worst case (`src/install_capacity.rs`), boot heap outside hooks, freed after the write; `result=line_overflow`; host tests incl. no reallocation; MAX_DRILL_FORM under `seam-drill` |
| `48eebe1` | Merge stream/pkg6 5e0fab3 (practice::panel_entry for pkg4, Solo Popo rows, R5 attribution scopes) |
| `a03529f` | Merge stream/sync d0ad5e7 (slice 2; pkg4 ledger regenerated, verify 42/42) |
| `e9da9de` | Merge stream/pkg4 4286dc8 (R5 macro-layer fixes, exact byte-meter test, SLACK gone (it came with pkg6 a307289); manifest = both sides' named_macros edits, full verify 42/42 from stage m5q) |
| `299df2e` | Merge stream/u4b 02aa91e (DI/SDI chips from the game's effect textures) |
| `5b471a6` | Unused imports the merges left in runtime-only files (Snapshot x3, settings::self) |
| `dc8ada6` | Merge stream/stack2 0e5fa23 (roster re-plan as a menu-worker event; boot::run no longer reaches the planner; table statics) |
| `8b73694` | tests/hook_locks.rs entry name `import::start_background` |

Gates:
- `48eebe1`: cargo test 65 binaries, 8,019 passed, 0 failed; clippy 0; no-default-features ok; Switch release
  `9c399f4bf15f7efa89858a28c8cc5f91372f34fed7fe3844b6df71ac5080940c` 3,317,760 B resident (bss 413,696: static thread slots), diag
  `a3bcb55e2fe43b31bc67cc29705f20f7bd7619188ac151872cd8b601cf2f0e83` 3,715,072 B; warnings 107 / 96. stack_depth.py: budget 0 over;
  **entry boot::run 43,792 B > 39,321 (fixed by stack2)**; **table: 4 drifted rows (refreshed by stack2)**.
- `dc8ada6`: cargo test 67 binaries, 8,907 passed, **2 failed (tests/hook_locks.rs)**; clippy 0; no-default-features ok. Name drift fixed in
  `8b73694`; the remaining violations (plugin threads reaching EDITED, notices QUEUE, chords STATE, recorder REC) go to ui-screens as sync
  slice 3 (lead). Switch builds and stack_depth wait for the next head that passes host gates.
- ui-screens slice 1 `1992045`: merge stopped and aborted (port.rs / replay_capture.rs: pkg4's retirement of the port shims vs
  ui-screens' design-C rewrite of the same shims); proposal to the lead: ui-screens merges work/implementation and reconciles with pkg4.

## Merged-tree Eden boot s6-05 (run by pkg6 A on the 566780a diag NRO `3d7a9215`, Surface 12:30-12:37Z)
Logs and shots: stream/pkg6 `eden/runs/s6-05`. Pass: cold boot (install line ~23 s after game.load, main menu ~40 s after, no
plugin-silent stall); install line 9,207 chars ending `result=installed heap_allocations=0`; roster started; diag settings reload
with periodic lines continuing; `[CyclePreload] applied` through `before_publish`; P1 hot swap (Kirby); record (50 frames) and
playback; menu open/close x3 with the game running; console.log 12,414 lines, 0 panic/abort/crash/Unmapped.
**Fail (R5):** hook_stats allocations rise ~180-210 per 10 s without a swap (2,302 at the end), as f6 saw on a pre-sync tree:
the "no heap in hooks" item stays **open** until the allocating hook is found (pkg6's attribution scopes) and fixed at its root.

## Checklist (docs/CHECKLIST.md) for the integrated tree

`compliance.py <work tree> --nro <release NRO of bf836c8>`: machine rows below; the four failing rows are named exactly.

### Implementation
| Item | Tag | Result | Evidence |
| --- | --- | --- | --- |
| crate-type cdylib only | M | pass | compliance: `Cargo.toml crate-type = ['cdylib']` |
| release profile s/lto/cgu 1/abort; skyline =0.6.0; toolchain 1.98.1 | M | pass | compliance rows (three) |
| pure logic in model with host tests; runtime only under cfg(switch) | M | pass | compliance: `src/lib.rs`; 7,826 host tests |
| gates, one cargo call each | M | pass for `bf836c8`; open for `73831b8` | table above |
| no heap in hooks: counting allocator, `heap_allocations=0` in the install line | M | **fail at run time** (s6-05: hook_stats +180-210 per 10 s); install line `heap_allocations=0` at boot | compliance allocator row; `tests/hook_alloc.rs`; no Eden boot of the integrated NRO yet (the line must read `heap_allocations=0`) |
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

## Dead-code warnings (plan; nothing deleted that a running stream wires)
At `566780a`: 105 release / 94 diag (`/tmp/integrator3/warns-*3.tsv`, `classify.py`). Owners: pkg4 port and replay stubs (~35),
pkg6 carried Mii/roster (~46) and lobby (~11), UI screens (`menu_app.rs` 11), one unused parameter (fixed in `73831b8`).
**Kept until ui-screens merges (lead 2026-11-02):** these pkg4 screen entry points stay dead until a screen calls them; at RC
each is called by a screen or deleted:
- `named_macros`: panel_view, on_pad, open_library, export_record_slot, offer_external_name, open_external_library,
  capture_active, playback_active, using_frames, has_captured_frame, frame_at, mii_at_lockout, open_editor, take_confirmed_name,
  take_list_action, take_list_choice, external_name_open, set_ui_ready, menu_context / MenuContext.
- `replay_capture`: open_editor, browser_view, browser_input, loading_view, cancel_reload, clip_view, owns_close_input, on_pad,
  scrub_pace, after_draw.
- `port`: export_named, frame_of, refusal_text.
- `replay_preview`: recovery_available, status.
The push waits until the warnings are gone (lead); what remains at each gate is listed here.

## Open after this record
- Gates of `73831b8` (queued), then `stream/sync` merged first when it has commits (no lock call site edited by the integrator;
  package 6 added no new `parking_lot` use: 22 files before and after).
- Dead-code pass plan sent to the lead (103 release / 122 diag warnings at `bf836c8`, by owner); nothing deleted that a running stream wires.

## Handover (2026-11-02 ~14:05Z, operator order: the lead role moves to a Codex GPT-6 Astra lead)

**Head.** `work/implementation` = `21334c0` (+ this docs commit), local only, never pushed; work tree clean; no claim held (last
RELEASED 14:00Z), no queue line. It contains, on top of `27d84f4` (pkg2): pkg6 `98d6f5b` and `5e0fab3`, one wrapper per native
(`2bd47c2`), diag probe features (`73831b8`, `b6c509a`), stream/sync slices 1-3 (`db12705`, `d0ad5e7`, via ui-screens `71ba1f7`), stack
`f3b89c6` + stack2 `0e5fa23`, the install line (b2: `ff1c62c`, `1c547e6`), pkg4 `4286dc8`, u4b `02aa91e`, ui-screens slice 1 `71ba1f7`
(reconciled with pkg4's providers + the facts rule), unused-import fix `5b471a6`.

**Gate status of 21334c0.**
- Host: cargo test 67 binaries, 9,253 passed, 0 failed (hook_locks 3/3 with one PENDING row: recorder REC, pkg4 design C);
  clippy --all-targets -D warnings 0; --no-default-features ok; crates/smash-match-data 25/0; crates/hos-lock 20/0 (14:00Z, Surface).
- **Not run on 21334c0:** Switch release + diag, `tools/stack_depth.py` table / budget / entry boot::run (<= 39,321 B), compliance with
  the release NRO, Switch warning count. Last Switch numbers are at `48eebe1`: release `9c399f4b` 3,317,760 B resident, diag
  `a3bcb55e` 3,715,072 B; warnings 107 release / 96 diag (5 of the new ones fixed since in `5b471a6` / used since `e9da9de`);
  stack_depth at 48eebe1: budget 0 over, boot::run 43,792 B (over; stack2's fix merged since, unmeasured), 4 drifted table rows
  (refreshed by stack2, unmeasured).
- sloc4 (`/home/intpa/smash-training-lobby/tools/sloc4.py git <work tree> 21334c0 1`): **106,952** implementation lines (src 102,496,
  crates 4,305, tools 151; tests excluded 28,672; host-only simulator 1,118).
- Compliance machine rows: the four open rows (guard-checker limit `Site::EMPTY`; README GIF/build/licence, docs/media GIF,
  HANDOFF.md -> pkg7).

**Known failures / open.** R5 hook allocations (s6-05: +180-210 / 10 s without a swap; pkg6 owns the fix; push held); the hook_locks
PENDING row (recorder REC: pkg4 design C); dead-code warnings (push held until gone; pkg4 screen entry points listed above stay until
called); boot::run depth and the thread table to be re-measured on the next Switch build.

**Merge queue, in order (none started):**
1. pkg6 R5 fix (priority) when pkg6 gates it: stream/pkg6 head now `d517dab` (WIP F9/F11 `d7b89a4`/`03036b7` and later: ask pkg6 for the
   gated sha). After it lands with ui-screens (already in), send pkg4 (`a93ab192186b224a5`) the work sha so design C resumes.
2. deluxe `e4432ad` (macros/mod.rs register, features/writer.rs, boot.rs install-line field, tools/feature_matrix.py; small conflicts
   expected). Then tell the lead to remove its worktree.
3. hitbox (stream/hitbox head `7131dba`; confirm the gated sha with hitbox).
4. slim (stream/slim head `cf6004d`; brings tests/size_budget.rs + size-budget.toml: from then on set the ceiling to each merged head's
   sloc4 count and drop that stream's pending item).
5. harness (stream/harness head `6ce55ef`; replaces the ledger helpers with one module that accepts `HH:MM:SSZ`).
6. pbaudit `0275f27` (docs only).

**Next commands** (work tree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-work`; target
`/home/intpa/smash-training-lobby/cargo-targets/trainingrooms-work`; every cargo call `CARGO_INCREMENTAL=0 CARGO_TERM_COLOR=never` under
`systemd-run --user --scope -p MemoryMax=6G /usr/bin/timeout 1800`, one call each, after a ledger + queue check incl. the interleave
owners imgui-nvn-sol, tm-f8-terra*, tm-f8-sol*):
- Switch release then diag of the head (docs/BUILD.md, campaign rustup home), keep the diag
  `aarch64-skyline-switch/release/libtraining_rooms.so`; then `python3 -I tools/stack_depth.py <so> table`, `... budget`,
  `... entry '^training_rooms::runtime::boot::run$'` (<= 39,321 B), `python3 -I ~/ssbu-mod-playbook/tools/build/nro_size.py <nro>`,
  `python3 -I ~/ssbu-mod-playbook/tools/check/compliance.py <work tree> --nro <release nro>`.
- `git merge --no-ff --no-commit <sha>`; carried files: `python3 -I tools/carry_pkg4.py verify` / `ledger` and tests/carry_pkg6.rs for
  manifest conflicts (never edit a carried file without its manifest edit); host gates incl. `cargo test --manifest-path
  crates/smash-match-data/Cargo.toml` and `crates/hos-lock/Cargo.toml` (delete the Cargo.lock those runs create in the crate dirs).
- After the gates: delete `.../trainingrooms-work/debug/incremental`.


## Lead 4 development integration: declaration slices (2026-11-02)

Adopted only src/model/hook_stats.rs and src/model/sites.rs from fixed slim a6e4d07
(the HookId slice8f762e1 and Feature slicea6e4d07). Both pre-change files matched
integration cb0e3b4 exactly. Independent review/probe confirms all40 HookId
ordinals/report labels and all28 Feature ordinals/labels are unchanged; all code
below those declarations is byte-identical, including allocator behavior and site
guards. Raw review receipt: ../artifacts/trainingrooms-lead-astra4/slim-declarations-review.json.

Audited archived a6 host/clippy/no-default exits0, completed-log hashes and all1110
before/after source hashes. The two adopted files match those archived hashes.
No full-suite arithmetic claimed: the supplied host command is test --no-fail-fast,
not the separately listed all-targets proof required by bounded-validation-evidence.
This is a development source integration; integrated compiler/stack/size/runtime
acceptance remains pending with the composed candidate. No new budget, suppression,
thread allocation exemption or runtime pass. The four OS-TLS cells below the HookId
table remain an open root explicitly assigned to UI; the table change does not fix them.
