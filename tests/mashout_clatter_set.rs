//! C10 runtime checks of the actual M3 observer (`src/runtime/features/mashout/clatter_set.rs`): the production arm,
//! release, service, scene, publish/retract and poll bodies run against fake slot cells. Only the hardware-bound
//! pieces are replaced: the svc page query, the thread id, the slot cell map and the two naked wrappers.
use std::{fs, process::Command};

fn block<'a>(source: &'a str, marker: &str) -> &'a str {
    assert_eq!(source.matches(marker).count(), 1, "unique seam: {marker}");
    let start = source.find(marker).unwrap();
    let opening = start + source[start..].find('{').unwrap();
    let mut depth = 0;
    for (offset, byte) in source.as_bytes()[opening..].iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => (),
        }
        if depth == 0 {
            return &source[start..=opening + offset];
        }
    }
    panic!("unterminated seam: {marker}");
}

fn production(source: &str) -> &str {
    source.split("#[cfg(test)]").next().unwrap()
}

const STUBS: &str = r#"
static THREAD: AtomicUsize = AtomicUsize::new(1);
static PERM: [AtomicU32; 3] = [AtomicU32::new(3), AtomicU32::new(3), AtomicU32::new(3)];
static CELLS: [AtomicUsize; 2] = [AtomicUsize::new(0), AtomicUsize::new(0)];
fn current_thread() -> usize { THREAD.load(Ordering::Relaxed) }
unsafe fn permission(address: usize) -> Option<u32> {
    let i = match address - game::text_base() { START_SLOT => 0, SET_SLOT => 1, ADD_SLOT => 2, _ => panic!("page") };
    Some(PERM[i].load(Ordering::Relaxed))
}
unsafe fn slot_at(offset: usize) -> &'static AtomicUsize {
    match offset { SET_SLOT => &CELLS[0], ADD_SLOT => &CELLS[1], _ => panic!("unexpected slot") }
}
unsafe extern "C" fn set_wrapper(_control: usize, _index: i32, _operand: f32) {}
unsafe extern "C" fn add_wrapper(_control: usize, _index: i32, _operand: f32) {}
"#;

const TESTS: &str = r#"
mod model {
    pub mod mashout {
        pub mod grade {
            #[derive(Clone, Copy, Debug, PartialEq, Eq)]
            pub struct Bounds { pub fastest: u32, pub no_input: Option<u32> }
        }
        pub mod native { // NATIVE
        }
        pub mod clatter { // CLATTER
        }
    }
}
mod runtime { pub mod game { pub fn text_base() -> usize { 0x1000_0000 } } }

struct Fake { control: Box<[usize; 4]>, record: Box<f32>, common: Box<[u8; 0x60]> }
fn fake() -> Fake {
    let mut record = Box::new(30.0f32);
    let mut control = Box::new([game::text_base() + TABLE, 0, 0, 0]);
    control[3] = &mut *record as *mut f32 as usize; // [control + 0x18] = record 0
    let mut common = Box::new([0u8; 0x60]);
    common[0x40..0x54].copy_from_slice(&COMMON_BUILD_ID);
    Fake { control, record, common }
}
fn ctl(f: &Fake) -> usize { &*f.control as *const [usize; 4] as usize }
fn base(f: &Fake) -> usize { &*f.common as *const [u8; 0x60] as usize }
fn reset() {
    retract_common();
    OWNER_THREAD.store(0, Ordering::Relaxed);
    THREAD.store(1, Ordering::Relaxed);
    for p in &PERM { p.store(3, Ordering::Relaxed); }
    CELLS[0].store(game::text_base() + SET_ORIGINAL, Ordering::Relaxed);
    CELLS[1].store(game::text_base() + ADD_ORIGINAL, Ordering::Relaxed);
    LOST.store(false, Ordering::Relaxed);
    INSTALLED.store(false, Ordering::Relaxed);
    SCENE_ACTIVE.store(false, Ordering::Relaxed);
}
/// One owner-site call, as mashout::owner_site does it.
fn owner(active: bool) { scene(active); service(); }

