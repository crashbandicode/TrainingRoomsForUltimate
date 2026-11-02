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

## Batch s3-03 to s3-07 (2026-11-02, Yoga, pilot and tile cursor installed, `core.volume=0`, console.log scan 0 matches except s3-03)

| Run | NRO (sha256) | Source commit | Result |
| --- | --- | --- | --- |
| s3-03 | diag `e4912885c2cd7fa6c9eb8b4eebdd38a5683c1bab9369e889d2eceb07799755ed` | 4fdf560 | install line `result=installed` with the new rows (`state_pokemon_decide=stock/near`, `state_kirby_opff=stock/far`, `state_effect_req_on_joint_hook=stock/far`, pilot present). **FAIL**: Cloud Load with Limit crashed after `done ... next=ApplyBuff` (`ExceptionRaised ... unmapped address 0x0`, PC=4, LR main+0x8dc1b0; `runs/s3-03/console-excerpt.txt`). Root cause: the game's add-limit routine reads the work module at `x0 + 0x50`, only a module accessor has one; `cloud_add_limit` passed the fighter object. Fixed in 1076ddc (site note too). |
| s3-04 | diag `d36dd4f4d8dc11e6baa121226950975f7e3c1cf1bccefc109481b4774c41089d` | 1076ddc | **Pass**: Cloud Load with Limit: `buff_done entry=0 kind=Cloud`, `restored`, Limit gauge full with the glow (`shots/062351-cloud-after-load.png`); the CPU Wii Fit Trainer also buffed and done. |
| s3-05 | same | 1076ddc | **Pass with two findings**: Pokemon Trainer (P1 Squirtle, saved Ivysaur): `pt_decide entry=0 saved=37 descriptor=Descriptor { count: 3, kinds: [36, 37, 38] }` (accepted, no rejection), three trainer-switch sounds `silenced=true`, the change-light effect hidden for the Pokemon (kind 36), `pt_switch_done grounded=true`, Ivysaur at x -20 (`shots/062629-pt-after.png`). Findings: `heap_allocations=1` after the load: `decide_index` built a `Vec` in the hook (root cause, fixed in 13be87c); the position dump printed only the CPU because a switched Pokemon runs after the CPU (dump bug, fixed). |
| s3-06 | diag `e39444a8d39d98f0942bcacfa3f401e5cd8a93370bcf78a0f283d28f5c4a90f2` | 13be87c | **Pass**: Kirby vs CPU kind 11: `diag-copy.req` gave the hat by the game's own routine; dump `copy_flag=true chara=11 module_kind=11` (both sources matched the copied kind, spec open question 5); Save wrote `has_charge true, int_z 11`; Load: `kirby_copy ... accepted=true`, the copy sound `0x14bdd3e7c8` `silenced=true`, `kirby_copy_started`, hat back (`copy_flag=true chara=11`), `copy_hook=1`, `heap_allocations=0`. (The copied charge of Samus and the other fighters is host-tested; the CPU was random.) |
| s3-07 | same | 13be87c | **Pass**: Pokemon Trainer again: `heap_allocations=0`, both bodies dumped (P1 kind 37 at x -20), `se_silenced=3 se_passed=0 decide=1`; then Down+B switched Ivysaur to Charizard (kind 38) and a Load of the saved Ivysaur brought out Ivysaur (kind 37, spec edge case 1) with no rejection and the same three silenced sounds. |

Evidence: `eden/runs/s3-03` to `s3-07` (`diag.log`, `events.log`, `run.json`, `memory.peak`, `save_states.json`,
`shots/`). The Kirby and Sora load-time silences showed `se_passed=0` in every run (spec open question 4 holds
on these rows; Sora's jingle was not exercised).

## Runs s3-08 to s3-10 (first-boot import on a copy of the fork folder, release NRO)

NRO release `aefbcad56be10d8fa16c39b3ae2be7e9a2a874b3984235f1788489d4a2685948` (13be87c) with the pilot and the tile
cursor; no `settings.json` or `save_states.json` placed, `developer.ini` = `developer_page=on` plus one LF placed in
`TrainingRooms/` (as the card swap does), the fork folder `TrainingModpack/` in the profile replaced by a fixture:
`training_modpack_menu.conf` (the real one), `training_modpack.toml` (legacy, not imported), a ported fork-written
`save_states.json`, `button_config.ini`, `replay_capture.ini`, `mii-presets.json`, `setups/one.json`,
`macros/one.json`, `replay_captures/cap-1.bin`, a staging folder `replay_captures/.cap-2.smd-12345.tmp/part`, and a
fork trace file `practice-trace-1.log` (not imported).

| Run | What | Result |
| --- | --- | --- |
| s3-08 | first attempt | not run: the harness wrote CLAIMED while the foreign `m5qo1` claim was open and the GPU lock refused; released at once (process slip, see IMPLEMENTATION-PROGRESS-pkg3.md) |
| s3-09 | fixture as above (character item setting None) | **Pass**: install line `migration=done captures=running skipped_staging=1 result=installed`; `migration.json` `result=done`, `converted_item_snapshots=0`; `check-migration.py verify` `MIGRATION CHECK PASS` (fork folder unchanged, every planned file present with the planned bytes and the receipt hashes, the staging folder not copied and counted, nothing from the not-imported list, `developer.ini` untouched and not taken as the marker). Scan 0 matches. |
| s3-10 | same, with the conf's `character_item` = Player 1st Var. and a `save_states.json` whose player records are Daisy without a snapshot | **Pass**: `converted_item_snapshots=5`; the copied `save_states.json` has the Smile Turnip snapshot (`choice 2, variation 0`) on the five player records and none on the CPU records; the fork's own file is unchanged; `verify` PASS. Scan 0 matches. |

Evidence: `eden/runs/s3-09`, `s3-10` (`rooms/`, `fork-after/`, `verify-report.json`, `diag.log`, `events.log`,
`run.json`, `memory.peak`; gitignored). Not run: the I/O-error and interrupted-background cases, which are covered by the
13 filesystem host tests (`tests/migration_run.rs`).

