# rc.9 (draft)

Notes for players. rc.9 is the internal name; the release's version name is set when it ships. Until then the menu's
footer shows this build as `0.2.0-rc.6 · rc9` and the commit it was built from. rc.9 fixes the places where a
TrainingRooms shortcut and another plugin's shortcut on the same buttons both fired, and it ships beside new builds of
three of those plugins. The fix IDs (C3, C4, …) are the numbers of the conflict survey the fixes came from. What rc.8
changed is [RELEASE-NOTES-rc8.md](RELEASE-NOTES-rc8.md).

## Fixed

### The D-pad chords and Online Deluxe's window (C4)

- With Online Deluxe 0.10.5, TrainingRooms' chords that use the D-pad don't fire while Deluxe's window shows a page
  with rows: Settings, Window Options, Plugins, or a plugin's own page such as CPU Pilot.
  - Those chords are save and load state, Record, Play on CPU, Play on Player 1, the Recording pop-up, the CPU and
    player cycles, the input display and the Game speed hold.
  - On those pages Deluxe uses D-pad Up/Down and ZR + D-pad Left/Right itself.
- **Before**, one press did both.
  - On Deluxe's CPU Pilot page, ZR + D-pad Right handed Player 1 to the CPU while TrainingRooms opened the Recording
    pop-up.
  - On Settings, ZR + Left/Right changed Deluxe's input buffer or rendering profile while Play on Player 1 started.
- **Not affected:**
  - Deluxe's Performance view has no rows, so the chords work while it's on screen.
  - TrainingRooms' menu shortcuts (Minus, and B + D-pad Up) always work.
- To use the D-pad chords again, hide Deluxe's window: L + R + D-pad Down until it closes.
- With an older Online Deluxe, or none, nothing changes.

### GameCube controller: both triggers are Online Deluxe's L + R (C3)

- On a GameCube controller, Online Deluxe's L + R chords are the two triggers, and TrainingRooms reads the triggers as
  ZL and ZR. So:
  - opening Deluxe's window (both triggers + D-pad Down) also saved a state and started a recording;
  - both triggers + D-pad Up also loaded a state and played the recording on the CPU;
  - both triggers + Left/Right also cycled the fighters.
- **Now:** while both triggers are held, no ZL or ZR chord fires, nor the Input display and Game speed chords, and the
  speed hold takes no step.
- **Unchanged:** one trigger works as before (left trigger + D-pad Down still saves a state), and Pro Controllers and
  Joy-Con are unchanged.
- This is checked by the host tests. It hasn't been tried on a GameCube controller.

### Play on Player 1 and Quick Reset are refused while an AI drives Player 1 (C14)

- As with Record in rc.8, Play on Player 1 and Quick Reset are refused while an AI drives Player 1, for example the
  standalone CPU pilot.
- The notices read "P1 is controlled by an AI: Play on Player 1 is off" and "P1 is controlled by an AI: Quick Reset is
  off".
- While Quick Reset is refused, Player 1's input is left to the pilot.
- Before, both wrote Player 1's input at the same time as the pilot.

### Online Deluxe's L + R + D-pad Left / Right (C10)

- Deluxe's L + R + D-pad Left/Right fires no TrainingRooms chord on any controller and doesn't step the Game speed
  hold. It picks which online opponent Deluxe's window reports.
- It was checked, and nothing changed for it besides C3.

## Good to know

### Leaving a Quickplay room with the TrainingRooms menu open (C13)

- The QuickPlay cancel fix plugin keeps the game's Pause › Leave Battle available for a few seconds after you join a
  Quickplay room.
- While the TrainingRooms menu is open, Plus doesn't reach the game. Close the menu first, then use Pause › Leave
  Battle.

### The build in the menu footer

- The build tag at the foot of every menu page now names rc.9, for example `0.2.0-rc.6 · rc9 01950601`.

## The plugins that ship beside it

### Online Deluxe 0.10.5

- **Its window state, for other plugins (C4):** Deluxe now tells other plugins whether its window is drawn, whether
  the page has rows, and whether the Debug view is open. TrainingRooms rc.9 and the CPU pilot 0.2.4 use it.
- **Row selection (C5):** D-pad Up/Down no longer move Deluxe's row selection while ZL, ZR, R or L is held.
  - Those presses are TrainingRooms' chords, or a trigger on a GameCube controller.
  - A moved selection was the row that the next ZR + Left/Right then changed.
- **The Debug view is off by default (C6, C7).** Set `debug_view=on` in Deluxe's `config.ini` to get it.
  - **Off:** L + R + Plus and L + R + Minus do nothing in Deluxe. L + R + Minus no longer also opens Deluxe's Debug
    view beside TrainingRooms' menu, and L + R + Plus only pauses.
  - **On:** a Debug chord pressed with A held is ignored, so the replay viewer's quit (L + R + A + Plus) no longer
    opens it.
- **The waiting-room practice (C11):** it never lets ZL + D-pad change Deluxe's native character-select panel. This was
  checked in Deluxe's code and is now pinned by a test; nothing changes in play.

### CPU pilot 0.2.4

- While Online Deluxe's window is on screen, L + R + X no longer steps the CPU level in battle, because the press
  belongs to the window (C8).
- On the CPU Pilot page's P1 row, Deluxe's own request is the one step, so a press steps the level once instead of
  twice.
- With the window hidden, L + R + X steps as before.

### imgui_smash 1.0.2

- Holding one stick click and pressing the other (L3 + R3) no longer turns on ImGui's controller navigation (C15).
  - With it on, the D-pad and face buttons also drove Online Deluxe's window while the game still got every press.
  - TrainingRooms' Kirby hat shortcut (B + a stick click) could turn it on by accident.
- To bring the toggle back, set `gamepad_nav_toggle = on` in `sd:/ultimate/imgui_smash/config.ini`.

## For developers

- **The host tests:** the suite went from 41,918 tests to 865, under the 1,000 limit.
  - The cut first took it to 942. Every cut and the reason for it is in [TEST-CUT-LEDGER-rc9.md](TEST-CUT-LEDGER-rc9.md).
  - The cut tests then came back as table-driven tests.
- **Coverage:** line coverage is 91.5% with those 865 tests (90.8% before the cut, 34.2% just after it). How it was
  measured, and what keeps it below 95%, is in [COVERAGE-rc9.md](COVERAGE-rc9.md).
- **Where the rules live:**
  - The chord rules are `model::chords::passes_row` (C3, C4) and `model::game_speed::Hold` (the speed hold).
  - Deluxe's export is read by `model::deluxe_overlay` and `runtime::deluxe_overlay`: looked up once, then again after
    a miss at most once a second. A missing export means no window.
- **The C14 refusals** use the same AI check as Record's: `recording::p1_driven_by_ai`, which also counts the game's
  own CPU flag on Player 1.
- **The footer's build name** comes from `build.rs` (`BUILD_NAME`). `build.rs` now reads `CARGO_MANIFEST_DIR` at run
  time, so a target directory shared between checkouts tags each build with its own commit.
