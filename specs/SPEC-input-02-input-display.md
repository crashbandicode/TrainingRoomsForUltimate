# Input display — behavioural specification

- **Spec id:** `SPEC-input-02`
- **Origin:** upstream (Training Modpack) — clean-room reimplementation required
- **Status:** draft
- **Spec writer:** Claude (Fable 5.1), spec-writer for area `input`, 2026-11-01
- **Upstream reference read (spec writer only):** `src/training/input_log.rs`, `src/training/ui/input_log.rs`, `src/training/ui/mod.rs` (draw dispatch and fade), `src/training/frame_counter.rs`, the `InputDisplay` option and the Misc tab rows in `training_mod_consts/src/{options,lib}.rs`, all at commit `97cbe9b9` (unchanged at `077a8e43`). Fork changes at `…/stages/m5i/stage/src/training/input_log.rs` lines 177, 187, 211, 312–318 and `…/stage/src/training/ui/input_log.rs` (the whole renderer, 341 fork-only lines), `…/stage/src/training/ui/mod.rs` lines 138–230 (UI anchor tick) and `…/stage/src/training/ui/anchors.rs` line 160 (cache invalidation), plus `docs/input-display-performance.md`, `docs/input-display-benchmark.md`, `docs/input-display-followup.md`.
- **Implementers must not open the upstream paths above.** Everything an implementer needs is in this file and in the native reference tables it cites.

## 1. Purpose (one paragraph)

A column of up to fifteen rows on the left of the Training HUD shows the player's recent inputs: each row is one distinct input (buttons and binned stick directions), with the number of frames it was held and, optionally, the fighter status it happened in. A new row appears only when the input changes; rows fade out over time and expire after ten seconds. Three modes read the mapped "Smash" inputs, the raw controller, or statuses only.

## 2. User-visible behaviour

### 2.1 Menu items (Misc Settings tab, id `misc`)

| Item | Setting id | Type | Values | Default |
| --- | --- | --- | --- | --- |
| Input Display | `input_display` | single choice | None, Smash Inputs, Raw Inputs, Status Only | Smash Inputs |
| Input Display Status | `input_display_status` | On/Off | | Off |

Help texts: "Log inputs in a queue on the left of the screen"; "Group input logs by status in which they occurred".

### 2.2 Visible outputs

