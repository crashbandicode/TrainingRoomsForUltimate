# Provenance

| Generated record | Data |
| --- | --- |
| feature-groups | [Text ledger](../tools/provenance-data/feature-groups.txt) |
| pkg4-carry-ledger | [Text ledger](../tools/provenance-data/pkg4-carry-ledger.txt) |
| pkg6-carry-ledger | [Text ledger](../tools/provenance-data/pkg6-carry-ledger.txt) |
| pkg6-layers | [Text ledger](../tools/provenance-data/pkg6-layers.txt) |
| provenance-files | [Text ledger](../tools/provenance-data/provenance-files.txt) |
| ui-item-assets-ledger | [Text ledger](../tools/provenance-data/ui-item-assets-ledger.txt) |

<a id="hb1-carry-ledger"></a>

<a id="hb1-carry-ledger--hb1-carry-ledger-worker-c-branch-p23-hb1-2026-11-04"></a>

## HB1 carry ledger — worker C, branch `p23/hb1` (2026-11-04)

| Carried fact | Source (commit / doc) | Where it landed |
|---|---|---|
| Draw hurtboxes, yellow; shapes read per fighter per frame on the hitboxes' per-frame entry; cost measured in the diag build; help text | operator decision HB1 (a), 2026-11-02, `docs/DECISIONS.md`, spec update `dcbb2d1` | `src/model/hitbox.rs` (rules + reader + dispatch), `docs/HB1-RENDER-ROUTES.md`, `eden/phase23/rows-C.md` |

What this branch re-implemented, and from where. Nothing was copied as bytes
from another branch: the RC7 R3-2 removal (cab6603 on `stream/slim-rc7-r3`)
took the runtime HB1 reader out of an older architecture, and `work/
implementation` (42b510c) refactored the planner/dispatcher into the model
behind `AttackReader`; the hurtbox side was rebuilt in that architecture.

| Carried fact | Source (commit / doc) | Where it landed |
|---|---|---|
| Draw hurtboxes, yellow; shapes read per fighter per frame on the hitboxes' per-frame entry; cost measured in the diag build; help text | operator decision HB1 (a), 2026-11-02, `docs/DECISIONS.md`, spec update `dcbb2d1` | `src/model/hitbox.rs` (rules + reader + dispatch), `docs/HB1-RENDER-ROUTES.md`, `eden/phase23/rows-C.md` |
| HitModule layout (accessor `+0xb0`; groups `+0x60`, `0x3c`/group, first `+4`, count `+6`; parts `+0x70`, `0x50`/part, status `+0`, offsets `+0x08`/`+0x14`, size `+0x20`, node `+0x28`; total `+0x78`; status 3 off) and the three relied windows (`0x203db10`, `0x469af0`, `0x468490`) | owned ELF, `docs/test-provenance/core-06.md:116-119` at `2550ef88`; guard bytes as in `stream/slim-phase3-rc6` `sites_hitbox.rs` | layout constants already in `model::hitbox::layout` (historical), re-read by `want_hurtboxes`; the three guard rows are NOT re-added yet — recorded in `docs/HB1-RENDER-ROUTES.md` as required with the wiring |
| Sphere counts per fighter (Steve 19, Luigi 23; hb-03), Ridley's first pass (55 wanted, 53 asked, 2 pool-refused, then `failed=0`; hb-04) | `stream/hitbox` `eden/EDEN-HITBOX.md` (Eden, 2026-11-02) | `core06_ridley_first_pass_completes_in_the_model`, `eden/phase23/rows-C.md` C1/C3 |
| lr02 part geometry (status 0, centres (0.6,0,0)/(2.4,0,0), radius 2.4, bone `0x51a36341b`; group first 0/count 10) | `docs/HB1-LR02-DISPOSITION.md` (stream/slim-phase3-rc6) | test fixtures' part geometry (synthetic tables built from it) |
| Option A negative result and the A/B question going back to the operator | `tmp-slim/phase3-rc6/OPTION-A-DESIGN.md` (2026-11-03) | `docs/HB1-RENDER-ROUTES.md` (no route picked) |
| The RC7 CPU-role lifetime correction the above composition depends on | composition `1769bdd` (ancestor of the baseline), `docs/RC7-R3-1-VALIDATION.md` | reported in `artifacts/phase23/C-REPORT.md`; no code change |

