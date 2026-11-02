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
