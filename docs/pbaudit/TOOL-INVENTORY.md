# Tool inventory against the playbook (pbaudit, 2026-11-02, playbook ae87443, tree bf836c8)

Every script under `eden/` and `tools/` in `bf836c8` that overlaps a tool or doc of `~/ssbu-mod-playbook`
(`tools/eden-solo`, `tools/eden-pair`, `tools/build`, `tools/check`, `docs/EDEN.md`, `docs/MACROS.md`,
`docs/TILE-CURSOR.md`, `docs/GIFS.md`). Coverage: **full** (the playbook tool does it as is), **partly**
(same job, but the playbook tool lacks something the plugin needs), **none**. Line counts and diffs were
measured by the audit (`git diff --no-index --numstat`).

The playbook's Eden drivers are gsp-visualizer's copies: `solo.ps1` hard-codes the owner string, the label
pattern `^e\d{2,}$`, the work root and display `:68`; `gv.py` has no `settings.json` install, no mute, no
`scan`, no `restore` and no host-aware ledger. So "replace" is only possible after those parameters
exist upstream; until then the repo keeps its adapted copies, but it keeps **nine** copies where one
parametrized file would do.

## Reuse that already happens (keep)

| Playbook tool | Used by |
| --- | --- |
| `tools/build/scan-offsets.py` | pkg2 successor (05:31Z), pkg3 (04:38Z), r1fix (04:49Z) over the 2026-11-01 10:19 card copy. Not run by pkg5, pkg6b, wire (checklist row 10 open) |
| `tools/build/nro_size.py` | pkg2/pkg5 through `eden/ally/ally_fetch.py` and `ally-build.sh`, pkg3, r1fix, wire |
| `tools/check/compliance.py` | pkg2, pkg3, pkg5, wire, integrator-3, r2fix (late); not r1fix, not pkg6b |
| tile cursor (`docs/TILE-CURSOR.md`) | every route (`lstick,<dir>` presses and the `settled screen=... at=N` log gate, e.g. `route4.py` tile 41); also `ally_route.py` |
| MACROS hybrid (`docs/MACROS.md`) | `route4.py` family: `play_segment` = `macro.play` + poll `macro.status` + `input.reset`, a reference-screenshot or log gate between segments, a cap, and a stop on a miss. Gap: the MACROS "Receipts" rule (sha256 of every played macro file kept with the run) is not implemented in `play_segment` or `tr_eden.py macro` |
| `tools/eden-solo/gv.py` + `solo.ps1` | adapted into the harness family below (the header of `tr_eden.py` says so) |

