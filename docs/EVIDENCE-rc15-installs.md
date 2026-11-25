# rc.15 installs 1 to 4: what was installed and what was tested

2026-11-25. The receipts behind [the rc.15 release notes](RELEASE-NOTES-rc15.md) and the status on
[Current release](CURRENT-RELEASE.md). Three claims are kept apart:

- **Installed:** the plugin file on the card was read back and matched the build. This is a deployment check.
- **Tested in Eden:** a named run gave a verdict for a row, on the build named with it.
- **The operator reported:** the operator's own report from play on the console is on record. A report is what they
  saw at that time; it is not a pass or a fail of the build.

Paths starting with `artifacts/` are in the shared workspace (`/home/intpa/smash-training-lobby/`), not in this
repository. "Inbox" is `artifacts/phase23/INBOX-opus-rc6-integration.md`; an entry is named by its time (UTC) and
author. Times are UTC.

## Summary

| | Install 1 | Install 2 | Install 3 | Install 4 |
| --- | --- | --- | --- | --- |
| Commit | `152629cd` | `71158421` | `e7667f26` | `a945b0c1` |
| Release NRO (sha256, first 8) | `3d6836da` | `1b5608ed` | `eff66d92` | `99e00e92` |
| Size | 5,226,496 B | 5,210,112 B | 5,210,112 B | 5,337,088 B |
| On the card | 2026-11-23 20:04 | 2026-11-24 00:43 | 2026-11-24 03:07 | 2026-11-25 07:26 |
| Card readback | match | match | match | match |
| Host tests (passed / failed / ignored) | 2,879 / 0 / 4 | 2,444 / 0 / 4 | 2,444 / 0 / 4 | 2,776 / 0 / 4 |
| Eden before the install | Training and a waiting room; limited release | partial; one product failure | none | Training in part; no TrainingRooms row in Quick Play |
| The operator's reports on record | none | none | two (2026-11-24) | two (2026-11-25) |

The menu footer of each build ends with its commit. The install 4 file holds the text `rc15 a945b0c1`
(`strings` on `artifacts/phase23/install4/gate-a945b0c1/nro/libtraining_rooms-a945b0c1-release.nro`).

The host-test count falls between install 1 and install 2 because three test binaries stopped repeating the same
184 tests each (`artifacts/phase23/test-speed/HANDOFF.md`, the entry that counts 2,911 against 2,359). The counts are
each gate's own; they were not compared test by test here.

## Install 1 (`152629cd`)

- **Card:** `artifacts/phase23/card-install/20261123-2004Z-install1-152629cd/INSTALL.md`. Over DBI MTP. Before
  `0a17c793` (the build of `17060a5e`), after `3d6836da`. The other 20 plugins and 185 settings files were unchanged by
  name and size.
- **Gate:** `artifacts/phase23/fix-round-20261123/integration/gate/batch4-152629cd/status.json`: release, diag, test,
  clippy, no-default-features and size all exit 0.
- **Review:** inbox 2026-11-23 17:50:30Z, rv-integration: source, build and release ELF pass.
- **Eden** (2026-11-23, the full card line-up of 22 plugins on both hosts):
  - Start-up: inbox 18:29:51Z, rv-integration: all 22 plugins loaded on the Yoga and the XPS.
  - Verdict: inbox 19:43:11Z, rv-integration. Yoga, Training: 16 pass, 5 partial, 2 fail (the Guard the Edge start
    point: Preview and the grounded rest differed), 1 setup failure, 2 not run, 1 hand check, of 27. XPS, waiting room:
    Record and Play on the CPU gave the same turns as in Training; Crop saved the chosen range without changing its
    source.
  - Not run in the waiting room: the preview's delivery and timing rows (F7-3, F7-4, F7-10) and one Record row (f4-w2).
  - Release: inbox 19:48:07Z, rv-integration: cleared for a limited release, with those rows carried to install 2.
    The entry says no waiver is a row pass.
- **The operator's reports:** none on record for this build.

## Install 2 (`71158421`)

- **Card:** `artifacts/phase23/card-install/20261124-0043Z-install2-71158421/INSTALL.md`. Over hekate UMS, with the
  playbook's `upgrade2-ums.py`. Before `3d6836da`, after `1b5608ed`. 20 of 20 other plugins and 3 of 3 protected
  configs were byte-identical afterwards.
