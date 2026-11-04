# Menu GIFs

Demonstration clips of the TrainingRooms menu, recorded for the operator's
review. All six are actual Eden footage (Eden-GIF's built-in recorder, whole
screen, 1280×720) from RC10 release boots on the Surface: runs `glm-gifs-04`
and `glm-gifs-05` on 2026-11-04Z, the stage-grid clip from `glm-mods-10`, and
the fighter-grid clip retaken from `glm-icons-06` (Eden-GIF `6a0c80d2…`; the
card's ARCropolis `92171b2f…` pinned and the profile's Cosmic Destination
stage mod live, so the mod-aware loader's mod side is exercised). The
character grids draw the game's own stock icons (the native key builder's
`picture=0` waiting-room icon at the fighter's default costume — the art the
fork's menu showed), loaded at run time through the game's mod-aware
file-resource service (operator decision 2026-11-03; SPEC-core-07 Q16
amendment). The first cut read the role's presentation row for the costume
instead; that byte belongs to the match entries, so a grid followed the
entries' costume (Eden `glm-icons-03` tracepoints on the key builder: the CPU
page requested `chara_0_*_07` for every fighter) or named files that do not
exist (`glm-gifs-04`, `glm-mods-10`: blank tiles) — fixed on this build. The
Player list draws the stock icons; after `ZR` the CPU list still renders blank
for at least ~2.5 s while it re-requests the same files the Player list just
released (`glm-icons-06`; with never-before-loaded files the same page drew in
≤2 s in `glm-icons-03`) — open defect, under investigation, not hidden.

Each clip is one continuous take, trimmed only at the ends and under 10 s
(game-speed timing untouched); the first, middle and last frames of every
published file were inspected before it was kept. The raw takes are retained
outside git under `/home/intpa/eden-runs/glm-gifs-0{4,5}/out/gifs/`. The
recording clip is full-motion gameplay, so it is kept at a lower frame rate
(≈3.3 fps) and palette to stay reviewable in git; the others keep the source
frame times.

![Menu tabs](media/menu-tabs.gif)
*In Training, `B`+`Up` opens the menu and `R` steps the active tab through the
five pages — CPU · Saves · Recording · Fighters & Stages · Display & Controls —
then `B` closes it. 7.9 s. Eden, 2026-11-04Z, RC10 candidate, NRO `50d95130`.*

![CPU direction grid](media/menu-cpu.gif)
*CPU tab: `Down` to DI Direction, `A` opens the direction grid, `Right`+`A`
picks Out with weight ×1 — the row ends reading "Selected: Out ×1". 9.8 s.
Eden, 2026-11-04Z, RC10 candidate, NRO `50d95130`.*

![Character stock icons](media/menu-fighters.gif)
*`R` steps to Fighters & Stages and Character Cycles opens: the grid draws each
fighter's own stock icon at the default costume, loaded from the game at run
time (the art the fork's menu showed); `A` adds the focused fighter to the
Player cycle order and the rank badges appear (1. Squirtle, 4. Captain
Falcon), `ZR` switches role — and the CPU list re-request of the same files
still renders blank (open defect, see above). 9.9 s. Eden, 2026-11-04Z, RC10
candidate, NRO `7cc1d6ce` (diag-feature build; the release build of the same
commit is `cf8fc543`), ARCropolis `92171b2f`.*

![Stage grid](media/menu-stages.gif)
*Fighters & Stages → Lobby Stages stage grid: the thumbnails load at run time
through the game's mod-aware loader, and with ARCropolis pinned and Cosmic
Destination in `ultimate/mods` the Final Destination tile draws the mod's
nebula picture (matching the mod's decoded `stage_1_end.bntx`) while every
other tile shows stock; the cursor steps along page 1 and `A` adds Battlefield
to the rotation (rank 2 appears). 9.7 s. Eden, 2026-11-04Z, RC10 candidate,
NRO `50d95130`, ARCropolis `92171b2f`.*

![Record and playback](media/menu-recording.gif)
*The recording side: with `ZR`+`Down` armed, Player 1's inputs are captured
(the input display and the fight show the recorded window), and `ZR`+`Up`
plays them back under the "Input Recording — Playback" notice. 9.9 s, ≈3.3 fps.
Eden, 2026-11-04Z, RC10 candidate, NRO `50d95130`.*

![Defaults keys](media/menu-defaults.gif)
*The page keys the hint line names: `X` save defaults, `Y` reset row,
`ZR` reset page, pressed on the Display & Controls page whose rows were
already at their defaults, so no value moves; the keys and the hint are the
clip's subject. 9.8 s. Eden, 2026-11-04Z, RC10 candidate, NRO `50d95130`.*

Not published: a defaults take with a visible `Y` reset (the CPU page's DI row
holding the CPU clip's pick) — the menu reopens on the page of the previous
close, and none of the recorded takes landed on a page that still held a
non-default value; the raw takes are retained with the runs.
