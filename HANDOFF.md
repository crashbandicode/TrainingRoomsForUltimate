## RC10 menu-assets worker handoff — 2026-11-04

Branch `rc10-menu-assets`, worktree
`/home/intpa/smash-training-lobby/tmp-rc10-menu-assets`, cut from
`work/implementation` at `42b510c`. One commit (`34e6576`), pushed to origin;
no tag, release or card action.

Operator order (2026-11-03, via primary f106ed01): the menu's character grids
were to show the same art the fork showed, pulled natively from the game at run
time (nothing baked), and the stage-mod question (Cosmic Destination in the
menu's stage select) was to be answered with evidence.

Change: `thumbs.rs` roster grids request the native key builder's `picture=0`
waiting-room stock icon (was `picture=1` chara_1 portrait) at the active
role's costume through the game's mod-aware file-resource service; the carried
`character_roster.rs` helper is renamed `menu_stock_icon_hash` via manifest
edit 42 (stage m5q, ledger regenerated). No layout/recipe change; no new hook.

Evidence: `docs/MEDIA.md` (RC10 menu clips, six, NRO `50d95130...`), runs
`glm-gifs-04`, `glm-gifs-05` (GIFs; character grids show the game's stock
icons) and `glm-fd-stock` (Cosmic Destination moved aside attempt) under
`/home/intpa/eden-runs/`; the stage grid follows stage mods - the FD tile
shows the mod's picture with the mod installed (menu-stages.gif). Gates green
(test/clippy/no-default/release/diag; carry verify). Disposed compliance
report: `../artifacts/glm-trial/glm-04-COMPLIANCE.md`. Open: no vanilla-side
(mod removed) tile shot - the menu's per-page focus memory defeated fixed
press counts; defaults clip shows the keys with the page already at defaults.

# TrainingRooms RC handoff

## RC9 isolated HATS build-worker handoff — 2026-11-04

Branch `rc9-hats`, worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-rc9`,
isolated from `work/implementation` at `97ca441a699a1733ebb3c965c6862b847c8e164a`.
No shared product edits, worker Eden boot, push/tag/release/card/DBI action.
The one local fix commit is resolved by `git rev-parse rc9-hats`; its immutable
full SHA is bound in the parent inbox's **Pkg6 → Primary RC9 built** post and the
external artifact handoff. This is not an RC READY/runtime-acceptance claim.

Root defect: `main+0x1fca0b0` returns bool (lr14 x0=1), not a Fighter pointer.
The strict active-entry resolver refuses a rejected layout before any entry/cell
read, returns a callback-scoped raw NonNull<Fighter> from a game-thread-only unsafe
helper, and checks both accessor and object id before full native cleanup. Old
hat leases remain pinned through cleanup and are released only on the later
callback. The unused predicate row/constants and roster dependency are retired.
Astra APPROVE-WITH-CHANGES source requirements applied; see `docs/RC9-HATS.md` (removed after v0.2.0-rc.1; see that tag).
Kirby remains physically pinned to cell0; generic non-zero-cell resolution and
non-first-roster Kirby are different acceptance clauses.

Authoritative evidence root: `../artifacts/trainingrooms-pkg6-rc9/build3/`.
Tested source content SHA256 `e1c2b62f66658e40dc639738a88f0dc203a1361d99be2bf867afc808d59ad268`,
archive SHA256 `83cd1bd7c246933f5368dfc2d1f0e17b9f9913e52c4d0de5fd4e7092eb9cf008`.
Only this documentation handoff differs after that freeze; executable/test/
generator/carry/dependency bytes must match `AUDIT.json`. No source gate is
substituted by a successful transport or an older candidate's result.

- Release: `release/libtraining_rooms.nro`, SHA256
  `7e6b7c13e134fde08a63b7ecc52f68a98c1d73b85bb6908c0817ea31d3079a89`.
- Diagnostic: `diag/libtraining_rooms.nro`, SHA256
  `5da1c5f11e4ecdf939867a1172a41a881a57687b8a14c6697c51fe2ccacbe46f`.
- Each retained `.so`, source-before/after, log, executed test ELF, resource,
  admission and release receipt is adjacent. Final test4/list and standalone
  test/list counts are computed by the bound helper; `AUDIT.json` is the gate
  authority. Never sum overlapping crate/model harness totals.
- Build1 pre-review clippy, build2 test-literal compile failure, first full test
  missing-Pillow/cache-verification failure and namespace/dev-null failures are
  retained, not silently replaced. Final tests use a private matching Pillow
  test-child namespace; no system dependency install. Compilers stay on Ally
  with an owned target, ext4 TMPDIR and bounded fresh admissions.
- `COMPLIANCE.md` explicitly disposes manual rows, including the unguarded
  read-only Get-Command discovery exception and interrupted/rearmed model watcher.
  `owned-target-retirement.json` records only owned finished-target cleanup.

Primary next: verify the final audit/full commit/artifact SHA bindings, then own
native P1/CPU Training/waiting multi-hat replacement, non-first-roster Kirby,
cancellation/loss/KO, cleanup entry/return and later lease settlement/source reuse.
No extra worker boot is authorized. The separate logging audit in
`../artifacts/trainingrooms-pkg6-rc9/logging/` leaves the missing-log cause unresolved:
zero Unmapped lines is not proof that no invalid read happened.

## Historical RC8 handoff

Updated 2026-11-03, RC8 batched gate repair. RC NOT READY. Operator reset supersedes the old359-row/full-feature acceptance bar.

## Current continuation

Lead `tr-lead-astra-6`, same NVIDIA GPT-6 Astra medium session/sole owner-bound watcher; playbook `03e2321`. This RC8 batch adds reviewed UI `dc6deb7` and its receipt documentation `16158f5` to frozen RC7 `1769bdd`. Exactly twelve missing Safety contracts in the real tick/native observer and ten host shims; executable bodies, guards, visibility, assertions, carry transforms, dependency/toolchain/size policies and lint allowances are unchanged. No Phase2/3 source imported. UI's OWN-branch all-target clippy `-D warnings` and no-default build both passed, independently reviewed by pkg4. PRIMARY now requires both checks before every worker fix return.

Original RC7 remains a failed-clippy candidate: Cargo101/twelve missing Safety sections preserved under parent `artifacts/trainingrooms-lead-astra4/rc7`. Its remaining coherent wave completed: all/list15902 top-level passes/91harnesses; no-default; release and diagnostic Cargo0/zero warnings; six standalone and four owned-fixture test/list pairs with exact source/log/preserved-binary audits. Independent normalizedsize135264/136069, full247+41carry and protected provenance checks passed. Counts from overlapping scopes are not summed. Actual compiled thread/entry budgets pass in the retained model; generic8KiB inventory still fails50 per binary/max111824 and is not waived. RC8 must be independently frozen/gated and submitted to PRIMARY for changed-boundary R3 review before its confirming run; no historical binary upgrades it.

PRIMARY's confirming plan: one10800-second Surface diagnostic process,10200-second driving deadline plus600-second cleanup margin, unchanged8GiB/swap0/Tasks256. Unreached/R3-affected rows first; all native UI tokens restore complete settings/defaults/unknown keys and native originals before gameplay reload fixtures. Source-only driver repairs retain actual image/log gates. Renderer-install refusal is covered by host probes; normal native pause/resume and the independent producer remain native rows. Load order A in diagnostic and order B in the already-planned release boot cover both, with actual order evidence. Special/PT/Aegis/IC/Mii mid-build interruption is N/A by synchronous construction; three genuinely pending ORDINARY async teardown/reentry rows remain required. No artificial delay or extra trace boot. Existing N11/HB1/option-A tracepoint plans run only at natural scheduled moments with the lead as sole driver.

Replay remains identical coherent17-file NAND/account/avatar/Mii staging, detector `29067bb63fd2721ed577f613088614a365804c9ac7101c4ad4ff8b8df3cd23a8`,1678 contiguous records/33560B, equal checks AND positions and computed==recorded. Yoga baseline stays as the LDN peer; Surface release reuses its replay process for release laps/online. Full source-bound runtime/replay/LDN, positive allocation coverage, applicable coexistence/placement and restoration evidence remain open. Parent STATUS/inbox/checkpoint and the final artifact root govern current live state. No push/tag/release/card/DBI/USB authority.

## Historical RC7 composition checkpoint

Lead `tr-lead-astra-6`, same NVIDIA GPT-6 Astra medium session and sole owner-bound watcher. Playbook `03e2321` pulled/current; complete playbook read, including the mandatory guest-tracepoint rule. Native facts use lead-armed tracepoints during authorized runs; no extra trace boot or diagnostic-print rebuild without a stated reason. The menu-open timing observation is not under investigation.

ONE RC7 correction is assembled from exact RC6 `719255d` plus pkg6 `39656a5`/`185e4c8`/`4e2e73c`, Slim `cab6603` through documentation/comment-only `b931875`, and UI `f9860f2`/`6545f1d`/`f2a6127`. Validation docs `cdada134` and `ef0269f` are included. No manual conflicts: complete product/test/tool union matches the reviewed blobs; the identical shared practice policy and carry entry occur once. Five carried destinations changed, with full247-file m5q derivation PASS. No dependency, budget, toolchain, exclusion or lint-allowance change. Worker focused tests/listings and pkg4 reviews are complete; coherent RC7 gates and R3 changed-boundary re-review remain required before confirming runtime.

RC6 `lr-01` bug-finding stopped15:01:22Z. Full console21,427lines/3,309,708B/SHA256 `90b31e04a55e122ebe87f16b132a7ac33b42bb5b3cc6456d577635790757acae`: total Unmapped0, panic/abort/crash0. Full1604profile+10harness restore and a separate verification passed; DONE15:01:28Z, raw15:01RELEASED independently parsed, no lead claim/Eden/display remains. Per-row results and failures are in parent `artifacts/trainingrooms-lead-astra4/rc6/runtime/lr-01/RESULTS.md`. Native roster token restoration was incomplete after an early Loading3/9 gate was refused; full stopped-profile restoration is separately proven. No additional demonstrated product root joins RC7.

No RC READY: genuine interrupted-build rows, full native resource IDs, remaining runtime/UI/special/pilot/coexistence rows, positive allocation/stack matrix, replay A/B and LDN are still open. Preserve generic8KiB compiled inventory failures and all historical failed observations. Runtime order remains core→native UI/restoration→gameplay-only reload fixtures. Same17-file replay staging and detector identity below apply. Phase2/3 implementation may proceed on isolated existing worker branches, with no Phase2/3 Eden before RC READY and no leakage into RC7.

## Historical 13:20 continuation

Lead `tr-lead-astra-6`, OpenCode `ses_e5ea4d5f3ffeMMtWumvKxZhl2t`, verified NVIDIA GPT-6 Astra medium. Durable operational state is in the parent `TRAININGROOMS-LEAD-ASTRA-5-CHECKPOINT.md`; PRIMARY orders and worker returns are in `TRAININGROOMS-LEAD-INBOX.md`. Earlier lead/process references below are historical. Playbook `2dee6cc` was pulled and read completely.

RC5 `5e3fe5e` passed its all/list pair: 14,189 top-level tests across 87 harnesses, with source continuity and preserved executed binaries independently checked. Other host jobs completed successfully; standalone inventories were deferred because RC5 failed clippy. Its 16 pane errors are corrected by reviewed UI `de92c4a` (safety contracts and pinned unsigned 25-byte name binding); development full clippy with `--keep-going` passed without warnings. Final-candidate gates remain required.

RC5 native release and diagnostic builds both exited Cargo 0 but each emitted five unused-function warnings. The release runner initially missed ANSI-colored warnings; its original receipt and raw log are preserved alongside an independent `warning-audit.json`. The corrected runner detected the diagnostic warnings and failed that gate. Native binaries remain evidence of these failed gates, not runtime-qualified artifacts. Reviewed orphan-API removal from `128b16c` joins the pane correction in the RC6 batch; no lint allowance is added. The proposed string-presence test is omitted because it mirrors deletion rather than exercising behavior; all existing pkg6 tests remain unchanged. Full RC6 host/native/carry/size/stack gates are pending.

RC5 normalized size was 134,924/136,069; full carry derivation passed 247+41. The vendored MIT binrw 0.13.4 prelude backport and two existing cfg-name declarations received independent source/license review. Final size/carry/compiled stacks/compliance and runtime acceptance are pending. No lead6 Eden has started.

Replay inputs are the coherent retained M5q full save/account/avatar/Mii set (17 files), with the same detector SHA256 `29067bb63fd2721ed577f613088614a365804c9ac7101c4ad4ff8b8df3cd23a8` for baseline, candidate and online peers. UI settings/default fixtures use real menu input; diagnostic reload does not synchronize the menu worker's full settings/defaults. Developer visibility is boot-only: visible in the diagnostic batch, hidden in the already-planned release setup. These are preparation decisions, not completed runtime evidence.

PRIMARY rescinded the aggregate 60GB cache admission gate. Fresh per-host disk >=40GB, memory, process, queue and shared-ledger checks remain mandatory, with completed-cache retirement and preserved evidence. No push, tag, release, card, DBI or USB operations are authorized.

## Objective and source

RC = the functional rows M5q actually passed + already-working new UI + one R-D1 replay A/B and one R-O1 LDN session + clean MIT provenance. Exact matrix and assignments: parent TRAININGROOMS-STATUS.md, section RC RESET PLAN. No new Phase-1 feature work. Subsequent Phase 2 and Phase 3 are explicitly authorized below; deferred scope and exclusion boundaries: NEXT-VERSION.md (removed after v0.2.0-rc.1; see that tag).

Branch work/implementation, worktree /home/intpa/smash-training-lobby/TrainingRoomsForUltimate-work. Current product7b035f2: UI22 inverse0c60459; StateWriter40KiB8fe6261; UI freeze87597fc/6ba06e7; advanced-workflow closure11ee226; optional observers8c0ec63; coupled F9/exact-created admission withdrawal661f7ad. Lead resolved observer/count seam and carry overlap: seven tables,247 remaining carried files re-derived with every reviewed destination byte preserved. Add New/native custom-Mii editor exposure is closed by7b035f2 at the production adapter and fresh/resumed provider admission, preserving ordinary saved catalog/CPU Mii selection. Generic enabled model policy remains for later qualification; adapter regressions await the final host gate. Final source/NRO identities follow coherent composition, no scratch receipts.

Lead thread01a19e0c-415a-7af3-ac6e-f574318709d2, Astra medium. Owner182663/watch303502; verify exact model/effort on compaction. Playbook2dee6cc fully read. Primary successor Claude f106ed01 owns the same four existing Sol workers; no duplicate launches. Operator-authorized OpenCode staffing supersedes the old no-OpenCode rule.

## Live state and assignments

Lead owns no compile/Eden/claim. Pkg4 p4c04 stopped10:29:57, full1362profile+14harness restoration plus separate verification, Yoga released10:31. Pkg6 s6-13 stopped10:33:49, full1600profile+17harness restoration plus separate verification, Surface released10:33. Recheck canonical ledger at any admission; these are past observations.

- Pkg6 Sol: proven roster/Mii/hat inventory and safe F9 disable boundary; later Surface laps/teardown/Mii/hats/release rows.
- Pkg4 Sol: proven recording versus advanced unfinished workflow boundary, M5q truncation facts; later host gates and replay assistance.
- UI Sol: Setup/Q15/UI22 boundary and evidence inventory; UI segment of the shared Surface batch.
- Slim Sol: size recovery through deferrals, state lifecycle stays unintegrated; native/pair exclusions and pilot segment of the shared Surface batch, size/carry/stack review.
- Lead: review boundaries, compose candidate, online pair, final integration and acceptance. All freeze slices integrated, including the lead native-editor closure7b035f2 after the UI deadline. Primary resumed the existing pkg4/UI sessions11:15; pkg4 prepares the final host runner, UI reviews the exact lead closure then prepares its shared segment. Pkg6/Slim shared-segment returns are under lead review. No worker compiles before coherent plan/composition.

## Kept evidence and limitations

Frozen r18 sourcef1db4cb, diagnostic NRO1be5790b/ELF985e9612, parent artifacts/trainingrooms-lead-astra4/main-r18, is development evidence only. ONE diagnostic build0warnings;16selectedhost tests, not all-target. Resident6242304bytes. UI22 is absent from r18. Earlier raw failures and receipts remain unchanged; no further intermediate manifests requested.

UI21-01/02 prove hidden/visible Developer, default/custom notices, actual Solo waiting menu and two-buffer retirement with full0Unmapped/0owned allocations. ui-17 proves exercised X/Y/ZR/current-default file values. ui-10/15/16 functional direction/stage/portrait/item rows have older whole-run health limitations and require final candidate repetition. Developer remains enabled by its existing opt-in. Real README GIF is9.5seconds from r16/758cee5, accurately labelled development footage; no fabricated final-candidate claim.

F11 empty/unpublished-cell crash guard and existing zero-allocation repairs stay. Six strict f631..36 boots and72laps/12bootstrap had0TOTALUnmapped/0owned allocation; enabled-capacity f637 also passed its scoped health row. Do not remove these fixes while deferring F9/F6 enhancements.

s6-13 partial1→2→3→2→1 body/HUD/LRA/save-load positives do not qualify complete F9. Full13856-line console0Unmapped/no panic-abort-crash, but StateWriter measured17664/36864 FAILS stack policy. Source8fe6261 corrects the table to40KiB; actual final native/runtime verification remains owed. Retain the original failure; do not attribute it to F9 without evidence. Cutover/exact-Mii/lifetime unrun.

p4c04 actual CaptureOff→On, native armed, KO nativeSave/persisted; accidental movement offstage made it a KOcase, not timer evidence. p4c02 had2x382frames/nontruncated. p4c03 timer had2x3600frames/truncatedtrue, root unknown. No new capture policy fix.

## Known issues and honest M5q comparison

- F9 count raise: RC intends same as M5q wrong-CPU behavior after disabling unproven promotion; final behavior must match the parity plan.
- F11 recording after swaps: known limitation in reset scope; independent TR empty-cell access repair is an improvement, not proof all recording-after-swap paths work.
- F6 cold swap hitches: same class as M5q. Its S RESULT explicitly fails cold20ms target (first122.9ms); no equal latency or performance improvement claimed.
- Large-fighter first-frame hitbox refusals and F5 power-loss partial writes: retain as known issues; exact M5q comparison references still being reconciled before final README wording.
- Timer recording truncation and input-log light-background contrast: observed TR limitations, M5q comparison UNKNOWN. Do not claim same as M5q or silently waive a regression.

No operator-blocked Hitbox redesign remains in the RC scope: A/B and redesign are deferred. Settings Profiles was an invented historical census item, not a feature to implement.

## Remaining execution

1. Consume parent LEAD-INBOX PRIMARY REQUESTs at least every10min; lead turn report about every45min without ending. Last10:52, next~11:39.
2. Finish bounded UI source review and compose ONE frozen candidate. All source cuts are integrated. Combined9c65c2e measured134898 normalized versus136069 ceiling; final7b035f2 count remains owed. No ceiling increase or formatting/count trick.
3. Candidate gates: all-targethost tests, clippy all-targets -Dwarnings0, no-default build, release+diag0warnings, stack policy, normalizedsize<=136069, carry/provenance, explicit-release compliance and allmanual dispositions scoped to reset matrix. Fix genuine failures and rerun affected checks, preserving failures.
4. ONE continuous Surface diagnostic session for all single-console rows; Yoga baseline replay retained as online peer and Surface release replay retained as online host/release-lap process: target THREE physical starts total. Every run0TOTALUnmapped/no new crash, no allocation regression from existingzero. M5q failed coldtiming bar and unrun stale-Mii/onlineledge cases are NOT extra gates.
5. R-D1 same complete replay baseline/candidate checks ANDpositions; R-O1 actual LDN approvedjoin/membership/waiting changes/one completedmatch and detector equalendframes/desync_seen0both. R-O1 uses LANOnlineOFF per actualM5q evidence, correcting older public-TLS planning. Full profiles restored/separately verified before release.
6. Final README/HANDOFF/compliance/GIF, RC READY, primary Astra-high review. No push/tag/release/card/DBI/USB. Fresh40GB disk floor, bounded jobs, one Eden per host/no compile beside it, shared ledger/queue remain; aggregate60GB cap was rescinded by PRIMARY.

The historical359-row inventory is post-RC backlog; do not resume chasing every old blocker. No current candidate acceptance is inferred from this handoff.
## StateWriter stack correction in reset composition

Personally checked s6-13 full console lines13457–13481: state saved/queued,
file_written, then high_water17664/36864 (free19200, low=false). The existing
health parser correctly fails the stronger policy despite low=false:
ceil_page(17664 +4416 +16384)=40960. The table still recorded historical15408,
so its allocation underestimated the now-observed writer path. Updated the
same table row to measured17664 and40KiB; static bound15184 and unchanged25%
plus16KiB reserve remain. runtime/threads.rs derives its actual static array
from that row, adding4096B BSS with no hook allocation or behavioral change.
No speculative writer refactor or F9 attribution. Existing thread_stacks host
checks and final compiled/static/runtime gates must verify the coherent RC;
no new intermediate build or PASS claim. Retain this historical maximum even
if disabling Setup later reduces one path.

## Subsequent operator-authorized phases (10:5x / 11:0x orders)

Phase 2 after RC: implement all missing listed core02/03/07/08, N1–N10, B3, Q5/Q7/Q16 behavior with host tests before one batched Eden validation. B11 already exists; no new implementation. S2/B4/B7 move to Phase 3. Target one validation plus at most one bug-confirmation boot, and at most one combined native trace boot only if facts block implementation.

Phase 3, TrainingRooms only: S2 field-wise settings recovery preserving settings.invalid-<n>.json and permission-denial regression; B4 no CPU-settings writes to entry0 and Record refusal under P1 pilot; B7 opponent-relative air-dodge/shield-tilt In/Out; HB1 all enabled hurtboxes plus option A isolated visualization sphere with equivalent geometry/translucency/tints/bone-following/size/first-frame and Ridley full first-pass proof. Ordinary shields unchanged, no fork backport. Implement/host-test the whole batch before ONE Eden run; batch any necessary native traces and bug fixes. Phase 1/2 keep unproven items disabled or at M5q behavior and are not blocked by Phase 3.

## Build resources and replay preparation — 11:01 UTC

Fresh10:54 census58,606,160,455B; subsequently retired only completed owned r18 native cache326,207,148B and setup-r1/r2 caches238,989,642B each. Archived NRO/ELF/source/logs and six actual executed host binaries were verified and retained; no live references and no new per-file manifest. Estimated57,801,974,023B is not admission: refresh cache, disk, memory and shared ledger before growth.

Fresh Surface lead rig is needed; the compile ledger parameter root trainingrooms-eden-state-core has no runtime profile. Existing parameterized harness supplies the route; sourceg stays read-only. Sourceg contains five native replays and complete save index plus three saved Mii files (actual class/selection still to verify). Use one identical complete game-save/replay fixture on both owned profiles for A/B after snapshots, then retain those processes for LDN. No replay fixture accepted or run yet.

## Final-freeze preparation checkpoint

Pkg4 provenance omission found by Slim on9c65c2e is corrected in a6afe07: exact11ee226 transforms, full41-file rederivation, all reviewed destination bytes and original source pins preserved. Original failure remains recorded. Editor closure7b035f2 similarly rederives all247pkg6 files without source-pin changes. No Cargo/native/Eden acceptance yet.

Prepared source-only adapters: /tmp/tr-rc-freeze.py (explicit expected full HEAD, clean source, new rc1 artifact root, absent host/native targets); /tmp/tr-rc-native.ps1 and tr-rc-native-claimed.ps1 (parameterized final source, archive release/diag separately, bounded6GiB/1800s); /tmp/tr-rc-profile.py (existing full mutable-profile/harness backup/restore, parameterized only to the owned lead root). Syntax checks only, none executed. Fresh three-host cache census11:17:42..50 completed under1GiB/150s:57,801,974,023B, below60GB. Recheck freshness and reserve actual target growth at admission; do not use old arithmetic.

Sourceg three saved-Mii file hashes match pkg6 s6-10 fixture exactly, including Gunner3137 ca5ddf724a10ffde5fee57347077487d2762e04b17c79503e82b8168baf9e240. This is file identity only; native current canonical selection/group proof remains owed. M5q actually used Lucas original with Gunner/DK choices; Gunner CSS origin is not required. Yoga private profile has two save UUID directories; confirm the actual selected Replay Vault fixture after copying, not merely a matching filename.

Qualified retained aid candidates: switch-mtp-v0101-v021/pilot/after/.../plugins libnro_hook ca7aec03, libparam_hook3ed7a294, pilot e3033426; Deluxe0.10.1 at switch-mtp-v0101-v021/deluxe/libssbu_online_deluxe_rewrite.nro a8dcda24. Only these selected files, never the old full plugin folder. Prior Deluxe dx-02 functional page evidence has11bootUnmapped/rising allocations; it is not final clean acceptance. Final common provider installation and actual pilot/Deluxe control must be positive in the shared session.

11:20–11:21 UTC: prepared five standalone Cargo.lock files under canonical Surface metadata-only claims, each command bounded6GiB/swap0/Tasks256/60s; no compiler/target/Eden. Initial offline resolution selected serde_json1.0.154 for match-data/oracle; explicitly pinned both to shipped-root1.0.151 before any test. All package/version/source/checksum tuples in all five locks now match root identities. Claims released, own queue cleared. Pkg4 runner return source reviewed; UI independent7b035f2 review found no concrete closure gap; Slim final product count134931<=136069, full247+41carryPASS. Ready to freeze after committing validation metadata; no gate acceptance yet.

## RC1 gate failure and correction — 11:28 UTC
Frozenb457670 all-target test exited101: library1325passed/1failed/2ignored; Cargo stopped before later harnesses. Source verified before/after, actualexecutedELFe989001e archived, log524a8613, peak4,989,300,736B/noOOM. Surface released11:27. Failure: constructor_context at0x377c0d0 absent from host seam census. rows_q15 was registered in TABLES, but cfg(diag) removed its data in default host tests while the census scans all runtime attributes. Corrected guard-data availability to cfg(any(test,diag)); release remains empty, diagnostic native rows unchanged. Existing full hook-coverage test caught the defect and must pass on the revised final candidate; no waiver/UNCONVERTED entry. RC1 failed evidence retained. No native or Eden was run. Revised finalfreeze/gates required, not acceptance from scoped1325passes.


RC2 host gate failure 2026-11-03T11:38:24.450768+00:00: originalQ15census fixed7535709 and passed. Later tests/core02_menu.rs draft row counts failed273pass/1fail; actual31/11/15/20/11/3 vs stale31/13/17/20/11/3. Lead aligns existing test with approved four-editor deferrals, adds explicit absence/stable-group assertions. All rawlogs/executedELFs retained in rc2; no all-targetPASS or laterharness execution inferred. Pkg4 review pending before revised finalfreeze. No native/Eden. Last leadreport2026-11-03T11:38:24.450768+00:00, next~12:23.

Pre-refreeze audits complete: pkg4/UI reviewed removed four editors, counts/stable hidden group and native Add New; pkg6 reviewed F9/Q15, Slim optional observers. No additional stale active-exposure expectation found. Pure storage/policy tests retained. Existing core02 contract now asserts approved counts, all four absent, only stable group7 empty/hidden with both pilot environments, retained Named library/save/capture rows. Next full host command and inventory both use --no-fail-fast; failures remain FAIL and all harnesses get a chance to run. No runtime changes in this correction.

## Complete RC3 host result and native-wrapper repair
RC3 6e2f3af all-target --no-fail-fast completed101, all87 top-level harness binaries executed/preserved; only native_wrappers failed. Rawlogc531075b2b35c307bbdbca6a09e7f98e3f7e2eb8764f6a02286490d089d5759f, sourcebefore/after1332/179bf7df intact, warning0/peak4666036224/OOM0, independentpkg4audit. No successful whole-suite count; inventorynotrun onfailedgate. Surface released11:46. All87executedELFs/archive/log retained/rehashed before retiring ownedfailedhost1807561569B.
Root0x15cdff0 duplicated in game::natives::fighter_kind andstate_natives::object_kind. Sole guarded boundary nowreturnsOption internally, preservingactualexternC ABI/globalgrant; Developer comparesSome(nonnegativeKirby), retainingunknownrefusal; stateadapter unwrap_or_default keeps0 fallback (using-1 wouldincorrectlyhide switch effects). Existing structural regression/callgraph tests unchanged. Pkg4 independentlyreviewed actualoffsets/bothcallers/guards/fallbacks; source-onlyPASS, fullhost/native gates owed. None ofthreefilescarried. Revisedfreeze/fullno-fail-fastwave required.
PRIMARYN11 added toPhase2 backlog/defaultOn nativewaitingitemrule only; noRCbehavior change. Sharedruntimecontroller reviewedsource-only; privatefullconsoleFileHandler fix/defaultmarkers/exactbytecodeallowlist pendingUIreturn, noEden. Lastleadreport11:38/next12:23; continuePRIMARYfirst.
