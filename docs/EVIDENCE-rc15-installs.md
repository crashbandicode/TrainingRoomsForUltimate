# rc.15 installs 1 to 6: what was installed and what was tested

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

| | Install 1 | Install 2 | Install 3 | Install 4 | Install 5 | Install 6 |
| --- | --- | --- | --- | --- | --- | --- |
| Commit | `152629cd` | `71158421` | `e7667f26` | `a945b0c1` | `b7a382e4` | `5df3d9dc` |
| Release NRO (sha256, first 8) | `3d6836da` | `1b5608ed` | `eff66d92` | `99e00e92` | `0cb29525` | `c1ced861` |
| Size | 5,226,496 B | 5,210,112 B | 5,210,112 B | 5,337,088 B | 5,373,952 B | 5,402,624 B |
| On the card | 2026-11-23 20:04 | 2026-11-24 00:43 | 2026-11-24 03:07 | 2026-11-25 07:26 | 2026-11-25 23:06 | 2026-11-26 12:22 |
| Card readback | match | match | match | match | match | match |
| Host tests (passed / failed / ignored) | 2,879 / 0 / 4 | 2,444 / 0 / 4 | 2,444 / 0 / 4 | 2,776 / 0 / 4 | 3,048 / 0 / 4 | 3,108 / 0 / 4 |
| Eden before the install | Training and a waiting room; limited release | partial; one product failure | none | Training in part; no TrainingRooms row in Quick Play | a smoke boot on the same file: nine of ten checks. After the install, a second boot opened the menu in a one-player waiting room | No smoke result on the installed release file. Two Training runs used the diagnostic file from the same commit. Earlier results on `eb39d6bc` are listed below. |
| The operator's reports on record | none | none | two (2026-11-24) | two (2026-11-25) | three (2026-11-25 and 2026-11-26) | Scrub positions without move poses; separately, a title-screen crash with no TrainingRooms frame (2026-11-26) |

