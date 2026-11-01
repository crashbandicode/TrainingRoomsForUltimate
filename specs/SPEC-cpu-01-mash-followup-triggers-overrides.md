# CPU mash, follow-up, mash triggers and situation overrides — behavioural specification

- **Spec id:** `SPEC-cpu-01`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), 2026-11-01
- **Upstream reference read (spec writer only):** `src/training/mash.rs`, `src/training/character_specific/mod.rs`, the mash parts of `src/training/mod.rs`, `src/common/mod.rs` (situation predicates), `training_mod_consts/src/options.rs` (`Action`, `MashTrigger`), `training_mod_consts/src/lib.rs` (Mash and Override tabs) at commit `97cbe9b9` (and `077a8e43` where it differs: `src/training/mod.rs` only); fork changes at `…/stages/m5i/stage/src/training/mash.rs` lines `66, 107, 197-211`, `…/character_specific/mod.rs` lines `67-76`, `…/training/mod.rs` lines `63-75, 160-188, 242-337` from `docs/provenance/provenance-files.md`
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

Companion specs: `SPEC-cpu-03` (ledge options buffer a mash through the same queue), `SPEC-cpu-04` (tech and mistech buffer a mash), `SPEC-cpu-05` (shield hold, out-of-shield handling and the shieldstun trigger), `SPEC-cpu-06` (aerial delay, falling aerials, full hop, fast fall, attack angle, throws), `SPEC-cpu-07` (frame counters used for delays).

## 1. Purpose (one paragraph)

The practice CPU performs a chosen action as soon as it can, in reaction to a situation the player wants to drill: being hit, being in shieldstun, tumbling, landing, being footstooled or ledge-trumped, mashing out of a grab, leaving a tech or ledge option, or simply always. The player picks one or more actions (weighted), a follow-up action to chain after the first, which situations trigger the mash, and optionally a different action set per situation (an "override"). Everything here runs for the CPU-operated fighter only; the player's own fighter is never affected.

## 2. User-visible behaviour

### Menu items

All multi-select rows are weighted (weight 0–5 per option, see §3 "Random choice"). "Action" rows share one option list:

| Action option (display name) | Internal name |
| --- | --- |
| Air Dodge | `AIR_DODGE` |
| Jump | `JUMP` |
| Shield | `SHIELD` |
| Spot Dodge | `SPOT_DODGE` |
| Roll Forwards | `ROLL_F` |
| Roll Backwards | `ROLL_B` |
| Neutral Air, Forward Air, Back Air, Up Air, Down Air | `NAIR`, `FAIR`, `BAIR`, `UAIR`, `DAIR` |
| Neutral Special, Side Special, Up Special, Down Special | `NEUTRAL_B`, `SIDE_B`, `UP_B`, `DOWN_B` |
| Forward Smash, Up Smash, Down Smash | `F_SMASH`, `U_SMASH`, `D_SMASH` |
| Jab | `JAB` |
| Forward Tilt, Up Tilt, Down Tilt | `F_TILT`, `U_TILT`, `D_TILT` |
| Grab | `GRAB` |
| Dash | `DASH` |
| Dash Attack | `DASH_ATTACK` |
| Playback Slot 1 … Playback Slot 5 | `PLAYBACK_1` … `PLAYBACK_5` |

Mash tab (upstream tab title "Mash Settings"; the new menu may regroup these):

| Row | Setting key | Type | Values | Default |
| --- | --- | --- | --- | --- |
| Mash Toggles | `mash_state` | multi-select, weighted | Action list | none selected |
| Followup Toggles | `follow_up` | multi-select, weighted | Action list | none selected |
| Mash Triggers | `mash_triggers` | multi-select, not weighted | Hitstun, Shieldstun, Parry, Tumble, Landing, Ledge Trump, Footstool, Clatter, Ledge Option, Tech Option, Mistech Option, Grounded, Airborne, Distance: Close, Distance: Mid, Distance: Far, Always | Hitstun, Shieldstun, Tumble, Clatter |

