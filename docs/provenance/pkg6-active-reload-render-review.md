# Active fixed-seat Reload has no qualified parked render boundary

Bounded review of the existing transaction at pkg6 `110fb77`, requested by the
lead at 2026-11-03 04:57:29Z. This returns a contradicted proposal, not an
activation workaround or an operator restriction.

The native install does deactivate the outgoing body, but that does **not**
offer a completed inactive replacement on which the current portrait renderer
may run. The existing managed transaction publishes exact Mii data during
loading and accepts its resource completion only after native activation.

1. Source admission validates the outgoing active identity and absence of live
   consumers. `begin_promoted_reload` creates a new Loading SourceToken under
   the existing INSTALL owner. The old promotion is not the new source's
   completion proof. `mii_managed_helper::prepare_reload` uses the existing
   exact G snapshot and selects `Flags::Reload`, bytes `[1,0,0]`.
2. Owned `653240` stages the descriptor into FE+70 at `6533a8..6533e8`, calls
   `653490(entry,1)` at `6533ec`, then writes outer=3 at `653424`, the Reload
   flag to FE+5931 at `653430`, and loader=0 at `65343c`. This is an inactive
   **loading** interval, not parked-ready resource completion.
3. The existing 650010 dispatcher performs plugin pre-service, then original
   native service, then gate leave. Exact Mii prequeue publication requires
   loader6 and its current ticket; `mii_selector_native::publication` requires
   the actual loading snapshot. Refusing this native callback is a retained
   terminal ticket, not a reversible pause that creates a parked lifetime.
4. `mii_managed_helper::completed_details(active_reload)` requires
   `native_source::active_identity`. `service_owned(active_reload)` likewise
   requires active identity and `ready_active`, then full G publication/job
   completion, before SourceBook.complete, a new promotion, registry proof and
   INSTALL release. `EntryState::ready` requires outer6, loader14, guard0 and
   the complete canonical body/index/count shape. No source ReadyToken is
   published for an intermediate inactive replacement.
5. Existing owned-native HUD design already directly rejects rendering inside
   this hot-reload interval: `mii-hud54-design/DESIGN.md` section2 and
   `ROOT-FLAG-RESPONSE.md` section4 show the renderer runs both 653490
   deactivations and 655f60/656ee0. During loading it forces outer3→6→0 under
   the loader; after completion it deactivates the live body. Template-write
   suppression does not suppress those effects. Keeping the old HUD request
   alive protects its texture, but does not protect the reloading fighter.

Therefore the proposed use of an **already existing** inactive-ready phase is
not supported by this transaction. Initial parked pre-render remains viable;
active exact-Mii target changes still require a different qualified root
contract. No active render, new hook, native flag write, forced lease release,
count freeze or fourth-created-row assumption was added.

Evidence: exact owned ELF
`0789f0a4ae9652f335eca5eabc7df3e58b0954012a0bebdea374f317706536c2`;
bounded fresh excerpts `tmp-pkg6/reload-install.asm` and `reload-service.asm`;
12-entry loader jump table read by `tmp-pkg6/reload-jump-table.py`;
existing `artifacts/enhancements/mii-hud54-design/{DESIGN,ROOT-FLAG-RESPONSE}.md`.
An attempted LLVM `-s` range dump ignored its range and emitted excessive
output; it was discarded, not used as evidence. The corrected table reader
reads only the ELF header/program headers and 48 table bytes. No compile,
Eden, profile mutation or new native activation was performed.
