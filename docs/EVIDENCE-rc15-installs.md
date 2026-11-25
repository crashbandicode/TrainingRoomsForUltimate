# rc.15 installs 1 to 4: what was installed and what was tested

2026-11-25. The receipts behind [the rc.15 release notes](RELEASE-NOTES-rc15.md) and the status on
[Current release](CURRENT-RELEASE.md). Three claims are kept apart:

- **Installed:** the plugin file on the card was read back and matched the build. This is a deployment check.
- **Tested in Eden:** a named run gave a verdict for a row, on the build named with it.
- **Hand-tested by the operator:** a report from the console is on record.

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
| Operator hand-test report | none on record | none on record | none on record | none on record |

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
- **Hand test:** none on record for this build.

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
- **Hand test:** none on record for this build.

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
- **Hand test:** none on record for this build.

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
- **Hand test:** none on record for this build.

## The build before install 1 (`17060a5e`)

- **Card:** `artifacts/phase23/card-install/20261123-1140Z/INSTALL.md`: `0a17c793`, 2026-11-23 11:40.
- **Hand-tested by the operator, 2026-11-23:** recorded in `artifacts/phase23/HANDOFF-PRIMARY-tr-primary-2.md`, the
  entries of 12:18:02Z and 12:56:08Z. Reported: lists did not wrap; nothing showed that a recording was running; a CPU
  recording's left and right shuddered; Crop's preview kept the menu up and did not play to the end; cropping a
  Player 1 macro did not work; the 1 Frame label left out its button. These became the fixes of install 1.

## The next build

Merges after `a945b0c1` are listed in the release notes. Each merged head passed the six gate steps on a build host
(`artifacts/phase23/next-build/gate-<commit>/`); none has run in Eden or on a console.

| Merge | Head | Gate |
| --- | --- | --- |
| Record while an AI plays Player 1 | `22b693b7` | `gate-22b693b7` |
| Named grab, throw and edge-attack keys | `76c00df3` | `gate-76c00df3` |
| Preview stop and the pop-up's chord line | `d807f1e4` | `gate-d807f1e4` |
| Delete for replay captures | `3fd708bc` | `gate-3fd708bc` |
| The clip editor's marks name the form of a move | `c51f4d81` | `gate-c51f4d81` |
