# sh-here-f63c186b-1 (XPS slot 1, 2026-11-14 19:07-19:23Z)

- Build: TrainingRooms diag `8d928c6f` = `rc14/setup-here` `f63c186b` (on setup-b `3bbf051c`); Eden `2f315fc4`; card plugin set (worker A's stage) + tile cursor. Muted (`persisted_volume: 0`), GDB stub on before `game.load` (`gdb_on` port 6583), main menu shot `diag-191250Z-226934-main-menu.png` (not the demo).
- Rows: g-gdb-on, g-load, g-gdb-attached, g-training, **t-setup-here**, z-boot: 6 PASS (runner RESULTS.md: `run/RESULTS.md`). t-setup-here failed once (19:14Z, harness: the first B + D-pad Up after g-training missed, the close's B fired in the game, P1 took damage, the HUD gate read 9.6%); the hook now checks `[Ui] menu opened` and starts from L+R+A; retried on the same boot, PASS 19:20Z. Receipt `t-setup-here/setuphere.json`.
- S1 Load: P1 placed x -5 (bound save jab-start).
- S2 live (diag-pos.req): P1 Mario x -67.28, CPU Dr. Mario x 47.15 at 18.5 %, stage 308.
- S3 Save setup here "Here spot": `setups/here-spot.json` P1 x -67.28, CPU x 47.15, CPU 18.522 %, stage 308; `save_states.json` and `saves/jab-start.json` byte-identical before and after.
- S4 Load: P1 x -5 again (the bound save unchanged, Load still goes there).
- S5 Setup library, A, own screen, Restore here: placed P1 -67.28462, CPU 47.1534 (the setup's spots).
- S6 Save States Off: help line "Save States is Off (Saves page)" (`s6-off-refused` shot), no new file.
- S7 Cycle CPU to the Mii Gunner (`HUD refresh owner=1 kind=0x4a`): "Here mii" keeps kind 74 with an exact Mii identical to the one Save then kept in jab-start.json; Restore here placed the CPU at 47.15339.
- Not run on Eden (host tests only): the busy refusal and the still-loading refusal (no reliable timing in a menu press).
- Console: 0 panic/abort/crash, 0 Unmapped; `trace.unmapped_access` total 0; hook allocations 0 (34 counts).
- Down: released=True, cleanup_failures=[], forced stop (SIGKILL) not a crash; prune 2.6 GB.
