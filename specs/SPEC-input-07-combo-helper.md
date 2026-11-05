# Combo helper — behavioural specification

- **Spec id:** `SPEC-input-07`
- **Origin:** TrainingRooms' own design. The operator's request of 2026-11-05 ("record a combo and give it an
  optional name and enable "combo helper" which tells you whether you got the combo or how off you were"), answered
  with option A on 2026-11-05 ~13:10Z: DECISIONS `COMBO-HELPER` on `review/menu-proposal`, pictures
  `docs/decisions/COMBO-HELPER-*.png`.
- **Depends on:** SPEC-input-06 (the frame strip). The helper reads the strip's tracker every frame and draws under it.
- **Status:** implemented on `p23/combo-helper`.
- **Spec writer:** tr-lane-yoga (Opus 5.5), 2026-11-05. Clean room: no upstream or fork source was read.

## 1. Purpose

The player lands a combo once and saves it as a **reference**. With the helper On, every later attempt at it is
graded link by link against the reference: whether each link was a true combo (the strip's own verdict), and how
early or late each press of the follow-up came compared with the recording. A bar under the strip shows the links
of the attempt, the result and the last 8 attempts.

## 2. Terms

The strip's terms (SPEC-input-06 §2) hold: **hit**, **link** (`≤ 0` is a true combo), **A₁** (P1's first actionable
frame after the first hit), **press**, **combo window**. New:

| Term | Definition |
| --- | --- |
| Exchange | The hits from the first hit to the last follow-up the strip links. A hit the strip does not link to the one before (on shield, too late after the CPU could act, or no hit before) starts a new exchange; another hit of the same action (a multi-hit move) does not. The tracker counts exchanges (`Tracker::exchange`). |
| Step | One press of a link's follow-up: its button class (attack, special, jump, shield, grab), its aim (neutral, up, down, side: the strip's stick bin with left and right merged, so facing does not matter) and its frame relative to the link's A₁ (negative when buffered). The presses kept for link *k* are those after hit *k* and up to the follow-up hit, oldest first, at most 4. |
| Reference | A saved exchange: per link, the two moves, the verdict, the combo window relative to A₁, and the steps. |
| Offset | An attempt's step frame (from its own A₁) minus the reference step it matches. Positive is late. |
| Tolerance | *On-time tolerance*: 0, 1 or 2 frames. A step whose offset is within it is on time. |

## 3. Attempts

The helper grades only while *Combo helper* is On, a target is set and the strip samples (Training, strip On).

- **Start.** The first link of a new exchange starts an attempt when its first move equals the reference's first
  link's first move. A link that is not its exchange's first (the helper turned On mid-combo) starts nothing.
