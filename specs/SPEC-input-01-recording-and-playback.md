# Input recording and playback — behavioural specification

- **Spec id:** `SPEC-input-01`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude (Fable 5.1), spec-writer for area `input`, 2026-11-01
- **Upstream reference read (spec writer only):** `src/training/input_record.rs`, the recording dispatch in `src/training/mod.rs`, the playback callers in `src/training/mash.rs`, `src/training/ledge.rs`, `src/training/save_states.rs`, the option types in `training_mod_consts/src/options.rs` and the Input Recording tab in `training_mod_consts/src/lib.rs`, all at commit `97cbe9b9` (`077a8e43` changes nothing in these files except an early return in the once-per-frame dispatch, folded in §5). Fork changes at `…/stages/m5i/stage/src/training/input_record.rs` (stage lines 12–14, 74–88, 110, 189–199, 223–255, 263–301, 303–330, 337, 420–590, 592–626, 628–673, 676–702, 704–748, 752–757, 778–836, 838–1009, 1037–1040) and `…/stage/src/training/mod.rs` (lines 50–54, 78–109, 242–337, 1083–1161), classified in `INDEX-input.md`.
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

The player records up to 600 frames of their own controller inputs into one of five slots while the CPU performs them live, then has the CPU replay a slot on demand: by a button chord, when a save state is loaded, when a mash trigger fires, or as a ledge option. Playback mirrors horizontal inputs when the CPU faces the other way, can wait for the right moment out of hitstun or shieldstun, can loop, and can be cut short by a mash trigger. The CPU is tinted and a HUD notice is shown while the recorder owns it. The fork's named-macro library, replay import and clip editor sit on top of this core and are carried over as they are; §9 states the contract they need.

## 2. User-visible behaviour

### 2.1 Menu items (Input Recording tab, id `input`)

| Item | Setting id | Type | Values | Default |
| --- | --- | --- | --- | --- |
| Recording Slot | `recording_slot` | single choice | Slot 1 … Slot 5 | Slot 1 |
| Recording Trigger | `record_trigger` | single choice | Button Combination, Save State Load | Button Combination |
| Recording Duration | `recording_duration` | single choice | 60, 90, 120, …, 600 frames (step 30) | 150 |
| Recording Crop | `recording_crop` | On/Off | | On |
| Playback Button Slots | `playback_button_slots` | multi choice, weighted (weights 1–5) | Slot 1 … Slot 5 | Slot 1 |
| Playback Hitstun Timing | `hitstun_playback` | single choice | As Hitstun Ends, As Hitstop Ends, As Hitstop Begins | As Hitstun Ends |
| Save State Playback | `save_state_playback` | multi choice, weighted | Slot 1 … Slot 5 | none |
| Playback Mash Interrupt | `playback_mash` | On/Off | | On |
| Playback Loop | `playback_loop` | On/Off | | Off |
| **Settings override playback** (draft 9, N4; advanced) | `settings_override_playback` | On/Off | | Off |
| **Replay capture** (draft 9, menu question 7; Named macros group) | `replay_capture` (writes `replay_capture.ini` `enabled=on/off`; the Deluxe Plugins page row stays in sync) | On/Off | | Off — the INI is the file of record and an absent INI means off, as the fork's `DEFAULT_CAPTURE_INI` (lead correction 2026-11-02; an imported INI keeps its value) |

Help texts (one line each, shown under the item): "Choose which slot to record into"; "Whether to begin recording via button combination or upon loading a Save State"; "How long an input recording should last in frames"; "Remove neutral input frames at the end of your recording"; "Choose which slots to playback input recording upon pressing button combination"; "When to begin playing back inputs when a hitstun mash trigger occurs"; "Choose which slots to playback input recording upon loading a save state"; "End input playback when a mash trigger occurs"; "Repeat triggered input playbacks indefinitely".

Although Recording Trigger is a single choice, the behaviour is defined by membership: recording by chord is available when the selection contains Button Combination; recording on save-state load happens when it contains Save State Load.

Settings owned by other specs that feed this one: the **Input Record** and **Input Playback** chords (SPEC-input-04; defaults ZR + D-pad Down and ZR + D-pad Up); the mash actions **Playback Slot 1–5** in Mash Toggles, Followup Toggles and every override (mash spec); the ledge options **Playback Slot 1–5** and Ledge Delay (defensive spec); save-state load (save-state spec).

**Settings override playback (N4, decided 2026-11-01).** Off (default): a playing recording wins over the CPU settings that would otherwise act on the possessed CPU, namely DI, SDI, air-dodge direction (SPEC-cpu-02), tech and mistech (SPEC-cpu-04) and throws (SPEC-cpu-06); they apply again when the playback ends. On: those settings override the recording, as upstream does (§5.6 lists what the recorder already defers). The row is *advanced*: collapsed by default on the console. It is read live; changing it mid-playback takes effect on the next frame. **Replay capture** is a menu toggle that writes the fork's `replay_capture.ini` (one small INI write when the row changes) so the menu, the INI and the Deluxe page always agree (the INI file is imported, SPEC-core-08 §4).

