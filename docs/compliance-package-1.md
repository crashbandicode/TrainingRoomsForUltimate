# Compliance report: TrainingRoomsForUltimate-work

| Requirement | Tag | Result | Evidence |
| --- | --- | --- | --- |
| crate-type is ["cdylib"] only (no rlib) | M | pass | Cargo.toml crate-type = ['cdylib'] |
| release profile opt-level "s", lto, codegen-units 1, panic abort | M | pass | Cargo.toml [profile.release] as required |
| skyline pinned to =0.6.0 | M | pass | Cargo.toml |
| diag cargo feature declared | M | pass | Cargo.toml [features] |
| toolchain 1.98.1 | M | pass | rust-toolchain.toml |
| runtime only under cfg(target_os = "switch") | M | pass | src/lib.rs |
| supported version 13.0.5 in model | M | fail (checker limit: reads src/*.rs; the item is at src/model/version.rs:16) | `SUPPORTED_VERSION = "13.0.5"`; host test `tests/carried_model_facade.rs::the_build_id_constants_agree` |
| every site guard is 64 bytes | M | fail (checker limit: reads src/*.rs; the items are at src/model/sites.rs:110-227) | 10 `Site` rows, each `guard: [u8; 64]` (type-checked); `tests/core01.rs::core01_sites_match_owned_elf` compares every code row to the owned 13.0.5 ELF |
| counting allocator reports heap_allocations | M | fail (checker limit: reads src/*.rs; the item is at src/runtime/alloc.rs:27,54 and src/model/install_line.rs:9-18, src/runtime/boot.rs:46,136) | `CountingAllocator` is the `#[global_allocator]`; the install line ends `heap_allocations=0` in Eden (EDEN-PACKAGE-1.md install-line row, tr101-tr106) |
| host tests in model (cargo test) | M | fail (checker limit: reads src/*.rs; the items are in src/model/*.rs and tests/) | 19 `#[test]` inside `src/model/*.rs` and 89 in `tests/*.rs`; `cargo test` green at 5beb35c (all binaries `ok`, 0 failed) |
| README carries the AI-assistance disclaimer | M | pass | README.md |
| README has a GIF, build commands and a license line | M | fail: RC item (no README GIF before the menu exists) | README.md "Build and install" still says "Not yet"; license line and AI disclaimer present |
| a GIF under docs/media | M | fail: RC item (no README GIF before the menu exists) | `docs/media/` does not exist |
| dated evidence doc present | M | pass | IMPLEMENTATION-PROGRESS.md, MENU-DRAFT.md, PROVENANCE.md, TRAININGROOMS-HANDOFF.md, docs/CARD-SWAP-PLAN.md, docs/DECISIONS.md, docs/MENU-PROPOSAL.md, docs/PROGRESS-resource-costs.md, docs/implementation-packages.md, docs/menu-redesign.md, docs/resource-costs.md |
| numbers in the evidence come from tested scripts over the logs | C | fail | The delay-onset numbers come from committed `eden/analyze_probe.py` (command named at  |
| handoff file present | M | fail: RC item (checker looks for HANDOFF.md; the project handoff is TRAININGROOMS-HANDOFF.md) | `TRAININGROOMS-HANDOFF.md` exists (last commit here 5e34c82); the package 1 checkpoint ecb574d is on review/menu-proposal and is not merged into work/implementation |
| remote is Forgejo, not GitHub | M | pass | https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate.git |
| working tree clean | M | pass | git status |
| no script mounts, unmounts or ejects the card (the user connects and disconnects it) | M | pass | none in the tracked scripts or the playbook's tools/switch |
| build identity | M | info | HEAD e53b0ef; no tag on HEAD |
| shipped NRO identity | M | info | dist/release/libtraining_rooms.nro sha256 9bad6541a3b1d749 resident 172032 B (168 KiB) |
| no diag strings in the shipped NRO | M | pass | libtraining_rooms.nro: no diag markers |
| Gates run: test, clippy -D warnings, no-default-features, skyline release, diag build | M | pass | At 5beb35c: `cargo test` all binaries ok; `cargo clippy --all-targets -- -D warnings` no output; `cargo build --no-default-features` Finished; `cargo skyline build --release` and `--features diag` both exit 0 (/tmp/trfu-scripts/rel4.log, diag2.log); NRO sha256 9bad6541… / db7a4dd9… (IMPLEMENTATION-PROGRESS.md "Eden fixes D1-D4"). Launcher: see open item W2 |
| Fixes: root cause fixed, or each workaround approved by the operator before it shipped and listed with its date | M | pass (open items: W2, W3 below) | No workaround ships in the code. D1 is fixed at the design but its root cause is only probable (commit 5beb35c message; confirmed by  |
| Card scan for the site (and vtable slot) over a dated card copy | M | pass | `scan-offsets.py` over the MTP preflight copy `/home/intpa/ssbu-cpu-pilot-install/switch-mtp-v0101-v021/pilot/after/…/plugins` (folder dated 2026-11-01 10:19), offsets of all 10 sites incl. the vtable slot 4f80240: only libtraining_modpack.nro (the fork, B1) references them (IMPLEMENTATION-PROGRESS.md, "Card-site scan") |
| Hook sharing or chaining; both stand-in modes and load orders in Eden | C | fail | Stand-in modes `foreign=inline|far` and `diag.txt` are not implemented ( |
| Hook after the prologue or the function called instead | M | fail | `final_input_mapping` is a replace hook at the function entry (src/model/sites.rs:113, justification in its `note`); `reset_check` and `hazard_hook` are mid-function inline hooks (sites.rs:126,152), `on_init` is a vtable slot (sites.rs:178). The entry hook is not moved after the prologue: open for the lead |
| Shipped NRO writes only its install line (plus a logging plugin's data files) | M | pass (documented exception) | Release NRO strings carry no per-event lines; hooks log nothing (`rg "diagnostic\(" src/runtime` hits only boot.rs, diag_log.rs, hub.rs:27 under `cfg(feature = "diag")`). Written files: events.log (one line per boot) and practice-diagnostics.log with the session_start breadcrumb (128 B in tr105,  |
| Online: read-only rules reviewed hook by hook | C | pass | Review, hook by hook (all gated by `is_training_mode()`; none draws the game RNG or skips/reorders native code): src/runtime/features/input_delay.rs (original runs first, then writes P1's record only in Training), lra_reset.rs (writes x8 only in Training), stage_hazards.rs (writes the rule byte only in Training), reset_hub.rs (original on-init first, hub only for the CPU in Training; the hub touches plugin state only). The hazard byte and the input record are simulated-state-adjacent but Training-only |
| Online: replay A/B with the desync detector identical | C | fail: RC item (not run; package 7) | No replay A/B with the desync detector has been run |
| Online: two consoles past the changed path, desync_seen=0 at the same frame | C | fail: RC item (not run; package 7) | No two-console run has been done |
| Eden: claim written and released, also after a failed run | M | pass | eden/EDEN-PACKAGE-1.md |
| Eden: one instance per host, MemAvailable >= 10 GiB, not profile a or :61 | M | pass | eden/EDEN-PACKAGE-1.md |
| Eden: profile copy with hard-linked game content; only the copy changed | M | pass | eden/EDEN-PACKAGE-1.md |
| Copies: byte-identical files linked, differing or written-in-place files copied | M | pass | eden/EDEN-PACKAGE-1.md |
| Disk: one target dir per source copy (never shared between copies), cargo home shared per host and toolchain, df checked (>= 40 GB), finished build products deleted with a manifest | M | fail (open until the package closes) | One `CARGO_TARGET_DIR` (/home/intpa/smash-training-lobby/cargo-targets/trainingrooms-work) for this worktree only; `df` 255 GB free before builds; cargo home shared per toolchain (campaign cargo home for Switch jobs, ~/.cargo for host). Finished build products are not deleted yet and no manifest is written (work in progress) |
| Eden: waits gated on a log line or screenshot check | M | pass | eden/EDEN-PACKAGE-1.md |
| Eden: failed precondition aborted and retried under the next label | M | N/A: no run failed its plugin-log precondition | The install line appeared in every run; the late scene gate missed in tr102-tr104 and was confirmed by screenshot ( |
| Eden: profile restored, temp files removed, host released | M | pass | eden/EDEN-PACKAGE-1.md |
| Eden: console.log searched for panic/abort/crash; run.json and memory.peak kept | M | pass | eden/EDEN-PACKAGE-1.md |
| Every local command under the memory and time guard | M | fail | Every cargo and cargo-skyline call ran under `systemd-run --user --scope -p MemoryMax=6G … timeout 1800` or the launcher script with MemoryMax and RuntimeMaxSec. Read-only helpers (nro_size.py, scan-offsets.py, llvm-objdump, ELF symbol scripts, compliance.py) ran without the guard |
| Evidence doc states run labels, build, what was done, what the log showed | M | pass | eden/EDEN-PACKAGE-1.md |
| GIF caption gives date and build; clip shows start, action and result | M | N/A: no GIF yet (RC item) |  |
| No operator quotes, no claims beyond the tests | M | pass | CARRY-OVER.md, IMPLEMENTATION-PROGRESS.md, docs/test-provenance/package-1.md and this file paraphrase decisions with their dates; claims are tied to tests or Eden rows |
| Handoff file current (branch, commit, evidence, live state, next commands) | M | fail: RC item | See "handoff file present": the repo handoff is on review/menu-proposal (ecb574d) and does not yet include tr105/tr106; IMPLEMENTATION-PROGRESS.md in this branch is current |
| Release: version bump, annotated tag, assets verified by anonymous download | C | N/A: the operator has not ordered a release |  |
| Release: rebuild of the tag byte-identical to the released NRO | C | N/A: the operator has not ordered a release |  |
| Card: MTP preflight, rollback copy, readbacks, receipts not re-run | C | N/A: the operator has not ordered an install |  |
| Card: the user connected and disconnected it; the agent never mounted, unmounted, ejected or remounted it, nor started or stopped DBI | M | pass | The agent never mounted, unmounted, ejected or remounted a card and never started DBI; the card scan used the local copies of earlier preflights |
| Nothing pushed, tagged or written to the card without the operator's order | M | pass | `git tag` is empty; no remote branch for work/implementation (`git branch -r`: origin/main, origin/review/menu-proposal only); nothing written to a card |
| Model/effort gate and watcher per the routing authority | M | pass | Own transcript `agent-a16caf62dc2a0d371.jsonl`: claude-sonnet-5-5 / high; watcher PID 2807135 alive, /tmp/trfu-impl-watch/watch.log empty (no change) |

Completed 2026-11-02 by the package 1 implementer on top of the lead's compliance.py run (HEAD e53b0ef). Rows:
29 pass, 15 fail, 5 N/A, 0 not authorized, 2 info.

Of the failures: 4 are the checker's src/*.rs glob (the items exist under src/model/ and src/runtime/; evidence in each row),
3 are release-candidate items (README GIF and build line, docs/media GIF, HANDOFF.md), and the rest are open work:
the replay A/B and two-console online rows (package 7), the hook-sharing stand-ins, the entry hook on the final input mapping,
tests for den/analyze_probe.py, build-product deletion at close, and the unguarded read-only helper commands.

## Workarounds, approved and open

| # | Item | Status | Source |
| --- | --- | --- | --- |
| A1 | Developer page: ships hidden in the normal build, shown only when developer.ini says developer_page=on (rows of the page are not built yet; package 1 has the INI parse and the boot flag) | **approved**: operator decision 2026-11-01 (N1, Q8, B9) | docs/DECISIONS.md lines 22, 42, 119 |
| W2 | Switch builds use the campaign launcher (RUSTUP_TOOLCHAIN=nightly-2026-02-14-…, skyline-v3 std) instead of the repo pin 1.98.1, because cargo-skyline passes -Z flags and stable refuses them (rror: the option 'Z' is only accepted on the nightly compiler). ust-toolchain.toml is unchanged; the NRO is compiled with skyline-v3 either way | **open**: not in DECISIONS.md; to be opened with the primary | IMPLEMENTATION-PROGRESS.md "Switch build" |
| W3 | Unknown settings rows are kept on rewrite (the spec says dropped); retired keys are dropped | accepted by the lead 2026-11-02 as the design (to be recorded in SPEC-core-04); not an operator decision | lead message; tests/core04_settings.rs core04_unknown_keys_kept_on_rewrite |
| W4 | D1 diagnostics file: append-open per drain with failures written to events.log, in place of the fork's long-held handle (design fix, root cause probable) | not a workaround; confirmed in Eden tr105 | commit 5beb35c; eden/EDEN-PACKAGE-1.md:140 |
