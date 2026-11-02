# Eden: the Replay capture row on the Deluxe Plugins page (deluxe stream)

2026-11-02, Yoga, display `:94`, own profile `/home/intpa/trainingrooms-eden-deluxe/profiles/dx` (hard-linked copy of
`ssbu-desync-1029/profiles/b`: 591 MB unique beside the 6.4 GB source), muted (`core.volume` 0 set, saved and read back before
`game.load`). Driver: `eden/dx_eden.py` / `route_dx.py` / `rows_dx.py` (playbook `gv.py`/`solo.ps1` lineage via pkg6's harness).
Outputs under `eden/runs/dx-NN/` (gitignored): `console.log`, `run.json`, `memory.peak`, PREP/LAUNCH receipts, logs, shots.

Build: tree `14016fe` (stream/deluxe with work/implementation 566780a merged), **diag** NRO
`a7c0c64831b566e5826ca8b8b7bba1e080ba30aeb038ce145fd41125df32a818` (release of the same tree `3b1a39c2…`).
Other plugins: Deluxe 0.10.1 = the card's `a8dcda242b890f09…` (2026-11-01 MTP install copy), its config = the card's with only
`plugins` removed from `hide_pages` (`96710302ea94db19…`, the m5q/M5o k04 config; the card itself hides the Plugins page),
CPU pilot 0.2.1 `e30334261370a094…`, tile cursor `b8313ca2…`.

| Label | Ledger | What ran | Result |
| --- | --- | --- | --- |
| dx-01 | Surface 13:38Z claimed / released at once | Up failed in `prep` (harness bug: a `diag_txt` line outside its `if`, fixed in `3121464`) before any display or Eden; same-minute claim by pkg6's s6-07 (priority), so the Surface was left to it | aborted, retried as dx-02 on the Yoga |
| dx-02 | Yoga 13:4xZ claimed, 13:4xZ released | with Deluxe 0.10.1: route to Training, boot gates, Deluxe window (`L+R` held + `Down`, twice: Settings, then the shared Plugins page), `ZR+Right`, then the menu row's path (`diag-capture.req off`) | **pass** |
| dx-03 | Yoga 13:4xZ claimed, 13:49Z released | without Deluxe: route, boot gates, the menu row's path (`on`) | **pass** |

## dx-02 (Deluxe 0.10.1 present)

- Install line: `named_macros=installed live_replay=installed deluxe_row=installed … heap_allocations=0`.
- `#15 [TrainingRooms] deluxe_row registered handle=1 descriptor=4 host_status=0x1 host_build=0.10.1-03e9d42ad2ef-1793545262`
  (handle 0 is the CPU pilot's own page): the real host accepted SDK 5.0.0's version-4 descriptor in one call; menu hooks active.
- `#16 [ReplayCapture] guarded live adapter ready capture=off host_registered=true save=entry`: with a registered row the
  adapter is resident although the INI said off.
- Plugins page (`shots/…-deluxe-cycle1.png` second pass): group **Training Rooms**, row **Replay capture  Off**, selected.
- `ZR+Right`: `#303 replay_capture.ini written by the writer thread enabled=true`, file `[capture] enabled=on`, row **On**
  (`after-on`). Only `replay_capture.ini` exists in the folder (no host-written `.previous.ini`): one writer.
- Menu row's path (`writer::request_replay_capture`): `#319 diag_request replay_capture enabled=false`, `#320 … written …
  enabled=false`, file `enabled=off`, Deluxe row **Off** (`after-off`): both directions agree.
- Allocations (`hook_stats`, diag): 0 through boot and menus (`#38`–`#42`); from the Training scene on (`#164`, before the
  Deluxe window was first opened) a steady 100–110 per 10 s window, the same before and after both Deluxe edits (`#289` 100,
  `#310` 110, `#324` 100). The Deluxe path adds none (its callbacks run on the host's draw thread, CAS and atomics only).
  The steady per-window allocations belong to areas this tree lacks fixes for (lead: sync slice 2, pkg6 R5), not this stream.
- `console.log` scan: 11 lines, all at 7.37 s of boot (`Unmapped Write64 @ 0x10dffeab80` just below a stack, then execute at
  0): ledger row F7 (thread stack), whose fix (`6541f2d`, `dc737c1`, `aa18732`) is not in `14016fe`. No `Unmapped`/panic line
  in the registration or toggle windows.

## dx-03 (no Deluxe)

- Install line: `live_replay=not_installed reason=capture_off_or_guard deluxe_row=not_installed reason=host_missing …
  heap_allocations=0`; `#15 [TrainingRooms] deluxe_row not registered reason=host_missing error=HostMissing host_status=0x0`.
- Menu row's path `on`: `#203 diag_request replay_capture enabled=true`, `#204 … written … enabled=true`, file `enabled=on`.
  As expected without a host and with the INI off, the capture adapter is not installed at this boot, so On takes effect at the
  next boot (open item for package 4, `IMPLEMENTATION-PROGRESS-deluxe.md`).
- `console.log`: the same 11 F7 boot lines (6.98 s), nothing else.

Profile restored after dx-03 (`dx_eden.py restore`: the source's `libarcropolis.nro`, `libssbu_desync_detector.nro` back),
host released, Eden processes left 0.

Not covered here: the real TrainingRooms menu screen showing the row's value after a Deluxe edit (the menu reads
`writer::replay_capture()`, the same preference; host-tested, not screenshotted); hardware.
