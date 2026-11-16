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

Draws the fighters' boxes and, optionally, hittable objects' hurtboxes in colour. It is a checklist: tick what you want to see, and each item draws on its own, so
any mix works, such as hurtboxes alone, or hitboxes with their launch angles. It works in Training, and on the two
practice fighters in a waiting room.

- **Values:** Hitboxes, Grab boxes, Hurtboxes, Frame data colours, Launch angles, Windboxes, Article hurtboxes.
- **Default:** none ticked: nothing is drawn.
- **How:** A opens the checklist; A ticks, ZL ticks all or none, B goes back. The row shows how many are ticked, such
  as "3 of 7".

Press **A** on **Hitbox Visualization** to read the **Colour legend** beside the checklist; **A** on **Frame data colours** changes its key along with the drawing. Its swatches follow the ticks, including strong/weak and hurtbox status colours. Fighter and article hurtboxes share the same key.

![Hitbox Visualization with Frame data colours unticked: the checklist beside thirteen labelled colour swatches](../media/rc15-colour-legend-normal.png)

*Colour legend with Frame data colours Off: eight hitbox colours, grab and shared hurtbox colours, windboxes, wind areas and direction lines. Eden, 2026-11-15, rc.15 source `35268287`, diag `1660a9e4`; untested on hardware.*

![Frame data colours ticked: red strong and pink weak hitboxes, purple grab boxes, and white, blue and green hurtbox swatches](../media/rc15-colour-legend-frame-data.png)

*The same checklist with Frame data colours On: the legend changes as the tick changes. Eden, 2026-11-15, rc.15 source `35268287`, diag `1660a9e4`; untested on hardware.*

The current menu names each colour and explains it. The older captures above and below show the previous palette: wind areas are now burnt orange, direction lines silver, and invulnerable hurtboxes teal.

A settings file from before this checklist keeps what it showed: On ticks Hitboxes, Grab boxes and Hurtboxes, and Off
ticks none. Existing checklists keep their choices; Article hurtboxes starts unticked.

| Box | Colour | With Frame data colours ticked | What it is |
| --- | --- | --- | --- |
| Hitbox | Each hitbox of a move gets its own colour; the first is red | Red when strong, pink when weak | The attack |
| Grab box | Always magenta, a colour no hitbox uses | Purple | The grab |
| Hurtbox | Yellow | White; sky blue while nothing can hit it; teal while hits do no damage | Where a fighter can be hit |
| Article hurtbox | Yellow | The same status colours as a fighter's hurtbox | Where an article or item can be hit |
| Windbox | Its hitbox colour; deep blue with Windboxes ticked | Deep blue | A hitbox that pushes or pulls without hitstun |
| Wind area | Burnt orange | Burnt orange | A box or circle that pushes fighters |
| Launch / wind direction | Silver | Silver | The direction of a launch or push |

**Frame data colours** gives strong and weak hitboxes different colours, and shows whether a hurtbox can be hit. A hitbox is strong when it
is the strongest one out on that frame, such as a sword's tip, and weak when another hitbox out on the same frame is
stronger, or when it is weaker than the strongest hitbox earlier in the same move, as the late frames of a knee are.
A hitbox that cannot be ranked, such as a jab's single hitbox, is red. A hurtbox is blue during a spot dodge or a roll,
and on the revival platform; it is teal for the moments after leaving the platform. The plugin reads all of this from
the game every frame, so no list of fighters is needed. Ticked alone, it draws nothing.

**Windboxes** draws the windboxes in deep blue. A windbox pushes or pulls without hitstun: the fighter it lands on is
moved but can still act. Most deal no damage, such as F.L.U.D.D.'s water, and some do, such as one of the two flames
of Bowser's Fire Breath. The game marks them itself, and the plugin uses that mark. A windbox is a hitbox, so Hitboxes
draws it too; with both ticked it is drawn once, in deep blue, and with Windboxes alone only the windboxes are drawn.

These moves have a windbox in the game's own move data. A few moves whose names the data does not give are left out.
Sheik's Burst Grenade is an item: its pulls are windboxes of the grenade, drawn like any item's. R.O.B.'s Gyro has none:
its spin on the ground hits with hitstun, so it is drawn under Hitboxes.
King K. Rool's Blunderbuss pulls with a windbox (and grab boxes catch who it pulls in). His Crownerang has a small
windbox on his arm while he throws the crown. Palutena's Counter and Greninja's down taunt have one too.

