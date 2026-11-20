# Guard the Edge + Juggler: design (edge-juggle-opus, 2026-11-20, for operator approval)

This proposes two new practice games in TR's practice-games framework. Both use the stock level-9 CPU, with an optional
Perfect mode: the same stock CPU plus targeted corrections at the decision points RESEARCH.md §4 identifies. Nothing is
implemented in Phase 1. Sizes are estimates, to be measured on the first slice. Open choices are §8, each with a
1920×1080 mockup in illustrations/.

## 1. Where they live

- Fighters & Stages › Practice games gains two rows, **Guard the Edge** and **Juggler** (`practice_games::Game` +2). Each
  opens its own sub-screen with Start first, as decided in "rc.15 practice menu" (DECISIONS PM, Q1–Q4). Only Start starts
  a game, Stop ends it, and one game runs at a time (`runtime::features::practice`).
- Training only (`scene::is_native_training`). Nothing runs in matches, online, or in the waiting room.
- Sub-screen rows:

| Guard the Edge | Juggler |
| --- | --- |
| Start / Stop | Start / Stop |
| Difficulty: Level 9 / Perfect | Difficulty: Level 9 / Perfect |
| Set up… (the wizard) / Use last setup | Set up… (the wizard) / Use last setup |
| CPU percent (0–300 %, default 100) | (percent is a wizard step) |
| Start state: Free fall / Launched (Q2) | Clock: from first hit / from release (Q3) |
| Attempts: 10 / 20 / Free | Attempts: 10 / 20 / Free |

## 2. Guard the Edge

- **Wizard** (Tech wizard pattern: `Step`, notice heading "Edge · n/4", A next, B back, X defaults → preview, + start):
  1. **Area:** off-stage lower left or lower right. The stage is split at its centre (Q1 asks about two vs four areas).
  2. **CPU start:** the Tech picker's free cursor, clamped to the chosen area: outside the ledge x, between
     blast-bottom + margin and ledge + 40.
  3. **P1 start:** the cursor snapped to the main stage's floor (picker restricted to floor lines of the main stage).
  4. **Preview:** both markers, the CPU percent and "double jump + up special available"; A or + starts.
- **Attempt:**
  1. Place both with the save-state placement routine (`states::place`; CPU in the air with status Fall; damage by heal +
     add).
  2. Restore the CPU's double jump and up special, as being hit does.
  3. Hold the CPU with `set_sleep` for a 60-frame "Ready 3-2-1" plate while P1 is free, then release.
  4. *Launched* (Q2, phase 2) instead applies a real knockback with Tech practice's native launch core, generalised from
     P1 to the CPU entry.
- **Outcome:**
  - **Secured:** the CPU is KO'd (its dead/rebirth status) before it reaches.
  - **Recovered:** the CPU enters `CliffCatch` (the ledge grab begins) or becomes grounded on any floor or platform.
  - A ledge P1 holds (trump) is not reached.
  - **Unresolved** after 15 s (a stall that never ends) does not count.
- **Score:** secured / attempts, streak and best streak, and time to secure (release → KO) with the fastest. Kept per
  (CPU fighter, area, difficulty) in the bounded best pool (`practice_games::record_time`).
- **Reset:** a 90-frame result plate ("Secured in 2.4 s" / "Recovered at the ledge"), then the same setup again. After a
  KO, the next placement waits for the CPU's Training respawn.

## 3. Juggler

- **Wizard** ("Juggle · n/3"):
  1. **Spot:** the free cursor anywhere in the camera range, off-stage left or right included. The notice shows the
     height above the stage.
  2. **Percent:** 0–300 %, ← → ±1, ↑ ↓ ±10.
  3. **Preview.**
  P1 starts under the CPU, clamped to the main stage (Q4).
- **Attempt:** place the CPU in the air at the spot (status Fall, percent set), place P1, then a 45-frame Ready plate and
  release.
