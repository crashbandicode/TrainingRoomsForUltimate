# Eden results, package 3 (save states)

Host: Yoga (`memento-claw-yoga-ubuntu`), own profile copy `/home/intpa/trainingrooms-eden-p3/profiles/t3` (copy of
`ssbu-desync-1029/profiles/b`), `core.volume=0` before boot, one Eden at a time, claim and release in
`/tmp/deluxe-surface-1007/OWNER-COORDINATION.md`. Labels `s3-NN`; evidence under `eden/runs/<label>/`
(`diag.log`, `events.log`, `shots/`). Harness: `eden/s3_eden.py`, `route_s3.py`, `s3d.py`.

Tile routes from the Steve tile (78) after the first `up` (the CSS grid is 13 columns, tiles numbered in
row-major order, Steve first of the last row): Samus up x6 then right (tile 3); Hero up, right x6; Cloud up x2,
right x7; Kirby up x6 then right x4 (tile 6); Pokemon Trainer up x4 then right x9 (tile 37).

## Run s3-01 (save/load core, release plus diag drive by request files)

Install line `result=installed`, `heap_allocations=0`. Pass: Save wrote slot 1 (x -45, y 27.085, kind 88); Load
returned P1 from x -30.1 to -45.0; mirroring None and Alternate (x 45 / -45 / 45, lr -1 / 1 / -1); damage Default,
Save State, Random 50-50; auto-load on death; respawn invincibility Off (drop from above, no platform) and On
(respawn platform, `shots/respawn-sheet.png`); fixture slot 3 Steve restored exactly; recorder adapter line once per
load; hook counts without quake or star KO.

## Run s3-02 (charges, items)

NRO libtraining_rooms `a9ddf957dd2006c104ace90b6c96ecbf69fbc83e4f87120c412590ab0a5929e0` with the pilot
`e3033426...` and the tile cursor `b8313ca2...` installed together; install line OK with the pilot present
(coexistence). Pass: Samus charge 112 restored with the glow (`shots/chg-sheet.png`), Bob-omb held-item snapshot
restored. The `training scene` screenshot gate missed its threshold (31.4 against 30); verified by dump and
screenshot, not a plugin defect.

## Planned s3-03 (diag NRO `e4912885c2cd7fa6c9eb8b4eebdd38a5683c1bab9369e889d2eceb07799755ed`, boot with the pilot)

1. Cloud buff Limit (`s3-cloud-states.json`, slot 3, kind 62): after Load the Limit gauge is full, no summon or
   cutscene, the silence holds (`buff_done` line).
2. Pilot coexistence: install line with the pilot, `req_on_joint` hooked by this plugin and called by the pilot.
3. Pokemon Trainer (`s3-pt-states.json`: saved Ivysaur, slot 3; P1 starts with Squirtle): Load; expect the trainer
   not to throw a ball, Ivysaur out at x -20, no flying plate, no switch sound (`se_silenced` > 0, `se_passed` = 0),
   `decide=1`, `pt_switch_done` line, trainer grounded. If `pt_decide_rejected` appears, log the descriptor and
   root-cause it.
4. Kirby: `diag-copy.req` (opponent's hat via the game's own routine), `diag-pos.req` (dump: copy flag, chara,
   module kind; report which of `INT_COPY_CHARA` and `+0x17398` matched the copied kind), Save, Load, dump again:
   the hat returns, `copy_hook` = 1; the saved file has `copy_charge` only for a copied fighter that has a flag.
5. Sound check: no `state se ... silenced=false` line during a load, `se_passed` = 0.

## Planned s3-04 (first-boot import on a copy of the fork folder)

Copy of the fork-folder fixture including a `developer.ini` of exactly `developer_page=on` plus one LF; run the
checker `check-migration.py verify`: imported files hashed, originals untouched, the marker written last, the
staging folder skipped and counted, the item setting converted once on the copy.
