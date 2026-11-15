# rc.14 cycle fix: Eden results

All runs used Eden 2f315fc4, muted, with the GDB stub on. Evidence for each run is in
`artifacts/rc14-cycle-load/eden/<run>/` (receipts, `press-tables.md`, diag logs, console.log).

- **cf-p3-y2** (Yoga, `542a23f8` diag `dba1a4c8`): 11 PASS, 3 FAILED; z-boot clean.
  - PASS: the deadlock fix. In `cf-k2`, press 5 onto the cold Mii Gunner went `demand`, then publication beside the press, then the serving prefetch, then the build (334 ms).
  - PASS: 2c, and 2d with an ordinary window.
  - The failures found Gaps 1-3: lowering Ready Fighters freed nothing; no free group slot (Gap 2, a-full-2's); a warm Mii exit skipped `demand`.
- **cf-p4-y1** (Yoga, `7e71c77a` diag `8469b99c`): 9 PASS, 2 FAILED; z-boot clean.
  - PASS: the settings fixture (`cf-boot` read "Ready 6/6" from the first frame).
  - PASS: Gap 3 (`cf-exit`: `demand`, then `background built`, then `warm exit`, with no Reload).
  - FAIL: `cf-k1-laps` and `cf-shrink`. A cold fighter was never freed: the lifecycle ticked only while the bank had other work.
- **cf-p5-x1** (XPS, `c3253b97` diag `ddd5cc45`, 2d on): 10 PASS, 1 FAILED; no Unmapped line.
  - PASS: `cf-k1-laps`. Every lap ran `demand [45] cold=true`, then `cold release [45]`: the same fighter, no Reload.
  - PASS: `cf-pyra-keep` (the a1 crash path). A Reload out of the Pyra window kept `[[4a]]`; the pins were released after Ready with `re_requested=true`; Gunner attached warm.
  - FAIL on a rig check only: `cf-shrink`.
    - The behaviour worked: `shrink [45]`, then `frees now … freed=true`, then a first status of "Ready 6/6", then one cold fighter per lap.
    - The old check read the status line printed before the leave. It is fixed; the row reruns in the final pass.
