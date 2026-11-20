# Guard the Edge + Juggler: research (edge-juggle-opus, 2026-11-20)

The stock CPU's recovery and juggle-escape logic is mapped statically from the game's own symbols. It explains the
public, well-known weaknesses: the CPU always heads for the nearer ledge, air dodges only near it, and drifts toward the
opponent when juggled. **An Eden trace of the exact release (P0, §5b) confirmed the recovery findings (R1, R3), refined
R2, and supported J1.** TrainingRooms (TR) already has the two primitives a "build on the vanilla CPU" Perfect mode needs:
a per-frame hook right after the stock AI writes its pad, and placement of any body. **Still open:** far/low starts
that force up special, real edge-guarding pressure (R5 and the danger delay), and juggle rates (one juggle start so far).
The next rows are in §5b.

Sources: stock 13.0.5 `main` and `lua2cpp_common.nro` from the operator's dump, read with headless Ghidra and llvm-objdump
(evidence/static/STATIC-FINDINGS.md has every address); TR read-only at /home/intpa/tr-fixes-20261113/integration
(96355cad); public descriptions only (SmashWiki AI-flaw list, community recovery guides). No upstream Training Modpack,
UnclePunch or TM-CE source was opened.

## 1. How the stock CPU is built

| Layer | What it is | Evidence |
| --- | --- | --- |
| Native AI object (`FighterAI`, main) | One per CPU entry, reached from the entry's controller (`+0x58`). It holds rank `+0xc0c` (Lv 9 = 100), mode `+0xc64`, action id `+0x198`, a self snapshot `[+0x168]` (status, situation, position, speed, jumps left, damage, ground line) and the per-fighter recovery parameters. | `ai::*` getters, TR `cpu_level.rs` |
| Think function | main+0x2da180 runs the Lua-compiled scripts. At its end it writes a **virtual pad**: stick to `[ai+8]+0x10/+0x40`, buttons to `+0xc`. The stock CPU is a controller like a player's. | main+0x2dbbac..0x2dbbe0 |
| Scripts (`lua2cpp_common.nro`) | `L2CFighterAIModeBase` modes and actions: `ACTION_RETURN` (recovery), `ACTION_DAMAGED` (hitstun), `ACTION_ESCAPE_AIR`, `ACTION_FALL`, `ACTION_CLIFF`, …, and selectors (`select_guard_action_air`, `select_aerial_jump_action_air`, `select_dive_action_air`). 1,563 named functions. | evidence/static/common-ai-functions-13.0.5.tsv |
| Per-fighter scripts | Each `lua2cpp_<fighter>.nro` registers its own mode/action agents (`create_agent_fighter_ai_{mode,action}_<fighter>`) and hooks in through `specializer_*` calls, for example `specializer_interrupt_action_on_return`. | nro_to_elf + nm |
| Level | Probabilities scale with rank through `rank_rate(lo, hi)`. Level changes how likely and how fast, not the algorithm. | `rank_rate*` calls throughout |

## 2. Recovery: what the stock CPU does

Static reading of `ACTION_RETURN__update` (20 KB), `common_return_set_goal`, `predict_able_to_return_from_here`, the r-jump
timing functions and main's `return_pos` (details and addresses: evidence/static/STATIC-FINDINGS.md).

1. **Target:** the native goal finder (main+0x2edf90) puts the goal **20 units outside the nearer floor edge, at edge
   height −1**. The drift (`common_return_set_goal`) steers to the per-fighter `return_goal_x`. Nothing in the path aims
   high onto the stage or at the far ledge.
2. **Jump:** the recovery jump fires on fixed geometry: ledge reachable, `pos_y` against thresholds, `dead_bottom + 20`.
   It does not depend on where the opponent is.
3. **Up special:** allowed by a rank-scaled roll in `ACTION_RETURN__init` (`rank_rate(100, 30)`), near-certain at Lv 9.
   Its timing comes from the per-fighter `sp_u_high`/`sp_u_length`/`return_sp_u_cliff_x/y` table.
4. **Can I make it:** `predict_able_to_return_from_here` counts **height only**: the remaining jump plus `sp_u_high × k`
   against the fall speed. The horizontal reach is not checked there.
5. **Air dodge:** used in RETURN only when the ledge is within about 0.75 × the air dodge's slide distance. It is never
   used to gain distance.
6. **Danger:** it checks its own straight path 8 frames ahead for hitboxes (`check_line_segment_is_danger`) and waits or
   evades. That is the "delays its recovery when you wait" behaviour players see.
