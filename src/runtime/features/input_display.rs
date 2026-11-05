//! The input display: the frame strip (SPEC-input-06; operator decision INPUT-VIZ option A, 2026-11-05). It replaced
//! the input log of SPEC-input-02.
//!
//! - The P1 poll (after the input delay) keeps P1's latest mapped inputs.
//! - Once per game frame, on P1's entry callback (`cpu::fighter_frame`), both fighters are read and the tracker
//!   (`model::frame_strip`) takes one step.
//! - The HUD tick (`hud_draw`) turns the tracker into pane writes (`model::frame_strip_plan`), writing only what
//!   changed.
//!
//! Every read is a guarded native (`game::natives`). The hitstun-left read is optional: when its work id or
//! `WorkModule::get_float` is unavailable, a true combo shows no margin and everything else is drawn (SPEC-input-06 §4).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use hos_lock::Mutex;

use crate::model::frame_strip::ids::{Table, C};
use crate::model::frame_strip::{Sample, Settings, Tracker};
use crate::model::frame_strip_plan::{Changed, Look, Renderer};
use crate::model::input_delay::{MappedInputs, Owners};
use crate::runtime::game::natives;
use crate::runtime::published;

/// The display's natives passed their guards (`model::capabilities`): until boot says so, nothing here reads the game.
static ENABLED: AtomicBool = AtomicBool::new(false);
/// The hitstun-left read may run (`reaction_left`).
static REACTION_ON: AtomicBool = AtomicBool::new(false);
/// P1's latest mapped inputs, packed (`pack`).
static INPUTS: AtomicU64 = AtomicU64::new(0);
/// The constants resolved once (`ids`).
static IDS_READY: AtomicBool = AtomicBool::new(false);
static IDS: Mutex<Option<Table>> = Mutex::new(None);
static TRACKER: Mutex<Tracker> = Mutex::new(Tracker::new());
static RENDERER: Mutex<Renderer> = Mutex::new(Renderer::new());

/// AttackModule slots read for P1's active frames (`model::hitbox::SLOTS`).
const ATTACK_SLOTS: i32 = crate::model::hitbox::SLOTS;

/// Boot: the display's closure passed its guards.
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Release);
}

fn settings() -> Settings {
    published::live().map_or(Settings::factory(), |live| live.input_display)
}

/// Whether the strip is sampling (boot enabled it and the row is On).
pub fn sampling() -> bool {
    if !ENABLED.load(Ordering::Acquire) {
        return false;
    }
    settings().on
}

fn pack(m: MappedInputs) -> u64 {
    let b = |v: i8| u64::from(v as u8);
    u64::from(m.buttons) | b(m.lstick_x) << 32 | b(m.lstick_y) << 40 | b(m.rstick_x) << 48 | b(m.rstick_y) << 56
}

fn unpack(v: u64) -> MappedInputs {
    let b = |shift: u32| (v >> shift) as u8 as i8;
    MappedInputs { buttons: v as u32, lstick_x: b(32), lstick_y: b(40), rstick_x: b(48), rstick_y: b(56) }
}

/// One P1 poll after the delay: keep the mapped record (what the fighter acts on).
///
/// # Safety
/// `out` is the delayed mapped record of the poll (or null).
pub unsafe fn on_p1_poll(_controller: usize, out: *const MappedInputs) {
    if !ENABLED.load(Ordering::Acquire) || out.is_null() || published::input_owners().contains(Owners::MENU) {
        return;
    }
    INPUTS.store(pack(*out), Ordering::Release);
}

/// The constants, resolved through the game's own table on first use (`None` until the table answers anything).
unsafe fn ids() -> Option<Table> {
    if !IDS_READY.load(Ordering::Acquire) {
        let mut table = Table::unresolved();
        let missing = table.resolve(|key| crate::runtime::game::lua_const(key));
        if missing == crate::model::frame_strip::ids::COUNT {
            return None;
        }
        *IDS.lock() = Some(table);
        IDS_READY.store(true, Ordering::Release);
        #[cfg(feature = "diag")]
        crate::runtime::diag_log::diagnostic(format_args!(
            "[FrameStrip] constants resolved missing={missing} reaction_frame={}",
            IDS.lock().as_ref().map_or(0, |t| t.get(C::ReactionFrame))
        ));
    }
    IDS.lock().clone()
}

/// The hitstun left (SPEC-input-06 §4): `WorkModule::get_float` is the CPU tech unit's row, so the read runs only
/// while that unit's closure passed (boot); otherwise `None`, and a true combo shows no margin.
unsafe fn reaction_left(accessor: usize, id: i32) -> Option<f32> {
    if !REACTION_ON.load(Ordering::Acquire) {
        return None;
    }
    Some(natives::get_float(accessor, id))
}

/// Boot: the hitstun-left read's row passed its guard (the CPU tech unit's closure).
pub fn set_reaction(on: bool) {
    REACTION_ON.store(on, Ordering::Release);
}

