# F6 r15 matched development evidence — 2026-11-03

The bounded development comparison is complete. **Final F6 performance and release acceptance remain open:** f6-41 lacks cache inventories, order was fixed Off then On, and r15 is no longer the final integrated source. Lead accepted this limited development evidence at 08:04:31Z. No new performance fix or further runtime run is inferred or authorized by this report.

## Identity and execution

- Compiled source: `ab13a0f96397e8fe9b89715a46f046d0006eff8f`, from `../artifacts/trainingrooms-lead-astra4/main-r15/lead-development-qualification.json`.
- Diagnostic NRO SHA256: `0bbcfde5782d91d495dbea008b4495c332d901ec03d06b015e72fb448084cc50`.
- ELF SHA256: `4b68ec2a3fb2d9fe25d3af71107c49b4f89b04727c7107fd7f3db2be49c7b19f`.
- All 1,321 archived source hashes, qualification audit files, archive, NRO and ELF independently verified. F6 worktree HEAD is a validation-tool revision, **not** the NRO's source identity.
- Actual pair: **f6-41 native Off / f6-42 On**. Lead replaced the original 39/40 labels after failed39; cancelled38 and r14 are not paired proof.
- Run freeze: `../tmp-f6-worker/matched-r15-qualified-freeze.json`, SHA256 `68cc1fc5e12fbe36fa85ef5c84d89238a69ed95cf37856b2e8424ec792fc8b97`.
- Same Surface `f6debug` profile, display97, Kongo stage6, visual and native Steve58 / Ridley42 gates. Off slots0/bank0/groupOff; On slots5/bank3/groupOn. Shared six-second post-observed-ready cadence, 1s polling, 2 bootstrap + 11 P1 + 4 CPU transactions; no skipped or retried laps.
- Normal queue admission; no compiler beside Eden; volume0 saved/read back before game load. Eden 8GiB/3600s, controller512MiB/2700s. Pair claim07:44 through final restore/release08:00, within the 45-minute box ending08:29.

| Evidence | f6-41 Off | f6-42 On |
| --- | ---: | ---: |
| Strict role/ticket/target/released transactions | 17/17 | 17/17 |
| Whole-console TOTAL Unmapped | 0 | 0 |
| Hook allocation/delta samples, every value zero | 32 | 30 |
| Maximum observed roster_input calls | 16,936 | 16,564 |
| Workers with measured policy-compliant stacks | 15 | 15 |
| Observed Eden memory peak, bytes | 6,835,187,712 | 7,505,281,024 |
| Restored mutable-profile / harness entries | 1600 / 18 | 1600 / 18 |
| Stopped and released | 07:50Z | 08:00Z |

Both logs contain a post-final-lap hook sample. No panic/abort/crash or OOM was observed. Eden's deliberate stop is `interrupted`, exit1; this is not a game crash. The 42 outer wrapper returned143 after restoration/release/queue removal; fresh full-ledger, process and queue checks confirmed no owned resource remained. The raw exit is retained, not called wrapper success.

## Qualification and original failure

Run39 failed before its first accepted transaction. Raw #240/#241 input/handoff preceded #249 catalog-ready and #250/#251 owner qualification. `mii_ordinary_player::intercept` lines280–306 consumes an input while `origin_capture_pending` without enqueueing it; the loading notice asks the user to retry. “Handed to owner” is not an accepted transaction. The driver aborted with `GATE FAIL: matching cycle transaction owner=0 missing after cycle` and fully restored/released. All failed evidence remains under `f6-39-failed`.

Validation fix `4af99c6` requires the latest diagnostic session, intended latest physical fighters, matching queued/ready catalog generations/opening, and both qualified owners before input in **both** fresh lanes. It times out without input; no fixed warmup, extra cycle, speculative bank gate or accepted-lap retiming was added. Eight offline tests cover original log order, partial/old boot markers, timeout, exact transactions and unchanged cadence.

| Qualification interval | Off | On |
| --- | ---: | ---: |
| Guest boot to both owners qualified | 209,425ms | 178,194ms |
| First physical observation to both owners qualified | 4,500ms | 4,236ms |
| Driver qualification wait | 2.001s | 0.000260s |
| Host CSS Plus to observed qualification | 14.491s | 24.440s |

Boot duration includes human CSS selection. CSS Plus is explicitly not the guest GO frame. On still requires its pre-existing delivered preload notice; that gate explains why qualification may already be complete when checked. This measures the first **accepted** swaps, not arbitrary early-input performance.

## Timing result and limits