- **Gate:** `artifacts/phase23/fix-round-20261123/install2/gate/final-71158421/status.json` (release) and
  `final-71158421-rest/status.json` (test, clippy, no-default-features, size, diag): all exit 0.
- **Eden** (2026-11-23 and 24; the runs were stopped when the install was ordered):
  - Final verdict: inbox 2026-11-24 00:48:19Z, rv-integration: **partial, with one confirmed product failure**.
  - Passed: list wrapping, the 1 Frame label, both recording indicators, Record and Play on the CPU, the Playback Slot
    when idle and at both hit timings, the fighter pop-up's box, two of the frame strip's direction marks and the
    Guard the Edge start-point rows; in a waiting room, a fresh Stop, an ordinary Guard the Edge Stop returning the
    room's partner, and its Training Partner Behavior restored.
  - **Product failure:** with Lobby Practice turned Off during a running Guard the Edge drill, the room's partner did
    not return: the borrowed fighter stayed in all 169 reads after the menu closed. Fixed in install 3.
  - Not run or not shown: the preview counts, the drill preview, the chain picker's rows, the special-direction rows,
    the menu's hold on the CPU in a room, and no two-console result.
  - The install record's line "no product fail observed" was written at the install; the verdict above came just
    after it and is the final result.
- **The operator's reports:** none on record for this build. It was on the card from 00:43 to 03:07.

## Install 3 (`e7667f26`)

- **Card:** `artifacts/phase23/card-install/20261124-0304Z-install3-e7667f26/INSTALL.md`. Over hekate UMS. Before
  `1b5608ed`, after `eff66d92`. 20 of 20 and 3 of 3 byte-identical.
- **Gate:** `artifacts/phase23/fix-round-20261123/install3/gate/final-e7667f26/status.json`: all six steps exit 0.
- **Review:** `artifacts/phase23/fix-round-20261123/install3/RV-FINAL-e7667f26.md`: source, build and release ELF pass.
  It states that there was no Eden run before the install and that it is not a gameplay verdict.
- **Eden, after the install** (2026-11-24, this build):
  - Waiting room, run `rc15-i3-e7667f26-xps-f15-4-1`: inbox 14:34:27Z, rv-integration. The practice partner's Stop
    passed (120 consecutive reads). The install 2 failure is fixed: after Lobby Practice Off during a running drill
    the original fighter returned and stayed through 729 and then 900 reads.
  - Training, run `mod-inputs-yoga-01`: `artifacts/phase23/mod-tests/inputs-yoga/RESULTS.md`. Passed: the frame
    strip's marks for Pikachu's up special and Kirby's angled side tilt; drill files and grades for Pikachu, Pichu,
    Greninja, Palutena, Zelda and Mewtwo; Kirby's angled tilt as a drill.
  - **Product failures found there:** Sheik's, Fox's, Falco's and Wolf's special direction was saved as a second
    step, and after a Training reset the next drill attempt was not graded. Install 4 addresses both.
  - Not tested there: the waiting room, a hit combo with an up special, loading an older drill file.
- **The operator's reports:** two, on 2026-11-24, [below](#the-operator-s-reports-from-the-console).

## Install 4 (`a945b0c1`)

- **Card:** `artifacts/phase23/card-install/20261125-0703Z-install4-a945b0c1/INSTALL.md`. Over hekate UMS. Before
  `eff66d92`, after `99e00e92`. 20 of 20 and 3 of 3 byte-identical. Online Deluxe 0.10.9 went on in the same session.
- **Gate:** `artifacts/phase23/install4/gate-a945b0c1/SUMMARY.md`: all six steps exit 0.
- **Eden:** `artifacts/phase23/mod-tests/install4-final/RESULTS.md` (2026-11-25, Yoga, the full card line-up). The
  session ran on three heads, so each row is listed with its build:

