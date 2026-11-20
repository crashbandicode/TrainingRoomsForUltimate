# Guard the Edge + Juggler: design v2 (edge-juggle-opus, 2026-11-20)

**v2 (02:1xZ):** the operator's Q1–Q6 decisions are applied (§2–§4, "Decided" under §8). New: the recovery planner
with opposite-ledge crossing (§10), the per-fighter niche-recovery table (§11) and stage crossability from geometry (§12).

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
  1. **Area (v2, operator Q1):** one of **four**: lower left, lower right, upper left, upper right. The stage is split at
     its centre and at the main stage's top. Upper areas start the CPU above ledge height, off-stage, so high-to-stage
     recoveries get practised too.
  2. **CPU start:** the Tech picker's free cursor, clamped to the chosen area, outside the ledge x. Lower areas run
     from blast-bottom + margin to ledge height; upper areas from ledge height to blast-top − margin.
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

- **Wizard** ("Juggle · n/4"):
  1. **Spot:** the free cursor anywhere in the camera range, off-stage left or right included. The notice shows the
     height above the stage.
  2. **Percent:** 0–300 %, ← → ±1, ↑ ↓ ±10.
  3. **P1 start (v2, operator Q4):** the cursor snapped to any floor (main stage or platform), as Guard the Edge step 3.
  4. **Preview.** The heading is "Juggle · n/4".
- **Attempt:** place the CPU in the air at the spot (status Fall, percent set), place P1, then a 45-frame Ready plate and
  release.
- **Airtime (v2, operator Q3):** counted in game frames **from release**. The free fall before the first hit counts. The
  HUD also shows hits, so a run with no hit reads "0 hits".
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
| R2 jump timing | RETURN, jump left | The stock CPU jumps on its first actionable frame every time (P0 trace). Vary the jump timing within what the reach model allows (delay when safe, jump now when far). Gravity read from fighter params |
| R3 distance air dodge | RETURN, double jump and up special spent, the ledge out of reach without it but within reach with it | A directional air dodge toward the ledge |
| R4 reach model | used by R1–R3 | Height **and** horizontal reach: `air_accel`, `jump`, the up-special table (`sp_u_high/length`) and the per-fighter extension table |
| R5 no detour | RETURN, the stock rolls an attack on P1 | Keep recovering unless the attack is safe (P1 in lag) |
| R6 fighter table | RETURN | Per-fighter extensions and stalls, each verified by a trace before it ships: Little Mac side special then up special, counter when P1's hitbox will connect; Pikachu Quick Attack second angle around P1; Ness PK Thunder path around P1; Steve a block placed for a stall; Mario cape stall; then more |
| J1 landing mix | DAMAGED/FALL, airborne, P1 below | Drift away, fall straight, fast fall, double jump to change the arc, or drift to a ledge or platform. Weighted against P1's x and their move's reach |
| J2 timed air dodge | P1's attack startup will connect within its active frames (read from input + hitboxes) | Air dodge on the frame that covers the active window; no air dodge when P1 waits under the landing |
| J3 DI/SDI | hitstun | DI and SDI away from P1's follow-up (survival DI near blast zones), through TR's DI/SDI direction code |
| J4 meteor bounce | after a bounce | Don't air dodge on frame 1 when P1 waits |

- **Reaction speed (v2, operator Q5: a setting for all three):**
  - Sub-screen row "Reaction" with three values, the three options of the Q5 mockup: **Frame-perfect** (0 f), **Human**
    (12 f, default) and **Custom** (1–30 f, ← → when the row is focused).
  - Answers to P1's moves wait at least that many frames. Route planning and jump timing from position act immediately.
  - Both games have the row; it is greyed under Level 9.
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
  +40–70 KB resident. v2 adds the planner and the stage corridor test, about +400–600 lines (estimate).
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
| P0 | Eden traces of the stock CPU on the release (RESEARCH §5b/§6) | **Done 2026-11-20 (yoga-1):** R1 and R3 confirmed, R2 corrected, J1 supported. Open: a juggle-only boot, far/low starts, and P1 edge-guarding (R5) |
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

### Decided (operator, 2026-11-20 ~02:06Z, relayed by the primary)

| # | Decision |
| --- | --- |
| Q1 | All four start areas: lower left, lower right, upper left, upper right (§2) |
| Q2 | Free-fall start first, a launch start later (§2) |
| Q3 | The Juggler clock starts at release (§3) |
| Q4 | P1's start is a Juggler wizard step (§3) |
| Q5 | Reaction speed is a setting with all three options: Frame-perfect / Human 12 f / Custom (§4) |
| Q6 | Suspend TR's other CPU settings, CPU Behavior = CPU Lv 9 while running, restore on Stop (§4) |