7. **Attack detour:** when the opponent is close (|Δy| < 30), rank-scaled rolls send it to attack instead of recovering.
8. **Fighter specifics:** go through the specializer hooks. Public lists name the gaps: Sheik never uses Bouncing Fish to
   extend; Greninja never charges Shadow Sneak; Incineroar uses Cross Chop before Alolan Whip; Pyra doesn't switch to
   Mythra. An amiibo comparison notes that Little Mac's CPU never goes off-stage and that CPUs jump too late. Little Mac's
   counter as a stall is a known player technique the CPU doesn't use.

## 3. Juggle escape: what the stock CPU does

1. **DI and drift in hitstun** (`ACTION_DAMAGED__function_input_air`): a full stick toward the opponent's side
   (`target_lr`), or toward or away from the stage by position.
2. **SDI** (`hit_stop_slide`): gated by slide type and rank thresholds; short input bursts of 6–8 frames.
3. **After hitstun** (`ACTION_DAMAGED__update`): a 20 % roll to press (a jump if one is left), a 33 % roll to push the
   stick; on stage, 50 %.
4. **Air dodge** (`select_guard_action_air`): only when a hitbox lies on **its own projected path**. It doesn't air dodge
   because an attack is coming up from below at its landing spot.
5. **Double jump** (`select_aerial_jump_action_air`): 2 %, 10 % or 50 % rolls by where the opponent is.
6. **Fast fall** (`select_dive_action_air`): after a danger check, `rank_rate(0, 90)` and 50 % near edges.

Together, these make the CPU land toward the juggler and rarely change trajectory. The public list also names poor SDI
and an air dodge on the first actionable frame after a meteor bounce.

## 4. Shortcomings worth correcting (for Perfect)

| # | Shortcoming | Evidence now | Correction (DESIGN §4) |
| --- | --- | --- | --- |
| R1 | Always the nearer ledge; never high, never late-low | **trace (P0): goal (±80, 0) on every attempt**, static, public | Route mix chosen against P1's position |
| R2 | Jump timing always the same (trace: first actionable frame from below the ledge, identical paths) | **trace (P0)**, static, public | Vary the timing; jump from a reach model (delay when safe, early when far) |
| R3 | No air dodge for distance | **trace (P0): only from 20 units out**, static, public | Insert a directional air dodge when it reaches the ledge and nothing else does |
| R4 | Height-only "can I make it" | static | Horizontal reach in the model (fighter params + table) |
| R5 | Attacks the edge-guarder instead of recovering | static (rank rolls) | Suppress the detour in Perfect unless it is safe |
| R6 | Per-fighter extensions unused (Mac side special + counter, Sheik fish, Greninja charge, Pikachu second angle, Ness PK Thunder angle, Steve blocks) | public; to trace | Per-fighter recovery table |
| J1 | Lands toward the juggler; rare double jump / fast fall | static (DI toward target_lr, 2–10 % jump rolls), public | Landing mix: away, straight, fast fall, double jump, ledge or platform |
| J2 | Air dodge only on its own predicted path | static | Air dodge timed on P1's move startup (input read allowed) |
| J3 | Poor SDI / DI on multi-hits | public; static gates | DI and SDI away from the follow-up (TR DI / SDI machinery) |
| J4 | Air dodge on the first actionable frame after a meteor bounce | public | Delay or replace when P1 waits |

## 5. TR systems to reuse

| Need | TR system | File |
| --- | --- | --- |
| Correct the stock CPU's pad without replacing it | AI-epilogue inline hook main+0x2dbc38, after the pad write. Mash out already owns the pad there; Aerial Drift already overrides only stick X on chosen frames | `runtime/features/recorder.rs` `ai_epilogue`, `on_ai_frame` |
| Ask for a legal action through the CPU's own command path | the `get_command_flag_cat` hook (Mash/Followup) | `model/cpu/command.rs`, `runtime/features/cpu.rs` |
| Place both bodies, set damage, set facing, air or ground | save-state placement (`states::place`: clear speed, `init_pos`/`set_pos`, `set_lr`, request Fall/Wait; damage by heal + add) | `runtime/features/states.rs`, `state_natives.rs` |
| A real launch (optional start) | Tech practice's native launch core (P1-bound today) | `tech_adapter.rs`, `tech_launch_native` |
| Wizard, cursor, surfaces, blast bounds | Tech wizard (`Step`, notices, A/B/X/+), picker over camera range, surface discovery, `blast_bounds` | `model/tech/{wizard,picker,surface}.rs`, `runtime/features/tech.rs` |
| One running game, Start/Stop, best pools, result countdown | practice-games framework | `model/practice_games.rs`, `runtime/features/practice.rs` |
| Lv 9 | CPU Level (rank 100 via main+0x6b8400) | `runtime/features/cpu_level.rs` |
| Ledge/landing/KO statuses | the CPU id table (`C::StatusCliffCatch`, `CliffWait`, situations) | `model/cpu/ids.rs` |
| P1's hitboxes and frame data for timing | hitbox visualizer readers, `ai::attack_*` per-command data (projectile-madness) | `runtime/features/hitbox*`, projectile-madness RESEARCH |

