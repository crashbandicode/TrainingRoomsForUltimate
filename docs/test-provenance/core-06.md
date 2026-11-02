# Test provenance — SPEC-core-06, the hitbox visualizer

Stream `hitbox` (branch `stream/hitbox`), 2026-11-02. Spec: `specs/SPEC-core-06-hitbox-visualizer.md`. Code: `src/model/hitbox.rs`
(rules), `src/model/sites_hitbox.rs` (13 rows), `src/runtime/features/hitbox.rs` (hooks, pass, natives calls),
`src/runtime/game.rs` (`natives::` additions). Host tests: `tests/core06_hitbox.rs`; capability closure: `tests/capabilities.rs`.

## Spec section 9, row by row

| Requirement | Host test | Eden row (`eden/EDEN-HITBOX.md`) | Evidence |
| --- | --- | --- | --- |
| Sphere count and spacing for capsules; single sphere for a point hitbox | `core06_capsule_sampling` | hb-E1 extended hitbox | `eden/runs/hb-*` |
| Colour table by `id mod 8`; grab boxes `(id + 3) mod 8` | `core06_colour_table` | hb-E2 multi-hitbox move | |
| Per-frame step order and status exemptions (grab range, shield, tread, DOWN) | `core06_frame_rules_table` | hb-E3 grab, shield, missed tech | |
| First-frame draw from the `ATTACK` / `CATCH` parameters (indices as spec section 3) | `core06_command_param_indices` | hb-E4 first active frame | |
| Rebound cleared kills the spheres unless shielding | `core06_rebound_rule` | hb-E5 | |
| Training-only gate incl. `077a8e43`'s early return; no effect-sleep calls elsewhere | `core06_training_gate` | hb-E6 Quickplay with the setting On | |
| Three sites guarded (64 B) and installed independently; neighbouring thunk windows compared first | `core06_site_guards`, `core06_rows_match_the_owned_elf`, `tests/core01.rs` `BOOT_PHASES` | hb-E7 install line | |
| Off restores effects next frame (Q4: once, on the transition) | `core06_off_restores` | hb-E8 | |
| `AttackData` and Lua stack layouts the plugin reads | `core06_attack_data_layout_matches_the_owned_elf`, `core06_lua_stack_layout_matches_the_owned_elf` | (all rows) | below |
| Every native the feature calls is a granted capability; its closure is computed from the code | `tests/capabilities.rs` (`cross_needs_match_the_call_graph`, `a_poisoned_row_disables_...`, `the_native_call_layer_checks_the_grant`) | hb-E9 poisoned row | `CROSS_NEEDS[Hitbox]` |
| No heap allocation in the hooks and the pass | `HookScope` counters (`HookId::Hitbox*`), `tests/hook_alloc.rs` | hb-E10 `hook_stats` delta 0 | |
| Cost per frame | diag `[Hitbox] stats` line (ticks per pass, hitbox and hurtbox spheres) | hb-E10 | `eden/EDEN-HITBOX.md` |
| Hurtbox shapes per fighter (HB1 (a)): group 0 parts not switched off, capsule from `+0x08` / `+0x14`, size `+0x20`, node `+0x28`, yellow | `core06_hurtbox_shapes_per_fighter`, `core06_hurtbox_layout_matches_the_owned_elf` | hb-E11 screenshot with hurtboxes | |
| Hurtbox cost row | diag `[Hitbox] stats hurtbox_spheres= max_hurtbox_spheres=` | hb-E10 | `eden/EDEN-HITBOX.md` |

## Where the layouts come from (13.0.5 `main`, owned ELF `research/game-13.0.5/main-13.0.5.elf`)

Reproduce with `python3 -I tools/re_hitbox_layout.py <elf>` (it finds the `AttackModule` vtable from the NSO's relocation table, `main+0x4f6a5b8`,
and prints the functions below). `tests/core06_hitbox.rs` re-reads the ELF and asserts each instruction word named here.

`AttackData` (one attack slot; `attack_data(accessor, slot, false)` returns it):

