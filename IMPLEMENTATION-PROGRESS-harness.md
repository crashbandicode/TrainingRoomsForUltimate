# Implementation progress, harness stream (`stream/harness`)

Branch `stream/harness`, worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-harness`, from `7bb86fd`. Claude Sonnet 5.5 high
(verified from the session transcript `subagents/agent-a238856696f98fa41.jsonl`: `claude-sonnet-5-5`, effort `high`; hidden watcher in the temp
folder `harness-watch`, exits with its owner). Lead session `0caa72b8`. Playbook followed: `ae87443cbb801472e474b3f0c20b5f9c3f5f1f77`
(README, every file of docs/, tools/PROVENANCE.md, tools/eden-solo/*, tools/eden-pair/* read in full; TRAININGROOMS-STREAM-RULES.md read in full).
No Eden, cargo, Switch, card, DBI or USB was touched; no host claim was written (the only ledger access was reading it, over ssh, and one
read-only `ledger.py open/free/queue` smoke on the Surface and the Yoga). Nothing pushed.

## Objective
One parameterised Eden harness in `eden/harness/` replacing the streams' nine copies of the playbook's `gv.py`/`solo.ps1`, the route/driver copies and the
Ally lent-rig scripts; usage in `docs/HARNESS.md`; the exact playbook parameterisation in `docs/PLAYBOOK-PROPOSAL.md`.

## Done (uncommitted until the commit below)
- `eden/harness/{params,gv,ledger,route,deploy}.py`, `solo.ps1`, `step.ps1`, `harness.example.json`, `ally/{wa.py,stage.ps1,client.py}`; each header names its
  playbook origin. gv.py keeps the playbook's structure and subcommand names; additions listed in its header and in docs/HARNESS.md.
- Lead's binding rule (queue file "Interleave rule", ~12:4xZ): `ledger.parallel_turn` + `busy`/`claim`: while `imgui-nvn-sol` has a queued line for the host (a line
  that does not say it took the host or is satisfied) and the ledger shows no imgui-nvn-sol CLAIMED on that host after the last TrainingRooms RELEASED there
  (any TrainingRooms owner's), the claim is refused with the reason; not applied to imgui-nvn-sol itself. Tests: waits when queued and not served, clear after its
  CLAIMED, not queued / satisfied line / no queue file / before our first release, a compile release counts. On the live ledger and queue it answers: Yoga waits,
  Surface and Ally do not.
- `tests/eden/test_harness.py` (42 tests, all pass: `/usr/bin/python3 -I tests/eden/test_harness.py`) and `tests/eden_harness.rs` (runs it from `cargo test`; not run here,
  no cargo per the host rules). Ledger tests use lines copied from the real ledger (stamps with and without seconds, Codex/OpenCode/tangent owners, `again`, `(build only)`,
  `(not started)`); on the full real ledger (1,920 lines, 852 events) the parser reads 852 claim/release events and the open claims at the 11:59Z snapshot matched
  what the ledger and queue showed (stack-eden st-09 on the Yoga, f6-eden f6-03 on the Surface, wire compile, u4b build on the Ally).
- Host-tested: parameters (precedence, labels, host table, bad display/profile/root), `gv.py` prep on a synthetic profile (hard links, forced volume/pause keys, set-aside,
  archives, `--put/--keep-data`, receipt, fork NRO refused, source untouched) and restore (set-aside back by sha256, data removed, refusal when an original is gone), the
  console scan verdict (Unmapped below 0x1000 and in a window fail, others attributed, panic review), the mute read-back shapes, argument parsing, route plan and gating on a
  scripted fake instance (order of presses, the tile gate ignoring pre-existing log lines, a missed screen stops without pressing on, 10-press menu cap, install-line first,
  plugin-line training gate), the Ally log wait.
- Smoke on this host (Surface), read only: `gv.py params`, `gv.py label` (accepts `hx-01`, refuses `e19`), `ledger.py open|free|queue` over ssh to the Yoga's ledger.

## Not done / limits
- Not run on a real Eden: `solo.ps1` (syntax-checked with the PowerShell parser only), `gv.py launch/mute/macro/shot`, `route.py` over ssh, `deploy.py`, the whole Ally variant
  (`wa.py` ops other than the log wait, `stage.ps1`, `client.py`). The first stream run is their test; the lead decides whether one short smoke boot is wanted (it would be queued
  per the rules: own profile, muted, restored).
- `mute` asserts the `settings.get` read-back is 0 but the reply shape is not documented (CLIControl.md); `gv.muted` accepts a bare number or a `value`/`current`/`effective` field and
  fails loudly otherwise. If the first boot reports an unrecognised shape, add it there.
- CPU-coin setting on the CSS (TILE-CURSOR) was never implemented by any copy; not added. `tools/eden-pair/` not used by any stream; not wrapped.
- The copies are not deleted here; the streams adopt at their next run, the lead removes the copies after (list in docs/HARNESS.md).

## Findings
- Displays collided across copies (`:93` p1/r2fix, `:95` pkg2/pkg5, `:98` pkg6/r1fix); `preflight` now refuses a display already in use.
- Only pkg6's and f6's `open_claims` read stamps with seconds; the others (and `eden/ally/ledger.py`) never saw those RELEASED lines. The old key was the owner alone, so a release
  naming one host closed the owner's claims everywhere; keyed by (owner, host) now. Owners such as `Claude tangent <name>` / `Codex worker <name>` / `Claude claude-opus-5-5 <id>/x` need
  the id word, not the second word.
- None of the copies kept the MACROS "Receipts" (sha256 of each played macro); `gv.py macro` does.
- The old `scan` only listed lines; the playbook's rule (fail below 0x1000 or in the plugin's window) was not applied by any copy. A real `Unmapped` console line was not available on this
  host to test the regex on (none in the 15 console.logs here); it follows Eden's `Unmapped Read32 @ 0x...` form quoted in docs/EDEN.md, and the verdict reads the `[ seconds]` stamp for windows.

## Playbook checklist (tooling items; this is not a plugin, the plugin items are N/A)
Implementation / hook safety / online / release and card rows: N/A (no plugin, NRO, hook, Switch or card work). Evidence for the rest:
- Eden M, claim before Up and release after Down also after a failed Up: `solo.ps1` (claim line, `catch` runs Down which releases; `-KeepClaim` only for a relaunch inside a claim);
  ledger writes tested in `Claiming.test_claim_writes_when_free_and_refuses_when_busy`. Not run live.
- One Eden per host, MemAvailable >= 10 GiB, never profile `a` / display `:61`: `gv.preflight`, `params.check`, `prep` asserts; tests `test_never_display_61_profile_a...`, `test_the_source_profile_a_is_never_used`.
- Profile hard-linked copy, only the copy changed, written-in-place files copied: `prep` (READ_ONLY via `os.link`; settings/put files real copies); test asserts `st_nlink == 2` and the source untouched.
- Every wait gated on a log line or screenshot check: `route.py` (no bare sleep step; tests check the plan's step kinds).
- Failed precondition aborted and retried under the next label: route exits 2 on a miss and names the gate; the retry loop is the stream's (playbook `retry-batch.ps1`). Partial: not a harness feature.
- Ruleset shortest (1 stock, 1:00): N/A (not a match harness).
- Profile restored after the run: `gv.py restore`, `solo.ps1 Down -Restore`; tested on a synthetic profile.
- `console.log` scan with the Unmapped rule, `run.json`/`memory.peak` kept: `gv.py scan`, `collect`.
- Every local command under the memory/time guard: the harness's own commands are short; heavy ones (cargo) are not part of it. Tests ran under no guard (0.3 s, pure Python).
- Each run's outputs under its label: `runs/<label>`, `PREP-<label>.json`, `LAUNCH-<label>.json`, `solo-<label>.log`.
- Evidence and docs: `docs/HARNESS.md`, `docs/PLAYBOOK-PROPOSAL.md`; numbers from tests only. No operator quotes beyond the queue rule's name.
- Git: commit message from a UTF-8 file, identity `crashbandicode`; no push, tag, card write. Routing: model gate and watcher above.
- `tools/check/compliance.py` on this tree: machine row "no script mounts, unmounts or ejects the card" passes; the plugin rows fail or stay `agent` as they do for any worktree of this repo (not this stream's scope).

## New tools written (reason: the playbook does not cover them)
`ledger.py` (host-aware multi-host ledger, queue, fairness, interleave rule), `route.py` (one route with transports), `deploy.py` (copy harness + config to the Eden host),
`params.py`, `ally/*` (Windows rig; the playbook has none). `gv.py`/`solo.ps1`/`step.ps1` are the playbook's, parameterised.

## Next
1. Commit (done at the end of this turn; the hash is in the lead report). 2. Lead merges into `work/implementation` and offers it to the streams; first adopter runs one boot and reports gv `mute`/`solo.ps1`
shape problems here. 3. After adoption the lead deletes the copies listed in docs/HARNESS.md.

## Smoke boot 1: Surface, `solo.ps1` + ssh route (2026-11-02, claim 13:16Z to 13:18Z, own profile `g`, display :91)
NRO: integrator-3 diag `a3bcb55e` (48eebe1) + tile cursor `b8313ca2` + CPU pilot `e3033426`. Whole sequence ran unattended from `/tmp/harness-wait/smoke-surface.ps1` (it waits for the turn, then
deploy, Up, route, shot, collect, Down); no manual step. Ledger lines: CLAIMED 13:16Z, RELEASED 13:18Z. All passed on the first try:
- preflight ok (19.6 GiB MemAvailable, 265 GB free), claim written (turn reached only after imgui-nvn-sol and deluxe had theirs; the helper held me back correctly), `prep`: pilot/tile/diag installed,
  set-aside of the source's `libssbu_gnw_judge_log.nro` (and the old copies), old tile-cursor data archived, display and launch (Eden pid recorded in `LAUNCH-hx-01.json`).
- mute: `settings.set` -> value "0", `save`, `get` -> value "0" (the reply is a dict with `value`; `gv.muted` read it), 2 s after launch, before `game.load`.
- route over the ssh transport (`route.py hx-01 --transport ssh --install-gate plugin=training_rooms/ --training-gate diag:[CharacterRoster] observed owner=0`): install line at launch+12 s,
  main menu matched at launch+59 s (13:16:35 -> 13:17:34; A presses 3 to 7 saw dist 123 on the title/loading screens, the 8th matched at 1.8), the macro `tr-main-to-stage-select` finished
  (sha256 `ce6714b4...` kept in `macros.jsonl`), stage select dist 0.6, tile gate `settled screen=sss at=6` read only after the stage select appeared, character select 3.2, ready 5.6,
  `[CharacterRoster] observed owner=0` at launch+111 s, `ROUTE DONE` at 13:18:30 (115 s after launch with every wait gated).
- one shot through `gv.py shot` over ssh (PNG in `<root>/shots/hx-01/`), `gv.py call status` (running, loading_complete, game_fps 24.0, system_fps 22.0 on this offline Surface with the diag NRO).
- `gv.py scan hx-01`: 11,934 lines, 0 failing, 0 other Unmapped, 0 panic/abort/crash. `memory.peak` 7,037,403,136 (kept in `eden/runs/hx-01/`).
- `Down -Restore`: Eden stopped (state stopped), display stopped, scan, restore (plugin set back to the source's `libssbu_gnw_judge_log.nro`, our data folders removed), RELEASED, 0 Eden processes left.
- Link economy: the copy keeps the read-only game content hard-linked (`st_nlink == 2`); `du -sh` and `--apparent-size` of the copy are both 6.5 G because profile g's non-content part (xdg caches, saves) is copied, as in every stream's copy.
Fixes found before and during the smoke: `deploy.py` did not create the macros/nro folders on the local host and the tile cursor/pilot paths were driver-side paths (commit `9ab3392`), comma NRO lists (`solo.ps1`), the
ledger's ssh read had no retry (one dropped connection killed the helper; `7a8c43a`), Ally `ps` printed CLIXML progress noise, the interleave rule's list of parallel owners and 3-minute grace (`0c7020a`, `613997c`).
Not exercised on a live Eden: the `local` route transport (host-tested with a fake control API; the smoke used `ssh`, which on the Surface goes to itself).
