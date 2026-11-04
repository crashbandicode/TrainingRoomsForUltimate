# Compliance report: TrainingRoomsForUltimate-rc9

| Requirement | Tag | Result | Evidence |
| --- | --- | --- | --- |
| crate-type is ["cdylib"] only (no rlib) | M | pass | Cargo.toml crate-type = ['cdylib'] |
| release profile opt-level "s", lto, codegen-units 1, panic abort | M | pass | Cargo.toml [profile.release] as required |
| skyline pinned to =0.6.0 | M | pass | Cargo.toml |
| diag cargo feature declared | M | pass | Cargo.toml [features] |
| toolchain 1.98.1 | M | pass | rust-toolchain.toml |
| runtime only under cfg(target_os = "switch") | M | pass | src/lib.rs |
| supported version 13.0.5 in model | M | pass | src/model/archive_recipe.rs, src/model/developer_debug.rs, src/model/effect_chips.rs |
| every site guard is 64 bytes | M | pass | 847 guard(s) in src/, sizes [64] |
| counting allocator reports heap_allocations | M | pass | src/**/*.rs |
| host tests in model (cargo test) | M | pass | 1392 #[test] in src/** |
| README carries the AI-assistance disclaimer | M | pass | README.md |
| README has a GIF, build commands and a license line | M | pass | README.md |
| GIF waiting-menu-r16.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.5 s |
| dated evidence doc present | M | pass | CARRY-OVER.md, IMPLEMENTATION-PROGRESS-deluxe.md, IMPLEMENTATION-PROGRESS-f6.md, IMPLEMENTATION-PROGRESS-harness.md, IMPLEMENTATION-PROGRESS-hitbox.md, IMPLEMENTATION-PROGRESS-integrator3.md, IMPLEMENTATION-PROGRESS-pkg2.md, IMPLEMENTATION-PROGRESS-pkg3.md, IMPLEMENTATION-PROGRESS-pkg4.md, IMPLEMENTATION-PROGRESS-pkg5.md, IMPLEMENTATION-PROGRESS-pkg6-rc7.md, IMPLEMENTATION-PROGRESS-pkg6.md, IMPLEMENTATION-PROGRESS-pkg6b.md, IMPLEMENTATION-PROGRESS-r1fix.md, IMPLEMENTATION-PROGRESS-r2fix.md, IMPLEMENTATION-PROGRESS-slim.md, IMPLEMENTATION-PROGRESS-stack.md, IMPLEMENTATION-PROGRESS-state-core.md, IMPLEMENTATION-PROGRESS-sync.md, IMPLEMENTATION-PROGRESS-u4b.md, IMPLEMENTATION-PROGRESS-ui-screens.md, IMPLEMENTATION-PROGRESS-wire.md, IMPLEMENTATION-PROGRESS.md, MENU-DRAFT.md, NEXT-VERSION.md, PROVENANCE.md, TRAININGROOMS-HANDOFF.md, docs/APPROVED-MENU-ARTWORK-CONTRACT.md, docs/ASSET-REPRODUCTION-R15.md, docs/BUILD.md, docs/CARD-SWAP-PLAN.md, docs/DECISIONS.md, docs/DEVELOPMENT-R16.md, docs/F6-COUNT-HOOK-SITE-REVIEW.md, docs/F6-HELPER-DEMAND-ABI-REVIEW.md, docs/F6-HITBOX-OBSERVER-ABI-REVIEW.md, docs/F6-MATCHED-PERFORMANCE-PLAN.md, docs/F6-R15-MATCHED-DEVELOPMENT.md, docs/HARNESS.md, docs/HITBOX-DEMAND-OBSERVER-CONTRACT.md, docs/HITBOX-OBSERVER-CONTRACT.md, docs/HITBOX-POOL-OBSERVATION-PROPOSAL.md, docs/MENU-PROPOSAL.md, docs/NOTICE-CONSUMER-CONTRACT.md, docs/NOTICE-LAYOUT-REVIEW.md, docs/PACKAGE-6-PLAN.md, docs/PKG4-COMPLETION-OBSERVER.md, docs/PROGRESS-resource-costs.md, docs/R12-COMPILED-STACK-REVIEW.md, docs/R13-COMPILED-STACK-REVIEW.md, docs/R17-ONLINE-RELEASE-DELTA.md, docs/RC-COMPLIANCE-EVIDENCE.md, docs/RC-NATIVE-WARNING-ROOT.md, docs/RC-RESET-OBSERVATION-RETIREMENT.md, docs/RC7-R3-1-VALIDATION.md, docs/RC7-R3-2-VALIDATION.md, docs/RC7-R3-UI-VALIDATION.md, docs/RC9-HATS.md, docs/SH01-MEASURED-STACK-REVIEW.md, docs/SIZE-PLAN.md, docs/STACK-SCANNER-CONTEXT-FIX.md, docs/U4-RUNTIME-TEXTURE.md, docs/U4B-RESIDENT-EFFECT-TEXTURE.md, docs/UI-DEMO-PROVENANCE.md, docs/UI16-PROBE-STACK-FAULT.md, docs/compliance-package-1.md, docs/compliance-package-3.md, docs/compliance-ui-screens-development.md, docs/implementation-packages.md, docs/menu-redesign.md, docs/resource-costs.md |
| numbers in the evidence come from tested scripts over the logs | C | pass | tools/: carry_pkg4.py, carry_pkg6.py, carry_ui_items.py, crop_menu_captures.py, elf_symbols.py, f6_matched_table.py, f6_split_table.py, feature_groups.py, feature_matrix.py, gen_catalog.py, gen_hitbox_sites.py, gen_sites_pkg3.py, gen_sites_pkg4.py, gen_sites_pkg6.py, gen_sites_u4.py, gen_sites_u4b.py, gen_stage_textures.py, pkg6_deps.py, pkg6_inventory.py, provenance_scan.py, re_hitbox_layout.py, render_decisions.py, render_menu_mockup.py, render_sdi_stretch.py, seam_census.py, size_gate.py, sloc.py, stack_depth.py, stack_table.py, test_f6_rows.py, test_f6_split_table.py, test_size_gate.py, test_stack_depth.py, test_stack_table.py |
| handoff file present | M | pass | HANDOFF.md |
| remote is Forgejo, not GitHub | M | pass | https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate.git |
| working tree clean | M | pass | git status |
| no script mounts, unmounts or ejects the card (the user connects and disconnects it) | M | pass | none in the tracked scripts or the playbook's tools/switch |
| build identity | M | info | HEAD 5205a8f; no tag on HEAD |
| shipped NRO identity | M | info | /home/intpa/smash-training-lobby/artifacts/trainingrooms-pkg6-rc9/build3/release/libtraining_rooms.nro sha256 7e6b7c13e134fde0 resident 5451776 B (5324 KiB) |
| no diag strings in the shipped NRO | M | pass | libtraining_rooms.nro: no diag markers |
| Gates run: test, clippy -D warnings, no-default-features, skyline release, diag build | M | pass | AUDIT.json and individual job.json/logs; all final Cargo jobs0, source before/after bound, no warnings; build2 regression failure retained |
| Fixes: root cause fixed, or each workaround approved by the operator before it shipped and listed with its date | M | pass | docs/RC9-HATS.md; boolean 0x1fca0b0-as-pointer removed, guarded active Fighter + both identities; operator23:50Z/Astra AWC requirements applied, no workaround |
| Card scan for the site (and vtable slot) over a dated card copy | M | N/A | no new/moved site or hook; retired unused predicate and dependency only; existing full cleanup seam unchanged; no card authority |
| Hook sharing or chaining; both stand-in modes and load orders in Eden | C | N/A | no hook added/changed; no worker runtime authorized; existing hook coexistence is not requalified |
| Hook after the prologue or the function called instead | M | pass | full native cleanup is called at existing guarded sources.kirby_copy_clear; no hook/prologue rewrite |
| Shipped NRO writes only its install line (plus a logging plugin's data files) | M | pass | release compliance NRO diag-string scan plus unchanged diagnostic logging facade; no new release logging/file writer |
| Online: read-only rules reviewed hook by hook | C | N/A | delta adds no online hook; practice-only cleanup/resolver change. Historical online qualification not upgraded |
| Online: replay A/B with the desync detector identical | C | not authorized | no worker boot; Primary owns runtime acceptance |
| Online: two consoles past the changed path, desync_seen=0 at the same frame | C | not authorized | no worker boot; Primary owns runtime acceptance |
| Eden: claim written and released, also after a failed run | M | not authorized | no worker Eden boot/profile mutation; Primary owns runtime. Actual Ally Cargo claims/admission/releases retained per job |
| Eden: one instance per host, MemAvailable >= 10 GiB, not profile a or :61 | M | not authorized | no worker Eden boot/profile mutation; Primary owns runtime. Actual Ally Cargo claims/admission/releases retained per job |
| Eden: profile copy with hard-linked game content; only the copy changed | M | not authorized | no worker Eden boot/profile mutation; Primary owns runtime. Actual Ally Cargo claims/admission/releases retained per job |
| Copies: byte-identical files linked, differing or written-in-place files copied | M | pass | frozen archive extracted into separate Ally source at required absolute path; complete inventory verified, prior copies retained; no runtime profile copied |
| Disk: one target dir per source copy (never shared between copies), cargo home shared per host and toolchain, df checked (>= 40 GB), finished build products deleted with a manifest | M | pass | unique build1/build2/build3 target paths and ext4 TMPDIR, admission JSONs >=40GB; owned-target-retirement.json binds removal; source/test/native evidence retained |
| Eden: waits gated on a log line or screenshot check | M | not authorized | no worker Eden boot/profile mutation; Primary owns runtime. Actual Ally Cargo claims/admission/releases retained per job |
| Eden: test run plays a runner dry-run before Up (recorded segments + sha256, automated gates, per-row receipts, rows chained on end state); manual input only to build a missing segment | M | not authorized | no worker Eden boot/profile mutation; Primary owns runtime. Actual Ally Cargo claims/admission/releases retained per job |
| Eden: rows grouped into as few boots as hosts/builds allow; a failed row is fixed or deferred on the same boot; per-run RESULTS.md <= 20 lines, compliance once per phase | M | not authorized | no worker Eden boot/profile mutation; Primary owns runtime. Actual Ally Cargo claims/admission/releases retained per job |
| Eden: failed precondition aborted and retried under the next label | M | not authorized | no worker Eden boot/profile mutation; Primary owns runtime. Actual Ally Cargo claims/admission/releases retained per job |
| Eden: game-side facts from guest tracing (persistent trace.* points or the one-shot tracepoint) on a run already happening; diag rebuild or extra trace boot only with a stated reason | M | not authorized | no worker Eden boot/profile mutation; Primary owns runtime. Actual Ally Cargo claims/admission/releases retained per job |
| Eden: profile restored, temp files removed, host released | M | not authorized | no worker Eden boot/profile mutation; Primary owns runtime. Actual Ally Cargo claims/admission/releases retained per job |
| Eden: console.log searched for panic/abort/crash; run.json and memory.peak kept | M | not authorized | no worker Eden boot/profile mutation; Primary owns runtime. Actual Ally Cargo claims/admission/releases retained per job |
| Every local command under the memory and time guard | M | fail | all actual gates/remote adapters explicitly memory/time bounded and required PowerShell used; one Get-Command objdump-discovery shell call omitted memory/time guard. Retained tool history; no compiler/runtime in that call |
| Evidence doc states run labels, build, what was done, what the log showed | M | pass | docs/RC9-HATS.md, AUDIT.json, HANDOFF.md and parent inbox; no worker run label because no Eden boot |
| GIF caption gives date and build; clip shows start, action and result | M | N/A | no GIF created/modified; existing GIF dimension/duration machine pass is not fresh visual/content qualification |
| No operator quotes, no claims beyond the tests | M | pass | handoff separates source/artifact gates from Primary-owned native acceptance; logging anomaly explicitly unresolved |
| Handoff file current (branch, commit, evidence, live state, next commands) | M | pass | HANDOFF.md RC9 heading, one rc9-hats commit and final parent inbox/artifact HANDOFF bind immutable commit and hashes |
| Release: version bump, annotated tag, assets verified by anonymous download | C | not authorized | no push/tag/release requested |
| Release: rebuild of the tag byte-identical to the released NRO | C | not authorized | no tag/release requested |
| Card: MTP preflight, rollback copy, readbacks, receipts not re-run | C | not authorized | no card/device/DBI action requested |
| Card: the user connected and disconnected it; the agent never mounted, unmounted, ejected or remounted it, nor started or stopped DBI | M | pass | no device action performed; scripts checked by compliance |
| Nothing pushed, tagged or written to the card without the operator's order | M | pass | no push/tag/release/card operation performed; one local commit only |
| Model/effort gate and watcher per the routing authority | M | pass | exact Sol/high verified before work; owner-bound watcher log and final stop receipt retained; no delegated worker |

0 mandatory machine row(s) failed; 32 row(s) for the agent.

## Scope limits

Native P1/CPU Training/waiting multi-hat, non-first-roster Kirby, cancellation/loss/KO, source reuse and lease settlement remain Primary-owned. Generic cells1/2 host spies do not qualify a non-zero Kirby layout (Kirby is pinned to cell0). No RC READY claim.
Missing Eden unmapped-log cause is unresolved; observed zero lines is not proof of no invalid load. See ../logging/ and the separate inbox post.
Build1 pre-review clippy pass/build2 overflowing-test-literal failure and build3 first full test missing-Pillow/cache-verification failure are retained. Namespace launch failures test2/test3/pillow-gate traced to missing device bind: /dev/null EACCES; devprobe proves --dev-bind fixes that. Focused pillow-gate2 and final test4/list use a private test-child Pillow namespace, no system install. Original test2 receipt status=completed does NOT mean passed: actual Cargo101 is preserved.
Test admission refusal at10GiB Windows free retained; actual jobs require6GiB cap+2GiB Windows headroom and7GiB WSL availability. Old model watcher ended SIGKILL; exact thread reverified Sol/high and rearmed at512MiB; watcher interruption is disclosed, not continuous-coverage proof.
Only HANDOFF.md/docs/RC9-HATS.md may differ documentation-only from tested freeze; executable/tests/generator/carry and dependencies must byte-match AUDIT.json. No source acceptance inferred from a successful transport.
All 32 manual rows explicitly disposed. Process exception above remains a fail, not waived.