- **Each link** `k` (the strip's next link in the same exchange) is graded against reference link `k`:
  - **verdict:** the strip's, measured live (TRUE −N or DROP +N). The recorded verdict is shown for comparison only.
  - **wrong move:** the follow-up's move differs from the reference's (e.g. a forward air where it has a neutral
    air: both are `Aerial` to the strip, so aerials differ only through a step's aim).
  - **steps:** each reference step, in order, is matched to the attempt's next unused step with the same button
    class and aim. Matched: its offset. Not found: **missing**. Unmatched attempt steps are **extra** and counted.
  - **true:** the verdict is TRUE and the move is right. **clean:** true, no missing step, every offset within the
    tolerance.
- **Completed:** all reference links were graded. **Stopped:** the exchange closed first (a new exchange began, or
  the CPU stayed actionable for longer than the strip's `DROP_MAX` after the last hit). **Cleared:** the strip was
  cleared (save-state load, scene change, hot swap) while an attempt was open.
- **Result:** "N of M true", and the first link that was not true with its reason (DROP +N, wrong move, a missing
  press) or, when every link was true, the largest offset outside the tolerance.
- **History:** the last 8 results (newest first), the clean streak and the best number of true links. A new target
  clears it.
- **After an attempt:** with *Load save state*, once a Completed or Stopped attempt's exchange has closed, the
  helper requests the existing save-state load (`features::states::request_load`, the Load chord's own path). It
  only acts when save states are enabled, as the chord.

## 4. The reference library (storage)

- `sd:/ultimate/TrainingRooms/combos/<id>.json` on the generic `model::store::Store` (path-safe ids, atomic writes,
  a size cap), and `combos/selected.txt` naming the target. Not inside the named-macro library (its records are
  playback macros, and a combo field there would need a schema bump older readers refuse).
- The record: `schema_version` 1, `name` (1–48 characters), `recorded_on` (optional: P1's fighter at the save,
  metadata only), and `links` (1–16), each with `from`, `to`, `verdict` (`true` with an optional link value, or `drop`
  with its value), `window` (`[start, end]` relative to A₁, or null) and `steps` (`button`, `aim`, `offset`; 0–4).
  Unknown move or button names are refused.
- The format is open and shared (`docs/COMBOS.md`, operator 2026-11-05):
  - unknown fields are ignored, and a rename keeps them;
  - `schema_version` may be left out (it is 1), and a newer one is skipped;
  - `name` may be left out (the file name is used);
  - a file may have any safe name: 1–64 characters, none of `/ \ : * ? " < > |`, no leading dot and no surrounding
    space. Hidden files are never combos.

  The list re-reads the folder at each opening, shows `Name · N links · recorded on X`, and names the first unusable
  file and its reason in one notice.
- A combo is not tied to a fighter: the helper grades whatever fighter P1 is. `recorded_on` comes from the strip's
  per-frame sample (`natives::fighter_kind`, an existing guarded native, the CPU core unit's `utility_get_kind` row;
  it runs behind its own gate, `input_display::p1_fighter` / `KIND_ON`, so without that row a save records no fighter
  and the strip is unchanged). Choosing a combo recorded on another fighter can switch P1 to it (§4a).
- **Save last combo** keeps the most recent exchange that has at least one link: the open one, else the one before.
- **Rename** rewrites only `name` (the id never changes). **Delete** removes the file; deleting the target clears
  `selected.txt` and the helper's target.

## 4a. Switching P1 to the recorded fighter

Operator, 2026-11-05 (`docs/DECISIONS.md` DEFERRED-LIST): "the switch p1 to a combo's recorded fighter should follow
the same logic that the replay macro uses where it prompts the user to swap and they can say yes or no."

- **When it asks.** Only when the user chooses a combo in **Saved combos** (A). The target restored from
  `selected.txt` and the target that *Save last combo* sets never ask.
- **The gate.** It is the named macros' own gate (`macro_panel::fighter::gate`):
  - **Matches:** `recorded_on` names a roster fighter (the names `recorded_on` is written with, ignoring case and
    surrounding spaces) whose kind is P1's, as the roster loader last observed it.
  - **Can switch:** that fighter exists **and** Player Swapping is On. With it Off, the roster loader changes only the
    CPU.
  - **Asks:** the setting **Ask before switching fighter** (Recording › Named macros) is On.
- **No question** when `recorded_on` is missing, when it is P1's fighter, or when P1's fighter is not known yet.
- **Kept without asking**, with the target note saying why:
  - the setting is Off (no note, as for a named macro);
  - Player Swapping is Off ("P1 stays Mario: turn on Player Swapping to switch");
  - the name is not a fighter this build can load ("Ryuu is not a fighter this build can load").
- **The question.** It is the shared **Fighter Mismatch** screen: *Keep current fighter* (the default) or *Switch to
  recorded fighter*, A chooses, B cancels. The footer reads "Recorded on Ryu · P1 is Mario". Nothing is written before
  the answer.
  - **Keep:** the combo becomes the target, and P1 is unchanged.
  - **Switch:** the combo becomes the target. When the menu closes, P1's switch goes to the roster loader's Player role
    with the request a Player Character pick makes (`character_roster::request_player_kind`, `Selection::Direct`). No
    native site is added. A refusal carries the loader's own notice, plus "P1 was not switched to Ryu".
  - **B:** nothing changes, and the list opens again.
- **A question nobody answered** is retired when a later choice, the list closing or the scene ending moves it on.

## 5. Names: the Switch keyboard

- Names come from the plugin's keyboard path: `named_macros::offer_external_name_titled`, the caller-owned naming
  the setup library uses. The keyboard job is `named_macros`' own (one job at a time, the shared keyboard lease);
  the confirmed name comes back through `take_confirmed_name()` with the caller's token, so no completion is lost.
  The header is "Name combo"; the limit is 48 characters (`MAX_NAME_CHARS`).
- *Name (optional)* sets the draft name of the next save. An empty or cancelled name keeps the previous draft (or
  none); the save then uses "Combo N", the lowest N not in use. The draft lives in the menu worker, not in
  `settings.json`, and is cleared by a save.
- In the list, X renames the row with the same keyboard.
- Names are never written to the diagnostics log.

## 6. Menu: the Recording page's "Combos" group

| Row | Control | Setting / action |
| --- | --- | --- |
| Name (optional) | editor | the keyboard, §5 |
| Save last combo... | editor | §4; the saved combo becomes the target |
| Saved combos | editor | the list: A target, X rename, Y delete, B back |
| Combo helper | toggle | `combo_helper` (Off by default) |
| Target combo | editor (shows the target's name) | opens the list |
| On-time tolerance | dropdown | `combo_tolerance`: Exact / 1 frame / 2 frames (1 frame by default) |
| After an attempt | dropdown | `combo_after`: Keep playing / Load save state (Keep playing by default) |

The rows are in Training; in waiting-room practice the strip does not sample, so the editors refuse with a notice.

## 7. The combo bar (HUD)

- Under the strip, inside its container `TrIv`: its own null container `TrCb` and panes named `TrCb…`, so it hides
  with the strip and the layout keeps three root containers. 864 × 132 canvas units, centred 184 units below the
  strip's centre.
- Header: the target's name and the tolerance; on the right, the last 8 results as pills (green clean, amber true
  with an offset, red not true, grey stopped or cleared) and the clean streak.
- Four link cells (the attempt's last four links, or the reference's first four before any attempt): the link
  number and moves, the verdict (TRUE −N, DROP +N, WRONG MOVE), a timing ruler from −10 to +20 frames from A₁ with
  the combo window (green), the recorded first step (outline) and the attempt's (filled), and the first step's
  offset ("Jump 2f late", "on time").
- Footer: the result and its reason (§3).
- Shown when the strip is shown, the helper is On and a target is set. Panes are written only when they change
  (the strip's `Renderer` rule); about 50 panes.

## 8. Observability (no diag prints)

The helper reports through five exported functions that do nothing but take their arguments, so an Eden guest
trace point (`libtraining_rooms.nro!<name>` with the unstripped ELF loaded) records each event's registers:

| Symbol | x0 | x1 | x2 | x3 |
| --- | --- | --- | --- | --- |
| `tr_combo_link_v1` | link index (0-based) | verdict link value (TRUE ≤ 0, DROP ≥ 1; `i32::MIN` unknown) | flags: bit0 true, bit1 clean, bit2 wrong move, bit3 a missing step | first step's offset (`i32::MIN` none) |
| `tr_combo_attempt_v1` | outcome: 1 completed, 2 stopped, 3 cleared | true links | reference links | first link not true + 1 (0 none) |
| `tr_combo_saved_v1` | links saved | first link's verdict value | 0 | 0 |
| `tr_combo_listed_v1` | files listed | files skipped (one notice names the first) | listed files whose names the plugin did not choose (copied in) | 0 |
| `tr_combo_fighter_v1` (§4a) | event: 1 no question, 2 kept without asking, 3 asked, 4 Keep, 5 Switch, 6 cancelled, 7 switch asked of the loader | the recorded fighter's kind (`u32::MAX` none or unknown) | P1's kind (`u32::MAX` unknown) | event 2: why (1 setting Off, 2 Player Swapping Off, 3 unknown name); event 7: 1 queued, 0 refused |

The rows also read `combos/*.json` and the HUD.

## 9. Interplay with the CPU pilot

The strip reads P1's mapped inputs after the standalone CPU pilot writes them (`input_delay` callers run after the
mapping function and the pilot's hook), so a pilot-driven P1 is graded like a player. B4-1 refuses only the input
recorder (numbered slots and named macros) while the pilot drives P1; *Save last combo* is not a recording and is not
refused. Whether it should be is an open design question (§10).

## 10. Open

- Saving a combo the CPU pilot made: allowed today (§9). The operator may want it refused like recording.
- The keyboard in a diag build opens only with `TrainingRooms/diag-keyboard.on` (the named-macro rule); otherwise
  the row's naming is confirmed with the diag name hook the named library uses.
- Pikachu's nair loop frame data, whether the strip marks Pikachu actionable between the jump and the nair, and the
  pane positions are unproven until the combined Eden session.
