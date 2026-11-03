# RC compliance evidence map

2026-11-03 11:01 UTC: RC reset and three-phase operator order supersede the old359-row final acceptance references below. Current product7b035f2 incorporates all freeze removals and closes native-editor exposure; coherent candidate gates and runtime remain pending. Latest compiled development evidence is r18 f1db4cb, not a release candidate. All historical scoped evidence/failures below remain unchanged.

Phase1 final acceptance is the STATUS RC RESET PLAN: M5q actual functional passes, already-working UI, one full replay A/B and one LDN session, zero total Unmapped/no new crash, preserved zero-hook-allocation fixes, unchanged size/stack/lint/provenance gates and explicit final-release checker. S2/B4/B7/HB1 belong to later Phase3 and do not block RC. Three physical starts are planned, with shared replay/online processes. No final gate or compliance completion is claimed here.

This maps the **30 agent rows** emitted by playbook `2dee6cc` when no matching
release NRO is supplied: one binary row plus 29 explicit agent rows.
It does not replace the full playbook checklist or mark the RC ready.

## Evidence keys

Paths beginning `artifacts/` are relative to the private campaign root
`/home/intpa/smash-training-lobby`, not committed game data.

- **D**: `artifacts/trainingrooms-nro-archive/slim/hitbox-demand-observer/`;
  main127f6f6 patch SHA-256
  `659ceae1b8c9b8f9d5541861e81bffbf2734d89b5e9aa50ca20da6a2dfcf1281`,
  integration manifest `5d413206727e6091a1631a9de9a0e7713e77c16eb3e4ff34d4a6ce38e375df74`.
  Five exact after-hashes applied by lead onto main620d3e8.
- **DH**: `artifacts/trainingrooms-nro-archive/slim/hitbox-demand-host/receipt.json`;
  13 passing host tests, binary
  `eb369a6e3662a3d17d8ad8444364826606915e2e94f40dedfc7b9ee56f7705e6`.
  This does not compile the Switch callbacks.
- **ABI**: `artifacts/trainingrooms-f6-worker-proof/helper-demand-abi/receipt.json`,
  SHA-256 `4e455c43f125bf16d705588068952d0b4689fda7fd54eb80a900eec3129f7a97`;
  F6 `0f1995e`, 42 files rehashed by Slim and lead. Bounded displaced-window
  and known-caller liveness only; arbitrary computed callers are not exhausted.
- **SH**: `artifacts/trainingrooms-nro-archive/slim/sh-01/receipt.json`,
  SHA-256 `f9af6e53f3b1ed032ae738ff78b322f5ea9ae0aab0e9a9d332982cc75d6a68a3`.
  Compiled source `5d92df4`, diagnostic NRO
  `e0374823c3ed4f01bc22a73e4574a00d1e29e699dedd71e3cd6036cf9dc19f4f`.
  First pass 53 successes/2 failures, then two retry successes: not first-pass
  acceptance. Full console zero Unmapped; 35 observed hook records zero/zero.
  Full restore 1600 profile and 19 owned harness entries; released 05:40Z.
- **UI**: `artifacts/trainingrooms-nro-archive/ui-screens/ui19-boundaries/`;
  patch `5e9a471d36c67598115495ff5f06c9707e1ebb9f9bc0a60fca51f5e0d9928ee3`,
  manifest `70441a42a1ae3abe286fa040640a2a2b248e49e255095ea9fa9a4f088776ecbb`.
  Sixteen targeted tests and serialized preservation checks; runtime pending.
- **CFG**: `artifacts/trainingrooms-nro-archive/slim/453e997-*` exact historical
  five-gate receipts; Configuration/Draft/Seed first/repeated allocation zero,
  allocating positive controls one. Not the final composition's gate wave.
- **P**: [IMPLEMENTATION-PROGRESS-slim.md](../IMPLEMENTATION-PROGRESS-slim.md),
  the full existing M/C checklist, source/build/run identities and disclosed
  process gaps. Shared assignments: `../TRAININGROOMS-LEAD-INBOX.md` in the
  campaign root; authoritative live claims: Yoga
  `/tmp/deluxe-surface-1007/OWNER-COORDINATION.md`.

