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
use crate::model::input_delay::{MappedInputs, Owners, Sticks};
use crate::runtime::game::natives;
use crate::runtime::published;

/// The display's natives passed their guards (`model::capabilities`): until boot says so, nothing here reads the game.
static ENABLED: AtomicBool = AtomicBool::new(false);
/// The hitstun-left read may run (`reaction_left`).
static REACTION_ON: AtomicBool = AtomicBool::new(false);
/// P1's fighter read may run (`p1_fighter`).
static KIND_ON: AtomicBool = AtomicBool::new(false);
/// P1's latest mapped inputs, packed (`pack`).
static INPUTS: AtomicU64 = AtomicU64::new(0);
/// The constants resolved once (`ids`).
static IDS_READY: AtomicBool = AtomicBool::new(false);
static IDS: Mutex<Option<Table>> = Mutex::new(None);
static TRACKER: Mutex<Tracker> = Mutex::new(Tracker::new());
static RENDERER: Mutex<Renderer> = Mutex::new(Renderer::new());

/// Boot: the display's closure passed its guards.
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Release);
}

fn settings() -> Settings {
    published::live().map_or(Settings::factory(), |live| live.input_display)
}

/// The strip's panes were found in the layout last resolved (`hud_draw`, once per registration).
static PANES: AtomicBool = AtomicBool::new(false);

/// `hud_draw` resolved the layout's panes: whether the strip's container is there.
pub fn set_panes(found: bool) {
    PANES.store(found, Ordering::Release);
}

/// Whether the strip is sampling, and so owns the frame advantage (`model::frame_strip::strip_owns_advantage`): boot
/// enabled it, the row is On and the layout has its panes.
pub fn sampling() -> bool {
    if !ENABLED.load(Ordering::Acquire) {
        return false;
    }
    crate::model::frame_strip::strip_owns_advantage(true, settings().on, PANES.load(Ordering::Acquire))
}

fn pack(m: MappedInputs) -> u64 {
    let b = |v: i8| u64::from(v as u8);
    u64::from(m.buttons) | b(m.lstick_x) << 32 | b(m.lstick_y) << 40 | b(m.rstick_x) << 48 | b(m.rstick_y) << 56
}

fn unpack(v: u64) -> MappedInputs {
    let b = |shift: u32| (v >> shift) as u8 as i8;
    MappedInputs { buttons: v as u32, lstick_x: b(32), lstick_y: b(40), rstick_x: b(48), rstick_y: b(56) }
}

/// One P1 poll after the delay: keep the mapped record's buttons (what the fighter acts on) with the physical sticks
/// of the same delayed poll (`Sticks`: the record's own sticks carry the C-stick on a C-stick attack's first frames).
///
/// # Safety
/// `out` is the delayed mapped record of the poll (or null).
pub unsafe fn on_p1_poll(_controller: usize, out: *const MappedInputs, sticks: Sticks) {
    if !ENABLED.load(Ordering::Acquire) || out.is_null() || published::input_owners().contains(Owners::MENU) {
        return;
    }
    INPUTS.store(pack(sticks.onto(*out)), Ordering::Release);
}

