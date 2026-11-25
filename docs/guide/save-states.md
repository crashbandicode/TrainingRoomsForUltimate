# Save states

Save where both fighters are, then load to return to that spot. The options are on the [Saves](../menu/saves.md) page.

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/2f0394ea36fb533c09e157925c016b4e369ab2a6" autoplay muted loop playsinline controls></video>

*ZL + D-pad Down saves beside the CPU; two jabs take it to 4.9%; ZL + D-pad Up puts both fighters back at 0%. Eden, 2026-11-25, build `c51f4d81`.* [Open the clip](../media/tut-save-load.mp4)

## Saving and loading

1. Turn Saves › [Save States](../menu/saves.md#save-states) On. It is On by default.
2. Put the fighters where you want them.
3. Hold ZL and press D-pad Down to save. Both fighters flash and the notice says "Save State · Saved".
4. Hold ZL and press D-pad Up to load. Both fighters return to the saved spots.

The chords work in Training and in a waiting room with Lobby Practice On. A refused Save or Load normally shows a
notice. A press used by a menu, the speed control or strip scroll-back shows no Save State notice. See
[Troubleshooting](troubleshooting.md#a-notice-says-no). You can change the chords at
[Button chords](controls.md#changing-a-chord).

## The bound save

The Save and Load chords use the *bound save*. Save overwrites it; if nothing is bound, Save makes and binds a new
save. Load with nothing bound leaves the fighters in place and shows "Nothing is saved here yet: Save first, or bind a save".

To bind another save, open the Recording pop-up with ZR + D-pad Right, choose a save and press A. The Recording page
also lists the saves and lets you bind, rename, duplicate or delete them. More:
[Save states & Macros](../menu/recording.md#save-states-macros).

## What a save keeps

| Saved | On load |
| --- | --- |
| Each fighter's position, facing and ground, air or ledge state | Both return to the saved spots. |
| Damage | Used if that fighter's damage on load is Save State. |
| Special-move charges and Kirby's copy ability | Restored when the fighter is the same one that was saved. |
| A supported held item for Player 1, such as a turnip or bomb | Restored with the save. |
| Fighters, stage and exact Mii | Kept in the save; an ordinary load does not change the fighters or stage. |

## Damage on load

Set [Player damage on load](../menu/saves.md#player-damage-on-load) and
[CPU damage on load](../menu/saves.md#cpu-damage-on-load) separately.

| Value | Damage after a load |
| --- | --- |
| Default | 0%, the default for both fighters. |
| Save State | The damage saved for that fighter. |
| Random Value | A new random value within that fighter's damage range. |

<!-- CLIP: tut-save-damage-range.mp4 | CPU damage range: Min and Max edited with taps and a hold -->
*Clip coming.*

To practise against a CPU at 90–110%:

1. Set CPU damage on load to Random Value. Open CPU damage range with A.
2. Up or Down chooses Min or Max; A picks up that handle.
3. Tap Up or Down to change it by 0.5%, or hold to move faster. A or B puts the handle down.
4. Set Min to 90 and Max to 110, then press B to leave the slider.

Each range starts at 0–150%. Player damage can range from 0–255%; CPU damage from 0–150%. A range is greyed unless its
damage row is Random Value. More: [Player damage range](../menu/saves.md#player-damage-range) and
[CPU damage range](../menu/saves.md#cpu-damage-range).

## Mirroring

Saves › [Mirroring](../menu/saves.md#mirroring) flips positions across the middle of the stage and reverses facing.
None (the default) never flips. Alternate flips every other load, starting with the first after Save. Random flips by
chance.

## Buffs

Tick the character buffs you want at Saves › [Buffs on load](../menu/saves.md#buffs-on-load). They apply to both
fighters after a load. Shulk and Wario can have one selected art or waft at a time, chosen at random if several are
ticked. The menu page lists every buff.

## Auto-load on death

Turn Saves › [Auto-load on death](../menu/saves.md#auto-load-on-death) On to load the bound save when either fighter is
KO'd. It is Off by default. Damage, mirroring and buff settings apply. With nothing bound, the fighter respawns instead.

### Respawning

With Auto-load Off, Fighters & Stages › [Respawn on platform](../menu/fighters-and-stages.md#respawn-on-platform)
chooses between immediate respawn and the revival platform. It is greyed while both Save States and Auto-load are On.

## The setup library

A setup is a named practice spot, stored separately from save states. It keeps the fighters, stage and saved positions.
See [Setup library](../menu/saves.md#setup-library) for every option.

### Making a setup

1. Put the fighters where you want them.
2. Open Saves › Save setup here... and press A.
3. Type a name and confirm. Your bound save is unchanged.

To copy the bound save instead, use Saves › Save setup from bound save....

### Using a setup

Open Saves › Setup library, choose a setup and press A:

| Row | What it does |
| --- | --- |
| Restore here | Moves both fighters there when the menu closes; the next Load still uses the bound save. |
| Make Load use it | Leaves both fighters in place; later Load and Auto-load use the setup until you Save or bind another save. |
| Rename / Delete | Renames or removes the setup without changing your saves. |

Restore here needs Save States On and the same stage and fighters, including the same Mii. See
[the setup's own screen](../menu/saves.md#a-setup-s-own-screen) for details.

More: [Saves](../menu/saves.md), [Controls](controls.md).
