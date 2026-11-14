# Display & Controls

This page sets what is drawn on screen while you practise, and the buttons that drive the plugin. Open the menu with
B + D-pad Up (or Minus), then press L once from the CPU page, or R until the title reads Display & Controls.
[Using the menu](../guide/using-the-menu.md) explains the buttons inside the menu.

## On screen

### HUD

Off hides the game's own HUD in Training while you play: the damage and stock panel, the player pointers above the
fighters, the item pointers and the off-screen bubbles. On keeps it. The frame strip and the plugin's notices stay
either way.

- **Values:** On, Off
- **Default:** On
- **How:** A, Left or Right flips it.

![HUD turned Off: the damage and stock panels and the player pointers go, and the frame strip and the plugin's own panels stay](../media/rc11-hud.gif)

### Hitbox Visualization

Draws the fighters' boxes in colour. It is a checklist: tick what you want to see, and each item draws on its own, so
any mix works, such as hurtboxes alone, or hitboxes with their launch angles. It works in Training, and on the two
practice fighters in a waiting room.

- **Values:** Hitboxes, Grab boxes, Hurtboxes, Frame data colours, Launch angles, Windboxes.
- **Default:** none ticked: nothing is drawn.
- **How:** A opens the checklist; A ticks, ZL ticks all or none, B goes back. The row shows how many are ticked, such
  as "3 of 6".

A settings file from before this checklist keeps what it showed: On ticks Hitboxes, Grab boxes and Hurtboxes, and Off
ticks none.

| Box | Colour | With Frame data colours ticked | What it is |
| --- | --- | --- | --- |
| Hitbox | Each hitbox of a move gets its own colour; the first is red | Red when strong, pink when weak | The attack |
| Grab box | Always magenta, a colour no hitbox uses | Purple | The grab |
| Hurtbox | Yellow | White; blue while nothing can hit it; green while hits do no damage | Where a fighter can be hit |
| Windbox | Its hitbox colour; deep blue with Windboxes ticked | Deep blue | A hitbox that pushes or pulls without hitstun |

**Frame data colours** recolours whatever is ticked in the colours of Ultimate Frame Data. A hitbox is strong when it
is the strongest one out on that frame, such as a sword's tip, and weak when another hitbox out on the same frame is
stronger, or when it is weaker than the strongest hitbox earlier in the same move, as the late frames of a knee are.
A hitbox that cannot be ranked, such as a jab's single hitbox, is red. A hurtbox is blue during a spot dodge or a roll,
and on the revival platform; it is green for the moments after leaving the platform. The plugin reads all of this from
the game every frame, so no list of fighters is needed. Ticked alone, it draws nothing.

**Windboxes** draws the windboxes in deep blue. A windbox pushes or pulls without hitstun: the fighter it lands on is
moved but can still act. Most deal no damage, such as F.L.U.D.D.'s water, and some do, such as one of the two flames
of Bowser's Fire Breath. The game marks them itself, and the plugin uses that mark. A windbox is a hitbox, so Hitboxes
draws it too; with both ticked it is drawn once, in deep blue, and with Windboxes alone only the windboxes are drawn.

Some pushes are not hitboxes but wind areas, such as the Gust Bellows item and some fighters' wind effects. These are
not drawn.

**Launch angles** draws a white line inside each hitbox, pointing where the hit sends the fighter it lands on. The line
mirrors when the attacker turns around. Two dashes mean the game picks the angle when the hit lands, from how hard it
hits and whether the target is on the ground; they point at 38 degrees, where such a hit sends a fighter in the air.
A dot means the hit carries the fighter along with the attack. With Hitboxes not ticked, the lines are drawn alone.

<!-- CAPTURE-REQUEST hb-legend-v01: Hitbox Visualization with Hitboxes, Grab boxes, Hurtboxes and Frame data colours ticked, 1/4 speed. The CPU spot-dodging next to a standing Player 1: the CPU's hurtboxes blue while it dodges, white otherwise. Player 1 walks off the stage with Respawn on platform On: blue on the platform, green for the moments after it. Mario's grab: a purple grab box. Marth's forward smash: the tip red, the blade pink. See artifacts/rc14-hb-legend/. -->
<!-- CAPTURE-REQUEST hb-legend-v03: Windboxes alone, 1/4 speed: Mario's F.L.U.D.D. (the water deep blue, nothing else drawn), then Bowser's Fire Breath (only the flame that does not cause hitstun, deep blue). Then Hitboxes alone on F.L.U.D.D. (the water in its hitbox colour). See artifacts/rc14-hb-legend/. -->
<!-- CAPTURE-REQUEST hb-legend-v02: Hitboxes and Launch angles ticked, 1/4 speed: Ganondorf's up tilt (the windbox lines, then the kick's dashes and its 70-degree line) facing right then left; Mario's jab (dashes). Then Hurtboxes alone: only the hurtboxes. See artifacts/rc14-hb-legend/. -->

