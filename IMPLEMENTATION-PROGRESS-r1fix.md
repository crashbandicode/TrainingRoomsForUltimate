# r1fix stream: the five findings of the Astra review of package 1

Stream `r1fix` (claude-sonnet-5-5, high), lead `0caa72b8`. Worktree
`/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-r1fix`, branch `stream/r1fix`, base `4895ea3`, merged with
`work/implementation` `c5863bd` (merge `ffc93e4`). Review: `artifacts/trainingrooms-reviews/R1-astra-pkg1/REVIEW.md`.

## Commits (one per finding; gates green after each)

| Finding | Commit |
| --- | --- |
| R4 developer.ini fails closed | `2b7b00a` |
| R3 delay 0 | `949a373` |
| R1 settings recovery transaction | `a449e33` |
| R5 diag hook observation | `aa69d2b` |
| R2 native pause owner (first attempt: scene `StatePause` hooks) | `009f6c8` |
| scene wrapper consumers (lead order) | `ba67245` |
| merge of work/implementation and reconciliation | `ffc93e4` |
| R2 by the lead ruling (UI flag producer; `StatePause` hooks removed) | `e2effe9` |

Spec text on `review/menu-proposal` (docs only, not pushed): `251fec2` (first), `c3c8a7b` (Eden correction), `9fe08d0` (the
`L_staying_help` flag as the authoritative source).

## Where things are

- R1 `src/model/settings_file.rs` (runtime keeps `boot::load_settings`); one transactional write: `Store::write_replace`.
- R2 `InputAdapter::pause_flag` / `pause_begin` / `pause_end` / `poll` in `src/model/input_adapter.rs`; the runtime calls
  them from `features/input_delay.rs` (poll, `pause_flag`) and from the UI tick (`runtime/ui/tick.rs`, the edge of
  `PAUSE_ACTIVE`). `published::input_owners()` is kept for the other packages.
- R3 `DelayQueue::step` step 0.
- R4 `src/model/developer_ini.rs` (`load`, `read`, `parse`, `Reason`); install line `developer=on|off reason=...`.
- R5 `src/model/hook_stats.rs` (allocator, `HookScope::enter()` unnamed / `enter_hook(HookId)` named, `Observer`), diag line
  from the heartbeat thread in `src/runtime/diag_log.rs`; gate `eden/hook_stats.py`.

## Gates, builds, Eden

Host gates: `$env:CARGO_TARGET_DIR='/home/intpa/smash-training-lobby/cargo-targets/trainingrooms-r1fix'`, one cargo call each
under `systemd-run --user --scope -p MemoryMax=6G /usr/bin/timeout 1800`. All test binaries pass except `tests/input_sites.rs`
`every_row_matches_the_owned_elf` (pre-existing on work: `work_get_int` is a row of both `sites_pkg2` and `sites_pkg4`; the
wire stream fixes it). Switch builds on the Surface with the campaign rustup home (docs/BUILD.md). NROs (gitignored `dist/`):
release `11f74916...` 778,240 B resident, diag `9c4ea584...` 933,888 B.
Eden: Yoga, `/home/intpa/trainingrooms-eden-r1`, profile `r1`, display `:98`, labels `r1-NN`, results in
`eden/EDEN-PACKAGE-1.md` ("R1-fix").

## Open

- Nothing from the review. Not done here: `tools/check/compliance.py` (no READY claimed).

## Playbook record (retroactive audit, 2026-11-02, pbaudit, playbook ae87443)

Stream `r1fix` (the five Astra review findings of package 1, R1-R5; transcript `agent-a7d59e7cc96ddc626.jsonl`, claude-sonnet-5-5 high). The progress file records no playbook commit and says "Not done here: `tools/check/compliance.py`". Code in scope: `src/model/{settings_file,developer_ini,hook_stats,input_adapter}.rs`, `Store::write_replace`, `DelayQueue::step`, `src/runtime/diag_log.rs`, `tests/{core04_recovery,hook_alloc,input_pause}.rs`, `eden/{lanes_r1,hook_stats}.py`, `eden/tr_eden.r1fix.config.json`.

### Reading evidence (transcript audit)

**Read: no evidence of reading.** The transcript has no `Read`, `Get-Content`, `cat` or `rg` of `README.md`, any file in `docs/`, or `tools/PROVENANCE.md`, and no `git pull` of the playbook. It did read the stream rules (04:28:27Z, which at that moment contained "Playbook read in full" in its First list) and `MY_MODEL.md` (04:28:31Z). Playbook tools it did use: `sed -n 1,40p tools/build/scan-offsets.py` (04:48:57Z), `scan-offsets.py` (04:49:05Z) and `tools/build/nro_size.py` (04:51:45Z). Its Eden harness is `eden/tr_eden.py` driven by `tr_eden.r1fix.config.json`; that file's header says it was adapted from the playbook's `gv.py` + `solo.ps1` at `53fd1f2` by an earlier stream, so the stream inherited playbook behaviour through code, not through reading. It never ran `compliance.py` (no match in the transcript).

