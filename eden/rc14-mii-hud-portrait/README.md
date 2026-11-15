# rc14-mii-hud-portrait Eden kit (2026-11-15)

- **Boots:**
  - `mhp-repro-1`, XPS, TR diag `60a1b69b` = int3 `4f505d3b` (before the fix): the empty bust exactly when Hide CPU is
    On during the render, CPU and Player 1, warm and Reload (`RESULTS-mhp-repro-1.md`; four runner passes in the one
    boot, the hook fixed between them; the per-pass receipts are in `artifacts/phase23/mii-hud-portrait/runs/`).
  - `mhp-proof-1`, Yoga, TR diag `3e0a7fe8` = `89db9543` (the fix): 11 of 11 rows PASS (`RESULTS-mhp-proof-1.md`).
- **`rows.json`:** the proof plan, integration-3's eggs plan (`artifacts/phase23/rc14-int3/eden/plans/eggs/rows`,
  read-only) with this lane's rows; `build_plan.py <out> repro|proof` rebuilds it.
- **`mhp.py`:** each case swaps to Mii Gunner, waits for the render's `after render` line, takes one HUD still after Hide
  CPU Off and scores the portrait square (`busy`: an empty bust 0.03 grey / 0.12 red, a fighter 0.47 and up); `proof`
  fails an empty bust or a `cpu_event role|hide entry=2..7` line inside the render window.
- **Staging:** `stage-mhp.ps1` (Yoga or XPS) renders `up-mhp.sh`. Runner: the playbook's `tools/eden-runner` at
  `13b2e83` (`~/pb-terra-13b2e83` on the hosts).