Override tab (upstream tab title "Override Settings"): each row is a weighted multi-select over the Action list, default none selected. When a row has at least one option, it replaces `mash_state` for that situation and the situation no longer needs its trigger to be ticked.

| Row | Setting key | Situation it overrides |
| --- | --- | --- |
| Ledge Neutral Getup | `ledge_neutral_override` | after a neutral ledge getup (SPEC-cpu-03) |
| Ledge Roll | `ledge_roll_override` | after a ledge roll (SPEC-cpu-03) |
| Ledge Jump | `ledge_jump_override` | after a ledge jump (SPEC-cpu-03) |
| Ledge Attack | `ledge_attack_override` | after a ledge getup attack (SPEC-cpu-03) |
| Tech Action | `tech_action_override` | tech and mistech situations (here and SPEC-cpu-04) |
| Clatter | `clatter_override` | clatter situations (grab, bury, freeze, …) |
| Tumble | `tumble_override` | tumble |
| Hitstun | `hitstun_override` | hitstun |
| Parry | `parry_override` | parry (perfect shield) |
| Shieldstun | `shieldstun_override` | shieldstun (SPEC-cpu-05) |
| Footstool | `footstool_override` | footstooled |
| Landing | `landing_override` | landing |
| Ledge Trump | `trump_override` | ledge-trumped |

Also read here: `playback_mash` ("Playback Mash Interrupt", Input Recording tab, on/off, default On).

### Where it is live

Training mode and the fork's lobby practice (waiting room), for the practice CPU. Never in a real match. There are no button chords; the behaviour is driven entirely by the menu and the game state.

### Visible outputs

Only the CPU's actions. No HUD text, notification or effect belongs to this spec.

## 3. Inputs

Menu settings: the rows above, read live from the current menu each frame (no caching across frames except the queue described in §5).

Random choice: a weighted multi-select is resolved by expanding each option `weight` times (weight 1–5) and picking uniformly; an option with weight 0 is never picked; an empty row resolves to "no action". A single-select row resolves to its single selection. The pick must not consume the game's random number generator (it must leave recordings, desync tests and the game's own RNG stream untouched); use the plugin's own PRNG. It must not allocate on the heap.

Controller state: none directly (the CPU has no controller; the plugin injects command flags).

Game state read, per CPU-operated fighter, by role:

- Operator flag: whether the fighter's entry is operated by the CPU (`FighterManager::get_fighter_information(entry).is_operation_cpu`), through the entry id stored in the fighter's work module (`FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID`). Entry 0 is never treated as the CPU by the upstream predicate; the fork's predicate additionally refuses reserved/unqualified bodies and non-permitted lobby entries (see §6). Weapons and items are never operated-CPU fighters.
- Status kind (`StatusModule::status_kind`), previous status kind (`StatusModule::prev_status_kind(0)`), situation kind (`StatusModule::situation_kind`: ground, air, cliff), motion kind and motion frame (`MotionModule::motion_kind`, `MotionModule::frame`).
- Transition-term availability (`WorkModule::is_enable_transition_term`) for: `CONT_CATCH_DASH` (dash-grab cancel, used to detect the start of a dash attack), `CONT_DASH`, `CONT_ATTACK_AIR`.
- The work integer `FIGHTER_INSTANCE_WORK_ID_INT_INVALID_CATCH_FRAME` (non-zero means a grab cannot start now).
- Fighter positions (`PostureModule::pos`) of the player entry and the CPU entry, for the distance triggers; the status of the player entry (`FIGHTER_STATUS_KIND_NONE` means "no player yet", distance = infinity).
- Fighter kind (`utility::get_kind`) for the character-specific status table (§4).
- Save-state state: "a save state is loading" (interface of the save-state spec: any slot state other than idle).
- Input recording/playback state (interface of the input-recording spec): is playing back, is recording, is in standby/lockout, stop playback, begin playback of slot n, "is a playback queued" and "queued slot" (the two queries are provided by this spec to the recording spec).
- The fork's lobby interface: "effective action set" filter (removes `PLAYBACK_n` from any Action row while in a waiting scene with practice not enabled) and "entry permitted" (only entries 0 and 4 in a waiting scene).

