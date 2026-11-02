# Eden evidence, ui-screens stream (menu screens on packages 4 and 6)

Dated 2026-11-02. Stream `ui-screens` (lead 0caa72b8). Screenshots and logs: `eden/runs/<label>/` (gitignored); the shots of the
rows are listed per run.

## Current acceptance status (ui-screens successor, 2026-11-02)

The historical runs below are not acceptance evidence for the current candidate.
In particular, their `Unmapped` failures must not be dismissed in the final gate:
every retained final run requires zero `Unmapped` and zero hook allocation deltas.

Waiting archive/tick/HUD source through `cd09454` is **not yet runtime accepted**.
Builder `44177d8` built and passed 11 tests; private generation/payload/pane evidence
is in `runs/ui-waiting-build-44177d8/`. The 549 added panes preserve all 63 stock
pane records and all 183 non-layout archive payloads. This establishes artifact
construction, not rendering or native lifetime safety.

The ui-11 driver now uses `hook_stats.gate` on complete log lines and requires
archive/menu-draw coverage, every delta zero and the final allocation total zero.
`gate_layout_archives` checks the separate per-load counter, matching releases
and the 5,000 ms minimum grace; no skipped injection passes. All raw lines and
row results are retained. A failure exits the row driver nonzero.

Still required in admitted, muted, batched Eden runs:

| Rows | Required evidence |
| --- | --- |
| ui-11 roster and Solo Popo | selections and role-specific marks persist; open/close and swap remain responsive |
| Portraits | native costume/mod-aware image, absent-image text fallback, repeated pages and layout retirement |
| Mii lists | all three classes, saved/replay entries, role switch, available busts, selection and Add New/editor actions |
| Waiting host | menu/input/notices; native pause/dialog ownership; request off/on hides/restores the host; Training/waiting transitions and same-address registration refresh |
| B3 and archive lifetime | modified archive injected via vanilla base; zero per-load allocations; every release >=5 s; boot/re-enter/stage/mode/hot-swap/reset hand-outs and any buffer-pressure refusal counted |

Each final run retains source/NRO identity, full diagnostics and console log,
`run.json`, `memory.peak`, reviewed screenshots and verified profile restoration.
Waiting-room hosting must use an authorized host; this stream does not host from
the Ally.

## ui-01 .. ui-05: Ally, rig `C:\Eden\trainingrooms-pkg5` (before the restart rules; that rig had no firewall rule)
Build: diag of `a67c349` + the install-line fix (`81fcfdce…`), CPU pilot, tile cursor, nro_hook, param_hook. Route: Final Destination,
Steve vs the CPU. Rows that ran (ui-05, shots in `eden/runs/ui-05/shots/`):

| Row | Result | Evidence |
| --- | --- | --- |
| Named library opens from its Recording row; empty library | pass | `named-library-open.png`: crumb `Recording > Named Library`, the layer's footer on the two help lines |
| Save slot to library with an empty slot | pass (refusal) | `save-slot-refused.png`: help line "Record into the slot first" |
| Save slot to library after a recording (7 frames) | pass | `save-slot-panel.png`: `Recording > Name macro`, "Name and save", footer `Recording · 7f · 0.12s` |
| The confirmed name is written by the worker (diag `name evtest`) | pass | `macros/evtest.json` in the plugin folder; the panel closed back to its row (`after-name.png`) |
| Named library lists the saved macro | pass | `library-with-macro.png`: `evtest · Dark Samus` |
| Edit placement without a restore | pass (refusal) | `edit-placement.png`: "Load a macro and restore a Training state first" |
| Replay window | not run | the game froze before the row (below) |

