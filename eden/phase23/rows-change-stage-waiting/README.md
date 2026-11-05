# rows-change-stage-waiting (Eden `tr-p23-surface-cs3w`, 2026-11-05)

The rows as they ran (after the in-boot fixes) on the diag build of `p23/change-stage` `5d19922` (`f37110697f81…`): a
host-only Local Wireless room (the b3w2 route), Lobby Practice On (pushed through the diag settings reload: the fixture's
`[0,1]` is Off, option order `[On, Off]`), Change Stage -> Final Destination, two rotation entries (close the room and
create it again: Pokemon Stadium 2, then Town and City), Lobby Stages Direct -> Battlefield Omega, Change Stage ->
Smashville, the room afterwards, `trace.unmapped_access`. Later definitions in `steps.py` replace earlier ones (in-boot
fixes). The library's `waiting`, `native-pause` start gates match only the game's own waiting stage, so the waiting rows
gate on log lines. Image references stay with the run (game frames), not in git.