| Fighter | Moves with a windbox |
| --- | --- |
| Bowser | neutral special (one of the two flames) |
| Cloud | neutral special, down special |
| Corrin | up special |
| Dark Samus, Samus | Final Smash (before the beam) |
| Ganondorf | up tilt |
| Greninja | up special (the water), down taunt |
| Hero | Kaboom; Whack and Thwack |
| Ice Climbers | down special |
| Incineroar | side special |
| Inkling | neutral special (the ink) |
| Jigglypuff | Final Smash |
| Kazuya | the last hit of Flash Punch Combo, Final Smash |
| Ken | Final Smash |
| King Dedede | neutral special (Inhale) |
| King K. Rool | neutral special (Blunderbuss, held), side special (Crownerang) |
| Kirby | neutral special (Inhale) |
| Little Mac | dash attack, side special |
| Lucas | down special |
| Luigi | down special |
| Mario | down special (F.L.U.D.D.) |
| Mega Man | up aerial |
| Meta Knight | neutral special |
| Mii Brawler | side special (Suplex) |
| Mii Gunner | down special (Absorbing Vortex, as it ends) |
| Mii Swordfighter | neutral special (Shuriken of Light) |
| Ness | down special (as it ends) |
| Pac-Man | dash attack, down special (the hydrant's water) |
| Palutena | forward smash, down smash, down special (Counter), Final Smash |
| Piranha Plant | side special, Final Smash |
| Pit, Dark Pit | down special |
| Pyra and Mythra | side special (Pyra's sword), Final Smash |
| Robin | up special (where it lands) |
| Rosalina | neutral special (calling Luma back) |
| Sephiroth | Final Smash |
| Sheik | up special, side special (the grenade's pull, before it explodes) |
| Squirtle | neutral special |
| Toon Link | down aerial (as it lands) |
| Wii Fit Trainer | neutral special (while charging) |
| Zelda | down special (the Phantom) |

Some moves push through a wind area instead of a hitbox. Windboxes draws those too: a see-through burnt-orange box or
circle where the wind blows, with a silver line, edged in dark so it shows on a pale stage, pointing the way it pushes. A
dot in the middle means the wind has no one direction. The wind areas of what a fighter throws or summons are drawn too,
such as the pillar of Ness's PK Fire, Samus's bomb and Palutena's Explosive Flame. Zelda's Triforce of Wisdom, which
pulls, and Hero's Kaboom make theirs the same way: they take the same path, but were not filmed on their own. An item's
wind area is drawn the same way, such as the one Sheik's Burst Grenade makes when it explodes; other items, such as the
Gust Bellows or a Poké Ball's Pokémon, take that path too and were not filmed on their own.
Stage hazards are not drawn, such as Pac-Land's fire hydrant.

These fighters have moves with a wind area. Eden tried the special moves and smashes of every fighter whose moves can
make one, and Ganondorf's up tilt; aerials, throws and the other tilts were not tried, so a move missing here may still
have one.

| Fighter | Moves with a wind area |
| --- | --- |
| Bayonetta | neutral special |
| Bowser Jr. | neutral special |
| Chrom | neutral special (Flare Blade), down special, up smash, down smash |
| Cloud | neutral special, down special |
| Corrin | neutral special |
| Dr. Mario | side special |
| Falco | up special |
| Fox | up special |
| Ganondorf | neutral special, up special, down special, up tilt |
| Greninja | side special, up smash |
| Ike | neutral special, side special, down special, forward smash, down smash |
| Ken | every special, forward smash, up smash, down smash |
| King Dedede | neutral special, up special |
| Kirby | neutral special |
| Lucas | neutral special, side special, down special, forward smash, down smash |
| Lucina | neutral special (when held), down special, forward smash, up smash, down smash |
| Mario | side special |
| Marth | neutral special (when held), down special, forward smash, up smash, down smash |
| Meta Knight | neutral special |
| Mewtwo | neutral special, down special, forward smash, up smash, down smash |
| Ness | side special (the PK Fire pillar) |
| Palutena | every special, forward smash, up smash, down smash |
| Robin | up smash, down smash |
| Roy | neutral special, down special, up smash, down smash |
| Ryu | every special, forward smash, up smash, down smash |
| Samus | down special (the bomb) |
| Sheik | side special (the Burst Grenade as it explodes) |
| Shulk | forward smash, up smash |
| Wario | down special |

**Launch angles** draws a silver line inside each hitbox, pointing where the hit sends the fighter it lands on. The line
mirrors when the attacker turns around. Two dashes mean the game picks the angle when the hit lands, from how hard it
hits and whether the target is on the ground; they point at 38 degrees, where such a hit sends a fighter in the air.
A dot means the hit carries the fighter along with the attack. With Hitboxes not ticked, the lines are drawn alone.

<!-- CAPTURE-REQUEST hb-legend-v01: Hitbox Visualization with Hitboxes, Grab boxes, Hurtboxes and Frame data colours ticked, 1/4 speed. The CPU spot-dodging next to a standing Player 1: the CPU's hurtboxes blue while it dodges, white otherwise. Player 1 walks off the stage with Respawn on platform On: blue on the platform, green for the moments after it. Mario's grab: a purple grab box. Marth's forward smash: the tip red, the blade pink. See artifacts/rc14-hb-legend/. -->
![Windboxes ticked at 1/4 speed: F.L.U.D.D.'s water as a deep-blue disc pushing the CPU; Ganondorf's up tilt with deep-blue windboxes inside the see-through wind-area box and its white line; then the same up tilt with Hitboxes ticked too, the kick's hit drawn](../media/rc14-windboxes.gif)<br>
*Windboxes on, at 1/4 speed: F.L.U.D.D.'s water and Ganondorf's up-tilt wind push without hitstun, drawn deep blue; the wind area is the see-through box with its white line. With Hitboxes ticked too, the kick's hit is drawn as well (the last frames). Eden, rc.14 (untested on hardware).*

![Windboxes only at 1/10 speed: King K. Rool holds B and the Blunderbuss becomes a vacuum; its deep-blue windbox pulls Mario in until he is caught](../media/rc14-windbox-krool-vacuum.gif)<br>
*Windboxes only, 1/10 speed: after the cannonball hits, holding B turns the Blunderbuss into a vacuum. Its windbox (deep blue) pulls Mario in without damage until he's caught. Eden, rc.14 (untested on hardware).*
<!-- CAPTURE-REQUEST hb-legend-v03: Windboxes alone, 1/4 speed: Mario's F.L.U.D.D. (the water deep blue, nothing else drawn), then Bowser's Fire Breath (only the flame that does not cause hitstun, deep blue), then Ganondorf's up tilt (its deep blue windbox discs and the see-through blue box with its white line, which shows where the wind blows), then Chrom's Flare Blade held to charge (the see-through blue box on him with its white line pointing up and to the left). Then Hitboxes alone on F.L.U.D.D. (the water in its hitbox colour). Shoot it over the white Training grid on the build with the stronger wind-area colour. See artifacts/rc14-hb-legend/. -->
<!-- CAPTURE-REQUEST hb-legend-v02: Hitboxes and Launch angles ticked, 1/4 speed: Ganondorf's up tilt (the windbox lines, then the kick's dashes and its 70-degree line) facing right then left; Mario's jab (dashes). Then Hurtboxes alone: only the hurtboxes. See artifacts/rc14-hb-legend/. -->

A round box is a disc; a long one, such as a sword's hitbox or an arm's hurtbox, is one pill. The shapes are flat and
drawn over the stage and the fighters. While the camera pans they trail by a frame or two.

![Hitbox Visualization's checklist with Hitboxes, Grab boxes and Hurtboxes ticked: the yellow hurtbox pills, then at 1/4 speed a neutral air's red hitbox](../media/rc14-hitbox.gif)<br>
*Display & Controls > On screen > Hitbox Visualization, A: the checklist (its help line: Tick what to draw. Windboxes: hits that push or pull without hitstun. Launch angles: where each hit sends.). A ticks Hitboxes, Grab boxes and Hurtboxes, B goes back, and the menu closes onto both fighters' hurtboxes as yellow pills, one pill for each long limb or the torso. At 1/4 speed Mario's short hop and neutral air show its red hitbox; the menu footer reads 0.2.0-rc.14 · rc14 17e50cd0. Eden, rc.14 (untested on hardware).*

![At 1/4 speed, Ridley's hurtboxes as yellow pills, and his tail attacks' hitbox as one red pill along the tail](../media/rc13-pills.gif)

*Pills: at 1/4 speed, Ridley standing shows his hurtboxes as yellow pills; his forward tilt and down tilt draw their hitbox as one red pill along the tail. Eden, rc.13 (untested on hardware).*

**Shield and grab.** While a fighter shields nothing of it is drawn. While it is grabbing, its hurtboxes and grab box
are, since a missed grab can be punished. Grab boxes show only in the grab's active frames: a grab that catches on the
frame its box appears, as Samus's tether does, shows its box for that frame only, not through the pull.

![At 1/4 speed, the magenta grab boxes of Mario's grab, Samus's tether and Olimar's Pikmin; each grabber keeps its yellow hurtboxes](../media/rc13-grab-boxes.gif)

*Grab boxes: at 1/4 speed, Mario's standing grab, Samus's tether (a pill along the beam) and the Pikmin Olimar throws, each grab box in magenta; the grabber keeps its yellow hurtboxes through the grab. Eden, rc.13 (untested on hardware).*

**Projectiles and thrown items.** A projectile, or a thrown item such as Link's bomb, Peach's turnip or Diddy Kong's
banana, has its hitboxes drawn while its attack lasts. Up to four at a time; a fifth replaces the oldest. Olimar's Pikmin
show their grab box. **A** on **Article hurtboxes** ticks the row and draws the enabled hurtboxes the game gives articles and items, including
objects without an active attack. It works in Training; waiting-room objects whose owner is unknown stay hidden.
An object with no enabled hurtbox group draws none. These use the same discs, pills and status colours as fighters.
At most eight endpoints per object and 24 together are admitted, always as complete shapes. They take spare object
slots and draw after the other boxes, so attacking objects and fighters keep priority.

![Olimar's three Pikmin with Article hurtboxes off, yellow hurtbox pills on, status colours on, and off again](../media/rc15-article-hurtboxes.gif)

*Article hurtboxes: Off, yellow pills, Frame data colours (blue for the Pikmin's current hit status), then Off again. Eden, 2026-11-15, rc.15 source `6def8553`, diag `48ea01d1`; untested on hardware.*

![At 1/4 speed, a red disc rides on Ridley's fireball, Link's arrow and Link's thrown bomb while each is in flight](../media/rc13-projectile-hitboxes.gif)

*Projectiles and thrown items: at 1/4 speed, a red disc rides on Ridley's fireball, Link's arrow and Link's thrown bomb while each is in flight. Eden, rc.13 (untested on hardware).*

### Frame data colours

On draws hitboxes in the colours of Ultimate Frame Data: a strong hitbox is red and a weak one is pink. Strong means
it launches hardest. A hitbox is strong when it would send the other fighter further than any other hitbox out on that
frame, such as a sword's tip or a knee's sweet spot. It is weak when another hitbox out on the same frame launches
harder, such as the gust of King K. Rool's Gut Check beside its hit. SmashWiki says the gust deals the same damage as
the hit; in the game's own data it deals 0.7 times the hit's damage, and the plugin follows the game. It is also weak
when it launches less than the strongest hitbox earlier in the same move, as the late frames of a knee do. This is SmashWiki's definition of
sweet and sour spots. The plugin works out the launch from each hitbox's own numbers in the game (damage, base knockback,
knockback growth, set knockback), using the game's knockback formula and the other fighter's percent and weight at that
moment, so no list of fighters is needed. It also counts the attacker's rage the way the game does: from 35% damage a
hitbox launches harder, up to 1.1 times at 150%, and no harder after that; a set knockback hitbox gets no rage. The game
never launches harder than its own limit, so two hitboxes that both reach it launch the same. Damage decides only between
hitboxes that launch the same. The launch angle
plays no part: which direction is best depends on the situation. With no other fighter to measure against, the colours
rank the hitboxes as they would launch a middleweight at 0%. A hitbox it cannot rank, such as a jab's single hitbox or
several identical hitboxes, is red. Grab boxes and hurtboxes use the colours in the table above. It recolours only what
is ticked. Off gives each hitbox its own colour, as before.

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
a menu or the clip editor is open, the waiting room's own menu included, and comes back when it closes. The combo or drill helper bar steps aside with it.

- **Values:** Off, Frame strip
- **Default:** Frame strip
- **How:** Left or Right switches it; A opens the list. The Input display chord (R + D-pad Down) switches it without
  opening the menu.

![The frame strip in a waiting room, hidden while the waiting room's menu is open](../media/rc11-strip-under-menu.gif)

More: [Frame strip](../guide/frame-strip.md)

### Spot labels

On writes Sweet, Sour or Late on each hit that connects in the frame strip. The label sits on the grey hitlag block of
the fighter that was hit: your hit on the CPU's lane, the CPU's hit on yours. Sweet is the hitbox out on that frame that
launches the other fighter hardest, Sour one that launches it less, and Late a hit that launches less than the strongest
hitbox earlier in the same move. The ranking is the one [Frame data colours](#frame-data-colours) uses; the launch angle
plays no part. The game itself records which hitbox connected, and the plugin reads that record. It checks it against
what the other fighter took: the damage the hit added and how hard the game launched it. So two hitboxes of the same
damage, such as the two pairs in Mario's jab, are told apart, and so is a stale hit. A hit has no label when it cannot
be checked, such as identical hitboxes, a hit on a shield or a projectile's hit. A charged smash attack is labelled like
an uncharged one: the plugin reads the charge from the game. Labels work at any damage on either side. Damage stops at
999%, so a hit there adds little or nothing, and the plugin then reads the hit's damage from the game's record. It
needs Input Display set to Frame strip.

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
