//! rc.6 item 3, Ice Climbers pair (task `TASK-OPUS-rc6-bg-special.md`, design `rc6-item3-re/SPECIAL-GROUPS.md` §0 X1
//! and §2). The eligibility rule is host-tested in `src/model/swap/body_bank.rs`
//! (`a_background_build_takes_the_pair_group_and_builds_beside_a_live_pair_window`); this pins the runtime half:
//! - the construction tail's pair test (`0x60de68`, `w8` = the live `FE+0x18`) is redirected for every background body,
//!   so its tail takes the per-kind release `0x17e8610` and never the pair rule's whole-R reset beside a live
//!   `[Popo, Nana]` window;
//! - the hook is installed with the background builds, guarded by its 13.0.5 words and its seam row, and only then may
//!   a live pair window take part (`BACKGROUND_PAIR_TAIL` into the planner);
//! - a background build never writes `R+0x50` (no `set_pair_refs`, no reset-batch permit): its tails are per kind.

fn body<'a>(src: &'a str, start: &str) -> &'a str {
    let at = src.find(start).unwrap_or_else(|| panic!("{start}"));
    &src[at..at + src[at..].find("\n}\n").expect("end of the function")]
}

fn ordered(src: &str, steps: &[&str]) {
    let mut from = 0;
    for step in steps {
        let at = src[from..].find(step).unwrap_or_else(|| panic!("missing or out of order: {step}\n{src}"));
        from += at + step.len();
    }
}

const BANK: &str = include_str!("../src/runtime/roster/native_body_bank.rs");
const ROWS: &str = include_str!("../src/model/seams/native_seams/rows_a1.rs");

/// The tail hook: a background body (its owner's build in flight) sees no Popo in the live cell 0, so the pair test
/// falls through to the per-kind release.
#[test]
fn a_background_bodys_tail_never_takes_the_pair_rule() {
    let hook = body(BANK, "unsafe fn background_tail(");
    ordered(hook, &["if ctx.registers[8].w() == POPO && background_body(ctx.registers[19].x() as usize) {",
        "ctx.registers[8].set_x(u64::from(u32::MAX));"]);
    let at = BANK.find("unsafe fn background_tail(").unwrap();
    assert!(BANK[..at].ends_with("#[skyline::hook(offset = 0x60de68, inline)]\n"), "the hook sits on `cmp w8, #0x4b`");
    let owner = body(BANK, "unsafe fn background_body(");
    ordered(owner, &["BACKGROUND_OWNERS.load(Ordering::Acquire) != 0 && aligned(body as *const u8)",
        "fields::read::<u64>(body, 0x10).is_some_and(|owner| owner < 32",
        "BACKGROUND_OWNERS.load(Ordering::Acquire) & owner_bit(owner as u32) != 0)"]);
}

/// Installed with the background builds: the site's 13.0.5 words (`cmp w8, #0x4b; b.ne 0x60debc; ldr w8, [x9, #0x1c];
/// cmp w8, #0x4c`) and its seam row, then the flag the planner reads.
#[test]
fn the_tail_hook_is_guarded_and_gates_the_live_pair_window() {
    let install = body(BANK, "pub(super) unsafe fn install(");
    ordered(install, &[
        "BACKGROUND_ENABLED.store(async_ok && identity_view, Ordering::Release);",
        "fingerprint(base, BACKGROUND_TAIL, &[0x71012d1f, 0x54000281, 0xb9401d28, 0x7101311f], \"GroupSwap\")",
        "gate(SEAM_GROUP, BACKGROUND_TAIL_SEAMS,",
        "skyline::install_hooks!(background_tail);",
        "BACKGROUND_PAIR_TAIL.store(true, Ordering::Release);",
    ]);
    assert!(BANK.contains("const BACKGROUND_TAIL: usize = 0x60de68;"));
    assert!(BANK.contains("const BACKGROUND_TAIL_SEAMS: &[&str] = &[\"background_build.construct_tail\"];"));
    let row = &ROWS[ROWS.find("label: \"background_build.construct_tail\"").expect("the seam row")..];
    assert!(row.starts_with("label: \"background_build.construct_tail\", group: \"native_body_bank\", offset: 0x60de68,"));
    for planner in [body(BANK, "fn background_candidate("), body(BANK, "fn background_owed(")] {
        assert!(planner.contains("BACKGROUND_PAIR_TAIL.load(Ordering::Acquire)"), "{planner}");
    }
}

/// A session that starts on Ice Climbers (TASK-OPUS-rc6-bg-special 22:32Z; Eden rc6-bg-ic1 had no start-up window for
/// it): H1 extends the pair start (`entry_window::extendable`, the pair-lead window) only with the pair bank on and the
/// pair as the slot's requested form; Solo Popo stays native's one body. H2 (`prepare_resources`) already writes the
/// pair's `R+0x50` references for a pending pair-lead bank, H4 binds on count, actors and kinds.
#[test]
fn the_start_up_window_serves_an_ice_climbers_start_with_the_pair_bank() {
    let h1 = body(BANK, "unsafe fn scene_header(");
    ordered(h1, &["let Some(original) = extendable(facts) else { return; };",
        "let role = if facts.owner == 0 { super::Role::Player } else { super::Role::Cpu };",
        "if original == POPO && !(pair_enabled(&settings)",
        "&& super::requested_forms_at(&menu, role, POPO, Some(0)) == Forms::starting(POPO)) { return; }",
        "let Some(members) = entry_window(", "if !publish_pending(facts.owner, members, scene) { return; }"]);
    let h2 = body(BANK, "unsafe fn prepare_resources(");
    ordered(h2, &["let Some(bank) = state.as_ref().filter(|bank| bank.owner == owner && bank.phase != Phase::Retired) else { return; };",
        "if let Some(references) = crate::engine::swap::body_bank::pair_resource_references(members) {",
        "r_handle::set_pair_refs(lrole, resource as usize, r_handle::WINDOW, references as u8)"]);
    let h4 = body(BANK, "unsafe fn bind_pending(");
    assert!(h4.contains("read::<u32>(entry, 0x28) == bank.members.actor_count()"), "{h4}");
}

/// A background build writes no `R+0x50` and asks no reset-batch permit: its tails are per kind (beside an ordinary
/// window natively, beside a pair window through `background_tail`).
#[test]
fn a_background_build_writes_no_pair_references() {
    for f in ["unsafe fn start_background(", "unsafe fn queue_background_bodies(", "unsafe fn settle_background(",
        "unsafe fn finish_background(", "unsafe fn drain_background("] {
        let b = body(BANK, f);
        for forbidden in ["set_pair_refs", "reset_exclusive", "pair_resource_references", "0x50"] {
            assert!(!b.contains(forbidden), "{f} contains {forbidden}");
        }
    }
}
