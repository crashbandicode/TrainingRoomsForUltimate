//! Break the Targets (rc.14, `model::btt`, SPEC V2.5), the runtime half. P1's poll keeps the controller from the game while
//! Placing and feeds the cursor; P1's pass runs the clock and breaks the targets with Eggs-ercise's hit test; while the mode
//! is on the camera is the game's fixed one (`btt_camera`); the layout tick draws the targets, their breaks and the cursor
//! (`look`). The menu worker reaches this module through atomics and mailboxes only. The stage's saved layouts
//! (`targets/<stage>.json`) are read and written by the writer thread only (`service`): game threads do no file I/O and
//! allocate nothing here, they leave a request in an atomic or a mailbox and take the answer from a published copy
//! (`hos_lock::Published`, read without waiting).

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU8, Ordering};

use hos_lock::{Mailbox, Mutex, Published};

pub use crate::model::btt::LOOKS;
use crate::model::btt::{self, Btt, Pad, Phase, Slots, HEADER, NOTICE_ID, SLOTS};
use crate::model::chords::{field, Sample};
use crate::model::eggs::View;
use crate::model::notify::DEFAULT_COLOR;
use crate::model::panes::EggLook;
use crate::model::settings::Snapshot;
use crate::model::storage::ROOT;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::features::{btt_camera, notices};
use crate::runtime::game::natives;
use crate::runtime::{published, scene};

/// The mode's state: taken by P1's poll (the game's input thread), P1's pass (the game thread) and the layout tick. No plugin
/// thread takes it; they use the atomics and the mailboxes below.
static BTT: Mutex<Btt> = Mutex::new(Btt::IDLE);
/// The `btt` row is On (every settings publish), the menu closed since the last pass, Place targets was chosen.
static ON: AtomicBool = AtomicBool::new(false);
static START: AtomicBool = AtomicBool::new(false);
static PLACE: AtomicBool = AtomicBool::new(false);
/// The menu is open: the pass keeps the layout's stage and file current for the Saved layouts screen even with the row Off.
static OPEN: AtomicBool = AtomicBool::new(false);
/// The mode's phase (`Phase as u8`, 0 Off), for the poll and the chords before they take the lock.
static PHASE: AtomicU8 = AtomicU8::new(0);
/// What the Saved layouts screen asks of the game thread (the menu worker posts, the pass runs it): `(delete, slot)`, a load
/// when not a delete.
pub static CMDS: Mailbox<(bool, usize), 2> = Mailbox::new();
#[cfg(feature = "diag")]
static DIAG: AtomicU32 = AtomicU32::new(0);
/// No stage: no read is waiting, nothing has been read yet.
const NONE: i32 = i32::MIN;
/// The stage whose file the game thread wants read (newest wins).
static READ: AtomicI32 = AtomicI32::new(NONE);
/// Whole-file writes, in order; four is more than a player can ask for in one poll of the writer.
static WRITES: Mailbox<(i32, Slots), 4> = Mailbox::new();
/// What the last read found (stage, slots, whether the file could not be read), and the last generation the game thread took.
static RESULT: Published<(i32, Slots, bool)> = Published::new((NONE, [None; SLOTS], false));
static SEEN: AtomicU32 = AtomicU32::new(0);
/// The stage and slots as last read or written (an unreadable file is empty slots): the Saved layouts screen's (any thread).
pub static CACHE: Published<(i32, Slots)> = Published::new((NONE, [None; SLOTS]));

/// Every settings publish: whether Break the Targets is On.
pub fn publish(snapshot: &Snapshot) {
    ON.store(snapshot.get(btt::ROW).is_some_and(|row| row.iter().position(|v| *v != 0) == Some(1)), Ordering::Release);
}

/// The menu opened (the menu worker; atomics only).
pub fn menu_opened() {
    OPEN.store(true, Ordering::Release);
}

