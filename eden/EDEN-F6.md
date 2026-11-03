# Eden results, F6 (the first-boot group-build hitch)

2026-11-02, Surface (display `:97`, muted through the control API before every `game.load`, offline), own profile
`/home/intpa/trainingrooms-eden-f6/profiles/f6` (hard-linked copy of `ssbu-desync-1029/profiles/g`), plugins: the TrainingRooms diag
NRO as `libtraining_rooms.nro`, standalone CPU pilot `e3033426…`, tile cursor `b8313ca2…`. Settings through `settings.json` only
(`eden/settings/f6-boot.json`: Player Swapping on, P1 roster R.O.B., Lucario, Mr. Game & Watch, Kirby, Captain Falcon, Dark Pit,
Toon Link; CPU roster Pikachu, Bowser, Donkey Kong, Fox, Link; preload rows written out). Driver `eden/f6_boot.py` (route
`eden/route_f6.py`, rows `eden/f6_rows.py`). Numbers: `python3 -I tools/f6_split_table.py eden/runs/<label>/TrainingRooms/practice-diagnostics*.log`.

## The question
The carried Phase B′ group build (`runtime::roster::native_body_bank`) reported 75-107 ms of `setup_us` on the game thread for the
first build of a boot, 10-16 ms later (fork stage M5q, `R/stages/m5q/eden/{A,S}/RESULT.md`, W20 `R/c1162/group-build-offthread/`).

## f6-01 (diag `cfe67ff9`, the split meter): precondition miss
The route's install-line gate waited for `result=installed`; the integrated tree's install line is 8,142 characters and is cut before
`result=` (reported; owner integrator-3). The Training gate compared a Final Destination reference; this profile's stage tile 6 is
Kongo Jungle. Both gates now read the install line's start and the plugin's `[CharacterRoster] observed owner=0`. No laps.

## f6-03 (diag `cfe67ff9`): where the first build's setup time goes
First group build of the boot, P1, the Kirby-first group `[6, b, 1f]` (Kirby, Captain Falcon, Dark Pit), asynchronous:

| step (game thread, µs) | first build of the boot |
| --- | ---: |
| `0x64f820` control init (AI params per context) | **127,566** |
| `[Registry]` line formatting | 1,703 |
| plugin code between the natives | 973 |
| `exchange_articles` | 589 |
| `0x17f26c0` unprepare (U18) | 567 |
| `0x654dc0` dispatcher arrays | 93 |
| `0x655320` control contexts | 56 |
| memory sample (4 × `svcGetInfo`) | 38 |
| 3 × `0x3ae5f0` + task + functor + `0x3549720` | 191 |
| carried `setup_us` / split total | 130,551 / 131,822 |

The swap: `built ok=true us=1,996,840`, `settled_us=[1,069,524, 1,493,329, 1,994,235]`, 105 ticks, `max_tick_gap_us=174,666`.
Gap timeline (`[BuildSplit] gaps`): `t49:174665:poll, t2:147833:setup, t34:26512:poll, t108:25447:poll`: the setup tick took
147.8 ms; the largest gap follows a plain poll tick mid-construction (near Kirby's body settling at 1.07 s), not the plugin's setup or
settlement (settlement tick 8.0 ms: verify/ack 3.3, ledger 2.3, memory sample 1.1). Plain poll ticks: up to 26.5 ms.
Health: `console.log` 12,072 lines, 0 panic/abort/crash/Unmapped; `memory.peak` 7,323,860,992 B.

**Root cause (static, owned 13.0.5 ELF and owned decompile `native_entry_dispatch_6556b0.c`):** native loader state 8 runs `0x654dc0`
and `0x655320` on the game thread and queues `0x64f820` as a task on the worker (functor vtable `0x4f81510`, slot `0x6574e0`:
`ldr x8,[x0,#8]; ldr x0,[x8]; b 0x64f820`; callback `0x655f30`; `0x3549720(task+0x10, 0x655f30, 2, 1)`); state 9 waits for it,
state 10 allocates the bodies. The carried Phase B′ ran `0x64f820` on the game thread, in the synchronous `0x658af0` order (whose only
caller is the respawn `0x65be14`).

## f6-04 (fix diag `3c9ff2d5`): boot-time stall before the main menu (finding)
`[GroupSwap] async build enabled functor_slot=0x8120e2a0 expected=0x8120e2a0 control_slot=0x8149a4e0 expected=0x8149a4e0` (the new
live check of the state-8 functor slot passes). Then the game stalled before the main menu: plugin `hook_stats` at +23 s (2 calls) and
+37 s (844 calls) after the install line, nothing for the next ~40 s; the opening movie's NVDEC line at 58.6 s console time (31.6 s in
f6-03), which is also the console's last line; screenshots black. No group code ran. The tree lacked stream/sync (the F8 lock livelock
fix); not attributed further. Reported; the next launch runs on the merged tree.

## 2026-11-02 Astra continuation: frozen `7bdeacf`

Plugin source is frozen at `7bdeacf`; diagnostic NRO SHA256
`fd6a78bcfb54372bf082b0e9119a40447e760a29e52990120b58ffc9653bc59f`.
Shared harness is `225d16b`; driver fixes are `ca2ab5c` (initial bank bootstrap)
and `4f9f585` (native configuration and exact-owner completion gates).
These Python changes do not change the plugin. All runs use muted Surface
profile `f6`, display `:97`, under the lead's 16:09Z claim. Runtime receipts,
console hashes, screenshots and raw diagnostic logs are retained per run.

### f6-05: Kirby-first group