Files this worker changed: `src/model/hitbox.rs`,
`src/runtime/features/hitbox.rs` (comments and the `None` argument only),
`tests/core06_hitbox.rs`, plus new `docs/HB1-RENDER-ROUTES.md`,
`eden/phase23/{rows,trace}-C.md` and this ledger. `sites_hitbox.rs` and
`tests/capabilities.rs` were left untouched (not in this brief's owned list).

<a id="hb1-carry-ledger--c2-runtime-reader-census-restored-rows-session-tr-glm-hb1-reader-2026-11-04"></a>

### C2 — runtime reader, census, restored rows (session tr-glm-hb1-reader, 2026-11-04)

| Carried fact | Source (commit / doc) | Where it landed |
|---|---|---|
| The three relied `hurtbox_*` guard rows, bytes exactly as before the RC7 removal | removed by `cab6603` ("exclude deferred HB1 from RC hitbox dispatch and guards"); bytes recovered from its parent `719255d`, where they had been generated from the owned 13.0.5 ELF by `tools/gen_hitbox_sites.py` (the generator's three rows restored too) | `src/model/sites_hitbox.rs` (`SITES` 18→21; consts `HITBOX_HURT_{MODULE,GROUPS,PART}_OFFSET`), re-checked against the owned ELF by `core06_rows_match_the_owned_elf` |
| The checked-pointer live reader: type-check every pointer (null/misaligned refused, 0) because Eden answers 0 for an unmapped read | playbook EDEN "Read the results"; the runtime's raw-read pattern (`GameAttacks`) | `model::hitbox::GameHurt` (reads bound to one `load`, host-probed over a fake layout at exact addresses), runtime `stats::mem_load` |
| The windowed diag-line pattern (accumulate, close a window, one formatted line) | `model::hook_stats::Observer` + `Display` (review R5 precedent) | `model::hitbox::HurtCensus`/`HurtWindow`, runtime `stats::CensusBook`/`hurt_census` (diag only), line `[Hitbox] hurt census kind=… parts=… parts_max=… first_pass=… frames=… ticks_min/avg/max=…` |

- Not carries but this session's decisions, for the record: `WANT_SCRATCH` (hit + hurt spheres together, `model::hitbox`).
- Hitbox's `CROSS_NEEDS` gains `utility_get_kind` — the census names each line with the fighter's kind through the already-guarded `utility::get_kind` row (diag-only caller; the release build calls no native and reads no HitModule; reverting is one identifier in the census line if the lead prefers an accessor-keyed line).

<a id="hb1-carry-ledger--c3-route-b-wired-session-tr-glm-hb1-routeb-2026-11-04"></a>

### C3 — route B wired (session tr-glm-hb1-routeb, 2026-11-04)

| Carried fact | Source (commit / doc) | Where it landed |
|---|---|---|
| Route B: hitbox, grab-box and hurtbox spheres drawn the fork's way, shared-pool `sys_shield`; hurtboxes yellow; refusals counted and logged | operator decision 2026-11-04 ~06:5xZ, `docs/DECISIONS.md` HB1-route, `review/menu-proposal` commit `5ca5e3b` (reverses the 2026-11-03 option A default; route A stays later research) | the runtime dispatch (`frame` constructs `GameHurt` behind `enabled`, both builds), spec help-text line, rows-C C2/C3 |
| The persistent-sphere HandleSet semantics the wiring reuses unchanged: one request per want per pass, refusals counted (`Counts::failed`), a refused want asked again on a later pass once the pool has room, never a respawn loop | `stream/hitbox` `src/model/hitbox.rs` `HandleSet` (proven Eden hb-03; refusals recorded hb-04: 53 asked, 2 refused, `failed=0` after) | already ported in C1 (`model::hitbox`); the Ridley test now pins one attempt per refused want per pass and the recovery |
| The C3 release build read the HitModule only while the setting was On | the C2 release-silence gate, re-formed for the wired route | test `core06_release_reads_the_hit_module_only_while_the_setting_is_on` (source scan) + the built-NRO string check at the gates |
| Help text "On: hitboxes, grab boxes and hurtboxes are drawn."; §8 cost row pending until the Eden census | the brief (task 1), rows-C C4 | `specs/SPEC-core-06-hitbox-visualizer.md` (help-text line, §8 note) |

Not a carry: the runtime read (`mem_load`) moved from the diag `stats` module
to the module level — the same raw read C2 host-probed, now reachable in the
release build behind the same reader type-checks. No new native, no new site;
the three `hurtbox_*` rows stay relied on, never written.

<a id="pkg6-active-reload-render-review"></a>

<a id="pkg6-active-reload-render-review--active-fixed-seat-reload-has-no-qualified-parked-render-boundary"></a>

## Active fixed-seat Reload has no qualified parked render boundary

| Build | Finding |
| --- | --- |
| Package 6, 110fb77 | Review found no qualified parked render boundary. |

Bounded review of the existing transaction at pkg6 `110fb77`, requested by the
lead at 2026-11-03 04:57:29Z. This returns a contradicted proposal, not an
activation workaround or an operator restriction.

The native install does deactivate the outgoing body, but that does **not**
offer a completed inactive replacement on which the current portrait renderer
may run. The existing managed transaction publishes exact Mii data during
loading and accepts its resource completion only after native activation.

1. Source admission validates the outgoing active identity and absence of live
   consumers. `begin_promoted_reload` creates a new Loading SourceToken under
   the existing INSTALL owner. The old promotion is not the new source's
   completion proof. `mii_managed_helper::prepare_reload` uses the existing
   exact G snapshot and selects `Flags::Reload`, bytes `[1,0,0]`.
2. Owned `653240` stages the descriptor into FE+70 at `6533a8..6533e8`, calls
   `653490(entry,1)` at `6533ec`, then writes outer=3 at `653424`, the Reload
   flag to FE+5931 at `653430`, and loader=0 at `65343c`. This is an inactive
   **loading** interval, not parked-ready resource completion.
3. The existing 650010 dispatcher performs plugin pre-service, then original
   native service, then gate leave. Exact Mii prequeue publication requires
   loader6 and its current ticket; `mii_selector_native::publication` requires
   the actual loading snapshot. Refusing this native callback is a retained
   terminal ticket, not a reversible pause that creates a parked lifetime.
4. `mii_managed_helper::completed_details(active_reload)` requires
   `native_source::active_identity`. `service_owned(active_reload)` likewise
   requires active identity and `ready_active`, then full G publication/job
   completion, before SourceBook.complete, a new promotion, registry proof and
   INSTALL release. `EntryState::ready` requires outer6, loader14, guard0 and
   the complete canonical body/index/count shape. No source ReadyToken is
   published for an intermediate inactive replacement.
5. Existing owned-native HUD design already directly rejects rendering inside
   this hot-reload interval: `mii-hud54-design/DESIGN.md` section2 and
   `ROOT-FLAG-RESPONSE.md` section4 show the renderer runs both 653490
   deactivations and 655f60/656ee0. During loading it forces outer3→6→0 under
   the loader; after completion it deactivates the live body. Template-write
   suppression does not suppress those effects. Keeping the old HUD request
   alive protects its texture, but does not protect the reloading fighter.

Therefore the proposed use of an **already existing** inactive-ready phase is
not supported by this transaction. Initial parked pre-render remains viable;
active exact-Mii target changes still require a different qualified root
contract. No active render, new hook, native flag write, forced lease release,
count freeze or fourth-created-row assumption was added.

- Evidence: exact owned ELF `0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2`; bounded fresh excerpts `tmp-pkg6/reload-install.asm` and `reload-service.asm`; 12-entry loader jump table read by `tmp-pkg6/reload-jump-table.py`; existing `artifacts/enhancements/mii-hud54-design/{DESIGN,ROOT-FLAG-RESPONSE}.md`.
- An attempted LLVM `-s` range dump ignored its range and emitted excessive output; it was discarded, not used as evidence.
- The corrected table reader reads only the ELF header/program headers and 48 table bytes.
- No compile, Eden, profile mutation or new native activation was performed.

<a id="pkg6-copy-roster-lifetime"></a>

<a id="pkg6-copy-roster-lifetime--copy-package-roster-pointer-lifetime"></a>

## Copy-package roster pointer lifetime

| Review opened | Revision |
| --- | --- |
| 2026-11-02 | 743d2b0d |

Historical review opened 2026-11-02 (`743d2b0d`). Implementation details and outstanding checks refer to the revisions reviewed below.

Owned 13.0.5 ELF SHA256:
`0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2`.
Complete shell static disassembly retained under
`artifacts/trainingrooms-nro-archive/pkg6/copy-roster-lifetime/`.
This is static evidence, not runtime acceptance or REA Evidence.

- Acquire `main+0x17efb40..0x17eff9c`: the incoming roster x1 is copied to
  x20 at 0x17efb58. Reads use its eight 0x80-byte records and kind/costume fields.
  The only call/tail target is 0x17effa0, receiving G68 in x0 and scalar
  costume/kind in w1/w2. The roster pointer is neither stored nor forwarded.
  Tail exit 0x17eff8c restores callee-saved x20; the other exit returns at 0x17eff9c.
- Release `main+0x17f0290..0x17f069c`: x20 receives x1 at 0x17f02c0;
  x24 addresses records during the Kirby loop, ending at offset 0x400.
  The only call/tail target is 0x17e2fe0, receiving G68 and the constructed key.
  No roster pointer is stored or forwarded. The exits return at 0x17f067c or
  tail-call at 0x17f069c after restoring callee-saved registers.

Thus each call may use a call-local 1024-byte native-layout buffer. The retained
`Acquisition` value contains all roster fields, member and costume needed to
reconstruct the identical release arguments. Acquisition-before-release order
and lock-free native calls remain unchanged. The compiled stack gate is still
required; this finding does not authorize enlarging any stack budget.

REA CLI 6.4.0 was attempted once under 1 GiB/60 s and returned
`{"code":"UNKNOWN","message":"write EPIPE"}`. No REA result was obtained.
Existing llvm-objdump produced the complete listings; initial shorter release
ranges were extended through its final tail exit before drawing this conclusion.

<a id="pkg6-count-activation-marker"></a>

<a id="pkg6-count-activation-marker--s6-11-count-activation-retains-the-completed-install-marker"></a>

## s6-11 count activation retains the completed install marker

| Review opened | Revision |
| --- | --- |
| 2026-11-03 | 96da3a4c |

Historical review opened 2026-11-03 (`96da3a4c`). Implementation details and outstanding checks refer to the revisions reviewed below.

Qualified r15 ab13a0f, NRO 0bbcfde5782d91d495dbea008b4495c332d901ec03d06b015e72fb448084cc50, failed the first count1→2 boundary. Raw archive `artifacts/trainingrooms-nro-archive/pkg6/s6-11/receipt.json` SHA bbb194ab8909e2e7ffb6981f13360291f2fdc50f81e9b8777f1ec2052c300d43 contains the complete run and actual screenshots. Count303 reserved generation1/epoch1/serial1 for entry77644931072. Completion308 observed canonical Samus state except changing=1; actual Samus body retained Lucario HUD. No publication followed. This is a retained failure, not acceptance.

Owned main13.0.5 ELF0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2, bounded `tmp-pkg6/s6-11/native-entry-state.asm`, establishes the distinction:

- Install653240 stages the descriptor;6533bc sets w22=1 and653438..444 writes that value to FE+5934. Nonactivating completion seals the fully loaded parked source.
- Native count155c950 calls656ee0 directly. Its ordinary body branch656f3c activates the body and656fb4..bc sets outer6. The count path does not call the full activation wrapper655f60.
- Full activation wrapper calls656ee0 at656134 or656520, then656530..534 clears FE+5934. This clear is outside656ee0. Loader650010 returns unless outer3; count has already set outer6.

Therefore EntryState::ready's changing0 requirement describes full active Reload completion, not direct count activation of a completed parked install. No native flag should be cleared by the plugin and no extra activation call is appropriate.

- The correction introduces a separate count-promotion shape.
- Only a current completed source with a reserved PromotionToken uses it; its full native lifetime must still equal the sealed identity.
- The completed transaction's existing active_reload purpose selects the contract: parked installation may retain marker1 or reach0 after a native reset; active Reload requires0.
- Values above1 remain invalid.
- Every other canonical field, each body's owner/kind/phase/accessor/vtable, resource kind and all sealed lifetime identities retain their checks.
- The strict general active_identity, new active Reload completion, and unmanaged native-seat fallback remain unchanged.

- The same purpose travels atomically with the existing Mii promotion receipt under the existing registry lock, so exact G consumers validate the same contract without taking STATE under G.
- Count completion, retained-seat admission, outgoing promoted reload admission and retained portrait consumers use the promotion shape.
- Fresh load completion never uses it.
- No new hook, flag write, native call, forced consumer release, or kind-only HUD publication is added.

Original host regression uses the actual s6-11 state, strict-reload negative control, both completion purposes, native reset marker0, and thirteen malformed/loading/parked shapes. Targeted execution and a new coherent native build/runtime remain required. This document does not close F9, cut-over, exact-Mii or lifetime acceptance.

<a id="pkg6-count-diagnostic-bound"></a>

<a id="pkg6-count-diagnostic-bound--count-promotion-diagnostic-boundary-review"></a>

## Count promotion diagnostic boundary review

| Record | Conservative bytes |
| --- | ---: |
| count Observation | 1273 |

Historical review opened 2026-11-03 (`ab13a0f9`). Implementation details and outstanding checks refer to the revisions reviewed below.

- The existing count188 and completion callbacks keep their native predicates.
- Both new hook observers now return immediately unless `runtime::diag_log::async_writer_ready()` is true, before touching the changed-tuple mutex, consuming a budget, formatting, or calling diagnostic.
- Main620d3e8 already owns this API (Slim23ec705); this old worker branch does not duplicate its shared logger.
- Native qualification therefore applies to the exact composed main candidate.
- Boot registration remains the existing boot logging path.

Main logger WRITER has one store, true/Release, and readiness loads Acquire. A true observation stays true through diagnostic: its pre-writer synchronous drain branch cannot run from these observers. Observations still format after source and limiter guards drop. Pure 0654cf0 three-test evidence remains valid for unchanged limiter/dispatch code; these two Switch-only guards are not host runtime evidence.

<a id="pkg6-count-diagnostic-bound--conservative-line-bound"></a>

### Conservative line bound

| Record | Conservative bytes |
| --- | ---: |
| count Observation | 1273 |
| completion PromotionObservation | 1405 |
| timestamp + sequence prefix | 43 |
| cut marker + newline reserve | 10 |
| maximum required ring room | 1458 |
| existing diag LINE_MAX | 12288 |

- Receipt and expanded upper strings: `artifacts/trainingrooms-nro-archive/pkg6/count-format-bound/`; generator `/tmp/pkg6-count-format-bound.py` (retained in handoff).
- This is a static derived-Debug schema bound, not a native formatter execution.
- Every unsigned field, even u8/u32, is charged twenty decimal digits (target usize64).
- All field/type/punctuation strings follow the actual derived Debug structures.
- Only fixed ASCII outcome and body-field literals are allowed; the generator asserts their maximum source lengths.
- SourceToken includes all four fields; PromotionToken includes its full SourceToken again; each NativeIdentity includes all three body pairs.
- Count has both helper and both source/promotion tuples.
- Completion compares both expected and maximum actual Ok(identity)/Err(failure) shapes, including full EntryState.

- StackLine512 is a first formatting attempt, not a512-byte line cap: on overflow `diagnostic` passes original Arguments to ByteRing::push, which formats directly into the ring.
- No larger stack buffer or allocation is introduced.
- Available ring room can still be smaller than LINE_MAX; dropped/truncated counters, `[cut]`, missing record endpoints and observer budget exhaustion must remain explicit unknown/failure during runtime review.
- The bound proves no maximum-line truncation with sufficient ring room; it does not claim capacity or callback-stack acceptance.
- Lead must inspect compiled callbacks/Debug frames and fresh measured margins before runtime admission.

<a id="pkg6-count-hook-relocation"></a>

<a id="pkg6-count-hook-relocation--count-hook-placement-correction"></a>

## Count-hook placement correction

| Loader review | What it established |
| --- | --- |
| Yoga Skyline `9959ecb1` | Static placement qualified |

Historical review opened 2026-11-02 (`b97643ab`). Implementation details and outstanding checks refer to the revisions reviewed below.

The previous inline site `155c16c` is unsafe in Skyline's far placement mode.
It is four modulo eight, so the installer overwrites five instructions through
`155c17c`. Native `CBZ x12` at `155c164` targets `155c174`, inside that patch.
A matching instruction guard did not make this placement safe.

The corrected site is `155c188`, eight-byte aligned. Its four displaced words
are `BIC w10,w10,w10,ASR #31`, `CMP w8,w23`,
`STR w10,[x20,#0x1c4]`, and `BEQ 155d9a0`. The nearby null-entry branch remains
intact. A whole-executable aligned direct B/BL/B.cond/CB/TB scan of the owned
game ELF found no incoming edge into this replacement span. This does not
prove absence of computed indirect targets.

- At this point w8 is the requested count, x23 the previous count, w9 the cooldown, and w10 its already-computed decrement.
- The callback reacquires the canonical manager singleton; x9 is no longer a manager pointer.
- For an unfinished new seat, active managed install, failed promotion reservation or unavailable manager, it substitutes w9=6 and w10=5.
- Native then stores its existing five-tick cooldown and the original `CMP w9,#1; BGT` returns before the committed-count store.
- Equal/invalid counts leave the context untouched.
- Normal readiness leaves the original decrement and count reconciliation intact.

Incoming NZCV is dead at the displaced CMP. Its newly produced flags must
survive both conditional-branch relocation and the trampoline tail. The owned
function `155c020..155d9c0` has no x16/x17 operand; return continuations and the
actual Skyline relocation are independent review gates, not assumed by this
source change. No native activation or runtime acceptance is claimed here.

Inputs: owned game ELF SHA256
`0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2`;
retained addressed disassembly in
`artifacts/trainingrooms-nro-archive/pkg6/f9-native-count/pkg6-count-promote-full.txt`;
direct-edge receipt `/home/intpa/smash-training-lobby/tmp-pkg6/count-interior-audit.json`.
The seam retains a full 64-byte guard at the new site, plus the existing input
and retry guards. Original regression checks the actual null-entry branch,
its untouched destination, new alignment and exact displaced words/BEQ target.
The test and native adapter remain uncompiled pending the coherent candidate.

<a id="pkg6-count-hook-relocation--integrated-loader-qualification"></a>

### Integrated loader qualification

| Loader review | What it established |
| --- | --- |
| Yoga Skyline `9959ecb1` | Static placement qualified |
| Compiled stack and live count transitions | Not established by the relocation review |

- Exact Yoga Skyline 9959ecb1 static conditional relocation is qualified in adjacent count-conditional-qualification.json.
- Far BEQ emits condition-preserving BEQ +8, B +20, LDR x17 literal and BR x17 to original155d9a0; false path continues via the qualified tail to155c198.
- Near hook displaces only BIC; original CMP/STR/BEQ remain.
- Both preserve newly produced flags.
- Combined with F6 exact110fb77 source/liveness review, this closes static placement review only.
- Compiled callback stack and live count transitions remain mandatory.

<a id="pkg6-created-lifetime-followup"></a>

<a id="pkg6-created-lifetime-followup--fourth-training-destination-lifetime-follow-up"></a>

## Fourth Training destination: lifetime follow-up

| Evidence | Limit |
| --- | --- |
| Manager teardown | Entry cleanup does not prove retirement of the separate resource owner |

Lead requested a 20-minute follow-up at 2026-11-03 05:18:57 UTC. This report supplements the immutable four-created review; no activation or instrumentation change occurred.

- **The separate resource cleanup order is still unresolved.** The inspected main scene cleanup proves an eight-owner Mii resource-release pass followed by eight-entry manager teardown, but those operations are not the deleting destructor of each context-owned resource object.
- No proof was found that a retained context or resource may legitimately be reused for waiting entry4 under a different scene epoch.
- Neither persistence nor replacement can be assumed from these facts.

The same owned ELF SHA `0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2` was used. Raw bounded excerpts are under `../tmp-pkg6/row4-review/`; `lifetime-manifest.json` SHA `c66a2026a35a6d134fba87e88c69ba0638b1dda55fceb56078a76fd3a57fbabf` binds them. The exact-target direct-branch and context-global reference query is retained as `lifetime-xrefs.json`; candidates were manually checked before interpretation. Nearby-register reuse produces false candidates, so the query is not a dataflow proof. No indirect-call completeness is claimed.

<a id="pkg6-created-lifetime-followup--established-ordering-and-rejected-substitutions"></a>

### Established ordering and rejected substitutions

| Evidence | Limit |
| --- | --- |
| Manager teardown | Entry cleanup does not prove retirement of the separate resource owner |
| Deleting destructor | Must match the resource identity under review |

- `14f139c..13a0` gates the manager cleanup on the current manager singleton.
  - `14f13a4..13ac` reads context `*(5324680)` and owner alias0.
  - The subsequent unrolled pass visits aliases through7 (`14f1dbc` reads alias7 at context+0x120).
  - For each existing owner it reads the current Mii record index and retires the selected G record's shared resource and face handles.
  - The final owner7 path clears G+record+0x58/+0x60 at `14f1e90` and owner face words at `14f1f24`.
- Only then does `14f1f38` invoke `66e850`, whose row4 destructor/free/count-decrement path was proved in the first report. Clearing a G record or face handle is **not** clearing `context.owning[4]` or invoking its native deleting destructor. These cannot substitute for the required resource-owner retirement marker.
- Actual owner deleting destructor `17e44b0` saves the owner address, calls body `17e4350` at `17e44c0`, reaches existing hook `17e44c4`, and tail-frees at `17e44d4`. Body `17e4350` cleans face handles, owner task/shared state and the key tree. The matching resource identity remains necessary; an arbitrary occurrence of this function is not our owner.
- Direct references to `17e4350` also occur at `66e27c` and `66e2ac`, on a wrapper's `this+0x18` embedded owner. They use a different wrapper vtable and cannot be relabelled as retirement of the separately allocated `17eb000` resource. No direct call to `17e44b0` was found by the exact-target query; its known vtable dispatch remains the material indirect edge.
- `14f381c ->1756470` was checked as the existing service-reset call later in the scene function. Its body clears service queues/flags under their locks; it contains no established per-owner deleting-destructor edge. It is not a replacement lifetime proof.
- The historical `resource-context-cleanup.asm` stays rejected. Adjacent `17eb140` only queries owners through virtual+0x30; it does not delete them. Context-global address candidates at `18768dc` were register-reuse false positives and supply no evidence.

The exact missing link is a qualified caller of the context-owned resource's virtual deleting destructor, ordered after the old created lifetime and before native construction/use of waiting4. The current evidence does not identify that caller or show all requested transitions reach it. Further unrelated native scanning would exceed this bounded review's purpose.

<a id="pkg6-created-lifetime-followup--proposed-bounded-observation-using-existing-diagnostics"></a>

### Proposed bounded observation using existing diagnostics

| Proposed observation | Status at review |
| --- | --- |
| Training exit to waiting entry 4 | Not run |
| Baseline with only entry 7 created | Cannot test retirement of a created entry 4 |

This is a proposal for lead review, not an executed run or activation authorization. A row4 candidate must first receive explicit review/permission to activate its strict native creation path; current code still cannot create4. A baseline that creates only7 cannot establish retirement of a nonexistent created4.

Use one fresh, coherently gated diagnostic process and the normal Training exit to menu followed by the existing waiting-room route. Batch this with the required four-created HUD/F9 cases, within the existing 45-minute runtime box. Keep the full ordered diagnostic/console output and controller timestamps; no new hook or per-frame probe is proposed.

1. Require existing `[CharacterSources] capacity creation` records for slots7/6/5/4, `backend=Training`, `qualified=true`, one scene epoch and exact nonzero entry/resource identities. Require the existing strict pre/postflight contract and full native-ready/Mii receipts; no quantity-based synthetic receipt.
2. For each creation retain `[ManagedSourceCleanup] entry destruction`, the matching `[ManagedSourceCleanup] listener ... removed=true`, and `[ManagedSourceCleanup] retirement ... entry_destroyed=true reservation_cleared=true listener_removed=Some(true)` with the exact old entry/owner/epoch. Also retain `[ManagedSourceCleanup] extended claim slot=4 owner=... released=true` plus the matching `[ManagedSourceCleanup] claim owner=... released=true`. Actor cleanup must match the loaded source, not another generation.
3. Require retirement of old slot4 **before** the existing `[TrioBoundary] allocation owner=4 ... mode=...` for native waiting construction.
   - This allocation diagnostic is emitted at `678174` before publication, so it is a stronger boundary than the first waiting service.
   - Preserve raw sequence on the producing thread; do not infer order by merging separately buffered streams.
   - If ordering cannot be established, return inconclusive.
4. Require fresh waiting mode/epoch and native visible4 identity, no Training-created registration interpreted as waiting, normal waiting helper/input/owner checks, and no old source receipt accepted. Existing strict snapshot reads authorize live identities; saved old pointers do not authorize dereferences.

- Falsifiers: any missing/mismatched retirement, next waiting4 allocation before old resource cleanup, old registration still present, stale epoch accepted, failed listener/actor cleanup, changed original roots, any Unmapped or nonzero hook-allocation delta.
- If the bounded `[TrioBoundary]` log budget384 is exhausted, absence of the allocation marker is not success.
- If the actual configured feature set does not emit the needed marker, report that exact observation gap before adding instrumentation.
- The existing diagnostics do not directly prove whether the context address itself persists; do not make that additional claim without a current-root receipt.

No product/native policy was changed for this follow-up. The pure ownership WIP received only test-fixture corrections for four owners and explicit cross-owner row rejection; it remains uncompiled. Four-row implementation and all runtime acceptance remain conditional on the lead's disposition.

<a id="pkg6-created-lifetime-followup--accepted-observation-refinement-lead-2026-11-03-05-45z"></a>

### Accepted observation refinement (lead, 2026-11-03 05:45Z)

| Observation at the menu | Decision before entering waiting |
| --- | --- |
| All four retirements matched | Proceed and bind the fresh waiting identity |
| Missing or ambiguous marker | Stop and restore |

- Existing diagnostics contain a global sequence but no producer-thread ID.
- Supersede the producing-thread criterion above with an interactive causal barrier: observe every exact entry/listener/owner/claim retirement for all four created rows while still at the menu, BEFORE sending any waiting-entry command.
- Missing or ambiguous markers mean stop and restore without entering waiting.
- Then bind fresh waiting4 construction and epoch.
- Do not infer producer thread from the log drainer.

<a id="pkg6-f9-promote-contract"></a>

<a id="pkg6-f9-promote-contract--f9-promotion-proposal-2026-11-02"></a>

## F9 promotion proposal (2026-11-02)

| Proposed count transition | Body ownership |
| --- | --- |
| Increase | Retain the helper body; transfer its receipt to the visible CPU |

This records the proposal and gaps before implementation. The later [hat/item cutover decision](DECISIONS.md#f9-live-hat-item-source-immediate-cutover-2026-11-03) superseded lossless transfer for those two cases; the [roadmap](ROADMAP.md#deferred-operator-decision-2026-11-03) records what shipped.

Operator/lead direction in TRAININGROOMS-LEAD-INBOX.md, 2026-11-02 20:5x/21:0xZ,
supersedes delayed-count/setter replay. Preserve the requested count and retain
the helper body on newly active entry 2 or 3. Entry 2 holds CPU rotation's next
member, entry 3 the following member. Exact saved Mii identity is part of the
member, not merely its class kind. Defaults/capacity changes are separate work.

<a id="pkg6-f9-promote-contract--ownership-transition"></a>

### Ownership transition

| Proposed count transition | Body ownership |
| --- | --- |
| Increase | Retain the helper body; transfer its receipt to the visible CPU |
| Decrease | Wait for native deactivation before re-adopting the parked entry |

1. Before native activation, capture the exact scene/source generation, entry
   address, body IDs, selected member, costume and saved-Mii configuration.
   Exclude newly active entries from speculative and demand-source replacement.
2. Retain the existing native body, descriptor and resources. Do not call the
   old return-to-original installer, fake a destructor, or replay the count setter.
3. Transfer the helper receipt to a visible CPU receipt. Existing hat/item/portrait
   consumers must either retain a valid resource lifetime through this transfer
   or complete their real release before it; invalidating tokens alone is not
   evidence that native consumers stopped using the resource.
4. Publish identity, native selection and presentation through the existing
   field/selector/HUD paths, extended explicitly to these owners. Record the
   promoted value for Training reset and save/load; it must not alias CPU1's
   mutable rotation state. Scene/entry destruction retires only the exact receipt.
5. On decreasing count, wait for native deactivation to finish before re-adopting
   the exact parked entry; create fresh helper authority without destroying or
   reloading a retained body solely to change its ownership label. In slow mode,
   no helper occupies an active CPU entry while count exceeds one.

<a id="pkg6-f9-promote-contract--source-findings-and-implementation-gaps"></a>

### Source findings and implementation gaps

| November 2 review | Gap at that revision |
| --- | --- |
| Source selection | Demand-first selection did not guarantee fixed seat order |
| Presentation | Two visible-role stores did not represent owners 2 and 3 independently |

- `character_sources::service` calls `service_owned` before admission gates.
  Loading source authority/INSTALL is retained until actual native completion;
  it cannot be dropped when the menu count changes.
- `SourceBook` supports Ready, Loading, Failed and Departed, but no promotion.
  `native_destroyed` invalidates leases and means actual destruction; it must
  not stand in for promotion. `returned_native` describes the superseded
  original-restoration load and also cannot represent retained-body transfer.
- `selected_request` currently picks demand first and then a missing speculative
  target. It does not guarantee CPU rotation's fixed next/next-after seat order.
  Scheduling needs an explicit seat contract, including exact-Mii values.
- `publish_mii_presentation`, `allow_portrait`, and `republish_after_rebuild`
  currently use the two visible-role state stores and owner0/1 guards. Simply
  calling them with owner2/3 either refuses or aliases CPU1. Promotion therefore
  needs owner-indexed publication receipts and reset handling, not a guard bypass.
- The existing native count reconciler's readiness scan covers the previous
  count (owned-ELF fact recorded earlier). A still-loading newly active helper is
  outside that scan. Retaining its body avoids a new reload but does not prove
  activation safe during its already-running load. Need a proven native
  activation/readiness boundary that preserves count and callback progress.
  No delayed setter or silent loading-helper restriction is authorized here.

<a id="pkg6-f9-promote-contract--evidence-required"></a>

### Evidence required

| Evidence requested | Coverage |
| --- | --- |
| Host regressions | Seat order, receipt lifetime and exact Mii identity |
| Eden sequence | Count changes, reset and save/load with multiple CPUs |

- Original regressions: fixed seat selection; transfer with live leases; loading completion after count increase; stale scene/entry callbacks; reverse adoption; exact-Mii identity and independent CPU1/2/3 reset/save-load receipts.
- Then compiled native stack/guard gates and batched Eden count 1→2→3→2→1, loading helper, exact Mii, L+R+A and save/load at 2–3 CPUs, with full console zero Unmapped and all hook allocation deltas zero.
- This document is a proposed source contract, not completion evidence.

<a id="pkg6-four-created-hud-review"></a>

<a id="pkg6-four-created-hud-review--four-training-portrait-destinations-bounded-proof-review"></a>

## Four Training portrait destinations: bounded proof review

| Static review | Result |
| --- | --- |
| Entry 4 | Creation and entry destruction supported by static evidence |

Review requested by lead at 2026-11-03 05:05:39 UTC; returned within the 25-minute box. Source baseline `110fb77` plus explicitly uncompiled HUD WIP. This is a design review, not permission to create entry 4 or runtime acceptance. Waiting entry 4 remains its native visible CPU.

The owned 13.0.5 ELF is `/home/intpa/trainingrooms-build-pkg6-host/70d4927/research/game-13.0.5/main-13.0.5.elf`, SHA256 `0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2`. Existing authorized LLVM fallback was used; no new hook or tool system. Fresh bounded excerpts and their hashes are in `../tmp-pkg6/row4-review/manifest.json` relative to this worktree. The earlier `three-source-capacity` native reports supply the existing lifecycle contract; stage/fork implementation sources were not read.

<a id="pkg6-four-created-hud-review--result"></a>

### Result

| Static review | Result |
| --- | --- |
| Entry 4 | Creation and entry destruction supported by static evidence |
| Resource-owner lifetime | Not proved across Training exit and waiting-room entry |
| Runtime result | No row-4 run in this review |

- There is positive static evidence for index 4 creation, count 8, linked-list service, eight-owner Mii protection, a physical request row 4, and entry-4 destruction in normal manager teardown.
- No native three-extra ceiling was found in these paths.
- **The complete Training-to-waiting lifetime requirement is not yet proved**: the addressed manager teardown destroys the FighterEntry, while the separately context-owned resource must also retire, and the specific transition must reach both before native waiting entry 4 construction.
- Existing claim retention is necessary but cannot serialize native waiting construction.
- No row-4 runtime exists in this review.

<a id="pkg6-four-created-hud-review--addressed-evidence"></a>

### Addressed evidence

| Boundary | Evidence and limit |
| --- | --- |
| Empty creation | `4eb6b0` rejects indices >=8. `4eb6c4..c8` reads primary[index]; occupied entries skip creation. `4eb74c` calls `678150` with native empty descriptor; `4eb76c` ensures resource owner via `17eb000`. No index-4 exception. Admission must still prove vacancy; the helper alone does not prove scene authority. |
| Native count 8 | `678a70` checks index <8; `678a84` publishes that primary; `678aac..ab4` increments native count exactly once. Fourth creation after count7 therefore produces8 without plugin count writes. Allocation failure and scene roots remain runtime predicates, not implied success. |
| Resource index | `17eb01c..20` rejects >7; `17eb024` reads owning[index]. Absent owner gets native 0x4c8 allocation/constructor, publication at `17eb0a4`, native init at `17eb0c8`. `17eb0fc..114` reads owning[index] under context mutex and publishes alias `context+0xe8+index*8`, with a second <8 check. This is the same contract as 5..7. |
| Dispatcher | `1755e58..88` walks the actual listener list, compares its sentinel, calls virtual+0x18 and only then reads next node. It has no count7 or index4 cutoff. Existing native `651640` functor contract invokes `650010` for the registered entry. Existing creation postflight must still witness listener count+1. |
| Portrait batch | `3302e48` starts pointer array at batch+0xd8; `3302ec0` ends at +0x358; `33030b4..bc` advances by8 until that end: exactly80 requests. Row4 subs7/8/9 are indices47/48/49, within this actual allocation. Table index is 0x12+4=0x16 within the existing 32-slot table. Original request/holder/native registration/table-lock and batch identity checks remain necessary. |
| Entry teardown | Main scene cleanup calls `66e850` at `14f1f38`. Within it, row4 is explicitly handled at `66eaa0..66eb10`: backup+0x80, primary+0x40, native deactivate `66eadc`, body cleanup `66eae4`, clear primary `66eaec`, entry destructor `66eaf8`, native free `66eb00`, count decrement `66eb04..0c`. It is not skipped at count8. |
| Separate resource retirement | Existing hook `17e44c4` observes the actual owner's deleting destructor after `17e4350`, before free tail. `native_source_capacity::owner_cleanup_complete` unregisters the exact matching owner before releasing its claim. This is separate from the entry destructor and does not prove when the requested scene transition invokes it. |

The historical `three-source-capacity/resource-context-cleanup.asm` is explicitly rejected evidence in its REPORT: it is an item-resource registration tail, not context cleanup. It was inspected but contributes **no** lifetime proof here.

<a id="pkg6-four-created-hud-review--existing-source-proof-obligations-that-must-stay-strict"></a>

### Existing source proof obligations that must stay strict

| Guard under review | What must remain true |
| --- | --- |
| Preflight | Fresh manager, context and service; exact prior roots |
| Admission | No new row inferred from count alone |

- `native_source_capacity::preflight_checked` already checks current manager/internal/context/service, creation-order prefix, exact prior registered roots, native count `4+earlier`, all backups zero, target primary/resource/alias vacant, target selection type2, target input routes `slot` and `slot+8` unused, and all original body/group/visible/resource identities.
- For fourth Training creation these checks mean count7, target4, routes4/12 and the three already registered creations unchanged.
- Neither requested CPU quantity nor a null primary alone is admission.
- Postflight checks the exact single-slot delta, native count+1, listener+1, empty entry identity and preserved original actors/input/rules.

- `mii_selector_native::snapshot` walks all8 FE/E/D/O roots under G; every non-layout owner must be a registered creation.
- `CreatedDestination::capture/protected_now` require Training, full exact registration, unique row/object/selector claims and record bounds.
- Training claims constrain each current index to0 or its own selector, with exact Miis requiring their selector.
- Thus for destination4 the other seven owners' current/selector union fits the existing8-element protected array (0..3,5..7); no protection truncation or allocation is needed.
- `record_count >4` alone is insufficient: every protected record must also be in bounds, so a full eight-owner layout requires records through7.
- The adapter compares full protected record bytes and mirrors after publication.
- This is a conditional proof, not a claim that every live G has enough records.

`character_sources::on_destroy` and `capacity_owner_retired` preserve the numeric reservation across scene departure until both entry and owner retirement. `registered_prefix` rejects mixed layout registrations; a Training registration cannot become waiting authority. These prevent plugin misuse, but cannot replace the missing native transition-order proof.

<a id="pkg6-four-created-hud-review--minimal-coherent-source-change-if-approved-after-the-lifetime-gate"></a>

### Minimal coherent source change, if approved after the lifetime gate

| Proposed layout | Created-row plan |
| --- | --- |
| Training | `[7,6,5,4]` after lifetime approval |
| Waiting | `[7,3,2]` unchanged |

1. Separate Training plan `[7,6,5,4]` from unchanged waiting `[7,3,2]`. Current APIs couple both to `[usize;3]`; use an explicit per-layout plan length and a bounded four-record storage whose unused waiting tail must be absent. Add4 to the physical retirement union only. Preserve serial prefix, claims and all fresh native pre/postflight predicates. Do not widen waiting or generic `Layout::helper`.
2. Give each of the four visible Training owners a stable distinct **renderer row** from that plan, for example `[7,6,5,4]`. Keep BOUND/PREPARED/in-flight exclusions and same-owner three-request rotation. No cross-owner table aliases, post-bind releases, whole-batch reset or active-fighter render. Exact source readiness still needs the existing CreatedDestination/G/resource proof.
3. Make portrait demand explicit in creation/admission.
   - Current non-speculative `creation_intent` picks only7 unless fixed CPU intent exists, and `helpers_unavailable` reasons about generic consumers rather than an exclusive render row.
   - Merely widening arrays would leave legitimate four-owner portrait jobs unschedulable.
   - The desired row must be part of the checked purpose, with serial creation of prerequisites; no quantity-only or preload-setting bypass of native capacity checks.
4. Reuse the pure four-owner/request lifetime WIP.
   - Supersede the incomplete own-active-row publication route with created-row rendering and normal visible publication for owners0..3, matching active promotion generation for2/3.
   - Retain normal SourceBook leases; a renderer reservation is not permission to revoke an unrelated hat/item lease.
   - If all suitable source capacity is pinned, report the real wait instead of reusing another owner's request or declaring a texture ready.
   - This contention must be exercised, not hidden by a timeout.
5. Add original policy tests for four-prefix Training, unchanged waiting row4 rejection, wrong-layout registrations, all-eight protected union/bounds/collisions, exclusive four-row ownership and both-order retirement. Keep current test evidence separate from these new, unexecuted cases.

<a id="pkg6-four-created-hud-review--remaining-acceptance-boundary"></a>

### Remaining acceptance boundary

| Acceptance boundary | Evidence still required at review |
| --- | --- |
| Separate resource owner | Ordered cleanup before the next waiting entry 4 |
| Four native sources | Measured memory headroom and runtime checks |

- Before activation, resolve the native resource-owner cleanup ordering for normal Training exit and subsequent waiting entry4 construction, using the existing owned lifetime evidence or a specifically authorized bounded observation.
- Existing `66e850` proof establishes entry cleanup only.
- Actual validation must then correlate all four exact entry/listener/owner retirements and old scene epoch with fresh waiting4 identity; absent cleanup is failure.
- Four native sources also require measured native memory/headroom, coherent stack/host/native gates, exact-Mii rendering and A/B/A/reset/reverse-count runtime checks.
- No compile, Eden launch, allowlist edit or native call occurred in this review.

<a id="pkg6-owner-locks"></a>

<a id="pkg6-owner-locks--cold-owner-locks"></a>

## Cold owner locks

| State | Entry paths and protected contract |
| --- | --- |
| roster STATES[2], WAITING_MEMORY | scene/input/service/destruction and settled roster queries; role state and remembered value remain under the same guards |

Historical review opened 2026-11-02 (`047c6f48`). Implementation details and outstanding checks refer to the revisions reviewed below.

The counted first-lock path is distinct from OS TLS. UI's retained `a05601b`
Switch ELF (`3d6251837c8ba6b67a0d6a6313cd893a29fbe8b216348660670ae150e60c5f31`)
shows ordinary-player State lock `0x5fa98`, null OnceBox branch `0x5fabc` to
`0x1dd294`, allocation counter at `0x1dd2a4`, then `calloc(40,1)` at `0x1dd2b0`.
This evidence was supplied by UI in the lead inbox at 21:46Z; it is not an
independent runtime attribution of every `note` allocation.

- The replacement uses the existing `hos_lock::Mutex`: constant AtomicU32, acquire/release exclusion, non-reentrant guards, and Horizon address-arbiter wait/wake SVCs.
- It does not require game-thread-only ownership.
- No guard extent, lock order, native call, readiness condition, or value-publication operation changes.
- The release profile aborts on panic, so post-unwind poisoning is not a reachable native continuation; previous poison-recovery wrappers in RHandle and CopyPkg become plain guards.
- `try_lock` still refuses contention immediately.

<a id="pkg6-owner-locks--audited-scene-chain-and-ownership"></a>

### Audited scene chain and ownership

| State | Entry paths and protected contract |
| --- | --- |
| roster STATES[2], WAITING_MEMORY | scene/input/service/destruction and settled roster queries; role state and remembered value remain under the same guards |
| ordinary player/cpu STATE | scene/service/destruction, catalog publication and settled configuration; exact descriptor/epoch/transaction checks remain inside guard |
| saved ALPHA, auto-init STATE | scene cancels/retire; native menu handoff and catalog completion; auto-init drops coordinator before entering ordinary owners |
| body BANKS[2], BANK_STORAGE[2] | scene/service/destruction plus boot reservation; retain all bank ownership and native call boundaries |
| resource LEDGERS[2], copy HELD[2] | scene reset/forget and bank service; release recipes still leave guards before native calls |
| source STATE, MEMORY_REFUSED | scene/source service/lease clients; source STATE -> helper STATE -> registry order retained |
| helper STATE, ordinary-native COMMITTED | source/native transaction and read-only provenance; registry performs no native calls, existing G -> transaction -> registry order retained |
| HUD STATE | scene/native service/render adapter; existing nonblocking callback reads stay nonblocking |
| item PENDING | scene cancel and native item service; same lease/cancellation boundary |
| hat STATES[2], SHORTCUT, OPERATOR_SHORTCUT | scene/input/service/save restoration; scene drops each role guard before optional reporting |
| practice OWNED | scene/frame/destruction and save/record acquisition; entry checks precede acquire, guard released before clear_session_state |
| practice-stage STATE | UI context/request/status, native stage update, scene departure; nonblocking refusal and epoch invalidation retained |
| training-stage ROTATION | sequence initialization, entry/reentry, resource callback and direct-stage completion/departure; menu snapshot precedes rotation guard, stage STATE -> ROTATION order retained |

`input_delay::note -> lobby::update -> training::lobby::update` detects a mode
transition and invokes `character_roster::scene_changed` and
`practice::scene_departed`. These locks are also used by their owners' regular
callbacks; preserving exclusion avoids assuming those callbacks cannot overlap.

- The composed `4f9878d` diag ELF proves another cold scene path: `practice_stage_request::depart` at `0xa2018` calls its standard-mutex `try_lock` at `0x5c3f8`; the null branch at `0x5c408` reaches OnceBox initialization at `0x1f7e1c`.
- Nonblocking acquisition did not avoid lazy allocation.
- Neither stage owner's init acquires its lock.
- Stage-request retirement can enter ROTATION via `direct_uncertain`; the native sequence hook also acquires ROTATION on first use.
- Both now use the same constant primitive, with unchanged guard boundaries and callback ordering.

- Parent `character_roster` retains its standard Mutex import for unmodified probe and inherited owners.
- Explicit OwnerMutex types select only roster/waiting states.
- Five probe readers need only the return-guard API adjustment when they read those states; their separate probe locks are unchanged.
- Native catalog broker is already initialized in boot and is outside this cold scene slice.
- Editor/keyboard and named UI storage are not globally rewritten.

<a id="pkg6-owner-locks--required-evidence"></a>

### Required evidence

| Environment | What the tests can establish |
| --- | --- |
| Host | Exclusion, waiter handoff and allocation checks on the host path |
| Switch | Needs exact compiled paths, stack gates and first-use counts |

- The cold/fresh-host-thread test exercises the actual public primitive's first lock, held-lock refusal, settlement and repeated reads with zero allocations.
- Existing lock tests cover mutual exclusion and sleeping-waiter handoff.
- Host futex waiting differs from Horizon; these tests do not prove native no-heap.
- Exact composed Switch call-path inspection, stack gates and runtime first-use counts remain required.
- No warm-up, counter waiver, stack increase, or whole-R5 acceptance follows from this source change.


<a id="pkg6-owner-locks--preload-bookkeeping-continuation"></a>

### Preload bookkeeping continuation

| Preload state | Lock boundary |
| --- | --- |
| Grace | No source, roster or native lock held inside GRACE |
| Requests | Release before source metrics |
| Condition | Release before metrics |

- `character_sources::scene_changed` invokes `cycle_preload::scene_end`, which touches GRACE, CONDITION, REQUESTS and STATUS even before preload first use.
- Settings changes and visible-player planning use those same value owners.
- GRACE never takes a source/roster/native lock; final source admission reads its atomic generation/deadline snapshot.
- REQUESTS releases its guard before finishing metrics against sources.
- CONDITION is released before metrics.
- STATUS snapshots desired values before source readiness, and retains its existing notification-lock nesting only in show_status.
- No reverse ownership or reentrant assumption is introduced by the primitive replacement.

- Exact `80ee800` diag shows cold OnceBox `0x1f82dc` from GraceCoordinator lock `0x5c690`, Status lock `0x5c6c8` and Settings lock `0x5bf14`.
- close_requests `0xbde90` calls the OperationStamp-array standard mutex at `0x5be34`; reset_status `0xbda44` and reset_admission_grace `0xbe110` reach the corresponding cold locks.
- Complete disassembly is archived as cycle-*.txt with that ELF.
- This source slice changes only those four lock primitives/guard APIs; no metrics, admission, notice or load-order change.

- The direct compiled note call graph additionally reaches preload_metrics SESSION through close_requests -> OperationStamp::finish, lock `0x5c5e4`.
- Its only owners are finish and snapshot: sample first, absorb/snapshot under the guard, drop before diagnostic formatting/heap-profile output.
- This value-only session uses the same constant primitive.
- Native panic=abort excludes the old poisoned fallback; no successful record or disabled-measurements behavior changes.
- The remaining catalog-broker lock is already initialized by its boot owner, as recorded above.
- Direct-call reachability does not prove indirect paths absent.


<a id="pkg6-owner-locks--managed-creation-registry"></a>

### Managed creation registry

| Managed creation receipts | Review finding |
| --- | --- |
| Protected value | Fixed numeric receipt table |
| Inside its guard | Slot copies or updates; no native call or nested owner lock |

- native_source_capacity::managed CREATED is a fixed numeric receipt table.
- Only created/register/unregister acquire it; they copy or update table slots, without native calls or nested owner locks.
- init sets ENABLED after checking the load contract and does not initialize the old mutex.
- Exact80ee lock5be6c is reachable from eight native callback roots (all-hook-standard-locks.json).
- The constant primitive preserves exclusion and index/lifetime decisions; release panic=abort excludes the old poisoned empty-table continuation.

<a id="replay-editor"></a>

<a id="replay-editor--replay-editor-what-came-from-the-fork-2026-11-24"></a>

## Replay editor: what came from the fork (2026-11-24)

| Fork commit | Fork file | Added lines | Here | Verbatim |
| --- | --- | --- | --- | --- |
| `eb4eec2` (accelerating scrub) | `training_mod_tui/src/clip_panel.rs` | +16-124, +288, +477-537 | `src/model/macro_ui/clip_panel.rs` | 84 of 111 |

- The crop and preview editor of install 4 (`feat/replay-editor`, base `b5177cdb`) copies no fork code.
- The fork-own hunks it uses were already in this repository through the package-4 carry of 2026-11-02 (`tools/carry_pkg4.py`, stage m5q, ledger `pkg4-carry-ledger.md`).
- This table lists them by the fork commit that added them.
- Fork worktree: `UltimateTrainingModpack-lobby-parity`; line ranges are the added lines at that commit.
- "Verbatim" counts the added lines of more than 14 characters found unchanged in this repository at `b5177cdb`.

| Fork commit | Fork file | Added lines | Here | Verbatim |
| --- | --- | --- | --- | --- |
| `eb4eec2` (accelerating scrub) | `training_mod_tui/src/clip_panel.rs` | +16-124, +288, +477-537 | `src/model/macro_ui/clip_panel.rs` | 84 of 111 |
| `eb4eec2` | `src/replay_capture.rs` | +15, +361-364, +386-395, +399-407, +419-420, +423-427 | `src/runtime/macros/replay_capture.rs` | 18 of 19 |
| `eb4eec2` | `src/training/ui/clip_editor.rs` | +12, +474-484, +486-496, +498-499 | not carried | 3 of 20 |
| `15ada37` (snap to move starts) | `training_mod_macros/src/move_start.rs` | +1-256 | `src/model/macros/move_start.rs` | 145 of 185 |
| `15ada37` | `src/replay_move_start.rs` | +1-45 | `src/runtime/macros/replay_move_start.rs` | 40 of 41 |
| `15ada37` | `training_mod_macros/src/replay.rs` | +3, +65, +67-85, +89, +92-93 | `src/model/macros/replay.rs` | 18 of 19 |
| `15ada37` | `training_mod_tui/src/clip_panel.rs` | 28 hunks from +2 to +704 | `src/model/macro_ui/clip_panel.rs` | 67 of 79 |
| `15ada37` | `training_mod_macros/tests/move_crop_continuation.rs` | +1-63 | the carried unit test of `clip_panel.rs` | 2 of 52 |
| `6f250bb` (preview without a manual save) | `src/training/save_states.rs` | 17 hunks from +2 to +1177 | not carried | 1 of 88 |
| `6f250bb` | `src/replay_preview.rs` | +31-32, +56, +59, +155, +165, +188, +208, +211-214 | not carried | 0 of 11 |
| `6f250bb` | `training_mod_tui/src/clip_panel.rs` | +119 | `src/model/macro_ui/clip_panel.rs` | 1 of 1 |

- **`6f250bb`:** its preview is not here. Previews play through Recording's player and keep the pair in a game-side
  checkpoint (fix F7, `6bc005f6`), written in this repository.
- **The fork's `src/training/ui/clip_editor.rs`** (the fork's `4c0d785` and `6a77411`) draws through the Training
  Modpack's menu panes and was never carried. The strip over the game, the timeline and the play target are written
  here from the fork's documents and clips (`docs/named-macros.md`, `docs/named-macros/images/`).
- **Upstream Training Modpack** `7c522c0` (#587) and `1c00d2a` (#607) are not used.
- **New in this branch, written here:** `src/model/macro_ui/clip_marks.rs` (move marks, the watch, snapping), the strip
  geometry, the play-target swap and the `actions` file key. The edits to carried files are recorded in the package-4
  manifest.

Change a carried file by adding manifest edits: `python3 -I tools/carry_edits.py <dest> <base> --reason <reason>`
writes them from the file as it was and the file as wanted. Then run `carry_pkg4.py carry`, `ledger` and `verify`.

The counts come from a scan of `git show --unified=0 <commit> -- '*.rs'` against `src/**/*.rs`; the scan and its output
are kept with the lane's artifacts (`artifacts/phase23/replay-editor/prov.py`, `provenance-scan.txt`).

<a id="tech-timing-native"></a>

<a id="tech-timing-native--native-tech-timing-13-0-5"></a>

## Native tech timing (13.0.5)

| 13.0.5 reader | Address |
| --- | --- |
| Fighter ControlModule address point | main+0x4f85de0 |

The timing reader uses the Fighter ControlModule address point `main+0x4f85de0`, installed by the constructor at `0x6b8530`. The base class address point rejects fighter objects. The constructor stores `this+0x20` at `this+0x110`. Exact vtable byte offsets `0x178/0x220/0x228/0x230/0x250` resolve to stick, trigger word, current count, previous count and native trigger check respectively. All call targets retain exact 64-byte guards.

- The update at `0x3f9140` produces the native positive-edge word.
- Its loop at `0x3f9530` copies the old current count into previous and resets current to zero on that edge; otherwise current increments and saturates at 254.
- Therefore a press at contact has age zero.
- Previous is the pre-update age, one less than the elapsed gap between presses; comparisons use the unchanged native value.
- No physical shoulder sample or counter decrease establishes a press.

The common check at `0xed460` accepts `count < base * multiplier` and `lockout <= previous`. Multiplication and mixed comparison use f32, without truncating the window. Window, lockout and reaction are read live. DamageFall and held-input paths remain distinct. Runtime ordering and availability require exact-build trace validation; static counter origin alone is not a host-input timing measurement.

- Actual entered native contact statuses are independently proven through the common constant table and status requests: floor DOWN 80 / bounce 78; wall STOP_WALL 108 / bounce 76; ceiling STOP_CEIL 109 / bounce 77.
- STOP statuses also occur outside tech attempts: grading still requires the owned attempt, valid first contact and actual entered status.
- Unknown status or unreadable contact remains unavailable.
- A proven native miss remains counted even if later input coverage is unreadable; the neutral “Missed” label makes no claim that no button was pressed.

Evidence: main ELF SHA256 `0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2`; source NSO `8b3dc2eb011f26dda1f87947e4f62665e8d8fe3d84397e85ecf9b3cd9d0137ce`. The coordinator artifact `artifacts/rc15-tech/native-timing/re/NATIVE-CONTRACT.md` binds original bytes, relocations, six main and eight common REA records, native request sites and retained runtime statuses. REA full-main analysis timed out; bounded carriers preserve original VMAs and copied bytes, with omitted callees explicitly outside coverage. They are analysis containers, never guest builds.

Host regressions cover the constructor address point (old guard RED), native counter instructions, exact native guards, three counted/streak-breaking no-tech results (old policy RED), labels, timing averages and ten missed attempts reaching the summary. Missing native observations are tested separately from readable false values.

<a id="vault-recordings"></a>

<a id="vault-recordings--vault-and-recordings-provenance"></a>

## Vault and Recordings provenance

| Feature | Origin |
| --- | --- |
| Vault and Recordings, 2026-11-24 | Written in TrainingRooms for the accepted design. |

2026-11-24. Written in TrainingRooms for the accepted Vault and Recordings design. No line was copied from the fork
or upstream Training Modpack. Decision provenance: inbox 2026-11-24 22:45:56Z, design accepted 23:02:29Z.
