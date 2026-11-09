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
const CAPACITY: &str = include_str!("../src/runtime/roster/native_source_capacity.rs");

/// Eden rc6-hot-xps16/xps17: releasing a detached group's contexts cleared the live fighter's pad-slot controller (`S[slot]`,
/// `pad controllers changed ... [(0, 11e5d63800, 0)]`), and native's pad code reads it without a null test (`0x631984`:
/// 16 unmapped reads in xps16). A slot that held the live window's selected controller before a release or a background
/// build and is 0 after it gets that controller back (xps17: `live restored=[0]`, Unmapped 0).
#[test]
fn a_release_puts_the_live_pad_controller_back() {
    let leave = body(BANK, "unsafe fn leave_group(");
    ordered(leave, &["let pads_before = pad_controllers(base);", "graph::execute(base, &plan, &keys, owner, \"left the cycle\"",
        "let pads = pad_changes(&pads_before, &pad_controllers(base));", "let restored = restore_live_pads(base, entry as usize, &pads_before);"]);
    let finish = body(BANK, "unsafe fn finish_background(");
    ordered(finish, &["let pads = pad_changes(&b.pads, &pad_controllers(base));", "let restored = restore_live_pads(base, entry as usize, &b.pads);"]);
    let restore = body(BANK, "unsafe fn restore_live_pads(");
    ordered(restore, &["let live = fields::child(entry, 0x4178).and_then(|ctx| fields::read::<usize>(ctx, graph::CONTEXT_CONTROLLER))",
        "if *was == live && fields::read::<usize>(state, slot * 8) == Some(0) {", "fields::write_unchecked(state, slot * 8, live);"]);
}

/// Operator decision 2026-11-08 23:42Z ("free under the pause menu"; Eden rc6-hot-xps16: native's body destroy holds the
/// game thread 75-134 ms per body, a 12-19 frame gap mid-play): a parked group (Built, left the cycle) is destroyed when
/// Training's pause opens, from the running state's exit-block hook. Not while a native install runs, never a group a
/// committed switch targets, with the headroom before and after. A switch to a parked group makes it the window: its
/// record (now the outgoing window) is no longer leaving. The notice counts a parked group as built.
#[test]
fn a_parked_group_is_freed_when_the_pause_opens() {
    let exit = body(CAPACITY, "unsafe fn pause_state_exit(");
    ordered(exit, &["super::sources::pause_opening();", "#[cfg(feature = \"native-body-bank-alpha\")]",
        "super::body_bank::pause_opening();"]);
    let pause = body(BANK, "pub(super) unsafe fn pause_opening()");
    ordered(pause, &[
        ".find(|(i, g)| g.leaving && leaves_at_pause(g.state) && bank.swap.is_none_or(|s| s.group != *i))",
        "if super::INSTALL.active() {",
        "let before = MemorySample::capture();",
        "leave_group(entry as *const u8, owner, role, group);",
        "let after = MemorySample::capture();",
        "trace!(\"[GroupSwap] pause frees owner={} group={} kinds={:x?} us={} headroom={:?}->{:?}",
    ]);
    let swap = body(BANK, "unsafe fn perform_swap(");
    ordered(swap, &["group.configuration = x_configuration;", "group.leaving = false;"]);
    // rc6-hot-xps18: a parked group still serves its wanted members warm, so the notice counts it as built.
    let progress = body(BANK, "pub(crate) fn progress(");
    assert!(!progress.contains("if g.leaving { GroupState::Released }"), "{progress}");
}

/// rc6-hot-xps19 `mb-align` (also xps16/xps17): with a Mii group live, the window the planner wants for the next lane
/// kind is the one the exchange left in a detached record. Its kinds count with the groups, not again as window targets
/// ("Loaded 7/8 · swap the CPU to load 1 more" with every member built).
#[test]
fn the_window_targets_skip_what_a_group_holds() {
    let budget = body(BANK, "pub(crate) fn plan_budget(");
    ordered(budget, &[
        "let next = wanted_banks[role].map_or(0, |(members, target)| {",
        "let held_by_group = |kind: u32| BANKS[role].lock().as_ref().is_some_and(|bank| !bank.departed",
        "&& bank.groups.iter().any(|g| g.state != GroupState::Released && g.members.index_of(kind).is_some()));",
        "lane_window_targets(&shown, target, originals[role], held_by_group)",
        "let targets = crate::engine::swap::body_bank::window_targets(grouped, live[role], originals[role], next);",
        "PLANNED_TARGETS[role].store(targets.saturating_add(group_targets[role]), Ordering::Release);",
    ]);
}

