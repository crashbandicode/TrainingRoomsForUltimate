# Guard the Edge and Juggler: the return rule, the crosshair, Player 1's start (answered)

**Answered 2026-11-22.** GJ1: A agreed. GJ2: approved. GJ3: approved, Percent page kept. GJ4: as recommended (measure first, then A or B; the next attempt waits for a KO'd Player 1; Player 1's damage between attempts stays as today).

Mockups, not Eden stills (2026-11-22, edge-juggle-sol, source `7d962416`; branch `design/gte-juggler-crosshair`, commits `52997fd2` and `9be68ecd`).
Four decisions, one picture each; the chosen option has a green outline. Nothing here is implemented yet, and no Eden measurement was made for this page.
This version carries the reviewer's wording corrections; what changed is listed at the end.

Kept in all four: every fighter's measured start, the remembered start, the larger mark and the pointer, Player 1's start page, the Preview page.
Line counts are estimates. The full entry, with the source lines, is the "Guard the Edge and Juggler (2026-11-22)" section of `docs/DECISIONS.md` on that branch.

## GJ1: a KO counts only once you are back on the stage (agreed: A)

Your words: "I want to add the requirement that the p1 returns to the stage successfully after a successful edgeguard for it to count."

- **Back on the stage** means standing on a floor or a platform, or holding a ledge (no getup needed). It is a new test for Player 1, modelled on the drill's test for the CPU's own recovery but not the same read.
- If you stand on the stage or hold a ledge when the KO happens, it counts at once, as today. If you are in the air at that moment, also above the stage, it counts when you land or grab a ledge.
- **Limit:** 10 seconds after the KO.
- **You are KO'd, or the limit runs out:** the attempt is a miss. It is added to the total, the streak goes to 0, and no time is recorded. The card says `Not counted · P1 KO` or `Not counted · no return`.
- **Fastest** stays the time to the KO.
- **Read from the game:** your situation (ground, ledge) and your KO statuses. No new hook.
- The other options were: **B** a floor only, so a ledge grab needs a getup; **C** as A, but an uncounted KO leaves the total and the streak alone.

About +60 to +80 lines. Unproven on a running game: which statuses Player 1 goes through at a KO in Training, and what the respawned CPU does while you are still returning (its pad is held neutral for that time).

![GJ1: the return rule](gte-juggler-crosshair/gte-next-GJ1-return.png)

## GJ2: Guard the Edge uses Break the Targets' crosshair (approved)

One crosshair page replaces the quadrant page and the CPU page, so the wizard has 3 pages in place of 4.

- **Shared code:** Break the Targets' controls (stick 2 units a frame, D-pad 1 unit a press), its crosshair picture and the Fixed camera. Tech practice already uses the same controls.
- **Read from the point:** the side and its ledge, and upper or lower (only the best times and the best streak use that). The recovery routes already read the point itself.
- **Where a start is allowed:** anywhere off the stage, 1 unit outside the ledge and inside the blast line. Over or under the stage, A is refused.
- **Where the crosshair opens:** on the fighter's left-side default. That is its measured start when it has one, else the middle of the lower-left area, which is where the wizard opens today.
  - Changed from today: a fighter whose measured start is in the upper area (Link, Lucas, Sonic and others) opens on it directly. Today it opens on the lower-left middle and needs a step to Upper left.
- **X and Y on the crosshair page:** X returns to the default of the side the crosshair is on; Y jumps to the other side's default. On the other pages X is as today (everything back to the opening values).
- **Larger mark, pointer, remembered start:** all kept. The pointer follows the crosshair, which must be able to leave the picture because some default starts are outside it. A start you never moved follows the fighter's default for its side.
- **Player 1's point:** the same crosshair, on the floor.

About −110 and +65 lines, with about 230 lines of tests rewritten. It is simpler to use; the code saving is small, because the quadrant page is only about 55 lines.

Risks:
- Every Eden route and reader of the wizard's pages changes.
- The one-press quadrant middles are gone, except the lower one for a fighter with no measured start.
- A D-pad press moves the point 1 unit from where it is. It does not snap to whole coordinates, and the default starts are fractions measured from the ledge.
- A start at exactly the ledge's height is always in the upper bucket for the bests. No measured start is at that height.

Unproven: the crosshair outside the picture.

![GJ2: the crosshair in Guard the Edge](gte-juggler-crosshair/gte-next-GJ2-edge-crosshair.png)

## GJ3: Juggler uses the same crosshair for the CPU spot (approved; Percent page kept)

Juggler has no quadrant page: its CPU page is already a free point. It gets the crosshair picture and the D-pad's 1-unit step; the height above the floor is still read from the point. About −5 and +10 lines.

Percent page: **A** keep it as it is (chosen); **B** make it a menu row like Guard the Edge's percent, which gives 3 pages for about +40 lines and one new saved setting.

![GJ3: the crosshair in Juggler](gte-juggler-crosshair/gte-next-GJ3-juggler-crosshair.png)

## GJ4: Player 1's start, two measured defects (as recommended: measure first)

The crosshair removes neither: it changes how the point is picked, not what the game does with a fighter written onto it.

- **A start on the floor's last point** (Sonic, 26 releases): the wizard shows P1 (−80, 0), but 24 releases begin in the edge-teeter wait at −79.03 and 2 (releases 1 and 4) begin hanging on the ledge. What decides between the two is not known. After a measurement of the start write on the bridge lineup: **A** keep that start and make it repeat (recommended), or **B** stop the crosshair 1 unit inside the end.
- **Player 1 above the stage centre at attempt 18** (Lucas, both boots): the traces fit a KO of Player 1 after attempt 17 and a respawn falling from (0, 150) as the next attempt started. That is an inference, not an observed KO:
  - The first boot has Player 1's request for Standby high above the stage, then its request for Fall at exactly (0, 150).
  - The second boot has only the Fall request and the same release above the centre.
  - The traces record status requests, not completed transitions or a lost stock.
  - Decided: the next attempt waits until Player 1 is back in play, then writes its start. GJ1 uses the same KO read.

Not in the traces: the start write itself, and which hit KO'd Player 1.

![GJ4: Player 1's start](gte-juggler-crosshair/gte-next-GJ4-player-start.png)

## What changed in this version

- Your four answers are recorded.
- GJ1 no longer says the return test is the one the drill uses for the CPU, or that an edge-guard from the stage is always scored as today: a Player 1 in the air at the KO counts on landing.
- GJ2 names the point the crosshair opens on, what X and Y do, and what happens to the quadrant middles and to the bests' bucket.
- GJ2 no longer says the D-pad gives exact whole-unit points.
- GJ4 calls the Lucas KO an inference and no longer says how long after the start write Player 1 fell.
