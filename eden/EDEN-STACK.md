# Eden: thread and hook stacks (stream/stack), 2026-11-02

Yoga, Eden-GIF `d483c082…`, profile `/home/intpa/trainingrooms-eden-st/profiles/y` (hard-linked copy of
ssbu-desync-1029/profiles/b), muted (core.volume 0 set and saved through the control API before `game.load`), display :99,
one Eden at a time, claim/release per boot in the Yoga ledger. Driver: `eden/stack/st_run.py` over the small-diff copies of
the playbook's `solo.ps1` / `gv.py` (`IMPLEMENTATION-PROGRESS-stack.md`), pkg4's screen-gated route to Training.
Plugins: TrainingRooms diag **6541f2d** `5d21cf2fbaf9f558…` (static nn::os stacks, lock-free diag registry), tile cursor
`b8313ca2…`, CPU pilot `e3033426…`. Logs per run under `eden/runs/st-NN/` (gitignored); numbers below from
`python3 -I tools/stack_table.py eden/runs/st-*/practice-diagnostics*.log` (tested: `tools/test_stack_table.py`).

## The fault being fixed

s4-07 (pkg4) and u4b-02 / ui-09: at boot, `Unmapped Write64 @ …81b80` / `…9b80` / `0x10a061ab80`, then `Cannot execute
instruction at unmapped address 0x0`; SP `…9b90`, saved FP `…b3f0`, R28 `…9e38` in every case. The thread was
`tr-trace-writer` (64 KiB): its body's frame was 59,712 B (an 11 KB Mii editor trace moved by value), so the SDK call
under it ran 0x480 below the stack. Fixed by `BoxedSlot` (by reference, buffers made at init): static depth 61,968 → 2,624 B.

## Runs

| Label | Kind | Result |
| --- | --- | --- |
| st-01 | cold | not booted (driver: claim-check edit read `$Matches` after a second `-match`; fixed); no claim, no Eden |
| st-02 … st-09 | cold ×8 | boot to the install line, two heartbeat beats; every `[ThreadStack] spawn` line; console scan 0 (panic/abort/crash/Unmapped/Cannot execute) in all 8 |
| st-10 | busy | save (tr-state-diag took `diag-save.req`, tr-state-writer `state file_written`), load, `replay_capture.ini` by tr-ini-writer, diag settings reload published; the record row stopped at Standby (driver did not move P1; fixed); console scan 0 |
| st-11 | busy | save, load, capture toggle passed; after the settings-reload request the plugin's log stopped (no heartbeat beat after 519088743) with no fault line: stream/sync's publish-path lock livelock, not a stack fault; console scan 0 |
