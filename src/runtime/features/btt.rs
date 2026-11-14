//! Break the Targets (rc.14, `model::btt`, SPEC V2.5), the runtime half. P1's poll keeps the controller from the game while
//! Placing and feeds the cursor; P1's pass runs the clock and breaks the targets with Eggs-ercise's hit test; while the mode
//! is on the camera is the game's fixed one (`btt_camera`); the layout tick draws the targets, their breaks and the cursor (`look`). The menu
//! worker reaches this module through atomics and a mailbox only; the stage's saved layouts come and go through `btt_files`
//! (the writer thread), never a game thread's file call.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use hos_lock::{Mailbox, Mutex};

pub use crate::model::btt::LOOKS;
use crate::model::btt::{self, Btt, Pad, Phase};
use crate::model::chords::{field, Sample};
use crate::model::eggs::View;
use crate::model::input_delay::Owners;
use crate::model::notify::DEFAULT_COLOR;
use crate::model::panes::EggLook;
use crate::model::settings::Snapshot;
use crate::model::storage::FixedPath;
use crate::runtime::features::{btt_camera, btt_files, notices};
use crate::runtime::game::natives;
use crate::runtime::{published, scene};

/// The mode's state: taken by P1's poll (the game's input thread), P1's pass (the game thread) and the layout tick. No plugin
/// thread takes it; they use the atomics and the mailbox below.
static BTT: Mutex<Btt> = Mutex::new(Btt::IDLE);
/// The `btt` row is On (every settings publish), the menu closed since the last pass, Place targets was chosen.
static ON: AtomicBool = AtomicBool::new(false);
static START: AtomicBool = AtomicBool::new(false);
static PLACE: AtomicBool = AtomicBool::new(false);
/// The menu is open: the pass keeps the layout's stage and file current for the Saved layouts screen even with the row Off.
static OPEN: AtomicBool = AtomicBool::new(false);
/// The mode's phase (`Phase as u8`, 0 Off), for the poll and the chords before they take the lock.
static PHASE: AtomicU8 = AtomicU8::new(0);
/// What the layout menu asks of the game thread.
static CMDS: Mailbox<Cmd, 2> = Mailbox::new();
#[cfg(feature = "diag")]
static DIAG: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// A layout action from the Saved layouts screen (the menu worker posts, the pass runs it).
#[derive(Clone, Copy)]
pub enum Cmd {
    Load(u8),
    Delete(u8),
}

/// Every settings publish: whether Break the Targets is On.
pub fn publish(snapshot: &Snapshot) {
    ON.store(snapshot.get(btt::ROW).is_some_and(|row| row.iter().position(|v| *v != 0) == Some(1)), Ordering::Release);
}

/// The menu opened (the menu worker; atomics only).
pub fn menu_opened() {
    OPEN.store(true, Ordering::Release);
}

/// The menu closed (the menu worker; atomics only).
pub fn menu_closed() {
    OPEN.store(false, Ordering::Release);
    START.store(true, Ordering::Release);
}

/// Place targets was chosen (the menu worker, before the menu closes).
pub fn request_place() {
    PLACE.store(true, Ordering::Release);
}

/// A layout action (any thread).
pub fn command(cmd: Cmd) {
    CMDS.post(cmd);
}

/// Placing: the save-state chords are the layout's and P1's controller is the cursor's (the chords' poll asks).
pub fn placing() -> bool {
    PHASE.load(Ordering::Acquire) == Phase::Placing as u8
}

/// A practice game is running (Placing, Ready, Running or Done): Hide CPU's rule reads it beside `eggs_running()` (V2.6).
pub fn running() -> bool {
    PHASE.load(Ordering::Acquire) != Phase::Off as u8
}

/// The mode needs P1's pills: the row is On, the mode is running, or a menu close is waiting to start it.
pub fn reading() -> bool {
    ON.load(Ordering::Acquire) || START.load(Ordering::Acquire) || PHASE.load(Ordering::Acquire) != Phase::Off as u8
}

/// Nothing to do: the row is Off, the mode is not running and no menu is open.
pub fn idle() -> bool {
    !reading() && !OPEN.load(Ordering::Acquire)
}

/// One P1 poll (`chords::on_poll`, the game's input thread, before the game maps the controller): while Placing the cursor
/// takes the stick and the buttons; in Ready the first input starts the clock. `keys` is the Save and Load chords' passing,
/// asked of the detector only while Placing. True while the game must not see the controller.
///
/// # Safety
/// `controller` is the live `Controller` the poll passed.
pub unsafe fn poll(controller: usize, sample: Sample, keys: Option<(bool, bool)>) -> bool {
    if PHASE.load(Ordering::Acquire) == Phase::Off as u8 {
        return false;
    }
    let stick = [0, 1].map(|i| ((controller + field::STICKS[i]) as *const f32).read_volatile());
    let (save, load) = keys.unwrap_or_default();
    let mut mode = BTT.lock();
    let hide = mode.feed(Pad { stick, held: sample.held, pressed: sample.pressed, save, load });
    PHASE.store(mode.phase() as u8, Ordering::Release);
    hide
}

