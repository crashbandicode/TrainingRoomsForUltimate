# Test provenance, package 6 (the fork's engines carried: hot swap, preload, Kirby hats, items, Mii, lobby practice, CPU-as-P1)

`Requirement | host test | Eden row | status`. The carried engines bring their own host tests (the fork's unit and integration tests, run in
every by-path test binary; counts overlap) and are not listed one by one: see `docs/provenance/pkg6-carry-ledger.md` and
`tests/carry_pkg6.rs` (a hand edit of a carried file fails). This file lists what the clean-room work of package 6 added, and the Eden rows
(labels `s6-NN`, results in `eden/EDEN-PACKAGE-6.md`). Status: **host pass**, **Eden pass**, **runtime, unrun** (written and Switch-compiled,
no Eden row yet), **not built** (reported).

## Allocator and features (lead ruling 2026-11-02: one global allocator, every feature builds)

| Requirement | Host test | Eden row | Status |
| --- | --- | --- | --- |
| The plugin has one `#[global_allocator]` (`runtime::alloc`: the counting allocator, or the recorder's forwarding `TraceAllocator` under `console-practice-trace`) | `pkg6_features::the_plugin_has_one_global_allocator` | - | host pass |
| The byte meter of the one allocator counts live, peak, total, calls and largest request (`plugin-heap-meter`, `console-practice-trace`) | `hook_alloc::the_byte_meter_counts_what_the_one_global_allocator_serves` | diag boot with `diag,plugin-heap-meter`: `[HeapMeter]` lines | host pass |
| Every feature of the table builds, alone and on the default set, and the allocator features together | `pkg6_features::the_feature_matrix_lists_every_feature` (the list); `python3 -I tools/feature_matrix.py --run` (the builds: host `cargo check`, Switch `cargo skyline check`) | - | see `IMPLEMENTATION-PROGRESS-pkg6.md` |
| `default` is the shipped list (minus `deluxe-sdk`, L1); no diagnostic feature in it; every probe requires `diag` | `pkg6_features::*` | - | host pass |

## Owners of the moments the fork called the unused helpers from (lead ruling 2026-11-02)

| Requirement | Host test | Eden row | Status |
| --- | --- | --- | --- |
| Solo Popo (no Nana): a load of Popo out of REBIRTH ends at Popo, no `NanaPosMove` marker | `state01_machine::state01_solo_popo_has_no_handoff` | native Training, Solo Popo on, save and load | host pass; runtime, unrun |
| The Ice Climbers hand-off with Nana is unchanged | `state01_machine::state01_nana_handoff` | native Training, Ice Climbers, save and load | host pass |
| Kirby's saved hat comes back through the hat engine without the hat's owner in the match; a cancelled load drops the queued restore | none host-side (game memory) | Kirby hats row | runtime, unrun |
| A menu change of the 13 preload rows reaches the engine's INI and cache (`publish_applied` from the menu commit) | `cycle_preload_config` INI transaction tests (carried, 9 clean-room tests) | change a preload row in the menu or by `diag-settings-next.json`: `[CyclePreload] applied` | runtime, unrun |
| Held item save/restore is the item engine's alone; the duplicate natives are gone | `pkg6_item_table`, `state04_items` | item save/restore with the owner not in the match (`diag-item.req`) | runtime, unrun |
| An automation never touches a helper body of the cycle (entries 2, 3, 5, 6): by role, not by a reserved test | `pkg2` role-table tests (`model::cpu::role`) | managed Kirby / third source row | host pass; runtime, unrun |
