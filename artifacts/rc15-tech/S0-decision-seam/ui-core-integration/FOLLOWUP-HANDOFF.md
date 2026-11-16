# Closure follow-up / initializer blocker — 2026-11-16

Implemented: only the three TechPractice CROSS_NEEDS omissions identified by canonical `CLOSURE-f84e0de4.json`: camera_limit_range, stage_id_call, utility_get_kind. Added `tech_stage_and_kind_dependencies_are_admission_requirements` to assert each is required for admission and poisoning refuses its grant. Exact graph audit remains unchanged. No speculative receipt hardening.

Worktree `/home/intpa/tr-fixes-20261113/rc15-tech-seam`, branch `rc15/tech-seam`, base `f84e0de44317e6d9f2e84e6e1dabe7734e65f821`. Main V9 worktree untouched. Compiler/build admission remains primary-owned; Rust tests added, not run by this worker. Full previous SSBU playbook read remains valid at `f0dbc0b`; current pull returned already up to date. Exact-thread model/effort reverified Sol medium; watcher rearmed for this task and reaped before completion.

## Init ordering is not implemented

Primary requires initialization after Lua constants ready and before P1 hooks, never in on_frame. Current source has no proven event satisfying both:

- `src/runtime/boot.rs:507`: `install_cpu` writes the CPU frame hook immediately.
- `src/runtime/features/cpu.rs:383`: `ensure_consts` resolves constants; if the table is absent it returns false and retries next frame (lines 394–396).
- `src/runtime/features/cpu.rs:478`: that readiness check is called from the already-installed `on_frame` callback.
- `src/runtime/game.rs:125`: the guarded lookup explicitly allows a missing/unbuilt table and returns None.
- `src/runtime/features/cpu_common.rs:167`: common-NRO load installs common hooks but supplies no proven constant-table-ready event.

Therefore a boot-time unconditional `tech::init` would assume the very readiness it must establish. Calling it after `ensure_consts` would run it from a hook, which is expressly forbidden. No guessed call, boot-blocking loop or unapproved deferred-install mechanism was added. Correct completion needs a proven non-hook readiness event and deferred hook installation, with tests covering event/order and failed/partial initialization.

## UI dependency review

`01807de5` already preserves UI `aa7e1da1`; base `39a51a24` is retained. Requested `5d03aa9b` moves tumble IDs into `tech::init` and changes VIEW accesses to try_lock, but its parent `9bc597ad` removes the current camera shim and switches to BTT-owned request/cursor APIs. That requires BTT `1d2a1d8b`, itself after `d1bda714` and `179b8e0d` (gameplay/layout/assets/catalog updates). UI `2f807ebb` also precedes it and adds stage/form/receipt epochs and fake-native surface tests. Latest `ba0b914b` additionally moves surface guard OnceLock initialization into the same initializer; `ee1d1696` fixes thread/platform traversal in the lock audit. `c7d2a2ba` adjusts BTT owner-mask precedence.

No UI or BTT source was copied or cherry-picked. The bounded, reviewable integration route is for the UI owner to rebase the needed nonblocking initializer changes onto this seam (or primary approve and integrate the full required BTT slice), then wire the proven deferred event. No broad ancestry was imported and no UI-owned implementation was edited to force a partial cherry-pick.

## Validation and next action

- Worker source validation and compliance receipts will be placed beside this handoff; no canonical build claim.
- Primary: review the independent closure fix, run exact `print_cross_needs` and capability poison regression on the admitted host, then the required test/clippy/no-default/release/diag gates on the final integrated source.
- Primary/UI owner: settle committed initializer dependency and readiness event before wiring. Native grade/guard press mapping remains unchanged and unbound.
- No push, tag, release, card write or Eden operation.
