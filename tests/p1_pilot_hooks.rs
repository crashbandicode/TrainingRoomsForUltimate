//! The CPU-as-P1 fallback's hooks allocate nothing (playbook "No heap in hooks"; primary inbox 2026-11-08 19:13Z). Eden
//! `rc6-int-smoke1` (no standalone pilot, so the fallback was installed) read `hook_stats allocs=input_caller_a_apply:2,
//! cpu_frame:5754`:
//! - 4 per `request.json` poll: the skyline std's `File::open` of a missing file allocates a `CString`, a `format!` and the two
//!   boxes of `io::Error::new` (`library/std/src/sys/fs/switch.rs`);
//! - the `admission` / `first_pair` lines built `serde_json::Value`s at every new binding (each fighter switch);
//! - the first lock of each `std::sync::Mutex` boxes its pthread mutex on this target (`input_caller_a_apply:2`).
//!
//! The writers the hooks use now are proven here under the plugin's counting allocator; the wiring of the Switch-only
//! `runtime/p1/cpu_pilot.rs` (no file and no heap call on a hook path; the file work on the heartbeat) is pinned by its source.

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;
#[path = "../src/model/compat/fixed.rs"]
#[allow(dead_code)]
mod fixed;
#[path = "../src/model/compat/json_line.rs"]
mod json_line;

use std::fmt::{self, Write as _};

use fixed::FixedQueue;
use json_line::{DebugStr, Json, Obj};
use model::hook_stats::{hook_allocations, CountingAllocator, HookId, HookScope};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// The counters are process-wide and the harness runs tests on several threads: the tests take turns.
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A diag line on the stack, as the diag ring's `StackLine` holds one.
struct Line {
    bytes: [u8; 2048],
    len: usize,
}

impl Line {
    fn new() -> Self {
        Self { bytes: [0; 2048], len: 0 }
    }
    fn text(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).unwrap()
    }
}

impl fmt::Write for Line {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len + s.len();
        if end > self.bytes.len() {
            return Err(fmt::Error);
        }
        self.bytes[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum Scope {
    OfflineTraining,
}

#[derive(Clone, Copy, serde::Serialize)]
struct Binding {
    ai: usize,
    local_boid: u32,
    local_entry: i32,
    scene: Scope,
}

#[derive(Clone, Copy, Debug)]
enum Status {
    Pending,
}

const BINDING: Binding = Binding { ai: 77_040_471_552, local_boid: 524_288, local_entry: 0, scene: Scope::OfflineTraining };

#[test]
fn an_admission_line_is_written_without_the_heap_the_json_value_needed() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    // The fork's line: a `serde_json::Value` per line, inside the hook.
    let before = hook_allocations();
    let old = {
        let _scope = HookScope::enter_hook(HookId::CpuFrame);
        let value = serde_json::json!({"gate": "admission_ready", "detail": {"binding": BINDING, "fim_mask": 0u8, "stage_initialization": null}});
        let mut line = Line::new();
        write!(line, "[CpuPilot] admission {value}").unwrap();
        line.text().to_owned()
    };
    assert!(hook_allocations() > before, "the Value line allocates inside the hook");
    // The typed line: the same text, nothing allocated.
    let before = hook_allocations();
    let mut line = Line::new();
    {
        let _scope = HookScope::enter_hook(HookId::CpuFrame);
        let detail = Obj((("binding", BINDING), ("fim_mask", 0u8), ("stage_initialization", None::<u8>)));
        write!(line, "[CpuPilot] admission {}", Json(Obj((("detail", detail), ("gate", "admission_ready"))))).unwrap();
    }
    assert_eq!(hook_allocations(), before, "the typed line allocates nothing");
    assert_eq!(line.text(), old);
}

#[test]
fn strings_floats_options_and_debug_text_are_written_without_the_heap() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let before = hook_allocations();
    let mut line = Line::new();
    {
        let _scope = HookScope::enter_hook(HookId::CpuFrame);
        let value = Obj((
            ("damage", 46.9f64),
            ("mode", DebugStr(Some(Status::Pending))),
            ("name", "Mii \"Gunner\"\n\u{e9}"),
            ("outputs", [0x8000u64, 0x8001]),
            ("rank", None::<i32>),
        ));
        write!(line, "{}", Json(value)).unwrap();
    }
    assert_eq!(hook_allocations(), before);
    let expected = serde_json::json!({"damage": 46.9f64, "mode": "Some(Pending)", "name": "Mii \"Gunner\"\n\u{e9}",
        "outputs": [0x8000u64, 0x8001], "rank": null});
    assert_eq!(line.text(), expected.to_string());
}