These are single-pair **development observations with missing Off cache evidence and fixed Off-before-On order**, not final performance acceptance.

| Matching G&W→Kirby transition | Off request→ready | On request→ready | Off contained gap | On contained gap |
| --- | ---: | ---: | ---: | ---: |
| First, P1 press3 | 2199ms | 1615ms | 233,796µs | 135,848µs |
| Repeat, P1 press11 | 1486ms | 53ms | 221,739µs | unknown/censored |

Request timing uses guest adapter submit→released-ready at 1ms resolution, not host polling. First On is584ms shorter than first Off; repeated On is1433ms shorter than repeated Off. However, first On request1615ms is129ms longer than warmed plain Off1486ms. Request duration and stall maximum are distinct criteria.

The first unprefetched `[Kirby,Falcon,DarkPit]` group has local setup3309µs, control56µs, queue758µs, settle5587µs, group maximum tick gap135868µs and group request→ready1612751µs. The first group's setup and measured gap are below the plain Off repeated-Kirby observed221739µs gap **in this limited pair**. Local setup/queue spans exclude queued worker execution and do not prove total-latency or zero-hitch acceptance. Common owner0 gap135848µs and group-local135868µs are different measurement boundaries and remain separate.

All retained rotated logs are included. Common-gap rows use conservatively fully-contained intervals, preserving raw timestamped lines. Only gaps strictly above60000µs are logged; no On repeat sample means unknown/censored, never zero. The JSON names `cold_kirby_builds` and `cold_bootstrap_to_first_kirby` mean first unprefetched **group** work, not cold shader/disk caches. No broad cold-cache claim is made.

## Cache omission and future correction

The frozen inventory helper existed but the run wrapper did not invoke it. Earlier preparation claims about invocation are withdrawn. The mutable-profile snapshot excludes `user/shader`, `user/cache` and `xdg-cache`; these caches were preserved, never purged. Missing41 pre/post hashes cannot be reconstructed.

Lead approved proceeding42 as development only. Separate inventory ran at07:54:28.869 before42 and08:00:27.235 after stopped/restored42. Both saw absent user/shader,24 user/cache files and4 xdg-cache files; inventories differ. Before SHA256 `f1ca67d44acd3559e20edc91d43482ebc6084a1a0e9e82da727f5cdef09554de`; after `779dbdd03ab37bfee37a2ff42382c7e195b660aa61de21b25967af285bfb4a81`. These do not establish41 cache identity.

After clean42 release, future wrapper now invokes inventory before launch and after stop. Before failure prevents launch; after failure leaves a retained error and continues collection/restoration/release. Future auditor requires inventory by default; historical exception is explicitly named development-only. Three offline checks cover the original omitted invocation, actual future command order, and executing the inventory against a temporary cache with hashes and no mutation. PowerShell syntax passes; actual41 is rejected by the new default audit for missing inventory. Six reporting tests pass, including explicit missing/error inventory limits. These are offline checks, not a fresh Eden validation of the corrected wrapper.

Future-only freeze: `../tmp-f6-worker/matched-future-cache-freeze.json`, SHA256 `c5e0cdd112a36f367e57de08e89c33d2b9071a6f8479555c1b12d58711696028`. Archive28 files in `future-cache-preparation`, receipt `4a265ba2be06194ce9bdbb2749a2db6f11cf2f81d6559e15108066928364d492`. This cannot retroactively qualify41/42. A final coherent candidate requires a newly admitted pair with both inventories actually executed and reviewed.

## Retained evidence

Root: `../artifacts/trainingrooms-f6-worker-proof/`.

| Artifact | SHA256 |
| --- | --- |
| f6-39-failed/manifest.json,58 files | 3ed3eb8d2225e67b724583ea3ee1b892b621be71c54987ca0a48f6ddebb09044 |
| f6-41/manifest.json,58 files | 99a238fbd674242975119b529a7204a3c69cac09a645326415e2cc2cbe756472 |
| f6-42/manifest.json,63 files | 1ce91b41b625cb004bded5df7ec803857c8f4b2d82235e1b132a45169d2d8397 |
| Original r15-41-42-development-comparison.json | d9fa607028da3e5b50ba8489bbd55ab86dfe75fdf835357595e4af39a80886e2 |
| Additive r15-41-42-development-comparison-scoped.json | 30d66d96bb4a4a70212cbc57a79037e55f277a117d9ec20e7a275efa001a0899 |

