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
| st-12 | busy | stopped by me during the route at the lead's release order (Down: Eden 0); console scan 0 |

The recording row and the menu open/close row move into integrator-3's merged-tree boot (stream/sync + stack).

## Numbers (st-02..st-12; diag 6541f2d stack sizes)

| Thread | Stack (B) | High-water used (B) | Least free (B) | low |
| --- | --- | --- | --- | --- |
| boot | 65536 | 38160 | 27376 | no |
| replay-capture-save | 131072 | 24400 | 106672 | no |
| tr-archive | 65536 | 7712 | 57824 | no |
| tr-diag-reload | 131072 | 9840 | 121232 | no |
| tr-heartbeat | 65536 | 9072 | 56464 | no |
| tr-import | 262144 | 10080 | 252064 | no |
| tr-ini-writer | 65536 | 9344 | 56192 | no |
| tr-macro-diag | 65536 | 8768 | 56768 | no |
| tr-menu | 262144 | 13488 | 248656 | no |
| tr-settings-load | 262144 | 9488 | 252656 | no |
| tr-state-diag | 65536 | 8592 | 56944 | no |
| tr-state-load | 262144 | 18624 | 243520 | no |
| tr-state-writer | 131072 | 15408 | 115664 | no |
| tr-trace-writer | 65536 | 1696 | 63840 | no |

| Hook / callback | Calls | Least free at entry (B) | Stack region (B) | Region |
| --- | --- | --- | --- | --- |
| ui_pass_calculate | 597 | 195184 | 8753152 | ? |
| other | 17561 | 523536 | 917504 | ? |
| cpu_motion_change | 16 | 1021280 | 1048576 | ? |
| cpu_shield_param | 5310 | 1022336 | 1048576 | ? |
| cpu_transition_term | 13300 | 1022560 | 1048576 | ? |
| cpu_check_button_on | 7956 | 1022752 | 1048576 | ? |
| live_replay | 424 | 1025760 | 1048576 | ? |
| pipeline_after_tick | 352 | 1028128 | 1048576 | ? |
| cpu_stick_x_no_clamp | 4 | 1028816 | 1048576 | ? |
| cpu_stick_y_no_clamp | 4 | 1028816 | 1048576 | ? |
| cpu_stick_y | 212 | 1029152 | 1048576 | ? |
| cpu_stick_x | 212 | 1029200 | 1048576 | ? |
| pipeline_before_tick | 352 | 1030080 | 1048576 | ? |
| cpu_frame | 212 | 1030208 | 1048576 | ? |
| ui_archive | 21 | 1031520 | 1048576 | ? |
| cpu_check_button_off | 4 | 1032448 | 1048576 | ? |
| reset_check | 101 | 1032656 | 1048576 | ? |
| native_service | 1444 | 1032672 | 1048576 | ? |
| hazard_rule | 1 | 1033120 | 1048576 | ? |
| input_caller_a_apply | 3320 | 1033504 | 1048576 | ? |
| input_caller_a_note | 516 | 1033504 | 1048576 | ? |
| ui_register_calculate | 4 | 1034832 | 1048576 | ? |
| reset_hub | 2 | 56227312 | 60899328 | ? |

Region ?: these runs predate the `kind=` field. A region of 1,048,576 B is the main thread; 917,504 B is the 896 KiB thread
of the `other` HookScope sites; 8.7 MB / 56-60 MB regions are heap blocks holding a fiber stack (not a stack bound).

## Sizes from these numbers (dc737c1)

Every row of `model::thread_stacks` = max(static, measured) × 1.25 + 16 KiB in pages (`tests/thread_stacks.rs`); the hook
budget (`tools/stack_depth.py <diag elf> budget`) allows (523,536 − 16 KiB) / 1.25 = 405,721 B of static depth per hook on a
thread stack and 16 KiB on a fiber: 134 roots, 0 over (the deepest, the native service dispatch, ≥ 185,328 B).