- **Airtime:** counted in game frames, from P1's first hit on the CPU (Q3), so a free fall scores nothing.
- **What ends it:**
  - **Landing:** the CPU becomes grounded on any floor or platform (situation Ground), or grabs a ledge.
  - **KO:** the run ends as "KO"; its airtime still counts (Q3 option C).
  - A wall touch or a tech off a wall does not end it.
- **HUD:** "Airtime 3.42 s · hits 7 · best 5.10 s" on the practice HUD line. The best is a maximum, so the pool stores it
  negated or gets a max variant (`practice_games` helper).
- **Reset:** a 90-frame result plate, then the same setup.

## 4. Difficulty

**Level 9 (plain):**
- For the game's duration: CPU Level Lv 9 (rank 100, `cpu_level`), and Training's CPU Behavior set to CPU (Lv. 9) (Q6).
- TR's CPU-override rows are suspended (Mash, Followup, the DI/SDI/Airdodge chips, Ledge Options, overrides), so the
  stock CPU decides alone. Stop restores everything.

**Perfect** = Level 9 plus a correction layer. The corrections run in TR's existing AI-epilogue hook (main+0x2dbc38),
**after** the stock AI has written its pad:
- **Inputs:** the AI's own mode (`ai+0xc64`) and action (`ai+0x198`), the CPU's snapshot (`[ai+0x168]`: status,
  situation, position, speed, jumps left), and P1's position, status, input and active hitboxes. Inputs may be read.
- **Pure function:** `model::edge_juggle::perfect::step(facts, table) -> Correction { stick, press, release }`.
- **Applied:** only when a rule fires, by overwriting the pad fields, as Aerial Drift does today. Otherwise the stock pad
  passes through unchanged.
- **Commands:** air dodge and specials can also be asked for through the CPU's own command-flag path.
- **Legality:** the game's status scripts still decide every transition, so startup, recovery, helplessness and landing
  lag are the real ones. No teleports and no impossible inputs. The only position writes are the attempt's starting
  placement.

| Rule | Fires when (stock mode/action + facts) | Correction |
| --- | --- | --- |
| R1 route mix | RETURN, off-stage | Pick the ledge, high onto the stage, or late-low, weighted by P1's position: P1 at the ledge → high; P1 off-stage → low; otherwise random by weights. Steer the stick; time the jump and the up special for the chosen route |
| R2 jump timing | RETURN, jump left | Jump when the reach model says it is needed now (far), or save it (close-high). Gravity read from fighter params |
| R3 distance air dodge | RETURN, double jump and up special spent, the ledge out of reach without it but within reach with it | A directional air dodge toward the ledge |
| R4 reach model | used by R1–R3 | Height **and** horizontal reach: `air_accel`, `jump`, the up-special table (`sp_u_high/length`) and the per-fighter extension table |
| R5 no detour | RETURN, the stock rolls an attack on P1 | Keep recovering unless the attack is safe (P1 in lag) |
| R6 fighter table | RETURN | Per-fighter extensions and stalls, each verified by a trace before it ships: Little Mac side special then up special, counter when P1's hitbox will connect; Pikachu Quick Attack second angle around P1; Ness PK Thunder path around P1; Steve a block placed for a stall; Mario cape stall; then more |
| J1 landing mix | DAMAGED/FALL, airborne, P1 below | Drift away, fall straight, fast fall, double jump to change the arc, or drift to a ledge or platform. Weighted against P1's x and their move's reach |
| J2 timed air dodge | P1's attack startup will connect within its active frames (read from input + hitboxes) | Air dodge on the frame that covers the active window; no air dodge when P1 waits under the landing |
| J3 DI/SDI | hitstun | DI and SDI away from P1's follow-up (survival DI near blast zones), through TR's DI/SDI direction code |
| J4 meteor bounce | after a bounce | Don't air dodge on frame 1 when P1 waits |

