# hos-lock

`Mutex` and `RwLock` for Skyline plugins (Super Smash Bros. Ultimate on Horizon). They are
[`lock_api`](https://crates.io/crates/lock_api) raw locks on one 32-bit word that wait in the
kernel's address arbiter: no spin, no heap, `const` constructors. The API is parking_lot's, so code
moves over by changing its import:

```rust
use hos_lock::{Mutex, RwLock};          // was: use parking_lot::{Mutex, RwLock};
static STATE: Mutex<u32> = Mutex::new(0);
*STATE.lock() += 1;
```

Types: `Mutex<T>`, `MutexGuard`, `MappedMutexGuard`, `RwLock<T>`, `RwLockReadGuard`,
`RwLockWriteGuard`, `MappedRwLockReadGuard`, `MappedRwLockWriteGuard`, and the raw `RawMutex` and
`RawRwLock`. Neither lock is reentrant. The `RwLock` lets writers go first: a read waits while a
writer waits, so a thread must not take a second read lock on a lock it already reads (parking_lot's
`read` has the same rule). Guards are not `Send`.

## Why

Dated 2026-11-02 (TrainingRoomsForUltimate, Eden run ui-09). Closing the plugin menu froze the game
every time. Probes written straight to the kernel debug output showed the menu worker stopping at
`STATE.lock()` of a `parking_lot::Mutex` that the game's input poll takes every frame.

- `aarch64-skyline-switch.json` has `"os": "switch"` and no `target-family`. parking_lot_core 0.9
  (`src/thread_parker/mod.rs`) therefore picks `generic.rs`, whose `park()` is
  `while parked { spin_loop() }`, a pure busy loop.
- Horizon schedules by fixed priority. A waiter that spins on the core of a lower-priority holder
  keeps that holder from ever running again, so the lock is never released and the game livelocks.

This crate's waiters sleep in the kernel (`svc 0x34` WaitForAddress, `WaitIfEqual`), and an unlock
with sleepers wakes them (`svc 0x35` SignalToAddress, `Signal`). The numbers, the argument registers
and the semantics come from the kernel the tests run on (Eden `src/core/hle/kernel/svc.h`, `svc.cpp`,
`svc/svc_address_arbiter.cpp`, `k_address_arbiter.cpp`, `svc_types.h`, `svc_results.h`). The value
check and the sleep are atomic under the kernel's scheduler lock, and a negative timeout waits
forever. Every early return (a mismatch, a timeout, a spurious wake) makes the lock re-check its word.
`horizon::self_test()` checks the kernel's result codes at run time (a mismatched value gives
`InvalidState` 0xFA01, a zero timeout gives `TimedOut` 0xEA01, a signal with no sleeper gives 0).

The state machines (`src/word.rs`) are the Rust standard library's futex mutex and futex
reader-writer lock, without their spin phases and `downgrade`. A wake is treated as possibly having
woken nobody, because the arbiter does not report how many threads it woke.

Off the Switch, `src/host.rs` provides the same wait and wake over one std `Mutex` and `Condvar`, so
the state machines run in host tests.

## Tests

```text
cargo test --manifest-path crates/hos-lock/Cargo.toml
```

`src/word.rs` tests the state machines with a test futex that also returns spuriously and on a
timeout. They cover exact counts under contention across 6–8 threads, the hand-off to a sleeping
waiter, an uncontended unlock that wakes nobody, writes that stay whole with several readers and
writers, and a waiting writer that keeps new readers out. `tests/locks.rs` covers the same through
the public types on statics.

## License

MIT (`LICENSE`).
