# Phase 23 trace request — worker C, HB1 (**fallback only**; no Eden boot by this worker)

**Fallback only (C2):** the diag build now runs the hurt census itself — one
`[Hitbox] hurt census` line per fighter kind per 600 frames (`parts=`,
`parts_max=`, `first_pass=`, the read's `ticks_min/avg/max`), over the three
relied HitModule rows restored in `sites_hitbox`. The rows file (rows-C.md)
takes C1/C4 from that line. Request this trace boot only if a combined Eden
boot shows the census line absent or contradicting the recorded counts
(hb-03/hb-04); everything below is that fallback's plan, unchanged.

What the host cannot settle and one combined trace boot can: **Ridley's
hurtbox part census** — how many parts group 0 of his `HitModule` holds and
their shapes — which the Ridley first-pass row (C3) needed before the draw
route was wired (route B, 2026-11-04: wired; the census answers it without a
trace). The lr02 capture
(`docs/HB1-LR02-DISPOSITION.md`) read one part of one fighter and its group
descriptor (`first 0`, `count 10` for that fighter); it does not census
Ridley. No other fact is requested: the per-frame cost comes from the diag
build's own `[Hitbox] stats` lines, and the pool behaviour is already recorded
(hb-04, ly-01).

## The one point (13.0.5 main-relative; the lr02 disposition's own plan)

| Moment | Address | Registers | Memory | Decides |
|---|---|---|---|---|
| The hurtbox part update's outer loop, during Ridley's first idle seconds in Training (the same moment lr-02 captured) | `main+0x469e50` | `pc, x19, x26, x27, x22, x23, x21, sp, x30` | 128 B from `x19` (the executing HitModule: group count, group/part table bases, part total, owner link) | Ridley's group-0 `first`/`count` from the group table at `x19+0x60` (`0x3c` stride, first `+4`, count `+6`, both `i16`) and the part total at `+0x78` (`i32`) |

Follow-up (same boot, same rules, only if the first hit lands cleanly): a
bounded table sample — one capture of `count x 0x50` bytes from the part table
pointer read out of the 128 B above — gives each part's status word, capsule
centres, size and node hash; that is the census. Keep it one capture, one
fighter, one moment; it is not a universal census, bone-transform contract or
visual acceptance (lr02's own boundary, unchanged).

## Rules carried from the lr02 disposition

Natural scheduled moments only; the lead is the sole driver; one request, one
hit at a time, `timeout_ms <= 600000`; record current `B`, build/NRO identity,
mapped address, `pc` and raw capture; memory is CPU-visible, not an atomic
snapshot. No game bytes or native derivatives committed. This is planning, not
an arm order; existing lead requests keep priority.