/// Operator decision 2026-11-08 23:42Z, rc6-hot-xps18: a parked group counts only the members its cycle keeps, in the
/// planner's targets and in the notice (`mb-p1-change` showed kept Samus as "swap P1 to load"). New members that only the
/// plan without the parked groups places wait for the pause and show as held for memory (`mb-cpu-change`: Marth was
/// planned nowhere and the notice never named it); the pass after the pause plans them.
#[test]
fn new_members_wait_for_the_pause_and_parked_groups_count_their_kept_members() {
    let plan = body(BANK, "fn plan_groups(");
    ordered(plan, &[
        "let budget = GroupBudget { ready_fighters: usize::from(settings.ready_fighters), pool_free, special };",
        "let counts = count_group_members(bank.groups.iter().map(|g| (shown_members(g, &wanted[role]), g.state)));",
        "if group.leaving { parked_bodies += group.members.count(); } else { kept[role].push(*group.members); }",
        "if parked_bodies > 0 {",
        "let after = plan_extra_groups_with(policy, windows, [&kept[0], &kept[1]], [&upcoming[0], &upcoming[1]], originals,",
        "GroupBudget { pool_free: pool_free.saturating_add(parked_bodies), ..budget });",
        "waiting[role] = waiting_for_pause(&after[role], |kind| bank.groups.iter()",
        "WAITING_PAUSE[role].swap(waiting[role], Ordering::AcqRel)",
        "targets[role] = targets[role].saturating_add(waiting[role]);",
    ]);
    let shown = body(BANK, "fn shown_members(");
    assert!(shown.contains("if group.leaving { kept_members(*group.members, wanted) } else { group.members.count() as u8 }"), "{shown}");
    let progress = body(BANK, "pub(crate) fn progress(");
    ordered(progress, &["let wanted = [cycle_wanted(&menu, 0), cycle_wanted(&menu, 1)];",
        "for (role, state) in BANKS.iter().enumerate() {", "let waiting = WAITING_PAUSE[role].load(Ordering::Acquire);",
        "count_group_members(bank.groups.iter().map(|g| (shown_members(g, &wanted[role]),", ".with_waiting(waiting);"]);
    let scene = body(BANK, "pub(super) fn scene_changed(");
    assert!(scene.contains("for value in &WAITING_PAUSE { value.store(0, Ordering::Release); }"), "{scene}");
}

/// The sparse planner (run on the menu's close, `cycle_preload::on_frame`) reads each role's cycle as it is now: a
/// group holding a member that left is Released at once when nothing native exists for it, else marked leaving (a Built
/// one parked until the pause, operator decision 23:42Z); new members are planned beside it; a new group takes a Released
/// slot.
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
        "let planned = plan_extra_groups_with(",
        "if bank.groups.next_slot().is_none() { break; }",
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
        "} else if let Some(group) = bank.groups.iter().position(|g| g.leaving && g.state != GroupState::Prefetching",
        "&& !leaves_at_pause(g.state)) {",
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
    assert!(service.contains("bank.groups.iter().any(|g| g.state.startable() || (g.leaving && !leaves_at_pause(g.state)))"), "{service}");
    for (name, f) in [("candidate", body(BANK, "fn background_candidate(")), ("owed", body(BANK, "fn background_owed("))] {
        assert!(f.contains("leaving"), "{name}: a leaving group is never built in the background:\n{f}");
    }
    let forget = body(LEDGER, "pub(crate) fn forget_group(");
    assert!(forget.contains("if held == 0 { handle.batches.retain(|(g, _)| *g != group); }"), "{forget}");
}
