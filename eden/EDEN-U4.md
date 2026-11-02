# Eden evidence, stream u4 (a game-file texture on a picture pane)

Dated 2026-11-02. Host: the Yoga (`memento-claw-yoga-ubuntu`), own root `/home/intpa/trainingrooms-eden-u4`, own profile `u4` (copy of
`ssbu-desync-1029/profiles/b`, game content hard-linked), display `:90`, muted (`core.volume` 0 set through the control API and saved
before `game.load`), one Eden at a time, claims `CLAIMED` / `RELEASED` by `tr_eden.py up/down` in the Yoga ledger (owner `Claude 0caa72b8/u4`;
the Surface was claimed only for compile). Harness: `eden/u4/` (`tr_eden.py` + `tr_eden.u4.config.json`, `route_u4.py`, `shoot_u4.py`; the route
is pkg5's: title -> main menu -> Training, Steve vs Roy on Pokemon Stadium 2, every wait gated on a log line or a screenshot match).
Plugins: the diag build of this branch (`b7ca084` for u4-01, `b7ca084` plus the probe's repeat cycle for u4-02 and u4-03; see the table), the CPU pilot `libssbu_cpu_pilot.nro` (`e3033426…`), the tile cursor
`b8313ca2…`, and ARCropolis 4.1.1 (`92171b2f…`) in u4-01 and u4-02. The profile's `mods/` holds the operator-style Final Destination mod
"Cosmic Destination" (replaces `ui/replace/stage/stage_1/stage_1_end.bntx` and the other three slots).

| Label | Set | What | Result |
| --- | --- | --- | --- |
| u4-01 | diag `0fd244d7` (probe without the repeat cycle) + ARCropolis | first run: guards, request, poll, bind, retire, release | all nine guards `Stock`; four of five requests loaded (the fifth, `stage_1_not_a_stage`, is not in the resource table: `loaded=false`); ready after 3 UI frames; bound; retire and release after 8 frames; `allocations=0` throughout; the hold window was missed by the screenshot driver |
| u4-02 | diag `ad9c3817` (probe repeats every ~25 s) + ARCropolis + the mod | screenshots while bound, three cycles, one screenshot after release | `shots/082614-mod-c2-1.png`: tile 0 Battlefield (stock), tile 1 the mod's Final Destination (a nebula), tile 2 Small Battlefield (patch stage), tile 3 Hyrule Castle; `shots/082711-released-c4-0.png`: the normal Training HUD after release, nothing left behind |
| u4-03 | diag `ad9c3817` without ARCropolis | the same cycle on stock files | `shots/083224-stock-c2-1.png`: tile 1 is the stock Final Destination (red, the Master Hand arena) where u4-02 showed the mod's picture: the mod-aware route is the game's own loader |

Per run: `console.log` 0 matches of panic / abort / crash / Unmapped (5,634 / 5,634 / 5,666 lines); `memory.peak` 6.00 / 5.90 / 6.13 GB (Eden, 8 GiB
limit); each run ended by `down` (`reason: interrupted`, Eden processes left: 0). Logs, `run.json`, `memory.peak` and screenshots are under
`eden/runs/<label>/` (gitignored).

## What the log showed (u4-02, `practice-diagnostics.log`, tag `[Texture]`)
- `install usable=true guards=[Stock x9] service_slot=0x858bff20` (the lookup's `adrp`/`ldr` decoded to `main+0x5332f20`).
- Per cycle: `request cycle=N ... allocations=0`, five `request idx=` lines (hash40 per path, `loaded=true` x4, `loaded=false` for the absent file), four `ready idx=
  after=3|4` lines (`handle=0xffffff job=0`: the handle consumed, no pending job), `shown cycle=N bound=[true,true,true,true,false] allocations=0`, 900 frames later
  four `retire` lines, and `released cycle=N after=8 allocations=0`. Cycles 0-3 repeat identically, no growth.
- R5: `heap_allocations` (the counting allocator's hook-scope count) stayed 0 at every line; the game's allocator made the UI textures.

## Not covered
- The layout being destroyed while a texture is bound (the helper's `release` is independent of the pane; the page must `unbind` first).
- A file whose load fails after the request (the helper reports Loading forever; pkg5's page times out to the text tile). Not provoked here.
- 40 tiles at once (the probe binds four); each texture is one 0x68-byte object plus the game's 20 KB copy and GPU memory.
- Switch hardware: Eden only.
