//! rc.6 mid-session cycle change (operator requirement 2026-11-08, `artifacts/phase23/INBOX-opus-rc6-hotslots.md` 19:21Z):
//! a P1 or CPU cycle change re-plans; groups whose members left the cycle are released (memory back) and the new members
//! load the way item 3 loads groups (prefetch, then a background build). The leave rule, the Released state and the slot
//! reuse are host-tested in `src/model/swap/body_bank.rs`; this pins the planner's marking, the entry tick's release and
//! the order of its teardown.

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
const LEDGER: &str = include_str!("../src/runtime/roster/r_handle.rs");

/// The sparse planner (run on the menu's close, `cycle_preload::on_frame`) reads each role's cycle as it is now: a
/// group holding a member that left is Released at once when nothing native exists for it, else marked leaving; a role
/// with a leaving group plans nothing new until it is gone; a new group takes a Released slot.
#[test]
fn the_planner_marks_groups_that_left_the_cycle_and_waits_for_their_release() {
    let wanted = body(BANK, "fn cycle_wanted(");
    assert!(wanted.contains("cycle_members(menu.player_roster, super::player_cycle_admission())"), "{wanted}");
    assert!(wanted.contains("cycle_members(menu.cpu_roster, super::cpu_cycle_admission())"), "{wanted}");
    assert!(wanted.contains("super::original_kind("), "{wanted}");
    let plan = body(BANK, "fn plan_groups(");
    ordered(plan, &[
        "let wanted = [cycle_wanted(menu, 0), cycle_wanted(menu, 1)];",
        "crate::engine::swap::body_bank::group_left_cycle(*group.members, &wanted[role])",
        "if matches!(before, GroupState::Planned | GroupState::Held(_) | GroupState::Failed) {",
        "group.state = GroupState::Released;",
        "group.leaving = true;",
        "leaving[role] = bank.groups.iter().any(|g| g.leaving);",
        "let planned = plan_extra_groups_with(",
        "if leaving[role] || bank.groups.next_slot().is_none() { break; }",
        "g.state != GroupState::Released && g.members.index_of(members.kinds()[0]).is_some()",
        "let slot = bank.groups.plan(*members, identity);",
    ]);
    let slots = body(BANK, "    fn next_slot(&self)");
    assert!(slots.contains("crate::engine::swap::body_bank::reuse_slot(self.iter().map(|g| g.state))"), "{slots}");
}

/// The entry tick releases a leaving group before any prefetch or background build starts, never while a prefetch is in
/// flight or a native install runs; the release is the teardown protocol for that one group, under its own header with
/// the window leaving the registry around it, then the ledger forgets its batch and the group is Released.
#[test]
fn the_entry_tick_releases_a_leaving_group_through_the_teardown_protocol() {
    let lifecycle = body(BANK, "unsafe fn group_lifecycle(");
    ordered(lifecycle, &[
        "if let Some((group, token, since)) = bank.prefetch {",
        "} else if let Some(group) = bank.groups.iter().position(|g| g.leaving && g.state != GroupState::Prefetching) {",
        "Step::Leave(group)",
        "let start = crate::engine::swap::body_bank::next_start(",
        "Step::Leave(group) => if !super::INSTALL.active() { leave_group(entry, owner, role, group) },",
    ]);
    let leave = body(BANK, "unsafe fn leave_group(");
    ordered(leave, &[
        ".filter(|g| g.leaving && g.state != GroupState::Prefetching)",
        "(GroupState::Built, Some(saved)) => {",
        "(state, _) if state.holds_keys() => {",
        "if alias == 0 || resource(base, owner) != Some(alias) { return; }",
        "copy_registry::teardown_begin(registry, window);",
        "release_group_keys(base, role, Some(group), None, &occupancy, owner, \"left the cycle\");",
        "copy_registry::teardown_group(registry, members);",
        "graph::execute(base, &plan, &keys, owner, \"left the cycle\", &mut release, &mut before_step)",
        "copy_registry::teardown_end(registry, window);",
        "let held = r_handle::forget_group(ledger_role(role), group);",
        "g.state = GroupState::Released;",
        "g.leaving = false;",
    ]);
    assert!(!leave.contains("bank.groups.clear()"), "only this group leaves:\n{leave}");
    let service = body(BANK, "unsafe fn service_locked(");
    assert!(service.contains("bank.groups.iter().any(|g| g.state.startable() || g.leaving)"), "{service}");
    for (name, f) in [("candidate", body(BANK, "fn background_candidate(")), ("owed", body(BANK, "fn background_owed("))] {
        assert!(f.contains("leaving"), "{name}: a leaving group is never built in the background:\n{f}");
    }
    let forget = body(LEDGER, "pub(crate) fn forget_group(");
    assert!(forget.contains("if held == 0 { handle.batches.retain(|(g, _)| *g != group); }"), "{forget}");
}
