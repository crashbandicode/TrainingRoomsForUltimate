# TrainingRooms for Ultimate

TrainingRooms adds practice tools to Super Smash Bros. Ultimate's Training mode and to the waiting rooms of online and
local play: hitboxes, save states, recordings, combo drills, game speed and more, all from one menu.

![The menu's five pages, then hitboxes turned on](docs/media/rc7-menu-tour.gif)<br>*The menu, then hitboxes on*

## Install

1. Set up Skyline for Smash Ultimate 13.0.5 on your Switch.
2. Copy `libtraining_rooms.nro`, `libnro_hook.nro` and `libparam_hook.nro` into
   `atmosphere/contents/01006A800016E000/romfs/skyline/plugins/` on the SD card.
3. If `libtraining_modpack.nro` is there, remove it: the two can't run together.
4. Start the game and open Training. Settings from the Training Modpack are copied over the first time.
5. Press B + D-pad Up (or Minus) to open the menu.

## Controls

| Button | What it does |
| --- | --- |
| B + D-pad Up, or Minus | Open the menu |
| ZL + D-pad Down / Up | Save / load a state |
| ZR + D-pad Down | Record |
| ZR + D-pad Up / Left | Play the recording on the CPU / on yourself |
| ZR + D-pad Right | Recording pop-up |
| ZL + D-pad Left / Right | Switch the CPU's / your fighter |
| R + D-pad Down | Input display on / off |
| Hold R + D-pad Up, then Left / Right | Game speed |
| B + click the left / right stick | Kirby hat: yours / the CPU's |
| L + R + A | Reset positions |

You can change every button in the menu. On a GameCube controller, R is Z. With Online Deluxe, the D-pad buttons wait
while its settings window is open. While the CPU pilot plays for you, Record, Play on yourself and the waiting-room
reset are off.

## Features

### Hitboxes
See every hitbox, hurtbox and grab box. Display & Controls › Hitbox Visualization.

![Hitbox Visualization turned on, then a forward smash](docs/media/rc7-hitbox.gif)<br>*Hitboxes on, then a forward smash*

### Input display
A strip at the top shows your inputs and the CPU's, and who can act first. R + D-pad Down turns it on and off.

![R + D-pad Down hides the strip, then shows it again](docs/media/rc8-input-display-chord.gif)<br>*Turning the input display off and on*

### Game speed
Slow the game down or speed it up. Hold R + D-pad Up and press Left or Right.

![The speed steps to 1/2, then 1/4, and the jabs slow down](docs/media/rc8-game-speed-chord.gif)<br>*Stepping down to 1/4*<br>
![The stopwatch shows x1.5, then goes away at x1](docs/media/rc8-speed-hud-fixed.gif)<br>*The stopwatch follows the speed*

### CPU reactions
Teach the CPU to DI, tech, shield, mash and more, on the CPU page.

![No DI, then DI Up: the same smash sends the CPU higher](docs/media/rc7-di-up.gif)<br>*No DI, then DI Up*<br>
![The DI and SDI direction grids](docs/media/di-sdi-chips.gif)<br>*Picking DI directions*<br>
![The CPU level notice](docs/media/rc7-cpu-level-notice.gif)<br>*Setting the CPU's level*

### Save states and respawn
Save where you stand with ZL + D-pad Down, and go back with ZL + D-pad Up.

![Respawn on platform turned on, then a fall](docs/media/rc7-respawn-on.gif)<br>*Respawning on the platform*

### Recording
Record yourself, then let the CPU play it back. ZR + D-pad Down records, ZR + D-pad Up plays.

![Recording two jabs](docs/media/rc7-record.gif)<br>*Recording two jabs*

### Combos
Save a combo you landed and practise it: a bar grades each try. Recording › Combos & Sequences.

![The combo helper grading a combo](docs/media/rc7-combo-helper.gif)<br>*Grading a combo*<br>
![Drilling two combos in a row](docs/media/rc7-sequence-drill.gif)<br>*Drilling a sequence*

### Fighter cycles
Switch fighters without leaving Training. ZL + D-pad Right for yours, ZL + D-pad Left for the CPU's.

![Each press switches to the next fighter](docs/media/rc6-cycle-switch.gif)<br>*Cycling through your fighters*

### Kirby hats
Change Kirby's copy ability. B + click a stick.

![Kirby gets Falco's hat, then Sheik's](docs/media/rc7-kirby-hats.gif)<br>*Changing Kirby's hat*

### Mii fighters
Make and pick Mii fighters from Training. Fighters & Stages › Mii Brawler, Swordfighter or Gunner.

![Saving an unchanged Mii saves nothing new](docs/media/rc6-mii-same.gif)<br>*No duplicate Miis*<br>
![A new Mii with new specials, then played](docs/media/rc7-mii-editor.gif)<br>*Making a new Mii*

### Stages
Go to any stage from the menu, or rotate through your favourites. Fighters & Stages › Stages.

![Change Stage to Battlefield Ω](docs/media/rc7-change-stage.gif)<br>*Changing the stage*<br>
![The stage grid](docs/media/menu-stages.gif)<br>*Picking stages*<br>
![The next Training opens on Big Battlefield](docs/media/rc7-lobby-stages-rotation.gif)<br>*Stage rotation*

### Waiting rooms
Practise while you wait for an online or local match, with the same menu.

![A second training partner drops in](docs/media/rc6-waiting-partners.gif)<br>*Adding a training partner*<br>
![The next room remembers the partners](docs/media/rc6-waiting-reentry.gif)<br>*Partners remembered*<br>
![Saving a combo in a waiting room, then drilling it](docs/media/rc6-lobby-combos.gif)<br>*Combos in a waiting room*<br>
![The menu in a waiting room](docs/media/waiting-menu.gif)<br>*The menu in a waiting room*

### Menu
Jump between sections with ZL + Up/Down, and set your own menu button.

![ZL + Down jumps through the sections](docs/media/menu-section-jump.gif)<br>*Jumping between sections*<br>
![Changing the menu button to ZL + D-pad Up](docs/media/rc7-menu-shortcut.gif)<br>*Setting your own menu button*

### Replays
Capture moments from your replays and open them in Training.

![A replay moment opened against the CPU](docs/media/rc6-replay-window.gif)<br>*Opening a replay moment*<br>
![Replay capture on, in the menu and in Online Deluxe](docs/media/rc7-replay-capture.gif)<br>*Turning replay capture on*

## Build

```powershell
cargo test --all-targets
cargo skyline build --release
```

## License

MIT License: see [LICENSE](LICENSE).

This project was developed with the assistance of AI. It is an unofficial fan project, not affiliated with or
endorsed by Nintendo.

## More

What's new: [rc.9](docs/RELEASE-NOTES-rc9.md). [Technical notes](docs/TECHNICAL.md).
