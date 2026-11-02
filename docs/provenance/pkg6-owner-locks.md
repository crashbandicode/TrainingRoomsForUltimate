# Cold owner locks

The counted first-lock path is distinct from OS TLS. UI's retained `a05601b`
Switch ELF (`3d6251837c8ba6b67a0d6a6313cd893a29fbe8b216348660670ae150e60c5f31`)
shows ordinary-player State lock `0x5fa98`, null OnceBox branch `0x5fabc` to
`0x1dd294`, allocation counter at `0x1dd2a4`, then `calloc(40,1)` at `0x1dd2b0`.
This evidence was supplied by UI in the lead inbox at 21:46Z; it is not an
independent runtime attribution of every `note` allocation.

The replacement uses the existing `hos_lock::Mutex`: constant AtomicU32,
acquire/release exclusion, non-reentrant guards, and Horizon address-arbiter
wait/wake SVCs. It does not require game-thread-only ownership. No guard extent,
lock order, native call, readiness condition, or value-publication operation
changes. The release profile aborts on panic, so post-unwind poisoning is not a
reachable native continuation; previous poison-recovery wrappers in RHandle and
CopyPkg become plain guards. `try_lock` still refuses contention immediately.

## Audited scene chain and ownership

`input_delay::note -> lobby::update -> training::lobby::update` detects a mode
transition and invokes `character_roster::scene_changed` and
`practice::scene_departed`. These locks are also used by their owners' regular
callbacks; preserving exclusion avoids assuming those callbacks cannot overlap.

| State | Entry paths and protected contract |
| --- | --- |
| roster STATES[2], WAITING_MEMORY | scene/input/service/destruction and settled roster queries; role state and remembered value remain under the same guards |
| ordinary player/cpu STATE | scene/service/destruction, catalog publication and settled configuration; exact descriptor/epoch/transaction checks remain inside guard |
| saved ALPHA, auto-init STATE | scene cancels/retire; native menu handoff and catalog completion; auto-init drops coordinator before entering ordinary owners |
| body BANKS[2], BANK_STORAGE[2] | scene/service/destruction plus boot reservation; retain all bank ownership and native call boundaries |
| resource LEDGERS[2], copy HELD[2] | scene reset/forget and bank service; release recipes still leave guards before native calls |
| source STATE, MEMORY_REFUSED | scene/source service/lease clients; source STATE -> helper STATE -> registry order retained |
| helper STATE, ordinary-native COMMITTED | source/native transaction and read-only provenance; registry performs no native calls, existing G -> transaction -> registry order retained |
| HUD STATE | scene/native service/render adapter; existing nonblocking callback reads stay nonblocking |
| item PENDING | scene cancel and native item service; same lease/cancellation boundary |
| hat STATES[2], SHORTCUT, OPERATOR_SHORTCUT | scene/input/service/save restoration; scene drops each role guard before optional reporting |
| practice OWNED | scene/frame/destruction and save/record acquisition; entry checks precede acquire, guard released before clear_session_state |
| practice-stage STATE | UI context/request/status, native stage update, scene departure; nonblocking refusal and epoch invalidation retained |
| training-stage ROTATION | sequence initialization, entry/reentry, resource callback and direct-stage completion/departure; menu snapshot precedes rotation guard, stage STATE -> ROTATION order retained |

The composed `4f9878d` diag ELF proves another cold scene path:
`practice_stage_request::depart` at `0xa2018` calls its standard-mutex
`try_lock` at `0x5c3f8`; the null branch at `0x5c408` reaches OnceBox
initialization at `0x1f7e1c`. Nonblocking acquisition did not avoid lazy
allocation. Neither stage owner's init acquires its lock. Stage-request
retirement can enter ROTATION via `direct_uncertain`; the native sequence
hook also acquires ROTATION on first use. Both now use the same constant
primitive, with unchanged guard boundaries and callback ordering.

Parent `character_roster` retains its standard Mutex import for unmodified probe
and inherited owners. Explicit OwnerMutex types select only roster/waiting
states. Five probe readers need only the return-guard API adjustment when they
read those states; their separate probe locks are unchanged. Native catalog
broker is already initialized in boot and is outside this cold scene slice.
Editor/keyboard and named UI storage are not globally rewritten.

## Required evidence

The cold/fresh-host-thread test exercises the actual public primitive's first
lock, held-lock refusal, settlement and repeated reads with zero allocations.
Existing lock tests cover mutual exclusion and sleeping-waiter handoff. Host
futex waiting differs from Horizon; these tests do not prove native no-heap.
Exact composed Switch call-path inspection, stack gates and runtime first-use
counts remain required. No warm-up, counter waiver, stack increase, or whole-R5
acceptance follows from this source change.


## Preload bookkeeping continuation

`character_sources::scene_changed` invokes `cycle_preload::scene_end`, which
touches GRACE, CONDITION, REQUESTS and STATUS even before preload first use.
Settings changes and visible-player planning use those same value owners.
GRACE never takes a source/roster/native lock; final source admission reads
its atomic generation/deadline snapshot. REQUESTS releases its guard before
finishing metrics against sources. CONDITION is released before metrics.
STATUS snapshots desired values before source readiness, and retains its
existing notification-lock nesting only in show_status. No reverse ownership
or reentrant assumption is introduced by the primitive replacement.

Exact `80ee800` diag shows cold OnceBox `0x1f82dc` from GraceCoordinator
lock `0x5c690`, Status lock `0x5c6c8` and Settings lock `0x5bf14`.
close_requests `0xbde90` calls the OperationStamp-array standard mutex
at `0x5be34`; reset_status `0xbda44` and reset_admission_grace `0xbe110`
reach the corresponding cold locks. Complete disassembly is archived as
cycle-*.txt with that ELF. This source slice changes only those four lock
primitives/guard APIs; no metrics, admission, notice or load-order change.

The direct compiled note call graph additionally reaches preload_metrics SESSION
through close_requests -> OperationStamp::finish, lock `0x5c5e4`. Its only
owners are finish and snapshot: sample first, absorb/snapshot under the guard,
drop before diagnostic formatting/heap-profile output. This value-only session
uses the same constant primitive. Native panic=abort excludes the old poisoned
fallback; no successful record or disabled-measurements behavior changes.
The remaining catalog-broker lock is already initialized by its boot owner,
as recorded above. Direct-call reachability does not prove indirect paths absent.


## Managed creation registry

native_source_capacity::managed CREATED is a fixed numeric receipt table.
Only created/register/unregister acquire it; they copy or update table slots,
without native calls or nested owner locks. init sets ENABLED after checking
the load contract and does not initialize the old mutex. Exact80ee lock5be6c
is reachable from eight native callback roots (all-hook-standard-locks.json).
The constant primitive preserves exclusion and index/lifetime decisions;
release panic=abort excludes the old poisoned empty-table continuation.
