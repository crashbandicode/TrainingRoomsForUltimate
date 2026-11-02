# Implementation progress, package 2 (stream pkg2): CPU behaviour (2026-11-02, Claude Sonnet 5.5 high, session transcript `agent-aa28309a2c043d4af` under lead `0caa72b8`)
Playbook followed: `53fd1f2`. Host + Switch build only: no Eden, no Switch, no push. Test provenance:
`docs/test-provenance/package-2.md`; planned Eden rows: `eden/EDEN-PACKAGE-2-PLAN.md`.

**Done (host tests pass).** `src/model/cpu/`: the game's constant table by name (`hash.rs`, `ids.rs`), the plugin's own
generator and picks (`rng.rs`, `settings.rs`), the CPU role and per-body state (`role.rs`, `state.rs`; decisions B4, B6),
the frame counters (`counter.rs`), the playback rule (`playback.rs`; B5/N4), the situation predicates (`status.rs`), the
mash queue and buffer entry (`mash.rs`), SPEC-cpu-02 (`defence.rs`: DI, SDI, air-dodge direction, clatter, dodge staling) and
SPEC-cpu-03 (`ledge.rs`). 19 new site rows (`sites.rs`, features `CpuCore`, `CpuDefence`, `CpuLedge`), checked against the ELF.
Runtime (`src/runtime/features/cpu.rs`): the fighter on-frame wrapper, the role table, five hooks, the constant resolver, the
diag hook counters.

**Sites added.** vtable slot `main+0x4f80568` (on-frame, original `0x614630`); Called: `lua_bind_get_value` 0x3728e10,
status_kind 0x2087720, prev_status_kind 0x2087770, situation_kind 0x20877a0, prev_situation_kind 0x20877b0,
change_status_request 0x20876e0, motion frame 0x205cc70, pos_x 0x206ae10, is_flag 0x208dc80, set_flag 0x208dcb0,
add_clatter_time 0x20010d0, req_on_joint 0x20176f0; replace hooks on leaf thunks (16-aligned, 16-byte far jump): SDI
0x20aeda0, get_stick_x 0x2001260, get_stick_y 0x2001280, start_clatter 0x2001080, is_enable_transition_term 0x208dd60.
Guard source: the owned 13.0.5 ELF (`core01_sites_match_owned_elf`). Card scan over the 2026-11-01 10:19 MTP preflight copy
(16 plugins): only `libtraining_modpack.nro` (the fork; B1) references any of the offsets; `libssbu_cpu_pilot.nro` and the
other 14 plugins none, also not within 0x40. Pilot sites (its `src/model.rs`, commit 0162eed): none within 0x40 of ours
(`tests/cpu_sites.rs`). `libnro_hook.nro` is on the card.