/// One fighter's frame (SPEC-input-06 §4).
unsafe fn sample(accessor: usize, ids: &Table, p1: bool) -> Sample {
    let status = natives::status_kind(accessor);
    let prev = natives::prev_status_kind(accessor, 0);
    let hitstun = ids.hitstun(status);
    let reaction = if !p1 && hitstun && ids.resolved(C::ReactionFrame) {
        reaction_left(accessor, ids.get(C::ReactionFrame))
    } else {
        None
    };
    Sample {
        stop: natives::is_stop(accessor),
        hitstun,
        shieldstun: ids.shieldstun(status, prev),
        actionable: crate::runtime::features::cpu::actionable(accessor),
        active: p1 && (0..ATTACK_SLOTS).any(|slot| natives::attack_is_attack(accessor, slot)),
        attack: if p1 { ids.attack(status) } else { None },
        reaction,
        shielding: ids.shielding(status),
        status,
        motion_frame: if p1 { natives::motion_frame(accessor) } else { 0.0 },
    }
}

/// One game frame, on P1's entry callback in Training or lobby practice (`cpu::fighter_frame`). `player` is P1's
/// accessor; the primary CPU's comes from the live manager (`cpu::strip_cpu`).
///
/// # Safety
/// `player` is P1's live module accessor; the call is made on the game thread inside the fighter's frame.
pub unsafe fn sample_frame(player: usize) {
    if !ENABLED.load(Ordering::Acquire) || !settings().on || player == 0 {
        return;
    }
    if crate::runtime::features::states::is_loading() {
        TRACKER.lock().clear();
        return;
    }
    let Some(cpu) = crate::runtime::features::cpu::strip_cpu(player) else { return };
    let Some(ids) = ids() else { return };
    let p1 = sample(player, &ids, true);
    let cpu = sample(cpu, &ids, false);
    let inputs = unpack(INPUTS.load(Ordering::Acquire));
    let mut tracker = TRACKER.lock();
    #[cfg(feature = "diag")]
    let (check, advantage) = (tracker.stun_check, tracker.advantage());
    tracker.step(p1, cpu, inputs);
    #[cfg(feature = "diag")]
    {
        let (check_now, link, advantage_now) = (tracker.stun_check, tracker.latest_link(), tracker.advantage());
        drop(tracker);
        if advantage_now != advantage {
            if let Some(a) = advantage_now {
                crate::runtime::diag_log::diagnostic(format_args!(
                    "[FrameStrip] advantage value={} on_shield={} p1={} cpu={}",
                    a.value, a.on_shield, a.p1, a.cpu
                ));
            }
        }
        diag_frame(&cpu, check, check_now, link);
    }
}

/// Diag build: the hitstun reads and every prediction checked against the free frame seen (SPEC-input-06 §4, `K`),
/// and each new link.
#[cfg(feature = "diag")]
fn diag_frame(cpu: &Sample, before: Option<(u32, u32)>, now: Option<(u32, u32)>, link: Option<crate::model::frame_strip::Link>) {
    use crate::runtime::diag_log::diagnostic;
    static LAST_LINK: Mutex<Option<crate::model::frame_strip::Link>> = Mutex::new(None);
    if let Some(r) = cpu.reaction {
        diagnostic(format_args!("[FrameStrip] reaction r={r} stop={} actionable={}", cpu.stop, cpu.actionable));
    }
    if now != before {
        if let Some((predicted, seen)) = now {
            diagnostic(format_args!("[FrameStrip] stun_check predicted={predicted} seen={seen} k={}", crate::model::frame_strip::K));
        }
    }
    let mut last = LAST_LINK.lock();
    if link != *last {
        *last = link;
        if let Some(l) = link {
            diagnostic(format_args!(
                "[FrameStrip] link verdict={:?} hit={} free={:?} actionable={} press={} input={} window={:?} from={:?} to={:?}",
                l.verdict, l.hit, l.free, l.actionable, l.press, l.input, l.window, l.from, l.to
            ));
        }
    }
}

/// Session clear: leaving Training, a hot swap, a lobby change.
pub fn session_clear() {
    TRACKER.lock().clear();
}

/// The plugin's layout was registered again: every pane is written again.
pub fn on_layout_registered() {
    RENDERER.lock().invalidate();
}

/// One HUD tick: hand every pane write the strip needs to `apply`. Shown unless a menu is open, the row is Off or
/// boot did not enable the display. Returns how many panes were written.
pub fn hud_tick(menu_open: bool, mut apply: impl FnMut(usize, &Look, Changed)) -> usize {
    let shown = !menu_open && sampling();
    let tracker = *TRACKER.lock();
    RENDERER.lock().draw(&tracker, shown, |local, look, changed| {
        // Diag build: every show or hide of the strip, with why (the menu, the vanilla pause, the row Off).
        #[cfg(feature = "diag")]
        if local == crate::model::frame_strip_plan::at::CONTAINER && changed.visible {
            crate::runtime::diag_log::diagnostic(format_args!(
                "[FrameStrip] shown={} menu_open={menu_open} on={}",
                look.visible,
                sampling()
            ));
        }
        apply(local, look, changed)
    })
}