/// The layout tick (`hitbox::draw_orbs`): the pictures of the targets, their break animations and the cursor (Eggs-ercise's
/// panes). The camera's map (`view`, read only while the mode is on) is kept for the placing box.
pub fn look(active: bool, view: impl FnOnce() -> Option<View>) -> [Option<([f32; 2], EggLook)>; LOOKS] {
    if !active || PHASE.load(Ordering::Acquire) == Phase::Off as u8 {
        return [None; LOOKS];
    }
    let seen = view();
    let mut mode = BTT.lock();
    mode.see(seen);
    mode.look()
}

/// L+R+A or Change Stage initialised the fighters again (the reset hub, the game thread): a run starts over.
pub fn reset() {
    BTT.lock().reset();
}

/// P1's pass (`at`: the fighter's accessor, `top` its top joint; entry 0, Ice Climbers' Nana too; the game thread), after the fighter
/// pass's reads: the stage, the layout file, the menu close, the run, the notice and the camera. `hit(spot)` says whether a live hit
/// of P1's breaks a target there (the caller's pills and rule); the number of breaks is returned (the hitlag).
///
/// # Safety
/// `at` is a live fighter's accessor.
pub unsafe fn pass(at: usize, top: [f32; 3], mut hit: impl FnMut([f32; 2]) -> bool) -> u8 {
    let facts = scene::facts();
    if facts.busy {
        return 0;
    }
    let on = ON.load(Ordering::Acquire);
    let start = START.swap(false, Ordering::AcqRel);
    let place = start && PLACE.swap(false, Ordering::AcqRel);
    let owners = published::input_owners();
    let live = crate::model::scene::scene(facts).is_some();
    // The waiting room keeps the game's own camera and the fallback box until its LDN check passes (V2.5, D8).
    let (range, stage) = if live { (facts.native_training.then(|| natives::camera_limit_range()).flatten(), natives::stage_id()) } else { (None, -1) };
    let (mut broken, mut mode) = (0, BTT.lock());
    mode.sense(range, top[1]);
    if !live {
        mode.close_menu(false, false);
    } else {
        if stage >= 0 && mode.enter(stage) {
            btt_files::request_read(stage);
        }
        if let Some((_, slots, unreadable)) = btt_files::take_read().filter(|read| read.0 == stage) {
            mode.adopt(slots, unreadable);
        }
        if start || !on && mode.phase() != Phase::Off {
            mode.close_menu(on, place);
        }
        CMDS.drain(|cmd| match cmd {
            Cmd::Load(s) => mode.load(usize::from(s)),
            Cmd::Delete(s) => mode.erase(usize::from(s)),
        });
        if on {
            let tick = mode.keeper(at) && !owners.covers_strip();
            mode.pass(tick, |spot| {
                let broke = hit(spot);
                broken += u8::from(broke);
                broke
            });
        }
        if mode.take_write() {
            btt_files::request_write(stage, *mode.slots());
        }
    }
    let phase = mode.phase();
    let was = PHASE.swap(phase as u8, Ordering::AcqRel);
    if phase == Phase::Off {
        if was != phase as u8 {
            notices::clear(btt::NOTICE_ID);
        }
    } else if let Some(length) = mode.notice() {
        let mut text = FixedPath::<96>::new();
        let _ = mode.write_notice(&mut text);
        notices::upsert(btt::NOTICE_ID, btt::HEADER, text.as_str(), length, DEFAULT_COLOR);
        // For the Eden rows: a line per change, and while Running (its clock changes every frame) every tenth, or on a break.
        #[cfg(feature = "diag")]
        if phase != Phase::Running || broken > 0 || DIAG.fetch_add(1, Ordering::Relaxed) % 10 == 0 {
            let (n, cursor) = (mode.layout().len(), mode.cursor());
            crate::runtime::diag_log::diagnostic(format_args!("[Btt] phase={phase:?} n={n} cursor={cursor:?} p1={top:?} text=\"{}\"", text.as_str()));
        }
    }
    drop(mode);
    // The fixed camera while the mode is on, except under the plugin's menu, the game's pause and the waiting room's UI.
    let covered = [Owners::MENU, Owners::VANILLA_PAUSE, Owners::NATIVE_UI_BUSY].into_iter().any(|o| owners.contains(o));
    btt_camera::apply(phase != Phase::Off && !covered && facts.native_training);
    broken
}
