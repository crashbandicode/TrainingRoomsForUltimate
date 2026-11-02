# Stream u4b progress (resident effect textures for the DI / SDI chips), 2026-11-02

Brief: lead 0caa72b8, operator decision Q17. Branch `stream/u4b` (worktree `TrainingRoomsForUltimate-u4b`), merged with
`work/implementation` 27d84f4 (`d7c898c`). Doc: `docs/U4B-RESIDENT-EFFECT-TEXTURE.md`. Model/effort: claude-sonnet-5-5 high, after
the 10:17Z restart claude-opus-5-5 high (in policy, lead); one hidden watcher per thread (`/tmp/u4b-model-watch.log`), reaped.

**Playbook followed: ae87443** (`git pull --ff-only` 11:35Z: already up to date). Read in full this session at about 11:23Z:
README.md, docs/CHECKLIST.md, EDEN.md, GIFS.md, HOOK-SHARING.md, MACROS.md, TILE-CURSOR.md, WORKFLOW.md, tools/PROVENANCE.md, and the
tools run: `tools/build/scan-offsets.py`, `tools/build/nro_size.py`, `tools/check/compliance.py`. (Before the restart the stream
worked from the stream rules and the u4 doc without having read the playbook; that is a slip, recorded here.)

## Result

- RE: `ef_common.eff` is resident (effect manager entry named by the folder hash `effect/system/common` = `0x1482d0fc5a`); its BNTX
  textures are built in place by the particle system and registered with `0x35b9360`, the function the UI texture builder uses.
- Helper: `model::effect_texture` (walk, colours), `model::effect_chips` (layer pick, placement, page plan), `runtime::ui::effect_texture`,
  `runtime::ui::chips` (the DI / SDI grid page), `texture::load_resident`; 10 rows in `sites_u4b.rs`.
- Eden u4b-01, u4b-02: see the doc. u4b-03 (chips in the menu): open.

## Checklist (docs/CHECKLIST.md at ae87443). Stream scope: the u4b files; project-level rows are named as such.

| Requirement | Tag | Result | Evidence |
| --- | --- | --- | --- |
| cdylib only | M | pass | compliance.py (Cargo.toml) |
| release profile, skyline =0.6.0, toolchain 1.98.1 | M | pass | compliance.py |
| pure logic in model with host tests; runtime switch-only | M | pass | `src/model/effect_texture.rs` (9 tests), `effect_chips.rs` (5), `tests/u4b_sites.rs` (8), screens tests (2); `cargo test` 8505 passed, 0 failed (Surface, 11:20Z, e0f48e8) |
| gates: test, clippy -D warnings, no-default-features, release, diag | M | partial | Yoga 11:31-11:35Z: clippy clean at the clippy-fix head, no-default-features exit 0, release `436f4d88` and diag `32c95978` built at e0f48e8 (before two test-only clippy fixes and `is_multiple_of`); a rebuild at the final head is open |
| no heap in hooks | M | pass | u4b-02 `[EffTex]` lines: allocation count 31 before and after each build/bind/release cycle (counting allocator); u4b-03 delta 0 open |
| fixed buffers, no plugin-heap use in the page | M | pass | `chips.rs`, `effect_texture.rs`: fixed arrays, the game's allocator makes the UI texture |
| root cause fixed, no workaround | M | pass | the fixes were the entry hash (folder, not file) and the unaligned byte read: both root causes; no guard/skip added |
| version guard | M | pass | project install line (`game=13.0.5`), unchanged |
| 64-byte guard on every called function and relied instruction | M | pass | `sites_u4b.rs` 10 rows (1 Called, 9 Relied), `tests/u4b_sites.rs`; Eden `[EffTex] install guards=[Stock x10]` |
| card scan before the site is chosen | M | pass | below |
| hook sharing / chaining | C | N/A | u4b hooks nothing: one called function (`0x35b9360`) and relied windows; a Called row tolerates other plugins' hooks |
| hook after the prologue or call instead | M | pass | calls only |
| no diag code in the shipped NRO | M | pass | compliance.py: no diag markers in the release NRO; `survey`/`describe` are `cfg(feature = "diag")` |
| shipped NRO writes only its install line | M | pass | u4b adds `[Chips]` lines through `hook_stats::per_load` (once per grid open; the shipped ring admits only its tags) |
| online rows | C | N/A | the chips draw in the offline Training menu only; no simulated state is read or written |
| Eden claim written and released | M | pass | ledger lines 09:53Z/10:14Z (u4b-01), 11:00Z/11:08Z (u4b-02); compile claims 09:52Z (released by the lead as orphaned 10:34Z), 10:59/11:00Z, 11:02/11:04Z, 11:31/11:35Z |
| one Eden per host, MemAvailable >= 10 GiB, not profile a or :61 | M | pass | own profile `u4b`, display `:95`, `run.json` mem_available 22.5 GB |
| profile hard-linked copy | M | pass | `tr_eden.py prep` (hard-linked copy of profiles/b) |
| copies linked/copied correctly | M | pass | `tr_eden.py prep` |
| disk: own target per copy, df >= 40 GB | M | pass | Surface `cargo-targets/trainingrooms-u4b(-sw)`, Yoga `trainingrooms-build-u4b/target(-host)`; df 187-258 GB free |
| waits gated on a log line or screenshot | M | pass | `route_u4b.py` gates (install line, menu, SSS, tiles, CSS, training scene) |
| failed precondition aborted and retried | M | pass | u4b-01 first route aborted at the install gate, retried; game restarted in the same session |
| profile restored, host released | M | pass | `tr_eden.py down` (plugins restored, RELEASED) |
| console.log searched; run.json, memory.peak kept | M | pass | u4b-02: one boot-time `Unmapped Write64 ...b80` + execute-at-0 per boot at ~3.5 s, also in pkg4 s4-07 (no u4b code); outside the plugin's u4b activity window; owned by the stack stream (lead) |
| every local command under memory/time guard | M | partial | cargo and Eden under systemd-run/timeout; small ssh/scp/ledger commands were not |
| evidence doc | M | pass | `docs/U4B-RESIDENT-EFFECT-TEXTURE.md` (labels, NRO hashes, log lines) |
| numbers from tested scripts | C | N/A | the doc cites log lines and texture sizes, no computed numbers |
| README GIF / docs/media | M | project | project README (lead); compliance machine rows "README GIF", "docs/media GIF", "HANDOFF.md", "guard sizes [0, 64]" (pkg6 seam rows) fail project-wide, not u4b files |
| handoff current | M | project | lead's `TRAININGROOMS-HANDOFF.md`; this file for the stream |
| release / card rows | C | not authorized | no operator order |
| card untouched, nothing pushed | M | pass | none |
| model gate and watcher | M | pass | see header |
| every worker read the playbook | M | pass | no workers spawned |

