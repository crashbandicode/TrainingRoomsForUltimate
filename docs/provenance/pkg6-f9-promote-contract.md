# F9 promotion contract — proposed, native implementation not yet qualified

Operator/lead direction in TRAININGROOMS-LEAD-INBOX.md, 2026-11-02 20:5x/21:0xZ,
supersedes delayed-count/setter replay. Preserve the requested count and retain
the helper body on newly active entry 2 or 3. Entry 2 holds CPU rotation's next
member, entry 3 the following member. Exact saved Mii identity is part of the
member, not merely its class kind. Defaults/capacity changes are separate work.

## Ownership transition

1. Before native activation, capture the exact scene/source generation, entry
   address, body IDs, selected member, costume and saved-Mii configuration.
   Exclude newly active entries from speculative and demand-source replacement.
2. Retain the existing native body, descriptor and resources. Do not call the
   old return-to-original installer, fake a destructor, or replay the count setter.
3. Transfer the helper receipt to a visible CPU receipt. Existing hat/item/portrait
   consumers must either retain a valid resource lifetime through this transfer
   or complete their real release before it; invalidating tokens alone is not
   evidence that native consumers stopped using the resource.
4. Publish identity, native selection and presentation through the existing
   field/selector/HUD paths, extended explicitly to these owners. Record the
   promoted value for Training reset and save/load; it must not alias CPU1's
   mutable rotation state. Scene/entry destruction retires only the exact receipt.
5. On decreasing count, wait for native deactivation to finish before re-adopting
   the exact parked entry; create fresh helper authority without destroying or
   reloading a retained body solely to change its ownership label. In slow mode,
   no helper occupies an active CPU entry while count exceeds one.

## Source findings and implementation gaps

- `character_sources::service` calls `service_owned` before admission gates.
  Loading source authority/INSTALL is retained until actual native completion;
  it cannot be dropped when the menu count changes.
- `SourceBook` supports Ready, Loading, Failed and Departed, but no promotion.
  `native_destroyed` invalidates leases and means actual destruction; it must
  not stand in for promotion. `returned_native` describes the superseded
  original-restoration load and also cannot represent retained-body transfer.
- `selected_request` currently picks demand first and then a missing speculative
  target. It does not guarantee CPU rotation's fixed next/next-after seat order.
  Scheduling needs an explicit seat contract, including exact-Mii values.
- `publish_mii_presentation`, `allow_portrait`, and `republish_after_rebuild`
  currently use the two visible-role state stores and owner0/1 guards. Simply
  calling them with owner2/3 either refuses or aliases CPU1. Promotion therefore
  needs owner-indexed publication receipts and reset handling, not a guard bypass.
- The existing native count reconciler's readiness scan covers the previous
  count (owned-ELF fact recorded earlier). A still-loading newly active helper is
  outside that scan. Retaining its body avoids a new reload but does not prove
  activation safe during its already-running load. Need a proven native
  activation/readiness boundary that preserves count and callback progress.
  No delayed setter or silent loading-helper restriction is authorized here.

## Evidence required

Original regressions: fixed seat selection; transfer with live leases; loading
completion after count increase; stale scene/entry callbacks; reverse adoption;
exact-Mii identity and independent CPU1/2/3 reset/save-load receipts.
Then compiled native stack/guard gates and batched Eden count
1→2→3→2→1, loading helper, exact Mii, L+R+A and save/load at 2–3 CPUs,
with full console zero Unmapped and all hook allocation deltas zero.
This document is a proposed source contract, not completion evidence.