## 10. Recovery planner (Perfect, v2): near ledge, opposite ledge, high, late-low

This replaces rule R1's fixed three-route mix. The stock CPU always takes the nearer ledge (P0-confirmed). The planner
chooses from what is **physically possible here**, then lets the stock CPU's own inputs run whenever the chosen route is
the one it is already flying.

- **Inputs, all read live and never from a hand list:**
  - Stage geometry from TR's surface reader (`model/tech/surface.rs`, the same lines Tech practice and the egg spawns
    use): floors, walls and **ceilings, the stage's undersides**, with their ends. Ledges are the floor ends that carry
    the game's cliff flag. The goal finder's line record has bits 0x40/0x80 for that (main+0x2edf90).
  - Blast bounds (`tech_surfaces::blast_bounds`).
  - CPU: position, velocity, jumps left (snapshot +0xcc), up special used or available (`is_sp_u_available`), status,
    damage. P1: position, active hitboxes, input.
  - The fighter's resources from §11's table and its params (jump heights, air speed and acceleration, gravity, fall
    speed).
- **Candidates:** near ledge, **opposite ledge (under the stage)**, high-to-stage (onto the floor), late-low (under the
  near ledge, then up).
- **Feasibility per candidate:**
  - Simulate a short sequence of real moves with the reach model R4: drift, jump, then the fighter's extension and
    recovery specials in their legal order.
  - Each move contributes its displacement envelope: a per-fighter table entry, or for the generic part the fighter's
    params.
  - The path must clear every collision line with the fighter's height plus a margin, and stay inside the blast bounds.
  - It must reach a grabbable point (the ledge box) or a floor.
  - Under-stage crossing needs a **corridor**: no floor, wall or underside line in the band between the path and the stage
    bottom along the whole crossing.
- **Choice:**
  - Remove infeasible candidates.
  - Score each feasible one: P1's threat along the path (distance from P1's reach, P1's current move frames), the time
    to safety, and the resources left on arrival.
  - Roll among the top candidates by score, with predictability penalised: the last N choices for this start lower a
    repeat's weight.
  - **Opposite ledge** is chosen when it is feasible and clearly safer: P1 covering the near ledge, or the near ledge
    unreachable.
  - With no feasible candidate, use the stock route (the CPU tries anyway).