## Thirty agent rows

“Open” means evidence or acceptance is still owed. Scoped evidence is not a
whole-candidate pass. Conditional release/card rows are outside this worker's
authorized scope; online rows remain open for integration disposition.

| # | Checker requirement | Status and evidence / next step |
| --- | --- | --- |
| 1 | Shipped NRO identity and diag strings | **Open.** Lead must insert final source/release SHA and run checker with that NRO. No substitute binary. |
| 2 | Test, clippy -D warnings, no-default, release, diag | **Open final wave.** CFG historical; DH/UI targeted. Lead owns coherent gates, including zero-warning clippy and both native builds. |
| 3 | Root fixes or prior operator-approved workarounds | **Scoped.** CFG removes duplicate storage/allocations; D only observes a remaining failure. Final root/workaround register must include unresolved shared defects without claiming fixes. |
| 4 | Dated card-copy site/vtable scan | **Scoped.** D retains saved16 NRO hashes and no-reference scan at a5020; no physical card access. Final all-site inventory review remains owed. |
| 5 | Hook sharing/chaining, stand-ins and load orders | **Open disposition.** D new site has no saved-inventory reference; that does not exempt other sites or prove arbitrary co-installation. Cite each applicable shared-site test or justified N/A. |
| 6 | Hook after prologue or call function instead | **Scoped.** D specific mid-helper site; ABI covers original stores/continuations. Final hook-by-hook rationale remains owed, including existing entry hooks. |
| 7 | Shipped NRO writes only install line / allowed data | **Open release audit.** D feature-gated diagnostics format after Request through async writer; this is not a complete release logging audit. |
| 8 | Online read-only rules hook by hook | **Open.** Observation nonmutation alone does not establish whole-plugin online/LDN safety. |
| 9 | Replay A/B with desync detector | **Open.** No matching final-candidate A/B receipt supplied here. |
| 10 | Two consoles, changed path, same-frame desync_seen=0 | **Open.** Offline Eden does not replace online/LDN evidence. No device work authorized to Slim. |
| 11 | Claim written/released including failed runs | **Scoped pass.** SH and DH retain admission/release; next run needs fresh ledger lines. |
| 12 | One Eden/host, >=10 GiB, not profile a/:61 | **Scoped pass.** SH preflight and own borrowed f6debug97 session. Next admission must recheck live host and queue. |
| 13 | Copied profile, hard-linked game, only copy changed | **Scoped.** P contains profile/inode proof; SH full mutable backup/restore. Requalify next borrowed profile before mutation. |
| 14 | Identical files linked, mutable files copied | **Scoped.** P profile-copy receipts; SH exact snapshots. Fresh next-run verification required. |
| 15 | Unique targets, shared Cargo home, disk >=40 GB, retirement manifest | **Scoped.** CFG unique frozen targets/retirement receipts; DH standalone bounded job. Final build owner retains current cache/disk/source receipts. |
| 16 | Waits gated on log/screenshot | **Scoped.** SH actual CSS/Training route and negative-pick screenshots retained. sh-02 must use fresh phase gates, not old timing alone. |
| 17 | Failed precondition abort / new label | **Scoped.** Prior failed lanes remain in P; no overwrite. sh-02 has a fresh output label and must stop on missing required gates. |
| 18 | Restore profile, remove temp state, release | **Scoped pass.** SH verified 1600+19 restore; no Slim live ownership. Next run must collect before restoring and release only after verification. |
| 19 | Full console panic/abort/crash scan, run.json, memory.peak | **Scoped pass SH.** Zero total Unmapped and retained artifacts do not repair its Hitbox first-pass failure. Final lane repeats complete scan. |
| 20 | Every local command under memory/time guard | **Disclosed failure.** Initial bootstrap and some short metadata commands lacked wrappers; later jobs bounded. Cannot retroactively label all commands guarded. See P. |
| 21 | Evidence doc identifies run/build/actions/log result | **Scoped pass.** D, SH, UI and linked reports distinguish source, binary and acceptance limits. Final build/run report remains owed. |
| 22 | GIF caption date/build, start/action/result | **Open; UI owns.** Real corrected-candidate capture, 1280 wide and under 10 seconds; no fabricated/placeholder media. |
| 23 | No operator quotes or overclaims | **Reviewed for this slice.** Final assembled README/evidence requires lead review; partial tests retain their limits. |
| 24 | Current handoff branch/commit/evidence/live state/next steps | **This slice.** Update to existing main root HANDOFF.md; final candidate identities pending lead freeze. Historical handoff preserved. |
| 25 | Version/tag/release/assets anonymous download | **N/A to worker scope; release pending.** No push/tag/release authorized. Lead must resolve applicability before publication. |
| 26 | Tagged rebuild byte-identical to released NRO | **N/A to worker scope; release pending.** No tag/release exists for this candidate. |
| 27 | Card MTP preflight/rollback/readbacks/nonrepeated receipts | **N/A to worker scope.** Physical card/DBI/USB prohibited; no installation acceptance claimed. |
| 28 | User controls connect/disconnect; no mount/eject/DBI control | **Pass for Slim actions.** No card/device operations; machine script scan passes. |
| 29 | No push/tag/card writes without order | **Pass for Slim actions.** Local commits and host evidence only. |
| 30 | Model/effort gate and owner-bound watcher | **Current pass, historical gap disclosed.** Exact thread Astra/medium and sole hidden service verified; initial watcher lifetime gap remains in P. No blanket continuous-coverage claim. |

