# CURRENT follow-up
UI lock fix aa7e1da1 is integrated as01807de5. This follow-up adds DecisionSnapshot.guard_event:Option<bool>, None for the unverified/unbound native stream, plus retained-delivery distinction test againstSome(false)/Some(true). No native press inferred.
Prior fc266f47 Surface gate was stopped during compilation before tests completed to incorporate the required UI/availability corrections. CanonicalStop18:46:02 releasedclaim; controllerinactive. No PASS claimed. Next canonicalcompiledtests/cross-needs/lock_order/hook_locks review remains required.
FrozenV9main0938ec33/XPSgate continuesindependently. UI-owned native accepted-status mapping/event provider andverifiedHitlagResolver stillpending; no productrun authorized.
Historical worker notes follow.
# UI/core integration checkpoint — 2026-11-16

Implemented; compiled tests and closure regeneration pending primary-owned gates. This is not build or runtime acceptance.

Final worker checkpoint: source frozen at `fc266f47`; primary started canonical Surface unit `rc15-tech-seam-gates-fc266f47`, lane `rc15-tech-seam`, output `gates-surface/` beside this file. Primary owns its logs and integration fixes. Eight static checks passed in `SOURCE-CHECK.json`. The bounded compliance scan returned no report; `COMPLIANCE.md` records its exact output and all scoped dispositions. Watcher PID 1165452 was reaped with no changed-model marker. These final evidence files/handoff update are deliberately left uncommitted to preserve the frozen source commit and primary's gate identity.

- Worktree: `/home/intpa/tr-fixes-20261113/rc15-tech-seam`, branch `rc15/tech-seam`, base `8b43bdf2`.
- Playbook: full read, pull already current at `f0dbc0b67a9de08560276864dae3b0137338daa0`; routing read; live worker `gpt-6.1-sol` / `medium` verified in exact thread `01a1e59f-d6c0-7a12-ac5c-266258288a4c`.
- Preserved primary's existing CPU call and all-sites edits. Completed boot comparison, diagnostic poison recognition/application, usable lookup, grant and report path for the 17 reviewed surface rows. Vec-backed grant storage needs no capacity adjustment; surface rows write no code.
- Added real first-condition `tech::frame` readiness boundary. Restored `state_work_set_int` in TechPractice CROSS_NEEDS at primary direction. Exact regenerated UI/core closure remains pending compiled `print_cross_needs`; no audit relaxation or guessed closure accepted.
- Expanded isolated poison regression to every surface row, retaining CpuCore/CpuCommon/Hitbox survival, grant refusal and install-line attribution. Added source/graph contract for one UI frame dispatch immediately after adapter, independent of Hitbox admission, and immediate return when Tech is refused.
- `git diff --check` passed before commit. Rust tests not executed here: primary explicitly owns compiler admission and all Cargo gates. Main worktree, Eden, build slots and ledger untouched. No pushes, tags, releases or card writes.

## Blocking UI hook locks (reported to primary)

Unchanged UI-owned `src/runtime/features/tech.rs`: `frame` line 311 reaches `VIEW.lock()` at 353, 417, 429, 463, 476, 481, 550; `frame -> still_tumbling` line 250 reaches `IDS.set` at 266. Other exposed paths are `poll` 153 -> VIEW 160, `look` 189 -> VIEW 194 and `menu_opened` 86 -> VIEW 87. `tech_surfaces::guarded` uses `VERIFIED.get_or_init`, reached from `NativeSource::new` and `blast_bounds` on the frame path. No exemptions were added. UI owner must replace blocking hook operations before acceptance.

The resolver and native outcome/grade mapping remain unbound as designed. DecisionObserver snapshot delivery stays unchanged; native guard-press exact-once evidence remains unresolved. No requested status is treated as observed native success.

## Next primary checks

Run on the admitted canonical build host from this source snapshot, with isolated target directory and the normal memory/time/priority guard:

1. `cargo test --test capabilities print_cross_needs -- --ignored --nocapture`, review exact delta and regenerate only factual CROSS_NEEDS.
2. `cargo test --test capabilities --test hook_locks --test lock_order` (UI hook-lock failures expected until UI owner fixes them).
3. Required full `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo build --no-default-features`, release and diagnostic builds on the integrated final head. None claimed here.

Compliance report beside this file records machine failures and scoped dispositions. Static source checks cannot prove native surface lifetimes, threading safety, gameplay behavior or unbound outcome calibration.