- **Execution:** stick and button corrections at the AI epilogue, as in §4. Commands go through the CPU's own command
  path. A crossing hitbox that can hurt P1 (Pikachu's Thunder, §11) adds score when P1 stands on the route's exit.
- **Cost:** the planner runs once when the CPU becomes free off-stage and again on a re-plan trigger (hit, P1 moves into
  the path, a resource spent). It does not run every frame. Each plan is a few hundred line tests over the stage's lines
  (FD has under 20 lines). No allocation: fixed arrays, as in TR's surface reader.

## 11. Per-fighter niche-recovery table (candidates; each entry verified before it ships)

One compact row per fighter: which moves extend, stall or cross; their order constraints; whether they can hit while
crossing. The table only says what exists. §10 decides whether to use it.

| Fighter | Candidate niche (operator list + design notes) | Status |
| --- | --- | --- |
| Pikachu | under-stage crossing with Quick Attack (two angles) and Skull Bash; **Thunder's hitbox can hit an edge-guarder while crossing**; extends with Skull Bash + Quick Attack | **unverified (MVP, Eden)** |
| Sora | under-stage crossing (operator list); multiple air resources | **unverified (MVP, Eden)** |
| R.O.B. | under-stage crossing with Robo Burner fuel (stall/extend) | **unverified (MVP, Eden)** |
| Diddy Kong | under-stage crossing (Rocketbarrel / Monkey Flip) | **unverified (MVP, Eden)** |
| Mr. Game & Watch | under-stage crossing (Fire) | **unverified (MVP, Eden)** |
| Little Mac | Jolt Haymaker extension; Slip Counter as a stall and a step closer when P1's hitbox connects | unverified |
| Jigglypuff, Meta Knight, Pit / Dark Pit, Pac-Man, Bayonetta, Sonic, Pichu, Peach / Daisy, Snake, Link, Lucas, Lucario, Olimar, Bowser Jr., Steve, Hero, Yoshi | under-stage crossing (operator's community list) | unverified |
| Luigi | misfire Green Missile as a niche extension (operator note) | unverified |
| Pokémon Trainer | Charizard specifics (operator note) | unverified |
| Sheik, Greninja, Incineroar, Pyra/Mythra | known stock-CPU gaps (RESEARCH §2.8): Bouncing Fish, charged Shadow Sneak, Alolan Whip order, the Mythra switch | unverified |

- **Verification rule:**
  - An entry becomes `verified (trace)` when an Eden trace on the exact release shows the move sequence reaching the
    stated ledge with that fighter, the stage named. A frame-data claim (displacement, startup, the hitbox) cites its
    source.
  - Until then the planner treats the entry as unavailable. The generic jump/special model still runs.
- **MVP verification (when rc.15 frees a slot):** Pikachu, Sora, R.O.B., Diddy Kong, Mr. Game & Watch on Final
  Destination (crossing possible) and one stage where it shouldn't be (§12).
  - Rows use the P0 package (save-state starts under the stage edge); P1 stands at the near ledge.
  - With Perfect not yet built, the trace measures: whether the stock CPU ever crosses (expected never; R1), and the
    move envelopes. The crossing itself is proven by a scripted P1 performing the sequence as that fighter, a macro
    segment, on the same release.
- **Data size:** about 16–24 bytes per fighter for flags, envelope indexes and an order mask. Envelopes are shared per
  move class.

## 12. Stages: crossing allowed or not, from geometry

- No hand list. When a stage loads (and again when a moving stage's lines move), the reader builds the main stage's
  outline from its collision lines.
- It computes:
  - the lowest point of the main stage's collision (its undersides and walls);
  - the clearance between that and the blast bottom;
  - whether the outline extends down to (or past) the bottom blast zone, so no corridor exists;
  - where the walls and undersides sit relative to each ledge.
- `crossable = a corridor of height ≥ (tallest fighter height + margin) exists under the main stage from ledge to ledge,
  above the blast bottom by the fall budget`.
  - Final Destination and Battlefield-type stages pass.
  - A stage whose collision goes down off-screen (the operator's Yoshi's Story example) fails, and the opposite-ledge
    candidate is removed.
- Per fighter, the corridor test runs again with that fighter's own envelope inside §10, so a short-recovery fighter can
  still be refused on a crossable stage.
- Moving or changing stages (platforms on rails, transforming stages) re-read their lines each plan. Lines that move
  between plans make the plan re-run.
- **Test:** host tests over synthetic outlines (FD-like: crossable; a deep wedge: not crossable; a gap narrower than the
  fighter: not crossable). Eden: read the real lines on Final Destination, Battlefield and Yoshi's Story, and log the
  crossable verdict in the diag build's planner line, never in the release build.

Illustrations: illustrations/V2-edge-four-areas.png (the wizard's four areas),
illustrations/V2-recovery-planner.png (the four candidates and the under-stage corridor test).

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
| "for guard the edge a lot of characters have niche recoveries I want accounted for" (v2) | §11 per-fighter table, verification rule | `niche_table_unverified_entry_is_unavailable` |
| "Pikachu in particular can actually hurt you while going to the other side with thunder" (v2) | §11 Pikachu row; §10 crossing hitbox scoring | `planner_scores_crossing_hitbox_against_p1_at_exit` |
| "optimal recovery doesn't necessarily mean returning to the closest ledge" (v2) | §10 candidates incl. opposite ledge | `planner_picks_opposite_ledge_when_near_is_covered` |
| "evaluate the physical possibility of an opposite-ledge recovery based on the stage collision geometry, current position, remaining jumps, available specials, and recovery resources" (v2) | §10 feasibility simulation | `planner_infeasible_without_jump_or_special` |
| "stages such as Yoshi's Story have geometry that extends downward, preventing underneath-stage crossings entirely" (v2) | §12 corridor test from the stage's lines | `stage_deep_wedge_is_not_crossable`, `stage_fd_like_is_crossable` |
| "character-dependent and stage-dependent" (v2) | §10 per-fighter envelope inside §12's corridor | `planner_refuses_crossing_for_short_envelope` |
| Q1 four areas, Q3 clock at release, Q4 P1 start in the wizard, Q5 reaction setting (v2 decisions) | §2, §3, §4 | `edge_wizard_four_areas`, `juggle_clock_starts_at_release`, `juggle_wizard_p1_start_step`, `reaction_setting_three_values` |