## Machine rows and final closure

The source-only checker on clean Slim `4526e3e`, playbook `2dee6cc`, passed
all 863 guard initializers (all 64 bytes). It failed the README combined
content, absent GIF and absent root HANDOFF.md rows. Main620d3e8 already contains HANDOFF.md: its absence applies only to the
older Slim branch. This slice updates that existing main document; it does
not repair a missing main file. UI owns README and actual media.
No checker rerun is claimed for this documentation patch. Source-only output:
`tmp-slim/compliance-4526e3e-source-only.md` (exit 1, 30 agent rows).

The full playbook also requires final all-hook zero allocations, zero total
Unmapped, measured stack margins, size/carry/lint gates, proper rulesets and
the evidence workflow. Those remain required even when not separate rows in
the checker. Historical failures, including SH first-pass rejection and UI's
TraceWriter margin failure, must remain visible beside later regression results.

## Reviewed development evidence, 2026-11-03 07:50Z

The qualified diagnostic remains frozen r15 ab13a0f. Main96da3a4 adds the
count-promotion root fix and is not yet native-qualified. These are scoped
updates to rows3,11–19,21 and24, not final closures or a new checker run.

- main-r15/lead-sh02-audit.json independently verifies69files,32 zero-allocation
  hook samples and zero total Unmapped. Both original first-pass failures
  positively request11 emitters with8 available. Eventual retry success does
  not close Hitbox acceptance. Full1600profile/19harness restore is verified.
- main-r15/lead-ui20-audit.json independently verifies47files/90178211bytes,
  full7846-line console,165 zero-allocation hook samples, zero total Unmapped,
  contiguous diagnostics1..910 and full1362profile/14harness restoration.
  Startup notice contrast, developer toggle polarity and waiting-room text
  failed. Six saved-Mii constructor observations establish context only;
  coherent snapshot/provider ownership remains open. Failure GIFs are not
  corrected-feature media and do not close row22.
- main-r15/lead-f639-failure-audit.json preserves the failed early bootstrap,
  58files and full1600profile/18harness restoration. No accepted transaction
  or paired performance result occurred. Fresh41/42 use the same bounded
  catalog-and-owner qualification in both lanes and retain startup cost.
- main-r15/lead-count-marker-host-audit.json verifies all1203 worker source
  files, actual executed ELF19a41a7c and raw log5bd29f15: five targeted tests
  pass, zero fail/ignored,1277 filtered. Main patch78e82fff applies as96da3a4;
  all eight after-hashes and253 carried files match. Full native qualification
  and runtime count/lifetime regression remain owed.