#[test]
fn a_line_that_does_not_fit_stops_without_the_heap() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let long = [BINDING; 32];
    let before = hook_allocations();
    let mut line = Line::new();
    let result = {
        let _scope = HookScope::enter_hook(HookId::CpuFrame);
        write!(line, "[CpuPilot] first_pair {}", Json(Obj((("pairs", long),))))
    };
    assert_eq!(hook_allocations(), before, "serde_json builds no error value: the sink never fails");
    assert!(result.is_err() && line.len > 2000, "the formatter's error, after the bytes that fit");
}

#[test]
fn receipts_move_through_the_queue_without_the_heap() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let mut queue = FixedQueue::<(u32, Vec<u64>), 2>::new();
    let traces: Vec<Vec<u64>> = (0..3).map(|n| Vec::with_capacity(128 + n)).collect();
    let before = hook_allocations();
    let (refused, first) = {
        let _scope = HookScope::enter_hook(HookId::CpuFrame);
        let mut traces = traces.into_iter();
        let mut trace = traces.next().unwrap();
        for value in 0..128u64 {
            trace.push(value); // within the capacity reserved off the hook
        }
        assert!(queue.push((1, trace)).is_ok() && queue.push((2, traces.next().unwrap())).is_ok());
        let refused = queue.push((3, traces.next().unwrap())).expect_err("a full queue hands the receipt back");
        (refused, queue.pop())
    };
    assert_eq!(hook_allocations(), before);
    assert_eq!((refused.0, queue.take_refused()), (3, 1));
    let first = first.unwrap();
    assert_eq!((first.0, first.1.len(), first.1.capacity()), (1, 128, 128));
    assert_eq!(queue.pop().map(|(n, _)| n), Some(2));
}

// ---- the Switch-only fallback's wiring --------------------------------------------------------------------------------

const PILOT: &str = include_str!("../src/runtime/p1/cpu_pilot.rs");

fn body<'a>(src: &'a str, start: &str) -> &'a str {
    let at = src.find(start).unwrap_or_else(|| panic!("{start}"));
    &src[at..at + src[at..].find("\n}\n").expect("end of the function")]
}

/// Every function a hook reaches (the fighter frame's `on_frame`, the input stage's `input`, the AI service's
/// `ai_process_begin`) and what they call.
const HOOK_PATHS: &[&str] = &[
    "pub(crate) unsafe fn on_frame(",
    "pub(crate) unsafe fn input(",
    "unsafe fn ai_process_begin(",
    "fn gate<",
    "unsafe fn pair_diagnostic(",
    "unsafe fn entry_details(",
    "fn reset_admission(",
    "fn reject_pending(",
    "unsafe fn poll_request(",
    "unsafe fn arm(",
    "unsafe fn finish(",
    "fn publish(",
    "fn post(",
    "unsafe fn command_output(",
    "fn pause(",
    "unsafe fn sample_flags(",
    "unsafe fn think(",
    "unsafe fn snapshot(",
    "fn stage_evidence(",
    "fn clear_stage(",
    "fn cancel_stage_initialization(",
    "unsafe fn stage_admission(",
    "unsafe fn stage_live(",
    "fn invalidate_stage(",
    "unsafe fn pair(",
    "unsafe fn entry(",
    "unsafe fn native(",
    "unsafe fn current_scene(",
    "fn menu_busy(",
];

/// What allocates or opens a file.
const HEAP: &[&str] = &[
    "json!(", "format!(", "fs::", "File::", "with_capacity(", ".to_string()", ".clone()", "vec!", "Box::new", "String::", "to_value(",
    ".collect", ".to_vec()", ".to_owned()", "request()", "plan(",
];

#[test]
fn no_hook_path_of_the_fallback_allocates_or_opens_a_file() {
    for start in HOOK_PATHS {
        let text = body(PILOT, start);
        for call in HEAP {
            assert!(!text.contains(call), "`{call}` on the hook path `{start}`:\n{text}");
        }
    }
    assert!(PILOT.contains("use hos_lock::Mutex;") && !PILOT.contains("std::sync::Mutex"), "no lazily boxed std mutex in a hook");
}