`console.log`: 10 `Unmapped` lines at 5 s at `0x109f5f3b7x` (outside the plugin's window, above 0x1000): attributed stock emulator noise.
The freezes of ui-02/ui-04/ui-05 were first read as the firewall prompt of that rig; ui-06/ui-07 show they are the menu-close hang below.

## ui-06, ui-07: Ally, LENT rig `C:\Eden\c1162-w5-ally` (restart rules; `eden/lent/stage_ui.ps1` set-aside, verified restore)
- ui-06, diag of `97e8eb4` (`8f42d9ce…`): install line whole (8,979 characters, ends `result=installed heap_allocations=0`); Named
  library opened and closed (`rec-named-library-empty.png`, `rec-page-after-library.png`); then **B on the page (menu close) froze the
  game**: `game_fps 0`, last frame age growing, no `[Ui] menu closed` line, the diag counters stop.
- ui-07, integrator-2's own diag NRO of the integration head (`cargo-targets/integ2-nro/libtraining_rooms-diag.nro`, `8993aaed…`): open
  (B + Up) and close (B) at once: the same freeze. The hang is in the integrated tree, not in this stream's changes.
- Restore after ui-07: `mismatches: []`, kept trees unchanged, CLAIM back (`restore-verify-*.json` on the rig).
- Same signature as pkg4's s4-07 (a settings publish from a non-game thread, then the game and the plugin's threads stop).

## ui-08, ui-09: the probe builds (lent rig, restored after: `mismatches: []`, CLAIM back)
- ui-08, diag of `3569c8d`: the probes went through the ring, and the drain thread stops with the game, so nothing after the open was logged.
- ui-09, diag of `2202e23` (`9af41937…`): the probes write straight to `svcOutputDebugString` (`diagnostic_now`). In `eden/runs/ui-09/console.log`,
  after `[Ui] menu opened` (239.78 s) and B:
  `probe input push B` 245.819, `close 10/11/12/1`, `publish 1` 245.826, `2` .830, `3` .835, `4` .839, `5` .851, **then nothing**. The guest froze.
- Probe 5 sits right after `compat::menu::apply_snapshot` and probe 6 right after `features::chords::publish` = `STATE.lock()`, a
  `parking_lot::Mutex` that the game's P1 poll (`chords::on_poll`) takes every frame.
- Root cause (source): `aarch64-skyline-switch.json` has `os: switch` and no `target-family`, so parking_lot_core 0.9.12 picks
  `thread_parker/generic.rs`, whose `park()` is a pure `spin_loop()`. Under Horizon's fixed priorities, a waiter that spins on the
  holder's core keeps the holder from ever running again, and the guest livelocks. Fix: the `sync` stream (kernel address-arbitration
  locks, no hook waits).

## ui-12: failed diagnostic batch, restored 2026-11-02 16:30Z
- Rust cd09454, diag NRO935f95c656d9ad837fc3da362c489f35b40d57d617d5a21a65cc3691a90d0fed. Existing lent Ally rig; muted before load, offline Training, pilot/provider coexistence.
- Menu worker exhausted its56KiB stack. Spawn base0x10e76b5000; first unmapped write0x10e76b487c below that base; high-water57344/free0. Game continued near60fps but the menu blanked. Static direct path71680B; candidate008b564 removes nested Frame construction/copies and awaits gates.
-18167 Unmapped lines; hook total408. Single training archive load0/release5007ms is narrow evidence, not a reload/lifetime pass. Roster/Solo/portrait acceptance FAILED; unchanged pre-existing settings made two permissive row checks false-positive. Driver now selects All before deterministic ranks, requires exact Mario/DK/Samus choices, and checks both Solo role transitions.
- Raw console/diagnostics, run.json, memory.peak5232082944B, screenshots, row JSON, restore receipt, static trace and SHA manifest retained in runs/ui-12. Restore mismatches[] and kept-tree changes[], original rig CLAIM restored; ledger released. No final acceptance inferred.
## ui-13: stack/roster regression, overall failed; restored 2026-11-02 17:35Z
- Frozen8d296e2 diag3344f2377d486b6e01cf6e9091f93cd6050b9eacfb544cde6e868769c8a785ab; muted offline Ally Training with pilot/providers/tile.
- Exact PlayerMario/DK ranks, CPUMario rank, Samusonehot and independent Solo toggles passed. Screens show labels/ranks/S marks. Menu highwater32336/61440, free29104, lowfalse; policyrequired61440. Fullconsole0Unmapped. Trainingarchive load0/grace passed for one hand-out only.
- OverallFAIL: finalhooktotal1262 (row-end752), mostlyroster_preload1038; menu_draw1 remains. Portraitrequested0 despite texturehelperusable. Rootwrongguardregistry identified; correction not yet runtime tested. No finalacceptance/modportrait/reloadclaim.
- Rawlogs/run.json/rows/screens/restoremanifest/evidence.json in runs/ui-13. memory.peak5239590912 from WindowsJob PeakJobMemoryUsed in run.json. Restored5plugins/2exefs/87ultimatefiles/qt with mismatches[] and keptchanges[], originalCLAIM, freshEdenprocess0; ledgerreleased17:35:29Z.

