//! Break the Targets (rc.14, `model::btt`, SPEC V2.5), the runtime half. P1's poll keeps the controller from the game while
//! Placing and feeds the cursor; P1's pass runs the clock and breaks the targets with Eggs-ercise's hit test; while the mode
//! is on the camera is the game's fixed one (`btt_camera`); the layout tick draws the targets, their breaks and the cursor
//! (`look`). The menu worker reaches this module through atomics and mailboxes only. The saved layouts (`layouts/`, one file
//! each; btt-5) are the writer thread's on the game's behalf (`service`): game threads do no file I/O and allocate nothing here,
//! they leave a request in an atomic or a mailbox and take this stage's hot five from a published copy (`hos_lock::Published`,
//! read without waiting). The Saved layouts screen does its own file work on the menu worker and asks for the five again.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU8, Ordering};

use hos_lock::{Mailbox, Mutex, Published};

pub use crate::model::btt::LOOKS;
use crate::model::btt::{self, Ask, Btt, Five, Hot, Pad, Phase, HEADER, HOT};
use crate::model::chords::{field, Sample};
use crate::model::eggs::{self, View};
use crate::model::panes::EggLook;
use crate::model::notify::DEFAULT_COLOR;
use crate::model::practice_games::Game;
use crate::model::settings::Snapshot;
use crate::model::storage::ROOT;
use crate::runtime::diag_log::diagnostic;
use crate::runtime::features::{btt_camera, notices};
use crate::runtime::game::natives;
use crate::runtime::{published, scene};

/// The mode's state: taken by P1's poll (the game's input thread), P1's pass (the game thread) and the layout tick. No plugin
/// thread takes it; they use the atomics and the mailboxes below.
static BTT: Mutex<Btt> = Mutex::new(Btt::IDLE);
/// Projectiles break targets (every settings publish), a Start since the last pass, and whether it was Place targets. Whether
/// the mode is on is the running game (`practice::is`, rc.15 practice menu), no longer the `btt` row.
static PROJECTILES: AtomicBool = AtomicBool::new(true);
/// The projectiles rule of the run: latched by Start (rc.15 practice menu: a running game's settings apply at its next
/// Start), read by the hit path.
static PROJECTILES_RUN: AtomicBool = AtomicBool::new(true);
static START: AtomicBool = AtomicBool::new(false);
static PLACE: AtomicBool = AtomicBool::new(false);
/// The menu is open: the pass keeps the layout's stage and file current for the Saved layouts screen even with the row Off.
static OPEN: AtomicBool = AtomicBool::new(false);
/// The mode's phase (`Phase as u8`, 0 Off), for the poll and the chords before they take the lock.
static PHASE: AtomicU8 = AtomicU8::new(0);
/// A layout the Saved layouts screen loaded, for the game thread (the menu worker posts, the pass puts it in place).
pub static CMDS: Mailbox<Hot, 2> = Mailbox::new();
#[cfg(feature = "diag")]
static DIAG: AtomicU32 = AtomicU32::new(0);
/// No stage yet.
const NONE: i32 = i32::MIN;
/// The stage in play (the pass's), for the writer and the Saved layouts screen.
static STAGE: AtomicI32 = AtomicI32::new(NONE);
/// The writer is to list the stage's layouts again (a stage change, or the screen changed a file).
static READ: AtomicBool = AtomicBool::new(false);
/// The game thread's asks, in order; four is more than a player can ask for in one poll of the writer.
static ASKS: Mailbox<Ask, 4> = Mailbox::new();
/// What the writer last listed (stage, its hot five, whether it answers a Save), and the last generation the game thread took.
static RESULT: Published<(i32, Five, bool)> = Published::new((NONE, [None; HOT], false));
static SEEN: AtomicU32 = AtomicU32::new(0);
static HUD: Published<crate::model::storage::FixedPath<96>> = Published::new(crate::model::storage::FixedPath::new());

/// The game-thread's one-line HUD copy; the layout tick never waits on the mode.
pub fn hud() -> crate::model::storage::FixedPath<96> {
    HUD.read()
}

/// The stage in play, `i32::MIN` before the first.
pub fn stage() -> i32 {
    STAGE.load(Ordering::Acquire)
}

/// The Saved layouts screen changed a file: the writer lists the stage again for the game thread.
pub fn reread() {
    READ.store(true, Ordering::Release);
}

/// A stage's name for a layout's default name: the lobby catalog's, or `Stage <id>`.
pub fn stage_label(stage: i32) -> String {
    match crate::editors::setups::stage_name(Some(stage)) {
        "unknown stage" => format!("Stage {stage}"),
        name => name.to_owned(),
    }
}

