//! Primary inbox 2026-11-09 06:16Z (hot rc6-hot-xps26 `x6-align`; Eden rc6-bg-pf2, rc6-bg-pf4): after a CPU switch the
//! two parked F9 seats' members sat swapped (entry 2 held entry 3's member and the other way round), each entry found
//! its target on the other one and stayed idle, and the notice read "Loading 4/6" with the preload grace unasked until a
//! P1 switch, 160 s later. The seat rule is host-tested in `src/model/preload/cycle_preload_policy.rs`
//! (`seat_missing`); this pins the runtime: a helper entry `purpose_view` gave its seat's member alone (an armed Reload,
//! else the parked member, as `purpose_view` orders them) is judged on its own entry.

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

const SOURCES: &str = include_str!("../src/runtime/roster/character_sources.rs");

#[test]
fn a_parked_seat_is_judged_on_its_own_entry() {
    let select = body(SOURCES, "fn selected_request(");
    ordered(select, &[
        "let targets=purpose_view(state,slot as usize,targets);",
        "let seat=parked_seat_view(state,slot,&targets);",
        "if seat {",
        "crate::engine::preload::cycle_preload_policy::seat_missing(targets,sources,slot as usize,replaceable,demand)",
        "} else {",
        "crate::engine::preload::cycle_preload_policy::missing(targets,sources,slot as usize,replaceable,demand)",
    ]);
    let view = body(SOURCES, "fn parked_seat_view(");
    ordered(view, &[
        "if !matches!(slot,2|3) || !unsafe{crate::runtime::compat::common::is_native_training_mode()} {return false;}",
        "let (Some(only),None)=(wants.next(),wants.next()) else {return false;};",
        "crate::runtime::roster::f9_gate::override_seat(slot).or_else(",
        "if quantity.is_some_and(|count|slot<=count) {return None;}",
        "crate::model::parked_entries::parked_seat(slot,quantity,borrowed,crate::runtime::roster::f9_gate::seat_member(slot))",
        "member==Some((only.kind,only.configuration))",
    ]);
    // The notice counts a parked seat by the same parked member (`seat_states`).
    let seats = body(SOURCES, "pub(crate) unsafe fn seat_states(");
    ordered(seats, &["if quantity.is_some_and(|count|slot<=count) {return None;}",
        "let member=crate::runtime::roster::f9_gate::override_seat(slot).or_else(||crate::model::parked_entries::parked_seat(",
        "slot,quantity,borrowed,crate::runtime::roster::f9_gate::seat_member(slot)))?;"]);
}

/// Primary 2026-11-09 09:47Z (Eden final-xps-hot hs06): a parked seat's view admits its own seat member, which a
/// hats-only plan never lists, so the creation walk counted it unfilled and held the created entries 7/6/5/4 back for
/// good. The rule is host-tested (`hat_slots` `parked_seats_on_their_members_let_a_hats_only_plan_create_its_entries`);
/// here `creation_intent` asks it for every entry before the next created one, with the seat judged by
/// `parked_seat_view` on the same view that `holds_back_creation` reads.
#[test]
fn a_parked_seat_on_its_member_does_not_hold_back_creation() {
    let creation = body(SOURCES, "fn creation_intent(");
    ordered(creation, &[
        "for slot in candidates(backend).iter().copied().take(backend.helpers().len()+position) {",
        "let view=helper_view(state,slot);",
        "crate::engine::preload::hat_slots::creation_filled(",
        "state.book.is_current(source.token.epoch()) && state.book.phase(slot as usize)==Ok(Phase::Ready),",
        "state.preload.iter().flatten().any(|want|want.kind==source.kind && want.configuration==source.configuration),",
        "state.book.can_replace(slot as usize)==Ok(true),",
        "parked_seat_view(state,slot,&view)",
        "&& view.iter().flatten().any(|want|want.kind==source.kind && want.configuration==source.configuration)));",
        "if crate::engine::preload::hat_slots::holds_back_creation(view,filled) {return None;}",
    ]);
}