Playbook tools **no stream used**: `tools/eden-pair/*` (two-console runs: needed by the open online rows 16/17 of
pkg2, pkg5, pkg6b, wire, r1fix; `local_pair.ps1` and WORKFLOW section 5), `eden-solo/encode.py` + `record.ps1` and
`eden-solo/examples/css_check.py` / `retry-batch.ps1`, `eden-pair/examples/queued-run.ps1` (the wait-then-run
queue), `tools/release/*`, `tools/switch/*` (card: the lead's).

## 1. Eden harness: one file copied nine times

| Script | What it does | Playbook coverage | Recommendation |
| --- | --- | --- | --- |
| `eden/tr_eden.py` (package 1; identical copy `eden/u4/tr_eden.py`) | `up/down/shot/press/stick/call/tree/macro/scan/restore` for one owned profile: hard-linked profile from `profiles/b` (`os.link`), set-aside of other NROs, `settings.json` into the profile, `core.volume=0` via `settings.set/save/get` before `game.load`, claim before Up and release after Down (also after failure) with a host-aware open-claim parser, MemAvailable >= 10 GiB and `df` asserts, `console.log` scan, plugin-set restore by sha256; reads `tr_eden.config.json` for root, profile, owner, display, label pattern | **partly**: `gv.py` (prep/launch/shot/press/stick/call/tree) + `solo.ps1` (claim, MemAvailable, Up/Down) are the same skeleton; missing upstream: parametrization, settings install, mute, host-aware ledger, scan, restore | **keep one**: `tr_eden.py` + a `tr_eden.<stream>.config.json` (the mechanism exists: `tr_eden.r1fix.config.json`, `u4/tr_eden.u4.config.json`). Contribute the parametrization, mute, scan and restore to the playbook's `gv.py`/`solo.ps1`, then replace |
| `eden/s2_eden.py`, `s3_eden.py`, `s5_eden.py`, `s6_eden.py`, `tr_eden4.py`, `tr_eden_w.py`, `tr_eden_r2.py` | copies of the above with their own root, profile, owner, display and label (differences vs `tr_eden.py`, lines added/removed: s2 113/66, s3 36/57, s5 33/51, s6 116/67, tr_eden4 31/59, w 40/57, r2 68/61; w vs r2 46/22) | same as above | **replace** by `tr_eden.py` + config; the real differences (s2/s6 host-aware `open_claims`, the Ally-aware parsing) belong in the one file |
| `eden/tr_eden.r1fix.config.json`, `eden/u4/tr_eden.u4.config.json` | per-stream config for the one harness | n/a | keep (this is the right pattern) |
| `eden/ally/ledger.py` | claim/release helper for the Ally flow, host-aware | **partly**: `solo.ps1` writes the same CLAIMED/RELEASED lines for Linux solo runs only; the queue file `TRAININGROOMS-EDEN-QUEUE.md` and compile claims are campaign rules the playbook does not have | keep, but make it the single claim helper that `tr_eden.py` imports (three more helpers exist outside the repo under `/tmp/integrator3/`: `ledger.py`, `surface_free.py`, `claim_when_free.py`) |
| `eden/look.ps1` | screenshot of the Eden window on the Yoga fetched to `eden/runs/<label>/shots/` (hard-coded `trainingrooms-eden-p1`) | **partly**: `gv.py shot` does the screenshot locally; remote fetch over ssh is not in the playbook | replace by `tr_eden.py shot` + scp inside the route code (`snap()` in `route4.py` already does this); delete |

## 2. Routes and drivers

| Script | What it does | Playbook coverage | Recommendation |
| --- | --- | --- | --- |
| `eden/route4.py`, `route_local.py`, `route_r2.py`, `route_s3.py`, `route_s5.py`, `route_w.py`, `u4/route_u4.py` | title -> main menu -> Training stage select -> tile-cursor to a stage -> CSS -> Training, driven from the Surface over one multiplexed ssh; every wait gated on a log line or a reference-screenshot distance with a cap, stop on a miss. Seven copies; vs `route4.py` they differ by 3-13 lines (WORK, profile, harness name, reference list) | **partly**: it is MACROS.md's hybrid and TILE-CURSOR's `settled` gate; `eden-solo/to-css.ps1` routes to the Quickplay CSS (not Training) with fixed sleeps, `examples/css_check.py` is the marker pattern | **replace** the copies by one `route.py` with a config; keep the gating design; add the macro sha256 receipt |
| `eden/route_s2.py`, `route_s6.py` | the same route run ON the host that runs the instance (the Surface), 4 lines apart | same | merge into the one route with a `--local` flag |
| `eden/ally/ally_route.py` | the route on the Ally (Windows Eden) through the adapter | **none** (the playbook has no Windows Eden) | keep; share the gate helpers with `route.py` |
| `eden/drive_s5.py`, `eden/u4/drive_u4.py` | step-list driver over ssh (5 lines apart) | **partly**: `step.ps1` (`b:`/`s:`/`w:` steps) is local-only, `eden-pair/drive.py` is the ssh form for pairs | merge the two copies; keep as ssh form of `step.ps1` until the playbook has one |
| `eden/calm_then.py`, `eden/seq.py` | ad-hoc: `exec` the source of `route_local.py` with a string replace; hard-coded `/TrainingRoomsForUltimate-work/eden` | **none** | delete (scratch); fold "wait until calm, then run a command" into the route module if needed |
| `eden/probe.py`, `step_probe.py`, `analyze_probe.py`, `analyze_step.py` | frame-counted input probes: play a macro, pause at a frame, screenshot, find the onset on synthetic-tested analysis; `tests/eden/test_analyze.py` via `tests/eden_scripts.rs` | **none** | keep (merge `probe.py`/`step_probe.py`, 35/67 lines apart) |
| `eden/mkmacros.py`, `mkmacros_s2.py`, `eden/macros/*.edenmacro.json` | generate the plugin's own P1 macros in Eden's format | **partly**: MACROS.md describes the format and safe edits, no generator | keep; record each file's sha256 next to the run (MACROS "Receipts") |
| `eden/mk_settings.py`, `eden/settings/*` | write `settings.json` fixtures for the plugin | **none** (plugin-specific) | keep |
| `eden/hook_stats.py`, `lanes_r1.py`, `rows4.py`, `rows_r2.py`, `rows_s44.py`, `rows_s6.py`, `rows_w.py`, `s3d.py` | per-package Eden rows and gates (log lines, hook counts, allocation delta) | **none** (plugin-specific); they implement the CHECKLIST's `heap_allocations` and gated-wait rows | keep; rows scripts still contain 7-15 short fixed sleeps beside their polls (rounded up per MACROS, not each justified) |
| `eden/compare_s5.py`, `make_summary.py` | mockup-vs-screenshot side by side; one-picture package summary | **none** (GIFS.md covers GIFs, not these) | keep |
| `eden/ally/gif2png.py` | one frame of an Eden GIF as PNG for review | **full** by `python3 ~/.claude/skills/record-ui-demo/scripts/gif_tool.py inspect` (GIFS.md "Check before you use it") | replace |

## 3. Ally (Windows Eden) scripts: not covered, keep, consolidate

`eden/ally/{ally_conf,ally_ops,ally_launch,ally_log,ally_fetch,ally_rows_mash,s5_ally,s5_walk}.py`,
`ally-stage.ps1`, `ally-setup.ps1`, `ally-build.sh`, `wa_s2.py`, `wa_s5.py`: Windows Eden on the Ally (scheduled-task
launch, Job-object kill, MSI pwsh over ssh, WSL build host). The playbook's `eden-solo`/`eden-pair` drive Linux Eden on
the Yoga and Surface. Coverage **none**, except `ally-build.sh` which calls `tools/build/nro_size.py` (reuse) and
`ally_fetch.py` (reuses `nro_size`). Recommendation: keep; `wa_s2.py` and `wa_s5.py` are 9 lines apart (one adapter
with a stream parameter); `ally_conf.py` is already the one place for roots.

## 4. `tools/`: plugin-specific, none covered

| Script | What it does | Playbook overlap | Recommendation |
| --- | --- | --- | --- |
| `tools/elf_symbols.py`, `tools/elf/*.py` (`sym`, `armdis`, `hashscan`, `fields`, `guardbytes`, `nrosym`, `ptrscan`, `qwords`, `stagetab`) | symbols, 64-byte guard windows, disassembly and hash-constant scans over the owned 13.0.5 ELF | **none**: `scan-offsets.py` scans plugin NROs on the card for offsets, a different job | keep; `tools/elf/` helpers are 10-60-line scratch with no tests |
| `tools/gen_sites_pkg3.py`, `gen_sites_pkg4.py`, `gen_sites_pkg6.py`, `gen_sites_u4.py` | generate `src/model/sites_*.rs` rows with 64-byte guards from the ELF (107-207 lines each) | **none** | keep; consolidate the four near-identical generators into one with per-package tables |
| `tools/feature_matrix.py` | compile every cargo feature (decision D11) | **none** (`cargo build --no-default-features` is the playbook's only feature gate) | keep |
| `tools/carry_pkg4.py`, `carry_pkg6.py` | manifest-driven clean-room carry with a path rewrite | **none** | keep; consolidate |
| `tools/provenance_scan.py`, `feature_groups.py`, `pkg6_deps.py`, `pkg6_inventory.py` | provenance and dependency census | **none** | keep |
| `tools/layout-builder/` (Rust) | builds the `info_training` archive recipe from the player's stock archive | **none** | keep |
| `tools/render_decisions.py`, `render_menu_mockup.py`, `render_sdi_stretch.py`, `crop_menu_captures.py`, `gen_catalog.py`, `gen_stage_textures.py` | mockups and generated catalogs for docs | **none** | keep |

## Summary

- 85 scripts under `eden/` (57) and `tools/` (28) inspected. Fully covered by a playbook tool: 1 (`gif2png.py`). Partly covered:
  the harness family (9 copies), 7 route copies, 2 drivers, `look.ps1`, `ledger.py`, the macro generators. Not covered:
  the Ally set, the probes, the rows, all `tools/` generators and ELF helpers.
- The duplication is the main cost: seven harness copies differ from `tr_eden.py` by 31-116 added lines each (an eighth,
  `u4/tr_eden.py`, is identical), 7 route copies by 3-13 lines. The eight harness copies are 4,027 lines and the seven route copies about 1,100; consolidating to one
  parametrized harness and one route removes most of those 5,000 lines without losing a feature.
- Upstream candidates for the playbook (so the next plugin does not copy again): harness parametrization (owner, label
  pattern, root, display), settings install and mute, `scan` and `restore`, a host-aware ledger parser, the macro sha256
  receipt, and the reference-screenshot gate (`wait_match` / `wait_line` of `route4.py`) next to `css_check.py`.
- Playbook reuse missing in the streams: `tools/eden-pair/local_pair.ps1` for the online rows (16, 17), and
  `retry-batch.ps1`/`queued-run.ps1` for the queue-and-retry pattern the streams re-implemented with watchers.
