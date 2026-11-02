# Eden evidence, package 2 (CPU behaviour), 2026-11-02

Tester: claude-sonnet-5-5 (high), successor worker of the pkg2 stream under lead `0caa72b8`. Host: the **Ally** (the Surface was held by a foreign
Eden from 05:24Z to past the end of this work; the lead moved the batch). Rig and every step: [ALLY-SETUP.md](ALLY-SETUP.md). Playbook `ae87443`.
Runs: `eden/runs/s2-01..s2-04/` (gitignored: `console.log`, `run.json`, `ops.jsonl`, the plugin folders; the ring's two files for s2-02/03/04 under
`s2-0N.practice-diagnostics*.log`). Claim `CLAIMED`/`RELEASED` lines in the Yoga ledger (owner `Claude 0caa72b8/pkg2-eden`). Muted: every boot ran
`settings.set core.volume 0`, `settings.save`, `settings.get` (value 0) before `game.load`; the profile config carries `volume=0`.
One window, Training on Final Destination, P1 Steve, the CPU a random fighter (Bayonetta in s2-01, Incineroar in s2-02 by screenshot; fighter kind 71 in s2-02 and 16 in s2-03 per the `role` line; s2-04 not looked up),
offline, display of the Ally. Every `console.log` scanned for `panic|abort|crash|Unmapped`: **0 matches in all four** (4,8xx lines each);
`run.json` kept (peak 5.54 GB s2-02, 5.21 GB s2-04; `reason: crashed`, exit 137 is the Job kill the rig ends every run with).

