# Phase 23 — worker B's tracepoint list (CPU-Kirby hat lookup, prepared 2026-11-04)

For the primary's one combined trace boot. Arm these only if the diag build's own
`[KirbyHats] role_body role=1 case=…` line (added on `p23/b4-hats`) does not land or leaves the case
ambiguous — the diag line already splits accessor-missing from kind-mismatch and prints the kind it
saw; the trace adds the entry memory and the native return values behind it.

**Build:** the phase candidate's diag NRO (worker B: `6bea7449…`) with its unstripped ELF for
`symbols load` (build id must match). **Module:** `libtraining_rooms.nro`.

**When to arm:** after the fixture settles (Training mode, ready-go, the `[GroupedVisibleCapacity]
creation … owner=1` line with `active_kind=6` in the diag log), before the first CPU hat chord;
disarm after the chord attempt (the `request role=1` or the `role_body role=1` line ends the window).

| # | Target (symbol) | Capture | Condition | Decides |
| --- | --- | --- | --- | --- |
| 1 | `libtraining_rooms.nro!<…kirby_hats::sample_shortcut>` entry, `on_return` | x0–x3; on_return x0 | `x3 == 1` (right click → role 1) | the chord reached the sample on the P1 path again (glm-03: x2=0 x3=1); the return's x0 pairs the sample outcome |
| 2 | `<…game::natives::fighter_entry>` entry, `on_return` | entry x0; on_return x0 | `x0 == 1` (Training's CPU entry) | the entry object pointer the game's manager answers for entry 1 (0 = the entry/manager guard refused → accessor-missing case) |
| 3 | `<…game::natives::entry_active_body>` entry, `on_return` | entry x0; on_return x0; captures `[x0+0x4150]` u64 (installed), `[x0+0x5918]` u8 (active index) | `x0 != 0` | the running body the guarded F11 reader resolves for the CPU entry, with the game's own active-cell fields at the same instant (glm-03's bank snapshot said kinds `[6, 0x28]`, count 2) |
| 4 | `main!<app::utility::get_kind>` entry, `on_return` | entry x0 (the accessor), on_return x0 (the kind, i32) | window is armed, no address condition | every kind the chord window reads: P1's lookups pass through the same call, so the CPU's call is the one whose accessor is not `role_body(0)`'s; a `0x28` (Lucas) return on the CPU's accessor is the kind-mismatch case at the native source |
| 5 | `libtraining_rooms.nro!<…natives::module_accessor>` entry, `on_return` | entry x0; on_return x0 | `x0 == 1` | the wrapped accessor answer (0 = None) at the same moment as #2/#3, so the plugin's wrap is not the divergence |

**Order:** arm 2, 3, 5 (cheap, entry-scoped), then 1 (the chord edge), then 4 last (hottest). Rate
limits: default 1,000 records/s; the chord window is short, no hot-loop point is included.

**What each outcome fixes:** `case=kind` with `kind_seen=0x28` and `active_index=1` → the game's
active cell points at the banked second body while the bank's ready state believes the Kirby member
is live — the fix follows the active cell in the swap bookkeeping (bank `active` vs `+0x5918`).
`case=accessor` with `active_index=2, installed=2` → the plugin's strict `active_cell` (index <
count) refuses an index the game's own reader accepts (`b.ls` allows index == count) — the fix
matches the game's reader or proves the game never does this. `case=accessor` with `entry_ptr=0` →
the entry guard/manager fact, not the swap engine.
