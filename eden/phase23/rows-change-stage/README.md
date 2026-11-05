# rows-change-stage (Eden `tr-p23-surface-cs1`, 2026-11-05)

The rows as they ran (after the in-boot fixes), on the diag build of `p23/change-stage` `5d19922` (`f37110697f81…`):
rows 00-03 reach ordinary Training (from the rc.2 combined rows); 04-06 change the stage three times through the Change
Stage row (Final Destination, Battlefield Omega, Smashville); 07 save/load and L+R+A after the changes; 08
`trace.unmapped_access` (log mode). `cs_cursor.py` reads the Fighters & Stages cursor from a frame (the menu keeps each
page's cursor, and a menu-session reset puts it back on the first row).

The image-gate references (`refs/`) are frames of the game and stay with the run
(`/home/intpa/eden-runs/tr-p23-surface-cs1/rows/refs/`), not in git. Playbook `tools/eden-runner` at `0b68204`.
