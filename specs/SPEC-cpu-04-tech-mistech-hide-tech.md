# CPU tech options, missed-tech options and hidden tech animations — behavioural specification

- **Spec id:** `SPEC-cpu-04`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), 2026-11-01
- **Upstream reference read (spec writer only):** `src/training/tech.rs`, the motion hook and tech calls in `src/training/mod.rs`, `src/lib.rs` (install of the common-module hook), `src/common/mod.rs` (fighter script object lookup), `src/common/offsets.rs` (camera needles), `training_mod_consts/src/options.rs` (`TechFlags`, `MissTechFlags`, `OnOff`), `training_mod_consts/src/lib.rs` (Defensive tab) at commit `97cbe9b9` (and `077a8e43` where it differs); fork changes at `…/stages/m5i/stage/src/training/tech.rs` lines `12, 19-25, 355-372, 384-389, 415, 437, 753, 771-775`, `…/training/mod.rs` lines `202, 297-304` from `docs/provenance/provenance-files.md`
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

Shared definitions are in `SPEC-cpu-01` (mash queue, weighted picks, CPU predicate, situation predicates) and `SPEC-cpu-07` (frame counters, reset bundle).

## 1. Purpose (one paragraph)

When the practice CPU is slammed into the ground, a wall or a ceiling, it techs (in place, rolling forward, rolling backward, or not at all) the way the player chooses; when it misses a tech it gets up (neutral, attack, roll forward, roll backward) the way the player chooses, including after a banana slip or Snake's down throw; and, optionally, its tech animation, effects and player cursor are hidden after the first frames so the player can practise reacting to the start of the animation, with the fixed camera pulled to a better view on tournament stages.

## 2. User-visible behaviour

### Menu items

| Row (Defensive tab) | Setting key | Type | Values | Default |
| --- | --- | --- | --- | --- |
| Tech Options | `tech_state` | multi-select, weighted | No Tech (`NO_TECH`), Roll Forwards (`ROLL_F`), Roll Backwards (`ROLL_B`), Tech In Place (`IN_PLACE`) | all four |
| Mistech Options | `miss_tech_state` | multi-select, weighted | Neutral Getup (`GETUP`), Getup Attack (`ATTACK`), Roll Forwards (`ROLL_F`), Roll Backwards (`ROLL_B`) | all four |
| Hide Tech Animations | `tech_hide` | on/off | On, Off | Off |

Also read: Mash tab trigger "Tech Option" and "Mistech Option", `mash_state`, and the Override tab row Tech Action `tech_action_override` (weighted Action multi-select, default none).

Help texts (upstream): "Actions to take when slammed into a hard surface"; "Actions to take after missing a tech"; "Hides tech animations and effects after 7 frames to help with reacting to tech animation startup".

"Forwards" means the fighter's facing direction (the game's `passive_stand_f` / `down_forward_*` animations), "Backwards" the opposite.

### Where it is live

Training and lobby practice, CPU-operated fighters. Hide Tech Animations acts on the primary CPU body only (not Nana, see §6). The fixed-camera change applies when the Training fixed camera (camera mode 4) is selected in native Training.

### Visible outputs

The CPU's tech/getup; when hiding: the CPU model, its tech effects and its player cursor disappear from motion frame 6 of a tech/miss-tech animation until its last five frames; the miss-tech camera shake is suppressed; the fixed camera centre moves to a per-stage position.

## 3. Inputs

