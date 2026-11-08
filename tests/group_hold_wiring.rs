//! tr-p23-hot-measure-1 (release `031f1eae`), g03 frame 85: "Loaded 2/5 (3 failed)" after the CPU cycle left its Mii
//! stretch. The window `[34,0,1]` was built and its next group `[2,3,4]` was refused at its prefetch start for process
//! headroom (≤ 478 MiB against a need of 530.5 MiB; `memory::tests::frame85_group_refused_by_headroom_then_admitted`).
//! The adapter used to fail the group there, which also stops every retry for the scene ("never retried"). The capacity
//! refusals now hold the group (`GroupState::Held`) and the lifecycle retries it. This pins that wiring.

fn group_lifecycle() -> &'static str {
    let src = include_str!("../src/runtime/roster/native_body_bank.rs");
    let start = src.find("unsafe fn group_lifecycle(").expect("group_lifecycle");
    &src[start..start + src[start..].find("\n}\n").expect("end of group_lifecycle")]
}

#[test]
fn capacity_refusals_hold_the_group_and_never_fail_it() {
    let body = group_lifecycle();
    assert!(body.contains("hold_group(role, group, Hold::Pool)"), "the pool refusal holds the group");
    assert!(body.contains("hold_group(role, group, Hold::Memory)"), "the memory refusal holds the group");
    assert!(!body.contains("fail_group(role, group, \"fighter pool reserve\")"), "the pool refusal no longer fails");
    assert!(!body.contains("fail_group(role, group, \"process headroom\")"), "the memory refusal no longer fails");
    let refuse = body.find("Admission::Refuse =>").expect("the refusal arm");
    let arm = &body[refuse..refuse + body[refuse..].find("return;").expect("the arm returns")];
    assert!(!arm.contains("fail_group"), "{arm}");
}

#[test]
fn held_groups_are_started_again_and_counted() {
    let body = group_lifecycle();
    assert!(body.contains("next_start(bank.groups.iter()"), "the start picks a planned group, then a held one");
    assert!(body.contains("bank.held_turn)"), "held retries rotate");
    let src = include_str!("../src/runtime/roster/native_body_bank.rs");
    // rc.6 mid-session cycle change: a leaving group keeps it running too.
    assert!(src.contains("bank.groups.iter().any(|g| g.state.startable() || g.leaving)"), "a held group keeps the lifecycle running");
    let progress = &src[src.find("pub(crate) fn progress()").expect("progress")..];
    let progress = &progress[..progress.find("\n}\n").expect("end of progress")];
    assert!(progress.contains("result.held = result.held.saturating_add(counts.held)"), "{progress}");
    assert!(progress.contains("merge_hold(result.hold, counts.hold)"), "{progress}");
    let hold = &src[src.find("fn hold_group(").expect("hold_group")..];
    let hold = &hold[..hold.find("\n}\n").expect("end of hold_group")];
    assert!(hold.contains("GroupState::Held(hold)") && !hold.contains("GroupState::Failed"), "{hold}");
}
