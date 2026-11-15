# rc14-hide-cpu-tag Eden kit (2026-11-15)

- **The boot:** `hidetag-2` on the Yoga, TR diag `9df57c69` = `6d084604`. 7 of 7 rows PASS (`RESULTS-hidetag-2.md`).
- **`rows.json`:** integration-3's eggs plan (`artifacts/phase23/rc14-int3/eden/plans/eggs/rows`, read-only) with this
  lane's rows.
  - `build_plan.py` rebuilds it.
  - The plan's other hooks (`hidecpu.py`, `eggshide.py`, `hotslots.py`, `gna.py`, `steps.py`) are in that folder.
- **`hidetag.py`:** gates `ht-tag` and `ht-eggs` on the diag lines:
  - the tag off and on, and the panel off and on, for entry 1;
  - no Player 1 panel write;
  - no panel flash while hidden.
- **Staging:** `stage-hidetag.ps1` stages and launches the boot (Yoga or XPS). It renders `up-hidetag.sh`, a copy of
  integration-3's `up-final.sh` with this lane's owner and a base folder per label.
- **Runner:** the playbook's `tools/eden-runner` at `13b2e83` (`~/pb-terra-13b2e83` on the hosts).