The original report was temporarily regenerated with cache metadata before the lead's additive-only instruction. Its exact original bytes/hash were restored and verified; the corrected scoped report is separate. Raw run archives/proofs were not rewritten. Full console hashes: Off `afd21e2b2a63e066086cce72a1eb94ac6c536904ab21318d6d35db6e2a4cde99`, On `bc856a20209ca9b18c1db469e3212d66da0592bdca8fa5b0f55e2a8c3e83c5fe`. Lead independently audited both archives and recomputed timing in `main-r15/lead-f641-audit.json`, `lead-f642-audit.json` and `lead-f641-42-comparison-audit.json`.

## Playbook scope resolution

Raw checker: `../tmp-f6-worker/r15-successor-compliance-raw.md` (before final report), final raw checker is retained separately. Three old-worktree packaging failures (README build/GIF/license combination, missing docs/media GIF, missing HANDOFF.md) are **open**, not waived. The designated progress/handoff is current, but does not make the checker's exact HANDOFF.md test pass. Its diagnostic-string heuristic pass does not make this diagnostic NRO a release binary. No final Cargo/native/online/media/release gate is claimed by this validation-tool task.

| Agent row | Scoped disposition |
| --- | --- |
| Cargo test/clippy/no-default/release/diag gates | OPEN final candidate; assigned r15 qualification was independently verified. No Cargo/native source changed or new build run here. |
| Root cause or approved workaround | PASS driver cause and gate regression above; cache omission corrected for future runs, not retroactively accepted. |
| Dated card site/vtable scan | N/A new scan: no hook or native site changed; card access prohibited. Existing r15 qualification retained. |
| Hook sharing/chaining/load orders | OPEN final release; no hook change, only supplied pilot/tile companions used. |
| Hook after prologue/called function | N/A this change: no hook/native code edits. |
| Shipped NRO writes install line only | OPEN release; this is explicitly a development diagnostic NRO, not shipped. |
| Online read-only review | OPEN lead-owned; no online test or change here. |
| Online replay/desync A/B | OPEN lead-owned; not executed. |
| Two consoles identical-frame desync | OPEN lead-owned; not executed. |
| Eden claims/releases including failure | PASS39,41,42 full ledger and restores. |
| One Eden/host, memory floor, private profile | PASS Surface exclusive, preflights above10GiB, f6debug/:97. |
| Private profile/hardlinked game copy | PASS existing admitted private harness; mutable profile alone modified/restored, no new game copy. |
| Linked identical vs copied mutable data | PASS existing harness policy; mutable backup copies verified by manifests. No content-link changes. |
| Target/source/cache/disk policy | N/A build targets; runtime preflights above40GB. No compiler or target deletion. |
| Log/screenshot gated waits | PASS visual selection, native fighter, qualification and exact transaction gates. |
| Failed precondition abort/new label | PASS39 retained; lead approved41/42, no39 retry or40 pairing. |
| Restore/temp cleanup/host release | PASS both1600/18 manifests, no owned runtime/control/claim/queue. Evidence/backups deliberately retained. Watcher reaped at final yield. |
| Full console/run/memory evidence | PASS both archives; zero TOTAL Unmapped/all hooks0/no OOM, raw wrapper143 disclosed. |
| Every command memory/time guarded | PARTIAL: runtime/tests/heavy commands guarded; some short read/git/bootstrap commands were not systemd-wrapped. No universal compliance claim. |
| Dated evidence with labels/build/actions | PASS this report, raw receipts and progress. |
| Captioned GIF start/action/result | OPEN final media; no GIF requested or produced in this bounded pair. Selection/Training PNGs retained. |
| No quotes or claims beyond tests | PASS scoped development observations and explicit limits. |
| Current handoff | PASS designated IMPLEMENTATION-PROGRESS-f6.md and SUCCESSOR-LIVE-STATE.md; exact HANDOFF.md packaging requirement remains OPEN. |
| Version/tag/assets anonymous download | N/A no release authorized or performed. |
| Tag rebuild byte-identical | N/A no release/tag; exact assigned NRO independently hashed. |
| MTP/rollback/readback receipts | N/A card/DBI/USB prohibited and untouched. |
| User connects card/no mount/DBI changes | PASS no card/USB/DBI action. |
| No unapproved push/tag/card write | PASS none performed. |
| Model/effort gate/watcher | PASS exact thread Astra/medium verified, one hidden owner-bound watcher; no model change observed. |

Next action belongs to the lead: integrate validation changes as appropriate, qualify the next coherent candidate, then admit a fresh final pair using the corrected cache wrapper. No current resource claim or pending run exists.
