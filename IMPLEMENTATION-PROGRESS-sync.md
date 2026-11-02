# Implementation progress, sync stream (the menu-close hang's root fix)

Dated 2026-11-02. Owner: ui-screens (Claude, lead 0caa72b8). Branch `stream/sync` from `27d84f4`, worktree
`/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-sync`. Standalone on purpose (lead): only the lock module, the import changes and the
hook-lock treatment, so integrator-3 merges it first and every stream picks it up.

## Root cause (Eden ui-09, ui-screens diag 9af41937, evidence in the ui-screens branch `eden/EDEN-UI-SCREENS.md`)
- Probes: `input push B`, `close 10/11/12/1`, `publish 1..5`, then nothing; the guest froze. Probe 6 follows `features::chords::publish`
  (`STATE.lock()`, parking_lot), which the game's P1 poll (`chords::on_poll`) takes every frame.
- `aarch64-skyline-switch.json`: `os: switch`, no `target-family`. So `parking_lot_core` 0.9.12 `thread_parker/mod.rs` picks `generic.rs`, whose
  `park()` is `while parked { spin_loop() }`. Horizon schedules by fixed priority, so a waiter spinning on the holder's core starves the holder forever.
- std on this target (skyline-v3 rust-src): Mutex/Condvar are pthread (nn::os), and RwLock/Once are queue-based with a pthread parker. They block in
  the kernel, but the Mutex boxes its pthread mutex on first lock (`OnceBox`), which is a heap allocation in hook scope.

