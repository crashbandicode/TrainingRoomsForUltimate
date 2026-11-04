# Menu GIFs

Demonstration clips of the TrainingRooms menu, recorded for the operator's
review. All four are actual Eden footage (Eden-GIF's built-in recorder, whole
screen, 1280×720, 10 fps equivalent) from the RC9 release boot on the Surface:
runs `glm-gifs-02` and `glm-gifs-03` on 2026-11-04Z, product build `5205a8f`
(merged to `work/implementation` as `5ed8db1`), NRO SHA-256
`7e6b7c13e134fde08a63b7ecc52f68a98c1d73b85bb6908c0817ea31d3079a89`.

Each clip is one continuous take, trimmed only at the ends (game-speed timing
untouched); the first, middle and last frames of every published file were
inspected before it was kept. The raw takes are retained outside git under
`/home/intpa/eden-runs/glm-gifs-0{2,3}/out/gifs/`.

![Menu tabs](media/menu-tabs.gif)
*In Training, `B`+`Up` opens the menu and `R` steps the active tab through the
five pages — CPU · Saves · Recording · Fighters & Stages · Display & Controls —
then `B` closes it. 9.8 s. Eden, 2026-11-04Z, build `5205a8f`, NRO `7e6b7c13`.*

![CPU direction grids](media/menu-cpu.gif)
*CPU tab: `Down` to DI Direction, `A` opens the direction grid, `Right`+`A`
picks Out with weight ×1, `B` back (the row reads "1 selected"), then on to the
SDI Direction grid. 9.5 s. Eden, 2026-11-04Z, build `5205a8f`, NRO `7e6b7c13`.*

![Character cycles](media/menu-fighters.gif)
*Fighters & Stages → Character Cycles: `A` adds the focused fighter to the
Player cycle order and the rank badges appear (5. Mario, 6. Donkey Kong);
`ZR` switches role. 8.6 s. Eden, 2026-11-04Z, build `5205a8f`, NRO `7e6b7c13`.*

![Stage grid rotation](media/menu-stages.gif)
*Fighters & Stages → Lobby Stages stage grid: the cursor steps along page 1 and
`A` adds Battlefield to the rotation (rank 2 appears); the help line lists
`Y` form and `X` direct. 5.0 s. Eden, 2026-11-04Z, build `5205a8f`, NRO
`7e6b7c13`.*

Not published from the same boots: a Recording-chord clip (the record-side
notices Lockout/Standby/Recording were captured, but the 180-frame recording
outlasted every window under the recorder's slowdown, so no playback notice was
captured) and a defaults clip (`X`/`Y`/`ZR` pressed, no visible result inside
the window). The raw takes are retained with the runs.
