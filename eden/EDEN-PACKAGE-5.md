# Eden evidence, package 5 (TrainingRoomsForUltimate menu rendering)

Dated 2026-11-02. Host: the Yoga (`yoga-ubuntu`), own work root `/home/intpa/trainingrooms-eden-p5`, profile `t5` (copy of
`ssbu-desync-1029/profiles/b`, game content hard-linked), display `:95`, muted (`core.volume` 0 persisted before `game.load`), one Eden at a time,
claims `CLAIMED`/`RELEASED` in `/tmp/deluxe-surface-1007/OWNER-COORDINATION.md` (owner `Claude 0caa72b8/pkg5-eden`). Plugins: the diag build of this
branch, the CPU pilot `libssbu_cpu_pilot.nro` (sha256 `e3033426...`) and the tile cursor `b8313ca2...`. Route and driver: `eden/route_s5.py`,
`eden/drive_s5.py`, `eden/s5_eden.py` (Yoga side). Outputs under `eden/runs/<label>/` (gitignored); `console.log` scan: 0 matches of panic / abort /
crash / Unmapped in every run.

| Label | What |
| --- | --- |
| s5-01 | diag NRO with the archive hook only: stock `info_training` archive dumped (451,604 B, fnv `8833748e9518a461`, sha256 `90a8dc81...`) |
| s5-02..s5-06 | archive injected, 50 panes resolved, text buffers: found that the game sizes a text box's buffer from its text id, so the boxes carry none; colours are linear |
| s5-07 | first accepted menu: `menu-open`, `nav-sdi-right`, `closed` screenshots |
| s5-08 | HUD hidden while open, pilot present (`pilot_present=true`, one info row in "Player 1 control") |
| s5-09 | near-opaque panel; pages (CPU, Saves, Recording, Fighters & Stages, Display & Controls) and the Mash, SDI Strength, Ledge Delay and DI popups |

Pilot absent, Developer page, grids, HUD Off and the per-mockup comparison: batch 2.