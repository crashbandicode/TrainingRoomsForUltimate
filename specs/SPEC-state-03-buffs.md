# Save states: buffs on load — behavioural specification

- **Spec id:** `SPEC-state-03`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), 2026-11-01
- **Upstream reference read (spec writer only):** `src/training/buff.rs`, `src/training/mod.rs` (`handle_add_limit`, `handle_req_screen`, `handle_check_doyle_summon_dispatch`, `handle_add_damage`, `handle_reused_ui`, `handle_fighter_play_se`), `src/training/frame_counter.rs` (`should_delay`), `training_mod_consts/src/options.rs` (`BuffOption`), `training_mod_consts/src/lib.rs` (Buff Options row), `src/common/offsets.rs` (`CLOUD_ADD_LIMIT`, `ADD_DAMAGE`, `REUSED_UI`, `PLAY_SE`) at commit `97cbe9b9` (unchanged at `077a8e43`); fork changes at `…/stages/m5i/stage/src/training/buff.rs` lines 9, 19-26, 30-35, 37, 45, 53, 65, 73 and `…/stage/src/training/mod.rs` lines 565-624, 717-751 from `docs/provenance/provenance-files.md`
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

After a save state has placed a fighter, the plugin can put that fighter straight into a powered-up state that would otherwise take time to set up: Hero's spells, Joker's Arsene, Wii Fit's Deep Breathing, Cloud's Limit, Little Mac's KO Punch, Sephiroth's Winged form, a Shulk Monado Art, Min Min's Power Dragon arm and a Wario waft level. The buff is applied by driving the fighter through the game's own activation (sped up and silenced) or by writing the gauge directly, so the resulting state is exactly what the game would produce.

## 2. User-visible behaviour

- Menu item (Save States tab): **Buff Options**, setting id `buff_state`, multi-choice without weights, default none selected. Values and the fighter each applies to:

| Value | Label | Fighter | Group |
| --- | --- | --- | --- |
| `ACCELERATLE` | Acceleratle | Hero | Hero spells (all selected are applied, in menu order from last to first) |
| `OOMPH` | Oomph | Hero | Hero spells |
| `PSYCHE` | Psyche Up | Hero | Hero spells |
| `BOUNCE` | Bounce | Hero | Hero spells |
| `ARSENE` | Arsene | Joker | single |
| `BREATHING` | Deep Breathing | Wii Fit Trainer | single |
| `LIMIT` | Limit | Cloud | single |
| `KO` | KO Punch | Little Mac | single |
| `WING` | 1-Winged Angel | Sephiroth | single |
| `MONAD_JUMP`, `MONAD_SPEED`, `MONAD_SHIELD`, `MONAD_BUSTER`, `MONAD_SMASH` | Jump, Speed, Shield, Buster, Smash | Shulk | Monado Arts (one selected at random per load) |
| `POWER_DRAGON` | Power Dragon | Min Min | single |
| `WAFT_MINI`, `WAFT_HALF`, `WAFT_FULL` | Mini Waft, Half Waft, Full Waft | Wario | waft levels (one selected at random per load) |

- Help text: "Buff(s) to be applied to the respective fighters when loading a save state".
- Live in Training only (fork: and in owned lobby-practice restores). Applied to whichever of the two fighters is the right kind; both can be buffed in one load.
- Visible outputs: the fighter appears already buffed when the load finishes; the activation animation is skipped or played at high speed; the activation sounds, voice lines, camera quakes and screen darkening are suppressed; Little Mac's KO meter reads 100 on the HUD; Wii Fit does not heal.

## 3. Inputs

- `buff_state` (read on each ApplyBuff frame).
- Per fighter: fighter kind, `StatusModule::status_kind`, `StatusModule::prev_status_kind(0)`, work ints/flags named in §4, Wario's `param_special_lw` parameters `gass_middle_time`, `gass_large_time`, `gass_max_time` (`WorkModule::get_param_float`), Min Min's parameter `WorkModule::get_param_int(0xdf05c072b, 0xf4fd45d48)` (arm "big" frame count).
- The in-game frame counter service: a counter registered as an "in-game" counter that resets on training reset; `should_delay(n)` starts counting on first call and returns "still delaying" until `n` in-game frames have elapsed, then resets itself.
- Which record (player or CPU) the fighter belongs to (SPEC-state-01 §5), for the two per-record flags below.