Carried over from the fork, same tab, not re-specified here (interface in §9): Macro Source (`macro_source`: Numbered Slots / Named Library, default Numbered Slots), Fighter Prompt (`fighter_prompt`, On), and the action rows Named Library, Save Slot to Library, Edit Placement, Replay Window.

### 2.2 Chords (live in Training; in lobby practice while the practice session is active, §6.7)

- **Input Record** chord: starts a recording into the Recording Slot (only when Recording Trigger contains Button Combination). Pressing it again during a recording restarts the recording from the lockout phase (the slot is cleared again).
- **Input Playback** chord: plays one slot drawn at random (weighted) from Playback Button Slots. If both chords pass in the same frame, playback wins and the record request is dropped.
- Both chords are ignored while a save-state load is in progress (fork fix, §6.4).

### 2.3 Visible outputs

| Phase | CPU model tint (RGB, colour-blend) | HUD notification (header "Input Recording") | Lasts |
| --- | --- | --- | --- |
| Lockout | blue (0, 0, 1) | "Lockout", text colour (8, 8, 200, 255) | re-posted every frame of the phase, 60-frame length |
| Standby | magenta (1, 0, 1) | "Standby", text colour (200, 8, 200, 255) | same |
| Recording (CPU possessed) | red (1, 0, 0) | "Recording", text colour (200, 8, 8, 255) | same |
| Playback | none | "Playback", text colour (0, 0, 0, 255) | posted on playback frames 0 and 1 with a length equal to the playback length in frames |

The notification queue and its drawing are SPEC-input-05. The tint is applied through the fighter's model module colour-blend call (§4) every frame of the phase; nothing clears it explicitly (open question 10).

While the recorder owns the CPU (lockout, standby, recording, playback), P1's controller does not move P1: during lockout and recording P1's mapped output is replaced by neutral; during playback P1 controls P1 normally.

## 3. Inputs

- Menu settings of §2.1, plus `playback_mash`, `recording_crop`, the two chords, Macro Source (fork).
- P1's final mapped controller output for the frame: the game's mapped-input record (§3.1), taken after input delay has been applied (SPEC-input-03) so recordings are of delayed inputs.
- CPU game state, read each CPU control frame: facing (posture module `lr`, ±1.0), status kind, situation kind, whether in hitstop (stop module `is_stop`), whether a status transition term is enabled (work module `is_enable_transition_term`), fighter kind (Shulk check), Shulk's dial flag (`FIGHTER_SHULK_INSTANCE_WORK_ID_FLAG_SPECIAL_N_CIRCLE_MENU`), the cliff-wait rest timer (`FIGHTER_STATUS_CLIFF_WORK_INT_CATCH_REST_TIME`), entry id (`FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID`), Nana kind (`FIGHTER_KIND_NANA`).
- Hitstun and shieldstun predicates: in hitstun = status in the half-open range [`FIGHTER_STATUS_KIND_DAMAGE`, `FIGHTER_STATUS_KIND_DAMAGE_FALL`); in shieldstun = status is `FIGHTER_STATUS_KIND_GUARD_DAMAGE`, or previous status was `GUARD_DAMAGE` and current is `FIGHTER_STATUS_KIND_GUARD_OFF`.
- Mash queue state (mash spec): whether the queued action is a Playback Slot and which slot; the request to reset the queue head.
- Save-state state (save-state spec): "load in progress", "both fighters finished restoring", the saved situation kind.
- Fighter resolution: the module accessor for P1 (entry 0) and for the CPU (entry 1 in Training; entry 4 in the waiting room, §6.7), resolved every time from the fighter manager singleton (`_ZN3lib9SingletonIN3app14FighterManagerEE9instance_E`) → fighter entry → current fighter id → module accessor, validated (fork: non-null, object id matches, is a fighter, entry id matches). No module accessor pointer is retained across frames.

### 3.1 The per-frame record (`MappedInputs`, the game's final mapped input, 8 bytes)