### Card scan (`tools/build/scan-offsets.py`, 2026-11-02 11:37Z)

Card copy: the MTP preflight copy `/home/intpa/ssbu-cpu-pilot-install/switch-mtp-v0101-v021/pilot/after/atmosphere/contents/01006A800016E000/romfs/skyline/plugins`
(16 plugins, folder dated 2026-11-01 10:19). Offsets: 35b9360 34cb16c 3560884 35609ac 35609fc 3560a98 9a1b8 9abb4 9ac08 9ac4c (u4b),
37852a0 379e9c0 379e470 (u4 functions u4b calls). Output: `libarcropolis.nro: movz/movk:0x379e9c0@0xc590c` (ARCropolis builds the pane
setter's address: a call, which the Called row tolerates); `libelite_stages_counterpick.nro: const32:0x9abb4@0xd12b` (an unaligned
constant inside data, not code); `libtraining_modpack.nro: movz/movk:0x379e9c0@0x72d7c` (the fork, never co-installed, B1); every other
plugin: no reference. No plugin references the effect manager or particle-system windows u4b relies on.

## Scripts written in this stream

| Script | Status |
| --- | --- |
| `tools/gen_sites_u4b.py` | project convention (the u4 generator); no playbook tool generates guard rows |
| `eden/u4b/tr_eden.py`, `tr_eden.u4b.config.json` | the project's Eden harness (adapted from the playbook's `gv.py` + `solo.ps1`: claim, prep, launch, drive, down), u4 copy with u4b paths; `solo.ps1` itself is fixed to gsp-visualizer's profile and display |
| `eden/u4b/route_u4b.py`, `drive_u4b.py`, `shoot_u4b.py` | the project's routes (playbook MACROS hybrid: macro segment + screenshot gates + tile cursor); the install gate now matches `game=13.0.5` |
| scratch (not committed): `ledger.ps1` | ledger line append; replaced by `tr_eden.py`'s claim for Eden runs, kept for compile-only claims (no playbook tool) |
| scratch: `wait-*.ps1`, `open-claims.ps1` | queue/ledger waits; the playbook's `queued-run.ps1` waits for a free host before an Up; to be used for u4b-03 |
| scratch: `hotswap.ps1`, `load.ps1`, `jobs.ps1`, `uitree.ps1` | `game.stop`/`game.load` in one session and the stop dialog; `gv.py call` covers the calls, the scripts only add the dialog click |
| scratch: `gates*.ps1`, `clippy.ps1`, `yoga-build.ps1`, `surface-build.ps1`, `host-test.ps1` | the playbook's gate commands with the project's BUILD.md paths |
| scratch: `Q.java`, `q.ps1` | Ghidra headless queries (RE; the playbook covers RE through the reverse-engineer-anything skill only) |
| scratch: `efflog.ps1`, `compare-fault.ps1` | log reads |