#[test]
fn arm_refusals() {
    let f = fake();
    reset();
    assert_eq!(arm(5, 113, ctl(&f)), Err(Refusal::NotOwnerThread), "service never ran: no owner thread yet");
    owner(true);
    THREAD.store(2, Ordering::Relaxed);
    assert_eq!(arm(5, 113, ctl(&f)), Err(Refusal::NotOwnerThread), "another thread");
    THREAD.store(1, Ordering::Relaxed);
    assert_eq!(arm(5, 40, ctl(&f)), Err(Refusal::WrongPhase), "not a clatter-write tag");
    assert_eq!(arm(5, 113, ctl(&f)), Err(Refusal::CommonUnverified), "common not published");
    publish_common(base(&f));
    let other = Box::new([game::text_base() + 0x4f6bd90, 0usize, 0, 0]);
    assert_eq!(arm(5, 113, &*other as *const _ as usize), Err(Refusal::VtableMismatch), "a non-FighterControl vtable");
    PERM[1].store(1, Ordering::Relaxed);
    assert_eq!(arm(5, 113, ctl(&f)), Err(Refusal::PageCheck), "slot page differs from the Start slot's");
    PERM[1].store(3, Ordering::Relaxed);
    assert_eq!(arm(5, 113, ctl(&f)), Ok(5));
    assert_eq!(CELLS[0].load(Ordering::Relaxed), set_wrapper as *const () as usize, "installed and read back");
    assert_eq!(arm(6, 116, ctl(&f)), Err(Refusal::AlreadyArmed));
}

#[test]
fn foreign_before_and_after_are_never_overwritten() {
    let f = fake();
    reset();
    owner(true);
    publish_common(base(&f));
    CELLS[0].store(0xdead, Ordering::Relaxed);
    assert_eq!(arm(5, 113, ctl(&f)), Err(Refusal::ForeignSlot), "foreign before");
    assert_eq!(CELLS[0].load(Ordering::Relaxed), 0xdead, "a foreign value is left alone");
    assert_eq!(arm(5, 113, ctl(&f)), Err(Refusal::ForeignSlot), "LOST latched: further installs refused");
    reset();
    owner(true);
    publish_common(base(&f));
    assert_eq!(arm(7, 113, ctl(&f)), Ok(7));
    CELLS[0].store(0xbeef, Ordering::Relaxed); // foreign after install
    owner(true);
    assert_eq!(CELLS[0].load(Ordering::Relaxed), 0xbeef, "restore only over TR's own wrapper");
    assert!(matches!(poll(7, 1), Poll::Poisoned(Poison::Lost)), "the attempt is poisoned Lost");
}

#[test]
fn scope_loss_retracts_closes_and_restores_in_the_same_owner_call() {
    let f = fake();
    reset();
    owner(true);
    publish_common(base(&f));
    assert_eq!(arm(8, 113, ctl(&f)), Ok(8));
    owner(false);
    assert_eq!(COMMON_LR.load(Ordering::Relaxed), 0, "word retracted first");
    assert_eq!(P1_CONTROL.load(Ordering::Relaxed), 0, "gate closed");
    assert_eq!(CELLS[0].load(Ordering::Relaxed), game::text_base() + SET_ORIGINAL, "slot restored in the same call");
}

/// L2 (rc15-m3-l2-owner-xps-3): the owner site does not run on the main menu, so an exit must not leave the slot
/// installed until the next Training frame. A release on the owner thread restores in the same call.
#[test]
fn release_on_the_owner_thread_restores_in_the_same_call() {
    let f = fake();
    reset();
    owner(true);
    publish_common(base(&f));
    assert_eq!(arm(13, 113, ctl(&f)), Ok(13));
    release();
    assert_eq!(CELLS[0].load(Ordering::Relaxed), game::text_base() + SET_ORIGINAL, "restored without another owner call");
    assert_eq!(arm(14, 117, ctl(&f)), Ok(14));
    THREAD.store(2, Ordering::Relaxed);
    release();
    assert_eq!(CELLS[1].load(Ordering::Relaxed), add_wrapper as *const () as usize, "off the owner thread: gate only");
    THREAD.store(1, Ordering::Relaxed);
    owner(true);
    assert_eq!(CELLS[1].load(Ordering::Relaxed), game::text_base() + ADD_ORIGINAL, "the owner site restores it");
}

#[test]
fn a_common_load_while_armed_poisons_lost() {
    let f = fake();
    reset();
    owner(true);
    publish_common(base(&f));
    assert_eq!(arm(9, 113, ctl(&f)), Ok(9));
    publish_common(base(&f)); // remap/reload mid-attempt
    assert!(matches!(poll(9, 1), Poll::Poisoned(Poison::Lost)));
    owner(true);
    assert_eq!(CELLS[0].load(Ordering::Relaxed), game::text_base() + SET_ORIGINAL, "generation changed: released and restored");
}

