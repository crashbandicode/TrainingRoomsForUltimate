# windart-y1 (2026-11-15, Yoga, Up 05:43:43Z, Down 05:54:18Z)

- **Build:** TR diag `3b3b49d9a9b032d7…` = `rc14/wind-articles` `e1cd4e20`. Eden `2f315fc4b7d9…`. Muted, GDB stub on, one instance.
- **Rows:** 9 PASS, 0 FAILED (`w-krool-v` added mid-boot, rows and hooks only). Receipts and stills are in
  `artifacts/rc14-wind-audit/runs/windart-y1/`.
  - **`w-krool`:** Crownerang's arm windbox drawn deep blue (`…crownerang-t0188.png`). The vacuum drew 1 pill for about 6 frames
    (status 482) until Zelda was pulled in and caught, between the 25 % stills.
  - **`w-krool-v`** (10 %): the vacuum windbox, a deep blue capsule in front of the gun, in `…vacuum-10-t0960.png`.
  - **`w-articles`:** each article's own wind area read and drawn. Probe lines `areas=1`:
    - Samus's bomb: circle r 16;
    - Ness's PK Fire pillar: circle r 20, drawn in `…ness-pkfire-t0340.png`;
    - Palutena's Explosive Flame: circle r 29.
  - **`w-items`:**
    - Sheik's Burst Grenade (item `0x40000000`): pull slots `nr=1` with kb 100/20, /25, /30, /40 and 0 damage (windboxes, drawn
      deep blue in `…sheik-grenade-t0571.png`), then the 13 % hit `nr=0` and a wind area of r 40.
    - R.O.B.'s Gyro (item `0x40000001`): slots `nr=0` only (4 %, 5 %, then 2 % kb 50/0/3 spinning on the ground). No windbox, no area.
  - **`z-boot`:** PASS.
- **Console:** panic 0, abort 0, crash 0, Unmapped 0 (4,597 lines); `trace.unmapped_access` total 0.
- **Down:** `cleanup_failures=[]`, `released=True`; prune freed 2.6 GB.
- **Deferred** (no route in the rig): Zelda's Triforce of Wisdom, Hero's Kaboom, the Gust Bellows and a Poké Ball.