- Up to 15 rows (`NUM_LOGS = 15`), newest at the top, each 52.5 layout units below the previous (row *i* at y = −52.5 × *i* relative to the column root).
- Each row: up to 5 icon slots (left to right: left-stick direction in white, right-stick direction in yellow, then buttons in the order of §3.4), a frame-count text ("Frame" text box) showing how many consecutive frames the row's input has been held, capped at **99**, and a status text ("Status" text box) showing the fighter's status name when Input Display Status is On, otherwise empty.
- Fade: a row with remaining time-to-live `ttl` is fully opaque when ttl ≥ 200 frames; for 0 < ttl < 200 its alpha is `255 / log10(201) × log10(ttl + 1)` (logarithmic fade); at ttl 0 it is invisible. A row is created with ttl 600 and loses 1 per in-game frame, so it is opaque for 400 frames, fades for 200 and disappears.
- The whole column is hidden while the mod menu is open, while the vanilla pause menu is open (SPEC-input-04 §3.3 for the detection), or when Input Display is None.
- Colours (RGB, applied as the icon picture material's "black" colour): GREEN (22, 156, 0), RED (153, 10, 10), CYAN (0, 255, 255), BLUE (0, 40, 108), PURPLE (100, 66, 202), YELLOW (230, 180, 14), WHITE (255, 255, 255).

**Icons — lead decision 2026-11-02 (packages 4 and 5).** The specs name no source for the input-log icon art, and
upstream's icon pictures are unlicensed. Each row's five icon slots are therefore **text boxes that show the game's own
controller glyphs** from its font (code points in SPEC-core-03 §4.3: A E0E0, X E0E2, Y E0E3, L E0E4, R E0E5, ZL E0E6,
ZR E0E7, D-pad E079–E07C, Plus E0EF, Minus E0F0, stick presses E104/E105, the rest per that section), coloured with
this spec's per-icon colours as the text colour; stick directions and their strong tiers use the clean room's own
arrow glyph built from rectangles (the font has no arrows). No new texture, 15 rows × 5 text slots instead of
2,775 picture panes. `Frame` and `Status` stay text boxes. Pane names are package 5's single pane table
(`panes::name_of`). An Eden screenshot of the log goes into the release summary for the operator.
## 3. Inputs

- The two menu settings.
- P1's game `Controller` snapshot (raw, 0xD0 bytes; layout in SPEC-input-04 §3.1) and P1's final mapped input record (`MappedInputs`, SPEC-input-01 §3.1) as they stand **after** input delay (SPEC-input-03) in the P1 input poll.
- P1's status kind and fighter kind (status module `status_kind`, `utility::get_kind` on the P1 accessor, resolved per poll; skip the poll when the accessor is missing).
- Two in-game frame counters of the "in-game, no reset" kind (they advance once per CPU-body in-game frame and are **not** reset by L+R+A or save-state loads): one "overall" stamp used to detect a new game frame, one "per-row" counter used for the frame count.
- Mod-menu-open flag (the logger skips while it is set); mod-menu and vanilla-menu flags (the renderer hides while either is set).

### 3.1 Row model

| Field | Type | Meaning |
| --- | --- | --- |
| ttl | u32 | remaining frames, 600 at creation |
| frames | u32 | frames the input has been held, shown capped at 99 |
| overall_frame | u32 | the overall counter value when the row was last written |
| raw_inputs | Controller snapshot (0xD0 bytes) | for Raw mode comparisons and icons |
| smash_inputs | MappedInputs (8 bytes) | for Smash mode comparisons and icons |
| status, fighter_kind | i32, i32 | for the status text and status comparisons |

15 rows × ~240 bytes ≈ 3.6 KB static (fork today).

### 3.2 "Different" rule (decides when a new row is created), by mode

- **Smash Inputs:** mapped buttons differ, or the binned left stick differs, or the binned right stick differs, or (Status On and status differs).
- **Raw Inputs:** the raw current-buttons bit set differs, or binned raw sticks differ, or (Status On and status differs).
- **Status Only:** Status On and status differs. (With Status Off nothing ever differs: a single row counts up to 99.)
- **None:** never (the logger is off anyway).

### 3.3 Stick binning (direction, strength)

Inputs are the signed stick values as i8: mapped sticks directly; raw sticks (f32, ±1.0) scaled by 120 and truncated. Thresholds: X walk threshold 20, Y dead zone 30. Rules, in order: if |y| < 30 → x > 20 gives 0°, x < −20 gives 180°, otherwise no direction; else if |x| < 20 → y > 30 gives 90°, y < −30 gives 270°, otherwise none; else if y > 30 → x > 20 gives 45°, x < −20 gives 135°, otherwise 90°; else (y ≤ −30) → x > 20 gives 315°, x < −20 gives 225°, otherwise 270°. Strength is always "weak" (a "strong" tier exists in the layout's icon names but is not produced). Direction icons: 0 right, 45 up_right, 90 up, 135 up_left, 180 left, 225 down_left, 270 down, 315 down_right.

### 3.4 Button icons

Smash mode, from mapped button bits in ascending bit order, duplicates removed by icon name: ATTACK or ATTACK_RAW → "a" GREEN; SPECIAL or SPECIAL_RAW2 → "b" RED; JUMP → "x" CYAN; GUARD or GUARD_HOLD → "lb" BLUE; CATCH → "zr" PURPLE; STOCK_SHARE → "plus" WHITE; APPEAL_HI → "dpad_up"; APPEAL_LW → "dpad_down"; APPEAL_SL → "dpad_right"; APPEAL_SR → "dpad_left" (WHITE). Other bits produce no icon.

Raw mode, from the raw current-buttons bit set, each inserted at the front so the final order is the reverse of this list: a GREEN, b RED, x CYAN, y CYAN, lb BLUE (L or the real digital L), rb BLUE (R or the real digital R), zl PURPLE, zr PURPLE, plus WHITE, minus WHITE, dpad_up, dpad_down, dpad_left, dpad_right (WHITE).

Status-only mode produces no icons. Then the right-stick direction (YELLOW) is inserted at the front, then the left-stick direction (WHITE), so the first two slots are the sticks. Only the first five icons are shown; an icon whose name is not in the layout's icon list (§4) is skipped without consuming a slot.

Icon names available in the layout (37 per slot): a, b, x, y, lb, rb, zl, zr, up, down, left, right, up_left, up_right, down_left, down_right, gcc_l, gcc_r, gcc_z, plus, minus, l_stick, r_stick, gcc_c_stick, dpad_up, dpad_down, dpad_left, dpad_right, dpad_left_right, up_strong, down_strong, left_strong, right_strong, up_left_strong, down_left_strong, up_right_strong, down_right_strong. The layout can start with any icon visible, so every slot's icons are hidden once before first use.

### 3.5 Status name

The status text is the game constant's display name for (fighter kind, status id): the constant's identifier with its `FIGHTER_STATUS_KIND_` or `FIGHTER_<CHARACTER>_STATUS_KIND_` prefix removed, preferring the fighter's own (or its echo parent's) status set over the common one when the numeric id is ambiguous. Upstream keeps a 9,000-line generated table for this; open question 1 covers how the clean room produces it without copying that table.

## 4. Outputs and native state touched

All writes go to the modpack's own panes inside the Training layout `info_training` (or, in lobby practice, inside the host part `TrModPort` of `info_waiting`), reached from the layout's root pane by name. No game-owned pane outside the modpack's subtree is written. Pane pointers are never retained across draws (fork rule).

| Native state | How it is reached (function / field by name; `main+0x…` for 13.0.5) | When | Guard |
| --- | --- | --- | --- |
| Column root pane `TrModInputLog` visibility | root pane → find by name (recursive) → set visible | every UI tick of the training layout | pane exists (the layout archive is the modpack's own, injected at `main+0x3774704`) |
| Row panes `TrModInputLog0` … `TrModInputLog14`: visibility, alpha and global alpha, pos_y, the global-matrix-dirty flag bit after a position change | direct children of `TrModInputLog` (fork: walked once via the pane's `children_list` sentinel and each child's `link`; a missing row is a panic "Could not find pane …") | when a row's position, fade or content changed | row found |
| Per row: icon picture panes `Input{0..4}/<icon>` visibility and material black colour; text boxes `Frame` and `Status` text | find by name inside the row | when the row's content changed (newest row always; other rows only after a full refresh) | panes found |
| UI entry point | Revision 29 anchors: an inline hook one instruction after the UI manager's per-frame pass loads the holder's layout (`main+0x375efa8`; locator needle at `+0x375efac`, `LOCATOR_DELTA = 4`) and one at the manager's insert path (`main+0x3784ad4`; locator `+0x3784ad8`); the manager singleton slot is `main+0x593ba90` (.bss); the UI-manager pass prologue `main+0x375e260` is located by a needle starting 0x14 bytes in. Upstream instead hooked `nn::ui2d::Layout` vtable entry +0x98 at `main+0x4b620` (the layout's Calculate). The modpack's HUD work runs per layout, after the layout's animate and before its calculate, once per frame and once at registration. | per layout per frame | five-word pins at both anchors; a mismatch skips the UI with a log line (fork) |

The two in-game frame counters are plugin state ticked by the per-fighter post-frame dispatch (SPEC-input-01 §4, primary CPU body only).

## 5. Timing and frame rules

### 5.1 Logger (P1 input poll, after input delay, before the recorder)

Skip when Input Display is None, when the mod menu is open, when `player_idx ≠ 0`, or when the P1 accessor is missing. Otherwise: read both counters, then make sure both are counting. Build the candidate row (ttl 600, frames 1, overall stamp = overall counter, the raw snapshot, the mapped record, status, kind). Let `new_frame` = the newest row's stamp ≠ the current overall stamp (this de-duplicates repeated polls within one game frame and polls while gameplay is paused).

- If `new_frame` and the candidate is different (§3.2) from the newest row: reset the per-row counter to 0 and tick it to 1; insert the candidate at the top, shifting the others down and discarding the fifteenth; advance the physical-row base `(base + 1) mod 15` (so the physical pane that held the oldest row becomes the newest; rows keep their panes while they age — upstream relies on this to redraw only the top row's content).
- Else if `new_frame`: overwrite the newest row with the candidate, then set its frames = min(per-row counter, 99) and keep its previous ttl.
- Then, if `new_frame`, decrement every row's ttl that is > 0.

Consequences: the frame count counts in-game frames (gameplay paused → no change); a held input that becomes different only by status (Status On) starts a new row; with Status Off, Status Only mode shows one row counting to 99.

### 5.2 Renderer (UI tick of the training or waiting-room layout)

Visible = mod menu closed ∧ vanilla menu closed ∧ mode ≠ None; set the column root's visibility. If hidden, do no content work; remember that a full refresh is due when visible again (fork). Otherwise for each row *i* (0 = newest) with physical index `(i + 15 − base) mod 15`: position y = −52.5·*i* (set the matrix-dirty bit when it changes), visibility true, fade from ttl (§2.2), and content:

- Upstream wrote content only for row 0 every draw (icons hidden then shown, "Frame", "Status"); rows 1–14 keep the content written when they were row 0, which is correct because rows rotate with their panes.
- Fork fix (fold in as the required behaviour): keep, per physical row, the last written position, ttl-min-200, mode, status flag, icon set, frame count and status text; write only what changed; write a row other than row 0 only after a full refresh (the layout was re-registered, or the column was hidden and is visible again — this redraws retained history in the **current** mode, which upstream did not do); never store pane pointers between draws; invalidate the cache whenever the layout is registered, even at the same address.

Visible results must match upstream's for every ordinary update (the fork's host comparison checked repeated frames, 32 input changes crossing the ring twice, all modes, status toggles, icon colour changes and removal, the 99 cap, menus, fades, clearing, same-address recreation). After recreation the fork restores all 15 retained rows (upstream left default panes); that is the intended behaviour.

### 5.3 Setting changes

Mode change: the logger uses the new mode's difference rule immediately; retained rows were created under the old rule and are not re-evaluated; the renderer redraws them in the new mode only after a full refresh (menu close triggers one because the column was hidden). Status toggle: the "Status" text of row 0 follows the toggle at once; older rows after the refresh. None → anything: logging resumes with the existing rows (not cleared).

### 5.4 Session clear (fork, fold in)

Leaving the practice scene, a hot swap, lobby activation change: all rows reset to default, base = 0, both counters fully reset (count 0, not counting).

## 6. Edge cases and interactions

1. **Input delay** (SPEC-input-03): the display shows the delayed input, by design ("read potentially delayed state for loggers").
2. **Recording and playback** (SPEC-input-01): during Lockout and Recording P1's mapped record is neutralised *after* the logger, so the display shows what P1 pressed, not what P1's fighter received.
3. **Ice Climbers as CPU**: the counters tick once per frame only when the primary-CPU rule of SPEC-input-01 §6.2 is applied; upstream ticked twice, so frame counts ran at double speed with an Ice Climbers CPU (fold in the fix).
4. **Pokémon Trainer / hot swap as P1**: fighter kind and status come from the current P1 body each poll; the status text uses the row's own kind, so old rows keep the old fighter's names.
5. **Mod menu open**: logger paused (no rows, no ttl decay because no polls are recorded), column hidden. **Vanilla pause menu**: column hidden; the logger keeps running on polls, but `new_frame` is false while fighters are paused, so nothing changes.
6. **Lobby practice**: the column lives under `TrModPort` in `info_waiting`; the layout's readiness (all of `TrModMenu`, `TrModInputLog`, `TrModDisp0`, `TrModSlider` present) gates the UI; P1 is still `player_idx 0`.
7. **Layout re-registration at the same address** (scene reload): the cache must be invalidated by registration, not by address comparison alone (fork rule; the missing-invalidation control fails the host comparison).
8. **99 cap**: the count saturates at 99 in the row; the per-row counter keeps counting (irrelevant to display).
9. **CPU-as-P1**: the logger logs P1's *controller* mapping; an AI-driven P1 shows an idle controller (consistent with SPEC-input-01 §6.6).

## 7. Persistence

None. The two menu settings are saved in the menu file (menu spec).

## 8. Resource budget

Measured on the fork's host fixture (2,897 panes, `docs/input-display-performance.md`, `docs/input-display-followup.md`); these are pane-operation and host-CPU counts, not Switch FPS.

| Metric | Fork today (measured, source) | Target | How it will be measured |
| --- | --- | --- | --- |
| resident bytes (code + data) | rows ≈ 3.6 KB static; code not isolated — see `docs/resource-costs.md` (W5) | ≤ fork | `tools/build/nro_size.py` per-feature build |
| per-frame work, unchanged held input | pane lookups 212 → 1 (upstream → fork 7.1.2); text writes 2 → 0; pane visits 2 after the direct-child walk | ≤ 1 lookup, 0 writes | diag counters (lookups, visits, writes per tick) |
| held input, frame count advancing | lookups 210 → 3; text writes 2 → 1 | ≤ 3 / 1 | diag counters |
| rapid changing inputs, populated ring | lookups 214 → 19–24; visits 20,382–20,469 → 102–189 per change; 120-frame workload 27.0 ms → 0.216 ms host | ≤ fork | host bench (`scripts/input-display-bench` pattern) |
| history rotation / three fade frames / last-row frame tick | visits 20,314 → 34; 11,602 / 11,988 / 9,284 → 19 / 19 / 18; 2,708 → 19 | ≤ fork | host bench |
| hidden by the mod menu | lookups 210 → 1; text writes 2 → 0 | 1 / 0 | diag counters |
| layout re-registration (one-off) | 2,912 lookups restoring 15 rows (upstream 212, newest row only) | ≤ 3,000 once | diag counters |
| heap allocations in hooks | upstream 24–26 per visible update → fork 0–2 (status-name string, icon list) | 0 steady state; ≤ 2 on content change | counting allocator, install line |

## 9. Test provenance

| Requirement | Host test (planned name) | Eden row (planned label) | Evidence file |
| --- | --- | --- | --- |
| Menu items, defaults | `display::menu_defaults` | IN02-E01 | `evidence/input/IN02-E01.md` |
| New row only on difference, per mode (§3.2) | `display::difference_rules_smash_raw_status` | IN02-E02 (three modes) | |
| Stick binning table and thresholds (§3.3) | `display::bin_stick_values` | IN02-E03 | |
| Icon order, colours, de-duplication, five-slot cap, unknown icon skipped (§3.4) | `display::icon_order_and_colours` | IN02-E03 | |
| Frame count counts in-game frames, caps at 99, keeps ttl on overwrite | `display::frame_count_cap_99` | IN02-E04 | |
| ttl 600, fade curve 200, invisible at 0 | `display::fade_alpha_curve` | IN02-E05 (expiry after ~10 s) | |
| Ring rotation: physical base advances, rows keep panes | `display::ring_rotation_physical_index` | IN02-E02 | |
| Hidden under mod menu and vanilla menu; logger paused under mod menu only | `display::hidden_under_menus` | IN02-E06 | |
| Full refresh after hide and after re-registration restores history in the current mode | `display::refresh_after_hide_and_reregister` | IN02-E07 (mode change while hidden) | |
| Visible state identical to upstream for all ordinary updates (fixture comparison) | `display::renderer_fixture_parity` | — | host bench report |
| Resource rows of §8 | `display::bench_counts` | — | host bench report |
| Status name resolution | `display::status_display_names` | IN02-E08 (Status On, two fighters) | |
| Session clear resets rows and counters | `display::session_clear` | IN02-E09 (lobby enter/leave) | |
| Ice Climbers CPU: counts advance once per frame | — | IN02-E10 | |

## 10. Open questions

1. **Status name table.** Upstream ships a generated 9,000-line table mapping status constants to names. The clean room must not copy it. Recommendation: generate the names at build time from the identifiers of the third-party `smash` constants crate (a build script over its source, or a `strum`-style derive if the crate exposes one), falling back to the numeric id when a name is missing. Lead to confirm licensing of the constants crate.
2. **Frame-count clock source.** Counting in-game CPU frames means the count stalls when the CPU body is absent (hot swap) or during the vanilla pause; it also means P1-only situations (CPU missing) log nothing. Recommendation: keep upstream semantics for parity, but tick from the P1 body's frame in the clean room since the display is about P1. Lead decides.
3. **"Strong" direction tier.** Icons exist; upstream never produces them. Recommendation: not implemented (parity); note for the menu redesign.
4. **Right-stick in Smash mode** comes from the mapped record's right-stick bytes, which the recorder never injects into the CPU (SPEC-input-01 R6). No conflict, but the implementer should not infer from the display that those bytes drive gameplay.
5. **Panic on a missing row pane.** Upstream and fork panic ("Could not find pane"). Recommendation: log once and disable the display for the session instead (root cause: a layout archive mismatch, which the archive injection guard should catch first).
