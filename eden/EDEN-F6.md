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