unsafe extern "C" fn fake_set(control: usize, _index: i32, operand: f32) {
    **(((control + 0x18) as *const *mut f32)) = operand.max(0.0);
}

#[test]
fn qualified_writes_publish_in_order_and_overflow_poisons() {
    let mut f = fake();
    reset();
    owner(true);
    publish_common(base(&f));
    assert_eq!(arm(10, 113, ctl(&f)), Ok(10));
    ORIGINAL.store(fake_set as *const () as usize, Ordering::Relaxed);
    let frame = Box::new([3u32, 0, 250.0f32.to_bits(), 0]); // L2CValue{number, d_max} at [x29 - 0xc0]
    let fp = &*frame as *const [u32; 4] as usize + D_MAX_SLOT;
    *f.record = 30.0;
    unsafe { qualified(ctl(&f), 0, fp, 41.5) };
    let Poll::Event(w) = poll(10, 1) else { panic!("first write") };
    assert_eq!((w.slot, w.tag, f32::from_bits(w.old), f32::from_bits(w.operand), f32::from_bits(w.new)),
        (Slot::Set, 113, 30.0, 41.5, 41.5));
    assert_eq!(f32::from_bits(w.d_max), 250.0);
    assert!(matches!(poll(10, 2), Poll::Empty));
    assert!(matches!(poll(11, 1), Poll::Empty), "another generation is refused without poisoning");
    for k in 2..=RING as u32 + 1 {
        unsafe { qualified(ctl(&f), 0, fp, 41.5 + k as f32) };
    }
    assert!(matches!(poll(10, 1), Poll::Poisoned(Poison::Overflow)), "a 9th write overflows the ring");
}

#[test]
fn tag_117_installs_the_add_instance_only() {
    let f = fake();
    reset();
    owner(true);
    publish_common(base(&f));
    assert_eq!(COMMON_ADD_LR.load(Ordering::Relaxed), base(&f) + COMMON_ADD_RETURN);
    assert_eq!(arm(12, 117, ctl(&f)), Ok(12));
    assert_eq!(CELLS[1].load(Ordering::Relaxed), add_wrapper as *const () as usize);
    assert_eq!(CELLS[0].load(Ordering::Relaxed), game::text_base() + SET_ORIGINAL, "Set untouched (C4)");
    release();
    owner(true);
    assert_eq!(CELLS[1].load(Ordering::Relaxed), game::text_base() + ADD_ORIGINAL, "Add restored");
}
"#;

#[test]
fn actual_clatter_set_arm_restore_poison_and_add_instance() {
    let source = include_str!("../src/runtime/features/mashout/clatter_set.rs");
    let mut runtime = source.replace("#[unsafe(naked)]\n", "");
    for marker in ["fn current_thread()", "unsafe fn permission(", "unsafe fn slot_at(",
                   "unsafe extern \"C\" fn set_wrapper(", "unsafe extern \"C\" fn add_wrapper("] {
        let body = block(&runtime, marker).to_owned();
        runtime = runtime.replace(&body, "");
    }
    let fixture = TESTS
        .replace("// NATIVE\n", production(include_str!("../src/model/mashout/native.rs")))
        .replace("// CLATTER\n", production(include_str!("../src/model/mashout/clatter.rs")));
    let program = format!("{}\n{STUBS}\n{fixture}", runtime.replace("//!", "//"));
    let dir = crate::test_scratch::Scratch::new("tr-clatter-set");
    let input = dir.join("actual.rs");
    let binary = dir.join("actual");
    fs::write(&input, program).unwrap();
    let compiled = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--edition=2021", "--test", "-A", "dead_code", "-A", "unused"])
        .arg(&input).arg("-o").arg(&binary).output().unwrap();
    assert!(compiled.status.success(), "{}", String::from_utf8_lossy(&compiled.stderr));
    let run = Command::new(&binary).args(["--test-threads=1", "--nocapture"]).output().unwrap();
    print!("{}", String::from_utf8_lossy(&run.stdout));
    let _ = fs::remove_dir_all(&dir);
    assert!(run.status.success(), "{}\n{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));
}
