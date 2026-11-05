# Frame strip — behavioural specification

- **Spec id:** `SPEC-input-06`
- **Origin:** TrainingRooms' own design. Operator decision INPUT-VIZ, option A, 2026-11-05
  (`docs/DECISIONS.md` on `review/menu-proposal`, `d2f0db8`; the mockup is
  `docs/decisions/INPUT-VIZ-A-frame-strip.png`). It supersedes SPEC-input-02 (the input log).
- **Status:** implemented on `p23/input-viz`
- **Spec writer:** tr-lane-ally (Opus 5.5), 2026-11-05
- **Clean room:** no upstream or fork source was read for this spec. The shapes and colours are our own; the
  button glyphs come from the game's font, as the input log's did.

## 1. Purpose

A strip at the top of the Training HUD shows the last 60 game frames of P1 and the primary CPU, so a player can see
**when** each input landed against what both fighters were doing. Below the frames it states how the last hits linked:
whether the follow-up was a true combo and by how many frames, how early or late the follow-up was pressed, and the
frame advantage after the exchange. It replaces the 15-row input log of SPEC-input-02.

## 2. Terms

Frames are game frames, counted once per frame on P1's entry callback (§5.1). "Actionable" is the frame-advantage
test of SPEC-cpu-07 §3: any of the five transition terms, or a cancel (`runtime/features/cpu.rs` `actionable`).

| Term | Definition |
| --- | --- |
| **hit** | A frame `H` on which the CPU's hitstop starts (`StopModule::is_stop` false on the frame before, true now) while P1 is not in hitstun, shieldstun or shield (that would be the CPU hitting P1). On shield when the CPU is shielding (`GUARD_ON ..= GUARD_DAMAGE`) or in shieldstun and not in hitstun; on hit otherwise. A projectile's hit counts. |
| **action** | A run of P1 frames that are not actionable. It starts on the first non-actionable frame `S`, or when a press buffered during the last action starts a new attack with no actionable frame between: another move than the action's, or the same attack status from its motion's start (the motion frame goes back). |
| **follow-up** | A hit `H₂` of another action than the previous hit `H₁`'s, both on hit, with the CPU free for at most 20 frames before `H₂` (`DROP_MAX`; later, it is a new exchange). `A₁` is P1's first actionable frame after `H₁`, or `S₂` when the follow-up was buffered. Hits of one action (multi-hit moves) are not follow-ups. |
| **free frame `F`** | The CPU's first actionable frame after a hit. |
| **link** | For a follow-up on hit: `link = H₂ − F₁`, where `F₁` is the free frame the first hit would have given. If the CPU had an actionable frame before `H₂`, `F₁` is that frame (observed) and the link is a **drop** (`link ≥ 1`). Otherwise it is a **true combo** and `F₁ = (H₂ − 1) + r + K`, where `r` is the CPU's hitstun left (§4) read on frame `H₂ − 1` and `K` is the sampling constant (§4). A true combo's link is `≤ 0`; `−link` is the number of frames it had to spare. |
| **input** | The follow-up's press: the latest frame `I ≤ S₂` on which an attack, special, grab, jump or shield button went down (a new press in P1's mapped inputs), looked for up to 15 frames back; if there is none, `I = S₂`. `input = I − A₁`: negative means pressed before P1 could act (buffered), 0 on time, positive late. |
| **combo window** | The presses that would still have made a true combo, assuming the same startup: from `A₁` to `S₂ − link`. Empty when `S₂ − link < A₁` (even a frame-perfect press drops). Its width is `S₂ − link − A₁ + 1`. |
| **spare** | `−link` for a true combo: how many frames later the same press could have been. For a drop, `link` frames past the window. |
| **advantage** | After an exchange ends (both fighters actionable, no hit in between): `F − A`, the CPU's first actionable frame minus P1's, both counted from the last hit. Positive means P1 acts first. |

## 3. User-visible behaviour

### 3.1 Menu

On the Screen page (`model/menu/table.rs`):

| Item | Setting id | Values | Default |
| --- | --- | --- | --- |
| Input Display | `input_display_strip` | Off, Frame strip | Frame strip |

- The row replaces "Input Display" (`input_display`: Off / Smash Inputs / Raw Inputs / Status Only) and
  "Group inputs by status" (`input_display_status`). Both old ids are retired: they are read and dropped.
