# Troubleshooting

Find what you see, then try the suggested action. New here? Start with [Getting started](getting-started.md).

## The plugin does nothing, or the game acts up after installing

Look for the install line in `sd:/ultimate/TrainingRooms/events.log`.

| What you see | Why | What to do |
| --- | --- | --- |
| No new install line | The plugin probably did not load. | Check Skyline and the three plugin files in the [install steps](getting-started.md#installing). |
| `result=unsupported_version` | TrainingRooms installs nothing on another game version. | Use Smash Ultimate 13.0.5. |
| `cpu_common=not_installed reason=no_nro_hook` | `libnro_hook.nro` is missing. Some CPU options are unavailable. | Put it next to `libtraining_rooms.nro`. |
| `<name>=not_installed reason=site:<place>` | A game site the feature needs did not match. | Send the install line with your report. |
| The game crashes or acts strangely | Training Modpack and TrainingRooms cannot run together. | Remove `libtraining_modpack.nro` if it is installed. |

## The menu does not open

| What you see | Why | What to do |
| --- | --- | --- |
| B + D-pad Up does nothing | Another menu is open, or Lobby Practice is Off in the waiting room. | Close the other menu. Turn Fighters & Stages › Lobby Practice On in Training. Hold B, then press Up. |
| Minus does nothing | Menu shortcut 1 was changed, or your controller has no Minus. | Use B + D-pad Up; it cannot be changed or turned off. |

More: [Opening the menu](controls.md#opening-the-menu), [Waiting rooms](waiting-rooms.md).

## A chord or a button does nothing

| What you see | Why | What to do |
| --- | --- | --- |
| Cycle Player does nothing | Player Swapping is Off at first. | Turn Fighters & Stages › Player Swapping On. |
| A chord does nothing | A menu or the Recording pop-up is open. | Close it and press the chord again. |
| A D-pad chord waits | Online Deluxe is using that button on its open settings page. | Hide its window or change the chord. See [Controls](controls.md#online-deluxe). |
| Kirby does not take a hat | The chord needs B held with the left stick at rest. | Click the left stick for your Kirby, or the right stick for the CPU. See [Kirby hats](fighter-cycles.md#kirby-hats). |
| No hitboxes are drawn | No visualisation is ticked at first. | Tick an item at Display & Controls › Hitbox Visualization. |
| The input display is gone | It is Off or a menu has hidden it. | Press R + D-pad Down or close the menu. |
| The input display shows old frames | R3 is held during scroll-back. | Let go of R3 to return to now; R3 + D-pad Left / Right scrolls. |

## A row is greyed

Read the help line below the row. It tells you what must change.

| Reason | What to do |
| --- | --- |
| Auto-load on death is On | Turn it Off before changing Respawn on platform. |
| Damage on load is not Random Value | Set the corresponding damage row to Random Value to use its range. |
| A delay range or ticked values are set | Press Y to clear the selection that is in use. |
| A Mii is unavailable | That role has no Mii of the class it can load. See [Mii fighters](mii.md). |

## A notice says no

<!-- CLIP: tut-notice-save-states-off.mp4 | Save States Off → the save chord shows the refusal → Save States On → the chord saves -->
*Clip coming.*

| Notice | Why | What to do |
| --- | --- | --- |
| "Save States is Off (Saves page)" | Save or Load needs Save States On. | Turn Saves › Save States On. |
| "Nothing is saved here yet: Save first, or bind a save" | The slot has no save. | Save with ZL + D-pad Down, or bind a save. |
| "Save states have not started in this match yet" | The save-state tools are not ready. | Wait a moment and try again. |
| "A load is already being placed" | A load is in progress. | Wait, then try again. |
| "A CPU drives Player 1: take Player 1 back to load" | The game's CPU controls Player 1. | Take Player 1 back from the CPU. |
| "Wait for the character swap to finish" | A swap is in progress. | Wait, then try again. |
| "Close the current game menu before swapping" | The game's menu is open. | Close it, then try again. |
| "Select a Kirby hat first" | The hat list is empty. | Add a hat under Fighters & Stages › Kirby Hats. |
| "Kirby hat could not be loaded" | The hat did not load. | Try again. |
| "Wait for the practice fighters to finish loading" | The fighters are not ready. | Wait, then open the list again. |
| "P1 is controlled by an AI: Play on Player 1 is off" or "P1 is controlled by an AI: Quick Reset is off" | An AI plays Player 1. | Take back Player 1 to use that action. Install 4 also refuses Record; the next build lets you record the AI's inputs. |
| "Could not save button_config.ini" | The shortcut file could not be written. | The change lasts until you close the game. |

More: [Restrictions](../RESTRICTIONS.md).

## Fighters load slowly, or the notice says "waiting for memory"

| What you see | Why | What to do |
| --- | --- | --- |
| A loading count or "Paused" | Fighters or hats are loading, or another action must finish first. | Wait for loading to finish. |
| "waiting for memory" or "waiting for pool space" | Some fighters wait until there is room. | If it persists, lower Ready Fighters or Ready Hats. |
| "load after you pause" | Other preloaded fighters or hats hold the room needed. | Pause once. |
| A swap takes time | Cycle Preload is Off, or the cycle exceeds Ready Fighters. | Adjust the [Loading rows](fighter-cycles.md#what-the-loading-rows-do). |

## Waiting rooms

| What you see | Why | What to do |
| --- | --- | --- |
| The menu and chords do nothing | Lobby Practice is Off; a saved Off stays Off. | Turn it On in Training before looking for a match. |
| Practice stops when the match starts | Practice is separate from the match. | Practise again when you return to the room. |
| Plus does not open the game's Quick Play menu | The TrainingRooms menu is open. | Close it with B first. |
| L + R + A is refused | An AI is playing Player 1. | Take back Player 1 to use Quick Reset. |

More: [Waiting rooms](waiting-rooms.md).

## Settings look wrong or went missing

| What you see | Why | What to do |
| --- | --- | --- |
| A row is not what you set | Y restores its saved default; X saves the current settings as defaults. | Set the row again. See [Your defaults](using-the-menu.md#your-defaults). |
| Settings were not read cleanly | The original is kept as `settings.invalid-1.json`, with a new number each time. | Check the preserved file; bad fields return to their defaults. |
| Training Modpack settings did not import | `sd:/ultimate/TrainingRooms/migration.json` marks a completed import. | Check the source folder and the import receipt. If you need to retry, close the game and move the marker aside before restarting. The importer copies missing files; it never overwrites existing files. |

## Where the logs are, and what to send

Both logs are in `sd:/ultimate/TrainingRooms/`:

| File | What it contains |
| --- | --- |
| `events.log` | The install line, including game version, build and installed features. |
| `practice-diagnostics.log` and `practice-diagnostics.previous.log` | Diagnostics from this and the previous start. |

Copy both diagnostics files before starting the game again. Send them with `events.log`, the game version, other
installed plugins, the build tag from the menu, what you did, what happened and the exact notice.

## Report a problem

See [Where the logs are, and what to send](#where-the-logs-are-and-what-to-send).

More: [Using the menu](using-the-menu.md), [Controls](controls.md).