- Menu rows above (live).
- CPU predicate; the fork's practice interface: "practice enabled", "primary CPU body", "entry of role 1" (used to tag the hidden body).
- Game state per CPU fighter: the status the fighter's script is about to change to (hook argument), status kind, previous statuses (`StatusModule::prev_status_kind(accessor, 1)`: the second-previous status), motion frame and end frame (`MotionModule::frame`, `MotionModule::end_frame`), damage (`DamageModule::damage(accessor, 0)`).
- Transition terms: `FIGHTER_STATUS_TRANSITION_TERM_ID_PASSIVE` (ground tech possible), `…_PASSIVE_WALL`, `…_PASSIVE_CEIL`.
- The fighter script's own "is passive enabled" query (`L2CFighterCommon::is_enable_passive()` returning a Lua boolean): the script object is reached from the hook's `L2CFighterBase` argument, or from a module accessor as the pointer at `lua_module + 0x1D8` where `lua_module` is the pointer stored at `accessor + 0x190` (13.0.5 layout used by upstream).
- Parameters (`WorkModule::get_param_float(accessor, hash40("common"), hash40(name))`) for Snake's lay-down lockout: `laydown_no_action_frame`, `laydown_no_action_frame_max`, `laydown_damage_max`.
- Work float `FIGHTER_INSTANCE_WORK_ID_FLOAT_CURSOR_OFFSET_Y` (the P-cursor vertical offset; read to save, written to hide).
- Camera: the camera mode argument of the game's "change active camera" call (4 = Fixed), the fixed-camera values struct passed to the game's "set training fixed camera values" call (first field: centre `Vector3f`), the camera manager singleton and its fixed-camera centre, and the save-state spec's current stage id.
- One in-game frame counter (SPEC-cpu-07) for the lay-down lockout.

## 4. Outputs and native state touched