## Builds (Ally WSL gates, each green: `cargo test` 820 tests 0 failed, clippy `-D warnings`, `--no-default-features`, Switch release and diag, 0 warnings)
| Boot | NRO | sha256 | resident |
| --- | --- | --- | --- |
| s2-01 release | tree before the install-line fix | `d0c2e38bab59e7b0030c9519c56b7b10b9fd17fef94b81b5ef32d2e869df0781` | 270,336 B |
| s2-02 diag | tree with the install-line fix | `f119cf9a6aebfa7f599939db955a83eefc7d91eb973516ccf8511081cd94cc17` | 405,504 B |
| s2-03 / s2-04 diag | + command/DI/tech/OOS query events | `cea35f9525133496c7acd699b8e273cadb5d2d06fb6676354d17e704fe1bb611` | 405,504 B |
| final release | same tree as the last diag | `7f7d626769fbb1e7cd22359f2b68292bb745bb317d73e44c835d26ebd913584d` | 270,336 B (text 126,976, ro 69,632, data 28,672, bss 45,056; unchanged against package 2's last release; no diag strings: `compliance.py`) |
Set: `libnro_hook` `ca7aec03…`, `libparam_hook` `3ed7a294…`, CPU pilot `e3033426…` (v0.2.1), tile cursor `b8313ca2…`, ours. Eden `45af318c…` (`stable-1.0.0`, MSVC).

## Rows
| Row | Boot | Result | Evidence |
| --- | --- | --- | --- |
| `cpu-0N-install-line` (release) | s2-01 | **fail, then fixed** | every site `stock` (near/far as designed, the four new sites too) but the line stopped at `… cpu_tech=installed ` with no `result=` and no `heap_allocations=`: the 2048-byte buffer cut it. Root cause: 62 sites. Fix `2d1df59` (buffer 4096, a host test renders the line of every site and feature). |
| install line, complete | s2-02, s2-04 | **pass** | `… cpu_tech=installed cpu_shield=installed cpu_mash=installed cpu_shield_damage=installed cpu_camera=installed cpu_common=not_installed reason=awaiting_common_nro settings: input_delay=0 lra_reset=on stage_hazards=off result=installed heap_allocations=0` |
| `common` module loader name | s2-01..04 | **pass** | `cpu_common_load module=common common_di_hook=guard_ok common_change_status_hook=guard_ok common_sub_guard_cont_hook=guard_ok common_is_enable_passive=guard_ok cpu_common=installed` (the name is `common`; `is_common` matches it) |
| constants by name | s2-02..04 | **pass** | 33 `cpu_ids` lines, no `cpu constants missing` line (all resolved) |
| pilot coexistence, both orders | s2-01/02/03 (pilot first), s2-04 (ours as `0libtraining_rooms.nro`) | **pass** | both install lines present in each, `result=installed`, pilot `level=3 … result=installed heap_allocations=0` (v0.2.1) |
| hook budget | s2-02..04 | **pass** | `cpu_hook_counts frames=600 … stick_x=1200 stick_y=1200 transition_term≈6000 command_flag≈4800 … heap_allocations=0` (about 10 + 2 + 8 hook calls per frame) |
| `cpu-02-sdi-high-up` | s2-02, s2-03 | **pass** | `sdi_input entry=1 x=0.000 y=1.000` every 4-5 frames of hitlag between `sdi_none` |
| `cpu-02-airdodge-down-out` | s2-02, s2-03 | **pass** | `airdodge_stick entry=1 x=0.707 y=-0.707 player_x=33.7 cpu_x=41.3` (player left of the CPU: out is +x) |
| DI (common hook), `cpu-02-di-out-both-sides` (one side) | s2-03 | **pass** (one side) | `di_before accessor=0x11e564b840 role=true`, `di entry=1 x=1.000 y=0.000 player_x=76.5 cpu_x=79.9` repeated each hitstun frame, only in `DAMAGE_FLY` (73); `DAMAGE_AIR` (72) hits never call the hook. The other side and `di-multihit` not run. |
| dodge staling | s2-02 | **pass** | `stale_flag value=true resolved=true` with the row Off; `value=false` after the reload to the factory row |
| settings reload (B11 / Q18 A infrastructure) | s2-02, s2-03 | **pass** | `diag_settings_reload rerolls=0x3` (SDI + shield tilt), `0x4` (ledge), `0xc` (ledge + throw), `0x1`, `0x0` |
| shield damage multiplier | s2-02 | **pass** | `common_params object=0x11a13c2418` at the game's fill; Infinite: `shield_damage_mul write=0.000 was=1.190 training=true` (1.19 is the vanilla value, so +0x174 is the right field); back to None: `write=1.190 was=0.000`. `guard_button held=true`, `shield_tilt x=0 y=1`, `shield_param decay=true value=0.0000 original=0.1500`, CPU `GUARD_ON` -> `GUARD` -> `GUARD_OFF`. Leaving Training (the restore outside Training) not exercised. |
| ledge Roll with a delay | s2-02 | **pass** | `status 118 (CLIFF_CATCH) situation=1`, `climb_denied status=119`, then (after about 1 s) `request_status status=122 (CLIFF_ESCAPE) from=119`, `status 122`, back to the stage. One grab only. |
| B11 re-roll on a menu change while hanging | - | **not run** | the CPU reached the ledge 1 time in 13 knock-offs; the other ledge options, the Wait row, the regrab and the tether rows are unrun |
| `cpu-01-mash-each-action` | s2-03 | **pass** (16 of 18) | Mash = one action, trigger Always, reload, L+R+A re-arms it (one roll per situation entry, B8): Spot Dodge bits `0x1000000` -> ESCAPE; Roll Fwd `0x2000000` -> ESCAPE_F; Roll Back `0x4000000` -> ESCAPE_B; Shield `0x400000` -> GUARD_ON, GUARD_OFF; Jab `0x1` -> ATTACK; F/U/D Tilt `0x2/0x4/0x8` -> ATTACK_S3/HI3/LW3; F/U/D Smash `0x10/0x20/0x40` -> …_START; Grab `0x20000000` -> CATCH; Neutral Special `0x1000` -> SPECIAL_N; Dash Attack -> ATTACK_DASH; Jump `0x200000` -> JUMP_SQUAT, JUMP, LANDING, again; Neutral Air `0x1`+jump -> ATTACK_AIR. Not shown: Dash alone (statuses by id collide in the dump) and Air Dodge (no events: the last row's log was read before it ran). `eden/runs/s2-03/mash-each-action.json`. |
| fixed camera: stage id and class | s2-02..04 | **pass** | `camera_setup game=[0.0, 85.0, 450.0] class=Standard stage=3` (`sv_information::stage_id()` = 3 = Final Destination, as the stage table says) |
| fixed camera: write | s2-04 | **pass** (write issued) | Hide Tech Animations On: `write=Some([0.0, 50.0, 230.0])`; Off (s2-02/03): `write=None`. The on-screen effect and `camera_mode` (mode 4 = fixed) not observed: the camera mode was not changed in the Training menu. |
| tech, mistech, hide tech, quake, clatter, OoS, throws, aerial rows, frame advantage | - | **not run** | no tech situation was reached (hits gave `DAMAGE_FLY` then `LANDING`); tech_call events show the `common` change-status hook sees the CPU body (`tech_call accessor=0x11e564b840 requested=… role=true`) |
| Playback request word | - | host test only | no recorder (package 4) |

## Findings for the lead
1. Install line cut at 2048 bytes (fixed, `2d1df59`): a diag-free check that the shipped line ends with `result=installed heap_allocations=0` should be in every package's Eden rows.
2. The spec's camera manager slot `main+0x52b6f00` is `0x52b8f00` in 13.0.5 (the fixed-camera setup function reads it; decoded by a host test).
3. `correctDamageVectorCommon` fires only for `DAMAGE_FLY` hits in these runs; DI Out is applied there.
4. The Training CPU is a level-3 fighting AI (here Incineroar used its specials by itself), so rows that need a still CPU (ledge, tech) need the save-state package or a different CPU setting.
5. Open: tech/mistech/hide-tech effects, clatter, OoS, throws, B11 on a ledge, camera mode 4, restore of the multiplier on leaving Training.
