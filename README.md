# TrainingRooms for Ultimate

Practice tools for Training mode and the online waiting rooms of Super Smash Bros. Ultimate 13.0.5, as a Skyline plugin.

![The TrainingRooms menu: the five pages, then Hitbox Visualization turned On](docs/media/rc7-menu-tour.gif)

*Eden, 2026-11-11, rc.7 `722525a5` as shipped (diag NRO `cfec85c7`, log lines only; release NRO `b6e60bb9`): Training,
the menu on CPU, then Saves, Recording, Fighters & Stages and Display & Controls; Hitbox Visualization is turned On, and
back in Training yellow shapes cover both fighters. Cuts inside are listed in [docs/MEDIA.md](docs/MEDIA.md).*

## Install

Put the NRO at `atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libtraining_rooms.nro`, next to
`libnro_hook.nro` and `libparam_hook.nro`, which it needs. **Never install it together with `libtraining_modpack.nro`.**
Settings and saves live in `sd:/ultimate/TrainingRooms/`. On first boot, settings, save states and recordings from the
Training Modpack fork's `sd:/ultimate/TrainingModpack/` are imported once by copying.

```mermaid
flowchart LR
    pad["Controller chords"] --> plugin["libtraining_rooms.nro"]
    menu["Menu: five pages"] --> settings[("TrainingRooms/settings.json")]
    settings --> plugin
    fork[("TrainingModpack/ (the fork)")] -- "first boot: copied once" --> settings
    pilot["libssbu_cpu_pilot.nro (optional)"] -. "drives P1" .-> plugin
    plugin --> training["Training mode"]
    plugin --> waiting["Waiting-room practice"]
```

## Controls

| Action | Default chord |
| --- | --- |
| Open the menu | B + D-pad Up (always), or Minus (menu shortcut 1, editable) |
| Save state / load state | ZL + D-pad Down / ZL + D-pad Up |
| Record / Play on CPU | ZR + D-pad Down / ZR + D-pad Up |
| Play on Player 1 / recording pop-up | ZR + D-pad Left / ZR + D-pad Right |
| Cycle the CPU / Player 1 fighter | ZL + D-pad Left / ZL + D-pad Right |
| Input display on / off | R + D-pad Down |
| Game speed: hold, then D-pad Left / Right; release to close | R + D-pad Up |
| Kirby hat: Player 1 / CPU | Special + L3 / Special + R3 |
| Reset the training room | L + R + A |
| Input display on / off | R + D-pad Down |
| Game speed: hold, then D-pad Left / Right | R + D-pad Up |

R is Z on a GameCube controller.

Game speed shows a small pop-up while the chord is held. In Training it steps through the game's own speeds (1/4, 1/2,
2/3, x1, x1.5) and sets Training's Speed setting, so the Training menu shows the same value. In the waiting-room
practice it offers 1/4, 1/2 and x1, only for that practice: the speed goes back to x1 before the match starts.

The first time Training opens on a new install, TrainingRooms also shows six reminders: Welcome, Open Menu, Save State, Load State, Input Record and Input Playback. A later launch leaves those reminders off. The chords in the table stay available.

In the menu: L/R change page, ZL + Up/Down jumps to the previous or next section, A edits, Y resets a row, X saves the
current values as defaults, B goes back. Each page ends with *Reset page to defaults*, which asks first. The menu reopens
where you left it. The chords are edited on Display & Controls › Button chords.

The key hints at the foot of every page end with the build tag: the version, then `rc8` and the commit the NRO was
built from, for example `0.2.0-rc.6 · rc8 eca14e4b`. A build from uncommitted changes adds `-dirty`.

## Features

Clips are Eden footage of the 0.2.0-rc.1 release NRO `706c993f`, recorded 2026-11-05, except those marked RC10 or with
their own build; see [docs/MEDIA.md](docs/MEDIA.md).

### Hitboxes and grab boxes

![Hitbox Visualization turned On, then a forward smash](docs/media/rc7-hitbox.gif)

*Eden, 2026-11-11, rc.7 `722525a5` as shipped (diag NRO `cfec85c7`, log lines only): Training with nothing drawn;
Display & Controls › Hitbox Visualization from Off to On; back in Training yellow shapes cover both fighters, and
Player 1's forward smash sends the CPU from 6.3% to 24.8%.*

Hitbox Visualization (Display & Controls) draws the fighters' hurtboxes (yellow), hitboxes and grab boxes. Since rc.3
every fighter sphere is the plugin's own disc, so none is ever missing, even with two large fighters; projectiles keep
the game's effect spheres, and the discs trail the camera by one to two frames while it pans.

