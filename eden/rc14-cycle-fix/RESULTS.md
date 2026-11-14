# rc.14 cycle fix: Eden results

Run `cf-p3-y2` (Yoga, 2026-11-14 22:52–23:08Z) on the `542a23f8` diag (`dba1a4c8…`), Eden 2f315fc4, muted, GDB stub on, with the rows.json in this folder. Result: 11 rows PASS, 3 FAILED; z-boot PASS and no Unmapped line in console.log; Down released=True.

- **PASS: the deadlock fix.** In `cf-k2`, press 5 onto the cold Mii Gunner went `demand` → `Mii group record published` → `prefetch … serves_press=true` → built. The press took 334 ms. Before the fix, `cf-p2-x2` deadlocked at the same step (#2418-#2468).
- **PASS: `cf-forced-reload` (2c).** The target-first Reload kept the ordinary groups `[[1a], [13]]` and released the Mii group. The press then reached Mii Gunner warm.
- **PASS: `cf-mii-failfirst` (2d, the rc.12 crash case).** With `reload mii`, the Reload kept `[[4a], [13], [1a]]`. Mii Gunner warm-attached in 81 ms, with no StallWatch line and no Unmapped line.
- **FAIL: `cf-k1-laps`.** The CPU was never short. Lowering Ready Fighters from 8 to 6 frees nothing that is already resident ("Ready 7/7" at 6), so every press was warm. This behaviour predates rc.14; see the handoff.
- **FAIL: `cf-direct`.** The pick of Bowser was `demand declined … no free group slot`, followed by today's target-first Reload. All 3 group slots (`MAX_EXTRA_GROUPS`) were full.
- **FAIL: `cf-k2`.** Presses 3, 8 and 17 had no free slot and took today's Reload (17 was a canonical Mii Reload). Press 12 left a live Mii group: an exit skips `demand`, so it Reloaded.
- **PASS: `cf-setup`, `cf-k1-setup`, `cf-k1-fits` and `cf-restore`.**

Evidence: `artifacts/rc14-cycle-load/eden/cf-p3-y2/` (receipts, `press-tables.md`, the diag logs, console.log).
