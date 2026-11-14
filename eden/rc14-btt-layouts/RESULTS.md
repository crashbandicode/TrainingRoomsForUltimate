# btt5-214653c2-1 (Yoga slot 1, display :77, 2026-11-14 20:49:56–21:09:20Z)

- Build: TrainingRooms diag `214653c2` (`rc14/btt-layouts`), sha256 `d1bb900e…`; Eden `2f315fc4b7d9`; base config `wa-eggsfly-9c16babe-1` (card set + tile cursor). Muted by the lifecycle, GDB stub on before `game.load` (`g-gdb-on`, `g-gdb-attached`); the main menu shot `m-main/…-rc13-main-menu.png` is the real main menu.
- Rows (`rows.json` btt-5 set): 14 PASS, 0 FAILED. Receipts in the run's `out/`, copied to `artifacts/rc14-btt-layouts/eden-btt5-214653c2-1/`.
- btt-E41 PASS (retry 3): two ZL + Down on Battlefield made two new files, the newer chosen 1/2; the list newest first, only Battlefield's.
- btt-E43 PASS (retry 1): own screen Load/Rename/Duplicate/Delete; Rename "Ledge trap" kept the file id; Duplicate made `Layout N · Battlefield` at the top; Delete asked (Keep it, Delete it) and removed only that file.
- btt-E42 PASS: the seeded `layouts/seeded-big-battlefield.json` named 1/1 by ZL + Right, loaded by ZL + Up (3 targets), Ready.
- btt-X-stages PASS (retry 1): Big Battlefield listed only its own; a save there changed no Battlefield file; back on Battlefield ZL + Right 1/3 then 2/3 (the older), ZL + Up loaded it and the writer listed it first.
- btt-X-saves PASS: 7 files; ZL + Right/Left stepped the five newest and wrapped; ZL + Up on 3/5 made it 1/5; ZL + Y twice deleted it, the five refilled.
- btt-G3 PASS: `btt-save-load.gif` (save, Y twice, load back).
- The retries were the hooks', fixed on the same boot: the panel reports its tiles cut and padded to 32 (labels now compared by prefix); E41 placed no fresh layout when one was up (now always fresh); the targets 8 apart are the overlap limit (now 12).
- Console: 0 panic/abort/crash/Unmapped (GDB stub on, no trace); hooks `heap_allocations=0`.
- Down: cleanup_failures=[], released=True (ledger RELEASED 21:09:20Z); forced stop (SIGKILL), not a crash. Prune 2.6 GB.
- The chain script was stopped by hand after the first runner pass so the same boot could retry; down was run by hand.