### Frame strip

The input display (Display & Controls › Input Display) is a strip at the top of the screen: the last second of
Player 1 and the CPU, a chip for each new button, and ADVANTAGE, LINK, and INPUT for the last exchange. ADVANTAGE is
the CPU's first actionable frame minus yours. Plus means you act first. [docs/FRAME-STRIP.md](docs/FRAME-STRIP.md)
is how to read the lanes, the stick row, and the readouts. The rc.5 clips are listed in
[docs/MEDIA.md](docs/MEDIA.md).

![R + D-pad Down hides the frame strip, then shows it again](docs/media/rc8-input-display-chord.gif)

*Eden, 2026-11-11, the rc.8 integration head `d3eeac81` (diag NRO `9f8afc66`, log lines only): a jab on the frame
strip; R + D-pad Down, "Input display · Off", and the next jab draws no strip; R + D-pad Down again, "Input display ·
On", and the jab draws on it. Cuts inside are listed in [docs/MEDIA.md](docs/MEDIA.md).*

R + D-pad Down turns the input display on and off without opening the menu, in Training and in the waiting-room
practice. A notice says "Input display" and On or Off. The menu's Input Display row shows the same value, and the
choice is saved.

### Game speed

![Hold R + D-pad Up: the Game speed pop-up steps to 1/2, then 1/4; the jabs slow down; the pause menu shows Speed 1/4](docs/media/rc8-game-speed-chord.gif)

*Eden, 2026-11-11, the rc.8 integration head `d3eeac81` (diag NRO `9f8afc66`, log lines only), in Training: a jab at
x1; R + D-pad Up held, the pop-up reads x1, then 2/3 and 1/2 with D-pad Left; a jab at 1/2; held again, 1/4; a jab at
1/4; the pause menu's Other Settings shows Speed 1/4, and the HUD's stopwatch reads 1/4. Cuts inside are listed in
[docs/MEDIA.md](docs/MEDIA.md).*

Hold R + D-pad Up and a small "Game speed" pop-up shows the current speed between two D-pad arrows. While you hold
it, D-pad Left and Right step one speed at a time; the steps stop at the slowest and fastest speeds instead of wrapping
around. Let go and the pop-up closes; the speed stays.

- While you hold the chord, no other chord fires, and the chord's buttons and D-pad Left / Right don't reach the game.
  The rest of Player 1's controller still does, so you can keep moving.
- **In Training** the speeds are Training's own: 1/4, 1/2, 2/3, x1 and x1.5. The chord sets the same Speed setting as
  the game's pause menu (Other Settings › Speed), so the pause menu and the HUD's stopwatch show the speed you chose, and a speed chosen in the pause menu is where the
  pop-up starts. 1 Frame stays in the pause menu: while Training is on 1 Frame, the pop-up shows "1 Frame", and the
  first step goes from x1.
  The stopwatch appears as soon as you step away from x1 and goes away when you step back to x1.

  ![Hold R + D-pad Up, Right: the stopwatch reads x1.5; Left back to x1: it goes away](docs/media/rc8-speed-hud-fixed.gif)

  *Eden, 2026-11-11, the rc.8 head `3f9d61dd` (diag NRO `61533917`, log lines only), in Training: x1 with no
  stopwatch; R + D-pad Up held, Right, and the stopwatch reads x1.5; held again, Left back to x1, and it goes away.*
- **In the waiting-room practice** the speed is local: 1/4, 1/2, 2/3, x1 and x1.5 change only your practice, while the
  room's timers and the other player stay at normal speed. Each practice session starts at x1.
