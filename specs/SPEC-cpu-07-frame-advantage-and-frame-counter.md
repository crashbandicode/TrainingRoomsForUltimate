# Frame advantage display, the frame-counter service and the reset bundle — behavioural specification

- **Spec id:** `SPEC-cpu-07`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude agent (Fable 5.1), 2026-11-01
- **Upstream reference read (spec writer only):** `src/training/combo.rs`, `src/training/frame_counter.rs`, `src/training/reset.rs`, the tick/reset placement in `src/training/mod.rs`, the reset call in `src/training/save_states.rs`, `training_mod_consts/src/options.rs` (`OnOff`), `training_mod_consts/src/lib.rs` (Misc tab) at commit `97cbe9b9` (and `077a8e43` where it differs: the reset-detection condition); fork changes at `…/stages/m5i/stage/src/training/combo.rs` lines `21-26, 88, 91-107`, `…/training/reset.rs` lines `12-34`, `…/training/mod.rs` lines `297-311, 342-347` from `docs/provenance/provenance-files.md`
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

Frame advantage tells the player, after an exchange (for example an attack on the CPU's shield), how many frames earlier or later than the CPU they could act again. The frame-counter service is the shared clock the other CPU specs use for every delay (aerial, ledge, throw, pummel, reaction, lay-down lockout): a small set of counters that advance once per simulation frame and are cleared on a Training reset or save-state load. The reset bundle is the list of plugin state that an L+R+A reset or save-state load clears across the CPU specs.

## 2. User-visible behaviour

### Menu items

| Row (Misc tab) | Setting key | Type | Values | Default |
| --- | --- | --- | --- | --- |
| Frame Advantage | `frame_advantage` | on/off | On, Off | Off |

Help text (upstream): "Display the time difference between when the player is actionable and the CPU is actionable. Note that the CPU must not be mashing any options." The frame-counter service and the reset bundle have no menu rows.

### Where it is live

Training and lobby practice. Frame advantage requires `mash_state` to be empty (a mashing CPU acts early and spoils the measurement) and both practice fighters present.

### Visible outputs

A notification titled "Frame Advantage" with the signed number (`+N`, `-N`, `+0`) shown for 60 frames, coloured red `(200, 8, 8)` when negative, black `(0, 0, 0)` when zero, green `(31, 198, 0)` when positive (all with alpha 255). Showing it first clears every other notification (frame advantage has priority). The notification widget itself belongs to the UI spec; this spec only calls "clear all notifications" and "show coloured notification(title, text, frames, colour)".

## 3. Inputs

- `frame_advantage`, `mash_state` (live).
- The player entry's and the CPU entry's module accessors (fork fix: when either is unavailable, do nothing this frame; never panic).
- Per fighter "actionable" test: any of the transition terms `FIGHTER_STATUS_TRANSITION_TERM_ID_CONT_ESCAPE_AIR` (air dodge), `…_CONT_ATTACK_AIR` (aerial), `…_CONT_GUARD_ON` (shield), `…_CONT_ESCAPE` (spot dodge / roll), `…_DOWN_STAND` (neutral getup from a missed tech or slip) is enabled (`WorkModule::is_enable_transition_term`), or `CancelModule::is_enable_cancel(accessor)` is true.
- The fighter's entry id (`FIGHTER_INSTANCE_WORK_ID_INT_ENTRY_ID`): frame advantage runs on the player entry's (entry 0) callback only, so it computes once per frame.
- Fork interface: "replay preview occupied", "replay capture panel visible / loading" (clear the frame-advantage state and skip while either holds).
- For the counter service: the once-per-frame dispatch (simulation frames), the UI draw tick (real frames), the primary-CPU-body predicate, reset and save-state events.

## 4. Outputs and native state touched

| Native state | How it is reached (function / field by name; `main+0x…` for 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| None. This spec reads game state and writes plugin state and a notification only | — | — | — |

Native facts it depends on (owned by the core spec): the fighter vtable "on frame" slot (`.data main+0x4f80568`, original `main+0x614630`), which provides the once-per-frame-per-fighter dispatch; the fighter vtable "on init" slot (`.data main+0x4f80240`, original `main+0x60dfa0`), which provides the re-initialisation event used for reset detection; and the inline hook at the Training reset check `main+0x1378e50` (the L+R+A Reset on/off row, Misc tab, core spec).

## 5. Timing and frame rules

### Frame-counter service

- A fixed table of counters (no growth at run time; at least 16 slots). Each counter has a count, a "counting" flag and a type: **InGame** (cleared by the reset bundle), **InGameNoReset** (ticks with the game but survives resets), **Real** (ticks with UI frames, used by the menu spec).
- Operations: register (returns a slot index, at boot), start, stop, reset count, full reset (count 0 + stop), get count, tick one index.
- `should_delay(delay, index)`: returns false at once when `delay == 0`; otherwise, if the count is 0 start counting; if the count has reached `delay` perform a full reset and return false; else return true. Callers use it as "still waiting?".
- **Tick (in-game)**: every counting InGame and InGameNoReset counter advances by one exactly once per simulation frame. Placement: the once-per-frame dispatch of the **primary CPU body** (the CPU entry's current body, not Nana, not an extra CPU). Upstream ticked from every CPU-operated fighter's callback, which advanced the clock twice per frame with Ice Climbers (fork fix, required). The fork also ticks during its replay preview from the same body (interface).
- **Tick (real)**: every counting Real counter advances once per UI tick (menu spec).
- **Reset all**: every InGame counter is set to 0 and stopped.

Consumers (one slot each): aerial delay (SPEC-cpu-01/06), ledge delay (SPEC-cpu-03), throw delay, pummel delay (SPEC-cpu-06), reaction time (SPEC-cpu-05), lay-down lockout (SPEC-cpu-04), frame-advantage elapsed (this spec), fast-fall delay (SPEC-cpu-06), the Pokémon Trainer switch delay (fighter-specific spec), the menu close wait (menu spec, Real).

### Reset bundle

Performed when:

1. the CPU-operated fighter is (re)initialised in native Training (Training entry and L+R+A reset) and no hot swap is in progress (fork gate), detected on the fighter "on init" event for a CPU-operated fighter; and
2. a save-state load places a fighter (right after position and facing are written; also the fork's lobby quick reset, which goes through the same load path).

The bundle: full reset of the mash queue (SPEC-cpu-01); re-roll the SDI direction (SPEC-cpu-02); reset all InGame counters; reset frame advantage (below); reset the ledge delay (SPEC-cpu-03); reset the throw delay (SPEC-cpu-06); re-read the shield tilt (SPEC-cpu-05). A menu save additionally re-rolls the SDI direction and shield tilt (fork: settings-changed event) and applies the mash-disable rule (SPEC-cpu-01).

Upstream HEAD `077a8e43` wraps the init-time reset in a condition that only holds outside training; that inversion is not adopted (rule 1 above is the 97cbe9b9 and fork behaviour).

### Frame advantage (once-per-frame dispatch, player entry's callback)

Skip entirely when `frame_advantage` is Off (open question 1), when `mash_state` is non-empty, when either accessor is missing, or while the fork's replay preview/capture holds (then also reset the state).

State: "counting" flag, the elapsed counter (InGame), the frame each fighter first became actionable ("locked" values), and last frame's actionable flags for edge detection.

1. Compute `player_actionable` and `cpu_actionable` with the test in §3; "just actionable" = actionable now and not last frame.
2. Not counting: if either fighter is not actionable, reset the elapsed counter to 0 and start it, set the locked value of any fighter that is still actionable to 0 (it has no frames to wait out), set counting.
3. Counting: `elapsed` = counter value. If the player just became actionable, lock `player = elapsed`; if the CPU just became actionable, lock `cpu = elapsed`. When both are actionable: if both locked values exist, `advantage = cpu − player` and show the notification (if On); then reset (full reset of the counter, clear locks, counting = false).
4. Remember this frame's actionable flags.

Positive advantage means the player can act before the CPU (plus frames); negative means the CPU acts first.

### Setting changes

Turning Frame Advantage Off clears the state; turning it On starts measuring at the next exchange. A change to `mash_state` from empty to non-empty aborts a measurement in progress.

## 6. Edge cases and interactions

- **Ice Climbers**: the clock ticks from Popo's body only; Nana does not tick. Frame advantage measures Popo.
- **Pokémon Trainer**: the CPU body changes on a switch; the accessor lookup resolves the current body each frame; a switch mid-measurement makes the CPU non-actionable, which is counted normally.
- **Kirby, Mii, Aegis, Steve, Olimar, Rosalina**: no special handling.
- **Save states**: loading resets the counters and the measurement.
- **Lobby practice**: the CPU is entry 4; the clock ticks from its body; the real match never ticks.
- **Hot swap**: a swap in progress suppresses the init-time reset (the new body is not a Training reset); session clear resets all InGame counters and the measurement.
- **Menu open**: no simulation frames, so no ticks.
- **CPU-as-P1**: the measurement assumes entry 0 is the player; with P1 under AI control the numbers still describe entry 0.

## 7. Persistence

Nothing is written. The toggle lives in the menu JSON.

## 8. Resource budget

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | see docs/resource-costs.md (W5) | counter table 16 × (u32 count + flags) ≈ 128 bytes; frame-advantage state < 32 bytes | `tools/build/nro_size.py` per-feature build |
| per-frame work | see docs/resource-costs.md (W5); upstream evaluates up to 12 transition-term queries per frame even with the toggle Off | tick: one pass over 16 slots; frame advantage: zero work when Off; when On, up to 12 transition-term queries + 2 cancel queries per frame | diag counters |
| heap allocations in hooks | upstream: a growable vector of counters, strings for the notification text | 0 in hooks (format the notification text into a fixed buffer) | counting allocator, install line |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| `should_delay` semantics (0, start on first call, reset at delay) | `cpu07_should_delay` | — | `evidence/cpu-07/host.md` |
| Tick advances only counting InGame/InGameNoReset; real tick only Real; reset-all clears InGame only | `cpu07_tick_types_and_reset` | — | `evidence/cpu-07/host.md` |
| Exactly one in-game tick per simulation frame with Ice Climbers and extra CPUs | `cpu07_single_tick_per_frame` | `cpu-07-ice-climbers-delay-exact` | `evidence/cpu-07/tick.md` |
| Reset bundle on Training entry, L+R+A and save-state load; not during a swap | `cpu07_reset_bundle_events` | `cpu-07-lra-reset`, `cpu-07-savestate-load` | `evidence/cpu-07/reset.md` |
| Frame advantage sign, lock-in, notification colours and duration | `cpu07_frame_advantage_math` | `cpu-07-jab-on-shield-minus`, `cpu-07-plus-frames` | `evidence/cpu-07/advantage.md` |
| No work when Off; aborted by mash_state; no panic without accessors | `cpu07_frame_advantage_gates` | `cpu-07-off-no-notification` | `evidence/cpu-07/advantage.md` |
| Lobby: ticks from entry 4; no ticks in the match | `cpu07_lobby_tick_source` | `cpu-07-lobby-delay` | `evidence/cpu-07/lobby.md` |

## 10. Open questions

1. **Work when Off.** Upstream runs the whole frame-advantage measurement every frame and only suppresses the display; this spec requires zero work when the toggle is Off. Confirm (cost only).
2. **Actionable definition.** The five transition terms plus "cancel enabled" do not include ground attacks, jumps, grabs or dashes; a fighter whose first available option is one of those is counted as actionable only when one of the five appears (usually the same frame, but not for every status). Recommendation: keep upstream's definition for parity, and record any Eden row where the displayed value differs from frame data by more than one frame for later extension.
3. **Nana ticks.** Ticking from the primary body only means Nana-only situations (Popo dead, Nana alive) stop the clock. Confirm that this corner case is acceptable (upstream's double tick was the alternative).
4. **Reset on Training entry.** Rule 1 fires on the CPU's first initialisation as well as on L+R+A. If the core spec provides a dedicated "Training entered" event, the bundle may hang off that instead; keep the behaviour identical.
5. **Upstream HEAD `077a8e43`** inverts the reset condition (reset only outside training). Not adopted; recorded here so nobody re-imports it as a fix.