A round box is a disc; a long one, such as a sword's hitbox or an arm's hurtbox, is one pill. The shapes are flat and
drawn over the stage and the fighters. While the camera pans they trail by a frame or two.

![Hitbox Visualization turned on: the yellow hurtboxes, then at 1/4 speed a neutral air's red hitbox](../media/rc11-hitbox.gif)

![At 1/4 speed, Ridley's hurtboxes as yellow pills, and his tail attacks' hitbox as one red pill along the tail](../media/rc13-pills.gif)

*Pills: at 1/4 speed, Ridley standing shows his hurtboxes as yellow pills; his forward tilt and down tilt draw their hitbox as one red pill along the tail. Eden, rc.13 (untested on hardware).*

**Shield and grab.** While a fighter shields nothing of it is drawn. While it is grabbing, its hurtboxes and grab box
are, since a missed grab can be punished. Grab boxes show only in the grab's active frames: a grab that catches on the
frame its box appears, as Samus's tether does, shows its box for that frame only, not through the pull.

![At 1/4 speed, the magenta grab boxes of Mario's grab, Samus's tether and Olimar's Pikmin; each grabber keeps its yellow hurtboxes](../media/rc13-grab-boxes.gif)

*Grab boxes: at 1/4 speed, Mario's standing grab, Samus's tether (a pill along the beam) and the Pikmin Olimar throws, each grab box in magenta; the grabber keeps its yellow hurtboxes through the grab. Eden, rc.13 (untested on hardware).*

**Projectiles and thrown items.** A projectile, or a thrown item such as Link's bomb, Peach's turnip or Diddy Kong's
banana, has its hitboxes drawn while its attack lasts. Up to four at a time; a fifth replaces the oldest. Olimar's Pikmin
show their grab box.

![At 1/4 speed, a red disc rides on Ridley's fireball, Link's arrow and Link's thrown bomb while each is in flight](../media/rc13-projectile-hitboxes.gif)

*Projectiles and thrown items: at 1/4 speed, a red disc rides on Ridley's fireball, Link's arrow and Link's thrown bomb while each is in flight. Eden, rc.13 (untested on hardware).*

### Frame data colours

On draws hitboxes in the colours of Ultimate Frame Data: a strong hitbox is red and a weak one is pink. A hitbox is
strong when it is the strongest one out on that frame, such as a sword's tip or a knee's sweet spot. It is weak when
another hitbox out on the same frame is stronger, or when it is weaker than the strongest hitbox earlier in the same
move, as the late frames of a knee are. The plugin reads each hitbox's damage from the game, so no list of fighters is
needed. A hitbox it cannot rank, such as a jab's single hitbox or several hitboxes of equal damage, is red. Grab boxes
and hurtboxes keep their colours. It needs Hitbox Visualization On. Off gives each hitbox its own colour, as before.

- **Values:** On, Off
- **Default:** Off
- **How:** A, Left or Right flips it.

<!-- CAPTURE-REQUEST spots-v01: Marth's forward smash at 1/4 speed with Hitbox Visualization On and Frame data colours Off, then On: the tip red and the blade pink on the early frames, the blade pink on the later frames. Captain Falcon's forward air: red on the knee's sweet frames, pink after. Same setup both times. See artifacts/rc14-spots/. -->
<!-- CAPTURE-REQUEST v18-hitbox-legend: a still of one fighter with hitbox, hurtbox and grab box all drawn, annotated with the colours (the first hitbox red, other hitboxes in their own colours, hurtbox yellow, grab magenta) and pointing at the grabber's hurtboxes. See artifacts/docs4/CAPTURE-REQUESTS.md. -->

### Frame Advantage

On shows a notice after each exchange with your frame advantage: plus means you can act before the CPU, minus means
the CPU can act first. The notice stays down while the frame strip is on screen, because the strip's ADVANTAGE readout
shows the same number, so you see it only with Input Display set to Off. Nothing is measured while the CPU has Mash
actions ticked.

![With Input Display Off, a jab into the CPU's shield shows nothing; Frame Advantage turned On, the same jab shows the notice with the frame advantage (-14)](../media/rc11-frame-advantage.gif)

- **Values:** On, Off
- **Default:** Off
- **How:** A, Left or Right flips it.

More: [Frame strip › ADVANTAGE](../guide/frame-strip.md#advantage)

### Input Display

Frame strip draws the last second of frames at the top of the screen: your presses, your frames and the CPU's, how
each follow-up linked, and the frame advantage. It works in Training and in waiting-room practice. It steps aside while
a menu is open, the waiting room's own menu included, and comes back when the menu closes.

- **Values:** Off, Frame strip
- **Default:** Frame strip
- **How:** Left or Right switches it; A opens the list. The Input display chord (R + D-pad Down) switches it without
  opening the menu.

![The frame strip in a waiting room, hidden while the waiting room's menu is open](../media/rc11-strip-under-menu.gif)

More: [Frame strip](../guide/frame-strip.md)

### Spot labels

On writes Sweet, Sour or Late on each hit that connects in the frame strip. The label sits on the grey hitlag block of
the fighter that was hit: your hit on the CPU's lane, the CPU's hit on yours. Sweet is the strongest hitbox out on that
frame, Sour a weaker one beside it, and Late a hit weaker than the strongest hitbox earlier in the same move. The plugin
works out which hitbox connected from the damage the hit added. When that is unclear, such as equal hitboxes or a hit on
a shield, there is no label. A charged smash attack is labelled like an uncharged one: the plugin reads the charge from the game and takes it out of the damage before it works out which hitbox connected. It needs Input Display set to Frame strip.

- **Values:** On, Off
- **Default:** Off
- **How:** A, Left or Right flips it.

More: [Frame strip](../guide/frame-strip.md#spot-labels)

<!-- CAPTURE-REQUEST spots-v02: the frame strip after Marth's forward smash lands with the tip (Sweet) and again with the blade (Sour); then a hit by the CPU on Player 1, labelled on Player 1's lane. Spot labels Off first, then On. See artifacts/rc14-spots/. -->

### Input Delay

Holds Player 1's inputs back by a number of frames, for example to practise as if playing online. Chords are not
delayed. The frame strip and recordings see the inputs as delayed. The frames are game frames at every
[game speed](../guide/game-speed.md): 18 is 18 game frames at 1/4 too, four times as long in seconds.

![The same jab from the same spot: with Input Delay 0 (left) Mario jabs as A lights on the pad; with Input Delay 18 (right) A lights at the same moment and the jab comes 18 frames (0.3 s) later](../media/rc11-input-delay-sbs.gif)

![Side by side at Game speed 1/4, the same jab: with Input Delay 0 (left) Mario jabs as A lights; with Input Delay 18 (right) the jab comes 18 game frames later, about 1.2 s at 1/4](../media/rc13-input-delay-sbs.gif)<br>*At Game speed 1/4, the same A from the same spot: Input Delay 0 (left) jabs at once; Input Delay 18 (right) jabs 18 game frames later, four times as long in seconds as at x1. Eden, rc.13 (untested on hardware).*

- **Values:** 0 to 30 frames
- **Default:** 0
- **How:** Left or Right steps one frame (it stops at 0 and 30); A opens the list.

## Controls

### Button chords

Opens the chord editor: the ten chords for save states, recording, fighter cycles, the input display and game speed.
A chord is "hold one button and press the others".

| Row | Default | What it does |
| --- | --- | --- |
| Save State Save | ZL + D-pad Down | Saves a save state. |
| Save State Load | ZL + D-pad Up | Loads it. |
| Record | ZR + D-pad Down | Starts or stops a recording. |
| Play on CPU | ZR + D-pad Up | Plays the CPU's active recording, or stops it. |
| Play on Player 1 | ZR + D-pad Left | Plays Player 1's active recording, or stops it. |
| Recording pop-up | ZR + D-pad Right | Opens the recording pop-up (B closes it). |
| Cycle Player | ZL + D-pad Right | Your next fighter (needs Player Swapping On). |
| Cycle CPU | ZL + D-pad Left | The CPU's next fighter. |
| Input display | R + D-pad Down | Turns the frame strip on or off. |
| Game speed | R + D-pad Up | Hold it, then D-pad Left or Right changes the speed. |

![L + D-pad Down (lit on the pad) first saves nothing; in the chord editor Save State Save is changed from ZL + D-pad Down to L + D-pad Down, which then saves. A caption names the chord](../media/rc11-button-chords.gif)

- **Values:** any button, or several, for each row; or disabled (an empty chord does nothing)
- **Default:** as in the table
- **How:** A opens the editor, and A on a row opens that chord. Press X, let go of every button, then hold the new
  chord: hold one button and press the others. Let go to see it, and A keeps it. Y empties it, B cancels. New chords
  work once you close the menu. Y on this row (on the page) puts all ten back to your saved defaults.

More: [Controls › Changing a chord](../guide/controls.md#changing-a-chord), [Game speed](../guide/game-speed.md)

### L+R+A Reset

On: holding L, R and A resets. In Training that is the game's own reset. In a waiting room with Lobby Practice on, both
practice fighters go back to where they first stood. Off: the chord does nothing (in Training, the game's reset is
blocked).

![On: after a forward smash, holding L, R and A (lit on the pad) puts both fighters back at 0%; turned Off, the same buttons do nothing. A caption names the chord](../media/rc11-lra-reset.gif)

- **Values:** On, Off
- **Default:** On
- **How:** A, Left or Right flips it.

More: [Controls › Fixed buttons](../guide/controls.md#fixed-buttons)

### Taunts

Off stops Player 1's D-pad taunts in Training and in waiting-room practice, so a D-pad press meant for a chord never
starts one. Kirby holding a copy ability keeps D-pad Up, which drops the hat. The chords and the menu shortcuts work
either way, and online matches are not changed.

- **Values:** On, Off
- **Default:** On
- **How:** A, Left or Right flips it.

![The menu on Taunts [ON]: D-pad Right flips it to [OFF], D-pad Left flips it back to [ON], with no A](../media/rc13-left-right-toggle.gif)<br>*Left or Right flips any On/Off row, as A does: on Taunts, Right turns it Off and Left turns it On again. Eden, rc.13 (untested on hardware).*

![Side by side from the same reset, the same D-pad Up, Side and Down: with Taunts Off (left) Mario stands; with Taunts On (right) each press is a taunt](../media/rc13-taunts-off.gif)

*The same D-pad Up, Side and Down from the same reset, lit on the pad: with Taunts Off (left) Mario does not taunt; with Taunts On (right) each press is a taunt. Eden, rc.13 (untested on hardware).*

![Taunts Off, Kirby: D-pad Up does nothing without a hat; with Falco's copy hat, Side and Down still do nothing and Up drops the hat](../media/rc13-taunts-kirby-hat.gif)

*Taunts Off, Player 1 Kirby: D-pad Up does nothing; with a copy hat, Side and Down still do nothing and Up drops the hat. Eden, rc.13 (untested on hardware).*

### Menu shortcut 1

A button or chord that opens this menu, as well as B + D-pad Up. It is kept in `button_config.ini` in
`sd:/ultimate/TrainingRooms/`, so the menu writes that file. A change works at once.

- **Values:** any button, or several; or disabled
- **Default:** Minus
- **How:** A opens the editor. Press X, let go of every button, hold the new chord, then let go to see it. A keeps it,
  Y empties it (disabled), B cancels. The editor refuses B + D-pad Up ("B + D-pad Up always opens the menu") and the
  chord the other shortcut already has. *Reset page to defaults* leaves the shortcuts as they are.

![Menu shortcut 1 set from Minus to ZL + D-pad Up, then ZL + D-pad Up opens the menu](../media/rc11-menu-shortcut.gif)

More: [Controls › Opening the menu](../guide/controls.md#opening-the-menu)

### Menu shortcut 2

A second button or chord that opens this menu. It works like Menu shortcut 1 and is kept in the same file.

![L + D-pad Up does nothing while Menu shortcut 2 is disabled; set to L + D-pad Up in its editor, the same chord opens the menu. A caption names the chord; the last frame before the menu is held 0.4 s](../media/rc11-menu-shortcut-2.gif)

- **Values:** any button, or several; or disabled
- **Default:** disabled
- **How:** as Menu shortcut 1.

### Permanent shortcut

B + D-pad Up always opens this menu. This row only shows it; it cannot be changed.

![B + D-pad Up (named in a caption; B lit on the pad) opens the menu on the Permanent shortcut row, which cannot be changed. The last frame before the menu is held 0.4 s](../media/rc11-permanent-shortcut.gif)

- **Values:** B + D-pad Up (read-only)
- **How:** nothing to change.