- It never changes a Quickplay or online match: a match always starts at normal speed.
- **With the standalone CPU pilot** driving Player 1, the chord works the same way. In the waiting-room practice the
  pilot still gets one input per game update at every speed, x1.5 included, so its inputs are never doubled.
  <!-- R14 (worker B's r14bc-eca14e4b pair): keep this line only if R14 passes on the final head. -->

Both chords are rows of Display & Controls › Button chords ("Input display" and "Game speed"), editable and clearable
like the others. They don't fire while L is held, so Online Deluxe's L + R + D-pad overlay chords leave them alone.

With Online Deluxe 0.10.5 or later, no chord that uses the D-pad fires while Deluxe's window shows a page with rows
(Settings, Window Options, Plugins or a plugin's own page, whose rows use D-pad Up/Down and ZR + Left/Right), so
ZR + Right on its CPU Pilot page no longer also opens the Recording pop-up. Its Performance view doesn't stop them,
and the menu shortcuts still work. On a GameCube controller, Deluxe's L + R is both triggers: the ZL/ZR chords and the
two chords above don't fire while both triggers are held, and one trigger works as before.

### Combos

*+ Save last combo…* (Recording › Combos & Sequences) saves a combo you landed, and the combo helper grades your next
attempts against it, link by link. A sequence of combos is drilled the same way, one combo after another. Each saved
combo is a small JSON file in `sd:/ultimate/TrainingRooms/combos/`. You can edit it, copy it to another card or share
it, and any fighter can practise it; [docs/COMBOS.md](docs/COMBOS.md) describes the format. The clip is under Combos
below.

### CPU reactions

![No DI, then DI Up from the menu: the same forward smash launches the CPU higher](docs/media/rc7-di-up.gif)

*Eden, 2026-11-11, rc.7 `722525a5` as shipped (diag NRO `cfec85c7`, log lines only): the CPU loads at 80% (CPU damage
on load, Random Value 80–80) and takes Player 1's forward smash with no DI; CPU › Defence › DI Direction gets Up; the
same load and forward smash launch the CPU on a steeper line, higher at the same moment after the hit. Cuts inside
are listed in [docs/MEDIA.md](docs/MEDIA.md).*

DI, SDI, air dodge, shield tilt, mash, ledge, tech and throw options on the CPU page. Above: the same hit with no DI
and with DI Up.

![The DI and SDI direction grids](docs/media/di-sdi-chips.gif)

The direction chips show the game's own DI line and SDI streak; each A adds weight to a chip. *In* and *Out* are
relative to you, on either side of the CPU.

*CPU Level* (CPU › Level: Off, or Lv 1–9) replaces Training's own CPU level. When the two differ as Training starts, a
notice says so once.

![Training's CPU card at Lv. 3; in Training, the notice names both levels](docs/media/rc7-cpu-level-notice.gif)

*Eden, 2026-11-11, rc.7 `b3678dfe` (diag NRO `9dfc784c`, log lines only): CPU Level is Lv 7. The character select's CPU
card reads CPU Lv. 3; in Training the notice reads "Lv 7 from TrainingRooms (Training menu: Lv 3)". Cuts inside are
listed in [docs/MEDIA.md](docs/MEDIA.md).*

### Save states and respawn

![Respawn on platform turned On, then a fall off the stage: back on the revival platform](docs/media/rc7-respawn-on.gif)

*Eden, 2026-11-11, rc.7 `722525a5` as shipped (diag NRO `cfec85c7`, log lines only): Fighters & Stages › Stages ›
Respawn on platform from Off to On; Player 1 runs off the left side and is KO'd; Player 1 comes back on the revival
platform at the top. Cuts inside are listed in [docs/MEDIA.md](docs/MEDIA.md).*

Named save states (one bound to Save and Load at a time), mirroring and damage on load (Saves page). *Respawn on platform* (Fighters & Stages ›
Stages) chooses between the game's respawn platform (above) and the instant respawn.

### Recording and playback

A save state only warps, and Load starts nothing. Each role has one active macro, a recording or a chain, and
that role's Play chord plays it without warping back on later loops. On the Recording page, Y does nothing and
X cycles the loop only while the cursor is in Macros.
[docs/RECORDING.md](docs/RECORDING.md) is how to use it. The pop-up and the Recording page are in that doc.
What rc.6 changes is [docs/RELEASE-NOTES-rc6.md](docs/RELEASE-NOTES-rc6.md).

![Record on Player 1: an empty library, two jabs, then Macro 1 and its start save](docs/media/rc7-record.gif)

*Eden, 2026-11-11, rc.7 `722525a5` as shipped (diag NRO `cfec85c7`, log lines only): the pop-up lists no saves and no macros; Record
on Player 1 and two jabs; the pop-up then lists "Macro 1" (Recorded on Mario) and its start save "Macro 1 · start".
Playback is shown in [docs/RECORDING.md](docs/RECORDING.md). Cuts inside are listed in [docs/MEDIA.md](docs/MEDIA.md).*

While an AI drives Player 1, for example the standalone CPU pilot, Record is refused, from the chord and from the
pop-up: the notice reads "P1 is controlled by an AI: recording is off", and nothing is recorded. A recording that is
already running can always be stopped.

### Combos

![The combo helper grading a drilled combo: Combo 1, jab → jab, TRUE −5, on time](docs/media/rc7-combo-helper.gif)

*Eden, 2026-11-11, rc.7 `722525a5` as shipped (diag NRO `cfec85c7`, log lines only): the pop-up's COMBOS with Combo 1
(jab → jab) drilled; the jabs; the bar under the frame strip grades the attempt: TRUE −5, on time. Cuts inside are
listed in [docs/MEDIA.md](docs/MEDIA.md).*

*+ Save last combo…*, the first row of COMBOS on the Recording page and in the recording pop-up, keeps your last combo
as a file and makes it the drill. With *Combo helper* On, a bar under the frame strip grades each attempt link by link:
whether the recorded inputs came, in order and on time, and whether the same move came out. A true combo is shown,
not required. Drilling a sequence adds a header and a row of its combos to that bar.

![Drill a sequence: combo 1, then combo 2, then a drop back at combo 1](docs/media/rc7-sequence-drill.gif)

*Eden, 2026-11-11, rc.7 `722525a5` as shipped (diag NRO `cfec85c7`, log lines only): combo 1 lands, combo 2
starts, then a wrong move (up tilt) drops the sequence back to combo 1.* See [docs/MEDIA.md](docs/MEDIA.md).

### Fighters, cycles and Kirby hats

![Character Cycles: the order badges, then each press of the cycle chord switches Player 1 in that order](docs/media/rc6-cycle-switch.gif)

*Eden, 2026-11-10, rc.6 `e6501ac6` (diag NRO `c1b9f694`, log lines only): the grid's red badges number Player 1's
order (Mario 0, Mii Brawler 1, Mii Swordfighter 2, Mii Gunner 3, Link 4). In Training, each press of the cycle chord
switches Player 1 from Mii Gunner to Link, back to Mario, then to Mii Brawler. Cuts inside are listed in
[docs/MEDIA.md](docs/MEDIA.md).*

Cycle chords step through your own order of fighters per role (red for Player 1, blue for the CPU), drawn with the
game's stock icons.

The fighters of your cycles load hot. Each role's first window of fighters is ready when Training starts, and the
other groups are built in the background while you play; once the notice reads Ready or All loaded, a switch into a
group took about 50–70 ms in Eden. Changing a cycle in Training builds the new fighters the same way, at once. The
fighters that left it are freed when you next open the pause menu, or right away if the new ones need their room (a
brief pause). Leaving a Mii for the fighters it replaced is a warm switch, not a reload. The background build
waits only while Kirby is on screen, and for a second Mii group while a Mii is on screen
(`background_build_eligible`, `src/model/swap/body_bank.rs:852`). Raising No. of CPUs brings in the CPU cycle's next
fighters, a saved Mii included.

![Player 1's Kirby: the hat chord twice, two hats in turn](docs/media/rc7-kirby-hats.gif)

*Eden, 2026-11-11, rc.7 `722525a5` as shipped (diag NRO `cfec85c7`, log lines only): Player 1 is a plain Kirby; the hat
chord gives Falco's hat, and again Sheik's.*

Kirby hats change with a chord, for Player 1 or the CPU.

### Mii fighters

Each Mii class (Fighters & Stages › Mii Brawler, Swordfighter, Gunner) lists your saved Miis per role (ZR switches
Player and CPU). *Add New…* opens Smash's own Mii Fighter editor from Training. Saving creates a new Mii and leaves the
others untouched, and leaving without saving creates nothing. Saving a Mii you already have saves nothing new and says
"Same as ‹saved name›, nothing new saved". See [docs/MII.md](docs/MII.md).

![Add New, then Save with the copy unchanged: nothing new is saved](docs/media/rc6-mii-same.gif)

*Eden, 2026-11-10, rc.6 `e6501ac6` (diag NRO `c1b9f694`, log lines only): the Mii Brawler list (three Miis), Add
New…, Smash's Mii editor on a copy of the class's Mii, Save at once. Back in Training, the notice reads "Same as 格斗,
nothing new saved". The row checked that every Mii file stayed byte-identical. Cuts inside are listed in
[docs/MEDIA.md](docs/MEDIA.md).*

![Add New, two new specials, Save: the new Mii is listed, picked, and played](docs/media/rc7-mii-editor.gif)

*Eden, 2026-11-10, rc.7 `7d720c68` (diag NRO `a45ba951`, log lines only): Add New… opens Smash's Mii editor. Neutral
Special 2 and Side Special 2, then Save. The Mii Brawler list gains "4. 格斗 [2211]"; A picks it and the "> " moves to
it. In Training, Player 1 is that Mii Brawler, and B is its new neutral special. The row checked that exactly one new
Mii file was written. Cuts inside are listed in [docs/MEDIA.md](docs/MEDIA.md).*

### Stages

![Change Stage: Battlefield, Y for its Ω form, and Training reloads there](docs/media/rc7-change-stage.gif)

*Eden, 2026-11-11, rc.7 `722525a5` as shipped (diag NRO `cfec85c7`, log lines only): Training; Fighters & Stages ›
Change Stage, the cursor from Waiting Room to Battlefield [N], Y gives [Ω], A; "Practice Stage · Stage requested"; and
Training reloads on Battlefield Ω. Cuts inside are listed in [docs/MEDIA.md](docs/MEDIA.md).*

*Change Stage* (Fighters & Stages › Stages) goes to the stage you pick when the menu closes, in Training and in
waiting-room practice; Y picks its form first.

![Stage grid](docs/media/menu-stages.gif)

Pick the stages and their order for Training rotation and waiting-room practice. *RC10, 2026-11-04, NRO
`50d95130`: the grid only.*

![Lobby Stages ranks two stages; with Training Stage Rotation On, the next Training entry goes to Big Battlefield](docs/media/rc7-lobby-stages-rotation.gif)

*Eden, 2026-11-10, rc.7 `7d720c68` (diag NRO `a45ba951`, log lines only): Lobby Stages ranks 1. Waiting Room and
2. Big Battlefield, and Training Stage Rotation is On. From Training on the training stage, back through the character
select, Training opens on Big Battlefield. Cuts inside are listed in [docs/MEDIA.md](docs/MEDIA.md).*

### Waiting rooms

![Training Partners 1 to 2 in a Quickplay waiting room: a second partner drops in](docs/media/rc6-waiting-partners.gif)

*Eden, 2026-11-10, rc.6 `e6501ac6` (diag NRO `c1b9f694`, log lines only), a Quickplay room hosted in Eden: one partner
standing; the game's own waiting-room menu, Training Partners 1, Right to 2; a second partner drops in, and both stand
(Training Partner Behavior: Stop).*

![The next room starts with the two partners and Stop](docs/media/rc6-waiting-reentry.gif)

*The same run, the next room: it fades in with the two remembered partners, standing still.*

The same menu works while you wait in an online or local room. The Training partners and CPU action you set in the
game's own waiting-room menu are remembered for the next room, and CPU › Level › CPU Level sets the partners' level.

With the CPU pilot plugin 0.2.3 (`libssbu_cpu_pilot.nro`), the pilot can also drive Player 1 in Quickplay's
waiting-room practice. Arm it from its CPU Pilot page as in Training. The practice shows no `CPU Lv` tag, and when a
match is found the pilot keeps Player 1 in the match. The lobby tools above work beside it, Record, Play on Player 1
and Quick Reset are refused while it drives, and the Game speed chord works at every speed.

To leave a Quickplay room before the match starts (the game's Pause › Leave Battle, which the QuickPlay cancel fix
plugin keeps open for a few seconds after you join), close the TrainingRooms menu first: while it is open, Plus
doesn't reach the game.

![Save last combo in a Local Wireless waiting room, then the helper bar drills it there](docs/media/rc6-lobby-combos.gif)

*Eden, 2026-11-10, rc.6 `e6501ac6`, the guest of a Local Wireless room hosted in Eden: jab into jab in the waiting
room; Recording › Combos & Sequences, *+ Save last combo…*; COMBOS lists "Lobby jabs, recorded on Sheik"; back in the
room, the helper bar drills it. Cuts inside are listed in [docs/MEDIA.md](docs/MEDIA.md).*

![The menu in a Local Wireless waiting room](docs/media/waiting-menu.gif)

*Eden, 2026-11-05, 0.2.0-rc.1 (release NRO `706c993f…`), a Local Wireless room hosted in Eden: the menu opens on its
last page and R steps to CPU and Saves. The menu only.*

### Menu navigation

![ZL + Down jumps through the CPU page's sections](docs/media/menu-section-jump.gif)

ZL + Up/Down jumps between a page's sections (in popup lists, a screenful). The menu reopens on the page and row you
left. ZR no longer resets a page: each page ends with *Reset page to defaults*, which asks first. *Eden, 2026-11-05,
`p23/menu-nav` diag NRO `1273c29f`, the navigation code rc.3 merged.*

### Menu shortcuts and replay capture

![Menu shortcut 1 changed from Minus to ZL + D-pad Up, which then opens the menu](docs/media/rc7-menu-shortcut.gif)

*Eden, 2026-11-11, rc.7 `722525a5` as shipped (diag NRO `cfec85c7`, log lines only): Display & Controls › Menu
shortcut 1 reads Minus; A opens the chord editor, X starts the capture, ZL held with D-pad Up gives "ZL + D-pad Up", A
confirms; back in Training, ZL + D-pad Up opens the menu. Cuts inside are listed in [docs/MEDIA.md](docs/MEDIA.md).*

Menu shortcuts are changed in the menu and work at once.

![A capture opened in the Replay window: its Mii goes to the CPU](docs/media/rc6-replay-window.gif)

*Eden, 2026-11-10, rc.6 `e6501ac6` (diag NRO `c1b9f694`, log lines only): Recording › Replay Captures lists a capture
(Mario vs Mii Gunner, 180 frames), A opens it in the Replay window, Fighter Mismatch offers the recorded fighter, and
Switch reloads the CPU as the capture's Mii Gunner ("Saved Mii applied"). The capture is a test fixture, made before
the clip. Cuts inside are listed in [docs/MEDIA.md](docs/MEDIA.md).*

![Replay capture turned On in the menu, then the same setting On in the Online Deluxe overlay](docs/media/rc7-replay-capture.gif)

*Eden, 2026-11-11, rc.7 `722525a5` as shipped (diag NRO `cfec85c7`, log lines only), with Online Deluxe 0.10.1: Recording
› Replays › Replay capture from [OFF] to [ON]; back in Training, L + R + Down opens the Online Deluxe overlay at the
top right, and its plugins page reads "Training Rooms › Replay capture On". Cuts inside are listed in
[docs/MEDIA.md](docs/MEDIA.md).*

The *Replay capture* row and the Online Deluxe overlay show the same setting.

## Status

**[0.2.0-rc.6](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.6)** is the
newest build (the release link resolves once it is published: TODO-FINAL-PASS). It follows 0.2.0-rc.3.1: 0.2.0-rc.5
was withdrawn before players ran it, and there is no separate 0.2.0-rc.4. What it changes is
[docs/RELEASE-NOTES-rc6.md](docs/RELEASE-NOTES-rc6.md), on top of [docs/RELEASE-NOTES-rc5.md](docs/RELEASE-NOTES-rc5.md).

**0.2.0-rc.3** (2026-11-05) is the previous console test build. It adds to rc.2:
- the **frame strip** input display, replacing the 15-row input log;
- **Change Stage**: a row that switches the stage now, in Training and in waiting rooms;
- **menu navigation**: ZL + Up/Down jumps sections, the menu reopens where you left it, ZR no longer resets a page by
  accident, and *Reset page to defaults* asks first;
- **own hitbox discs** for every fighter sphere, never refused (projectiles keep the effect spheres; the discs trail
  the camera by one to two frames, a known limit).

rc.2 added the pause-helper fix, the Solo Popo mark placement and the Kirby hat-request endings. Limitations kept from
the fork are listed in [Differences from the fork](docs/DIFFERENCES-FROM-THE-FORK.md); planned work is in the
[ROADMAP](docs/ROADMAP.md). 0.1.0 crashes at console boot; use the newest build, [0.2.0-rc.6](https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/releases/tag/v0.2.0-rc.6).

## Build

```powershell
cargo test --all-targets
cargo skyline build --release
```

The NRO is `target/aarch64-skyline-switch/release/libtraining_rooms.nro`. Toolchains, the diagnostic build and the
layout recipe: [docs/BUILD.md](docs/BUILD.md). Current acceptance state: [HANDOFF.md](HANDOFF.md).

## License and provenance

MIT ([LICENSE](LICENSE)). This is a clean-room implementation inspired by the Training Modpack
([jugeeya/UltimateTrainingModpack](https://github.com/jugeeya/UltimateTrainingModpack)), which publishes no license:
its features were re-specified from observable behaviour and none of its code, layouts or artwork is used. Code
carried over from this project's own earlier fork and all dependencies are recorded in [PROVENANCE.md](PROVENANCE.md)
and [CARRY-OVER.md](CARRY-OVER.md).

This project was developed with the assistance of AI. It is an unofficial fan project, not affiliated with or
endorsed by Nintendo.