### Checklist (`docs/CHECKLIST.md`, M and C items applied to this stream's code as merged in `bf836c8`)

Result vocabulary: `pass`, `fail`, `N/A: reason`, `not authorized`, and `open` (no evidence yet: not a pass). Tree-level rows cite the audit's compliance run: `python3 -I ~/ssbu-mod-playbook/tools/check/compliance.py <pbaudit worktree at bf836c8> --nro cargo-targets/integ3-nro/libtraining_rooms-release.nro` (report: `docs/pbaudit/compliance-bf836c8.md`). The audit did not rerun gates or Eden; evidence quoted from the stream's own files is marked as such, evidence marked "audit" was checked by the audit itself.

| # | Requirement | Tag | Result | Evidence |
| --- | --- | --- | --- | --- |
| 1 | Implementation: `crate-type = ["cdylib"]` only | M | pass | tree-level: `docs/pbaudit/compliance-bf836c8.md` row 1 (`Cargo.toml crate-type = ['cdylib']`) |
| 2 | Implementation: release profile `opt-level "s"`, lto, cgu 1, panic abort; `skyline = "=0.6.0"`; toolchain 1.98.1 | M | pass | tree-level: `docs/pbaudit/compliance-bf836c8.md` rows 2, 3, 5 (profile as required, `skyline = "=0.6.0"`, toolchain 1.98.1) |
| 3 | Implementation: pure logic in `model` with host tests; `runtime` only under `cfg(target_os = "switch")` | M | pass | `docs/pbaudit/compliance-bf836c8.md`; r1fix host tests `tests/core04_recovery.rs`, `tests/core04_settings.rs`, `tests/hook_alloc.rs`, `tests/input_pause.rs`, `tests/input_delay.rs` |
| 4 | Implementation: gates, one cargo call each (test, clippy -D warnings, no-default-features, skyline release, diag build) | M | pass | `IMPLEMENTATION-PROGRESS-r1fix.md` "Gates": one commit per finding with gates green, one cargo call each under `systemd-run ... MemoryMax=6G timeout 1800`, Switch release `11f74916...` (778,240 B resident) and diag `9c4ea584...` (933,888 B). At stream close one pre-existing failure was recorded (`input_sites::every_row_matches_the_owned_elf`, `work_get_int` row duplicated by pkg2/pkg4; fixed by wire `5cfc546`). Integrated tree at `bf836c8`: integrator-3 (7,826 passed, 0 failed) |
| 5 | Implementation: no heap in hooks (counting allocator, `heap_allocations=0` in the install line) | M | pass | R5 is this stream's delivery: `model::hook_stats` (allocator, `HookScope`, `Observer`), diag line from the heartbeat, `tests/hook_alloc.rs`, gate `eden/hook_stats.py`. Eden r1-09: `hook_stats allocations=0 delta=0 hook_calls=27834`; install lines end `heap_allocations=0` (`eden/EDEN-PACKAGE-1.md` "R1-fix") |
| 6 | Implementation: fixed buffers, writer thread on a static stack, stack-built paths | M | open | R1's transactional write (`Store::write_replace`, atomic replace and copy-aside) runs on the settings loader thread (`std::thread::Builder`, `settings::LOADER_STACK_BYTES`, `src/runtime/boot.rs:535`), not a static stack; the stream makes no claim for this row |
| 7 | Implementation: root cause, or each workaround (guard, skip, restriction, tolerance, lint `allow`, feature gate, setting off) approved and listed with its date | M | pass | five findings fixed at the root, one commit each. R2: the first attempt (scene `StatePause` hooks) was removed after Eden r1-04..r1-06 showed Training's pause menu never runs `StatePause`; the lead-ruled UI-flag producer replaced it (`e2effe9`). No lint `allow` is attributable to r1fix (audit: `git blame` of every `#[allow]` in `src/`) |
| 8 | Hook safety: version guard (`result=unsupported_version`) | M | pass | tree-level: `docs/pbaudit/compliance-bf836c8.md` row "13.0.5 in model" (`src/model/guard.rs`, `install_line.rs`, `archive_recipe.rs`); `tests/core01.rs:70` asserts `result=unsupported_version` |
| 9 | Hook safety: 64-byte guard on every hooked, called and relied-on site | M | fail | tree-level checker limit; r1fix evidence: its pause/UI-flag work adds no new guarded site beyond the input-mapping rows that `tests/input_sites.rs` compares with the owned ELF; `tests/core01.rs core01_sites_match_owned_elf`, `tests/input_sites.rs`, `tests/pkg6_sites.rs every_row_matches_the_owned_elf`, `tests/cpu_common.rs cpu_common_guards_match_the_owned_module` (all pass in integrator-3's run: 7,826 passed, 0 failed) |
| 10 | Hook safety: card scan (`scan-offsets.py`) of the site over a dated card copy | M | pass | audit (transcript 04:49:05Z): `python3 -I ~/ssbu-mod-playbook/tools/build/scan-offsets.py` over the 2026-11-01 10:19 card copy for the pause site `0x1750dd0`: only `libtraining_modpack.nro` (the fork) references it (`const32/const64/movz-movk`), all 15 other plugins "no reference" |
| 11 | Hook safety: sharing / chaining when another plugin may hook the site (both stand-in modes, both load orders in Eden) | C | N/A: scan shows only the fork | the fork is replaced by this plugin; the CPU pilot loaded first in every Eden run (`eden/EDEN-PACKAGE-1.md` R1-fix) and coexisted |
| 12 | Hook safety: hook after the prologue, or call the function instead | M | N/A: no hook site added | the first-attempt `StatePause` hooks were removed; R2 by the UI flag adds none; R5 observes existing hooks |
| 13 | Hook safety: no diag code or strings in the shipped NRO | M | pass | tree-level: `docs/pbaudit/compliance-bf836c8.md` "libtraining_rooms-release.nro: no diag markers" (sha256 bbe6ffceb32ff61e, `cargo-targets/integ3-nro/libtraining_rooms-release.nro`) |
| 14 | Hook safety: the shipped NRO writes its install line and nothing per frame | M | open | r1-01, r1-02, r1-07, r1-08 and r1-09 ran the release build for the settings rows and the install line was read, but no `events.log` of a release boot was checked for "nothing but the install line" in the doc; the doc shows `hub_run` is diag-only (package 1 section, "release has no `hub_run`") |
| 15 | Online: read-only rules reviewed hook by hook | C | open | R2 changes who owns input during the native pause (`PAUSE_ACTIVE`, `input_owners`); no read-only/online review of the input-delay hook or of the pause producer is recorded; applies if the Training pause can occur in lobby practice |
| 16 | Online: replay A/B with the desync detector identical | C | open | not run |
| 17 | Online: two consoles past the changed path, `desync_seen=0` at the same `frame=` | C | open | not run |
| 18 | Eden: claim written before Up and released after Down, also after a failed run | M | pass | Yoga ledger `/tmp/deluxe-surface-1007/OWNER-COORDINATION.md` (copied read-only by the audit): owner `0caa72b8/r1fix-eden` 9 CLAIMED / 12 RELEASED, none open; the 3 surplus RELEASED lines (07:29Z, r1-07..r1-09) are explained in `eden/EDEN-PACKAGE-1.md` ("a first attempt of the merged batch failed on a script argument bug before any Eden started") |
| 19 | Eden: one Eden per host, MemAvailable >= 10 GiB, never profile `a`, display `:61` or another agent's runs | M | pass | own profile `r1`, display `:98`, work root `trainingrooms-eden-r1` (`eden/EDEN-PACKAGE-1.md` R1-fix); harness asserts MemAvailable >= 10 GiB (`eden/tr_eden.py:269`) |
| 20 | Eden: profile is a copy with hard-linked game content; identical files linked, written files copied | M | pass | profile `r1` is a hard-linked copy of `ssbu-desync-1029/profiles/b` (`tr_eden.py prep`, `os.link`); never `a` |
| 21 | Disk: one `CARGO_TARGET_DIR` per source copy, cargo home shared, `df` >= 40 GB, finished products deleted with a manifest | M | open | own target `cargo-targets/trainingrooms-r1fix` (not present at audit) and the Surface build copy; no `df` check or deletion manifest recorded in the stream files |
| 22 | Eden: every wait gated on a log line or screenshot check | M | pass | `eden/route4.py`-derived route (every wait gated on a log line or a reference-screenshot match, with caps) and `eden/lanes_r1.py` (13 gate references); the lane's own FAIL on log rotation was fixed and documented |
| 23 | Eden: a run whose precondition line never comes is aborted and retried under the next label | M | pass | the first merged batch failed before any Eden started (script bug) and was rerun as r1-07..r1-09 (`eden/EDEN-PACKAGE-1.md`) |
| 24 | Eden: shortest ruleset (1 stock, 1:00, items/hazards/FS meter off) | M | N/A: Training mode | all r1fix rows run in Training; no match played |
| 25 | Eden: profile restored after the run (set-aside NROs back, temp files removed, host released) | M | open | each Down printed `alive: false`, `left: []`; profile `r1` keeps the three boots' data by design; a `restore` of the plugin set is not in the transcript |
| 26 | Eden: `console.log` searched for panic/abort/crash/Unmapped (fail below 0x1000 or in the plugin's window); `run.json`, `memory.peak` kept | M | pass | audit: Yoga `runs/r1-01..r1-09` `console.log`: 0 matching lines in all nine; `run.json` and `memory.peak` present for every run (3.4-6.9 GB) |
| 27 | Every local command under the memory and time guard | M | pass | cargo calls in the transcript go through `systemd-run --user --scope -p MemoryMax=6G /usr/bin/timeout 1800` (progress "Gates"); audit heuristic 26 of 26 cargo invocations via a guard wrapper |
| 28 | Each run's outputs kept under its label | M | pass | `/home/intpa/trainingrooms-eden-r1/runs/r1-NN/` (audit listing) |
| 29 | Docs: README (paragraph, full-screen GIF, install path, build commands, license, AI disclaimer) | M | fail | tree-level: `docs/pbaudit/compliance-bf836c8.md` rows "README has a GIF, build commands and a license line" and "a GIF under docs/media" fail (the AI-disclaimer row passes); assigned to package 7 by the lead (integrator-3 record), not this stream's |
| 30 | Docs: a dated evidence doc (run labels, build, what was done, what the log showed) | M | pass | `eden/EDEN-PACKAGE-1.md` section "R1-fix" (labels, builds with sha256 and resident sizes by `nro_size.py`, per-row evidence) |
| 31 | Docs: numbers cited from logs come from a committed, host-tested script | C | pass | onset and delay numbers come from `eden/analyze_probe.py` / `analyze_step.py`, tested by `tests/eden/test_analyze.py` through `tests/eden_scripts.rs` (planted 5-frame shift detected, no-shift not); the R5 gate is `eden/hook_stats.py` |
| 32 | Docs: no operator quotes, no claims beyond the tests | M | pass | the doc records its own gate FAILs (lane on log rotation, grey-difference gate under hazards) and the rows judged by eye; the write-failure and no-free-backup paths are stated as host-tested only |
| 33 | Docs: handoff file current (branch, commit, evidence, live state, next commands) | M | fail | tree-level: `docs/pbaudit/compliance-bf836c8.md` `HANDOFF.md` missing; the stream state is in this file and the campaign handoff is `TRAININGROOMS-HANDOFF.md`, neither is the plugin repo's `HANDOFF.md`; package 7 per integrator-3 |
| 34 | Release: minor bump, annotated tag, Forgejo release, assets verified | C | not authorized | no operator order |
| 35 | Release: rebuild of the tag byte-identical to the released NRO | C | not authorized | no operator order |
| 36 | Card: DBI MTP preflight, rollback copy, readbacks, receipts | C | not authorized | no operator order |
| 37 | Card: the user connects and disconnects; the agent never mounts, unmounts, ejects or remounts, nor starts or stops DBI | M | pass | `docs/pbaudit/compliance-bf836c8.md` row "no script mounts, unmounts or ejects the card" passes; transcript shows no `gio`/`udisksctl`/DBI/`tools/switch` command; ledger lines say "No physical Switch writes" |
| 38 | Without the order: nothing pushed, tagged or written to the card | M | pass | transcript shows no `git push` or `git tag` and no card write (`rg` over the stream's PowerShell commands) |
| 39 | Git: Forgejo, never GitHub; commit messages from a UTF-8 file | M | fail | remote is Forgejo (compliance pass). Commit messages: transcript heuristic 12 `git commit -F` and 7 single-line `git commit -m` |
| 40 | Routing: model/effort gate and per-thread watcher | M | fail | read `MY_MODEL.md` at 04:28:31Z; no watcher is created anywhere in the transcript (no `watch` string) |
| 41 | Routing: every spawned worker read this playbook itself | M | N/A | the stream spawned no worker (no Agent call in its transcript) |

Counts: 22 pass, 5 fail (#9, #29, #33, #39, #40), 7 open (#6, #14, #15, #16, #17, #21, #25), 4 N/A, 3 not authorized.

Finding for the lead: the stream closed with "Not done here: compliance.py", so its code was first checked by the tool only in integrator-3's run; every row above that is `open` could have been settled by the stream in minutes.
