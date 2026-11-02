# Compliance report: TrainingRoomsForUltimate-pbaudit

| Requirement | Tag | Result | Evidence |
| --- | --- | --- | --- |
| crate-type is ["cdylib"] only (no rlib) | M | pass | Cargo.toml crate-type = ['cdylib'] |
| release profile opt-level "s", lto, codegen-units 1, panic abort | M | pass | Cargo.toml [profile.release] as required |
| skyline pinned to =0.6.0 | M | pass | Cargo.toml |
| diag cargo feature declared | M | pass | Cargo.toml [features] |
| toolchain 1.98.1 | M | pass | rust-toolchain.toml |
| runtime only under cfg(target_os = "switch") | M | pass | src/lib.rs |
| supported version 13.0.5 in model | M | pass | src/model/archive_recipe.rs, src/model/guard.rs, src/model/install_line.rs |
| every site guard is 64 bytes | M | fail | 884 guard(s) in src/, sizes [0, 64] |
| counting allocator reports heap_allocations | M | pass | src/**/*.rs |
| host tests in model (cargo test) | M | pass | 1223 #[test] in src/** |
| README carries the AI-assistance disclaimer | M | pass | README.md |
| README has a GIF, build commands and a license line | M | fail | README.md |
| a GIF under docs/media | M | fail | docs/media/*.gif: none |
| dated evidence doc present | M | pass | CARRY-OVER.md, IMPLEMENTATION-PROGRESS-pkg2.md, IMPLEMENTATION-PROGRESS-pkg3.md, IMPLEMENTATION-PROGRESS-pkg4.md, IMPLEMENTATION-PROGRESS-pkg5.md, IMPLEMENTATION-PROGRESS-pkg6.md, IMPLEMENTATION-PROGRESS-pkg6b.md, IMPLEMENTATION-PROGRESS-r2fix.md, IMPLEMENTATION-PROGRESS-wire.md, IMPLEMENTATION-PROGRESS.md, MENU-DRAFT.md, PROVENANCE.md, TRAININGROOMS-HANDOFF.md, docs/BUILD.md, docs/CARD-SWAP-PLAN.md, docs/DECISIONS.md, docs/MENU-PROPOSAL.md, docs/PACKAGE-6-PLAN.md, docs/PROGRESS-resource-costs.md, docs/U4-RUNTIME-TEXTURE.md, docs/compliance-package-1.md, docs/compliance-package-3.md, docs/implementation-packages.md, docs/menu-redesign.md, docs/resource-costs.md |
| numbers in the evidence come from tested scripts over the logs | C | agent | tools/: carry_pkg4.py, carry_pkg6.py, crop_menu_captures.py, elf_symbols.py, feature_groups.py, feature_matrix.py, gen_catalog.py, gen_sites_pkg3.py, gen_sites_pkg4.py, gen_sites_pkg6.py, gen_sites_u4.py, gen_stage_textures.py, pkg6_deps.py, pkg6_inventory.py, provenance_scan.py, render_decisions.py, render_menu_mockup.py, render_sdi_stretch.py; no test_*.py: N/A only if no numbers are cited |
| handoff file present | M | fail | HANDOFF.md |
| remote is Forgejo, not GitHub | M | pass | https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate.git |
| working tree clean | M | pass | git status |
| no script mounts, unmounts or ejects the card (the user connects and disconnects it) | M | pass | none in the tracked scripts or the playbook's tools/switch |
| build identity | M | info | HEAD bf836c8; no tag on HEAD |
| shipped NRO identity | M | info | /home/intpa/smash-training-lobby/cargo-targets/integ3-nro/libtraining_rooms-release.nro sha256 bbe6ffceb32ff61e resident 2940928 B (2872 KiB) |
| no diag strings in the shipped NRO | M | pass | libtraining_rooms-release.nro: no diag markers |
| Gates run: test, clippy -D warnings, no-default-features, skyline release, diag build | M | agent |  |
| Fixes: root cause fixed, or each workaround approved by the operator before it shipped and listed with its date | M | agent |  |
| Card scan for the site (and vtable slot) over a dated card copy | M | agent |  |
| Hook sharing or chaining; both stand-in modes and load orders in Eden | C | agent |  |
| Hook after the prologue or the function called instead | M | agent |  |
| Shipped NRO writes only its install line (plus a logging plugin's data files) | M | agent |  |
| Online: read-only rules reviewed hook by hook | C | agent |  |
| Online: replay A/B with the desync detector identical | C | agent |  |
| Online: two consoles past the changed path, desync_seen=0 at the same frame | C | agent |  |
| Eden: claim written and released, also after a failed run | M | agent |  |
| Eden: one instance per host, MemAvailable >= 10 GiB, not profile a or :61 | M | agent |  |
| Eden: profile copy with hard-linked game content; only the copy changed | M | agent |  |
| Copies: byte-identical files linked, differing or written-in-place files copied | M | agent |  |
| Disk: one target dir per source copy (never shared between copies), cargo home shared per host and toolchain, df checked (>= 40 GB), finished build products deleted with a manifest | M | agent |  |
| Eden: waits gated on a log line or screenshot check | M | agent |  |
| Eden: failed precondition aborted and retried under the next label | M | agent |  |
| Eden: profile restored, temp files removed, host released | M | agent |  |
| Eden: console.log searched for panic/abort/crash; run.json and memory.peak kept | M | agent |  |
| Every local command under the memory and time guard | M | agent |  |
| Evidence doc states run labels, build, what was done, what the log showed | M | agent |  |
| GIF caption gives date and build; clip shows start, action and result | M | agent |  |
| No operator quotes, no claims beyond the tests | M | agent |  |
| Handoff file current (branch, commit, evidence, live state, next commands) | M | agent |  |
| Release: version bump, annotated tag, assets verified by anonymous download | C | agent |  |
| Release: rebuild of the tag byte-identical to the released NRO | C | agent |  |
| Card: MTP preflight, rollback copy, readbacks, receipts not re-run | C | agent |  |
| Card: the user connected and disconnected it; the agent never mounted, unmounted, ejected or remounted it, nor started or stopped DBI | M | agent |  |
| Nothing pushed, tagged or written to the card without the operator's order | M | agent |  |
| Model/effort gate and watcher per the routing authority | M | agent |  |

4 mandatory machine row(s) failed; 30 row(s) for the agent.