| Fact | Evidence (instruction at `main+...`) |
| --- | --- |
| slots are 0x130 bytes from the pointer at module `+0x68` | `attack_data` (vtable slot 0x1e8, `0x3dda80`): `ldr x9,[x0,#0x68]; mov w10,#0x130; madd x0,x8,x10,x9` (`0x3dda9c`..) |
| node (bone `Hash40`) at `+0x48` | `set_node` (slot 0x148, `0x3dd540`): `str x2,[x8,#0x48]`; `set_attack` copies `[x21+0x48] -> [x27+0x48]` (`0x3dc240`) |
| size at `+0x24` | `set_size` (slot 0x118, `0x3dd3b0`): `str s0,[x8,#0x24]` |
| offset vector (x, y, z) at `+0x00`, second vector at `+0x10` | `set_attack` (slot 0x68, `0x3dc180`): `str q0,[x19]` and `str q0,[x27,#0x10]`; `get_offset2` (slot 0x140, `0x3dd500`) calls `attack_data` and loads the vector at `+0x10` |
| active word at `+0x108` | `is_attack` (slot 0xc8, `0x3dd1f0`): `ldr w8,[x8,#0x108]` (non-zero, and the module's `+0x238` byte clear) |
| a second centre is the vector at `+0x10`, zero for a point hitbox | the `ATTACK` command zeroes the argument (`stp xzr,xzr,[sp,#0x30]`, `0x2284bd0`) and stores `q2` there only for a present second centre (`0x2285320`) |

The `ATTACK` command (`0x2284b80`, behind the `0x2284b20` wrapper) builds the same structure at `sp+0x20` (vectors `+0`, `+0x10`, size `+0x24` from
argument 9, node `+0x48` from argument 3) and calls vtable slot 0x68 with it (`ldr x8,[x8,#0x68]; add x3,sp,#0x20; blr x8`, `0x2285ba0`).

Lua stack (`lua_State*`): top at `+0x10`, a pointer at `+0x20` whose first word is the frame base; argument `i` is the 16-byte value at
`base + 0x10 * i` (nil when not below the top); payload at `+0`, tag at `+8`: 3 float, 0x13 integer (`scvtf` when a float is wanted); the node is the
integer. Instructions: `ATTACK` `0x2284cd0..` (argument 4, `add x8,x8,#0x40`), `0x2284dd8` (9, size), `0x2284e90`/`0x2284f34`/`0x2284fe0` (10..12),
`0x2285110`/`0x22851c0`/`0x2285270` (13..15, read when 13 and 14 are not nil); `CATCH` `0x22ac2ec` (argument 3, `add x8,x8,#0x30`).
`sv_system::battle_object_module_accessor(L)` (`0x227daf0`): `ldur x8,[x0,#-8]; ldr x0,[x8,#0x1a0]`.

Natives (symbol, owned ELF): `EffectModule::req_follow` `0x2017730` (`...Hash40, Hash40, Vector3f const&, Vector3f const&, float, bool, unsigned, int, int, int, int, bool, bool`),
`kill_kind` `0x2017860`, `set_rgb_partial_last` `0x2017c80`, `set_rate_last` `0x2017ce0`, `MotionAnimcmdModule::set_sleep_effect` `0x2069140`,
`AttackModule::is_attack` `0x1fd01c0`, `attack_data` `0x1fd0520`, `GrabModule::set_rebound` `0x2027bb0`; the rows' 64 bytes are generated by
`tools/gen_hitbox_sites.py` and compared by `core06_rows_match_the_owned_elf`.

## Card scan (playbook "Card scan before the site is chosen")

`python3 -I tools/build/scan-offsets.py <plugins> 2284b20 22ac290 2027bb0 1fd01c0 1fd0520 2017730 2017860 2017c80 2017ce0 2069140 227daf0 2284cd0 22ac2ec 2087720 3728e10 2017a50`
over `/home/intpa/ssbu-cpu-pilot-install/switch-mtp-v0101-v021/pilot/after/atmosphere/contents/01006A800016E000/romfs/skyline/plugins` (16 plugins, folder dated
2026-11-01 10:19): only `libtraining_modpack.nro` (the fork, never installed with this plugin) references the three hook sites and `req_follow`; no other
plugin references any of the 16 offsets. The `Called` rows tolerate another plugin's hook anywhere in their window; the hooks' rows are `Hook/Replace/Entry` like
the CPU package's. Sharing v1 / chain: N/A (the scan showed nobody at the sites); a later plugin that hooks them is the case `GuardOutcome::HookedByOther` handles
(the feature stays off and the install line names the row).

## Decisions made without the lead (reported at the gate)

### Lead refusal investigation, 2026-11-02 17:2xZ — not a fix

Owned-native trace: the req_follow thunk dispatches through EffectModule slot
0x80. RELA for vtable0x4f6ccb8 maps it to0x44f880; is_exist slot0x188 maps
to0x451800 (explicitly rejects handle0), and get_last_handle slot0x3a0 to0x4530a0.
req_follow calls EffectManager creation0x3563430 at0x44fc30 and returns its
zero result unchanged. That creation has several refusal paths: missing effect
resource mappings, null free-list head at manager+0x19460, and failure of the
lower constructor0x356cec0 after a free node was taken. Free count+0x19468 is
decremented at0x35636e0. Thus hb04's zero return does **not** by itself establish
pool exhaustion, and no spawn cap/spreading change is justified yet.

Two new64-byte relied guards cover singleton resolution at0x44fbe4 and free-list
layout at0x35636b8. All16 NROs in the saved2026-11-01 pilot/after card copy had
no reference to either site (playbook scan-offsets.py,17:2xZ; physicalcard untouched).
Diagnostic-only snapshots report free count/head before a request and after a
refusal, limited to8failures perboot. No pool write or rendering behavior change.
Host ELF assertions check these instructions; Cargo/Switch/Eden gates pending.

- Q4 (per-frame restore while Off): done as the spec recommends: one `set_sleep_effect(false)` on the first pass after the setting goes Off (a per-fighter
  record of the fighters put to sleep), no game call per frame while Off.
- Q2 (shield-damage rewrite in the `ATTACK` hook): not here. Package 2 applies the Infinite shield through the game's own parameters
  (`cpu_shield_param_hook`), so the hook does no Lua pop/push at all.
- Q1 / Q3 (projectiles drawn once; effect-pool cost): behaviour as specified (spheres at the command, per-frame pass for fighters only); the cost is measured
  in the diag build (`[Hitbox] stats`), see `eden/EDEN-HITBOX.md`.
- Hurtboxes: operator decision HB1 (a) (spec updated `dcbb2d1`): drawn yellow from `HitModule` (accessor `+0xb0`; vtable `main+0x4f6e1d0` by its relocations,
  shown by `set_status_all` / `set_status_joint` / `get_status` / `get_part_size` matching their thunk signatures): group table `+0x60` (0x3c bytes; first `+4`, count `+6`),
  part table `+0x70` (0x50 bytes; status `+0`, offsets `+0x08` / `+0x14`, size `+0x20`, node `+0x28`), part count `+0x78`; status 3 is off (the part update at
  `main+0x468490` skips it). Rows `hurtbox_module`, `hurtbox_groups`, `hurtbox_part_shape` (relied). Help text: "On: hitboxes, grab boxes and hurtboxes are drawn."