Both initial bank entries completed, followed by eight P1 and four CPU laps.
The first asynchronous group `[6,b,1f]` reports carried setup 4,046 us and
split setup 4,723 us, including control scheduling 55 us. Queue work is
1,087 us; the setup-following gap is 23,392 us. The largest gap, 146,094 us,
follows a poll tick. The second (CPU) group `[1,7,2]` reports split setup
81 us and largest gap 22,316 us. See `runs/f6-05/builds.json`,
`split-table.md`, `stalls.json` and `laps.jsonl`. The Kirby screenshot was
visually checked: Kirby/Pikachu bodies and HUD are present.

**Failed health gate:** 2,072 Unmapped Read8 lines at console 75.380563–80.098743 s,
addresses `0x50494c468b..0x50494c4692`; attribution remains open. Console SHA256
`a54fab0e8a06cd4f6fc86a262a0dac5a0f0c1666190a1dd343704c9ff173d26f`.
Peak observed memory 6,892,797,952 B within the 8 GiB unit. Hook allocations
remain nonzero on this pre-R5-integration binary. This is timing evidence,
not RC acceptance.

### f6-06: rejected native baseline configuration

The JSON asked for preload and bank Off, but runtime `cycle_preload_config`
wrote defaults and overlaid the JSON: slots5/bank3/groupsOn/ready8. The model
`boot_cache` implementing D7 has no runtime caller. The lane stopped on its
first completion gate, preserving the failed receipt. Console Unmapped=0.
Pkg6 owns fixing this production wiring; seeding an INI below is only a
controlled experiment on the frozen binary, not a product fix.

### f6-08: native reload, explicit Off JSON and INI

The startup line confirms slots0/bank0/groupsOff. The first reload physically
completed at diagnostic #307 through `MiiOrdinaryPlayer`, but the old driver
expected `CharacterRoster complete` and retained a failed receipt. After
checking the exact-owner adapter's readiness/publication/release boundary,
`4f9f585` accepts its role-matched ready event only with `released=true`.
The same boot then completed seven P1 and four CPU continuation laps; the
original failed attempt remains in `laps.jsonl`, and the two driver logs
are separate. No claim that the original driver attempt passed.

Kirby native loading has a 273,654 us P1 entry-tick stall at loader11, with
the CPU simultaneously reporting 273,682 us. This demonstrates a loading
stall with asynchronous bank construction disabled; it does not attribute
every poll gap to one native function. Console Unmapped=0. The run was
stopped and collected before the next boot. Final F6 acceptance still needs
ordinary-first evidence, R5 zero, zero-Unmapped proof and integrated gates.

### f6-07: ordinary-first precondition failure

The route's early owner-observation gate passed before the initial Training
reset and startup notice. The immediately sent first bootstrap chord did not
start a reload; no queued/ready event followed, so the driver stopped. The
notice was delivered later at diagnostic #254, after both owners and the reset
hub. Bootstrap now requires that delivery before any input; a regression
rejects owner-observed-but-notice-absent fixtures. All nine driver tests pass.
No timing acceptance from this run. Console Unmapped=0. It was stopped and
collected, then the profile's single original NRO was restored and its full
SHA256 set compared to source g. Surface released 16:38Z with zero Eden
processes remaining. The ordinary-first row requeues behind existing work.


## Current integrated r6 continuation, 2026-11-03

Frozen source `e1d81d78215bf975ca0e67e1707dae0a46279508`, diagnostic NRO
`331da18e8748fdbd6a5fdb48acd225e9ffa4675638eb8646f6d06dca2a286347`.
The required two standard fresh boots completed their bounded inputs:
f6-13 had 356 Unmapped Read64 events at `0xffff7e80ffff7ea0`; f6-14 had zero.
Both profiles were restored and claims released. Caller diagnostics reproduced
276 Read64 events in f6-15 and 1,520 Read8 events across
`0xffff807dffff809d..0xffff807dffff80a4` in f6-18. Runs f6-16,17,19,20
had zero total Unmapped. Full console counts, hashes, inputs and cleanup
receipts remain under `artifacts/trainingrooms-lead-astra4/f6-*` in the parent.
The f6-15 host stack reaches Dynarmic CPU MemoryRead; guest attribution is
still open. These retries do not close the intermittent health failure.

The old driver also accepted the wrong completion. In f6-14, diagnostic #421
requested Kirby with sequence4; #427 reported copy packages still loading;
#428 readied the fallback R.O.B.; #431 refused that same transaction.
CPU Fox had the same failure at #633/#640. Thus the former `LAPS DONE 12`
reported attempted commands, not the requested sequence. Replaying those
actual lines reproduced the false success without Eden.

The driver now requires the matching ordinary-owner cycle transaction receipt:
same role, ticket and target, with its lease released. Explicit refusal is
persisted in the attempted-lap receipt and ends the lane before another input.
Thirteen driver tests cover the captured fallback/refusal, successful matching
receipts, stale ticket/epoch, wrong owner/target and unreleased results. The
same captured log now fails with the actual refusal. A native submit on return to the original fighter requires its matching native ready receipt.
Native baseline/bootstrap keep their separate existing gates. The production copy-package scheduling
root and a fresh integrated sequence run remain open.

Standard-run timing tables are archived separately: Kirby-first setup was
12,185 us in f6-13 and 4,647 us in f6-14; maximum gaps were 254,605 and
165,804 us after poll ticks. Debugger runs are not performance acceptance.
Actual f6-14 hook allocations reached36 (roster_input14, roster_preload22),
so R5 and the first-switch/plain-lap comparison remain open.