| Part | Build | Verdict |
| --- | --- | --- |
| 1, Training (`i4-final-yoga-01`) | `a9131331`, two fixes before the installed build | Pass: the drill rows for Pikachu (two directions, a one-frame tap), a charged smash, Fox, Palutena, Mario steered against straight, Kirby's angled tilt; the angle half of Match angle and deflection; Move names in the drill list; the clip editor (open, layout, Y snap, both previews); Trim (198, 115, 88 and 5 frames for Off, Start, End and Both); the playback indicator for Player 1; the pop-up's ZR switch; scroll-back; a capture played on the CPU from RECORDINGS. |
| 1, Training | `a9131331` | **Product failure:** the reset chord by itself added a failed result, and with Load after On a short chord loaded a state. Fixed in `a945b0c1`. |
| 2, Quick Play (three boots) | `0d28433d` | Both windows were staged with this build before boot 1 (final-session `HANDOFF.md`, 05:33:42Z and 05:48:08Z). Setup stopped all three attempts before a Quick Play feature was exercised. |
| 2, Quick Play (boot 4) | `0d28433d` | Online Deluxe's font check only, pass. The match and its sudden death ended with no desync on both windows. |
| 3, Training (`i4-final-yoga-p3`) | `a945b0c1`, the installed build | Pass: the reset chord by itself, held 4 and 40 frames, on Mario's and Kirby's drills, adds no result; Load after starts no load after the chord; the two control drills grade as in part 1. |

- **Not exercised in the final Eden session** (from the install record): the airborne entry of a steered special;
  the deflection half of Match angle and deflection; in Quick Play the practice pick, a new pick and a rematch; ledge
  attacks beyond the host tests; Recordings beyond one played capture; step titles beyond the list; the clip editor in
  a waiting room; the Trim setting's migration; the CPU's playback indicator; the reset chord pressed one button at a
  time.
- **Repeated on the installed build:** the Mario and Kirby reset rows, Load after, and the Mario steering and Kirby
  tilt controls. Two airborne rows were attempted again, but neither produced a jump. The other part 1 rows were
  not repeated. The two fixes between `a9131331` and `a945b0c1` change a saved take's move name and the reset chord.
- **The operator's reports:** two, on 2026-11-25, [below](#the-operator-s-reports-from-the-console).

## The build before install 1 (`17060a5e`)

- **Card:** `artifacts/phase23/card-install/20261123-1140Z/INSTALL.md`: `0a17c793`, 2026-11-23 11:40.
- **The operator's reports:** at seven times on 2026-11-23, [below](#the-operator-s-reports-from-the-console).

## The operator's reports from the console

The operator's own reports from play on a Switch. Most were relayed by the coordinator (inbox 2026-11-25 17:09:15Z,
tr-primary-2). Two were recorded by the coordinator at the time, in its handoff
(`artifacts/phase23/HANDOFF-PRIMARY-tr-primary-2.md`); their rows name the entry. Each report is placed on the build
that was on the card at its time, by the install records' times above.
They say what the operator saw; none is a pass or a fail of a build. "Changed in" names the first install whose
history holds the change (`git merge-base --is-ancestor`), and what an Eden run showed of it. Reports about other
plugins are left out, except the first, which was made in Training and turned out to be another plugin's.