- **Migration:** a file without `input_display_strip` takes it from the old `input_display`: Off stays Off; Smash
  Inputs, Raw Inputs and Status Only become Frame strip. The same rule applies to the saved defaults. A file that has
  neither key takes the default.
- **Raw versus Smash is gone.** The chips show the mapped inputs, which are what the game acted on, and that is what
  combo timing is about. The 99-frame held count and the status names are gone too: the lanes show the states, and
  the LAST row shows the move names.
- Help text: "Show the last second of P1 and CPU frames, how each follow-up linked and the frame advantage".

### 3.2 Layout

The strip sits at the top centre of the screen. Its container is `TrIv`, and every pane name starts with `TrIv`
(the hitbox lane's panes start with `TrOrbs`). The native canvas is 1920 × 1080 with y up; the strip is 1296 × 216,
centred at (0, 420). From the top:

1. **IN row:** an input chip at the frame each new press landed: our own rounded box in the button's colour, the
   button glyph from the game's font and, for a tilted stick, our own arrow (a shaft and a head). Up to 8 chips. A
   tick runs from the chip down to the P1 lane. The combo window of the latest follow-up is a green bar above the P1
   lane, with "combo window Nf".
2. **P1 lane and CPU lane:** 60 frames, one cell column per frame, newest on the right, 13.5 units per frame. A lane
   is drawn as runs of one state (one rectangle per run, up to 16 per lane). A run of 7 or more frames of recovery,
   hitstun or shieldstun shows its length.
3. **Brackets** under the CPU lane: `link −N` between the follow-up hit and the free frame it beat, and `adv +N` between
   P1's and the CPU's first actionable frames.
4. **Readouts** on the right: ADVANTAGE, LINK, INPUT (§3.4).
5. **LAST row:** the last 4 links, newest first: a pill (`TRUE −3`, `DROP +2`) and the two moves (`Jab → Up tilt`).

State colours (sRGB; the legend of the mockup):

| State | Colour | Rule, first match |
| --- | --- | --- |
| hitlag | (232, 232, 236) | `is_stop` |
| hitstun | (170, 98, 234) | CPU or P1 in hitstun |
| shieldstun | (48, 196, 188) | in shieldstun |
| actionable | (64, 72, 88) | actionable |
| active | (238, 70, 84) | P1 only: any of the 8 attack slots is active |
| startup | (244, 180, 52) | P1 only: no active frame yet in this action, and the status is an attack (`move_observer::family`, or a fighter's own status above `FINAL`) |
| recovery | (82, 136, 232) | otherwise not actionable (lag, landing, dodges) |

The CPU lane uses the same rules without active and startup: a busy CPU frame that is none of the above is recovery.

### 3.3 Chips

Chip colours: attack (A) green (72, 190, 110), special (B) red (226, 82, 76), jump (X/Y) blue (64, 170, 220), shield
(L/R) slate (90, 110, 160), grab (Z) purple (140, 100, 210). A press of several buttons on one frame shows the first
in that order. The stick arrow is shown when the left stick is binned to a direction on that frame (the thresholds of
SPEC-input-02 §3.3, kept).

### 3.4 Readouts

| Readout | Shows |
| --- | --- |
| ADVANTAGE | `+N` (green) / `-N` (red) / `0`, then "on hit" or "on shield"; `-` until the first exchange ends |
| LINK | the latest link: `-N` and "TRUE combo" (green), or `+N` and "DROP" (red). A true combo whose `r` could not be read shows "TRUE" with no number. `-` until the first follow-up |
| INPUT | the latest follow-up: `+N` "Nf late", `-N` "buffered Nf", or `0` "on time", then ", S spare" or ", Sf past" |

The texts are ASCII: the game's font is not assumed to carry `−` or `→` (the LAST row writes `Jab > Up tilt`).

### 3.5 When it updates, freezes and hides

- **Sampling:** once per game frame, on P1's entry callback, while the scene is Training or an active lobby practice.
- **Freeze:** when both fighters have been actionable and P1's inputs unchanged for 30 frames, the strip stops
  scrolling and keeps the last exchange on screen. It moves again on the next non-actionable frame or new press.
- **Hidden:** while the TrainingRooms menu or the game's pause menu is open, when the row is Off, and outside Training
  and lobby practice. Sampling continues under the pause only on frames the game runs (none while paused).
- **Cleared:** on a scene change, a hot swap, a save-state load, and whenever the game initialises a fighter again (a
  Training reset, a CPU replaced; review IV-4, 2026-11-05): the frames before it no longer happened.
- **Frame advantage notice:** while the strip is shown, the "Frame Advantage" notice is not posted, because the
  ADVANTAGE readout shows the same number and the notice would cover the strip. The Frame Advantage row keeps its
  meaning when the strip is Off, and when the layout has no strip panes (another mod's `info_training`, the stock
  archive): then the strip neither samples nor draws and the notice posts as before (review IV-2).
- **Settings from before the strip:** the retired `input_display` row is read as the base read it, padded or cut to
  its four options, its first set option the mode; Off stays Off, anything else is the strip (review IV-3).

## 4. Native reads

All of them existed before this spec; no new hook or native site is added.

| Read | Native (13.0.5 `main+`) | Where in our tree |
| --- | --- | --- |
| P1's mapped inputs after input delay | the P1 poll callback | `runtime/features/input_delay.rs` (P1 poll) |
| Status kind, previous status | `StatusModule::status_kind` `0x2087720`, `prev_status_kind` `0x2087770` | `model/sites_pkg2.rs:14-15` |
| Actionable | `WorkModule::is_enable_transition_term` `0x208dd60`, `CancelModule::is_enable_cancel` `0x2092860` | `runtime/features/cpu.rs` `actionable` |
| Hitstop | `StopModule::is_stop` `0x20891f0` | `model/sites_pkg4.rs:9` |
| Active attack slots (P1) | `AttackModule::is_attack` `0x1fd01c0`, slots 0..8 | `model/sites_hitbox.rs:19` |
| Hitstun left (CPU) | `WorkModule::get_float` `0x208db50` with `FIGHTER_INSTANCE_WORK_ID_FLOAT_DAMAGE_REACTION_FRAME` | `model/sites_pkg2.rs:35` |

**Hitstun left.** The work id is resolved by name through the game's own constant table (`lua_const`,
`main+0x3728e10`); no number is written down. The id is proven from the owned `lua2cpp_common.nro` of 13.0.5:
- its constant table at `.rodata 0x6a5b70` (0x4c0d XXH64 keys, filled into `L2CAgentGeneratedBase::const_value_table__`
  by the loop at `0x9cc-0x9f0`) holds the key of this name at index 0x124;
- that entry is read by `ftStatusUniqProcessDamage_exec_common` at `0x107d64`, which calls `WorkModule::get_float` with
  it and turns on the reaction-end flag when the value is `<= 0`. The setters are
  `sub_set_reaction_frame_by_motion` and `sub_set_reaction_frame_by_current_motion` (`WorkModule::set_float`).

So the value is the hitstun frames left, and hitstun ends when it reaches 0. Where it is decremented is not in
`lua2cpp_common` and was not traced statically. The constant `K` (the frames between the sampled `r` and the sampled
free frame) is therefore measured: on every hit that is not followed up, the diag build logs `r` on each hitstun
frame and the observed free frame. **Measured `K = 0`** (Eden `tr-p23-ally-iv1`, 2026-11-05, diag `192502cc`): with
the first assumption `K = 1`, all 30 stun checks predicted the free frame one frame late; on the frame the CPU reads
`r = 0` it is already actionable.

**Known limits from that boot (open):**
- Fighter-specific attack statuses (above `FINAL`, e.g. Steve's jab) are named "Move" in the LAST row; only the
  engine-common attack statuses have names.
- P1's lane showed no hitlag cells: `StopModule::is_stop` was true for the CPU in hitlag and apparently not for P1 as
  the attacker. Whether the attacker's freeze is `StopModule::is_hit` (`0x2089210`) is to be settled with a guest
  tracepoint before any change.
- Steve's jab startup frames read as actionable (P1 became busy on the hit frame), so they show no startup cells.

**When `r` is unavailable** (the id did not resolve, the value is not finite or not in 0..=255, or `get_float`'s row
was refused): a true combo shows "TRUE" without its number, and the link margin is absent. Everything else is drawn.
`WorkModule::get_float` is the CPU tech unit's row, so the read runs behind its own gate (`input_display::reaction_left`,
`REACTION_ON`, set by boot from that unit's closure); the strip's other reads are the input display unit's closure
(`model::capabilities::CROSS_NEEDS`), and a refused row there turns the whole strip off, as for every unit.

**Hitlag left is not read.** `StopModule::is_stop` on every frame draws the hitlag cells and finds the hits; nothing
in §2 needs how many hitlag frames remain, so `StopModule::get_hit_stop_real_frame` (`0x2089250`) is not called.

**Diag lines** (diag build only): `[FrameStrip] constants resolved …`, `panes resolved found=N of 121`, `reaction r=…`
on each CPU hitstun frame, `stun_check predicted=… seen=…` for each hit not followed up, `link …` for each link,
`advantage …` for each exchange, `shown=… menu_open=… on=…` on each show or hide.

## 5. Timing

### 5.1 Sampling order

Per game frame, on P1's entry callback (after Frame Advantage's own read): read P1 and the CPU (§4), classify both
(§3.2), push one column, update the exchange state (§2) and publish a snapshot for the UI. The CPU accessor comes from
the live manager each frame (the role adapter's measurement), never from a saved pointer.

### 5.2 Drawing

On the UI tick of `info_training` (and of `info_waiting` in lobby practice), from the latest snapshot:
- the container's visibility;
- each pane's position, size, colour, visibility and text are written only when they differ from what was last
  written to it;
- the cache is dropped when the layout is registered again.

Pane pointers are resolved once per layout registration, by name, below the container.

## 6. Edge cases

1. **Hitlag in the middle of a link:** the hitstop frames are columns like any other; `r` does not count down
   during hitstop, and the link uses `r` from the frame just before the follow-up hit.
2. **Shield:** a follow-up on shield is not a link. It shows in the lanes (shieldstun) and as ADVANTAGE "on shield".
3. **Buffered input:** `I < A₁`, so `input` is negative; the window starts at `A₁` (a press earlier still starts the
   move on `A₁`).
4. **A follow-up that misses:** no hit, so no link. The exchange ends when both are actionable.
5. **Multi-hit moves:** hits within one action are not links (`P1` never actionable between them).
6. **Projectiles:** a projectile hit is a hit (the CPU's hitstop), but P1's lane has no active cells for it.
7. **Several CPUs, Ice Climbers:** only the primary CPU entry is measured.
8. **CPU-as-P1 (the pilot):** P1's inputs are the controller's, as for the log (SPEC-input-02 §6.9).

## 7. Resources

- About 120 panes, against the input log's 271 (`model/ui/panes.rs`).
- No heap in hooks: fixed arrays (60 columns, 16 runs per lane, 8 chips, 4 links).
- Native calls per frame: about 24 (2 × status, 2 × 6 actionable, 2 × is_stop, 8 × is_attack, 1 × get_float while the
  CPU is in hitstun).
- Pane writes per UI tick: 0 while frozen or hidden; up to about 70 while scrolling.

## 8. Test provenance

| Requirement | Host test | Eden row |
| --- | --- | --- |
| Classification order (§3.2) | `frame_strip::classify_*` | IV-1 |
| True combo, drop, buffered and late input, window, spare | `frame_strip::link_*` | IV-1, IV-2 |
| Hitlag in the middle of a link | `frame_strip::link_through_hitlag` | IV-1 |
| Shield | `frame_strip::shield_is_not_a_link` | IV-3 |
| Advantage on hit and on shield | `frame_strip::advantage_*` | IV-1, IV-3 |
| Freeze and clear | `frame_strip::freeze_*`, `frame_strip::clear_*` | IV-1 |
| LAST history (4, newest first) | `frame_strip::history_*` | IV-2 |
| Pane plan: names, the budget, writes only on change | `tests/frame_strip_plan.rs` | IV-1 |
| Settings migration, defaults import and a round trip | `tests/frame_strip_settings.rs` | — |
| Hidden under the menu and the pause, and when Off | `frame_strip::hidden_*` | IV-4, IV-5 |
| `K` and the hitstun-left read | diag `[FrameStrip] stun_check` lines | IV-1 |