## Lead conditions (2026-11-02) and status
1. Standalone branch from 27d84f4: done (this worktree).
2. Primitive: atomic word + `svc 0x34` WaitForAddress / `svc 0x35` SignalToAddress, const init, no heap. `src/model/lock_word.rs`
   (std's futex mutex and rwlock without spins or downgrade), `src/runtime/sync.rs` (lock_api types, asm SVCs, diag self-test).
   Evidence for the SVCs: Eden kernel source (ids `svc.h` 0x34/0x35, register order `svc.cpp` SvcWrap_*64, semantics
   `k_address_arbiter.cpp` WaitIfEqual/Signal, enums `svc_types.h`, results `svc_results.h`). The stock 13.0.5 main NSO has no arbitration SVC
   (they live in the sdk NSO, not dumped here). The diag self-test checks the result codes at runtime. Host tests: contention, spurious wakeups,
   timeouts, handoff, writer preference. Ban: open.
3. Hooks do not wait: audit open.
4. Hand-rolled spins: audit open.
5. Carried files via the carry manifest; fork pattern: open.
6. Gates + Eden (close over many cycles, Named-library macro load, allocations 0, no Unmapped): open.

## Operator order (2026-11-02): make the hang unlikely by design
1. A game thread never takes a lock another thread can hold across real work. Per-frame shared state goes through a wait-free
   snapshot (`hos_lock::Published<T: Copy>`: two slots, an atomic index, a reader count per slot; a read is one count-in, one
   check, one copy; fixed size, no heap). Game-thread to worker traffic goes through bounded lock-free SPSC queues with a counted
   drop when full. Nothing in a hook waits.
2. The kernel-waiting locks (`hos_lock::Mutex`/`RwLock`) remain for rare contention between non-game threads, and for game-thread-only
   state.
3. Audit table below; enforcement: `tests/hook_locks.rs` follows the runtime call graph from every plugin-thread entry (menu worker, writer threads,
   loaders, the publishing path) and fails if a reached function takes a lock a game-thread hook takes (exceptions listed with their reason).
   Lock sites: the static followed by `.lock/.read/.write/.try_*(`, or handed by reference to a call (the carried helpers), matched in its
   file, by `module::NAME`, or through listed re-exports. Edges: path calls, plus method calls whose name is unique among runtime functions;
   ambiguous method names are listed (`AMBIGUOUS`), not dropped. Limits stated in the test: `dyn`/trait calls, stored closures, function
   pointers in data, unlisted re-exports. A fixture (`tests/fixtures/hook_locks/`) seeds one violation per lock-site form and an ambiguous
   name; `the_scan_catches_a_seeded_violation` requires all three to be reported. `report_locks_reached_from_plugin_threads` prints every
   static lock (ours and carried package-6 code) a plugin thread reaches: the raw material for the rows below.
4. Eden: many menu-close cycles, named-macro load, frame-time evidence that the game thread never waits.

### Slice 1 (the hang fix; first commit)
- `crates/hos-lock` (MIT, only dependency `lock_api`): `Mutex`/`RwLock` on the kernel arbiter, `Published<T>`, README, host tests.
- `published::publish` (boot, menu worker, diag reload) computes every feature's `Copy` settings into `published::Live` and publishes it
  wait-free. It takes no lock a hook takes. Each feature adopts a new value at its own next hook under its own lock, which then only
  game threads take: CPU (`cpu::with`, plus the re-roll bits through an atomic), save states (`states::with`; the cancel on
  disable goes through an atomic, and the flags are set at publish), chords (`on_poll`; the menu shortcuts come from the existing
  atomic masks, so `commit_shortcut`/`load_ini` no longer take `STATE`), recorder and input display (their settings are read straight
  from `Live`, and their settings mutexes are removed), the typed menu (`compat::menu`: rows from `Live`, game-side edits in a
  game-thread-only `EDITED` slot valid until the next publish, saved defaults/kept keys in `KEPT`, assigned by the worker with no
  work under the lock), and P1 CPU control (already an atomic in `cpu_pilot`).
- Spins (item 4): no unbounded spin. `diag_log::Locked::with` makes at most 4096 tries (a yield every 256), then drops the line and
  counts it (`ring_dropped`). `native_unlink_probe` makes at most 4096 tries on a one-shot anomaly path. `capture_toggle`'s spin is in
  a host test only.

### Audit table (hook / callback on a game thread -> lock or wait -> treatment). Slice 1 rows done; the rest are slice 2
| Game-thread entry | Lock / wait | Other thread that took it | Treatment |
| --- | --- | --- | --- |
| P1 poll: `chords::on_poll` | `chords::STATE` | menu worker (publish, commit_shortcut), boot (load_ini) | slice 1: worker no longer takes it (Live + atomic masks); game-thread-only |
| CPU hooks: `cpu::with` | `cpu::CPU` | menu worker (publish) | slice 1: adoption in `with`; worker no longer takes it. Remaining callers of `with` off the game thread: slice 2 |
| State hooks: `states::with` | `states::STATE` | menu worker (publish); `tr-state-writer` (clones the save file under the lock) | slice 1: publish removed. slice 2 (lead decision (a), 2026-11-02): the Save hook hands a copy of each captured record (heap-free: a captured state carries no Mii value) to the writer through an 8-slot mailbox (compare-exchange claimed slots, a sequence number keeps capture order, a full mailbox drops and counts); the writer keeps its own `SaveFile` (seeded at boot) and never takes `STATE`. Persisted markers are the captured `NoAction` (a load's transient markers no longer reach the file). |
| Recorder hooks: `recorder::settings()` and 4 sites | `recorder::SETTINGS` | menu worker (publish) | slice 1: removed (reads `Live`) |
| P1 poll / HUD: `input_display::{on_p1_poll, hud_tick}` | `input_display::SETTINGS` | menu worker (publish) | slice 1: removed (reads `Live`) |
| Carried readers: `read(&MENU)`, `setting(&MENU)` | `compat::menu::CURRENT` (RwLock) | menu worker (apply_snapshot, apply_kept) | slice 1: rows from `Live` (wait-free); edits in `EDITED` (game threads only) |
| Carried readers: `DEFAULTS_MENU`, `stage_preferences`, `mii_preferences` | `compat::menu::KEPT` (RwLock) | menu worker (`apply_kept`, assignment only) | slice 1: no work under the write lock. Slice 2: `Published` for Clone values or a game-side copy |
| `named_macros` hooks | `MOTION`, `LEGACY` and other RwLocks | publishing thread (`prepare_buffers`) | slice 2: the reservation (allocation) runs before `MOTION` is taken; the write lock only swaps the buffers (pkg4 manifest edit, ledger regenerated) |
| `QUICK_MENU_APP` (carried Mii code) | Mutex | menu worker (`set_open` on open/close, dropping the catalog boxes under the lock) | slice 2: the worker only bumps `OPEN_SEQ` (atomic); every taker catches up in `App::follow_open` under the lock it already holds; the worker no longer takes it |
| HUD tick: `notices::hud_tick` | `notices::QUEUE` | menu worker / writer thread (`post` on refusal or write failure) | slice 2: `post` goes into an 8-slot lock-free mailbox (CAS-claimed slots of `Copy` entries; full: dropped and counted), drained by the HUD tick under `QUEUE` |
| UI tick | `ui::FRAME` | menu worker (frame build) | slice 2: the worker builds into its own `SCRATCH` and holds `FRAME` for one copy; the tick's `try_lock` skips a draw only during that copy (allowed: the holder does no other work) |
| others (`input_delay::ADAPTER`, `ui::input::ADAPTER`, thumbs, texture, std::sync users in carried package-6 code) | | | slice 2 |
## Playbook
Playbook followed: ae87443. Read completely: README.md, docs/{CHECKLIST,WORKFLOW,EDEN,HOOK-SHARING,MACROS,TILE-CURSOR,GIFS}.md,
tools/PROVENANCE.md, tools/check/compliance.py, tools/build/nro_size.py, tools/eden-solo/{solo.ps1,gv.py}.

### Checklist (docs/CHECKLIST.md): M 33: pass 12, open 12, N/A 9. C 9: N/A 9
| Item | Tag | Result | Evidence |
| --- | --- | --- | --- |
| cdylib only | M | pass | Cargo.toml; nro_size.py on the release NRO: open |
| release profile, skyline =0.6.0, toolchain 1.98.1 | M | pass | Cargo.toml, rust-toolchain.toml |
| model pure + host tests, runtime cfg(switch) | M | pass | `src/model/lock_word.rs` tests; `runtime/sync.rs` under runtime |
| gates | M | open | |
| no heap in hooks | M | open | the locks allocate nothing (const words); Eden install line + R5 delta open |
| fixed buffers, writer thread, stack paths | M | N/A | no file I/O added |
| root cause, one abstraction | M | open | one abstraction: every plugin lock goes through `runtime::sync` (not per-site fixes); Eden proof open |
| version guard | M | N/A | no hook added (unchanged) |
| 64-byte guards | M | N/A | no site added |
| card scan | M | N/A | no site added |
| hook sharing | C | N/A | no site added |
| hook after prologue | M | N/A | no hook added |
| no diag strings in shipped NRO | M | open | compliance.py --nro |
| shipped writes only install line | M | open | |
| logging plugin data files | C | N/A | not a logging plugin |
| online x3 | C | N/A | lock change: no simulated state, no RNG; Training/menu Eden rows |
| Eden claim / one per host / restored / console scan / outputs / waits / retry | M x7 | open | next Eden boot (Ally lent rig, ui-screens tools) |
| profile copy hard-linked | M | N/A | Windows lent rig with verified set-aside/restore (lead's Ally rule) |
| one target dir per copy, df | M | pass | cargo-targets/trainingrooms-sync(-sw); df 173G |
| shortest ruleset | M | N/A | Training mode |
| memory/time guard | M | pass | cgs.ps1 / sws.ps1 (systemd-run MemoryMax=6G timeout 1800) |
| README + GIF | M | N/A for this stream | repo-level product docs, not changed by a lock module |
| dated evidence doc | M | pass | this file; Eden rows go to `eden/EDEN-SYNC.md` |
| numbers from tested scripts | C | N/A | no numbers cited beyond counts read off logs |
| no operator quotes | M | pass | |
| handoff current | M | pass | this file |
| release/tag/rebuild/MTP | C x3 | N/A | no order |
| user connects / nothing pushed | M x2 | pass | |
| Forgejo, messages from a file | M | pass | |
| model/effort gate + watcher | M | pass | watch-opus.ps1 |
| spawned workers read the playbook | M | N/A | none spawned |

### Own tools
Same set as the ui-screens stream (see `IMPLEMENTATION-PROGRESS-ui-screens.md` on `stream/ui-screens`), with `cgs.ps1`/`sws.ps1` as the
sync worktree's guarded cargo wrappers. Playbook `nro_size.py` and `compliance.py` are used as they are.