| When (UTC) | Build on the card | The operator reported | Changed in |
| --- | --- | --- | --- |
| 2026-11-23 12:10 | before install 1 (`17060a5e`) | In Training's character select, L3 + B did not send the CPU's token to the hand. | Not TrainingRooms: the Tile Cursor plugin, whose 0.2.1 went on the card at 15:32 that day (`artifacts/phase23/card-install/20261123-1532Z-tile-0.2.1/INSTALL.md`). |
| 2026-11-23 about 12:20 | before install 1 | In Training, the 1 Frame label at the top right left out the button to press. (Handoff entry 12:18:02Z.) | Install 1 (`2ae8d0fe`). Its Training row passed in Eden. |
| 2026-11-23 12:32 | before install 1 | Lists inside the menu (save states, macros) did not wrap around. | Install 1 (`2158e5ff`). Its Training rows passed in Eden. |
| 2026-11-23 12:32 | before install 1 | Recording Player 1 showed no indicator; red for Player 1 and blue for the CPU were asked for. | Install 1 (`2ae8d0fe`). Both indicators passed in Eden in Training. |
| 2026-11-23 12:32 | before install 1 | Recording as the CPU made the left and right input shudder (CPU Greninja, Player 1 Mr. Game & Watch). | Install 1 (`6c717136`). Record and Play on the CPU gave the same turns in Training and in a waiting room. |
| 2026-11-23 about 12:40 | before install 1 | Crop's preview kept the menu up and did not play to the end, and cropping a Player 1 macro did not work. (Handoff entry 12:56:08Z.) | Install 1 (`6bc005f6`): every preview plays through Recording's playback. In Eden a crop of a Player 1 macro saved the chosen range in Training and in a waiting room; the preview's delivery and timing rows did not run in the waiting room. |
| 2026-11-23 15:51 | before install 1 | Starting a new chain and going back before saving opened the keyboard, froze the menu for a moment, and left Player 1 and the CPU frozen after the menu closed. | Install 2 (`d6015e33`): B backs out of the picker and saves nothing; X is done. Its rows did not qualify in the install 2 Eden runs, so this is not shown in a game. |
| 2026-11-23 16:31 | before install 1 | In Guard the Edge, picking the CPU in the fighter picker and backing out left a black box over Hitbox Visualization. | Install 2 (`5f082c2f`): the pop-up gives back the pane it borrowed. Its visual row passed in Eden. |
| 2026-11-23 19:12 | before install 1 | The operator reported that the Guard the Edge case GJ1-2 worked on the build they were playing. That build counted the KO immediately. | Install 1 (`c40cb717`), installed 52 minutes later, waits for Player 1 to return before counting the KO. The earlier report did not test that change. The Eden row was dropped and left to a hand check, not marked as passed (inbox 2026-11-23 19:13:17Z). |
| 2026-11-24 19:35 | install 3 | Quick Play: after a match the loser declined the rematch, picked a different fighter and came back to the room; the earlier fighter was put back as both the CPU and Player 1. | Install 4 (`6146f8e9`). Not run in Eden: the final session exercised no Quick Play feature of TrainingRooms. |
| 2026-11-24 22:09 | install 3 | The recording crop and preview showed a black screen, without the fighters and the timeline. | Install 4 (`42be97a3`): the clip editor is a strip over the game. Its layout passed in Eden on `a9131331`. |
| 2026-11-25 13:30 | install 4 | The move names Jab 1 and Jab 2 are fine. | – |
| 2026-11-25 14:38 | install 4 | Playback took a second or more to start. Placement changes and a Delete for replay captures were asked for. | A change for the wait is merged for the next build (`445d1100`), host-tested only and not timed in a game. A change to placement is merged for the next build (`6dcf5c40`), host-tested only. The Delete is merged for the next build (`3fd708bc`), host-tested only. |

## The next build

Merges after `a945b0c1` are listed in the release notes. The Gate column names the receipt under
`artifacts/phase23/next-build/` for the head that passed all six steps. Playback was gated together with the two
documentation merges that followed it. These receipts are host tests and builds, not gameplay tests.

On 2026-11-25, Eden run `tso-3fd708bc-ally-1` used the diagnostic build of `3fd708bc` to check the Training stage
**before** the off-stage changes. Guard the Edge and Juggler did not start there, and the menu then read "Stop … Running"
(`out/p05/ally-173401-786500-p05-open.png` for Guard the Edge and `out/p06/ally-173435-667300-p06-reopen.png`
for Juggler in the run's folder). This was not a test of the fix,
which merged later as `7e6ff810` and is host-tested only, or of capture Delete. Receipts: `artifacts/phase23/training-stage-tracing/ally-boot/tso-3fd708bc-ally-1/` and the
17:39:04Z report from training-offstage.

A later Eden probe (`sg-probe-yoga-01`, diagnostic `0f39ec04`) read the Training stage's long floor and Battlefield.
The point below the long floor read Solid; the point beneath Battlefield read Under. This was not a run of the
merged head `afd4ce01` or of any placement or playback feature. Receipts:
`artifacts/phase23/next-build-smoke/20261125-1745Z/probe/evidence/sdlog/previous-184632.log` (lines 270-274)
and `current-184745.log` in the same directory (lines 375-379).

The plan was one short Eden smoke run on the finished build, then the operator's full test on the console (inbox
17:06:54Z, tr-primary-2). The smoke run is below. The operator's test has not happened.

### The Eden smoke boot of the finished build

On 2026-11-25, Eden run `nb-smoke-yoga-01` (22:21 to 22:57) ran the release build of `b7a382e4`, the head of merge 10,
with the card's 20 other plugins. Its NRO, sha256 `0cb29525…215ca2f`, is the release file of that head's gate
(`artifacts/phase23/next-build/gate-b7a382e4/status.json`). The game did not crash. Receipts:
`artifacts/phase23/next-build-smoke/20261125-1745Z/RESULTS.md` and, in full, `smoke/RESULT.md` there; for each row its
JSON and one frame under `smoke/evidence/out/<row>/`. Reported by dup-audit (inbox 22:57:42Z).

