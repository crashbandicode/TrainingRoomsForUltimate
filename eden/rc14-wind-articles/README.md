# rc14-wind-articles Eden kit (2026-11-15)

- **The boot:** one instance (Yoga or XPS), TR diag of `rc14/wind-articles` with the diag-only `[Legend] object` probe lines,
  Training, muted, the GDB stub on before `game.load`. Launch only after the primary's grant.
- **`rows.json`:** integration-3's eggs plan (`artifacts/phase23/rc14-int3/eden/plans/eggs/rows`, read-only: the boot rows, the
  hb-legend rig's `hblegend.py`/`hblfinal.py`, `hbl-seg/` and `z-boot`) with this lane's rows. `build_plan.py` rebuilds it.
- **`windart.py`:** Windboxes alone, Eden speed 25 % stills.
  - `w-krool`: Blunderbuss held and Crownerang. Gate: the stills and a drawn-pill line.
  - `w-articles`: Samus's bomb, Ness's PK Fire and Palutena's Explosive Flame. Gate: an article's `areas=<n>` line, n > 0.
  - `w-items`: Sheik's Burst Grenade and R.O.B.'s Gyro (both items). Gate: the item was tracked; its slot and area lines are
    the finding.
- **Deferred:** these need a Smash Ball, a spell from the command menu or an item spawn route, and the rig has none:
  - Zelda's Triforce of Wisdom and Hero's Kaboom. They use the same article path as `w-articles`.
  - The Gust Bellows and a Poké Ball.
- **Staging:** `stage-windart.ps1` (from `rc14-hide-cpu-tag/stage-hidetag.ps1`) renders `up-windart.sh` (from `up-hidetag.sh`,
  owner `Claude wind-articles`, runtime 5400 s).
- **Runner:** the playbook's `tools/eden-runner` at `13b2e83` (`~/pb-terra-13b2e83` on the hosts). Offline dry-run READY
  (8 rows, 37 budget minutes, `artifacts/rc14-wind-audit/stage/dry-out`).