**Online (CPU hooks run in online lobby practice), hook by hook.** The wrapper reads game memory and writes only for a body the
role table holds, and the table is only filled in Training for an entry that is not 0 and is operated by the CPU: in an online
match (`is_training_mode` false) every hook finds no role and returns the original's answer unchanged; no hook draws a random
number from the game (the plugin's own generator, seeded from the tick and an address); no native code is skipped or reordered
except the SDI answer for a CPU body, the stick reads of a CPU body in aerial hitstun and the ledge climb term of a CPU body on
the ledge, all of them Training-only decisions about a practice CPU, never the player's fighter; the status request, flag write
and clatter calls are made on the CPU body only. Replay A/B and two-console rows are the lead's to schedule.

**Open spec gaps, not guessed.** (1) *DI (cpu-02 §4)*: the `correctDamageVectorCommon` hook is in the `common` script module and
its 64-byte guard has to come from that image; no 13.0.5 `common` NRO is available to the implementer, so the DI hook is not
installed (the model and its tests are done: `Defence::di_correction`). Needs the NRO bytes (or the lead's decision on a different
site). The tech (cpu-04) and shield OoS (cpu-05) hooks are in the same module and have the same dependency. (2) The three common-module
hooks need `libnro_hook.nro`'s `add_nro_load_hook`; reaching it by symbol lookup (not linking it) keeps the plugin loadable
without it. (3) SDI with the Neutral direction answers "no input" (the spec says an input frame writes `(cos a, sin a)` and
Neutral has no angle). (4) cpu-03 question 1 (empty Ledge Delay row buffers a playback late) is implemented as §5 says (needs a
set row); the recommendation to treat empty as "0" is not applied. (5) Playback requests (`FrameOut::playback`) have no recorder to go
to until package 4. (6) Counters are per body (B6), 7 per body x 16 bodies = 448 bytes against the spec's ~128-byte target
for one shared table; the clock still ticks once per frame. (7) `Scene::Lobby` is set by package 6 (`cpu::set_lobby`).

**Workarounds proposed.** None.

**Gates at the stream move (2026-11-02, worktree `TrainingRoomsForUltimate-pkg2`, target `cargo-targets/trainingrooms-pkg2`).**
`cargo test` (19 test binaries, all pass), `cargo clippy --all-targets -- -D warnings` (0), `cargo build --no-default-features`,
Switch release and diag builds (0 warnings, pin intact, campaign rustup home). Release NRO sha256
`f5921e9154d2368e45076a8482497b5e38504ade4fb95ea263fab1812f4d7583` (188,416 B file, 233,472 B resident; the lead's release before
package 2: 180,224 B resident, so +53,248 B: text +24.5 KB, ro +16 KB, data +12 KB for the 34 site rows; `cos`/`sin` were
removed because they linked the whole math library, +98 KB), diag `8b39f3cc247554a645c68167db40f667c347c81c4fd39f62728d9d41b91d9cf2`
(339,968 B resident). Site rows live in `src/model/sites_pkg2.rs` (registered by `SITES = concat(BASE_SITES, SITES_PKG2)`).
`compliance.py` machine rows that fail are the repo-level ones (README GIF, HANDOFF.md, dirty tree before commit).

## Lead rulings applied (2026-11-02): the `common` script module hooks
**Extraction (local research, never committed).** `lua2cpp_common.nro` came out of `data.arc` with W7's single-file extractor:
`cd /tmp; /home/intpa/ssbu-recomp/deps/venv/bin/python3 -I <c1162>/di-sdi-extract/tools/w7/arc_one.py /home/intpa/ssbu-recomp/build-linux/work/ro_study /home/intpa/ssbu-recomp/build-linux/dump/romfs/data.arc 0x281dd03565 /home/intpa/smash-training-lobby/research/game-13.0.5/nro/lua2cpp_common.nro`
(`0x281dd03565` is the Hash40 of `prebuilt:/nro/release/lua2cpp_common.nro`, listed in `ro_study/nro/index.json`). Result: 8,466,432 B, sha256
`43876f961a80e413370629d32c23af9de6e4cc26b448bab88d5433714a566097`, byte-identical to the earlier `ro_study/nro/lua2cpp_common.nro`.
**Sites.** `src/model/sites_pkg2.rs::COMMON_SITES`: `correctDamageVectorCommon` (text+0x15df0), `L2CFighterBase::change_status` (0xa1e0),
`L2CFighterCommon::sub_guard_cont` (0x18cea0) as replace hooks at the entry, `is_enable_passive` (0xed2a0) called. `tests/cpu_common.rs` reads the module from
the research path (skip-with-message only when absent and `SKIP_OWNED_ELF_FACT` is set) and compares the 64 bytes (`cpu_common_guards_match_the_owned_module`,
`cpu02_common_guard_gate`). ABI read from the disassembly: arguments by reference (`mov x21, x1`), result in x8 (`mov x19, x8`), the script object's module accessor at +0x40.
**Runtime.** `src/runtime/features/cpu_common.rs`: `libnro_hook`'s `add_nro_load_hook` found by `LookupSymbol`; on the `common` module's load each
function is found with `LookupModuleSymbol`, its 64 bytes are compared, and the three hooks are written only when all four match (all or nothing); one
late line in `events.log` (`cpu_common_load … cpu_common=installed|not_installed`). Without the provider the install line says
`cpu_common=not_installed reason=no_nro_hook` (`FeatureReport` gained an optional `reason`); with it, `reason=awaiting_common_nro`.
The logic entry points are in `features/cpu.rs` (`di_before`, `tech_request`, `oos_after`). Not run on Eden: whether the loader's name for the module is
`common` is a guess covered by `is_common` (`common`, `lua2cpp_common`, a suffix match) and the late line prints the name it saw.
**Builds.** release `6cea4a592c5d4214e9a1da618dee32e31b68a7bb8e205ef02e618479ccc2619f` (245,760 B resident), diag
`1b5381adafc744683edbb6ae12c87e9d9934f325fcdac4846f625aa23ca0aa8d` (348,160 B). Gates: `cargo test` (20 binaries), clippy `-D warnings`, `--no-default-features`.
**Diag counters** (`cpu_hook_counts`) now also print `heap_allocations=`; r1fix's API replaces it when merged.

## Still open (rulings 2 and 3)
* **Shield damage multiplier / `libparam_hook`.** The provider exports `add_param_load_hook(callback)`; the callback's argument layout (a pair of references
  to the params info and the file hash, read from `handle_load_prc_file`) and the `CommonParams` field offset are not in any spec or owned source, and decoding
  them from the plugin's machine code was not done. Not installed; `Cpu::damage_multiplier` (tested) is ready. Needs the provider's source or a decision.
