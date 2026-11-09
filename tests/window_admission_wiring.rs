//! rc.6 window admission (primary 2026-11-09 13:28Z; Eden `final-e39-xps-hot` hs08b on `e39479df`: the CPU's switch
//! planned the window `[42, 44, 1c]` at 27.6 MB of headroom, `prepare_resources` acquired both supplements and the game's
//! page commit aborted in the build). Design: `artifacts/phase23/rc6-window-admission/DESIGN-window-admission.md`.
//! The rule is `body_bank::memory::{window_extras, window_waiting, created_admission}` (host-tested there); this pins
//! where the adapter applies it.

fn body<'a>(src: &'a str, start: &str) -> &'a str {
    let at = src.find(start).unwrap_or_else(|| panic!("{start}"));
    &src[at..at + src[at..].find("\n}\n").expect("end of the function")]
}

const BANK: &str = include_str!("../src/runtime/roster/native_body_bank.rs");
const SOURCES: &str = include_str!("../src/runtime/roster/character_sources.rs");
const PROCESS: &str = include_str!("../src/runtime/roster/process_memory.rs");

/// The window's supplements are admitted where the switch plans the window (`proposed`, the request's preflight), and
/// the planner runs again with the admitted count; `prepare_resources` keeps acquiring exactly what was planned.
#[test]
fn the_window_is_admitted_where_the_switch_plans_it() {
    let proposed = body(BANK, "pub(super) fn proposed(");
    let admission = proposed.find("super::process_memory::admit_window(").expect("the window's admission");
    let first = proposed.find("members_for(&settings, target, &lane[..count], budget)").expect("the body-budget plan");
    assert!(first < admission, "the admission judges the members the body budget chose:\n{proposed}");
    assert!(proposed[admission..].contains("members_for(&settings, target, &lane[..count], admitted_budget)"),
        "a refused supplement re-plans with the admitted count:\n{proposed}");
    assert!(proposed.contains("WINDOW_WAITING[role].store(memory::window_waiting("), "what the window waits for:\n{proposed}");
    let process = body(PROCESS, "pub(crate) fn admit_window(");
    assert!(process.contains("memory::window_extras(headroom, outstanding(), supplements)"), "{process}");
    // Nothing is refused in the hook: the descriptor already names the window there.
    let prepare = body(BANK, "unsafe fn prepare_resources(");
    assert!(!prepare.contains("process_memory"), "{prepare}");
    // The entry's end clears the role's wait.
    let destroy = body(BANK, "pub(super) fn on_destroy(");
    assert!(destroy.contains("WINDOW_WAITING[role].store(0, Ordering::Release);"), "{destroy}");
}

/// A created entry's creation and every speculative load on it leave what the cycle waits for: the supplements a window
/// was refused and the groups held for memory. Other helpers keep today's admission.
#[test]
fn created_entries_are_admitted_beside_what_the_cycle_waits_for() {
    let create = body(SOURCES, "unsafe fn create_capacity(");
    assert!(create.contains("super::process_memory::admit_created(cost)"), "{create}");
    assert!(!create.contains("super::process_memory::admit(cost)"), "{create}");
    let service = body(SOURCES, "pub unsafe fn service(");
    assert!(service.contains(
        "if is_capacity { super::process_memory::admit_created(memory_cost) } else { super::process_memory::admit(memory_cost) }"),
        "{service}");
    let created = body(PROCESS, "pub(crate) fn admit_created(");
    assert!(created.contains("memory::created_admission(headroom, outstanding(), cycle_waiting(), cost)"), "{created}");
    let waiting = body(PROCESS, "pub(crate) fn cycle_waiting(");
    assert!(waiting.contains("super::body_bank::cycle_waiting()"), "{waiting}");
    let bank = body(BANK, "pub(super) fn cycle_waiting(");
    assert!(bank.contains("GroupState::Held(Hold::Memory)") && bank.contains("WINDOW_WAITING"), "{bank}");
}