Two rows, the eggs zone and placement, passed on a later try in the same boot, after the test's own steps were
corrected (it looked for a row named Stop, and expected the placement card to open on Spot). The build did not change
between tries.

For Guard the Edge and Juggler, Start opened the set-up on its CPU page with no refusal. **Neither drill was played
through**: no CPU was launched and no attempt was scored.

| Row | Verdict | What was read |
| --- | --- | --- |
| Guard the Edge, Training stage (`s1-guard-training`) | pass | Start opened the set-up on the CPU page at (−125, −68), beside the long floor's left ledge |
| Under the stage On (`s2-guard-under`) | pass | the set-up reported the option On; the setting was put back |
| Guard the Edge from the island (`s2b-guard-island`) | pass | with Player 1 on the island's floor the set-up opened beside its left ledge, and Y moved it to the right one |
| Juggler, Training stage (`s3-juggler-training`) | pass | Start opened the set-up on the CPU page at (−45, 140) |
| Eggs-ercise's off-stage zone (`s4-eggs-offstage`) | pass | one zone, in the gap between the island and the long floor; the game started with Off stage only and no "No usable Off stage zone" |
| Recording picker's Off stage switch (`s5-place-offstage`) | pass | Player 1's spot went from on stage at (−30, 0) to off stage at (−125, −68); D-pad Right then moved it to (−124, −68). No placement was saved or loaded |
| Play on Player 1 (`s6-playback`) | pass | a 64-frame take was played: 64 of 64 frames delivered. The time to start was not measured |
| The clip editor (`s7-marks-preview`) | pass | the editor opened with four marks; X played the preview to its end |
| Guard the Edge, Battlefield (`s8-guard-battlefield`) | pass | the set-up opened with CPU midair jumps set to Used; the setting was put back |
| The waiting room (`s9-lobby`) | **not run** | the test's route stopped at the main menu before its first input, twice; the cause is in the test bridge, not shown to be in TrainingRooms. Nothing about the waiting room was tested |
| Settings after the rows (`z-audit`) | pass | none of the toggles compared differed from its factory value; 21 keys, including rosters and ranges, were not checked |

Not covered by the smoke boot:

- playing any drill through, and what the two Guard the Edge options do once the CPU is launched;
- the waiting room, which goes to the operator's test on the console (inbox 23:00:34Z, tr-primary-2);
- Record while an AI plays Player 1, the named grab, throw and edge-attack keys, B stopping a preview, Delete for
  replay captures, the form names on the editor's marks, the Playback delay option, and Change placement's save with
  its shared start save question.

The table uses the row files for verdicts and values. The first Guard the Edge frame was also checked for this page.
On Battlefield the frame shows a fighter-preload notice, not the set-up card; the bridge recorded the wizard and its
jump setting. The closing settings verdict comes from the audit, not its screenshot.

| Merge | Head | Gate |
| --- | --- | --- |
| Record while an AI plays Player 1 | `22b693b7` | `gate-22b693b7` |
| Named grab, throw and edge-attack keys | `76c00df3` | `gate-76c00df3` |
| Preview stop and the pop-up's chord line | `d807f1e4` | `gate-d807f1e4` |
| Delete for replay captures | `3fd708bc` | `gate-3fd708bc` |
| The clip editor's marks name the form of a move | `c51f4d81` | `gate-c51f4d81` |
| Stage geometry: one floor ray for every reader and the off-stage rule (no change in play intended) | `afd4ce01` | `gate-afd4ce01` |
| Playback starts without reading the library; Playback delay | `445d1100` | `gate-2d140506` (one gate for this merge and the two docs merges after it) |
| Guard the Edge and Juggler start from the floor the player stands on; two Guard the Edge options | `7e6ff810` | `gate-f9b81e42` (one gate for this merge, the next row and a docs merge) |
| Eggs-ercise's off-stage zone follows the stage | `1c6a71e5` | `gate-f9b81e42` |
| A save's placement is changed with the crosshair; the Off stage switch | `6dcf5c40` | `gate-b7a382e4` (one gate for this merge and a docs merge) |
