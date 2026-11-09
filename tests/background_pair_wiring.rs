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
        // Cross cell (b), the pair group beside a live native window, needs the X4 hook (primary 2026-11-09 00:52Z).
        assert!(planner.contains("BACKGROUND_PAIR_TAIL.load(Ordering::Acquire), BACKGROUND_X4.load(Ordering::Acquire))"),
            "{planner}");
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

/// SPECIAL-GROUPS §0 X4 (primary 2026-11-09 00:52Z, cross cell (b): a pair group built beside a live native window).
/// The construction picks a Popo or Nana body's info wrapper from the live words (`0x60d1f0-0x60d26c`). At `0x60d270`
/// (x9 the chosen wrapper, x8 the manager, x23 the owner, w21 the kind, x19 the body) a background body gets the wrapper
/// its building group's words choose (`body_bank::info_wrapper`). Nana's `+0x100` is used only while the entry holds
/// its info (`+0x108`). The hook counts with atomics only (no allocation on the worker) and is installed with the
/// background builds behind its four 13.0.5 words and its seam row.
#[test]
fn a_background_ice_climbers_body_takes_its_groups_info_wrapper() {
    let hook = body(BANK, "unsafe fn background_info(");
    ordered(hook, &["let kind = ctx.registers[21].w();",
        "if !matches!(kind, POPO | NANA) || !background_body(ctx.registers[19].x() as usize) { return; }",
        "let owner = ctx.registers[23].x() as u32;",
        "let Some((count, cells)) = background_cells(owner) else { return; };",
        "let offset = info_wrapper(kind, count, cells);",
        "let Some(entry) = fields::read::<usize>(ctx.registers[8].x() as usize, 0x20 + 8 * owner as usize) else { return; };",
        "if offset == 0x100 && !fields::read::<usize>(entry, 0x108).is_some_and(|info| info != 0 && info % 8 == 0) {",
        "BACKGROUND_INFO_KEPT.fetch_add(1, Ordering::Relaxed);",
        "ctx.registers[9].set_x((entry + offset) as u64);"]);
    assert!(!hook.contains("trace!"), "no logging inside the worker hook");
    let start = BANK.find("BACKGROUND_OWNERS.fetch_or(owner_bit(owner), Ordering::AcqRel);").expect("start");
    let cells = BANK.find("if let Some(cells) = BACKGROUND_CELLS.get(owner as usize) { cells.store(pack_cells(b.members), Ordering::Release); }")
        .expect("cells");
    assert!(cells < start, "the group's words are stored before the owner bit");
    let install = body(BANK, "pub(super) unsafe fn install(");
    ordered(install, &["fingerprint(base, BACKGROUND_INFO, &[0xf9400529, 0x39425129, 0x35001da9, 0xf945bd08], \"GroupSwap\")",
        "gate(SEAM_GROUP, BACKGROUND_INFO_SEAMS, async_ok && identity_view && info_stock)",
        "skyline::install_hooks!(background_info);", "BACKGROUND_X4.store(true, Ordering::Release);"]);
    assert!(ROWS.contains("label: \"background_build.info_wrapper\"") && ROWS.contains("offset: 0x60d270"), "seam row");
}

const PLAYER: &str = include_str!("../src/runtime/mii/mii_ordinary_player.rs");
const CPU: &str = include_str!("../src/runtime/mii/mii_ordinary_cpu.rs");

/// The return to an Ice Climbers original is warm (primary 2026-11-09 00:23Z; model rule
/// `an_ice_climbers_pair_original_is_a_bank_member_while_the_pair_bank_is_on`). Both Mii adapters' Original gates ask
/// `native_body_bank::original_member` with the staged request's form, and the planner's canonical-original exclusion
/// asks the same model rule with the slot-0 setting's form. Every other kind keeps `original_bankable(kind)`. The
/// player's `at_origin` already recognises any live `0x4b` as the Ice Climbers origin.
#[test]
fn the_ice_climbers_original_is_served_by_its_bank() {
    let member = body(BANK, "pub(crate) fn original_member(");
    ordered(member, &["if !settings.original_bank_enabled() { return false; }",
        "if kind != POPO && Members::native_group(kind).is_none() { return original_bankable(kind); }",
        "request_forms(word(0x14), [word(0x18), word(0x1c), word(0x20)], word(0x28))",
        ".is_some_and(|forms| original_form_bankable(kind, forms, pair_enabled(&settings), special_policy(&settings).native))"]);
    let excluded = body(BANK, "fn excluded_original(");
    assert!(excluded.contains(
        "original_form_bankable(*kind, super::requested_forms_at(menu, logical, *kind, None), pair_enabled(settings),"), "{excluded}");
    assert!(excluded.contains("special_policy(settings).native)))"), "{excluded}");
    for (name, src, kept) in [("player", PLAYER, 1), ("cpu", CPU, 0)] {
        assert!(src.contains("&& super::body_bank::original_member(origin.original_kind,&descriptor) => Some(Live::Original),"), "{name}");
        assert_eq!(src.matches("original_bankable(origin.original_kind)").count(), kept, "{name}: only the player's at_origin");
    }
}

/// Eden rc6-bg-ic2 (`ic-start-loaded`, "Loaded 9/10 · swap P1 to load 1 more"): `plan_budget` plans a role's window
/// targets through `body_bank::window_targets` with its live window and active member, the values `progress` reads,
/// so a live pair-lead window counts what it serves.
#[test]
fn the_planner_counts_a_live_pair_lead_window_as_it_serves() {
    let budget = body(BANK, "pub(crate) fn plan_budget(");
    ordered(budget, &["let live: [Option<(Members, u32)>; 2] = std::array::from_fn(|role| BANKS[role].lock().as_ref()",
        ".filter(|bank| bank.phase == Phase::Ready && !bank.departed)",
        ".map(|bank| (*bank.members, bank.observed.map_or(bank.members.kinds()[usize::from(bank.target)], |(_, kind)| kind))));",
        "let targets = crate::engine::swap::body_bank::window_targets(grouped, live[role], originals[role], next);",
        "PLANNED_TARGETS[role].store(targets.saturating_add(group_targets[role]), Ordering::Release);"]);
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