### Situation predicates (by status id)

These are shared with the other CPU specs. All ranges are inclusive unless stated.

| Predicate | Definition |
| --- | --- |
| grounded / airborne | situation kind is `SITUATION_KIND_GROUND` / `SITUATION_KIND_AIR` |
| was airborne | previous situation kind (`StatusModule::prev_situation_kind`) is air |
| idle | status is `FIGHTER_STATUS_KIND_WAIT` |
| in hitstun | status in `FIGHTER_STATUS_KIND_DAMAGE` .. `FIGHTER_STATUS_KIND_DAMAGE_FALL` (end exclusive) |
| in tumble | status in `FIGHTER_STATUS_KIND_DAMAGE_FLY` ..= `FIGHTER_STATUS_KIND_DAMAGE_FALL` (checked before hitstun, because the hitstun range contains it) |
| in footstool | status in `FIGHTER_STATUS_KIND_TREAD_DAMAGE` ..= `FIGHTER_STATUS_KIND_TREAD_FALL` |
| shielding | status in `FIGHTER_STATUS_KIND_GUARD_ON` ..= `FIGHTER_STATUS_KIND_GUARD_DAMAGE` |
| in shieldstun | status is `FIGHTER_STATUS_KIND_GUARD_DAMAGE`, or status is `FIGHTER_STATUS_KIND_GUARD_OFF` with previous status `GUARD_DAMAGE` |
| in tech | status in `FIGHTER_STATUS_KIND_DOWN_STAND` ..= `FIGHTER_STATUS_KIND_DOWN_STAND_ATTACK`, or in `FIGHTER_STATUS_KIND_PASSIVE` ..= `FIGHTER_STATUS_KIND_PASSIVE_CEIL` |
| in ledge trump | status is `FIGHTER_STATUS_KIND_CLIFF_ROBBED` |
| in parry | motion kind is the hash of `just_shield_off` |
| in landing | status in `FIGHTER_STATUS_KIND_LANDING` ..= `FIGHTER_STATUS_KIND_LANDING_DAMAGE_LIGHT` |
| in clatter | status is one of: `CAPTURE_PULLED`, `CAPTURE_WAIT`, `CAPTURE_DAMAGE`, `SHOULDERED_DONKEY_START`, `SHOULDERED_DONKEY`, `BURY`, `BURY_WAIT`, `CAPTURE_YOSHI`, `YOSHI_EGG`, `CAPTURE_PULLED_YOSHI`, `CAPTURE_WAIT_YOSHI`, `CAPTURE_DAMAGE_YOSHI`, `SWALLOWED`, `SWALLOWED_CAPTURE`, `CATCHED_AIR_GANON`, `CATCHED_AIR_FALL_GANON`, `BIND` (Mewtwo Disable), `DAMAGE_SONG` (Sing), `BITTEN_WARIO`, `CLUNG_DIDDY`, `CLUNG_DAMAGE_DIDDY`, `ICE`, `CATCHED_REFLET` (Nosferatu), `KAMUI_PIERCE` (Corrin pin), `SWING_GAOGAEN_CATCHED`, `DRAGGED_RIDLEY`, `CATCHED_PICKEL_TROLLEY` (minecart), `SLEEP`, `FURAFURA` (dizzy), `GIMMICK_FISH_CAPTURE`, `CAPTURE_MASTERHAND`, `CAPTURE_ITEM`, `CAPTURE_BEETLE`, `CAPTURE_BLACKHOLE`, `CAPTURE_BEITCRANE`, `CAPTURE_KAWASAKI`, `CAPTURE_DRIVER`, `CAPTURE_MIMIKKYU`, `CAPTURE_CLAPTRAP`, `CAPTURE_BOSSGALAGA`, `CAPTURE_NABBIT`, `CAPTURE_MASTERCORE`, `CAPTURE_WAIT_OCTOPUS` (all `FIGHTER_STATUS_KIND_*`) |
| fighter distance | 3-D distance between the player entry's position and the CPU entry's position; infinity when either accessor is unavailable or the player's status is `NONE` |

