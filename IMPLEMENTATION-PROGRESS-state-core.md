# State-core completion — lead Astra

Objective: close required temporary restore, waiting quick reset and setup-library
interfaces without changing numbered slot contents or allocating in game hooks.
Base: `ebfcb9800fba1f82b2730b19ed26e231c810a2d6`, branch `stream/state-core`,
worktree `/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-state-core`.
Lead thread `01a19c96-0e78-7293-a76c-6f46a70be468`; no additional worker.

## Contract and source boundary

The lead completed the initial brief/spec/playbook intake (`ae87443`) and read
SPEC-state-06 completely again before this implementation. Section6.4 supplies
the quick-reset contract. The state machine, records, runtime state callbacks,
input adapter and practice lifetime providers were read; no mixed/upstream
implementation was opened. `bounded-behavior-fix` and
`bounded-validation-evidence` apply.

## First candidate — NOT GATED

- `Machine` owns an optional temporary pair separately from `SaveFile`. It uses
  the same death/placement/feature-wait machine, restores observed damage and
  unmirrored pose, preserves mirror/random-slot history, refuses overlapping
  restore or pending Save, and never sends the temporary pair to persistence.
- Raw native entry and logical state role are separate. Waiting entry4 is the
  practice CPU; entries1/2/3 and extra helpers cannot acquire a state record.
  Native Training retains roles0/1. Pokémon/Kirby record reads use the active
  restore source and preserve actual native entry IDs for game calls.
- Waiting quick reset caches first WAIT pose/damage per entry lifetime, clears
  on practice retirement or departure, reads native facts outside state locks,
  swallows the eligible held chord, clears the delay queue and requests once.
  Missing poses produce the specified120-frame notice. No JSON snapshot or
  numbered slot is used for this cache.
- Tests added for slot preservation across both callback orders, cancellation,
  overlapping requests, waiting role mapping, first-WAIT lifetime, physical and
  digital shoulder chords, and repeated temporary restores under the plugin's
  counting allocator. The new game-only QUICK lock is registered in hook_locks.

Formatting and diff whitespace checks pass. **No Cargo job or Eden run yet.**
No compile/Eden claim is held by this stream. Freeze before the queued host
tests; one bounded Cargo command per ledger claim with ordered output and
source verification. All-target clippy0 and no-default build remain required,
then Switch release/diag, stack/size and fresh Eden waiting/native regressions.

## Remaining work

Source review after45ba42f caught a lifetime error in the new adapter: the
widened Training predicate is false during hot swap as well as departure.
Clearing the first-WAIT cache on that predicate alone would replace an
entry-lifetime pose during a suspended callback. The departure path now checks
the underlying native/lobby scene facts; actual practice retirement still
clears it. Runtime acceptance must include a same-entry hot swap followed by
quick reset, and an actual entry retirement. No passing runtime claim yet.

Preview core is still the existing explicit stub: primary behavior-only
start/stop/retry contract requested in LEAD-INBOX. Setup Library UI has no core
selected-slot snapshot/bind provider yet. Settings Profiles has no located
approved behavior contract. These rows remain OPEN; no acceptance inferred
from adding the generic temporary-pair mechanism.

Review the new lifetime and completion paths, run tests, integrate only exact
gated commits in the lead merge order, and retain final0alloc/0Unmapped proof.
Worker pkg4 owns recorder/panels; pkg6 owns roster/Mii; UI owns screens. Preserve
their independent changes when composing this slice.
