//! One Random roll per attempt (mashout-demo-devin TRIAGE-RANDOM-REROLL.md): a Random attacker was rolled again by
//! the restart its own attacker load causes. Runs the ACTUAL `Game::step`, `if restart {` and `Step::Choose => {`
//! blocks of src/runtime/features/mashout.rs in a host fixture (RED before the fix: "the restart rolled another
//! attacker", 400 vs 300).
//! Live evidence: run rc15-cpu-receipt-ally-2, `[TrainingRooms] hub_run cause=TrainingReset` (#2742) directly
//! before `[Mashout] step gen=26 step=Choose` (#2743) and a new practice request for another fighter (#2760).
use std::{fs, process::Command};

const SOURCE: &str = include_str!("../src/runtime/features/mashout.rs");

fn block<'a>(source: &'a str, marker: &str) -> &'a str {
    assert_eq!(source.matches(marker).count(), 1, "unique actual-runtime seam: {marker}");
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
    panic!("unterminated actual-runtime seam: {marker}");
}

const FIXTURE: &str = r#"
#![allow(dead_code, unused_unsafe, unused_variables, unused_assignments, non_snake_case)]
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
static GENERATION: AtomicU64 = AtomicU64::new(10);
static TEMP_KIND: AtomicU32 = AtomicU32::new(u32::MAX);
static READY: AtomicBool = AtomicBool::new(true);
static REQUESTED: AtomicU32 = AtomicU32::new(u32::MAX);
static REQUEST_COUNT: AtomicUsize = AtomicUsize::new(0);
static TICK: AtomicU64 = AtomicU64::new(100);
static mut BODY_MEM: [u32; 4] = [0, 0, 0, 1];
mod skyline { pub mod nn { pub mod os {
    /// Each read differs, like the system tick between two Choose frames.
    pub fn GetSystemTick() -> u64 { crate::TICK.fetch_add(100, crate::Ordering::Relaxed) }
} } }
mod worker { pub enum Event { Cancel(u64), Setup(u64, &'static str) } }
fn post(_: worker::Event, _: u64) {}
fn revoke() {}
mod natives {
    pub fn fighter_entry(entry: usize) -> usize { entry }
    pub fn entry_active_body(_: usize) -> usize { unsafe { core::ptr::addr_of!(super::BODY_MEM) as usize } }
    pub fn is_stop(_: usize) -> bool { false }
}
mod runtime { pub mod roster { pub mod character_roster {
    pub fn cpu_reload_ready() -> bool { crate::READY.load(crate::Ordering::Relaxed) }
    pub fn request_practice_cpu_kind(kind: u32) -> bool {
        crate::REQUESTED.store(kind, crate::Ordering::Relaxed);
        crate::REQUEST_COUNT.fetch_add(1, crate::Ordering::Relaxed);
        true
    }
} } }
/// rc15/mashout's M3 observer gate (Game::step closes it on every exit from an attempt).
mod clatter_set { pub fn release() {} }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step { Off, Choose, Reload, Restore, Release, Place, Entering, Held }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Scenario { attacker_kind: u32 }
/// The Random attacker: every roll may name another fighter (moves.rs Config::resolve, RANDOM_ATTACKER).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Config { random: bool }
impl Config {
    fn resolve(self, roll: u64, _: u64) -> Option<Scenario> {
        Some(Scenario { attacker_kind: if self.random { (roll % 1000) as u32 } else { 63 } })
    }
}
struct Game {
    generation: u64, step: Step, requested: Option<Scenario>, roll: Option<Scenario>, p1: usize, cpu: usize, saw_catch: bool,
    followed: bool, trolley: Option<()>, hero: Option<()>, original: Option<u32>, config: Option<Config>,
    clock: Option<()>, restore_requested: bool, failed: Option<&'static str>, trail_magic: Option<()>,
    prep_damage: Option<()>,
}
impl Game {
    fn new(config: Config) -> Self {
        Self { generation: 10, step: Step::Choose, requested: None, roll: None, p1: 11, cpu: 22, saw_catch: false,
               followed: false, trolley: None, hero: None, original: None, config: Some(config), clock: None,
               restore_requested: false, failed: None, trail_magic: None, prep_damage: None }
    }
    // STEP
    fn fail(&mut self, text: &'static str) { self.failed = Some(text); }
}
unsafe fn actual_restart(game: &mut Game, config: Option<Config>, linked: bool) {
    let restart = true;
    // RESTART
}
unsafe fn actual_choose(game: &mut Game, config: Option<Config>, p1: usize, cpu: usize, cpu_entry: usize) {
    let linked = false;
    let ids = [0; 16];
    let (status, cpu_status, situation, cpu_situation) = (0, 0, 0, 0);
    match game.step {
        // CHOOSE
        _ => (),
    }
}
fn body_kind(kind: u32) { unsafe { BODY_MEM[3] = kind; } }
fn fresh() {
    REQUEST_COUNT.store(0, Ordering::Relaxed);
    REQUESTED.store(u32::MAX, Ordering::Relaxed);
    TEMP_KIND.store(u32::MAX, Ordering::Relaxed);
    READY.store(true, Ordering::Relaxed);
}

/// Mash On with a Random attacker; Choose rolls fighter A and asks for it. A's body initialises: the reset hub runs
/// (`hub_run cause=TrainingReset`, reset_hub.rs -> mashout::reset -> START) and Mash out restarts into Choose. The
/// attempt that has not started yet must keep fighter A, so no second load is asked for.
#[test]
fn the_restart_from_its_own_attacker_load_keeps_the_random_roll() {
    fresh();
    let config = Config { random: true };
    let mut game = Game::new(config);
    body_kind(1);
    unsafe { actual_choose(&mut game, Some(config), 11, 22, 99) };
    let first = game.requested.unwrap().attacker_kind;
    assert_eq!((game.step, REQUEST_COUNT.load(Ordering::Relaxed), REQUESTED.load(Ordering::Relaxed)), (Step::Reload, 1, first));
    body_kind(first);
    unsafe { actual_restart(&mut game, Some(config), false) };
    assert_eq!(game.step, Step::Choose);
    unsafe { actual_choose(&mut game, Some(config), 11, 22, 99) };
    assert_eq!(game.requested.unwrap().attacker_kind, first, "the restart rolled another attacker");
    assert_eq!(REQUEST_COUNT.load(Ordering::Relaxed), 1, "a second attacker load was asked for");
}

/// Each attempt still rolls: once the attempt starts (Entering), the next attempt's Choose rolls again; a settings
/// change drops a kept roll too.
#[test]
fn a_new_attempt_or_a_settings_change_rolls_again() {
    fresh();
    let config = Config { random: true };
    let mut game = Game::new(config);
    body_kind(1);
    unsafe { actual_choose(&mut game, Some(config), 11, 22, 99) };
    let first = game.requested.unwrap().attacker_kind;
    body_kind(first);
    game.step(Step::Entering, "Mash out · Waiting for move");
    game.step = Step::Choose;
    unsafe { actual_choose(&mut game, Some(config), 11, 22, 99) };
    assert_ne!(game.requested.unwrap().attacker_kind, first, "the next attempt kept the old roll");
    assert_eq!(REQUEST_COUNT.load(Ordering::Relaxed), 2);
    let kept = game.requested.unwrap().attacker_kind;
    let changed = Config { random: true };
    game.config = None; // Mash Off -> On: the settings changed
    unsafe { actual_restart(&mut game, Some(changed), false) };
    unsafe { actual_choose(&mut game, Some(changed), 11, 22, 99) };
    assert_ne!(game.requested.unwrap().attacker_kind, kept, "a settings change kept the old roll");
}

/// Control: a concrete attacker is unaffected (same fighter after the restart, no second load). Passes today.
#[test]
fn a_concrete_attacker_restart_asks_for_no_second_load() {
    fresh();
    let config = Config { random: false };
    let mut game = Game::new(config);
    body_kind(1);
    let before = REQUEST_COUNT.load(Ordering::Relaxed);
    unsafe { actual_choose(&mut game, Some(config), 11, 22, 99) };
    body_kind(63);
    unsafe { actual_restart(&mut game, Some(config), false) };
    unsafe { actual_choose(&mut game, Some(config), 11, 22, 99) };
    assert_eq!(REQUEST_COUNT.load(Ordering::Relaxed) - before, 1);
}
"#;

#[test]
fn actual_mashout_random_roll_survives_its_own_reload_restart() {
    let fixture = FIXTURE
        .replace("// STEP", block(SOURCE, "fn step(&mut self, next: Step, text: &'static str) {"))
        .replace("// RESTART", block(SOURCE, "if restart {"))
        .replace("// CHOOSE", block(SOURCE, "Step::Choose => {"));
    let dir = std::env::temp_dir().join(format!("mashout-random-roll-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let input = dir.join("actual.rs");
    let binary = dir.join("actual");
    fs::write(&input, fixture).unwrap();
    let compiler = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let compiled = Command::new(compiler).args(["--edition=2021", "--test"]).arg(&input).arg("-o").arg(&binary)
        .output().unwrap();
    assert!(compiled.status.success(), "fragments did not compile: {}", String::from_utf8_lossy(&compiled.stderr));
    let run = Command::new(&binary).args(["--test-threads=1"]).output().unwrap();
    print!("{}", String::from_utf8_lossy(&run.stdout));
    let _ = fs::remove_dir_all(&dir);
    assert!(run.status.success(), "actual runtime regression failed:\n{}", String::from_utf8_lossy(&run.stdout));
}
