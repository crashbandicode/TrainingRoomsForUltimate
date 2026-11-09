//! P-HOT helper entries by purpose (`artifacts/phase23/LANE-HOT-SLOTS-REPORT.md` §3.2, §3.5.1, §5.1). The value rules
//! are host-tested in `src/model/preload/hat_slots.rs`; this pins the runtime's use of them: the helper plan is
//! budgeted (Ready Hats apart from the fighters), every helper slot's request goes through the purpose view, the
//! borrow record is published at the hat's admission on entry 3 and ends with its scene, and the group planner no
//! longer charges hats against Ready Fighters.

fn body<'a>(src: &'a str, start: &str) -> &'a str {
    let at = src.find(start).unwrap_or_else(|| panic!("{start}"));
    &src[at..at + src[at..].find("\n}\n").expect("end of the function")]
}

#[test]
fn the_helper_plan_is_budgeted_with_ready_hats_apart_from_the_fighters() {
    let src = include_str!("../src/runtime/roster/cycle_preload.rs");
    assert!(src.contains("hat_slots::ready_hats(menu.cycle_preload_ready_hats).min(settings.slots)"));
    assert!(src.contains("hat_slots::budgeted(order, fighter, hat_budget, helper_slots)"));
    assert!(src.contains("set_preload_plan(epoch, desired, plan.map(|entry| entry.map(|(_, purpose)| purpose)))"));
    let bank = include_str!("../src/runtime/roster/native_body_bank.rs");
    let groups = body(bank, "fn plan_groups(");
    assert!(groups.contains("GroupBudget { ready_fighters: usize::from(settings.ready_fighters), pool_free, special }"),
        "hats are not charged against Ready Fighters");
}

#[test]
fn every_helper_request_goes_through_the_purpose_view_and_entry_3_publishes_its_borrow() {
    let src = include_str!("../src/runtime/roster/character_sources.rs");
    let select = body(src, "fn selected_request(");
    let view = select.find("let targets=purpose_view(state,slot as usize,targets);").expect("the purpose view");
    let schedulable = select.find("cycle_preload_policy::schedulable(targets").expect("the exact-Mii view");
    assert!(view < schedulable, "the purpose view comes first:\n{select}");
    let purpose = body(src, "fn purpose_view(");
    assert!(purpose.contains("borrow_admitted(unsafe{native_source::training_cpu_quantity()}"), "{purpose}");
    assert!(purpose.contains("PAUSE_ACTIVE.load(Ordering::Acquire)"), "{purpose}");
    assert!(purpose.contains("state.book.can_replace(BORROW_ENTRY)==Ok(true)"), "{purpose}");
    assert!(purpose.contains("if slot==BORROW_ENTRY && record.is_some() && borrowed.is_none() {return [None;"),
        "a borrowed entry 3 is never replaced by P-HOT:\n{purpose}");
    let commit = src.find("    publish_borrow(&state, slot, wanted, speculative);").expect("the publication");
    let source = src[..commit].rfind("state.sources[index] = Some(Source {").expect("the source commit");
    assert!(source < commit, "the borrow is published right after the load is admitted");
    let plan = body(src, "pub(crate) unsafe fn set_preload_plan(");
    assert!(plan.contains("state.preload_purpose=purposes;") && plan.contains("ENTRY3_BORROW.lock().departed(state.book.epoch());"),
        "{plan}");
    assert!(src.contains("pub(crate) static ENTRY3_BORROW:"), "F9 reads and releases the record");
}

/// rc6-hot-xps1 hs06 (2026-11-08, XPS, P1 Kirby, Ready Fighters 10, groups on): no hat loaded in 40 s. The plan arrives
/// budgeted (`hat_slots::budgeted`), but `set_preload_plan` cut it again to the fighter remainder (`helper_budget`, 0
/// here), and creation was capped by the same remainder and waited for entries 2 and 3 to hold targets they may not
/// load. The hats keep their own budget through planning, creation and the borrow.
#[test]
fn hats_are_planned_and_created_apart_from_the_fighter_remainder() {
    let src = include_str!("../src/runtime/roster/character_sources.rs");
    let limit = body(src, "fn preload_limit(");
    assert!(!limit.contains("helper_budget"), "the plan is already budgeted:\n{limit}");
    let intent = body(src, "fn creation_intent(");
    let speculative = &intent[intent.find("if state.wants.iter().any(Option::is_some) {return None;}").expect("speculative")..];
    assert!(speculative.contains(".take(created_allowance(state,backend))"), "{speculative}");
    // Primary 2026-11-09 09:47Z: the view is read once, so a parked seat is judged filled on the same view
    // (`preload_seat_wiring` `a_parked_seat_on_its_member_does_not_hold_back_creation`).
    assert!(speculative.contains("let view=helper_view(state,slot);")
        && speculative.contains("hat_slots::holds_back_creation(view,filled)"), "{speculative}");
    let allowance = body(src, "fn created_allowance(");
    assert!(allowance.contains("hat_slots::created_allowance(fighters,hats,helpers.len(),created,backend==Backend::Training)"),
        "{allowance}");
    assert!(allowance.contains("state.creation_hold.filter("), "a refused creation ends the allowance:\n{allowance}");
    let purpose = body(src, "fn purpose_view(");
    assert!(purpose.contains("created_busy(extras.iter().map(|s|reservation_for(state,*s as u32).is_some()&&in_plan(*s)),"),
        "the borrow waits for the allowed created entries:\n{purpose}");
}