| Native state | How it is reached (function / field by name; `main+0x…` for 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| The status a fighter script changes to (tech) | Replace hook on the common fighter-script function `L2CFighterBase::change_status(fighter, status_kind, unk)` in the `common` script module (symbol `_ZN7lua2cpp14L2CFighterBase13change_statusEN3lib8L2CValueES2_`; installed when the `common` NRO loads; guard checked against that image). For a CPU-operated fighter in training, rewrite the two arguments before calling the original (§5 "Tech") | every script status change of the CPU | training; CPU-operated |
| Mistech getup status | `StatusModule::change_status_force(accessor, status, true)` | once per frame while in a down-wait, lay-down (after lockout) or slip-wait status (§5) | training; CPU-operated; `tech_state` non-empty |
| Roll direction of tech and getup animations | Replace hook on `MotionModule::change_motion(accessor, motion_kind, f32, f32, bool, f32, bool, bool)`, `main+0x205cb30` (leaf thunk). Substitute the motion hash before calling the original (§5 "Motion") | every motion change of the CPU | training; CPU-operated; `tech_state` non-empty |
| Model visibility | `VisibilityModule::set_whole(accessor, false/true)` | hide from motion frame ≥ 6; show in the last 5 frames and on restore | training; primary CPU; `tech_hide` On |
| Tech effects | `EffectModule::set_visible_kind(accessor, Hash40(name), false)` for `sys_nopassive`, `sys_down_smoke`, `sys_passive`, `sys_crown`, `sys_crown_collision` | each hidden frame | same |
| Player cursor offset | `WorkModule::set_float(accessor, 800.0, FIGHTER_INSTANCE_WORK_ID_FLOAT_CURSOR_OFFSET_Y)` each hidden-status frame; the saved value is written back on restore | hide / restore | same |
| Miss-tech camera shake | Replace hook on `CameraModule::req_quake_pos(accessor, quake_kind)`, `main+0x1ff0ac0` (leaf thunk; shares its 64-byte window with `req_quake` at `main+0x1ff0ab0`, hooked by the save-state spec). For a CPU-operated fighter in training in status `DOWN` with `tech_hide` On, call the original with `CAMERA_QUAKE_KIND_NONE`; else pass through | every quake request | training; CPU-operated |
| Fixed camera centre | Replace hook on the camera-manager "change active camera" function at `main+0x4ee480` (needle site; args: camera manager, camera mode, int, pointer, bool). In training, when `camera_mode == 4`, set the centre (§5 "Camera") before calling the original. Centre write: `CameraManager + 0xbd0` (`Vector3f`), where the manager is `**(main + 0x52b6f00)` (pointer to pointer) | camera mode change | training |
| Stage default fixed-camera centre | Replace hook on the training fixed-camera setup at `main+0x3158e00` (needle site; args: a pointer (unused), `*mut` values struct whose first field is the centre `Vector3f`). In training, remember the struct's centre as the stage default, call the original, then apply §5 "Camera" (L+R+A reset re-enters this path without a camera-mode change) | Training camera setup and every reset | training |

Plugin state per CPU fighter: held tech roll direction, held miss-tech roll direction; one "hidden body" record (fighter identity = battle object id + accessor address, saved cursor offset, owning entry); the stage default centre.

## 5. Timing and frame rules

### Tech (script status-change hook)

Run only for CPU-operated fighters in training. Let `requested` be the status the script asked for.

- **Ground**: if `requested` is `FIGHTER_STATUS_KIND_DOWN` or `FIGHTER_STATUS_KIND_DAMAGE_FLY_REFLECT_D`, and a tech is possible (`PASSIVE` term enabled, the second-previous status is not `FIGHTER_STATUS_KIND_CATCHED_AIR_FALL_GANON` (Ganondorf's aerial side special slam cannot be teched; the first-previous status is `CATCHED_AIR_END_GANON` for both his grounded and aerial side specials, the second-previous one tells them apart), and the script's "is passive enabled" is true): pick from `tech_state`:
  - Tech In Place → request `FIGHTER_STATUS_KIND_PASSIVE` with the second argument true;
  - Roll Forwards → `PASSIVE_FB`, true, hold tech roll direction = forward;
  - Roll Backwards → `PASSIVE_FB`, true, hold tech roll direction = backward;
  - No Tech or empty pick → leave the request unchanged (missed tech).
  If a tech was performed and the Tech Option trigger is ticked, buffer `tech_action_override` (if non-empty) else `mash_state` through SPEC-cpu-01's external entry. Ground handling ends the hook's work whether or not a tech happened (wall/ceiling are not evaluated for a ground request).
- **Wall**: if `requested` is `STOP_WALL` or `DAMAGE_FLY_REFLECT_LR`, the pick is not No Tech, `PASSIVE_WALL` term enabled and passive enabled: Tech In Place → `PASSIVE_WALL`; Roll Forwards → `PASSIVE_WALL_JUMP` (wall-jump tech); Roll Backwards → nothing (no tech). Mash as above when a tech happened.
- **Ceiling**: if `requested` is `STOP_CEIL` or `DAMAGE_FLY_REFLECT_U`, the pick is not No Tech, `PASSIVE_CEIL` term enabled and passive enabled: any other pick → `PASSIVE_CEIL`; mash as above.
- Roll the pick only when one of the three situations applies (upstream rolled on every status change of the CPU; see open question 2).

### Motion (motion-change hook)

For CPU-operated fighters in training with `tech_state` non-empty: replace `passive_stand_f`/`passive_stand_b` with the one matching the held tech roll direction (forward → `passive_stand_f`); replace `down_forward_u`/`down_back_u` and `down_forward_d`/`down_back_d` with the one matching the held miss-tech roll direction (`_u` = face-up, `_d` = face-down variants). All other motions pass through. (The Pokémon Trainer switch speed-up that upstream chains onto this hook belongs to the fighter-specific spec.)

### Mistech (once-per-frame dispatch)

For CPU-operated fighters in training with `tech_state` non-empty (upstream gate; open question 1):

- status `DOWN_WAIT` or `DOWN_WAIT_CONTINUE` (lying after a missed tech): pick from `miss_tech_state`: Neutral Getup → `DOWN_STAND`; Getup Attack → `DOWN_STAND_ATTACK`; Roll Forwards → `DOWN_STAND_FB` with miss-tech roll direction forward; Roll Backwards → `DOWN_STAND_FB`, backward; empty → nothing;
- status `LAY_DOWN` (Snake down throw): wait `lockout` frames on the lay-down counter, where `lockout = min(base + (damage / damage_max) × (max − base), max)` (truncated to whole frames) from the three `common` parameters; then as above;
- status `SLIP_WAIT` (banana and similar slips): Neutral Getup → `SLIP_STAND`; Getup Attack → `SLIP_STAND_ATTACK`; Roll Forwards → `SLIP_STAND_F`; Roll Backwards → `SLIP_STAND_B`;
- any other status: fully reset the lay-down counter.

When a status was chosen, force it (`change_status_force`) and, if the Mistech Option trigger is ticked, buffer `tech_action_override` (non-empty) else `mash_state`. Because the forced status leaves the wait status on the same frame, this fires once per missed tech.

### Hide tech (once-per-frame dispatch; primary CPU body only)

When training is active and `tech_hide` is On:

- If the body's status is `DOWN` (missed tech), `PASSIVE` (tech in place) or `PASSIVE_FB` (tech roll): on the first such frame for this body, record the hidden-body identity, the owning entry and the current cursor offset. Every such frame set the cursor offset to 800.0 (off screen). From motion frame ≥ 6: hide the whole model and the five effect kinds. When `end_frame − frame ≤ 5`: show the model again (effects stay hidden until the status ends).
- Otherwise: restore (below).

**Restore** (fork fix, required) runs on the hidden body's own frame callback and whenever the hide is no longer valid: the body's status is no longer one of the three, or `tech_hide` turned Off, or practice is no longer enabled. Restore = show the model, write back the saved cursor offset, forget the record. Also forget the record (without touching a body) when its owning entry is destroyed or the scene changes (fork lifecycle interface), so a replacement body never inherits a stale "hidden" state. Upstream never restored the cursor offset and did not restore visibility if the toggle was turned Off mid-tech; both are required here.

### Camera

- When `tech_hide` is On and the Fixed camera is selected (mode 4) or the fixed values are (re)set: set the fixed centre to the stage's value if the stage has one; otherwise leave the game's value. Values (x, y, z): Final Destination (`StageID::End`), Battlefield (`BattleField`), Small Battlefield (`BattleField_S`), Town and City (`Animal_City`), Pokémon Stadium 2 (`Poke_Stadium2`) and every Ω form (`StageID::End_*`) and Battlefield form (`StageID::Battle_*`) of every stage: `(0, 50, 230)`; Smashville (`Animal_Village`): `(0, 35, 350)`; Hollow Bastion (`Trail_Castle`): `(0, 50, 210)` (its Ω/BF forms use the default `(0, 50, 230)`).
- When `tech_hide` is Off: write back the stage default captured at the last fixed-values setup.
- Both hooks act only in training; the camera-mode hook passes through for other modes.

### Setting changes

Live. Turning `tech_hide` Off while a body is hidden restores it on the next frame; turning it On takes effect at the next tech. The camera centre follows the toggle at the next camera-mode change or reset (not instantly; open question 5).

### Reset and save states

The reset bundle resets all in-game counters (the lay-down counter). Held roll directions persist harmlessly. A save-state load that moves a hidden body out of the tech statuses triggers restore.

## 6. Edge cases and interactions

- **Ice Climbers**: both are CPU-operated and both tech/mistech per the rows (per-fighter held directions, open question 4). Hide tech applies to the primary body (Popo) only; Nana stays visible (fork behaviour; upstream hid whichever body the CPU entry resolved to, from every CPU fighter's callback).
- **Pokémon Trainer**: the CPU entry's body changes on a switch; the hidden-body record is keyed by body and entry, so a switch mid-hide forgets the old body (restore cannot reach it; the game shows the new body normally).
- **Ganondorf** aerial side special: not techable (rule above). **Kirby, Mii, Aegis, Steve, Olimar, Rosalina**: no special handling.
- **Jab locks**: a locked fighter re-enters `DOWN`; the mistech rule fires again from `DOWN_WAIT` (upstream's changelog lists this as fixed behaviour).
- **Lobby practice**: both hooks run for entry 4; the fixed-camera hooks only matter in native Training (the waiting room has no Training camera); the quake hook applies in both.
- **Hot swap**: entry destruction forgets the hidden record (fork interface); session clear resets counters.
- **Input playback**: tech decisions still apply during playback (upstream); open question 6.
- **Menu open**: paused.

## 7. Persistence

Nothing is written. Settings live in the menu JSON.

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | see docs/resource-costs.md (W5) | stage camera table as a compact match on stage-id ranges (no hash map: upstream built a 200-entry hash map on every call); state < 64 bytes | `tools/build/nro_size.py` per-feature build |
| per-frame work | see docs/resource-costs.md (W5) | mistech: one status read per CPU fighter; hide: one status read for the primary body, writes only while hidden; hooks: one status comparison each | diag counters |
| heap allocations in hooks | upstream: hash map per camera change, vector per weighted pick | 0 | counting allocator, install line |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Ground tech rewrite per option; No Tech leaves DOWN; Ganon aerial slam not techable | `cpu04_ground_tech_table` | `cpu-04-tech-each-option`, `cpu-04-ganon-side-b` | `evidence/cpu-04/tech.md` |
| Wall/ceiling tech rewrites | `cpu04_wall_ceiling_tech` | `cpu-04-wall-tech-jump`, `cpu-04-ceiling-tech` | `evidence/cpu-04/tech.md` |
| Motion substitution follows held roll directions | `cpu04_motion_substitution` | `cpu-04-roll-back-animation` | `evidence/cpu-04/motion.md` |
| Mistech per option from down-wait, slip-wait and lay-down (lockout formula) | `cpu04_mistech_table`, `cpu04_laydown_lockout` | `cpu-04-mistech-each-option`, `cpu-04-banana-slip`, `cpu-04-snake-dthrow` | `evidence/cpu-04/mistech.md` |
| Tech/Mistech triggers buffer the override or mash | `cpu04_tech_triggers_mash` | `cpu-04-tech-then-usmash` | `evidence/cpu-04/mash.md` |
| Hide from frame 6, show in last 5, cursor 800 then restored; restore on toggle Off / status interrupt / practice end | `cpu04_hide_tech_lifecycle` | `cpu-04-hide-tech`, `cpu-04-hide-tech-toggle-off-mid-tech` | `evidence/cpu-04/hide.md` |
| Quake suppressed only in DOWN with hide On | `cpu04_quake_rule` | `cpu-04-mistech-no-shake` | `evidence/cpu-04/hide.md` |
| Camera centre per stage; default restored when Off; reapplied after reset | `cpu04_camera_table` | `cpu-04-fixed-camera-bf`, `cpu-04-fixed-camera-smashville` | `evidence/cpu-04/camera.md` |
| Primary body only; Nana visible; swap forgets record | `cpu04_primary_body_only` | `cpu-04-ice-climbers-hide` | `evidence/cpu-04/ic.md` |
| Common-module hook guard; both camera needles resolve at the documented sites | `cpu04_guards` | `cpu-04-install-line` | `evidence/cpu-04/install.md` |

## 10. Open questions

1. **Mistech gated on Tech Options.** Upstream runs the mistech logic only when `tech_state` is non-empty, so clearing Tech Options silently disables Mistech Options too. Recommendation: gate mistech on `miss_tech_state` alone.
2. **RNG on every status change.** Upstream picks a tech option on every script status change of the CPU, even when no tech decision is pending. This spec requires picking only when a tech situation applies. Confirm (pure cost change, no behaviour change).
3. **Ceiling and wall roll options.** A Roll Backwards pick on a wall does not tech, and any non-No-Tech pick on a ceiling techs in place. Confirm these mappings are wanted or whether Roll Backwards should also wall-jump.
4. **Per-fighter state.** Held roll directions are global upstream (shared by Popo and Nana). This spec asks for per-fighter state; confirm.
5. **Camera centre timing.** The fixed-camera centre changes only when the camera mode changes or the fixed values are reset; toggling Hide Tech Animations while already in the fixed camera has no visible effect until then. Recommendation: also apply the centre on the toggle change (one write).
6. **Playback.** Tech and mistech decisions override the CPU during input playback (a recorded tech cannot be replayed). Recommendation: yield to playback like the other CPU behaviours; decide together with SPEC-cpu-02 question 7.
7. **Hidden effects on restore.** Upstream re-shows the model but never re-enables the five effect kinds; the game appears to reset effect visibility per status. If an Eden row shows a lingering hidden effect, add the five `set_visible_kind(.., true)` calls to restore.
8. **Camera manager addressing.** The manager pointer (`main+0x52b6f00`) and the centre offset (`+0xbd0`) come from upstream comments for 13.0.5 and are not covered by a needle; they need a guard of their own (compare the value read at the fixed-values hook against the struct the game passes) before the first write.