| Field | Type | Meaning |
| --- | --- | --- |
| buttons | u32 bit set | ATTACK 0x1, SPECIAL 0x2, JUMP 0x4, GUARD 0x8, CATCH 0x10, SMASH 0x20, JUMP_MINI 0x40, CSTICK_ON 0x80, STOCK_SHARE 0x100, ATTACK_RAW 0x200, APPEAL_HI 0x400, SPECIAL_RAW 0x800, APPEAL_LW 0x1000, APPEAL_SL 0x2000, APPEAL_SR 0x4000, FLICK_JUMP 0x8000, GUARD_HOLD 0x10000, SPECIAL_RAW2 0x20000 |
| lstick_x, lstick_y | i8 each | left stick, −127…127 (the game's mapped range; 120 units = full deflection for clamping, see §4) |
| rstick_x, rstick_y | i8 each | right stick, same units; recorded and used for the neutral test and the input display only — never injected into the CPU (§4, rule R6) |

A frame is **neutral** when its buttons are empty or exactly FLICK_JUMP, and every stick axis satisfies |axis × (1/120), clamped to ±1| < 0.2 (that is |axis| < 24).

### 3.2 Slot storage (in memory, 24,000 bytes)

Five slots × 600 frames × 8 bytes, statically allocated; a per-slot length (initially 60 for every slot, even never-recorded ones); per slot the facing (±1.0) and the starting status captured at recording start (fork fix, §6.1; upstream kept one shared pair). The slot capacity 600 equals the largest Recording Duration, so a recording can never exceed its slot. Slot contents are not persisted (§7).

## 4. Outputs and native state touched

All addresses are offsets from the 13.0.5 main module text base; every site needs the playbook's version guard, 64-byte guard and card scan before it is chosen.

| Native state | How it is reached (function / field by name; `main+0x…` for 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| P1's final mapped input (the `out` record of §3.1) | Hook on the game's final-input-mapping function, `main+0x1750dd0` (8-instruction needle, prologue `sub sp, sp, #0x80; str x23, [sp, #0x40]; stp x22, x21, [sp, #0x50]; …`), arguments: controller-mapping table pointer, player index (i32; 0 = P1), `out` pointer, a struct whose field at +0x10 points at the game's `Controller` (SPEC-input-04 §3), a bool. The hook calls the original first, then runs the feature chain of §5.1; the recorder is last. Writes: during Recording the frame is copied into the slot at the current frame index and `out` is set to neutral; during Lockout `out` is set to neutral. | Each P1 input poll | Training or active practice session; the hook resolves exactly at the documented site; `player_idx == 0` |
| CPU controller data after AI assignment: the control struct reached through the second pointer (`+0x8`) of the argument array of `main+0x2da180` (needle `sub sp, sp, #0x180; str d14, [sp, #0xe0]; stp d13, d12, [sp, #0xf0]; …`). Fields written: buttons (u32 at +0xC), stick_x (f32 at +0x10), stick_y (f32 at +0x14), clamped_lstick_x (f32 at +0x40), clamped_lstick_y (f32 at +0x44). Not written: +0x50/+0x54 (misnamed "clamped right stick" upstream; the fork's native review identified them as rumble state), the vtable, controller_index (+0x8), padding. | Hook after the original returns; only when the recorder is Recording or in Playback (§5.2) | Each CPU control frame, per CPU body | Pointer non-null and readable (fork: validated child read); the struct is the **CPU entry's own controller** (fork: equals the pointer stored at fighter-entry + 0x4178 of the CPU entry; this excludes Nana and extra CPUs — §6.2); Training or practice |
| CPU cliff-wait rest timer `FIGHTER_STATUS_CLIFF_WORK_INT_CATCH_REST_TIME` ← 200 | Work module `set_int` on the CPU | In the CPU control hook when Recording/Playback, the CPU status is `FIGHTER_STATUS_KIND_CLIFF_WAIT`, the recorder is in Lockout or Standby, and the timer is < 50 | Per CPU control frame | CPU accessor valid |
| CPU model colour blend | Model module `set_color_rgb_impl` (symbol `_ZN3app8lua_bind31ModelModule__set_color_rgb_implEPNS_26BattleObjectModuleAccessorEfffNS_16MODEL_COLOR_TYPEE`, args: accessor, r, g, b, `MODEL_COLOR_TYPE_COLOR_BLEND`) | Every frame of Lockout / Standby / Recording, on the body whose entry id is the CPU entry | Per-fighter post-frame | CPU accessor valid |
| HUD notification queue (plugin state, SPEC-input-05) | `clear("Input Recording")` then post | Same frames as the tint; playback frames 0 and 1 | — | — |
| Per-fighter post-frame dispatch | The fighter vtable `on_frame` slot (`.data` `main+0x4f80568`, original function `main+0x614630`) is replaced by a wrapper that calls the original and then the modpack's per-fighter work; the recorder's end-of-recording check and the tint run there for the CPU body | Per fighter per frame | Slot currently holds the documented original (fork: logged and skipped otherwise); only the **primary CPU body** runs the recorder (entry = CPU entry, kind ≠ Nana, accessor equals the resolved CPU accessor — fork fix §6.2) |

Reads only (no write, no guard beyond validity): posture `lr`, status kind, previous status kind, situation kind, `StopModule::is_stop`, `is_enable_transition_term`, `utility::get_kind`, `WorkModule::is_flag` (Shulk dial), `get_int` entry id and rest timer.

Rules for every write above: R1 never write when the accessor failed to resolve (upstream aborted the process with an "expect" here; the fork returns early — fold in); R2 never write outside Training/practice; R3 the CPU control struct is written only when it belongs to the CPU entry; R4 the hook never allocates; R5 the game's RNG is never drawn by this feature (slot randomisation uses the menu's weighted pick, which is the game's `sv_math::rand("fighter")` — see open question 12); R6 the right-stick bytes of a frame are never written into the control struct.

## 5. Timing and frame rules

### 5.1 Order inside the P1 input poll

After the original mapping: dev-config hot reload → mod menu input (empties `out` while the menu is open and for 15 frames after it closes) → chord detection (SPEC-input-04) → [fork: named-macro stop chord; character-cycle chords] → input delay (SPEC-input-03) → input display logger (SPEC-input-02) → **recorder** (this spec). Consequences: recordings and the display see delayed inputs; chords are detected on the raw controller, not on `out`; while a menu owns input a recording stores neutral frames (§6.9).

### 5.2 Order inside a game frame (as observed in the fork; implementers confirm with the diag build)

1. P1 input poll (§5.1) stores frame *k* when Recording.
2. CPU control hook (per CPU body, after the AI wrote its controls): lockout countdown; mash-playback start check; inject frame *k* of the active slot; advance the frame index (rules below); buffer-frame handling.
3. Per-fighter post-frame (CPU body): chord consumption, recording-end check, tint and notification.

### 5.3 State machine

Recorder state ∈ {None, Pause, Record, Playback}; possession ∈ {Player, Lockout, Standby, Cpu}; frame index; lockout counter; buffer counter; current length; recorded facing; current facing; starting status; current record slot; current playback slot.

**Start recording (chord or save-state trigger):** fail silently if the CPU accessor is missing. Set current record slot = Recording Slot (re-read every frame from the menu). Clear that slot to all-neutral; slot length = Recording Duration; current length = duration; state = Pause; frame = 0; possession = Lockout; lockout counter = 30; buffer = 0; recorded facing = current facing = CPU `lr`; capture the slot's start snapshot (facing, status; fork also kind/costume/situation/speeds for export).

**Lockout (state Pause):** each CPU control frame: if counter > 0 decrement; when it is 0 → state Record, possession Standby. P1's `out` is neutral throughout (P1 cannot act; the CPU is AI-controlled and tinted blue). The 30-frame length exists so that a shield drop before the chord cannot leak into the recording (upstream comment; open question 8 asks whether 30 is right for ledge recordings).

**Standby (state Record, possession Standby):** every poll stores P1's frame into slot[0]; every CPU control frame injects slot[0] into the CPU and does **not** advance the frame index. On the first poll that finds slot[0] non-neutral: frame = 1, possession = Cpu. On the poll where frame == 1 the CPU's current status is captured as the slot's **starting status** (it is the status that resulted from frame 0). Standby at the ledge: see the rest-timer write in §4.

**Recording (possession Cpu):** poll stores P1's frame at slot[frame] and neutralises `out`; CPU control injects slot[frame], then advances frame while frame < length − 1. End detection (post-frame): when state is Record or Playback and frame ≥ length − 1: possession = Player; if the mash queue head is a Playback Slot, pop it (mash `reset`); if Record and Crop is On and frame > 0: let n = frame; while n > 0 and slot[n−1] is neutral, n −= 1; slot length = current length = n. Then frame = 0; if Playback and Loop is On → start playback of the current playback slot again (re-reads the CPU facing); else state = None.

Known upstream off-by-one (open question 1): with duration L the recorder stores frames 0…L−2 (the poll never reaches index L−1 because the end fires when the index becomes L−1), so a recording holds L−1 frames; and playback injects frames 0…n−2 of a slot of length n, so the last stored frame is never played. The fork kept this for the numbered slots ("historical") and corrected it for named macros (full-length bounds).

**Playback(slot):** refused while state is Pause (lockout) with a warning log; refused when no slot is given; refused (fork) when slot ≥ 5 or the slot length is 0 or > 600; refused when the CPU accessor is missing. Otherwise: current length = slot length; current playback slot = slot; state = Playback; possession = Player; frame = 0; buffer = 0; current facing = CPU `lr` now; (fork) recorded facing and starting status = that slot's captured pair. P1 regains control. Each CPU control frame injects slot[frame] and advances while frame < length − 1; end as above. Starting playback while Recording abandons the recording (the half-filled slot keeps length = duration; neutral tail) — document, do not prevent.

**Injection (Record or Playback):** mirror = recorded facing × current facing (±1); if the CPU is Shulk and the dial flag is set, mirror = 1. Written values: buttons = frame.buttons; stick_x = mirror × lx / 127; stick_y = ly / 127; clamped_lstick_x = mirror × clamp(lx/120, ±1) with |value| < 0.2 → 0; clamped_lstick_y likewise without mirror. If 0 < buffer ≤ 3 the injected frame is neutral instead (so a buffered press reads as a press, not a hold). After writing: if buffer > 0, buffer −= 1 (frame does not advance); else advance as above unless possession is Standby. Open question 2 concerns the /127 versus /120 scale.

**Ledge playback (`playback_ledge(slot)`, called by the ledge feature):** playback(slot), then buffer = 5, or 4 when the CPU status is `FIGHTER_STATUS_KIND_CLIFF_CATCH`. Effect with buffer 5: frame 0 injected twice, neutral three times, frame 0 once more, then frame 1 onward.

**Stop (`stop_playback`):** state None, frame 0, possession Player, (fork fix) buffer 0 and lockout counter 0, cancel any named motion. Callers: save-state load request, mash interrupt, session end, hot swap, lobby quick reset, named layer.

### 5.4 Mash-triggered playback (every CPU control frame while state is None)

Preconditions: not recording; the mash queue head is a Playback Slot action; not already in Playback. Then, by CPU status:

| CPU situation | Start playback when |
| --- | --- |
| In hitstun, Playback Hitstun Timing = As Hitstop Begins | immediately |
| In hitstun, As Hitstop Ends | the CPU is not in hitstop |
| In hitstun, As Hitstun Ends | the transition term derived from the starting status (table below) is enabled |
| In shieldstun | the derived transition term is enabled |
| Otherwise | the derived transition term is enabled |

Starting status → transition term: `FIGHTER_STATUS_KIND_ATTACK_AIR` → `CONT_ATTACK_AIR`; [`ESCAPE_AIR`, `ESCAPE_AIR_SLIDE`) → `CONT_ESCAPE_AIR`; `SPECIAL_HI` → `CONT_SPECIAL_HI`; `JUMP_SQUAT` → `CONT_JUMP_SQUAT`; `JUMP_AERIAL` → `CONT_JUMP_AERIAL`; `ESCAPE` → `CONT_ESCAPE`; [`ESCAPE_F`, `ESCAPE_B`) → `CONT_ESCAPE_F`; `CATCH` → `CONT_CATCH`; anything else → `CONT_SPECIAL_S` (an acknowledged placeholder upstream). All terms are `FIGHTER_STATUS_TRANSITION_TERM_ID_…`. The two ranges are half-open upstream, so `ESCAPE_AIR_SLIDE` and `ESCAPE_B` fall into "anything else" (open question 3). Upstream uses the shared starting status (from the last recording); with per-slot capture the slot that is queued must be used.

When playback starts this way the queued slot is used; at the end the mash head is popped so followups proceed.

### 5.5 Setting changes mid-flight

- Recording Slot changes while recording: the record slot is re-read every frame, so the remainder of the recording lands in the new slot while its length and start snapshot were set for the old one (open question 4; recommendation: latch the slot at lockout).
- Recording Duration / Crop: duration is latched at lockout; crop is read at the end.
- Playback Loop: read at each end; turning it off ends the loop at the current pass.
- Hitstun Timing: read each frame.
- Macro Source (fork) switches the whole chord path to the named layer (§9).

### 5.6 Other features that defer while the recorder owns the CPU (contract for their specs)

**Draft 9 (N4):** the deferral list below is the default (*Settings override playback* Off). The CPU-setting overrides that upstream applied during playback (DI, SDI, air-dodge stick, tech, mistech, throws) join it: while `is_playback()` they return the game's original value, unless *Settings override playback* is On, in which case they act as upstream did. Their owning specs (SPEC-cpu-02, -04, -06) state the same rule.

`is_playback()` is true in states Record **and** Playback; `is_recording()` is true only in Record; `is_standby()` is true in Lockout or Standby. While `is_playback()`: mash aerial-kind override, attack-angle stick override, ledge option forcing, shield automation (also while standby), full-hop and fast-fall automation all return the game's original value. The ledge feature additionally disables the CPU's cliff-climb transition while its chosen option is a Playback slot that has not started.

## 6. Edge cases and interactions

1. **Per-slot facing and starting status (fork fix, fold in).** Upstream keeps one recorded facing and one starting status for all slots; playing slot B after recording slot A facing the other way mirrors B wrongly and uses A's hitstun-cancel term. Each slot stores its own pair, captured at lockout (facing) and on frame 1 (status); playback uses the played slot's pair. The fork implements this by parking/restoring the shared statics around numbered playback because the same statics also serve named macros; the clean room should store the pair per slot and pass it to the injector directly.
2. **Ice Climbers as CPU (fork fix, fold in).** The CPU control hook runs for Popo and for Nana; upstream wrote the recorded inputs into both bodies and advanced the frame index twice per game frame, and its post-frame recorder work and in-game frame counters ran once per body. Fold in: inject only into the controller that belongs to the CPU entry (fighter-entry + 0x4178); run the recorder's post-frame work and tick in-game counters only for the primary CPU body (entry = CPU entry, kind ≠ Nana, accessor equals the resolved CPU accessor). Nana follows Popo through the game's own partner AI. P1 as Ice Climbers: P1's Nana never participates in chord handling or recording (entry 0, kind Nana is excluded).
3. **Pokémon Trainer.** The CPU accessor is resolved every frame through the entry's *current* fighter id, so a switch during playback continues injecting into the new Pokémon with the same frame index, facing pair and starting status. A switch during Standby or Recording is possible (the recorded input may be the switch). No special handling upstream or fork; open question 5.
4. **Save-state load (save-state spec drives this).** On a load request the recorder is stopped. Both chords are consumed and ignored while a load is in progress (fork fix: a held playback chord otherwise restarted the recorder while the fighters were being repositioned). When restoration finishes: if Recording Trigger contains Save State Load → start recording; else if Save State Playback has a selection and the saved situation is not the ledge → play a weighted random selected slot (ledge playbacks must be configured as a ledge option instead). Upstream runs this in each fighter's restore callback, so it fired once per fighter in the same frame; the fork notes the same and queues named work once when **both** fighters have finished restoring — do the same for numbered slots (open question 6).
5. **Hot swap mid-recording (fork, carry over).** While the character loader is busy, "training mode" reads false and the practice session is retired, which calls stop. A recording or playback in progress is cancelled; the slot keeps whatever was stored. Starting a recording or playback requires both fighters present (the fork's practice `acquire`).
6. **CPU-as-P1 (fork feature, carry over).** When the native "CPU Behavior = Control" operator or the fork's P1-CPU-control option drives P1, the recorder still records P1's *human controller* mapped output (idx 0), not the AI's controls; the CPU control hook writes only the CPU entry's controller, never P1's. Recording with an AI-driven P1 therefore records an idle controller. Open question 7.
7. **Lobby practice (fork, carry over).** In the waiting room the CPU entry is 4 and only entries 0 and 4 take part; chords, recording and playback are live only while the practice session is active and the native waiting UI is not busy (dialog/pause); the L+R+A quick-reset chord in the lobby stops playback. The tint and notification target the body whose entry id equals the CPU entry (4 there).
8. **Menu open during recording or playback.** The mod menu empties P1's `out` (so the recording stores neutral frames and the CPU stands still) and blocks chords; playback continues and chords stay blocked. The vanilla pause menu pauses fighter frames (the game does not advance) and blocks chords. Open question 9.
9. **Recording longer than the slot** cannot happen (duration ≤ capacity). A named recording longer than 600 frames cannot be copied into a slot (fork refusal "Recording is N frames; a Record Slot holds 600").
10. **Playing a never-recorded slot** plays 60 neutral frames (the initial length) during which the CPU does nothing; a slot cropped to length 0 must be refused (upstream would underflow its end check — fold in the fork's length check). Open question 11.
11. **L+R+A reset in Training** resets mash and counters but does not stop the recorder upstream; the CPU is repositioned while playback continues. Open question 13.
12. **Replay preview and quiescing capture (fork, carry over).** While the clip editor previews, the recorder's completion ignores loop/slot/record-on-load settings and reports completion to the preview; while the capture browser settles the CPU, the control hook writes neutral controls and does nothing else.

## 7. Persistence

- Numbered slots: not persisted; cleared at lockout; lost when the game exits. (Open question 14 asks whether to persist them as the fork's named format.)
- Menu settings: in the menu file (menu spec).
- Fork named library (carried over as-is; shape stated for the contract): one `<id>.json` per recording under `sd:/ultimate/TrainingModpack/macros/` in the fork; in the clean room `sd:/ultimate/TrainingRooms/macros/` (decided, SPEC-core-08; open question 15), `selected.txt` holding the selected id; compact JSON of `{schema_version: 1 | 2, name (≤ 48 chars, no control characters), game_version (≤ 32 bytes), start: {fighter_kind: i32, costume: u32, facing: f32 (±1.0), situation: i32, status: i32, speed_x: f32, speed_y: f32}, mii?: exact Mii configuration (schema 2 only), placement: {delta_x: f32, delta_y: f32, momentum: "preserve_recorded" | "at_rest"}, frames: [{buttons: u32, lstick_x, lstick_y, rstick_x, rstick_y: i8}]}`, 1–36,000 frames, file ≤ 8 MiB, written atomically via `<id>.json.tmp` and `<id>.json.bak`. A missing or malformed file is skipped with its reason listed; it is never rewritten.

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | 24,000 B static slot table (5 × 600 × 8), "the largest static object" (`docs/REVISION32_SMALL_PROFILE.md`); code size not isolated — see `docs/resource-costs.md` (W5) | ≤ 24 KB data; code measured per feature | `tools/build/nro_size.py` per-feature build |
| per-frame work | one 8-byte copy per P1 poll; one control-struct write per CPU control frame; post-frame: slot re-read, end check, tint call and notification re-post every frame of a phase | same minus the per-frame notification re-post (post once per phase change) | diag counters |
| heap allocations in hooks | upstream ≥ 3 per frame while in Lockout/Standby/Recording (two notification strings and a queue push each frame; `docs/resource-costs.md` (W5) to confirm) | 0 | counting allocator, install line |

## 9. Contract for the carried-over named-macro layer

The fork's `training_mod_macros` crate, `src/training/named_macros.rs`, `src/replay_*.rs` and `src/training/ui/clip_editor.rs` are carried over unchanged. They need the following from the clean-room core (names are the fork's; the core may rename them behind an adapter).

**Types.** The frame record of §3.1 converts losslessly to and from the layer's `InputFrame {buttons: u32, lstick_x, lstick_y, rstick_x, rstick_y: i8}`; the layer's `StartState {fighter_kind: i32, costume: u32, facing: f32, situation: i32, status: i32, speed_x: f32, speed_y: f32}` is produced by the core's start snapshot (fighter kind, colour work int `FIGHTER_INSTANCE_WORK_ID_INT_COLOR`, posture `lr`, situation, status, kinetic sum speeds x/y of `KINETIC_ENERGY_RESERVE_ATTRIBUTE_MAIN`).

**Core → layer queries, consulted at the points stated.** `source_is_named()` (Macro Source = Named Library; consulted when a chord passes: playback chord → `request_playback()`; record chord → start a named lockout if idle, or finish the named capture if recording/standby); `capture_active()` / `playback_active()` / `using_frames()` (whether the named buffer, not a slot, is the frame source); `has_captured_frame(i)` (the control hook writes neutral and does not advance until frame 0 exists); `store_frame(i, &frame)` (replaces the slot write during named capture; the first poll also takes the start snapshot `capture_start(cpu)` and resets both facings); `frame_at(i) -> frame` (replaces the slot read; also used by the neutral test); `prepare_playback_start(cpu) -> bool` (called in the control hook before input 0 of a named playback; false means write neutral controls and return — the layer repositions the CPU and may stop the playback itself); `begin_playback() -> Option<{facing, status, len}>` / `begin_preview(recording) -> {facing, status, len}` (the core sets its length, facing pair and state Playback from these); `end_playback()`, `cancel_motion()`; `remember_legacy(facing, status)` / `take_legacy()` (the layer parks and restores the core's shared facing/status pair around named runs — unnecessary once the pair is per slot, but the functions must still exist); `mii_at_lockout(kind)` (Mii identity capture at numbered lockout); `snapshot_start(cpu)`.

**Core entry points the layer calls.** `stop_playback()`; `start_named_lockout()` (lockout with length 36,000 and possession Cpu instead of Standby when the lockout ends); `playback_named() -> bool`; `playback_preview(recording) -> bool`; `export_named(slot) -> Result<(frames, StartState, Option<Mii>), &str>` with refusals "Record Slot is out of range", "Record into this slot first" (no capture or length 0/> 600) and the Mii refusal text; `import_named_slot(slot, &frames, start, mii) -> Result<(), String>` replacing one slot (1–600 frames) or changing nothing, with the messages "Record Slot is out of range. Nothing was changed.", "Recording is empty. Nothing was changed.", "Recording is N frames; a Record Slot holds 600. Nothing was changed."; read access to the current frame index and write access to the current facing (the layer sets the facing at a restored pose).

**Named stop chord ordering.** The layer needs the record chord evaluated **before** input delay, display and the recorder so that the stop press is consumed and not recorded; the fork does this with a hook point right after chord detection that empties `out` and clears the delay queue for that poll.

**Save-state and preview hooks.** `save_states::is_loading()`, `request_macro_restore()`, `cancel_pending()`; `replay_preview::{occupied, can_emit(cpu), complete, progress(frame)}`; `replay_capture::{quiescing_capture, loading, panel_visible}`. The core's completion handler, when the preview is occupied, stops at frame ≥ length and calls `complete()` without consulting loop/record-on-load.

**Events (fork diagnostics, `diag` feature only).** `[PracticeRecording] record_lockout cpu_kind=… frames=…`, `recording_complete slot=… frames=…`, `playback_shortcut player_kind=…`, `stop state=… frame=…`, `shortcut_ignored_during_state_load playback=… record=…`, `named_record_lockout …`, `named_playback_complete …`. The shipped build emits none of them.

## 10. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Menu items, values and defaults (§2.1) | `input::menu_defaults_match_spec` | IN01-E01 menu screenshot | `evidence/input/IN01-E01.md` |
| Record chord → lockout 30 frames → standby → recording; P1 neutralised during lockout and recording | `input::recorder_lockout_then_standby_then_cpu` | IN01-E02 | `evidence/input/IN01-E02.md` |
| Standby ends on first non-neutral frame; starting status captured on frame 1 | `input::standby_ends_on_non_neutral`, `input::starting_status_captured_frame_one` | IN01-E03 | |
| Neutral test thresholds (buttons empty or FLICK_JUMP; |axis| < 24) | `input::neutral_frame_rules` | — | |
| Crop removes trailing neutral frames; length bounds; zero-length refusal | `input::crop_trims_trailing_neutral`, `input::playback_refuses_empty_slot` | IN01-E04 | |
| Off-by-one bounds decision (open question 1) | `input::frame_bounds_store_and_play_full_length` | IN01-E04 | |
| Playback chord picks a weighted slot; playback refused during lockout | `input::playback_slot_pick_and_lockout_refusal` | IN01-E05 | |
| Mirroring by facing product; Shulk dial exception | `input::mirror_multiplier_and_shulk_dial` | IN01-E06 (Shulk) | |
| Stick scaling and clamping of injected values (§5.3, open question 2) | `input::injected_stick_values` | IN01-E06 | |
| Right-stick bytes never written to the control struct (R6) | `input::control_write_touches_only_verified_fields` | IN01-E07 memory check | |
| Ledge playback buffer schedule (5 / 4 frames) | `input::ledge_buffer_schedule` | IN01-E08 | |
| Hitstun timing modes and transition-term table | `input::hitstun_timing_modes`, `input::starting_status_to_term` | IN01-E09 (three modes) | |
| Loop repeats; Mash Interrupt stops playback | `input::loop_restarts_playback`, `input::mash_interrupt_stops` | IN01-E10 | |
| Save-state record-on-load and playback-on-load, not on ledge; chords ignored during load; triggered once per restore | `input::save_state_triggers_once` | IN01-E11 | |
| Per-slot facing and status (§6.1) | `input::per_slot_origin` | IN01-E12 (record two slots facing opposite ways) | |
| Ice Climbers CPU: one clock, Popo's controller only (§6.2) | `input::primary_cpu_only` | IN01-E13 | |
| Pokémon Trainer switch during playback (§6.3) | — | IN01-E14 | |
| Hot swap cancels; lobby practice entry 4; quick reset stops | `input::session_clear_stops` | IN01-E15 (lobby) | |
| Tint applied per phase; notification texts and colours | `input::phase_notifications` | IN01-E02 screenshots | |
| Named-layer contract functions exist and behave (§9) | `input::named_contract_export_import_refusals` | IN01-E16 (named library round trip) | |
| Zero heap allocations in the two hooks | `input::hooks_allocate_nothing` (counting allocator) | — | install line |
| **N4** Row `settings_override_playback`: default Off; Off → DI, SDI, air-dodge, tech, mistech and throw overrides return the original while `is_playback()`; On → they act as upstream; read live | `input::override_playback_default_off`, `input::override_playback_defers_when_off`, `input::override_playback_acts_when_on` | IN01-E17 (a recorded air-dodge direction plays unchanged with the row Off, is overridden with it On) | `evidence/input/IN01-E17.md` |
| **N4** The row is advanced (collapsed by default), persisted as `settings_override_playback`, and loads as Off from a file without it | `input::override_playback_row_metadata` (with `core04_draft9_fields_default`) | IN01-E17 screenshot (`menu-mockup/03-recording.png`) | |
| **Q7** (decided) Replay capture row writes `replay_capture.ini` once per change and stays in sync with the Deluxe page row | `input::replay_capture_row_writes_ini` | IN01-E18 | |
| **B4** (decided) While an AI drives P1, Record is refused with a notice and the CPU settings do not touch P1 | `input::record_refused_when_p1_ai` | IN01-E19 (pilot or fallback driving P1: record chord shows the notice, nothing recorded) | |

## 11. Open questions

1. **End-of-range off-by-one.** Upstream stores L−1 of L frames and plays n−1 of n. Recommendation: store and play the full range [0, L) (as the fork does for named macros); accept the one-frame timing difference from upstream and document it in the release notes. Needs the lead's decision because it changes parity. **Decided 2026-11-01 (B8 batch): accepted.**
2. **Stick scale.** Upstream injects `stick_x/y` as value/127 but computes the clamped fields with 1/120; the fork uses 1/120 for named macros after a 120-frame writer-versus-getter comparison matched exactly, and keeps /127 for numbered slots "historically". Recommendation: one scale, 1/120, verified once in Eden against the game's stick getters.
3. **Half-open status ranges and the placeholder term.** `ESCAPE_AIR_SLIDE` and `ESCAPE_B` are excluded by the half-open ranges, and "other" starting statuses use the side-special term. Recommendation: inclusive ranges; for "other", use a neutral actionable check (e.g. not in hitstun/hitstop and the generic attack term enabled) — needs a design decision from the lead.
4. **Recording Slot re-read every frame.** Recommendation: latch the slot at lockout.
5. **Pokémon Trainer switch mid-run.** Recommendation: continue (current behaviour) but re-read facing on the new body; verify in Eden.
6. **Save-state trigger runs once per fighter callback.** Recommendation: trigger once when both fighters have finished restoring (the fork's named path already does this).
7. **CPU-as-P1 recording.** Recommendation: when P1 is AI-driven, record the AI's final controls for P1 (the P1 entry's controller at +0x4178) instead of the idle human controller, or refuse recording with a notification. Operator decision. **Decided 2026-11-01 (B4): the second alternative;** Record is refused with a notification while an AI drives P1, and the CPU settings apply to the CPU role only (SPEC-cpu-01 question 7). Recording the AI's line (reading P1's controller slot) is not built.
8. **Lockout length 30.** Upstream suspects it is too long for ledge recordings. Recommendation: keep 30 for parity; make it a diag-tunable to test 15.
9. **Menu open during recording.** Recommendation: pause the record clock while a menu captures input (do not store neutral frames).
10. **Tint clearing.** Upstream never clears the colour blend; verify in Eden whether the game resets it each frame; if not, clear it when the phase ends.
11. **Unrecorded slot plays 60 neutral frames.** Recommendation: track "recorded" per slot and refuse with a notification. **Decided 2026-11-01 (B8 batch): accepted.**
12. **Random slot pick draws the game's RNG** (`sv_math::rand("fighter")`). In Training this is harmless; the online read-only rules forbid drawing checked RNG. Recommendation: use a plugin-owned PRNG for all weighted picks (also for lobby practice).
13. **L+R+A reset does not stop the recorder.** Recommendation: stop on reset. **Decided 2026-11-01 (B8 batch): accepted.**
14. **Persist numbered slots?** Not upstream. Recommendation: no, keep the named library as the persistent form.
15. **Storage root.** Carried-over code uses `sd:/ultimate/TrainingModpack/…`; the template says `sd:/ultimate/TrainingRooms/…`. Lead decides the migration (and whether to read the old path once). **Decided earlier (2026-11-01) by SPEC-core-08:** the root is `sd:/ultimate/TrainingRooms/`, with a one-time copy of the fork's data.
16. **Multiple input polls per game frame.** The fork's capture work observed repeated callbacks of the input-mapping function. If true, the recorder's store is idempotent (last write wins) but input delay is not (SPEC-input-03 open question 1). Needs a diag measurement.
17. **Playback started while Recording** abandons the recording silently. Recommendation: refuse playback while Recording, mirror of the lockout refusal. **Decided 2026-11-01 (B8 batch): accepted.**