Runtime corroboration from projectile-madness-fable's run `projectile-madness-d6393cb0-xps-1` (exact release d6393cb0,
2026-11-20; their PROGRESS.md 00:50Z/00:57Z entries):
- The stock AI's pad, traced before and after TR's hook at main+0x2dbc38, was equal on 2,626 pairs while no TR feature
  ran there. The hook site and pad layout used here are right, and TR writes nothing when idle.
- With command flags alone, the stock AI's own stick still ran under TR's bits and caused a charge cancel. A correction
  layer must therefore write the pad, not only command flags. That is this design's choice.
- At CPU Level 9 the AI ran at rank 100 (mode 1 on 1,305 of 1,342 think calls); the pause menu showed Training's own
  level.

## 5b. P0 Eden trace (2026-11-20, Yoga, exact release d6393cb0): what was confirmed

Run `edge-juggle-p0-d6393cb0-yoga-1`: 7/7 rows PASS, released clean, 0 panic/abort/crash/unmapped. Per-frame stock-AI
records: 21,143, 0 dropped. Lv 9 = rank 100 on every record. Details and numbers: evidence/yoga-1/P0-RESULTS.md
(analyze_p0.py, aggregate_p0.py).
- **R1 confirmed:** the AI's return goal is always the nearer ledge point, (±80, 0) on Final Destination, over 12
  recoveries from four starts. It never takes a high route or the far side.
- **Predictability confirmed:** identical starts gave identical jump positions (two distinct paths over three loads from
  (120, -45)).
- **R2 corrected:** from below the ledge it jumps on the first actionable frame. The timing is immediate and identical,
  not a fixed lower height. The flaw is that it is always the same, not that it is late.
- **R3 confirmed:** the air dodge toward the ledge came only from 20 units out (5/5, at release). From 35+ units it used
  the double jump.
- **J1 supported (one juggle start only):** the CPU double jumped and attacked at once, and its air stick pointed toward
  Player 1 on 68 of 70 frames.
- **Not covered yet:** starts far or low enough to force up special; the 8-frame danger-avoidance delay under real
  edge-guarding; R5 (attack detour); juggle rates. Next row: a juggle-only boot (slot 4) and far/low starts with P1
  edge-guarding at the ledge.

## 6. Eden trace plan (exact release, when a slot frees)

- **Build:** stock 13.0.5 + released TR d6393cb0 (no rebuild), Eden 2f315fc4, muted, GDB stub off. `trace.configure` and
  the points go in before `game.load`. The package is adapted from projectile-madness's console package (same pins).
- **Point A, per AI frame:** `main+0x2dbc2c` (stock instructions just before TR's hook, `x19` = AI object). Captures:
  `[x19+8]:0x50` (the pad), `x19+0x190:0x10` (action id), `x19+0xc40:0x50` (buttons, mode, previous mode, timers,
  return counter), `[x19+0x168]:0x100` (status, situation, position, speed, jumps, damage), `x19+0x270:0x40` (return
  goal). That is about 60 records/s, under the 1,000/s cap. One record per frame is the CPU's whole decision trace.
- **Point B, script entries (counters + backtrace):** `nro:13f5177a7e86f0150f1a22aa9e611eacd010397c+<offset>` for
  `ACTION_RETURN__update` 0x4d1ca0, `predict_able_to_return_from_here` 0x4a0260, `select_guard_action_air` 0x4a2610,
  `select_aerial_jump_action_air` 0x47c7f0, `select_dive_action_air` 0x49d330, `ACTION_DAMAGED__function_input_air`
  0x505350.
- **Starts:** a `save_states.json` fixture: a record holds `x`, `y`, `percent`, `lr`, `situation_kind` and `fighter_kind`
  (`model/state/record.rs`). The load-refusal rules (fighter kind, marker) must be checked offline before the boot. The `tr-load` chord (route library) puts the CPU off-stage low-left, low-right and far. For the juggle:
  the CPU high above the stage at 60 % with P1 below, and P1 repeating a recorded up-smash / up-air segment.
- **Questions each run answers:** does the goal always sit at the ledge (R1)? At what height does it jump (R2)? Is an air
  dodge ever used far out (R3)? How often do air dodge, jump and fast fall occur while juggled (J1, J2)? Mario, Little Mac,
  Pikachu, Ness, Steve; about 10 starts each; Final Destination.

## 7. Limits of this research

- Static reading shows structure, literal probabilities and thresholds. Some table keys are hash40 names I haven't
  resolved, and the per-fighter specializer bodies are unnamed. Runtime confirmation is §6.
- Fighter-specific claims in R6 come from public descriptions and are marked "to trace" until Eden shows them.
