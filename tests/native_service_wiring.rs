//! rc.7 FLAG 6 (integration 2026-11-11 01:20Z, Eden `final-surface-acc-d8679593-r2` `acc-rename`): the native service
//! `main+0x650010` runs on whatever stack its caller has, there a 64 KiB stack in a heap block (63,728 B free), and the
//! plugin's pre-service work (a 236,608 B static path; `sources::service` alone a 77,808 B frame) ran on it and wrote
//! under it. The plugin's part now runs on a stack of its own (`model::thread_stacks::SERVICE_STACK`, checked by
//! `the_service_stacks_cover_…` and by `tools/stack_depth.py <elf> table`). This pins the dispatch's shape: only the
//! game's call stays on the game's stack.

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

const SERVICE: &str = include_str!("../src/runtime/roster/native_service.rs");
const ENTRY: &str = include_str!("../src/runtime/roster/native_service.S");

#[test]
fn the_dispatch_runs_the_plugin_poll_on_a_service_stack_and_the_rest_on_the_game_s() {
    let dispatch = body(SERVICE, "unsafe extern \"C\" fn training_mod_native_service_dispatch(");
    ordered(dispatch, &[
        "match claim() {",
        "Some(stack) => {",
        "training_mod_native_service_on_stack(entry, context as *const _ as usize,",
        "training_mod_native_service_pre as *const () as usize, top(stack));",
        "release(stack);",
        "None => {",
        "EXHAUSTED.fetch_add(1, Ordering::Relaxed);",
        "if !training_mod_native_service_gates(entry) {",
        "return 0;",
        "let result = original(entry);",
        "training_mod_native_service_post(entry, result)",
    ]);
    // The stack is held only for the poll; nothing on the game's thread waits for one.
    for wait in ["loop", "while", "spin", "yield", "sleep", "lock("] {
        assert!(!dispatch.contains(wait), "the dispatch waits ({wait})");
    }
    // No plugin work is called on the game's stack except the small gates and leaves (SERVICE_STACK's native_bound).
    for call in ["super::service(", "enter_native(", "leave_native("] {
        assert!(!dispatch.contains(call), "the dispatch calls {call} directly");
    }
    let pre = body(SERVICE, "unsafe extern \"C\" fn training_mod_native_service_pre(");
    assert!(pre.contains("super::service(context, entry);"));
    assert!(!pre.contains("enter_native(") && !pre.contains("leave_native("), "the gates and leaves always run, poll or not");
    assert!(body(SERVICE, "unsafe fn training_mod_native_service_gates(").contains("enter_native(entry)"));
    assert!(body(SERVICE, "unsafe extern \"C\" fn training_mod_native_service_post(").contains("leave_native(entry);"));
}

#[test]
fn the_trampoline_switches_to_the_service_stack_and_back() {
    let at = ENTRY.find("training_mod_native_service_on_stack:\n").expect("the trampoline");
    let block = &ENTRY[at..at + ENTRY[at..].find(".size").expect("its end")];
    ordered(block, &["stp x29, x30, [sp, #-16]!", "mov x29, sp", "mov sp, x3", "blr x2", "mov sp, x29",
        "ldp x29, x30, [sp], #16", "ret"]);
    // It writes only x29, x30 and SP (the callee keeps x19-x28), and never touches the thread's TLS registers.
    for line in block.lines().skip(1).map(str::trim).filter(|l| !l.is_empty()) {
        let regs: Vec<&str> = line.split(|c: char| !c.is_ascii_alphanumeric()).filter(|t| t.starts_with('x') || t.starts_with('w')).collect();
        assert!(regs.iter().all(|r| ["x2", "x3", "x29", "x30"].contains(r)), "{line}");
        assert!(!line.contains("tpidr"), "{line}");
    }
}

#[test]
fn the_service_stacks_are_static() {
    // Allocated once with the plugin, never on a hot path (the rc.6 hook allocation rule).
    assert!(SERVICE.contains("static STACKS: [ServiceStack; STACK.count] = [const { ServiceStack(core::cell::UnsafeCell::new([0; STACK.bytes])) }; STACK.count];"));
    assert!(SERVICE.contains("const STACK: crate::model::thread_stacks::CallbackStack = crate::model::thread_stacks::SERVICE_STACK;"));
}
