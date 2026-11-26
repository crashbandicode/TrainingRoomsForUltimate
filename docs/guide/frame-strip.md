# Frame strip

The input display's frame strip shows about one second of game frames for Player 1 and the CPU: your inputs, their
action states and the timing of your follow-up. It works in Training and waiting-room practice.

<!-- CLIP: tut-input-display.mp4 | Input Display on: two jabs fill the lanes, the ADVANTAGE and LINK readouts update -->
*Clip coming.*

## Turning it on and off

Set Display & Controls › [Input Display](../menu/display-and-controls.md#input-display) to **Frame strip** or **Off**.
Frame strip is the default. The default chord **R + D-pad Down** switches it without opening the menu; on a GameCube
controller R is Z. The choice is saved, and you can change the chord in [Controls](controls.md#changing-a-chord).

While Spacing grader is On the frame strip stays on screen, even with Input Display Off. Parry practice keeps it on
screen the same way while it runs.

The strip hides while a menu or the clip editor is open and comes back when it closes. It clears on a scene change,
fighter replacement, Training reset or save-state load. While the strip is visible, the separate Frame Advantage
notice stays down.

## Lanes and colours

The newest frame is on the right. Each column is a game frame; adjacent frames of the same state form a block. A run
of at least seven frames of recovery, hitstun or shieldstun shows its length.

| Colour | What the lane shows |
| --- | --- |
| Light grey | Hitlag: that fighter is stopped. |
| Purple | Hitstun. |
| Teal | Shieldstun. |
| Dark blue-grey | Actionable, including a cancel window. |
| Red | Player 1's attack has an active hitbox. |
| Gold | Player 1's attack is in startup, before an active frame. |
| Blue | Recovery or another busy state that did not match the rows above. |

The first matching state in the table wins. The CPU lane has no startup or active colour.

## Chips

A chip above Player 1's lane marks a button going down. If several go down on the same frame, one is shown, in this
order: Attack, Special, Grab, Jump, Shield.

| Chip | Colour |
| --- | --- |
| Attack (A) | Green |
| Special (B) | Red |
| Grab (Z) | Purple |
| Jump (X or Y) | Blue |
| Shield (L or R) | Slate |

An arrow on a chip shows the left stick's direction on that frame; neutral has no arrow. A C-stick attack uses the
C-stick's direction when it is outside the dead zone. The stick row below Player 1's lane shows the held left-stick
direction each frame, or a blank for neutral.

The strip can also mark a special's recognised directed travel with an arrow on Player 1's lane. It does not put a
travel arrow on every special: the mark depends on what the movement detector observes.

<!-- CLIP: tut-input-special-direction.mp4 | A neutral special, then a side and an up special: the strip's special carries its direction mark -->
*Clip coming.*

### Spot labels

Turn Display & Controls › [Spot labels](../menu/display-and-controls.md#spot-labels) On to label a checked hit Sweet,
Sour or Late on the hit fighter's hitlag block. A hit whose spot cannot be told has no label.

## Readouts

Before an exchange finishes ADVANTAGE is a dash; before a follow-up LINK and INPUT are dashes.

### ADVANTAGE

| Readout | Meaning |
| --- | --- |
| Plus / minus / zero | You can act first / the CPU can act first / both can act on the same frame. |
| On hit / on shield | How the last hit connected. A shield hit sets advantage but does not make a link. |

Actionable includes cancel windows, so the number can differ from frame data that waits for a move and landing lag to
finish.

### LINK

| Readout | Meaning |
| --- | --- |
| TRUE, with zero or a negative number | The CPU could not act before the follow-up hit; the magnitude is the spare frames. TRUE may have no number. |
| DROP, with a positive number | The follow-up hit that many frames after the CPU could act. |
| Green `window Nf` bar | The width of the press window that would still make a true combo at the same startup. |

A follow-up is a hit from a different action within 20 frames of the CPU becoming free after a hit. Hits inside one
multi-hit action and hits on shield are not follow-ups. A later hit begins a new exchange. The window bar is absent
when a frame-perfect press would drop or the true-combo number could not be read.

### INPUT

| Readout | Meaning |
| --- | --- |
| `0`, on time | The press was on your first actionable frame. |
| `+N`, Nf late | The press was N frames after it. |
| `-N`, buffered Nf | The press was before it; the move started on the first actionable frame. |

The readout may also show spare frames for a true combo or frames past the window for a drop.

### LAST

| LAST | Meaning |
| --- | --- |
| Green TRUE / red DROP pill and two moves | A follow-up and its verdict; the last four are shown, newest first. |
| Link history Off | Hides LAST; LINK still shows the newest pair. |

## Scroll back

Hold **R3** (right stick click) and press **D-pad Left or Right** to look back through the last 600 columns. Hold a
direction to repeat. That's about ten seconds of continuous action. When the strip freezes during an idle stretch,
no new columns are added, so the time on the marker can be much longer. Release R3 to return to live.

<video src="https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate/media/blob/16db06e0ff5b07c584ded3173e52cfade51d4dfa" autoplay muted loop playsinline controls></video>

*Close-up of the input display. With R3 held, D-pad Left scrolls back (the label reads -0.2 s, then -5.0 s), and the display then returns to now. Eden, 2026-11-25, build `a945b0c1`.* [Open the clip](../media/i4-scrollback-zoom.mp4)

## Freeze

After 30 frames with both fighters actionable, no new Player 1 button press, no left-stick direction change and no
hitlag, the strip freezes to keep the exchange visible. It scrolls again when any of those conditions changes.

![The strip holds still after a hit, then scrolls again when Player 1 jabs](../media/rc7-frame-strip-freeze.gif)
*The strip holds the exchange, then moves again on a jab.*

More: [Display & Controls](../menu/display-and-controls.md).