/// Primary 2026-11-09 12:48Z (B's `rc6-bg-qp4` M5: `admission … scope lan_online_1v1 … "fim_mask":0,
/// "gate":"fim_slot_not_unique"`, P1 stood still): the fallback's online 1v1 admission needs the local slot in its FIM mask,
/// and only `cpu_pilot::input` sets it. The input stage returned before that call on every poll the delay does not act on
/// (`input_delay::applies` is P1's poll in a Training scene only), so in an online match the mask stayed 0. The stage reaches
/// the fallback on every poll that has a record: P1's Training poll after the delay (its pad goes in after the delay), and
/// every other poll (any slot, any scene) before the stage returns.
#[test]
fn the_fallback_sees_every_mapped_poll() {
    let stage = body(include_str!("../src/runtime/features/input_delay.rs"), "fn apply(");
    let call = "crate::runtime::p1::input(slot as i32, out";
    assert_eq!(stage.matches(call).count(), 2, "the delayed poll's call and the other polls' call:\n{stage}");
    let gate = stage.find("if !input_delay::applies(slot as i32, scene::is_training()) {").expect("the delay's own gate");
    let branch = &stage[gate..gate + stage[gate..].find("        return;\n    }").expect("the branch's return")];
    assert!(branch.contains(call), "a poll the delay does not act on still reaches the fallback:\n{branch}");
    assert!(!stage.contains("|| !input_delay::applies("), "no return before the fallback on a poll that has a record");
    // The fallback itself decides per scene and slot; nothing in between filters the slot or the scene.
    // `p1::input` is that file's last function (no newline after its brace, so `body` cannot cut it): its text is matched whole.
    let p1 = include_str!("../src/runtime/p1/mod.rs");
    assert!(
        p1.contains("pub(crate) unsafe fn input(slot: i32, out: *mut MappedInputs) {\n    if cpu_pilot::installed() {\n        cpu_pilot::input(slot, out.cast());\n    }\n}"),
        "p1::input passes every poll to the fallback"
    );
    let input = body(PILOT, "pub(crate) unsafe fn input(");
    assert!(input.contains("FIM_MASK.fetch_or(1usize << player_idx, Ordering::AcqRel);"), "{input}");
}

#[test]
fn the_file_work_runs_on_the_heartbeat() {
    // request.json is read and planned only by the heartbeat's `read_request`; the hook asks at its old cadence and takes it.
    assert_eq!(PILOT.matches("= request()").count(), 1, "one call of the file reader");
    assert!(body(PILOT, "fn read_request(").contains("let Some((request, kind)) = request() else { return };"));
    assert!(body(PILOT, "fn read_request(").contains("let planned = unsafe { plan(kind, &request.run) };"));
    let poll = body(PILOT, "unsafe fn poll_request(");
    assert!(poll.contains("if state.polls % 60 == 1 {") && poll.contains("READ_WANTED.store(true, Ordering::Release);"));
    assert!(poll.contains("POSTED.try_lock().and_then(|mut posted| posted.take())"), "the hook never waits for the heartbeat");
    // The receipts are posted, and written by `service`.
    assert!(body(PILOT, "fn publish(").contains("post(Outgoing::Receipt {"));
    assert!(body(PILOT, "fn reject_pending(").contains("post(Outgoing::Rejected {"));
    let service = body(PILOT, "pub(crate) fn service(");
    assert!(service.contains("while write_next() {}") && service.contains("if READ_WANTED.swap(false, Ordering::AcqRel) {"));
    assert!(body(PILOT, "fn write_next(").contains("write_receipt(run, reason, &value);"));
    assert!(body(PILOT, "fn next_receipt(").contains("let next = OUTBOX.lock().pop();"));
    // The command trace is reserved off the hook and never grows on it.
    assert!(body(PILOT, "unsafe fn plan(").contains("trace: Vec::with_capacity(TRACE_LIMIT),"));
    assert!(body(PILOT, "unsafe fn command_output(").contains("if command.trace.len() < TRACE_LIMIT {"));
    assert!(body(PILOT, "unsafe fn arm(").contains("first_outputs: FixedList::new(0),"));
    // The heartbeat calls it, as its own step (its frames do not stack on the seam report's).
    let boot = include_str!("../src/runtime/boot.rs");
    let seams = boot.find("diag_log::set_tick(crate::runtime::seams::seam_install::report_late);").expect("the seam report");
    let pilot = boot.find("diag_log::set_tick(p1::service);").expect("the fallback's file work");
    let drain = boot.find("diag_log::start_drain_thread();").expect("the heartbeat");
    assert!(seams < pilot && pilot < drain);
    assert!(body(include_str!("../src/runtime/p1/mod.rs"), "pub(crate) fn service(").contains("cpu_pilot::service();"));
    let diag_log = include_str!("../src/runtime/diag_log.rs");
    assert!(diag_log.contains("for tick in TICKS.iter().filter_map(|slot| slot.get()) {\n                tick();"));
}