State kept by the feature: per record (player, CPU) a **buffing** flag and a **spells remaining** count; one shared delay counter.

## 4. Outputs and native state touched

| Native state | How it is reached (function / field by name; `main+0x…` for 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| Sound, rumble, animcmd, quakes | `SoundModule::stop_all_sound`, `ControlModule::stop_rumble(false)`, `MotionAnimcmdModule::set_sleep(false)`, `CameraModule::stop_quake(CAMERA_QUAKE_KIND_M)`, `stop_quake(CAMERA_QUAKE_KIND_S)` | every ApplyBuff frame, before the fighter-specific step | record marker is ApplyBuff |
| Hero spell choice | `WorkModule::set_int(spell, FIGHTER_BRAVE_INSTANCE_WORK_ID_INT_SPECIAL_LW_DECIDE_COMMAND)`; spell ids: Acceleratle → `FIGHTER_BRAVE_SPECIAL_LW_COMMAND11_SPEED_UP`, Oomph → `…COMMAND12_ATTACK_UP`, Psyche Up → `…COMMAND21_CHARGE`, Bounce → `…COMMAND13_REFLECT` | §5.1 | fighter is Hero |
| Hero spell status | `StatusModule::change_status_force(FIGHTER_BRAVE_STATUS_KIND_SPECIAL_LW_START, true)`; `MotionModule::set_rate(50.0)` while in that status | §5.1 | — |
| Cloud Limit | call the game's add-limit routine at `main+0x8dc160` with `(100.0, fighter, 0)` (upstream hooks this routine only to obtain a callable; the hook body calls the original unchanged) | once per load | fighter is Cloud, `LIMIT` selected |
| Joker Arsene | `FighterSpecializer_Jack::add_rebel_gauge(fighter, FighterEntryID(1), 120.0)` (the entry id argument does not matter; the game fills the gauge of the passed fighter) | once per load | fighter is Joker, `ARSENE` selected |
| Joker summon dispatch | hook on `FighterSpecializer_Jack::check_doyle_summon_dispatch`: when the original returns `FIGHTER_JACK_STATUS_KIND_SUMMON` and the fighter is buffing, return `0xFFFFFFFF` instead (no summon cutscene) | every call while buffing | Training |
| Little Mac KO gauge | `WorkModule::set_float(100.0, FIGHTER_LITTLEMAC_INSTANCE_WORK_ID_FLOAT_KO_GAGE)` | once per load | fighter is Little Mac, `KO` selected |
| Little Mac HUD meter | hook on the shared UI-update routine at `main+0x68cda0` `(fighter_entry_data*, value)`: while a load is in progress and either fighter is Little Mac and `KO` is selected, the value argument is replaced by 100 | every call | Training |
| Sephiroth Wing | `WorkModule::on_flag(FIGHTER_EDGE_INSTANCE_WORK_ID_FLAG_IS_RULE_HP)` and `on_flag(…_FLAG_SUDDEN_DEATH)` (makes the game's per-frame wing check believe it is a stamina sudden-death match); when `…_FLAG_ONE_WINGED_ACTIVATED` becomes set, `off_flag` both | §5.1 | fighter is Sephiroth, `WING` selected |
| Sephiroth screen darkening | hook on `EffectModule::req_screen`: while the fighter is buffing, the wing background hash `72422354958` (decimal, as upstream writes it; `0x10DCB5700E`) is replaced by `bg` | every call | Training |
| Wario waft | `WorkModule::set_int(seconds × 60, FIGHTER_WARIO_INSTANCE_WORK_ID_INT_GASS_COUNT)` with seconds = `gass_middle_time` (Mini), `gass_large_time` (Half), `gass_max_time` (Full) read from `param_special_lw`; `WorkModule::set_int(level, …_INT_GASS_LEVEL)` with level `FIGHTER_WARIO_GASS_LEVEL_M` / `_L` / `_FLY` | once per load | fighter is Wario, a waft level selected |
| Shulk art | `WorkModule::set_int(art, FIGHTER_SHULK_INSTANCE_WORK_ID_INT_SPECIAL_N_TYPE_SELECT)` (art ids `FIGHTER_SHULK_MONAD_TYPE_JUMP/SPEED/SHIELD/BUSTER/SMASH`), `set_int(29, …_INT_SPECIAL_N_SELECT_TIMER)`, `on_flag(…_FLAG_SPECIAL_N_SELECT)`, `StatusModule::change_status_force(FIGHTER_SHULK_STATUS_KIND_SPECIAL_N_ACTION, true)`; `MotionModule::set_rate(40.0)` while in that status | §5.1 | fighter is Shulk, an art selected |
| Wii Fit Deep Breathing | `StatusModule::change_status_force(FIGHTER_WIIFIT_STATUS_KIND_SPECIAL_LW_SUCCESS, false)`; `MotionModule::set_rate(40.0)` while in that status | §5.1 | fighter is Wii Fit, `BREATHING` selected |
| Wii Fit heal | hook on the damage-module add-damage routine at `main+0x3ff9c0` `(damage_module*, amount, kind)`: the module accessor is read from `damage_module + 0x8`; when the fighter is Wii Fit and buffing, `amount` is replaced by 0 | every call | Training |
| Min Min Power Dragon | `VisibilityModule::set_status_default(0x59a6ef56c, 0x9b7cb3e40)`, `set_status_default(0xa9ffaf181, 0xef190b4e8)`; if `ArticleModule::get_article(FIGHTER_TANTAN_GENERATE_ARTICLE_SPIRALLEFT)` is non-null: `set_status_default(0x6ec1f4d21, 0xa9aba8db6)` and `VisibilityModule::set(0x59a6ef56c, 0xadd214353)`; `WorkModule::set_int(param, FIGHTER_TANTAN_INSTANCE_WORK_ID_INT_ARM_L_BIG_FRAME)`; `on_flag(…_FLAG_DRAGONIZE_L)`; if `…_INT_REINFORCE_L_EFFECT_HANDLE_L` is non-zero `EffectModule::kill(handle, false, true)`; `EffectModule::req(0x12600df9d4, pos 0, rot 0, 1.0, 0, −1, false, 0)`, `EffectModule::set_scale(handle, (1,1,1))`, store the handle in `…_INT_REINFORCE_L_EFFECT_HANDLE_L`; `MotionModule::add_motion_partial(FIGHTER_TANTAN_MOTION_PART_SET_KIND_DRAGON, 0xc86296416, 0.0, 1.0, false, false, 0.0, true, true, false)` (all hash40 values are the game's; upstream copied the sequence from the game's own routine) | once per load | fighter is Min Min, `POWER_DRAGON` selected |
| Buff sounds | fighter sound module `play_se` at `main+0x4cf6c0`: while either record is buffing, every sound hash is replaced by `se_silent` (KO Punch's own sound is the one exception upstream could not silence from `stop_all_sound`; this hook does) | every call | Training |

## 5. Timing and frame rules

The step runs on the fighter's frame callback while its record marker is ApplyBuff (set by SPEC-state-01 §5.3 step 8d for Hero, Cloud, Joker, Little Mac, Sephiroth, Wii Fit, Shulk, Min Min, Wario). Each frame: the common silencing of §4 row 1, then the fighter-specific step, which reports **done** or **continue**. On done: clear the record's buffing flag and set the marker to NoAction. A fighter whose kind is in the list but whose option is not selected reports done on the first frame (Hero and Shulk after their own empty-selection paths; Wario when no waft level is selected).

### 5.1 Per-fighter step

- **Hero**: first frame: set buffing, spells remaining = number of selected Hero spells (0–4). Each frame: if the previous status was `SPECIAL_LW_START` a spell has just been cast → remaining −1. If remaining is 0: done after a 3-frame delay (the spell sound is late; keep silencing). Otherwise take spell index `remaining − 1` of the selected Hero spells (so they are cast last-to-first); if the current status is not `SPECIAL_LW_START`, write the spell id and force the status (the `true` argument blocks shielding out of it); if it is, play it at 50× rate. Continue.
- **Cloud** (`LIMIT`): first frame: set buffing, add 100 limit. Done after a 2-frame delay.
- **Joker** (`ARSENE`): first frame: set buffing, add 120 rebel gauge. Done after a 2-frame delay (Arsene's voice call is late).
- **Little Mac** (`KO`): first frame: set buffing, KO gauge = 100. Done after a 2-frame delay (KO sound is late).
- **Sephiroth** (`WING`): first frame: raise the two rule flags; every frame set buffing; when `ONE_WINGED_ACTIVATED` is set, clear the two flags and done (so later deaths do not respawn with Wing). Otherwise continue.
- **Wario**: first frame: draw one waft level from the selected levels (weighted random); if none, done. Else write count and level; set buffing; done the same frame.
- **Shulk**: draw one art from the selected arts (weighted random) each frame (open question 2); if none, done. Set buffing. If the previous status was `SPECIAL_N_ACTION`: done after a 3-frame delay (the "JUMP!" voice line is late). Else if the current status is not `SPECIAL_N_ACTION`: write type, timer 29, flag, force the status; else rate 40. Continue.
- **Wii Fit** (`BREATHING`): first frame set buffing. If the previous status was `SPECIAL_LW_SUCCESS`: done. Else if not in it: force the status (`false`); else rate 40. Continue.
- **Min Min** (`POWER_DRAGON`): perform the whole §4 sequence; done the same frame. (Upstream never sets the buffing flag for Min Min.)

The delay counter is shared by all buffs and is reset by the training reset (SPEC-state-06), so a delay never carries across loads.

### 5.2 Ordering against other features

- Runs after charges and the Steve state have been written (same completion frame set the marker; the first ApplyBuff frame is the next callback).
- While buffing: `handle_effect` of SPEC-state-01 §4.1 does **not** zero effect sizes for this fighter (the buff visuals must show); the sound hook silences everything; the Joker, Sephiroth, Wii Fit and Little Mac hooks above are active.
- Recording/playback triggered by the load (SPEC-state-01 §5.3 step 10) may already be running while the buff is applied; upstream does not wait (open question 3).

### 5.3 Setting changed mid-sequence

`buff_state` is re-read every frame: deselecting Hero's spells mid-cast reduces the list and the index arithmetic uses the new list; deselecting Shulk's arts ends the step; deselecting `ARSENE`/`LIMIT`/`KO`/`WING`/`BREATHING`/`POWER_DRAGON` makes the fighter report done on the next frame (Sephiroth's two rule flags would then stay set until Wing activates on its own: open question 4).

## 6. Edge cases and interactions

1. Both fighters buffable (Hero vs Shulk): each record runs its own step with its own buffing flag and remaining count; the delay counter is shared, which can shorten one fighter's delay by the other's (open question 5).
2. Leaving Training mid-buff: the buffing flags must be cleared when the session ends (upstream lists this as a future enhancement; the fork added `clear_session_state` that clears both flags, both counts and the delay counter — folded as required behaviour).
3. CPU Control / CPU-as-P1 (fork): the buffing flag and remaining count belong to the save-state record (player or CPU), never to "who is operating the fighter" (fork fix, folded; upstream keyed them by `is_operation_cpu`, which flips when Control is selected).
4. Little Mac as the opponent: the HUD meter hook forces 100 for whichever Little Mac is present, as long as `KO` is selected and a load is in progress; it does not check which fighter is being buffed.
5. Kirby wearing a buffable fighter's hat is not buffed (upstream comment: Kirby's WaitForCopyAbility would have to move after the buff).
6. Hero with zero spells selected still spends 3 frames in ApplyBuff (delay path).
7. Sephiroth already in Wing when loaded: the game's respawn clears Wing; the buff re-grants it.
8. Lobby practice: applies only during an owned restore (SPEC-state-01 §6.9).

## 7. Persistence

None beyond the menu setting (`buff_state` in the settings file, `settings.json` in the clean room; carried over).

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | see docs/resource-costs.md (W5) | ≤ fork | `tools/build/nro_size.py` per-feature build |
| per-frame work | see docs/resource-costs.md (W5) | 0 outside ApplyBuff frames; the five hooks each add one flag read per call | diag counters |
| heap allocations in hooks | upstream builds a `Vec` of Hero spells every ApplyBuff frame | 0 (iterate the bit set) | counting allocator, install line |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Hero casts every selected spell last-to-first, 3-frame tail | `state03_hero_spell_sequence` | `E-state-03-hero-four-spells` | |
| Cloud / Joker / Mac write once, 2-frame tail | `state03_single_write_two_frame_tail` | `E-state-03-cloud-limit` | |
| Sephiroth flags on, cleared once Wing activates | `state03_sephiroth_flag_lifecycle` | `E-state-03-sephiroth-wing` | |
| Wario level draw and count = seconds × 60 | `state03_wario_waft_level` | `E-state-03-wario-waft` | |
| Shulk art selection, timer 29, 3-frame tail | `state03_shulk_art_sequence` | `E-state-03-shulk-art` | |
| Wii Fit success status, no heal while buffing | `state03_wiifit_breathing_no_heal` | `E-state-03-wiifit` | |
| Min Min single-frame sequence | `state03_minmin_sequence` | `E-state-03-minmin-dragon` | |
| Unselected option → done on first frame | `state03_unselected_done_immediately` | `E-state-03-unselected` | |
| Sounds silenced while buffing; summon dispatch suppressed; wing bg replaced | `state03_hook_predicates` | `E-state-03-silent-buff` | |
| Little Mac HUD meter forced to 100 while loading with KO | `state03_mac_meter_hook` | `E-state-03-mac-meter` | |
| Buff bookkeeping keyed by record, cleared on session end | `state03_bookkeeping_by_record_and_clear` | `E-state-03-cpu-control-buff` | |
| Both fighters buffable in one load | `state03_two_buffable_fighters` | `E-state-03-hero-vs-shulk` | |

## 10. Open questions

1. Hero's and Shulk's activations are the game's own statuses played at 50×/40× rate; frame-perfect parity with upstream depends on the frame counter semantics (count starts on the first `should_delay` call). Confirm the counter service spec (shared runtime) matches: first call at count 0 starts counting and returns "delay", returns "done" once count ≥ n and resets.
2. Shulk's art is drawn from the selected arts on every ApplyBuff frame; the first frame's draw is the one written, later draws only decide "is anything selected". Recommendation: draw once per load.
3. Playback or recording started by the load can begin while the buff is still being applied (up to ~30 frames for Hero). Recommendation: delay the SPEC-state-01 §5.3 step 10 trigger until both records reach NoAction (already proposed there).
4. Deselecting `WING` mid-buff leaves Sephiroth's two rule flags set until Wing triggers on its own. Recommendation: clear the flags whenever the step ends without activation.
5. The buff delay counter is shared between the player and CPU records. Recommendation: one counter per record.
6. Min Min's sequence writes raw hash40 values copied from the game's own routine (`0x59a6ef56c`, `0x9b7cb3e40`, `0xa9ffaf181`, `0xef190b4e8`, `0x6ec1f4d21`, `0xa9aba8db6`, `0xadd214353`, `0x12600df9d4`, `0xc86296416`, parameter `0xdf05c072b`/`0xf4fd45d48`). Their names are unknown to the spec writer; the implementer should treat them as opaque constants and verify the result (left arm dragonised, effect attached) in Eden.