All artifact paths above are beneath artifacts/trainingrooms-lead-astra4/.
Final release identity, full gates, all359 regression rows, online/LDN,
corrected media and explicit agent-row dispositions remain open.

## 2026-11-03 r16 development evidence update

R16 contains the count-marker root, displayed Developer control correction, notice palette/waiting-material fixes, and after-only Hitbox pool instrumentation. Lead independently verified source/binary/log identities, 28 UI and19 Hitbox component tests, all9 full64 observer guards, both exact layout inverses, stock textures and253 carried files. Integrated host gate passed1988 test executions across six binaries, zero failures/warnings and3ignored; shared model tests are repeated across binaries. One diagnostic native build passed with zero warnings. Sixteen thread tables and145 generic callbacks pass, but explicit modified formatter/native chains are pending. These are development gates, not the final release wave.

Evidence root artifacts/trainingrooms-lead-astra4/main-r16: candidate.json, lead-host-audit.json, lead-native-audit.json and stack-review/receipt.json. Source1329 archive17afb4f38f68f828127a37afd4b43d8316833188514bad277f76c08239042e41; NRO88abde8e01a5e4be11c6e31a7e25a302a7cd1daffda175d424a6c7ddc5769bf6; ELF7c3152adfc50425b8c3c312180988cedd229322464e5714a2f2b38187981029f. Generic prologue-only stack analysis is not a complete bound: Slim found an80-byte shrink-wrapped Capture::finish frame for explicit accounting. No measurement or allocation threshold was relaxed.

[The F6 development report](F6-R15-MATCHED-DEVELOPMENT.md) maps the completed41Off/42On pair and its missing41cache/nonfinal-source limitations. Lead verified27 future-driver inputs and actual inventory invocation before launch/after stop; the corrected wrapper remains runtime-unrun. Validation-only integration c61d4b0 passes14 matched/report and13 legacy Python tests. Frozen compiledr16 remains758cee5; changing validation tools does not change its recorded source identity.

Pkg4 p4c03 native timer/Replay saved route still fails capture completeness: native publishcounter4449, clean stop, imported tracks3600each and truncated=true. Lead independently verified1092 files, all4 payload hashes, full7219-line console0Unmapped, contiguous diagnostics and1362+14 restore receipts. Root review remains active. Evidence main-r15/lead-p4c03-archive-audit.json and lead-p4c03-payload-audit.json. Clean health/restoration does not close this failure.

All final release, full regression, online/LDN, media and remaining agent rows stay open. No new checker invocation or complete compliance claim is made by this documentation update.

### Qualification completed08:31 UTC
The preceding pending explicit-review state is superseded by main-r16/lead-development-qualification.json SHA4310ac12c95418cfc66828667905c6d823a62a14f83875912af66e559c377339. UI42/Hitbox101/count100 review files were independently verified; required conditional/typed/native local frames are included without relaxing thresholds. Development lanes are admitted as recorded in HANDOFF.md. Runtime, final release gates and complete compliance remain open.

### Source-only checker at main5664bf3, 2026-11-03 08:39 UTC

Playbook2dee6cc checker exited1: two mandatory machine failures (README GIF content and docs/media GIF),30 agent rows,894 guard initializers all64 bytes, clean source tree and root handoff present. Explicit nonexistent RELEASE-PENDING.nro prevented stale build auto-discovery. Raw report main-r16/source-compliance-5664bf3.md SHA256 06d448a947b81496adfcf99a102cb9f22711c0cdc7edb41ac5de2f5ec0bfd695. This report does not qualify the diagnostic as a shipped release, and the source test-attribute count is not an executed-test count. All thirty rows retain the scoped dispositions above; final closure is still owed.

The first r16 F9 runtime attempt s6-12 reports the corrected full identity completion followed by Samus body with Sonic HUD. The original mismatch was retained and the run stopped/restored; full lead archive audit is pending. No F9 closure follows from the count-marker repair. Pkg4 first-seal observation is authorized as bounded evidence collection only; timer truncation has no accepted root fix.

