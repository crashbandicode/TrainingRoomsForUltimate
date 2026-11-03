# lr-03 shared-runner evidence — 2026-11-03

Surface run stopped, restored, separately verified and released. **25 execution rows PASS; 08 and 26 FAILED; 27 deferred without execution. Full RC acceptance remains open.**

## Identity and use

- Product source accepted for this run: `cf4bf44c104ecc96a78d382b4c0276bf33838f18` (comments-only RC8 carry).
- Diagnostic NRO: `3ca7f0c5c9e5552be4725050949b1ba96266cf4f5875bef660f94ba9b323e9e4`.
- Shared runner: `/home/intpa/ssbu-mod-playbook/tools/eden-runner/runner.py`; final observed run commit `2f31d0c5cb4a10b6a3ff4562294969fc8488c98c`; runner SHA256 `c7d501319f9f5a4a420e17e048ad5c9c832182f84f98c1c3dfb0f4138b4017f3`.
- Shared tool changed during execution (including start checks and game-frame recipes). This final identity does not establish one immutable runner across all attempts. No tool copy is included, following rule 7 at playbook `236b0dd`; archival read followed `022be0d1c938b3873d4a06cc72ff266b273ded8c`.
- `rows.json` is the final **Training-start continuation**, with bootstrap history in `prior_rows`. It is not a fresh-boot script. Paths to available segments/references are relative. Segment and reference hashes are embedded.
- Portable shared dry-run: 18 active rows, 43 gates, zero errors; `READY_TO_RECORD` because unused deferred `cpu40-lucas-pick-frames`, `cpu-lucas` and reference `cpu-sonic-view` remain absent. Dry-run does not qualify recordings.

```text
python3 -I /home/intpa/ssbu-mod-playbook/tools/eden-runner/runner.py dry-run eden/lr03-shared/rows.json --out <external-output-directory>
```

## Recording dispositions

These recordings are qualified only for their observed starting state and subsequent gate/checkpoint, not arbitrary saves or layouts.

| Segment | Observed qualification |
| --- | --- |
| `smash-main` | Smash submenu → main; recovery recording, row01 |
| `main-stage` (existing `../macros/` file) | Main → Training stage select; actual replay, row02 |
| `stage-css` | Selected Training stage → CSS; row03 |
| `cpu74-lucas`, `lucas-pick` | CPU token already held at tile74 → Lucas39 → selected Lucas; row05 |
| `enter-training` | Ready Steve/Lucas CSS → Training originals/catalog owners; row06 and reentry28 |
| `pause-open`, `pause-close` | Native Training pause ownership/release; recorded then replayed in09; delay2 leak clause remains unexercised |
| `p1-next-frames` | Initial fixture forward P1 cycle; rows10–21, exact ticket/target/release and reviewed bodies/HUDs |
| `cpu-next-frames` | Initial fixture CPU Gunner → DK → original Lucas; rows22–24; saved-Mii pointed payload identity unverified |
| `focus-native-quit-frames`, `confirm-native-quit-frames` | Native pause → quit focus → CSS with both warm finalize gates; row25 |
| `steve-hover` | Fresh tile78 observed inside initially failed row04; not independently harvested |
| `coin-final-pick` | Actual acquisition confirmed at tile74, token=p2; original gate incorrectly expected token=cpu. Endpoint evidence only, not complete route qualification |
| `title-main`, `movie-title` | Startup timing/overshoot failures; not qualified |
| `steve-cpu-coin`, `selected-steve-to-cpu-coin`, `coin-correction`, `coin-correction-measured`, `coin-from-banner`, `coin-pick-refine` | Failed or intermediate CPU-coin recovery attempts; not qualified standalone routes |
| `css-return-cpu-pick-frames`, `css-cpu-near-pick-frames` | Failed row26 fresh-token gate; not qualified. Return CPU39 layout differs from initial Random CPU86 |

Shared `harvest.py` added 12 segments and 30 gates using actual PASS receipts and the historical+continuation row selection. It omitted implicit start-gate dependencies `smash-submenu` and `native-pause`; this shared-tool gap was reported to the primary. Library integration remains incomplete until those dependencies are supplied. Failed segments were excluded.

## Results and retained evidence

READY18:35Z; Up18:36:44Z; first row completed18:41:25Z; Down19:15:13Z; DONE19:15:18Z. Native pause/resume, two P1 laps, CPU cycle, warm finalize and fresh reentry were reviewed. First Falcon screenshot had HUD lag; the actual19:00:21 failed next-gate screenshot resolves it. Subsequent rows use changed/stable HUD gates.

`game.stop` accepted job2 but did not reach successful terminal completion before its cap. Forced Down stopped the process; **cleanup failure retained**. Full restore and separate verification each report originals restored (1604 profile + 10 harness entries). Exact owned CLAIMED/RELEASED lines and an owner-inclusive no-open-claim check are retained. Scanner receipt reports 17983 console lines, zero failing/other-Unmapped/panic-abort-crash entries.

Canonical external evidence root:
`/home/intpa/smash-training-lobby/artifacts/trainingrooms-lead-astra4/rc8/`

- `phase1-runner/`: exact executed `rows.json`, `instances.json`, 19 execution logs, `attempts/`, `out/` receipts/actions/checkpoint media, and `rows.harvest.json` (data selection only).
- `runtime/lr-03/RESULTS.md`: final disposition; `COMPLIANCE.md`: completed checklist.
- `runtime/lr-03/`: complete console/control/run/memory outputs, mutable post-run files, stop-job/error, restore/verify and ledger receipts; `archive-exact-copies/rows.executed.json` preserves the exact final executed bytes.
- `runtime/lr-03/archive-provenance.json`, `archive-files.sha256`, `archive-check/dry-run.json`, `harvest-result.json`.

Unclosed: delay2 leak check; CPU return route; interrupted builds×3; special forms/pairs, hats, exact saved-Mii identities; pilot behavior; Developer/resource clauses; full R3 active attack/catch/shield, independent pause producer and frame-advantage lifecycle; exact UI return contract; coexistence and remaining online clauses. Phase2/3 composition remains blocked. No push, tag, release or device action was performed.