Status, situation, transition-term and work ids are the game's own Lua constants (names as above); the implementer resolves them through the game's constant table, not through upstream code.

## 4. Outputs and native state touched

| Native state | How it is reached (function / field by name; `main+0x…` for 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| Category-1 pad command flags the CPU "presses" | Replace hook on the `ControlModule::get_command_flag_cat(accessor, category)` thunk, `main+0x2001630` (13.0.5; replace hook at entry, leaf thunk). Call the original, then OR in this spec's flag bits when `category == FIGHTER_PAD_COMMAND_CATEGORY1` and the fighter is CPU-operated. Also OR in the throw bits (SPEC-cpu-06) | every category-1 query (once per fighter per frame) | training/lobby-practice active; CPU-operated; lobby entry permitted; replay preview/quiescing capture not active (fork interface) |
| Aerial kind the game performs for an `ATTACK_N` press in the air | Replace hook on `ControlModule::get_attack_air_kind(accessor)`, `main+0x2001710`. Call the original; for a CPU-operated fighter not in input playback return the queued aerial's kind (`FIGHTER_COMMAND_ATTACK_AIR_KIND_N/F/B/HI/LW` for `NAIR/FAIR/BAIR/UAIR/DAIR`) when one is set, else the original | every query | training active; CPU-operated; not playback |
| Status change to Dash | `StatusModule::change_status_request_from_script(accessor, FIGHTER_STATUS_KIND_DASH, true)` on the fighter's own accessor, only when `CONT_DASH` is enabled | `DASH` or `DASH_ATTACK` action being performed | CPU-operated |
| Status change to Dash Attack | `StatusModule::change_status_request_from_script(accessor, FIGHTER_STATUS_KIND_ATTACK_DASH, true)` | the second consecutive frame the fighter is in `DASH` at motion frame 0 with motion kind `dash` (see §5) | CPU-operated |
| Mash queue, aerial-delay counter, dash-transition latch | plugin state, not game state | — | — |

The per-frame dispatch (the fighter vtable "on frame" slot, `.data main+0x4f80568`, original `main+0x614630`) belongs to the core spec; this spec's per-frame parts are called from it in the order given in §5. The hooked thunks are tiny leaf functions (four instructions: load, load, load, branch); the adjacent thunks `main+0x2001600`/`0x2001610`/`0x2001630`/`0x2001710` and `0x2001430`/`0x2001440` sit inside each other's 64-byte guard windows, so all guard checks must be taken before any of these hooks is installed (playbook rule, fork lesson).

Character-specific "performed" table (used by the completion check in §5): the action counts as performed when the current status equals the expected status, or when the fighter kind and expected status match one of these rows and the current status is in the row's set:

| Fighter kind | Expected status | Statuses that count as performed |
| --- | --- | --- |
| `FIGHTER_KIND_KOOPA` (Bowser) | `SPECIAL_HI` | `FIGHTER_KOOPA_STATUS_KIND_SPECIAL_HI_G`, `…_SPECIAL_HI_A` |
| `FIGHTER_KIND_TRAIL` (Sora) | `SPECIAL_N` | `FIGHTER_TRAIL_STATUS_KIND_SPECIAL_N1`, `…_N1_SHOOT`, `…_N1_END`, `…_N2`, `…_N3` |
| `FIGHTER_KIND_TRAIL` (Sora) | `ATTACK_AIR` | `FIGHTER_TRAIL_STATUS_KIND_ATTACK_AIR_N`, `…_ATTACK_AIR_F` |
| `FIGHTER_KIND_KROOL` | `SPECIAL_S` | `FIGHTER_KROOL_STATUS_KIND_SPECIAL_S_THROW`, `…_CATCH`, `…_FAILURE`, `…_GET` |
| `FIGHTER_KIND_KIRBY` (fork fix, required) | `SPECIAL_N` | every status in `FIGHTER_KIRBY_STATUS_KIND_MIIFIGHTER_SPECIAL_N` .. `FIGHTER_KIRBY_STATUS_KIND_NUM` (end exclusive): the whole copied-neutral-special range, so a Kirby wearing a hat completes a Neutral Special mash |

Make the table data-driven so rows can be added without code changes (see open question 6).

## 5. Timing and frame rules

### Queue

One queue of at most two entries: the main action and, behind it, the follow-up. "Current action" is the front entry (the main action until it completes, then the follow-up). Completing the current action pops it, re-evaluates shield suspension for the new front entry (SPEC-cpu-05), and clears the aerial delay and its frame counter (SPEC-cpu-06). "Full reset" empties the queue and then does the same clearing.

### Per-frame order (category-1 query of a CPU-operated fighter)

1. **Choose what to buffer** ("check buffer"). Skip entirely while a save state is loading. Otherwise evaluate the situations in this priority order and take the first that applies:
   1. in tech → `tech_action_override`, else `mash_state` if trigger Tech Option is set;
   2. in clatter → `clatter_override`, else `mash_state` if trigger Clatter is set;
   3. in tumble → `tumble_override`, else `mash_state` if trigger Tumble is set;
   4. in hitstun → `hitstun_override`, else `mash_state` if trigger Hitstun;
   5. in parry → `parry_override`, else `mash_state` if trigger Parry;
   6. in footstool → `footstool_override`, else `mash_state` if trigger Footstool;
   7. in ledge trump → `trump_override`, else `mash_state` if trigger Ledge Trump;
   8. in landing → `landing_override`, else `mash_state` if trigger Landing;
   9. otherwise `mash_state` if any of: trigger Grounded and grounded; trigger Airborne and airborne; Distance: Close and distance < 16.0; Distance: Mid and distance < 37.0; Distance: Far and distance < 64.0; Always.
   Shieldstun is triggered from the shield spec (SPEC-cpu-05) and Ledge Option / Mistech Option from SPEC-cpu-03/04; they are not evaluated here. An override row that resolves to "no action" (empty) falls back to the trigger rule; an override with options replaces `mash_state` even when the trigger is not ticked. The resolved action is a fresh weighted pick **every frame the situation persists**, and each pick performs a full reset and re-buffers (so the final action is the pick of the last frame of the situation; this is upstream behaviour and it keeps the chosen action responsive to the latest situation). Exception: while the fighter is beginning a dash attack (status `DASH`, or status `ATTACK_DASH` with `CONT_CATCH_DASH` enabled or motion frame ≤ 2), do not reset or re-buffer, because a new command could cancel the dash attack into a roll or grab.
2. **Buffer a menu mash** (used by step 1 and by the external entry points from SPEC-cpu-03/04/05/06): apply the fork's effective-action filter; refuse when the queue is non-empty or the action is empty. Playback interaction: if `playback_mash` is On, a playback is running (and it is not a recording, not standby, no playback is queued) and the new action is not itself a playback action, then stop the playback and buffer nothing (the mash re-triggers on a later frame once playback has stopped). Otherwise roll the attack angle (SPEC-cpu-06), roll the aerial delay if the action is an aerial (SPEC-cpu-06), push the action, then buffer a follow-up: a weighted pick from `follow_up` (effective-action filtered); if non-empty, roll its aerial delay and push it behind. Finally roll full hop, fast fall and falling-aerial (SPEC-cpu-06). The external entry point first performs a full reset, then does the above.
3. **Dash-transition latch**: the latch "transitioning dash" stays true only while the current action is `DASH_ATTACK` and the status is `DASH`; otherwise it is cleared.
4. **Perform the current action** and return its command bits:

| Action | Command flag returned (category 1) | Expected status that completes it | Notes |
| --- | --- | --- | --- |
| `AIR_DODGE` airborne | `FIGHTER_PAD_CMD_CAT1_FLAG_AIR_ESCAPE` | `FIGHTER_STATUS_KIND_ESCAPE_AIR` | |
| `AIR_DODGE` grounded, `SHIELD` grounded | `AIR_ESCAPE` (a harmless bit; the actual shield comes from the button hooks in SPEC-cpu-05 answering "shield held") | `GUARD_ON` | `SHIELD` airborne returns 0 and stays queued |
| `JUMP` | `JUMP_BUTTON` | `JUMP_SQUAT` when grounded, `JUMP_AERIAL` when airborne, `JUMP` otherwise | |
| `SPOT_DODGE` | `ESCAPE` | `ESCAPE` | |
| `ROLL_F` / `ROLL_B` | `ESCAPE_F` / `ESCAPE_B` | `ESCAPE_F` / `ESCAPE_B` | forward = facing direction |
| `DASH` | 0, plus a Dash status request when `CONT_DASH` is enabled | `DASH` | |
| `DASH_ATTACK` | 0; if not in `DASH`/`ATTACK_DASH`, request Dash (when `CONT_DASH`) and return. In `DASH` at motion frame 0 with motion `dash`: first such frame sets the latch, the next requests `ATTACK_DASH` from script | `ATTACK_DASH` | the game needs one dash frame before the dash attack |
| `NAIR`/`FAIR`/`BAIR`/`UAIR`/`DAIR` | see aerial rule below | `ATTACK_AIR` | |
| `NEUTRAL_B`/`SIDE_B`/`UP_B`/`DOWN_B` | `SPECIAL_N`/`SPECIAL_S`/`SPECIAL_HI`/`SPECIAL_LW` | `SPECIAL_N`/`S`/`HI`/`LW` | plus the character-specific table |
| `U_SMASH`/`F_SMASH`/`D_SMASH` | `ATTACK_HI4`/`ATTACK_S4`/`ATTACK_LW4` | `ATTACK_HI4_START`/`ATTACK_S4_START`/`ATTACK_LW4_START` | |
| `GRAB` | `CATCH` unless `INVALID_CATCH_FRAME` ≠ 0 (then 0) | `CATCH` | |
| `JAB` | `ATTACK_N` only when grounded (else 0, to avoid a nair) | `ATTACK` | |
| `F_TILT`/`U_TILT`/`D_TILT` | `ATTACK_S3`/`ATTACK_HI3`/`ATTACK_LW3` | `ATTACK_S3`/`HI3`/`LW3` | |
| `PLAYBACK_n` | 0 | — | the recording spec starts the playback when its timing rule allows and pops this entry when the playback ends ("playback queued" / "queued slot" queries) |

   Aerial rule: when grounded, include `JUMP_BUTTON`; if full hop was rolled, return only that (the attack waits until airborne). Then, if falling-aerial was rolled and the fighter is not yet falling (SPEC-cpu-06 "is falling"), or the aerial delay has not elapsed (SPEC-cpu-06), return the bits so far. Otherwise set the queued aerial kind to the action and add `ATTACK_N`; the `get_attack_air_kind` hook turns it into the right aerial.

   Completion check (applied whenever a flag is produced for an expected status): if the current status equals the expected status, or the character-specific table says so, pop the current action. If the current status is `DASH`, return 0 regardless (never interrupt a dash with command bits; the dash attack path is handled by status requests).

### Ordering against other features

Within the once-per-frame dispatch, the shield parameter installer runs first, then (for CPU-operated fighters) dodge staling (SPEC-cpu-02), input recording, the in-game frame-counter tick (SPEC-cpu-07; once per simulation frame, from the primary CPU body only), hide-tech (SPEC-cpu-04); then frame advantage (SPEC-cpu-07), hitbox visualisation, save states, mistech (SPEC-cpu-04), clatter (SPEC-cpu-02), fast fall (SPEC-cpu-06), ledge (SPEC-cpu-03), shield (SPEC-cpu-05), DI case reset (SPEC-cpu-02). The mash queue itself is serviced on the category-1 command query, which the game issues during the fighter's control update.

### Setting changes

Menu rows are read live, so a change applies on the next evaluation. One explicit rule (fork fix, required): when a menu save leaves both `mash_state` and `mash_triggers` empty and at least one of them changed, perform a full reset once, so a disabled mash does not keep running a queued action or follow-up. Other menu edits leave the queue alone.

### Reset, save state, session

L+R+A reset (detected on the CPU fighter's re-initialisation) and a save-state load both perform a full reset (together with the other resets listed in SPEC-cpu-07). A character item grant also clears the queue (fork item spec interface). The fork's session departure ("clear session state": leaving practice, scene change, entry replacement, swap) performs a full reset and clears the queued aerial kind.

## 6. Edge cases and interactions

- **Fighter identity.** The CPU predicate is per fighter body. Nana (`FIGHTER_KIND_NANA`) shares the Ice Climbers entry and therefore is CPU-operated too: upstream gives Nana the same command bits as Popo on her own category-1 query, so both mash. Keep that; but counters tick and hide-tech runs only for the primary CPU body (SPEC-cpu-04/07).
- **Pokémon Trainer.** The CPU entry's current body changes on a switch; the queue is per entry (plugin state), so a pending action carries over to the new Pokémon. The player entry's status `NONE` during a switch makes the distance infinite (no distance trigger).
- **Kirby.** Neutral Special completion covers all copy-ability statuses (table in §4). Hat changes by the fork's Kirby-hat feature do not touch the queue.
- **Bowser, Sora, K. Rool**: completion table rows in §4. **Mii, Aegis, Steve, Olimar, Rosalina/Luma**: no special handling; Luma is a weapon, never CPU-operated; an Aegis form change keeps the entry.
- **Save states.** No action is chosen while a save state is loading; loading performs a full reset.
- **Menu open.** Upstream keeps evaluating while the menu is open (the game is paused by the menu, so nothing advances). The core spec decides whether hooks are suppressed while the menu is open.
- **Lobby practice** (fork, carried over). The practice CPU is entry 4; only entries 0 and 4 are serviced; while the waiting scene has practice not enabled, `PLAYBACK_n` are filtered out of every Action row before a pick. Nothing runs during the real match.
- **Hot swap** (fork). A swap is refused while a training action is pending; the departure/arrival clears the session (full reset). After a swap the CPU predicate refuses "reserved" bodies, so a parked body never receives mash bits.
- **CPU-as-P1** (fork, cfg-gated). The predicate keys on the native operator flag, so a P1 under CPU AI control would receive mash bits too, and "player/CPU" for distance is still entry 0 / entry 1 (4 in the lobby). See open question 7.
- **Input playback.** During playback, a triggered mash ends the playback when `playback_mash` is On (and does nothing else that frame); when Off, the mash buffers normally but the playback keeps possession of the CPU inputs, so the mash acts only after the playback ends. A `PLAYBACK_n` action never produces command bits itself.

## 7. Persistence

Nothing is written by this spec. Its settings are rows of the menu JSON (menu spec).

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | see docs/resource-costs.md (W5) | queue of 2 entries, counters and latches: < 64 bytes of state; code under the per-feature size budget set in W5 | `tools/build/nro_size.py` per-feature build |
| per-frame work | see docs/resource-costs.md (W5); upstream performs one weighted pick with a heap-allocated vector on every frame of a trigger situation and a distance computation every frame | one status read, a few predicate comparisons, at most two weighted picks (main + follow-up) per frame of an active situation, no allocation; compute the distance only when a distance trigger is ticked | diag counters |
| heap allocations in hooks | upstream: one per weighted pick and one per queue push | 0 | counting allocator, install line |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Weighted pick: weight 0 never, weights proportional, empty → none, no game RNG, no allocation | `cpu01_weighted_pick_distribution`, `cpu01_weighted_pick_no_alloc` | — | `evidence/cpu-01/host.md` |
| Queue holds main then follow-up; completion pops; full reset empties | `cpu01_queue_main_then_followup` | `cpu-01-followup-nair-then-jump` | `evidence/cpu-01/followup.md` |
| Trigger priority order (tech > clatter > tumble > hitstun > parry > footstool > trump > landing > generic) | `cpu01_trigger_priority` | `cpu-01-tumble-vs-hitstun` | `evidence/cpu-01/priority.md` |
| Override replaces mash_state without its trigger; empty override falls back | `cpu01_override_precedence` | `cpu-01-hitstun-override-only` | `evidence/cpu-01/override.md` |
| Distance thresholds 16 / 37 / 64, infinity without a player | `cpu01_distance_thresholds` | `cpu-01-distance-mid` | `evidence/cpu-01/distance.md` |
| Each action's command bits and expected status; jab only grounded; grab blocked by invalid-catch frame; shield airborne returns 0 | `cpu01_action_flags_table` | `cpu-01-mash-each-action` (one sub-row per action) | `evidence/cpu-01/actions.md` |
| Dash attack: dash first, one dash frame, then dash attack; no re-buffer while beginning a dash attack | `cpu01_dash_attack_sequence` | `cpu-01-dash-attack` | `evidence/cpu-01/dash.md` |
| Completion via character-specific table incl. Kirby copy range | `cpu01_character_specific_completion` | `cpu-01-kirby-hat-neutral-b`, `cpu-01-bowser-up-b` | `evidence/cpu-01/charspec.md` |
| Playback mash interrupt On/Off | `cpu01_playback_interrupt` | `cpu-01-playback-interrupt` | `evidence/cpu-01/playback.md` |
| Menu disable (state and triggers emptied) clears the queue once | `cpu01_menu_disable_clears_queue` | `cpu-01-menu-disable` | `evidence/cpu-01/menu.md` |
| Save-state load and L+R+A reset clear the queue; no choice while loading | `cpu01_reset_clears_queue` | `cpu-01-savestate-reset` | `evidence/cpu-01/reset.md` |
| Lobby: entries 0/4 only, playback actions filtered without practice | `cpu01_lobby_effective_actions` | `cpu-01-lobby-mash` | `evidence/cpu-01/lobby.md` |
| Nana receives bits; no panic without accessors | `cpu01_nana_and_missing_accessor` | `cpu-01-ice-climbers` | `evidence/cpu-01/ic.md` |

## 10. Open questions

1. **Per-frame re-roll.** Upstream re-picks the action (and follow-up) on every frame a trigger situation persists, so with several options the executed one is effectively the last frame's pick, and the follow-up is re-rolled each time. Recommendation: keep the observable behaviour (a weighted choice per occurrence) but roll once per situation entry and keep it until the situation ends or changes kind; this removes the per-frame RNG and queue churn. Needs the lead's approval because it changes the distribution subtly (one roll per situation instead of per frame).
2. **RNG source.** Upstream uses the host `rand` thread RNG for weighted picks and the game's RNG elsewhere. Recommendation: one plugin-owned PRNG seeded at boot for all CPU-behaviour picks; never the game's RNG.
3. **Grounded air dodge = shield.** Upstream turns `AIR_DODGE` into a shield when grounded (and `SHIELD` does nothing in the air). Confirm this is wanted rather than, for example, a spot dodge.
4. **Shield mash completion.** `SHIELD` is "completed" when the status becomes `GUARD_ON`, while the shield is actually held by the button hooks (SPEC-cpu-05); after completion the follow-up may start while the shield button is still reported held for one frame. Verify on Eden that `SHIELD` followed by `JUMP` produces a jump out of shield without a dropped frame.
5. **`SHIELD` airborne.** The action returns 0 and stays queued until the fighter lands; a landing then triggers the shield (and a Landing trigger may re-buffer). Confirm that this is the intended drill behaviour.
6. **Character-specific completion coverage.** The table has four fighters plus Kirby's copy range. Other fighters with renamed special statuses (for example Mii Fighters, Shulk, Pyra/Mythra, Min Min, Steve) are not covered and their mash would stay queued until the generic expected status appears. Recommendation: make the table data-driven and extend it on Eden evidence; track misses with a diag counter ("action queued longer than N frames").
7. **CPU-as-P1.** No fork code excludes entry 0 from the CPU behaviours when P1 is AI-controlled (the `p1-cpu-control`/`cpu-pilot-probe` features are cfg-gated). Recommendation: apply CPU behaviours only to the CPU role (entry 1, or 4 in the lobby) and never to entry 0; the operator decides.
8. **Upstream HEAD `077a8e43`** returns from the once-per-frame dispatch outside training (adopted; the fork does the same) and inverts the reset-detection condition so that the reset runs only outside training. The inverted condition is not adopted; the reset behaviour is specified in SPEC-cpu-07.