## R17 independent development audit, 2026-11-03 09:20 UTC

Lead assigned this read-only 25-minute audit at09:17. No compiler, Eden, profile or claim. Exact source a34097866a37d72eb7a701521219a86dfc01d9cd: all1335 frozen files rehashed before and after; archive639d8f10 and manifest133cd4bc independently match. Actual diagnostic NROa9855f609436bbe183eca3e2111297386c207dc337b43346e4baec78c0394970, ELF77e4f15c1ed44fd0fc401816b79fc67fa75746afdd13866ac4439d7b95e41ac6 and build log31ec070e rehashed. Mainb148b62 was clean; exact diff from compileda340978 is only12 added lines in eden/EDEN-PACKAGE-2-PLAN.md. It is not the compiled source identity.

Evidence in the parent workspace: tmp-slim/r17-compliance-audit/receipt.json SHA256919ddd794213f3f5e852b70ff4038626ea1842059f61e6e1f0e372c2b9c8c8b1 binds seven raw results and all four tool hashes. Playbook2dee6cc checker and full carry tools were read before use; jobs bounded512MiB/60s. Full source-to-destination carry passes41 package4,253 package6 and39 UI assets/ledger. Package6 emits three existing left-as-written probe-path notes; retained without claiming probe coverage.

Checker invoked with the ACTUAL DEVELOPMENT DIAGNOSTIC NRO exits1, raw report10b13a891a564374840545e60e721b121f973ccca36994641bff98e2dd22adc0. Four mandatory machine failures: README combined GIF/build/license row, absent docs/media GIF, no origin, no git status. Last two describe the frozen archive without Git metadata; independent main Git read verifies Forgejo origin and clean b148b62, while frozen identity comes from the manifest. Raw results are unchanged. The checker emits29 agent rows because the explicit NRO replaces its usual missing-binary agent row; the map below retains the existing30-row numbering.

Static894 guard initializers are all64 bytes; this is not proof of every live guard or hook safety. Static1408 test attributes are NOT test executions. Existing host log12e874e2 and BOTH actual executed ELF hashes independently match: lib1339 passed/2 ignored and thread_stacks222 passed, total1561 executions/0fail. This is targeted, not unique requirements or all-targets. No tests were rerun. NRO resident6242304B and absence of only diag.txt/standin byte markers do NOT turn a --features diag build into release or prove logging compliance. The checker card-script pass had no tracked archive files; its existing scanner was separately applied to all1335 manifest paths and found no prohibited commands. Likewise test-script presence does not prove every reported number was correctly calculated.