The menu footer of each build ends with its commit. The install 4 file holds the text `rc15 a945b0c1`
(`strings` on `artifacts/phase23/install4/gate-a945b0c1/nro/libtraining_rooms-a945b0c1-release.nro`). The install 5
file holds `rc15 b7a382e4` and `0.2.0-rc.15` (the copy kept with its install record): its version was not raised.
The install 6 file holds `rc15 5df3d9dc` and `0.2.0-rc.15` (merge 17's gate release file).

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
tr-primary-2; the report on install 5, inbox 23:33:51Z). Two were recorded by the coordinator at the time, in its handoff
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
| 2026-11-25 14:38 | install 4 | Playback took a second or more to start. Placement changes and a Delete for replay captures were asked for. | A change for the wait is in install 5 (`445d1100`); its smoke boot played one take, and the start was not timed then; it was timed in Eden on 2026-11-26 ([Install 6](#install-6-5df3d9dc)). A change to placement is in install 5 (`6dcf5c40`); its smoke boot checked the Off stage switch only. The Delete is in install 5 (`3fd708bc`), host-tested only. |
| 2026-11-25 about 23:33 | install 5 | In the clip editor, the refusal that In is mid-action kept showing after a snap to the move start, so the clip could not be previewed or saved. | Not fixed in install 5. On `58c9ccb5`, with install 5's snap code, 4 of 24 host editor cases failed, including plain Y; the held-modifier control passed (`artifacts/phase23/clip-snap/gate-red-7982989b/logs/test.log`; inbox 23:49:51Z). Install 4 has the same snap code. The fix merged as `dbded2d1` (pin `3990f50e`; six gate steps exit 0). The install 5 smoke boot only previewed a take without moving a handle. Later, the release snap row on `eb39d6bc` showed Y resolving In so X previewed and A saved. This was a test capture, not the operator's own take. The fix is on the card since install 6 (2026-11-26). |
| 2026-11-26 about 11:53 | install 5 | Placement on a save: a fighter switched from on stage to off stage floats, as it should. If it is then moved into the ground it lands, and it stays on the ground as the crosshair moves. The operator called it mostly a nit; switching On stage and Off stage again puts it right. | Not fixed in install 6 (merge 17). Assigned for the following build (inbox 11:57:13Z, with the operator's words). A code review identified why the hold may not lift a landed fighter; this still needs confirmation in Eden (12:09:32Z). |
| 2026-11-26 about 11:53 | install 5 | Guard the Edge: the fighter sometimes starts slightly off the chosen spot, and not the same way each round. With one off-stage spot over ten rounds, the first two were in one place, the third slightly lower, the fourth higher again. | Not changed. Assigned for a later build, cause first (inbox 2026-11-26 11:57:13Z, tr-primary-2, with the operator's words). No cause is established, and it has not been reproduced. |
| 2026-11-26 12:29 | install 6 | The game crashed right after A on the title screen; the operator guessed it is intermittent. | No TrainingRooms frame is in the crash, and its owner is not established. The game's resource inflate thread faulted in the game's own copy into a fresh block, for a stock file; ARCropolis only forwards that call. The block's first page could not be written, and what left the heap in that state is not known (inbox 13:35:17Z, tr-primary-2; `artifacts/phase23/hw-crash-20261126-1229Z/SUMMARY.md`). A May 25 report predating TrainingRooms has the same signature. Under investigation by another lane; no fix was tested. Not listed as a TrainingRooms issue. |
| 2026-11-26 about 12:41 | install 6 | In Create macro from, moving the playhead moved the fighters in space but did not show their moves. The operator also asked to choose the recorded or current character separately for Player 1 and the CPU. | Install 6 stores positions but not move animation frames. Both requested changes are in design, not in this build (inbox 12:41:51Z, tr-primary-2). No cause for the reported uneven movement was established. |

## Install 5 (`b7a382e4`)

- **Card:** `artifacts/phase23/card-install/20261125-2306Z-card-b7a382e4/deployment.json`. Over DBI's USB file
  transfer, at 23:06. Before `99e00e92`, after `0cb29525`, read back. 21 plugins before and after; the other 20 are
  byte-identical. The old file is kept on the card as `ultimate/TrainingRooms/backups/pre-0cb29525506021e3.nro`, and
  its readback matches `99e00e92`.
- **Gate:** `artifacts/phase23/next-build/gate-b7a382e4/status.json`: all six steps exit 0; 3,048 tests passed, 0
  failed, 4 ignored. The installed file is that gate's release file.
- **Later docs build:** `f55dfbbe` changed only documentation. Its release NRO embeds the later commit tag and has a
  different SHA-256 (`0414f116…`); it was not installed. The card readback still matches `b7a382e4` (`0cb29525…`).
  Compare `artifacts/phase23/next-build/gate-f55dfbbe/SUMMARY.md` with the install gate above.
- **Eden:** the smoke boot below, on this exact file.
- **The operator's reports:** one, on 2026-11-25, [above](#the-operator-s-reports-from-the-console).

The changes in this build were merged one by one after `a945b0c1` and are listed in the release notes. The Gate column names the receipt under
`artifacts/phase23/next-build/` for the head that passed all six steps. Playback was gated together with the two
documentation merges that followed it. These receipts are host tests and builds, not gameplay tests.

On 2026-11-25, Eden run `tso-3fd708bc-ally-1` used the diagnostic build of `3fd708bc` to check the Training stage
**before** the off-stage changes. Guard the Edge and Juggler did not start there, and the menu then read "Stop … Running"
(`out/p05/ally-173401-786500-p05-open.png` for Guard the Edge and `out/p06/ally-173435-667300-p06-reopen.png`
for Juggler in the run's folder). This was not a test of the fix,
which merged later as `7e6ff810` (the smoke boot below opened both set-ups on that stage), or of capture Delete. Receipts: `artifacts/phase23/training-stage-tracing/ally-boot/tso-3fd708bc-ally-1/` and the
17:39:04Z report from training-offstage.

A later Eden probe (`sg-probe-yoga-01`, diagnostic `0f39ec04`) read the Training stage's long floor and Battlefield.
The point below the long floor read Solid; the point beneath Battlefield read Under. This was not a run of the
merged head `afd4ce01` or of any placement or playback feature. Receipts:
`artifacts/phase23/next-build-smoke/20261125-1745Z/probe/evidence/sdlog/previous-184632.log` (lines 270-274)
and `current-184745.log` in the same directory (lines 375-379).

The plan was one short Eden smoke run on the finished build, then the operator's full test on the console (inbox
17:06:54Z, tr-primary-2). The build was installed nine minutes after the first Eden boot ended. A later Eden boot
checked the waiting room. The operator reported the clip-editor refusal on the console; no full hardware verdict
is on record.

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
| The waiting room (`s9-lobby`) | **not run** | the test's route stopped at the main menu before its first input, twice; the boot used an older build of the test bridge (dup-audit, inbox 23:07:47Z), so the cause is not shown to be in TrainingRooms. Nothing about the waiting room was tested in this boot. The operator then asked for the check (about 23:04); see the second boot below |
| Settings after the rows (`z-audit`) | pass | none of the toggles compared differed from its factory value; 21 keys, including rosters and ranges, were not checked |

Not covered by the smoke boot:

- playing any drill through, and what the two Guard the Edge options do once the CPU is launched;
- the waiting room, which the second boot below covers in part;
- Record while an AI plays Player 1, the named grab, throw and edge-attack keys, B stopping a preview, Delete for
  replay captures, the form names on the editor's marks, the Playback delay option, and Change placement's save with
  its shared start save question.

The table uses the row files for verdicts and values. The first Guard the Edge frame was also checked for this page.
On Battlefield the frame shows a fighter-preload notice, not the set-up card; the bridge recorded the wizard and its
jump setting. The closing settings verdict comes from the audit, not its screenshot.

### The waiting-room boot

On 2026-11-25, after the install, Eden run `nb-lobby-yoga-01` (23:28 to 23:39) ran the same release file with the
card's 20 other plugins and the current test bridge. The game did not crash. Receipts:
`artifacts/phase23/next-build-smoke/20261125-1745Z/lobby/evidence/out/s9-lobby/` (the row file and one frame).
Reported by dup-audit (inbox 23:39:37Z).

| Row | Verdict | What was read |
| --- | --- | --- |
| The waiting room (`s9-lobby`) | pass | a Local Wireless room was created and its waiting room reached, as host and alone; TrainingRooms' menu opened there on Fighters & Stages, and its footer reads `rc15 b7a382e4` |
| Guard the Edge in the waiting room (extra, in the same row) | read from the logged lines only | Start opened the set-up on its CPU page at (−160, −69) with no refusal. No frame was taken of it, and the drill was not played |
| Settings after the row (`z-audit`) | **receipt failed** | the settings compare found no difference among the toggles it compares (21 keys not checked), but an extra read of the fighters got no answer in the waiting room, and the row is marked failed for that |

Not shown by this boot: a second player in the room, Lobby practice with a partner, any drill played through, and
leaving the room (the test's leave step stopped, and the boot's shutdown ended the room).

For this page the row file's `pass` field, the room, the set-up's values and both audit files were read, and the
frame was looked at.

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

## Install 6 (`5df3d9dc`)

- **Card:** `artifacts/phase23/card-install/20261126-1156Z-card-5df3d9dc/deployment.json`, state
  `installed_verified`. Over DBI's USB file transfer, finished at 12:22:55. Before `0cb29525`, after `c1ced861`,
  read back. 21 plugins before and after; the other 20 and the protected files are unchanged. The old file is kept
  on the card as `ultimate/TrainingRooms/backups/pre-c1ced861e4c67a54.nro`, and its readback matches `0cb29525`.
- **The installed file** is merge 17's gate release file,
  `artifacts/phase23/next-build/gate-5df3d9dc/nro/libtraining_rooms-5df3d9dc-release.nro`: 5,402,624 B and the same
  SHA-256, hashed for this page.
- **Ordered by the operator** on 2026-11-26 at about 11:53 (`operator-approval.json` in the same folder; inbox
  12:23:14Z, tr-primary-2).
- **Docs-only merge:** `main` already contained `7edddeaa` when the card was written. That merge did not rebuild
  the NRO; the installed file is from `5df3d9dc` (f9-fix, inbox 12:17:41Z; deployment finished 12:22:55Z).
- **Eden at installation:** the two running Training boots used merge 17's diagnostic NRO (`853964b2…`), not the
  release file read back from the card. The release smoke and waiting-room checks were still pending. The first
  report on the two diagnostic Training runs was posted at 12:45:51Z, about 23 minutes after installation.
  [Merge 17 test status](#eden-on-install-6-itself-5df3d9dc-merge-17)
- **The operator's reports:** scrub positions moved without the move poses. The earlier title-screen crash has no
  TrainingRooms frame; its owner is not established. The [reports table](#the-operator-s-reports-from-the-console) records both.

The changes in this build were merged one by one after install 5 (`b7a382e4`). The Gate column names the receipt
under `artifacts/phase23/next-build/`; the gates are host tests and builds. The Eden runs follow the table.

| Merge | Head | Gate |
| --- | --- | --- |
| The clip editor: Y on In resolves the move on a capture | `dbded2d1` | `gate-dbded2d1`: six steps exit 0; 3,049 tests passed, 0 failed, 4 ignored |
| The clip editor: a free playhead | `ce2dba91` | `gate-6dbf9aa9` (one gate for this merge and a docs merge): six steps exit 0; 3,052 tests passed, 0 failed, 4 ignored |
| The clip editor: the fighters shown while scrubbing, with their positions in a file beside the macro | `cdeb4d69` | `gate-eb39d6bc` (one gate for this merge, a tools merge and a docs merge): six steps exit 0; 3,106 tests passed, 0 failed, 4 ignored |
| The clip editor: a fighter is not put where it cannot stand on the stage (a fix to the row above) | `5df3d9dc` | `gate-5df3d9dc`: six steps exit 0; 3,108 tests passed, 0 failed, 4 ignored |

- **Not run in a game, by their merge records.** All four say so. The snap fix's adds that the operator's own take
  was not reproduced; the playhead's, that holding a direction on a pad has no host test; the scrub positions', that
  the Switch-only code is executed by no test and that its last review was still open at the merge; the fix's, that
  it is not shown that the stage reads in a room.
- **Why the fix:** an Eden session on `eb39d6bc`, the build before it, found on a match capture that both fighters
  stood beside their recorded points while the line read held (row SP-10; dup-audit, inbox 2026-11-26 10:24:00Z).
  tr-primary-2 ruled it a defect to fix in this build (10:27:48Z).
- **Repeated Play, host only:** starts took 0.98 to 1.07 times as long as on the previous `main` (`6dbf9aa9`).
  This range does not describe the first Play of an old macro without positions.
- **First Play that generates positions, host only:** the median of seven paired differences in cold-start
  overhead was 28 microseconds at 3 s (passes the 100-microsecond limit), 127 microseconds at 30 s (over the
  limit), and 764 microseconds at 10 min (inconclusive, not a pass). The 30 s and 10 min verdicts were left to the
  in-game comparison with install 5, which is in the Eden part below. Source:
  `artifacts/phase23/scrub-positions/READY-ERRATUM-d8713ee4.md`.

### Eden, on the build before the fix (`eb39d6bc`)

One session by dup-audit on 2026-11-26, on merge 16's release NRO (sha256 `d4557201…`) and diag NRO (`8abda32f…`).
**Nothing in this part ran on merge 17, the installed build.** Receipts: `artifacts/phase23/scrub-positions/eden-session/evidence/`.
The first run is install 5's own build, as the baseline.

| Run (folder) | Build | Passed | No verdict |
| --- | --- | --- | --- |
| `scrub-pb0-xps-01` (`pb0/`) | install 5, `b7a382e4` | The Play-start counts in the next table. | – |
| `scrub-rel-xps-01` (`rel/`) | `eb39d6bc`, release | The Play-start comparison. Record, then In held with no preview, on a 214-frame take. An old macro read "Play it once to see positions while scrubbing." and X still previewed it. Snap: with In one frame into an attack X played nothing, Y moved In to the attack's start, X previewed and A saved. Browse: the playhead reached frames 188 and 158 with In 0 and Out 214 unchanged. Both fighters were within 0.5 of their expected positions **after applying the shared offset**, not the raw coordinates in the file. | The playhead's cuts and its snap, and the second and fast snap cases, did not fly. |
| `scrub-xps-01` (`diag/`) | `eb39d6bc`, diag | Record with no preview, In follows, Out on the last frame, release on deselect (SP-1 to SP-4). The old macro (SP-7); positions from one Play, and at once after X (the two generation rows). A crop keeps its positions (SP-8). File sizes (SP-9). A stale positions file (SP-20). Rename (SP-22): pass with the keyboard note below. | The match capture (SP-10) is the defect row. Release on close (SP-6) was spoiled by its own row. Not flown: SP-5, SP-11 to SP-13, SP-17, SP-19, SP-21, the CPU take. The run ended on its own 90-minute limit. |
| `scrub-room-a-yoga-01`, `scrub-room-b-yoga-01` | `eb39d6bc` | A second Eden joined the first one's waiting room (the join rows). | The room rows (SP-14 to SP-16) did not fly. |

Frames from the Play press to the first played input (`evidence/compare_pb.txt`, which ends `PASS`; XPS, the same
bridge and the same three staged old macros on both builds):

| Macro | Install 5 `b7a382e4` | `eb39d6bc` |
| --- | --- | --- |
| Old ten-minute macro, the session's first Play | 27 | 26 |
| Old 30-second macro, its first Play | 11 | 10 |
| Old 3-second macro, first Play | 9 | 9 |
| Old 3-second macro, three later Plays | 9, 9, 9 | 9, 9, 9 |
| A 3-second take recorded in the boot | 9, 9, 9 | 9, 9, 9 |
| A 30-second take recorded in the boot | 10, 10, 10 | 10, 10, 10 |

- **What the first Plays are:** on `eb39d6bc` an old macro has no positions, so its first Play is the one that
  makes them. After it the ten-minute macro had a generated positions file with the 297 frames that played before
  the row stopped it, and the 30-second macro one with all 1,800 (dup-audit, inbox 08:10:00Z).
- **Limits of the counts:** one boot per build on one host; one sample each for the two long macros; whole frames
  only, so nothing under a frame is shown; the takes recorded in the two boots differ by a few frames in length.
  This is Eden, not the console.
- **What carries over:** the Play-start comparison is not repeated on merge 17: the fix changed the scrub hold
  and stage check, not the playback path (rv-preview-vis, inbox 11:07:22Z). The rename path is also unchanged.
  SP-22 **passed with a note** (tr-primary-2, 11:15:37Z): the automated pad attempt failed. Typing into Eden's
  own keyboard window changed "Macro 3" to "Macro 3q"; the list and Crop showed the new name, the ID and positions
  file's hash stayed the same, and Crop still held. See `eden-session/packet-a/gate/handoff45-rename.json` under
  `artifacts/phase23/scrub-positions/`. This did not test typing on the console. The other rows above report
  what ran on merge 16, not a fresh merge-17 result.
- **Logs:** dup-audit counted 0 lines with panic, abort, Unmapped, crash, Fatal or a break call in each run's Eden
  console log, the baseline included (inbox 11:15:03Z). The three runs that the runner stopped at its time limit
  show "CRASH (core dumped, signal 11)" in the Down summary; each log ends on the limit second.
- **Not established by this session:** anything in a waiting room beyond the join; a fighter swap or a scene change
  while a fighter is held; scrubbing the final frame of a long recording (SP-17); the menu thread's stack depth
  (SP-19 could not be read in that boot).

### Eden, on install 6 itself (`5df3d9dc`, merge 17)

**In testing; the results so far are on the diagnostic NRO.** Merge 17's NROs are release sha256 `c1ced861…`
(the file on the card) and diag `853964b2…` (`artifacts/phase23/next-build/gate-5df3d9dc/nro/`). Receipts:
`artifacts/phase23/scrub-positions/eden-session/evidence/`. Source: dup-audit, inbox 2026-11-26 12:45:51Z; the row
files named below were read for this page.

| Run (folder) | Host | Passed | Defect | No verdict |
| --- | --- | --- | --- | --- |
| `scrub-m17-xps-01` (`m17-xps/`) | XPS | SP-1 to SP-4 (record with no preview, In follows, Out on the last frame, release on deselect), SP-5 (release on X), SP-6 (release on close), SP-7 (old macro), SP-8 (a crop keeps positions), SP-9 (file sizes), SP-19 (menu thread stack) | SP-17 (long take); SP-10 (a fighter pushed off its point beside a wall) | SP-11 stopped in the test's own take |
| `scrub-m17b-yoga-01` (`m17b-yoga/`) | Yoga | SP-1 and SP-8 again as fixtures, SP-21 (delete removes both files), the take on the CPU, P1, P2, the no-move select and P3 (the playhead rows), S2 and S3 up to their preview | S1: the preview after the snap was refused (below) | SP-12 and SP-13: the swap chord and the pause menu did not act with the editor open, so the drop of a hold was not reachable; the hold stayed and nothing hung. Not a pass of the drop |

| Row | What the reads show |
| --- | --- |
| SP-17, long recording | **Defect accepted** (12:48:20Z). Two closed-menu takes kept 1,553 and 3,253 inputs, but only 1,200 positions each. On the first, Out read `pos=end`, the line said "Positions end here.", and the file was 86,440 B. Takes of 182 to 222 frames had full files. |
| Why the track can end early (code reading) | With no other work pending, the worker can skip allocating the next 600-frame block (`recording_host.rs:291`). It is not a fixed maximum. `eb39d6bc` has the same code, but this row did not run on it. |
| Existing short file on install 6 | Play preserves a valid file written by Record, even when short (`play_track.rs:151–157`). Fixed in the next build. |
| SP-5 and SP-6, release | On X the preview played with no `pos` and fed the take's 222 frames, and the fighters were released after. One B closed the editor, with both fighters idle at their homes on two reads (13:20:01Z; accepted 13:20:24Z) |
| SP-19, the menu thread's stack | `used=49904 size=90112`, under the 57,648 bound. Read after SP-1 to SP-10 and before the long take ended |
| SP-10, both flights | At frame 30 both fighters were at the expected points. At frame 137, Player 1's point was inside the floor; `pos=off` and Player 1 stayed at home. In was mid-action, so the snap hint hid the position warning. |
| SP-10, 12:08 flight | The CPU reached its open point at (−86.96, −15.10). This flight stopped later at the test's fighter-switch check. |
| SP-10, 13:01 flight | The requested CPU point (−81.72, −15.10) was beside the floor's end. It was not refused; two reads put the body **5.13 units left** of the point, against the wall. The row failed on that displacement, not the fighter-switch check. `eb39d6bc` showed a similar displacement. |
| Wall verdict | **Defect accepted** (13:20:24Z). **What it is, read from the stage's lines and not run** (preview-vis-sol, 13:42:30Z; ruled 13:43:10Z): the point is not inside the wall. The Training floor's solid run is x −80 to 620, so the point is 1.72 left of its end and the stage rule rightly answers open (`src/model/stage_geometry.rs` lines 222 to 254 read one column through the point and meet no wall). The fighter's body has width: it rests at x −86.85 when asked to x −81.72 and is exact at x −86.96, which fits a body pushed until it clears a wall at x −80. The editor does not read back where the game left the body. Fixed in the next build. |
| First X on a capture: two takes | In `tut-5df3d9dc-yoga-3`, Y snapped In but X refused with In selected; no input frame was fed (12:46:48Z). In the control take, L deselected In; after a one-second wait X previewed on the CPU (12:51:05Z). The earlier successful release snap on `eb39d6bc` previewed on Player 1. |
| S1 on the diagnostic build (third flight, 13:24:09Z) | In stepped to frame 7 and X played nothing, as wanted. Y moved In to 6. X then gave one `playing=1` line, the notice "Let the CPU settle to idle and leave the ledge before previewing", and no input frame; the CPU's status changed one frame after the check. No Replay-window preview fed a frame in that whole boot, so S2 and S3 show their snap and not a working preview. The Crop preview of a recorded macro (SP-5) did feed. On `eb39d6bc`'s release build the same steps fed 101 frames. **Treated as a regression of install 6 until the release smoke says otherwise** (13:24:36Z, tr-primary-2); whether it comes from merge 17 or from the diagnostic flavour is not separated |
| Why X can refuse (code reading, not a fix test) | A first temporary checkpoint requires a grounded, idle CPU (`states/preview.rs:128–137`). The request can reach it before the scrub hold releases the CPU. A macro loading its own start save bypasses that checkpoint. A fix is assigned, not tested here. |

- **Not flown yet:** a smoke boot of the release NRO, which is the file on the card, and a fresh waiting-room pair.
- **Not shown in the two test boots:** release through a fighter swap or a scene change (SP-12, SP-13), or a
  fighter walked off stage (SP-11). No match-capture preview fed inputs in `scrub-m17b-yoga-01`. In the separate
  filming boot, the deselected-In control **did** feed inputs; it is not a pass for X with In still selected.
  The playhead rows passed on Yoga; the filming boot also showed browsing, Y to a move and both cuts on one macro.