* **StageID values (fixed camera).** The route order was: (1) lua constants by name: not tried (cannot be known without a run; the constant names would be
  `STAGE_ID_*` guesses); (2) `ui_stage_db.prc` through `arc_one.py`: not done; (3) the ours-new fork file `training_mod_consts/src/lobby_stages.rs` holds the
  `stage_place_id` of each selectable stage (Battlefield 0, Small Battlefield 347, Big Battlefield 2, Final Destination 3, ...), joined to the game's form table at
  `main+0x454a7e8` (stride 72) for the Omega and Battlefield forms. The camera rule needs the Omega and Battlefield form ids of every stage, which come from that
  form table; the mapping is not built yet.

## P2-6, P2-7, P2-8 (2026-11-02)
Model: `command.rs` (SPEC-cpu-01's per-frame mash run: trigger priority, once-per-situation roll, the action table, the dash-attack
sequence, the character-specific completion table), `throw.rs` (SPEC-cpu-06: throws and pummels, fast fall, attack angle, crouch, full hop),
`advantage.rs` (SPEC-cpu-07's frame advantage). Runtime: hooks `get_command_flag_cat` (0x2001630, category 1 mash bits and category 2 throw bits),
`get_attack_air_kind` (0x2001710), `get_stick_dir` (0x2001030); crouch is in the stick-y hook and the full hop in the check_button hooks; the fall facts and
fast-fall flash in the frame wrapper; frame advantage on the player entry's frame. 9 new site rows (feature `CpuMash`, `cancel_is_enable` among them).
The playback requests (`FrameOut::playback`, Playback Mash Interrupt) and the advantage value go to `runtime::published` atomics for packages 4 and the UI.
Deviation noted: none from the specs; the 3-D distance uses the three `PostureModule::pos_*` natives.
Gates: `cargo test` (23 binaries), clippy `-D warnings`, `--no-default-features`, Switch release `9af591d832923ab90b8da6b56b4c9635a7f4f33313d96897a9b90e82e999099c`
(270,336 B resident) and diag `e1b3ae62e42e2d3d319531b283dc8a6ab6a34884f6587931ff9a22959e5fa88d` (368,640 B resident), 0 warnings. No Eden run yet.

## Successor worker (claude-sonnet-5-5 high, 2026-11-02): playback request, diag lines, the two open items by analysis
Playbook followed: `ae87443`. Gates run on the Surface worktree (target `cargo-targets/trainingrooms-pkg2`, campaign rustup home for the Switch builds).

* **Playback request (977dd7e).** `FrameOut::playback` already reached `published::CPU_PLAYBACK_REQUEST` from 850da30; the encoding is now the
  model's (`PlaybackStart::request_word` / `from_request_word`: `1 + slot` in the low byte, bit 8 = ledge) with host tests, the runtime stores that
  word and the diag build logs `cpu_event playback_request`. Package 4 takes it with
  `PlaybackStart::from_request_word(CPU_PLAYBACK_REQUEST.swap(0, Ordering::AcqRel))`.
* **Diag lines (02090b5, 9d95f54).** The diag build prints one `[TrainingRooms] cpu_event <name> ...` line per answer a behaviour gave (`role`, `status`,
  `sdi_input`/`sdi_none`, `di`, `airdodge_stick`, `clatter`/`clatter_start`, `request_status`, `force_status`, `fast_fall`, `hide`, `tech`/`tech_unchanged`,
  `motion_substitute`, `quake_suppressed`, `oos_escape`, `guard_button`, `shield_tilt`, `shield_param`, `command_flags`, `attack_air_kind`, `stick_dir`,
  `crouch`, `full_hop_button`, `climb_denied`, `playback_request`/`playback_stop`, `frame_advantage`, `common_params`, `shield_damage_mul`, `camera_mode`,
  `camera_setup`; per-frame hooks log only when their answer changes) and the resolved constant table by name (`cpu_ids`, six per line). The shipped
  build compiles all of it away (release resident size unchanged, 270,336 B). `diag-settings-next.json` (diag only): a one-second poll thread replaces
  the live settings from that file and publishes with the re-roll bits the menu would set (`menu::rerolls_between`), so the B11 / Q18 A rows run before
  the menu exists.
* **Shield damage multiplier: no provider needed (root cause).** Decoded `libparam_hook` (its `add_param_load_hook(callback)` pushes the callback onto a
  list guarded by a spin lock; `handle_load_prc_file(this, idx)` calls the original, then each callback with a pair of references to `this` and the
  file's hash40 read from a table; what the fork's `ParamsInfo::get::<CommonParams>` does with `this` is the fork's code and was not decoded): the
  game's object needs no provider. The 13.0.5 `main` fills the game's `CommonParams` object (allocated 0xc58 bytes by the factory at `main+0x736920`) in one
  70 KB function, `from_prc(this, params)` at `main+0x736a90`, which reads each field by its 40-bit hash constant and stores it into `this`: `shield_max`
  at +0x160, `shield_dec1` +0x16c, `shield_recovery1` +0x170, **`shield_damage_mul` +0x174**, `shield_damage_add` +0x178 (field names from
  `ParamLabels.csv`; found with the read-only ELF scripts in `tools/elf/`: `hashscan.py` (code that builds a hash40 constant), `armdis.py`, `sym.py`, `fields.py` (the loader's stores by field name), `stagetab.py`, `ptrscan.py`, `nrosym.py` (an NRO's exports); they need capstone, e.g. `/home/intpa/ssbu-recomp/deps/venv/bin/python3 -I`). New site row `common_params_fill` (`main+0x736ac0`, the
  `mov x19, x0`, inline hook, guard 64 bytes, card scan of the 16 plugins: no reference, also none to `0x736a90`/`0x736920`) and feature
  `cpu_shield_damage`: the hook keeps x0 (the object); once per frame on the player entry (also outside Training) the runtime writes 0.0 while Training is
  active and the shield is Infinite and the game's first-observed value otherwise (`Cpu::damage_multiplier`, already host-tested). `tests/cpu_sites.rs`
  `cpu_sites_common_params_fill_stores_shield_damage_mul_at_the_field_offset` locks the offset to the ELF (the hash constant, then `str s0, [x19, #0x174]`).
  A lost site costs only the multiplier (`cpu_shield` keeps its hooks). This is SPEC-cpu-05 question 6, alternative (a): the lead decides whether the
  `libparam_hook` dependency is dropped for good; nothing here needs it.
* **StageID and the fixed camera: resolved from the game's own table.** The game's `StageID` numbers are the indices of the stage table at
  `main+0x454a7e8` (364 records of 72 bytes): word 0 = the id, word 1 = the form (0 the stage, 1 its Omega form, 2 its Battlefield form), word 2 = the
  hash40 of the stage's name. `tests/cpu_sites.rs` `cpu_sites_stage_table_records_are_the_stages_the_camera_rule_names` proves the seven ids the spec
  names by that name hash: `battlefield` 0, `end` 3, `battlefield_s` 347, `animal_city` 257, `poke_stadium2` 107, `animal_village` 95, `trail_castle` 361
  (they equal the `stage_place_id` of the carried `lobby_stages.rs`, where its `native_forms` rows are exactly the table's form records), so no list of Omega
  and Battlefield forms is written down: `tech::stage_class(id, form)` takes the form from the record. `sv_information::stage_id()` (`main+0x227ebb0`,
  symbol checked, a called function) gives the current id. Needle sites checked on the ELF: `change_active_camera` (`main+0x4ee480`: x0 the manager,
  w1 the mode; inline hook after its prologue at `+0x18`), the fixed-camera setup (`main+0x3158e00`, a 64-byte leaf function: it copies the values struct,
  centre first, to `manager + 0xbd0`, `+0xbe0`, `+0xbf0`; replace hook at its 16-aligned entry). **Spec correction:** the setup function loads the manager through
  the slot at `main+0x52b8f00` (`adrp x8, 0x52b8000; ldr x8, [x8, #0xf00]; ldr x8, [x8]`), not `0x52b6f00` as the spec has it (open question 8);
  `tests/cpu_sites.rs` `cpu_sites_fixed_camera_setup_loads_the_manager_through_the_slot_and_stores_the_centre_at_the_field` decodes it. Hooks: `cpu_camera_mode_hook`
  and `cpu_camera_setup_hook` (the latter writes after the original, so the caller's struct is never written); feature `cpu_camera` (own sites, the
  core's natives). Model: `Cpu::camera_setup` / `camera_mode`, `StageClass`, host tests in `tests/cpu04.rs`.
  Not decided by this analysis: camera mode 4 = fixed (spec, upstream) is confirmed only by the Eden diag line `camera_mode mode=`.

## Eden on the Ally, 2026-11-02 (successor worker)
Four boots (s2-01 release, s2-02 to s2-04 diag) on the Ally's owned rig: see `eden/EDEN-PACKAGE-2.md` (rows, hashes, lines) and `eden/ALLY-SETUP.md`
(the rig for every stream). Found and fixed: the install line was cut at 2048 bytes (`2d1df59`). Confirmed: the `common` module's loader name is `common`;
DI, SDI, air-dodge direction, dodge staling, the shield multiplier (1.19 -> 0.0 -> 1.19 through the game's own `CommonParams` object), the ledge Roll row, the
mash table (16 of 18 actions) and the fixed-camera setup write. Not run: tech, mistech, hide tech effects, clatter, out of shield, throws, B11 on a ledge, camera
mode 4, leaving Training. Open items for the lead: none parked; the spec's camera manager slot is `0x52b8f00`; whether `libparam_hook` stays a dependency
(this package no longer needs it).

## Playbook record (retroactive audit, 2026-11-02, pbaudit, playbook ae87443)

Stream `pkg2` (package 2, CPU behaviour): implementer `agent-aa28309a2c043d4af.jsonl` (records playbook `53fd1f2`) and successor `agent-a356e349f6b85bdde.jsonl` (records `ae87443`), both claude-sonnet-5-5 high. The progress file has "Playbook followed" lines but no checklist. Code in scope: `src/model/cpu/`, `src/model/sites_pkg2.rs`, `src/runtime/features/{cpu,cpu_common}.rs`, `tests/cpu*.rs`, `eden/{route_s2,s2_eden,mkmacros_s2,mk_settings}.py`, `eden/ally/*` (pkg2 and pkg5 wrote them), `eden/EDEN-PACKAGE-2.md`.

### Reading evidence (transcript audit)

**Read: yes, both workers, all nine files each.** Implementer: its brief was the only one that ordered it explicitly ("Read the playbook completely (mandatory): /home/intpa/ssbu-mod-playbook ... record `git log -1`"). Transcript (UTC, 2026-11-02): 03:18:32 `MY_MODEL.md`, 03:18:53 hidden watcher `p2-watch`; 03:19:07 `git log -1` printed `53fd1f2` (the `53fd1f2` file sizes: PROVENANCE 8,380 B), 03:19:09-03:19:12 `Read` of `README.md`, `docs/CHECKLIST.md`, `docs/WORKFLOW.md`, `docs/HOOK-SHARING.md`, `docs/EDEN.md`, `docs/MACROS.md`, `docs/GIFS.md`, `docs/TILE-CURSOR.md`, `tools/PROVENANCE.md`, then work from 03:22; `compliance.py` run at 04:18:13Z. Successor (Eden on the Ally): 05:09:30 `MY_MODEL.md`; 05:09:39 `git log` printed `ae87443`; 05:09:42-05:09:46 `Read` of all nine files; then `tools/eden-pair/solo_surface.ps1`, `eden-solo/solo.ps1`, `gv.py` excerpts, `eden-pair/prep.py` (05:09:48-05:10:28); `scan-offsets.py` 05:31:25; `compliance.py` 06:48:32. No successor watcher is visible in the transcript.

### Checklist (`docs/CHECKLIST.md`, M and C items applied to this stream's code as merged in `bf836c8`)

Result vocabulary: `pass`, `fail`, `N/A: reason`, `not authorized`, and `open` (no evidence yet: not a pass). Tree-level rows cite the audit's compliance run: `python3 -I ~/ssbu-mod-playbook/tools/check/compliance.py <pbaudit worktree at bf836c8> --nro cargo-targets/integ3-nro/libtraining_rooms-release.nro` (report: `docs/pbaudit/compliance-bf836c8.md`). The audit did not rerun gates or Eden; evidence quoted from the stream's own files is marked as such, evidence marked "audit" was checked by the audit itself.

| # | Requirement | Tag | Result | Evidence |
| --- | --- | --- | --- | --- |
| 1 | Implementation: `crate-type = ["cdylib"]` only | M | pass | tree-level: `docs/pbaudit/compliance-bf836c8.md` row 1 (`Cargo.toml crate-type = ['cdylib']`) |
| 2 | Implementation: release profile `opt-level "s"`, lto, cgu 1, panic abort; `skyline = "=0.6.0"`; toolchain 1.98.1 | M | pass | tree-level: `docs/pbaudit/compliance-bf836c8.md` rows 2, 3, 5 (profile as required, `skyline = "=0.6.0"`, toolchain 1.98.1) |
| 3 | Implementation: pure logic in `model` with host tests; `runtime` only under `cfg(target_os = "switch")` | M | pass | `docs/pbaudit/compliance-bf836c8.md`; pkg2 host tests `tests/cpu01.rs`..`cpu07.rs`, `cpu_common.rs`, `cpu_core.rs`, `cpu_sites.rs` |
| 4 | Implementation: gates, one cargo call each (test, clippy -D warnings, no-default-features, skyline release, diag build) | M | pass | `IMPLEMENTATION-PROGRESS-pkg2.md` (gates at each tranche: `cargo test` 19-23 binaries, clippy `-D warnings` 0, `--no-default-features`, Switch release + diag with 0 warnings) and `eden/EDEN-PACKAGE-2.md` (Ally WSL gates for every Eden build: tests 0 failed, clippy, no-default-features, release and diag, 0 warnings; final release `7f7d6267...` 270,336 B resident). Integrated tree at `bf836c8`: integrator-3 |
| 5 | Implementation: no heap in hooks (counting allocator, `heap_allocations=0` in the install line) | M | pass | `eden/EDEN-PACKAGE-2.md`: install line `... result=installed heap_allocations=0` (s2-02, s2-04) and `cpu_hook_counts frames=600 ... heap_allocations=0` (hook budget row) |
| 6 | Implementation: fixed buffers, writer thread on a static stack, stack-built paths | M | N/A: no shipped-build output thread | package 2 adds no writer thread; its diag lines go through the carried diag ring; the install line buffer was a fixed 2,048 B that cut the line, enlarged to 4,096 B with a host test that renders the line of every site (`2d1df59`) |
| 7 | Implementation: root cause, or each workaround (guard, skip, restriction, tolerance, lint `allow`, feature gate, setting off) approved and listed with its date | M | fail | `IMPLEMENTATION-PROGRESS-pkg2.md` says "Workarounds proposed. None", yet seven `#[allow(clippy::too_many_arguments)]` ship in the stream's code (`src/model/cpu/shield.rs:163`, `tech.rs:263`, `throw.rs:48` and `:118`, `ledge.rs:78` and `:107`; commits `fa8b77d`, `850da30`), none approved or listed (audit: `git blame`). Root fixes shown elsewhere: the `libparam_hook` dependency dropped by decoding the game's own `CommonParams` fill (`common_params_fill`), the install-line cut, the spec's camera slot `0x52b8f00` |
| 8 | Hook safety: version guard (`result=unsupported_version`) | M | pass | tree-level: `docs/pbaudit/compliance-bf836c8.md` row "13.0.5 in model" (`src/model/guard.rs`, `install_line.rs`, `archive_recipe.rs`); `tests/core01.rs:70` asserts `result=unsupported_version` |
| 9 | Hook safety: 64-byte guard on every hooked, called and relied-on site | M | fail | tree-level checker limit; pkg2 evidence: `tests/cpu_sites.rs` (stage-table records, camera setup, `common_params_fill` field offset locked to the ELF), `tests/cpu_common.rs` compares the 64 bytes against the owned `lua2cpp_common.nro`. `tests/core01.rs core01_sites_match_owned_elf`, `tests/input_sites.rs`, `tests/pkg6_sites.rs every_row_matches_the_owned_elf`, `tests/cpu_common.rs cpu_common_guards_match_the_owned_module` (all pass in integrator-3's run: 7,826 passed, 0 failed) |
| 10 | Hook safety: card scan (`scan-offsets.py`) of the site over a dated card copy | M | pass | progress: card scan over the 2026-11-01 10:19 MTP preflight copy (16 plugins): only `libtraining_modpack.nro` (the fork) references any offset; pilot sites none within 0x40 (`tests/cpu_sites.rs`). Successor: `scan-offsets.py` run 05:31:25Z (audit, transcript) for `common_params_fill` (`main+0x736ac0`, also `0x736a90`/`0x736920`): no reference |
| 11 | Hook safety: sharing / chaining when another plugin may hook the site (both stand-in modes, both load orders in Eden) | C | N/A: scan shows only the fork | stronger evidence than required: pilot coexistence in both load orders in Eden (s2-01..s2-03 pilot first, s2-04 ours as `0libtraining_rooms.nro`), both `result=installed`; the `foreign=` stand-in modes were not run |
| 12 | Hook safety: hook after the prologue, or call the function instead | M | fail | the progress file lists replace hooks on leaf thunks (justifiable: no prologue) and the three `common` module functions `correctDamageVectorCommon`, `change_status`, `sub_guard_cont` "as replace hooks at the entry" (`cpu_common.rs`); the entry hooks of those large functions have no recorded operator-approved deviation. Inline-after-prologue is used for `change_active_camera +0x18` and `common_params_fill` |
| 13 | Hook safety: no diag code or strings in the shipped NRO | M | pass | tree-level: `docs/pbaudit/compliance-bf836c8.md` "libtraining_rooms-release.nro: no diag markers" (sha256 bbe6ffceb32ff61e, `cargo-targets/integ3-nro/libtraining_rooms-release.nro`) |
| 14 | Hook safety: the shipped NRO writes its install line and nothing per frame | M | pass | release resident size unchanged by the diag code (270,336 B), "no diag strings: `compliance.py`" (`eden/EDEN-PACKAGE-2.md`); the shipped build adds one `cpu_common_load` line to `events.log` per boot, not per frame |
| 15 | Online: read-only rules reviewed hook by hook | C | pass | `IMPLEMENTATION-PROGRESS-pkg2.md` "Online ... hook by hook" (lines 23-29): role table only filled in Training for a CPU-operated entry, no random number drawn from the game, no native skipped except Training-only CPU decisions. The audit did not re-verify each hook |
| 16 | Online: replay A/B with the desync detector identical | C | open | progress: "Replay A/B and two-console rows are the lead's to schedule"; not run |
| 17 | Online: two consoles past the changed path, `desync_seen=0` at the same `frame=` | C | open | not run (`tools/eden-pair/local_pair.ps1` is the playbook's path) |
| 18 | Eden: claim written before Up and released after Down, also after a failed run | M | pass | Yoga ledger `/tmp/deluxe-surface-1007/OWNER-COORDINATION.md` (copied read-only by the audit): `0caa72b8/pkg2-eden` 1 CLAIMED (06:02Z) / 1 RELEASED (06:48Z), on the Ally, covering s2-01..s2-04 ("Eden processes left: 0") |
| 19 | Eden: one Eden per host, MemAvailable >= 10 GiB, never profile `a`, display `:61` or another agent's runs | M | pass | owned Windows rig `C:\Eden\trainingrooms-pkg2` (one Eden; Job limit 8 GiB, peaks 5.2-5.5 GB in `run.json`); the MemAvailable >= 10 GiB test is a Linux-host check and is not recorded for the Ally |
| 20 | Eden: profile is a copy with hard-linked game content; identical files linked, written files copied | M | pass | `eden/ALLY-SETUP.md` / `eden/EDEN-PACKAGE-2.md`: own profile copy with hard-linked game content, muted `volume=0` persisted; never profile `a` |
| 21 | Disk: one `CARGO_TARGET_DIR` per source copy, cargo home shared, `df` >= 40 GB, finished products deleted with a manifest | M | open | own target `cargo-targets/trainingrooms-pkg2` (still present at audit); Ally WSL builds elsewhere; no `df` check or deletion manifest recorded |
| 22 | Eden: every wait gated on a log line or screenshot check | M | pass | `eden/ally/ally_route.py` gates every wait on a log line or screen (7 gate references, 1 sleep); `eden/EDEN-PACKAGE-2.md` rows |
| 23 | Eden: a run whose precondition line never comes is aborted and retried under the next label | M | pass | s2-01 (release) found the install line cut at 2,048 B ("fail, then fixed", `2d1df59`) and the batch continued as s2-02..s2-04 with the fixed build |
| 24 | Eden: shortest ruleset (1 stock, 1:00, items/hazards/FS meter off) | M | N/A: Training mode | Training on Final Destination, no match ruleset |
| 25 | Eden: profile restored after the run (set-aside NROs back, temp files removed, host released) | M | open | ledger: "profile retained, nothing deleted"; restore of the Ally rig with `stage_a.ps1` is not recorded for these runs |
| 26 | Eden: `console.log` searched for panic/abort/crash/Unmapped (fail below 0x1000 or in the plugin's window); `run.json`, `memory.peak` kept | M | pass | audit: retained `eden/runs/s2-01..s2-04/console.log` (pkg2 worktree): 0 matching lines in all four; `run.json` kept (peak 5.54 / 5.21 GB; `reason: crashed` exit 137 is the rig's Job kill); no separate `memory.peak` file on the Ally |
| 27 | Every local command under the memory and time guard | M | open | audit heuristic: 44 of 52 cargo invocations of the implementer went through a guard wrapper; the others were `cargo fetch` in `/tmp/p2-smash-fetch` (03:22-03:23Z) without a visible guard; the successor's Ally WSL builds run inside `ally-build.sh` (limits not audited) |
| 28 | Each run's outputs kept under its label | M | pass | `eden/runs/s2-01..s2-04/` (`console.log`, `run.json`, `ops.jsonl`, plugin folders) |
| 29 | Docs: README (paragraph, full-screen GIF, install path, build commands, license, AI disclaimer) | M | fail | tree-level: `docs/pbaudit/compliance-bf836c8.md` rows "README has a GIF, build commands and a license line" and "a GIF under docs/media" fail (the AI-disclaimer row passes); assigned to package 7 by the lead (integrator-3 record), not this stream's |
| 30 | Docs: a dated evidence doc (run labels, build, what was done, what the log showed) | M | pass | `eden/EDEN-PACKAGE-2.md`: dated, labels, NRO sha256 and resident sizes, per-row result and the lines behind it, rows not run |
| 31 | Docs: numbers cited from logs come from a committed, host-tested script | C | N/A: no log-derived statistics | the doc quotes lines and counts (33 `cpu_ids` lines, hook counts); resident sizes come from `nro_size.py` through `eden/ally/ally_fetch.py` |
| 32 | Docs: no operator quotes, no claims beyond the tests | M | pass | rows not run are listed (tech, mistech, clatter, OoS, throws, B11 on a ledge, camera mode 4); spec corrections are called out as findings |
| 33 | Docs: handoff file current (branch, commit, evidence, live state, next commands) | M | fail | tree-level: `docs/pbaudit/compliance-bf836c8.md` `HANDOFF.md` missing; the stream state is in this file and the campaign handoff is `TRAININGROOMS-HANDOFF.md`, neither is the plugin repo's `HANDOFF.md`; package 7 per integrator-3 |
| 34 | Release: minor bump, annotated tag, Forgejo release, assets verified | C | not authorized | no operator order |
| 35 | Release: rebuild of the tag byte-identical to the released NRO | C | not authorized | no operator order |
| 36 | Card: DBI MTP preflight, rollback copy, readbacks, receipts | C | not authorized | no operator order |
| 37 | Card: the user connects and disconnects; the agent never mounts, unmounts, ejects or remounts, nor starts or stops DBI | M | pass | `docs/pbaudit/compliance-bf836c8.md` row "no script mounts, unmounts or ejects the card" passes; transcript shows no `gio`/`udisksctl`/DBI/`tools/switch` command; ledger lines say "No physical Switch writes" |
| 38 | Without the order: nothing pushed, tagged or written to the card | M | pass | transcript shows no `git push` or `git tag` and no card write (`rg` over the stream's PowerShell commands) |
| 39 | Git: Forgejo, never GitHub; commit messages from a UTF-8 file | M | pass | remote is Forgejo (compliance pass); transcript heuristic implementer 6 / successor 7 `git commit -F`, 0 `-m` |
| 40 | Routing: model/effort gate and per-thread watcher | M | fail | implementer: `MY_MODEL.md` read and hidden watcher `p2-watch` created (03:18:32 / 03:18:53Z). Successor: `MY_MODEL.md` read at 05:09:30Z, no watcher in its transcript |
| 41 | Routing: every spawned worker read this playbook itself | M | N/A | the stream spawned no worker (no Agent call in its transcript) |

Counts: 22 pass, 6 fail (#7, #9, #12, #29, #33, #40), 5 open (#16, #17, #21, #25, #27), 5 N/A, 3 not authorized.

Finding for the lead: pkg2's checklist-relevant gaps are rows 7 (lint allows), 12 (entry hooks of the `common` module functions) and the stand-in half of row 11; its Eden evidence is the most complete of the audited streams.