| Map row | Current disposition and smallest remaining evidence |
| --- | --- |
| 1 Binary | Development identity closed above; matching final RELEASE NRO and audit still owed. |
| 2 Gates | Targeted1561 and diag build scoped; final all-targets, clippy zero-warning, no-default, release/diag, size/lint/carry wave owed. |
| 3 Root fixes | Configuration roots historical; HUD source root integrated, timer first-seal is diagnostic only. Hitbox first pass fails; Q15 provider unqualified. Actual corrected runtime owed. |
| 4 Site inventory | Prior dated owned inventory/ABI receipts scoped; final changed-site/vtable disposition owed, no new card scan. |
| 5 Coexistence | Overlay final-input/delete_records conflict remains open; both orders/stand-ins and pilot coexistence need exact-candidate proof. |
| 6 Hook boundary | Existing scoped ABI/displaced-window review retained; final per-hook rationale and current compiled changes remain lead/owner reviewed. |
| 7 Release logging | Marker scanner insufficient; actual release cfg/logging path and shipped output audit owed. |
| 8 Online rules | Existing scoped source review only; whole changed-path hook-by-hook disposition owed. |
| 9 Replay A/B | Matching final candidate desync-detector A/B evidence owed. |
| 10 Two consoles | Same-frame desync_seen=0 past changed path owed; offline Eden cannot substitute. Device work not authorized here. |
| 11 Claims | SH03 admission/release scoped pass; no claim in this audit. Future r17 lane needs its own receipt. |
| 12 Host isolation | SH03 scoped preflight pass; no runtime in audit, no inherited r17 host admission. |
| 13 Profile copy | Existing SH03 copy/restore evidence scoped; new run needs own verification. |
| 14 Copy semantics | Existing immutable-link/mutable-copy proof scoped; no copies/profile mutations here. |
| 15 Build resources | Frozen unique r17 target recorded in candidate/job; historical build-owner receipts retained. No new growth/retirement or fresh census claimed. |
| 16 Gated waits | SH03 observed phase gates scoped; future runtime must independently gate inputs. |
| 17 Failed preconditions | Prior failed labels retained; no runtime retry here and no same-label overwrite. |
| 18 Restoration | SH03 full1600+19 plus separate verification/release scoped; no live Slim ownership. |
| 19 Runtime health | SH03 full12226 lines zero TOTAL Unmapped/fatal,29 actual hook records0/0,15 measured margins scoped. Not r17 or final RC coverage. |
| 20 Guards on commands | This audit bounded; historical short unguarded commands remain disclosed failure. |
| 21 Evidence report | This additive report binds current source/artifacts/checks; final corrected runtime report still owed. |
| 22 Media | UI raw default/custom notice clips scoped; frozen README/GIF machine failures remain. Actual corrected clip under10s/1280/date/build owed. Input-log contrast failure remains open. |
| 23 Claims | This audit distinguishes static/targeted/development from final. Final assembled README/evidence claim review owed. |
| 24 Handoff | Frozen HANDOFF remains historical. Lead updates the current root HANDOFF with r17 identity, reviewed outcomes, pending resource decision and exact next qualification steps; final accepted-candidate handoff remains owed. |
| 25 Publication | Outside worker authority; version/tag/assets/download disposition belongs to lead/operator. |
| 26 Rebuild | No released tag exists for this candidate; byte-identical tagged rebuild still pending publication scope. |
| 27 Card | Prohibited to Slim; no installation acceptance or physical operations. |
| 28 Device control | No device action. Independent manifest-script scan zero hits; does not attest other agents or hardware runs. |
| 29 Authorized writes | Only own audit output/report/local commits; no push/tag/card/DBI/USB/OpenCode. |
| 30 Model | Exact own thread Astra/medium and sole owner-bound watcher verified09:13; historical watcher gap not erased. |

Provider gap remains separate from constructor-trigger coverage: Q15 observer contexts prove triggers, not catalog writer exclusion. CPU DI/tech/OoS requires current-boot cpu_common_load with all four guards and cpu_common=installed; absence in a minimal three-plugin lane is unexercised. Native ancestry and foreign load orders retain their limits; any Unmapped anywhere fails. No new final RC row is closed by this audit, and READY remains unset.

## Main source/media increment, 2026-11-03 09:38 UTC

Main fc7a28f checker exited0 with zero mandatory machine failures and29 agent rows; raw report SHA256 d80fbd0bd00a0e4ef8e7808794d8fee8e73b74eb4b0aa70cf821b11cae580a5a retained in main-r17/main-fc7a28f-source-compliance.md. README and the actual95-frame/9.5-second/1280-wide r16 waiting-menu clip now meet the mechanical media rules. Lead viewed decoded first/middle/last and verified pixel equality, exactthree afterhashes and original source identity. This is scoped development footage; input-log contrast and final release media remain open.

The checker automatically inspected the historical dist/release NRO c4750559c52a431a; its identity/string rows DO NOT qualify current source, r17, or a final release. The1408 test attributes are static inventory. All30 existing explicit dispositions remain except this scoped media increment and the independently reviewed changed-path audit in R17-ONLINE-RELEASE-DELTA.md. The Training guard correction2dbfe6e has source review/full253carry proof; next binary/runtime owed. This lightweight checker lacked a separate command memory/time wrapper; the command-guard failure remains disclosed. No RC gate or final regression row is closed.
