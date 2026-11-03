# Eden rows planned, package 2 (CPU behaviour), batched per boot

## Current RC admission and evidence requirements — 2026-11-03

The boot table below is the original package plan. Its missing-package and boot-only configuration limitations are historical: the integrated menu, recorder and state features now exist, but every final RC row still needs exact-candidate runtime evidence. Reuse the existing 359-row map and each original spec criterion; do not count historical small-plugin runs as final coverage.

For every current lane, **any Unmapped anywhere in the complete console fails**, regardless of address or activity window. Preserve raw failures, hook allocation samples, measured thread maxima, source/NRO/plugin identities and complete profile/harness restoration plus separate verification. Installation alone does not prove an exercised callback. A missing provider is unexercised coverage, not a successful no-op.

Before DI, tech or out-of-shield rows, require the actual current-boot cpu_common_load line: all four owned common-module sites guard_ok and cpu_common=installed. The initial awaiting_common_nro registration reason is not installation. no_nro_hook, missing or guard_refused cannot qualify these rows. Source boundary is runtime/features/cpu_common.rs: register resolves add_nro_load_hook; the registered module callback validates all four64-byte guards before writing the three hooks. Use a previously qualified plugin set with the required provider; retain its full hashes and load order. The minimal three-plugin F9/Hitbox sets that explicitly lack this provider cannot substitute for this positive CPU lane.

Use the integrated menu to batch compatible changes in one admitted boot, retaining actual before/after settings and actions. Preserve the original SDI/reset/B11 decision semantics; menu availability does not reduce repeats, side changes, multi-hit cases or actual tether/dual-body requirements. Pilot coexistence requires positive single-driver detection in both load orders; a guarded ai_think native call is not itself a hook collision. Keep normal-shield multiplier restoration on leaving Training and role retirement at scene changes as actual transition rows.

All lanes remain unrun on the final candidate. A development host or native gate does not close them. Host ownership, queue fairness, mute-before-load, resource caps and full restoration remain mandatory.

2026-11-02, package-2 implementer. Nothing here has been run. Rows are the specs' own labels (`SPEC-cpu-0N` §9);
they are grouped by the `settings.json` each boot needs, because settings are read at boot until the menu
(package 5) exists. One window, 1 stock, 1:00, items and hazards off, Eden muted (`core.volume=0`), claim written
before Up and released after Down, MemAvailable at least 10 GiB, `console.log` searched for `panic`, `abort`,
`crash` and `Unmapped` (an `Unmapped` below `0x1000` or inside the activity window fails the lane), per the
playbook. Builds: the release NRO for every row that only reads the install line and the diag build for rows that
read per-event lines. Every boot ends with `heap_allocations=0` in the install line.

Common to every boot: Training, P1 and the CPU on Final Destination, the CPU a fighter that has the move the row
needs. Read `events.log` (install line: `cpu_core=installed cpu_defence=installed cpu_ledge=installed`, each CPU
site `stock/far`, `cpu constants missing=0` absent) and the diag lines of the row.

| Boot | `settings.json` rows (others factory) | Eden rows covered | Needs |
| --- | --- | --- | --- |
| P2-1 `p2-factory` | none | `cpu-02-install-line`, `cpu-03-install-line` (the `cpu-0N-install-line` rows of every spec), `cpu-02-sdi-unset-none` (hit the CPU with SDI unset: no SDI inputs), pilot coexistence both load orders and the fork's NRO absent | release NRO |
| P2-2 `p2-defence` | `sdi_state` Up, `sdi_strength` High, `air_dodge_dir` Down Out, `clatter_strength` High, `stale_dodges` Off | `cpu-02-sdi-high-up` (one input every 4 hitlag frames, vector up), `cpu-02-airdodge-down-out`, `cpu-02-airdodge-out-left-and-right` (player on each side), `cpu-02-clatter-grab-high` (a grab: time reduced every 2 frames, `sys_clatter` shown), `cpu-02-clatter-freeze` (freeze: step 4), `cpu-02-dodge-staling-off` (repeated air dodges do not stale) | diag build for the per-event lines |
| P2-3 `p2-sdi-two` | `sdi_state` Up and Down, `sdi_strength` High | `cpu-02-sdi-two-directions` (one direction between resets, a new roll after L+R+A), `cpu-02-ice-climbers-sdi` (Popo and Nana count apart) | diag build |
| P2-4 `p2-ledge-all` | `ledge_state` Neutral, Roll, Jump, Getup Attack | `cpu-03-each-getup` (20 grabs: every option seen), `cpu-03-regrab-rerolls`, `cpu-03-zero-delay-first-frame`, `cpu-03-tether-first-frame` (a tether recovery fighter), `cpu-03-ice-climbers` | diag build |
| P2-5 `p2-ledge-wait` | `ledge_state` Wait | `cpu-03-wait` (hangs until the game drops it, climb denied) | diag build |
| P2-6 `p2-ledge-delay` | `ledge_state` Roll, `ledge_delay` 60 | `cpu-03-delay-60` (roll 60 frames after the ledge is actionable) | diag build |
| P2-7 `p2-ledge-range` | `ledge_state` Roll, `ledge_delay_range` 30 to 60 | `cpu-03-delay-range-30-60` (20 grabs: delays 30, 40, 50, 60 only) | diag build |
| P2-8 `p2-ledge-mash` | `ledge_state` Roll, `ledge_roll_override` Forward Smash | `cpu-03-roll-then-fsmash` (needs the mash triggers of SPEC-cpu-01, P2-7 of this package) | diag build; blocked until the per-frame mash is built |
| P2-9 `p2-ledge-playback` | `ledge_state` Playback Slot 1, `ledge_delay` 0, a recording in slot 1 | `cpu-03-playback-slot1`, `cpu-03-savestate-on-ledge` | blocked on packages 3 and 4 (recorder, save states) |

Rows that need the menu (package 5) and are not in a boot above: `cpu-02-sdi-change-no-reset` (SDI Up, then In,
no reset), `cpu-03-wait-then-change-to-roll`, `cpu-03-change-to-empty`; they are the B11 and Q18 A rows and run once
the menu exists (or in a diag build that republishes settings from `diag.txt`, which this package does not add).
Rows blocked on the missing `common` guard: `cpu-02-di-out-both-sides`, `cpu-02-di-multihit`, `cpu-02-di-neutral`,
`cpu-02-di-during-playback-off/on`, and the DI half of `cpu-02-install-line`.

Pass criteria are the spec's §9 rows. For every row also record: the row label, the boot label, the NRO sha256, the
`events.log` lines read, and the screenshot or GIF where the row shows an animation.
