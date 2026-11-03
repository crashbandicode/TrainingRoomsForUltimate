# lr-04 shared-tool evidence — 2026-11-03

**Stopped, restored, separately verified and released. RC NOT READY.**

Yoga loaded at 19:40:27.794907Z; owned release was 20:06:55.147785Z.
Product remains comments-only RC8 `cf4bf44c104ecc96a78d382b4c0276bf33838f18`,
using the retained diagnostic NRO `3ca7f0c5c9e5552be4725050949b1ba96266cf4f5875bef660f94ba9b323e9e4`.

## Results

`receipts/` contains all 23 original receipts: **16 PASS, 7 FAILED**.
`16-return-lucas-replay` was unrun. The final manifest has 21 active rows;
three earlier failed bootstrap rows remain in its history. Four PASS rows
(`01d`, `10b`, `11b`, `13b`) only observe/requalify state.

- **Delay2 positive:** rows08/09, reviewed native pause/resumed images,
  `queued_at_begin=2 owned_polls=121 owned_non_neutral=0 owners=0x00` and
  `resume_checked delay=2 leaked=0`.
- **Failed:** 01/01b/01c intro/title/main predicates; 10 warm `GroupSwap`
  finalize predicate; 11 expected CPU39→40; 13 fresh original-entry predicate;
  15 expected return Roy26→27. These receipts remain failures.
- Row11 actually acquired CPU at Roy27; row12's Roy27→Lucas39 correction
  passed. Row15 replay instead acquired Sonic40 (fresh39→40). Exact return
  replay remains unqualified and dependent16 was not attempted.
- Row13 diagnostic rotation/truncation invalidated the stored byte offset.
  Separately retained sequences1600/1601 and1607 show original Steve/Lucas,
  catalog generations[3,3]/opening4; read-only13b correlates this observation.
  Row14 proves native-original retirement, not warm/interrupted teardown.
- `game.stop` terminal deadline failed. Forced cleanup succeeded. The unit
  reports `Result=timeout`, `ExecMainCode=2`, `ExecMainStatus=9`, not an OOM
  result. Peak8589938688B exceeded configured8589934592B by4096B; cause is
  not established and resource acceptance remains open.

Full profile restore and independent verification cover21410 entries.
Source equals its snapshot; topology audit found252 immutable linked files,
20436 private mutable files and no violations. Owner-inclusive ledger and
unit checks found no owned open claim or PID. Failed initial Surface journal
`/home/intpa/tr-lr04-state` is preserved; neither stopped run may resume.

Full console:18086063 lines/3470923181B, zero Unmapped, one review hit:
`Cpu.cpuopt_ignore_memory_aborts: true` in the initial settings dump.
SHA256 `1663f84790ff30ee53261f0c4c689cd04f554df7f53a22d228094b20a25a2f18`.
The original remains on Yoga; a locally verified lossless gzip is retained.

## Reuse and provenance

Only row data, recordings, images and evidence are stored here. The shared
runner/lifecycle are referenced, not copied. `provenance.json` records exact
Up/Down/runner identities; tools changed concurrently, so no single immutable
lifecycle version is asserted. Up used playbook40fbe1d plus reviewed pending
changes; archival read follows923db2084977fa900b5fd91fcdfdaaed8ff55316.

`rows.json` preserves the final plan with portable path rewrites. Library
references are frozen as `archived_library` annotations so a future library
change cannot silently change these bytes. Original receipt manifest hashes
refer to executed versions, not this portable serialization. Exact final
manifest, retained intermediate versions and full outputs are external.
Four pre-record manifest hashes (05a/05b/05c/12) have no byte-identical
retained manifest: shared recording rewrote the file in place. Receipts,
recording hashes and the resulting manifests remain; this provenance gap is
explicit in `manifest-receipt-bindings.json` and is not repaired by inference.

Structural check (already passed:21rows/47gates/20recordings/0errors):

```text
python3 -B -I ~/ssbu-mod-playbook/tools/eden-runner/runner.py dry-run eden/lr04-shared/rows.json --out /tmp/opencode/lr04-dry-run
```

This does not prove a fresh-boot complete route. `harvest-selection.json`
selects12 actual non-observation PASS rows and is intentionally discontiguous;
it is harvest input, not an executable itinerary. Shared command used:

```text
python3 -B -I ~/ssbu-mod-playbook/tools/eden-runner/harvest.py eden/lr04-shared/harvest-selection.json --out eden/lr04-shared/receipts --label lr-04
```

`harvest-result.json`:6 new segments/11 gates; existing entries preserved.
Failed/unrun routes and sequence-bound13b recovery were excluded. Initial
free-hand approach/pick recordings have reviewed checkpoint prerequisites;
their Steve-name predicate alone does not establish the free-hand position.
Roy→Lucas is qualified only from heldRoy, not as a complete return route.

External evidence root:
`/home/intpa/smash-training-lobby/artifacts/trainingrooms-lead-astra4/rc8/lr04-shared/`.
`final-inputs/` retains complete runner output; `final-state/` retains lifecycle
receipts, snapshot inventory, control log, post-run plugin logs, run/memory.
`console.log.gz` is byte-verified; the full pristine/post-run profiles remain
at `memento-claw-yoga-ubuntu:/home/intpa/tr-lr04-yoga-state/`.

## Remaining acceptance

Interrupted teardown×3, pending-build ordering, forms/pairs/hats, pointed
saved-Mii identities, pilot, Developer/resource clauses, fullR3 (including
catch/shield positives, independent pause producer, frame-advantage lifecycle
and active attacks), UI exact-return, coexistence and remaining online paths
remain open. Existing replay evidence is carried separately; ongoing pkg4
online work is not qualified by this solo run. Phase2/3 composition remains
blocked. Compliance dispositions are retained beside the external results.
No push, tag, release, card, DBI or USB action was authorized or performed.