- **Reaction floor (Q5):** answers to P1's moves wait at least N frames (default 12). Position-based choices act
  immediately.
- **Randomness:** each rule rolls among good options, so Perfect is hard to read but never plays badly on purpose.

## 5. Feasibility, hooks and cost

- **New game hooks:** none for the MVP.
  - Reused: the AI-epilogue inline hook, the command-flag hook, status observation, save-state placement natives, and
    the Tech picker and surfaces.
  - Each per-frame step is O(1). It reads published values and uses `try_lock`; it never blocks (WORKFLOW "Hooks never
    wait on a plugin thread") and never allocates.
- **Natives that may need new guarded rows:** the jump count (WorkModule int, already reachable through
  `natives::set_int`), and reading Training's CPU Behavior (already read by `cpu_level`). Writing CPU Behavior is new:
  it needs a guard and an operator ruling (Q6).
- **Size (estimate; measure the first slice before building out):** about 1,600–2,200 Rust lines including host tests.
  Juggler MVP ~450, Guard the Edge MVP ~500, Perfect generic ~500, fighter table ~150 + data (≈ 16 B/fighter). Roughly
  +40–70 KB resident.
- **Online safety:** Training-only gate; no checked RNG drawn (TR's own RNG); nothing runs in matches.
- **Risks:**
  - TR's CPU features and Mash out also write the pad at the same hook; the games must own it exclusively while they run
    (the `mashout::owns_cpu` precedent).
  - The respawn wait after a KO.
  - Per-fighter scripts (specializers) may fight a correction. Every R6 entry is gated on a trace.

## 6. Testing

- **Host tests (pure models):**
  - wizard steps and clamps;
  - session state machines: place → ready → live → result → reset;
  - outcome detection from synthetic status/situation sequences (ledge grab, ground, platform, KO, trump, timeout);
  - airtime clock and the max-best pool;
  - each Perfect rule from synthetic facts, including "passes the stock pad through when no rule fires".
- **Eden proofs on the exact release:**
  - The trace points in RESEARCH §6 run on every proof boot.
  - Rows: placement matches the setup (`state pos` lines); each outcome detected with its status trace; the HUD and best
    shown; Stop restores the CPU settings.
- **Perfect A/B:** the same 10 setups per fighter, Level 9 vs Perfect, against a recorded edge-guard segment (P1 at the
  ledge with down tilt / forward smash). Measured: recovery rate, route diversity (count of route classes) and juggle
  airtime. The numbers come from a committed script over the trace JSONL.
- **Per-fighter coverage:** the MVP five have distinct recoveries (Mario, Little Mac, Pikachu, Ness, Steve). Then groups:
  tethers, multi-jumpers, teleports, glides, the rest. Each new table entry ships with its trace.

## 7. Phases

| Phase | Content | Exit |
| --- | --- | --- |
| P0 | Eden traces of the stock CPU on the release (RESEARCH §6) | R1–R4, J1–J2 confirmed or corrected |
| P1 | Juggler MVP, Level 9 | host tests + Eden proof rows |
| P2 | Guard the Edge MVP, Level 9, free-fall start | host tests + Eden proof rows |
| P3 | Perfect generic rules (R1–R5, J1–J4) | A/B numbers on Mario and Pikachu |
| P4 | Fighter table for the MVP five | per-fighter traces + A/B |
| P5 | Launched start; more fighters by group | traces per entry |

## 8. Questions for the operator

| # | Question | Mockup | Recommendation |
| --- | --- | --- | --- |
| Q1 | Two start areas (lower left / lower right) or four (adding upper left / upper right)? | illustrations/Q1-edge-areas.png | A: two, as asked |
| Q2 | Guard the Edge start: free fall, a real launch, or both? | illustrations/Q2-edge-start.png | C: free fall first, launch later |
| Q3 | Juggler clock: from the first hit or from release; does a KO keep the time? | illustrations/Q3-juggle-clock.png | C: first hit; a KO keeps its airtime |
| Q4 | Juggler: where does P1 start? | illustrations/Q4-juggle-p1.png | A: under the CPU |
| Q5 | Perfect's reaction speed: frame-perfect, a human floor, or a row? | illustrations/Q5-perfect-reaction.png | B: 12-frame floor |
| Q6 | Suspend TR's other CPU settings and set CPU Behavior to CPU (Lv. 9) while a game runs? | illustrations/Q6-cpu-settings.png | A: suspend and set; restore on Stop |

Design diagrams: illustrations/A-perfect-architecture.png, illustrations/B-recovery-routes.png.

## 9. Requirement trace (operator request of 2026-11-20 ~00:30Z; tests are the planned host tests)

| Operator clause | Design | Test |
| --- | --- | --- |
| "guard the edge: which takes the CPU you have and the goal is to prevent them from reaching the ledge" | §2: the current CPU fighter; outcome Secured / Recovered | `edge_outcome_ledge_grab_is_recovered`, `edge_outcome_ko_is_secured` |
| "The stage will be divided into two areas from which the CPU will start in" | §2 wizard step 1, split at the stage centre | `edge_wizard_area_split_at_stage_centre` |
| "The user , with the wizard, will select the area (off stage lower left, off stage lower right etc)" | §2 wizard step 1 (Q1) | `edge_wizard_area_left_right` |
| "then decide where in that area the CPU will start at" | §2 wizard step 2, cursor clamped to the area | `edge_wizard_cpu_cursor_clamped_to_area` |
| "and where on the stage the player will start at" | §2 wizard step 3, snapped to the main stage floor | `edge_wizard_p1_on_main_stage_floor` |
| "The goal is to keep the CPU off the stage and secure the stock" | §2 outcome and score | `edge_score_secured_over_attempts` |
| "Cpus can be a little dumb getting back to stage so you may need to research optimal strategies for each character (e.g. Little Mac using counter to get closer to stage)" | RESEARCH §2/§4; §4 R1–R6 and the fighter table | `perfect_r6_littlemac_counter_when_hit_will_connect` |
| "Second mode is juggler. The goal is to keep the CPU in the air as long as possible without touching the ground/platform." | §3 airtime and landing detection | `juggle_landing_on_platform_ends_run`, `juggle_airtime_counts_game_frames` |
| "You can pick a starting height and percent and where above the stage to put them which can also be off stage to the left or right." | §3 wizard spot (off-stage allowed) + percent | `juggle_wizard_spot_allows_offstage`, `juggle_wizard_percent_range` |
| "For both I'd have a couple difficulty options. One is plain CPU with a level toggle 9" | §4 Level 9 | `difficulty_level9_sets_rank_100` |
| "and another is a perfect mode where the CPU plays optimally" | §4 Perfect rules | `perfect_passes_stock_pad_when_no_rule_fires` |
| "The CPU can read the inputs in both modes" | §4 facts include P1's input; J2 | `perfect_j2_airdodge_on_p1_startup_read` |
| "so this is mostly piggybacking off the CPU behavior with some help getting back to stage" | §4: corrections over the stock pad | `perfect_r3_distance_airdodge_only_when_it_reaches` |
| "it may be helpful to start simple with a CPU in the air at a height and trace it's decision tree in Eden while I juggle in the air then go from there. Same for ledge guarding." | RESEARCH §6, phase P0 | (Eden P0 trace rows) |
| "identify and correct specific shortcomings in the game's existing CPU behavior rather than replacing its decision-making entirely" | RESEARCH §4 table; one rule per shortcoming | `perfect_rules_each_map_to_one_shortcoming` |
| "Ideally, Perfect mode would build on the vanilla CPU, not reinvent it." | §4 architecture (illustrations/A) | `perfect_passes_stock_pad_when_no_rule_fires` |