## ui-14 — portrait guard-domain fix164dc3e
The native roster guard now admits 86 portrait keys and all86 textures bind on each exercised role/page. Mario/DK ranks, CPU Mario and Samus pick pass; screenshots retain labels and runtime portraits. Full finalconsole0Unmapped; menu high-water32336/61440 (29104free), archivezero/grace pass. OverallFAIL: finalhooktotal1632 and originalCPU Solo row absent. Separate gated-by-screenshot CPU toggle persisted[1,0] without changingPlayer[1,0]; this does not erase the batch failure. Updateddriver waits for freshCPU portrait readiness beforeY and remainsungated. Costume/modreplacement/fightermissingtexture tests remainopen. Evidence: runs/ui-14 including failedrows, followupshots/settings, finalrawlogs, run.json, memory.peak5288398848 and restoremanifest. Profile fullyrestored17:58:44Z and claimreleased17:59Z.

## ui-15 — direction graphics and fresh CPU portrait binding
Frozen Rust0c3cd63, diagnostic NRO701a526e, driverbe9ce4b. DI bound10 with64x256 texture and tint[200,244,255,255]; SDI bound10 with256x128 white texture. Reviewed screenshots show the direction graphics. All86 roster portraits bound; exact role ranks/Samus and independent Solo Popo transitions passed after waiting for fresh CPU page binding. Menu high-water32480/61440, free28960, lowfalse. Full retained console has0Unmapped. Archive zero-allocation/grace checks passed for the exercised hand-outs only.

Overall **FAIL**: row-end hook total892; final attributed2202 (input-note27, input-apply1, CPU192, live1, preload1978, named2, draw1), later aggregate2222. These failures are not waived. The current item-picture recipes were not part of this run. Raw evidence, screenshots, settings, run.json and restore receipts are in runs/ui-15; Windows Job peak5501251584B. Eden stopped and fresh process count0; five plugins, two exefs,87 files and Qt configuration restored with no mismatches or kept-tree changes. Canonical ledger release18:52:48Z supersedes the malformed initial release.

## Current cc29613 candidate — runtime pending
Item pictures and regenerated Training/waiting recipes are host/artifact gated, not runtime accepted. Host lib1179pass/0fail/2ignored, clippy all-targets with warnings denied and no-default-features build pass. Switch diagnostic and release builds pass; fullhost still has the inherited QUICK_MENU_APP ownership failure pending the reviewed typed consumer/provider composition. Four static thread-table bounds remain unresolved. Neither successful compilation nor these historical screenshots constitutes final acceptance.

The next ui-11 driver requires --menu-static-bytes from the exact staged candidate's retained stack_depth.py --table report. Preserve that report with its ELF SHA in the run manifest; never substitute an earlier candidate's number. The driver enforces the composed 56 KiB allocation and applies the existing 25% headroom plus 16 KiB reserve to the larger of static and live high-water. It refuses the historical 60 KiB allocation.

The next ui-11 driver captures the38-item picture grid and the screen after leaving it; screenshots require review and the existing full-log allocation/Unmapped gates remain binding. Mii actions, mod/costume/fallback portraits, waiting-host behavior and B3 modified-archive/reload rows remain open. Helper-path acceptance boots are held by the lead pending the separate native fault investigation.
