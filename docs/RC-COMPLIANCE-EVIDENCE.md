# RC compliance evidence map

2026-11-03, reviewed packaging map updated by the lead. Latest development compilation is frozen r16 758cee5; root HANDOFF.md records
exact source/diagnostic identities and targeted gates. Modified-path qualification completed08:31 UTC; r16 runtime lanes are underway.
Their results do not establish final-candidate acceptance. The final release candidate remains unaccepted.
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