/// The menu closed (the menu worker; atomics only); `place`: it closed on Place targets.
pub fn menu_closed(place: bool) {
    OPEN.store(false, Ordering::Release);
    PLACE.store(place, Ordering::Release);
    START.store(true, Ordering::Release);
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
    ON.load(Ordering::Acquire) || START.load(Ordering::Acquire) || running()
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
    if !running() {
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
    if !active || !running() {
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
pub unsafe fn pass(at: usize, top: [f32; 3], hit: impl FnMut([f32; 2]) -> bool) -> u8 {
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
    let native = live && facts.native_training;
    let range = native.then(|| natives::camera_limit_range()).flatten();
    let stage = if live { natives::stage_id() } else { -1 };
    let (mut broken, mut mode) = (0, BTT.lock());
    mode.sense(range, top[1]);
    if !live {
        mode.close_menu(false, false);
    } else {
        if stage >= 0 && mode.enter(stage) {
            READ.store(stage, Ordering::Release);
        }
        if let Some((_, slots, bad)) = RESULT.read_if_newer(&SEEN).filter(|r| r.0 == stage) {
            mode.adopt(slots, bad);
        }
        if start || !on && mode.phase() != Phase::Off {
            mode.close_menu(on, place);
        }
        CMDS.drain(|(delete, s)| [Btt::load, Btt::erase][usize::from(delete)](&mut mode, s));
        if on {
            let tick = mode.keeper(at) && !owners.covers_strip();
            broken = mode.pass(tick, hit);
        }
        if mode.take_write() && !WRITES.post((stage, *mode.slots())) {
            diagnostic(format_args!("[Btt] target layouts write dropped stage={stage}: the writer is behind"));
        }
    }
    let phase = mode.phase();
    PHASE.store(phase as u8, Ordering::Release);
    // Asked on every pass, Off included: the text is empty exactly when the mode is Off, so the notice goes then, and the next
    // text is posted again even when it is the one shown before the Off (Ready of the same layout).
    if let Some(text) = mode.notice() {
        if text.as_str().is_empty() {
            notices::clear(NOTICE_ID);
        } else {
            notices::upsert(NOTICE_ID, HEADER, text.as_str(), u16::MAX, DEFAULT_COLOR);
        }
        // For the Eden rows: a line per change, and while Running (its clock changes every frame) every tenth, or on a break.
        #[cfg(feature = "diag")]
        if phase != Phase::Running || broken > 0 || DIAG.fetch_add(1, Ordering::Relaxed) % 10 == 0 {
            let (n, cursor) = (mode.layout().len(), mode.cursor());
            diagnostic(format_args!("[Btt] phase={phase:?} n={n} cursor={cursor:?} p1={top:?} text=\"{}\"", text.as_str()));
        }
    }
    drop(mode);
    // The fixed camera while the mode is on, except under the plugin's menu, the game's pause and the waiting room's UI.
    btt_camera::apply(phase != Phase::Off && !owners.covers_strip() && facts.native_training);
    broken
}

/// The writer thread, every poll: the writes asked for, then the read asked for.
pub fn service() {
    let root = Path::new(ROOT);
    WRITES.drain(|(stage, slots)| {
        // At most one retry. A failed write keeps the layouts in memory for the session and says so.
        if !(0..2).any(|_| btt::write(root, stage, &slots).is_ok()) {
            diagnostic(format_args!("[Btt] target layouts for stage {stage} not written; kept for this session"));
            let why = "Could not save the target layouts";
            notices::post(HEADER, why, 120, DEFAULT_COLOR);
        }
        CACHE.publish((stage, slots));
    });
    let stage = READ.swap(NONE, Ordering::AcqRel);
    if stage != NONE {
        let (slots, unreadable) = btt::read(root, stage);
        diagnostic(format_args!("[Btt] target layouts read stage={stage} unreadable={unreadable}"));
        RESULT.publish((stage, slots, unreadable));
        CACHE.publish((stage, slots));
    }
}
