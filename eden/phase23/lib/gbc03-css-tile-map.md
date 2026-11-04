# Ally Training-mode CSS tile map and coin facts (boot live-gbc-03, 2026-11-04)

Observed on the Ally rig `C:\Eden\glm-bc` (Eden `a3a27006…`, TrainingRooms diag `c04a316c…`,
plugin set bc2 = bc1 minus `libssbu_cpu_pilot.nro`). Facts only; the CPU-coin pickup did not
complete this boot, so the coin route below is partial.

- Tile-cursor steps on the Ally via wa.py ops: `["hold", ["lstick", "up"|"down"|"left"|"right"], 150, 0.6]`
  = L3 + D-pad, one tile. From the P1 panel below the grid, one `up` step lands on the row above
  the panel (here `from=none to=78`), NOT row 1 — the hand enters the grid under its own column.
  Read `settled screen=css at=N hovered=N` from `ssbu-tile-cursor/events.log` after each step.
- Training CSS after Games & More → Training (this profile, 87 tiles, 13-wide rows):
  - Row 1 (tiles 0-12) = Mario 0, Donkey Kong 1, Link 2, Samus 3, Dark Samus 4, Yoshi 5,
    **Kirby 6**, Fox 7, Pikachu 8, Luigi 9, Ness 10, Captain Falcon 11, Jigglypuff 12.
    Tile index equals the plugin's fighter kind on this row (census/GroupSwap kinds: Kirby=6,
    Luigi=9, Ridley=65, Steve=88).
  - Row 7 (tiles 78-86) = Steve 78, Sephiroth 79, Pyra/Mythra 80, Kazuya 81, Sora 82,
    Mii Brawler 83, Mii Swordfighter 84, Mii Gunner 85, **CPU coin 86**.
  - Row 6 (tiles 65-77) = **Ridley 65**, Simon 66, Richter 67, King K. Rool 68, Isabelle 69,
    Incineroar 70, Piranha Plant 71, Joker 72, Hero 73, Banjo & Kazooie 74, Terry 75, Byleth 76,
    Min Min 77.
- Placing and confirming: `a` on a fighter tile places P1's token; a second `a` raises the
  READY TO FIGHT banner (the match starts shortly after). `b` during the banner cancels back to
  the CSS with the token still in hand.
- CPU coin (partial): with P1's token still in hand, stepping to tile 86 and pressing `a`
  PLACED P1's token on the coin tile (P1 panel showed `?`/Random) instead of picking the coin
  up — the pickup needs P1's token already placed and the hand empty (playbook TILE-CURSOR
  "To set a CPU"); the confirm `a` must not fire first. Not yet achieved; next lane: place P1,
  verify the hand is empty (`token=p1` steps only place, no second `a`), then step to 86, `a`
  (expect `token=p2` step lines), step to the fighter, `a` to drop.
- Fighter changes in an open Training match stay on the menu picker (Fighters & Stages →
  Player/CPU Character; opens with cursor at Mario r1c1, red box = cursor, ~4 presses right
  after opening are swallowed by the transition — verify with a shot before `a`), while
  cycle-chord swaps (`bc-player-cycle`, `bc-cpu-cycle` + `waitplug complete owner=N target=…`
  and `[PreloadStatus] … pending=0`) are the log-gated route for catalog fighters.
