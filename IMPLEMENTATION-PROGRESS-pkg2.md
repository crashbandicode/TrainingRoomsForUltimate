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