/// Every settings publish: whether projectiles break targets.
pub fn publish(snapshot: &Snapshot) {
    PROJECTILES.store(snapshot.get("btt_projectiles").is_none_or(|r| r.get(1) == Some(&1)), Ordering::Release);
}

/// Break the Targets is the running game.
fn on() -> bool {
    super::practice::is(Game::Btt)
}

pub fn projectiles() -> bool {
    PROJECTILES_RUN.load(Ordering::Acquire)
}

/// The menu opened (the menu worker; atomics only).
pub fn menu_opened() {
    OPEN.store(true, Ordering::Release);
}

/// The menu closed (the menu worker; atomics only). A close alone starts nothing (rc.15 practice menu, Q1).
pub fn menu_closed() {
    OPEN.store(false, Ordering::Release);
}

/// Start (the menu worker, `practice::start`; atomics only): `place`, it was Place targets. With no targets placed the
/// placing view opens, else Ready with every target back.
pub fn start(place: bool) {
    PROJECTILES_RUN.store(PROJECTILES.load(Ordering::Acquire), Ordering::Release);
    PLACE.store(place, Ordering::Release);
    START.store(true, Ordering::Release);
}

/// A Start is on its way to the next pass.
pub fn starting() -> bool {
    START.load(Ordering::Acquire)
}

/// Snapshot on the game thread. A concurrent layout/menu write can defer one frame.
pub(crate) fn lockon_targets() -> Option<[Option<crate::model::lockon::Target>; btt::MAX]> {
    let spots = BTT.try_lock()?.lockon_spots();
    Some(spots.map(|spot| spot.map(|at| crate::model::lockon::Target { key: lockon_key(at), at })))
}

fn lockon_key([x, y]: [f32; 2]) -> u64 {
    (u64::from(x.to_bits()) << 32) | u64::from(y.to_bits())
}

/// Placing: the save-state chords are the layout's and P1's controller is the cursor's (the chords' poll asks).
pub fn placing() -> bool {
    PHASE.load(Ordering::Acquire) == Phase::Placing as u8
}

/// A practice game is running (Placing, Ready, Running or Done): Hide CPU's rule reads it beside `eggs_running()` (V2.6).
pub fn running() -> bool {
    PHASE.load(Ordering::Acquire) != Phase::Off as u8
}

/// The mode needs P1's pills: it is the running game, its phase is not Off, or a Start is waiting.
pub fn reading() -> bool {
    on() || START.load(Ordering::Acquire) || running()
}

/// Nothing to do: it is not the running game, the mode is not running and no menu is open.
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
pub unsafe fn pass(at: usize, top: [f32; 3], hit: impl FnMut(usize, [f32; 2]) -> bool) -> u8 {
    let facts = scene::facts();
    if facts.busy {
        btt_camera::apply(false);
        return 0;
    }
    let on = on();
    let start = START.swap(false, Ordering::AcqRel);
    let place = start && PLACE.swap(false, Ordering::AcqRel);
    let owners = published::input_owners();
    let live = crate::model::scene::scene(facts).is_some();
    // The mode's camera and the stage's range (the normal placing box) where `btt::own_camera` says; elsewhere the game's own
    // camera and the fallback box.
    let native = live && btt::own_camera(facts);
    let range = native.then(|| natives::camera_limit_range()).flatten();
    let stage = if live { natives::stage_id() } else { -1 };
    let (mut broken, mut mode) = (0, BTT.lock());
    mode.sense(range, top[1]);
    if !live {
        mode.close_menu(false, false);
    } else {
        if stage >= 0 && mode.enter(stage) {
            STAGE.store(stage, Ordering::Release);
            READ.store(true, Ordering::Release);
        }
        if let Some((_, five, saved)) = RESULT.read_if_newer(&SEEN).filter(|r| r.0 == stage) {
            mode.adopt(five, saved);
        }
        if start || !on && mode.phase() != Phase::Off {
            mode.close_menu(on, place);
        }
        // A layout loaded on the Saved layouts screen replaces the course; a run on the old one goes back to Ready with
        // every new target up, so no broken mark of the old course carries over. Placing and Off stay. (`close_menu`, not
        // `Btt::reset`: tests/lock_order.rs resolves calls by name, and a `reset` here reads as `lua_vm::reset`.)
        CMDS.drain(|hot| {
            mode.apply(hot);
            if matches!(mode.phase(), Phase::Ready | Phase::Running | Phase::Done) {
                mode.close_menu(true, false);
            }
        });
        if on {
            let tick = mode.keeper(at) && !owners.covers_strip();
            broken = mode.pass_indexed(tick, hit);
        }
        if let Some(ask) = mode.take_ask().filter(|ask| !ASKS.post(*ask)) {
            diagnostic(format_args!("[Btt] layout ask dropped {ask:?}: the writer is behind"));
        }
    }
    let phase = mode.phase();
    PHASE.store(phase as u8, Ordering::Release);
    if let Some(text) = mode.notice() {
        HUD.publish(text);
        // For the Eden rows: a line per change, and while Running (its clock changes every frame) every tenth, or on a break.
        #[cfg(feature = "diag")]
        if phase != Phase::Running || broken > 0 || DIAG.fetch_add(1, Ordering::Relaxed) % 10 == 0 {
            let (n, cursor) = (mode.layout().len(), mode.cursor());
            diagnostic(format_args!("[Btt] phase={phase:?} n={n} cursor={cursor:?} p1={top:?} text=\"{}\"", text.as_str()));
        }
    }
    drop(mode);
    // Placement alone owns Fixed; X start, Off and UI/scene ownership restore the saved native mode.
    btt_camera::apply(btt::wants_camera(phase, owners.covers_strip(), facts));
    broken
}