/// The constants, resolved through the game's own table on first use (`None` until the table answers anything).
unsafe fn ids() -> Option<Table> {
    if !IDS_READY.load(Ordering::Acquire) {
        let mut table = Table::unresolved();
        let missing = table.resolve(|key| crate::runtime::game::lua_const(key));
        if missing == crate::model::frame_strip::ids::COUNT {
            return None;
        }
        // Logged from the local table: a guard is a temporary that lives to the end of its statement, and the lock is
        // not reentrant, so taking `IDS` more than once in one statement would wait on itself.
        #[cfg(feature = "diag")]
        crate::runtime::diag_log::diagnostic(format_args!(
            "[FrameStrip] constants resolved missing={missing} reaction_frame={} attack={:#x} attack_hi3={:#x} final={:#x}",
            table.get(C::ReactionFrame),
            table.get(C::Attack),
            table.get(C::AttackHi3),
            table.get(C::Final)
        ));
        *IDS.lock() = Some(table);
        IDS_READY.store(true, Ordering::Release);
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

/// P1's fighter, for a saved combo's "recorded on" (SPEC-input-07 §4, metadata only): `utility::get_kind` is the CPU
/// core unit's row, so the read runs only while that unit's closure passed (boot); otherwise `None`, and a save
/// records no fighter.
unsafe fn p1_fighter(accessor: usize) -> Option<i32> {
    if !KIND_ON.load(Ordering::Acquire) {
        return None;
    }
    natives::fighter_kind(accessor)
}

/// Boot: the fighter read's row passed its guard (the CPU core unit's closure).
pub fn set_fighter_read(on: bool) {
    KIND_ON.store(on, Ordering::Release);
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
    // Facing and airborne are in the display's closure (`state_lr`, `situation_kind`). A rejected row turns the
    // display off. A default here would record every combo as right-facing and grounded.
    let lr = natives::lr(accessor);
    let situation = natives::situation_kind(accessor);
    // The attack slots' damage, for the spot of a hit (rc.14, SP1): both fighters', read from the game's own `AttackData`.
    let (active, powers) = crate::model::hitbox::powers(&mut super::hitbox::GameAttacks(accessor));
    Sample {
        stop: natives::is_stop(accessor),
        hitstun,
        shieldstun: ids.shieldstun(status, prev),
        actionable: crate::runtime::features::cpu::actionable(accessor),
        active: p1 && active,
        damage: natives::damage(accessor, 0),
        powers,
        attack: if p1 { ids.attack(status) } else { None },
        reaction,
        shielding: ids.shielding(status),
        status,
        motion_frame: natives::motion_frame(accessor),
        facing: if lr < 0.0 { -1 } else { 1 },
        airborne: ids.resolved(C::SituationAir) && situation == ids.get(C::SituationAir),
    }
}

/// One game frame, on P1's entry callback in Training or lobby practice (`cpu::fighter_frame`). `player` is P1's
/// accessor; the primary CPU's comes from the live manager (`cpu::strip_cpu`).
///
/// # Safety
/// `player` is P1's live module accessor; the call is made on the game thread inside the fighter's frame.
pub unsafe fn sample_frame(player: usize) {
    if !ENABLED.load(Ordering::Acquire) || !sampling() || player == 0 {
        return;
    }
    if crate::runtime::features::states::is_loading() {
        TRACKER.lock().clear();
        return;
    }
    let Some(cpu) = crate::runtime::features::cpu::strip_cpu(player) else { return };
    let Some(ids) = ids() else { return };
    let cpu_accessor = cpu;
    let p1 = sample(player, &ids, true);
    let cpu = sample(cpu, &ids, false);
    #[cfg(feature = "diag")]
    {
        probe_module(0, player, &p1, cpu.damage);
        probe_module(1, cpu_accessor, &cpu, p1.damage);
    }
    #[cfg(not(feature = "diag"))]
    let _ = cpu_accessor;
    // Diag build: P1's busy frames, once per classification. Motion frame changes every frame, so it
    // is not part of the key; logging it each frame rotated practice-diagnostics.log about once a minute
    // and the row gates lost their lines (rc.5 wire-fix, 2026-11-07).
    #[cfg(feature = "diag")]
    {
        static LAST: Mutex<Option<(i32, bool, bool, bool, Option<crate::model::frame_strip::Move>)>> =
            Mutex::new(None);
        let key = (p1.status, p1.stop, p1.active, p1.actionable, p1.attack);
        let mut last = LAST.lock();
        if *last != Some(key) {
            *last = Some(key);
            if !p1.actionable || p1.active || p1.stop {
                crate::runtime::diag_log::diagnostic(format_args!(
                    "[FrameStrip] p1 status={:#x} stop={} active={} actionable={} attack={:?} mf={}",
                    p1.status, p1.stop, p1.active, p1.actionable, p1.attack, p1.motion_frame
                ));
            }
        }
    }
    let inputs = unpack(INPUTS.load(Ordering::Acquire));
    // P1's fighter, for a saved combo's "recorded on" (metadata; the existing guarded read, no new native site).
    let p1_kind = p1_fighter(player);
    let mut tracker = TRACKER.lock();
    #[cfg(feature = "diag")]
    let (check, advantage) = (tracker.stun_check, tracker.advantage());
    tracker.step(p1, cpu, inputs);
    // The combo helper reads the same step (SPEC-input-07 §3); lock order TRACKER, then the helper's.
    crate::runtime::features::combo::after_strip(&tracker, p1_kind);
    #[cfg(feature = "diag")]
    {
        let (check_now, link, advantage_now) = (tracker.stun_check, tracker.latest_link(), tracker.advantage());
        let (read, mark) = (tracker.spot_read.take(), tracker.marks().next());
        drop(tracker);
        if let Some((frame, gained, powers)) = read {
            crate::runtime::diag_log::diagnostic(format_args!("[Spots] stop frame={frame} gained={gained:.3} powers={powers:?} newest_mark={mark:?}"));
        }
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

/// Diag build (spots-2 probe): the `AttackModule` floats that scale a hitbox's damage, read raw from the module at accessor `+0xa0` on
/// every frame a fighter attacks or is in the side-smash family, once per change. `AttackModule::get_power` (vtable slot 0x158,
/// `main+0x3dd580`) multiplies `AttackData.power` by the floats at `+0xf0`, `+0xf4`, `+0xe0` (then adds `+0xe4`), `+0xf8`, `+0xfc`,
/// `+0x100`, `+0x104` and `+0x210`, and blends `+0x20c` by its float argument; the charge of a smash is one of them or none of them.
/// The line pairs each fighter's raw floats with its status, motion frame, damage and both fighters' damage, so the Eden probe fits
/// the damage a hit added against every candidate. Every address is type-checked first (Eden answers 0 for an unmapped read).
#[cfg(feature = "diag")]
fn probe_module(who: usize, accessor: usize, s: &Sample, other_damage: f32) {
    use std::fmt;
    /// The module offsets read: `get_power`'s floats, `+0x108` (customize ratio), `+0x1e0` (power up), `+0x208..+0x214`.
    const OFFSETS: [usize; 20] = [0xe0, 0xe4, 0xe8, 0xec, 0xf0, 0xf4, 0xf8, 0xfc, 0x100, 0x104, 0x108, 0x10c, 0x1e0, 0x1e4, 0x208, 0x20c, 0x210, 0x214, 0x238, 0x23c];
    struct Fields([u32; 20]);
    impl fmt::Display for Fields {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            for (i, bits) in self.0.iter().enumerate() {
                write!(f, "{}{:#x}={:?}", if i == 0 { "" } else { " " }, OFFSETS[i], f32::from_bits(*bits))?;
            }
            Ok(())
        }
    }
    static LAST: Mutex<[Option<(i32, bool, bool, [u32; 20])>; 2]> = Mutex::new([None, None]);
    let family = (0x2d..=0x2f).contains(&s.status);
    let attacking = s.powers.iter().any(|&p| p > 0.0);
    if accessor == 0 || !(attacking || family) {
        return;
    }
    // SAFETY: a read of the live fighter's accessor field; the module pointer is checked (non-null, 8-aligned, a user address) before every field read.
    let module = unsafe { ((accessor + 0xa0) as *const usize).read() };
    if module < 0x1000 || module & 7 != 0 || module >> 47 != 0 {
        return;
    }
    let mut bits = [0u32; 20];
    for (slot, off) in bits.iter_mut().zip(OFFSETS) {
        // SAFETY: inside the AttackModule object (its fields end past 0x23c), read-only.
        *slot = unsafe { ((module + off) as *const u32).read() };
    }
    let key = (s.status, s.stop, attacking, bits);
    let mut last = LAST.lock();
    if last[who] == Some(key) {
        return;
    }
    last[who] = Some(key);
    drop(last);
    crate::runtime::diag_log::diagnostic(format_args!(
        "[SpotsProbe] who={} status={:#x} stop={} active={} mf={} damage={:.3} other_damage={:.3} powers={:?} fields: {}",
        ["p1", "cpu"][who],
        s.status,
        s.stop,
        attacking,
        s.motion_frame,
        s.damage,
        other_damage,
        s.powers,
        Fields(bits)
    ));
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
    crate::runtime::features::combo::session_clear();
}

/// The game initialised a fighter object again (`reset_hub::on_init`): a Training reset, a CPU replaced, a respawn.
/// Nothing on the strip happened in this timeline any more, as after a save-state load (review IV-4: a hit just
/// after a reset showed a link spanning it). Kept out of line: the follow-up Eden rows arm its entry on release.
#[inline(never)]
pub fn fighter_initialised() {
    if ENABLED.load(Ordering::Acquire) {
        TRACKER.lock().clear();
    }
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
    RENDERER.lock().draw_spots(&tracker, shown, settings().spots, |local, look, changed| {
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
