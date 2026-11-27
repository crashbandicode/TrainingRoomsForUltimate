# Display & Controls

Choose what you see during practice and which buttons control it.

## On screen

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/b5760c58e2de36d18ee1fc677043ffb7c7376dce" autoplay muted loop playsinline controls></video>

*Mario's jab and grab show different boxes.* [Open the clip](../media/tut-hitbox-visualization.mp4)

| Setting | Choices; default in bold | What it does |
| --- | --- | --- |
| <a id="hud"></a>HUD | **On**, Off | Off hides Training's damage panels, pointers and off-screen bubbles, not practice overlays. |
| <a id="hitbox-visualization"></a>Hitbox Visualization | Hitboxes, Grab boxes, Hurtboxes, Launch angles, Windboxes, Article hurtboxes; **none** | Draws the ticked layers; the checklist shows their colours. |
| Fighter windboxes | Crownerang, Blunderbuss, Palutena's Counter, Greninja's down taunt | Sky blue shows where a move pushes or pulls without causing hitstun. |
| Wind areas | Triforce of Wisdom, Kaboom and items appear in olive. | Stage hazards such as Pac-Land's fire hydrant aren't drawn. |

## Colour legend

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/92d7297c170fb3d7be983666052e3c45c7df40c1" autoplay muted loop playsinline controls></video>

*The checklist shows the colour key for selected layers.* [Open the clip](../media/tut-hitbox-colour-legend.mp4)

| <a id="hitbox-strength-colours"></a>Colour | How to read it |
| --- | --- |
| Red / pink | Strongest launch / weaker hit; an unranked hit stays red. |
| Purple | Active grab. |
| Grey / yellow / green | Can be hit / cannot be hit / takes no damage. |
| Sky blue / olive | Windbox / wind area; both push or pull without hitstun. |
| White | Launch or wind direction; a dot can mean the attack carries its target. |

## Frame strip

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/d0bf47fda93c3d22e02adf41433160fe425fd865" autoplay muted loop playsinline controls></video>

*Two jabs fill the strip and produce a TRUE link.* [Open the clip](../media/tut-input-display.mp4)

Learn to read the lanes, link timing and advantage in the [frame strip guide](../guide/frame-strip.md).

| Setting | Choices; default in bold | What it does |
| --- | --- | --- |
| <a id="input-display"></a>Input Display | **Frame strip**, Off | Shows both fighters' recent frames and inputs during practice. |
| <a id="frame-advantage"></a>Frame Advantage | On, **Off** | Shows who can act first when the strip is hidden and Mash is empty. |
| <a id="spacing-grader"></a>Spacing grader | On, **Off** | Grades player-controlled hits on the attacker's lane; keeps the strip visible during play. |
| <a id="spot-labels"></a>Spot labels | On, **Off** | Marks identifiable Sweet, Sour or Late hits on the defender's lane. |
| <a id="link-history"></a>Link history | **On**, Off | Shows the last four follow-ups; Off keeps only the newest pair in LINK. |

## Input timing

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/1275892b7d895b7e839248090d7861d88f01c3b2" autoplay muted loop playsinline controls></video>

*Input Delay changes from zero to ten frames.* [Open the clip](../media/tut-input-delay.mp4)

| Setting | Values; default in bold | What it does |
| --- | --- | --- |
| <a id="input-delay"></a>Input Delay | 0-30 game frames; **0** | Delays Player 1's inputs before the strip and Record see them; chords stay immediate. |

## Controls

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/3a68fe9e37b78333c5a0ca217f7349b12124fe89" autoplay muted loop playsinline controls></video>

*A different Save chord works after rebinding.* [Open the clip](../media/tut-controls-change-chord.mp4)

Use the [chord editor](../guide/controls.md#changing-a-chord) to replace or disable a binding.

| Setting | Choices; default in bold | What it does |
| --- | --- | --- |
| <a id="button-chords"></a>Button chords | Ten editable chords | Opens the bindings listed below. |
| <a id="l-r-a-reset"></a>L+R+A Reset | **On**, Off | Resets Training; restores waiting-room starting spots unless AI controls Player 1. |
| <a id="taunts"></a>Taunts | **On**, Off | Off blocks practice taunts; Kirby still drops his copy hat with Up. |

| Binding | Factory chord | Action |
| --- | --- | --- |
| Save State Save | ZL + D-pad Down | Writes the bound save or creates one. |
| Save State Load | ZL + D-pad Up | Loads the bound save or [chosen setup](saves.md#setup-library-1). |
| Record | ZR + D-pad Down | Starts or stops a take. |
| Play on CPU | ZR + D-pad Up | Plays or stops the CPU's active macro. |
| Play on Player 1 | ZR + D-pad Left | Plays or stops Player 1's active macro. |
| Recording pop-up | ZR + D-pad Right | Opens the recording library over practice. |
| Cycle Player | ZL + D-pad Right | Advances Player 1's cycle when [Player Swapping](fighters-and-stages.md#player-swapping) is On. |
| Cycle CPU | ZL + D-pad Left | Advances the CPU's cycle. |
| Input display | R + D-pad Down | Toggles the frame strip. |
| Game speed | Hold R + D-pad Up | While held, Left or Right changes [speed](../guide/game-speed.md). |

## Menu shortcuts

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/29fadaf711f22a5ea361c3aaebe670a04c07bdc1" autoplay muted loop playsinline controls></video>

*A new shortcut opens the menu.* [Open the clip](../media/rc11-menu-shortcut.mp4)

| Setting | Choices; default in bold | What it does |
| --- | --- | --- |
| <a id="menu-shortcut-1"></a>Menu shortcut 1 | Custom chord or disabled; **Minus** | Opens the menu; changes take effect immediately. |
| <a id="menu-shortcut-2"></a>Menu shortcut 2 | Custom chord or **disabled** | Adds another opener; it must differ from both other shortcuts. |
| <a id="permanent-shortcut"></a>Permanent shortcut | **B + D-pad Up**, read-only | Always opens the practice menu; cannot be reassigned. |
| <a id="reset-page-to-defaults"></a>Reset page to defaults | Confirm or cancel | Restores [saved defaults](../guide/using-the-menu.md#your-defaults); leaves menu shortcuts unchanged. |