/// The writer thread, every poll: the game thread's asks (a Save makes a new file, a Load uses one, a Delete removes one), then,
/// when anything changed or was asked for, this stage's hot five for the game thread.
pub fn service() {
    let (root, mut asked, mut saved) = (Path::new(ROOT), false, false);
    ASKS.drain(|ask| {
        asked = true;
        let done = match ask {
            Ask::Save(stage, n, at) => btt::save_new(root, stage, &stage_label(stage), &at[..usize::from(n)]).map(|_| saved = true),
            Ask::Use(id) => btt::use_file(root, id.as_str(), None),
            Ask::Delete(id) => btt::library(root).delete(id.as_str()),
        };
        if let Err(why) = done {
            diagnostic(format_args!("[Btt] layout {ask:?} failed: {why}"));
            notices::post(HEADER, "Could not save the target layouts", 120, DEFAULT_COLOR);
        }
    });
    let stage = stage();
    if (READ.swap(false, Ordering::AcqRel) || asked) && stage != NONE {
        let (all, skipped) = btt::all(root);
        let (rows, hot) = btt::listed(&all, stage);
        RESULT.publish((stage, btt::five(&rows, hot), saved));
        // For the Eden rows: the stage's list after each change, its first (newest) file named.
        #[cfg(feature = "diag")]
        {
            let first = rows.first().map_or("", |r| r.1.name.as_str());
            diagnostic(format_args!("[Btt] layouts stage={stage} files={} hot={hot} skipped={skipped} saved={saved} first=\"{first}\"", rows.len()));
        }
        #[cfg(not(feature = "diag"))]
        let _ = skipped;
    }
}

static USABLE: AtomicBool = AtomicBool::new(false);
pub fn set_usable(on: bool) { USABLE.store(on, Ordering::Release); }

/// # Safety
/// Called on the game thread with a live fighter or null.
/// Break the Targets' pass (`runtime::features::btt`): Player 1's entry-0 fighters read the same pills Eggs-ercise does, while the mode
/// needs them. Idle (the mode Off, no menu open) it does nothing; with a menu open and the row Off it only keeps the stage's layouts current.
pub unsafe fn frame(fighter: usize) -> Option<()> {
    if !USABLE.load(Ordering::Acquire) { return None; }
    (fighter != 0).then_some(())?;
    let entry = ((fighter + crate::model::lra_reset::FIGHTER_ENTRY_ID_FIELD) as *const i32).read();
    (entry == 0 && !idle()).then_some(())?;
    let at = Some(natives::fighter_accessor(fighter)).filter(|at| *at != 0)?;
    let (top, hits, n, aerial, _) = reading().then(|| super::hitbox::p1_read(at)).unwrap_or_default();
    let rule = super::hitbox::egg_rule();
    let mut melee = false;
    let grader = super::input_display::grader_live();
    let broke = pass(at, top, |slot, spot| {
        if let Some(depth) = eggs::deepest(rule, aerial, &hits[..n], spot) {
            melee = true;
            super::input_display::target_grade(depth);
            true
        } else {
            let helper = super::lockon::hit(at, slot + 1, lockon_key(spot), rule, aerial, projectiles()).is_some();
            match eggs::other_break(grader, helper, |deepest| {
                super::btt_projectiles::hit(at, spot, rule, aerial, projectiles(), deepest)
            }) {
                Some(depth) => {
                    if let Some(depth) = depth {
                        super::input_display::target_grade(depth);
                    }
                    true
                }
                None => false,
            }
        }
    });
    // V2.1: the fighter whose attack broke a target freezes, as on a hit.
    let frames = (broke > 0 && melee).then(scene::facts).and_then(eggs::hitlag)?;
    let _ = natives::set_hit_stop_frame(at, frames, false);
    Some(())
}
