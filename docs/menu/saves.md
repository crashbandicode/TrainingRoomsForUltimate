# Saves

Use [save states](../guide/save-states.md) to retry a position, or keep named setups without overwriting your bound save.

## Save states

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/2f0394ea36fb533c09e157925c016b4e369ab2a6" autoplay muted loop playsinline controls></video>

*Load returns both fighters after two jabs.* [Open the clip](../media/tut-save-load.mp4)

| Setting | Choices; default in bold | What it does |
| --- | --- | --- |
| <a id="save-states-1"></a>Save States | **On**, Off | Enables Save and Load; turning it off cancels a pending load. |
| <a id="mirroring"></a>Mirroring | **None**, Alternate, Random | Flips positions and facing; Alternate flips the first load after saving, then every other. |
| <a id="auto-load-on-death"></a>Auto-load on death | On, **Off** | KO loads the bound save or chosen setup; without either, the fighter respawns. |

## Damage on load

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/f3c84adb409c0422d29bcf0f85696d8e143511d9" autoplay muted loop playsinline controls></video>

*Load at zero damage or restore the saved percentages.* [Open the clip](../media/rc11-damage-on-load.mp4)

Choose damage separately for each fighter; a range is editable only with Random Value selected.

| Setting | Choices; default in bold | What it does |
| --- | --- | --- |
| <a id="player-damage-on-load"></a>Player damage on load | **Default (0%)**, Save State, Random Value | Starts Player 1 at zero, saved damage, or a random percentage. |
| <a id="player-damage-range"></a>Player damage range | 0-255%, half-percent steps; **0-150%** | Sets the minimum and maximum for Player 1's random damage. |
| <a id="cpu-damage-on-load"></a>CPU damage on load | **Default (0%)**, Save State, Random Value | Chooses how much damage the CPU starts with. |
| <a id="cpu-damage-range"></a>CPU damage range | 0-150%, half-percent steps; **0-150%** | Sets the minimum and maximum for the CPU's random damage. |

## Character buffs

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/04687c818ecd4c08db8cdf628883f00f273ab5a8" autoplay muted loop playsinline controls></video>

*Loading with Arsene ticked fills Joker's gauge.* [Open the clip](../media/rc11-buffs-on-load.mp4)

| Setting | Choices; default in bold | What it does |
| --- | --- | --- |
| <a id="buffs-on-load"></a>Buffs on load | 18 buffs; **none ticked** | Applies selected buffs to either matching fighter after loading. |

| Fighter | Buffs |
| --- | --- |
| Hero | Accelerate, Oomph, Psyche Up, Bounce; casts each ticked spell. |
| Joker / Wii Fit Trainer | Arsene / Deep Breathing. |
| Cloud / Little Mac | Limit / KO Punch. |
| Sephiroth / Min Min | 1-Winged Angel / Power Dragon. |
| Shulk | Jump, Speed, Shield, Buster, Smash; picks one ticked Art. |
| Wario | Mini, Half or Full Waft; picks one ticked charge. |

## Setup library

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/e45c03b0fbda390bc1179aad1ffd48557aa38f6f" autoplay muted loop playsinline controls></video>

*Save setup here… adds Corner trap to the Setup library.* [Open the clip](../media/rc14-save-setup-here.mp4)

Keep a [named setup](../guide/save-states.md#making-a-setup) from the fighters' current positions or the positions Load would restore.

| Setting | Input | What it does |
| --- | --- | --- |
| <a id="save-setup-here"></a>Save setup here... | Name, up to 48 characters | Saves both fighters where they stand without changing the bound save. |
| <a id="save-setup-from-bound-save"></a>Save setup from bound save... | Name, up to 48 characters | Copies both fighters' saved spots. They must have been saved on the same stage. |
| <a id="setup-library-1"></a>Setup library | Choose a named setup | Opens its actions; the footer shows both fighters, stage and spots. |

## A setup's own screen

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/6e8758ca241dbc8d09cc524cf7427b2bb47a224a" autoplay muted loop playsinline controls></video>

*Choosing the setup changes the next Load, not this spot.* [Open the clip](../media/rc14-setup-load-target.mp4)

| Action | Result |
| --- | --- |
| Restore here | Moves both fighters on close; later Load still uses the bound save. |
| Make Load use it | Sets this setup as the Load and Auto-load target without moving either fighter. |
| Rename | Changes the setup's name, not its identity. |
| Delete | Asks before removing the setup; your saves stay intact. |

## Good to know

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/2424854e2dc29b8427a5a56989fb14195ba67d0f" autoplay muted loop playsinline controls></video>

*Save is refused until Save States is turned on.* [Open the clip](../media/tut-notice-save-states-off.mp4)

| If | Then |
| --- | --- |
| Looking for the save files themselves? | Open [Save states & Macros](recording.md#save-states-macros) on Recording. |
| Rebound Save or Load conflicts with [Online Deluxe](../guide/controls.md#online-deluxe)? | Close its settings page before using that chord. |
| Restore here is refused | Check the [restore requirements](../guide/save-states.md#using-a-setup). |
| Respawn on platform is greyed | [Auto-load takes priority](fighters-and-stages.md#respawn-on-platform) while Save States is also On. |
| <a id="reset-page-to-defaults"></a>Reset page to defaults | Confirm to restore this page's [saved defaults](../guide/using-the-menu.md#your-defaults). |
