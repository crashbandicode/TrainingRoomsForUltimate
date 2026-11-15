//! Draw callbacks of the other packages' panels (notifications, input display, damage readouts): each is
//! called for every layout tick with the layout and its name, and gates on the name itself
//! (SPEC-core-05 §5: "name-gated draws"). Registered at boot; the tick reads the slots lock-free.

use std::sync::atomic::{AtomicUsize, Ordering};

use skyline::nn::ui2d::Layout;

/// `layout` is the layout being ticked, `name` its name bytes (e.g. `info_training`).
pub type HudDraw = unsafe fn(layout: *mut Layout, name: &[u8]);

const SLOTS: usize = 8;
static DRAWS: [AtomicUsize; SLOTS] = [const { AtomicUsize::new(0) }; SLOTS];

/// Register a callback; returns false when all slots are taken.
pub fn register(draw: HudDraw) -> bool {
    for slot in &DRAWS {
        if slot.compare_exchange(0, draw as usize, Ordering::AcqRel, Ordering::Acquire).is_ok() {
            return true;
        }
    }
    false
}

pub(super) unsafe fn run(layout: *mut Layout, name: &[u8]) {
    for slot in &DRAWS {
        let value = slot.load(Ordering::Acquire);
        if value != 0 {
            // SAFETY: only `register` stores a non-zero value, and it stores a `HudDraw`.
            let draw: HudDraw = core::mem::transmute::<usize, HudDraw>(value);
            draw(layout, name);
        }
    }
}
/// The game's HUD layouts the menu hides while it shows (SPEC-core-05 §2: "The game HUD is also hidden while the
/// plugin menu is open"): their root panes go invisible for the menu and visible again after it. The `hud` setting
/// (Misc, default On) joins this rule in step S5-4.
const HUD_LAYOUTS: [&[u8]; 5] = [b"info_playercursor", b"info_playercursor_item", b"info_melee", b"info_radar_a", b"info_radar_b"];
static HIDDEN_BY_MENU: AtomicUsize = AtomicUsize::new(0);

/// The `info_melee` panels Hide CPU hid, one bit per entry (only this rule's tick touches it).
static PANELS_HIDDEN: AtomicUsize = AtomicUsize::new(0);

/// Hide CPU: a hidden CPU's HUD panel (its `info_melee` part: portrait, name, damage, stocks) is invisible on every tick
/// while the body is, and visible again on the first tick after. Player 1's part and every other pane are left alone.
pub(super) unsafe fn cpu_panels(layout: *mut Layout, name: &[u8]) {
    use crate::model::cpu::hide;
    if name == b"info_waiting" {
        room_card();
        return;
    }
    if name != b"info_melee" {
        return;
    }
    let wanted = usize::from(crate::runtime::features::hide_cpu::panels());
    let was = PANELS_HIDDEN.load(Ordering::Acquire);
    let root = (*layout).root_pane;
    if wanted | was == 0 || root.is_null() {
        return;
    }
    let mut now = 0;
    for entry in 0..8 {
        let mut hidden = was & (1 << entry) != 0;
        if let (Some(visible), Some(part)) = (hide::panel_write(wanted & (1 << entry) != 0, &mut hidden), hide::panel_part(entry)) {
            let pane = super::pane::find(root, &part);
            if !pane.is_null() {
                super::pane::set_visible(pane, visible);
            }
            #[cfg(feature = "diag")]
            if (was & (1 << entry) != 0) != hidden {
                crate::runtime::diag_log::diagnostic(format_args!(
                    "[TrainingRooms] hide_cpu panel entry={entry} visible={visible} found={}",
                    !pane.is_null()
                ));
            }
        }
        now |= usize::from(hidden) << entry;
    }
    PANELS_HIDDEN.store(now, Ordering::Release);
}

/// The waiting room's CPU card Hide CPU hid (only this rule's tick touches it).
static ROOM_HIDDEN: AtomicUsize = AtomicUsize::new(0);

/// Hide CPU in a waiting room's practice: the room shows its CPU (entry 4) not in `info_melee` but as a partner card of
/// `info_waiting` (`tp_0N`: window, damage, portrait), found fresh each tick through the game's own partner mapping
/// (`character_roster::waiting_cpu_card`). Invisible on every tick while the body is hidden, visible once after.
unsafe fn room_card() {
    use crate::model::cpu::hide;
    let was = ROOM_HIDDEN.load(Ordering::Acquire) != 0;
    let wanted = hide::room_panel(crate::runtime::features::hide_cpu::panels());
    let mut hidden = was;
    if let Some(visible) = hide::panel_write(wanted, &mut hidden) {
        let card = crate::runtime::roster::character_roster::waiting_cpu_card();
        if let Some(card) = card {
            super::pane::set_visible(card, visible);
        }
        #[cfg(feature = "diag")]
        if was != hidden {
            crate::runtime::diag_log::diagnostic(format_args!(
                "[TrainingRooms] hide_cpu room card visible={visible} found={}",
                card.is_some()
            ));
        }
    }
    ROOM_HIDDEN.store(usize::from(hidden), Ordering::Release);
}

pub(super) unsafe fn hud_rule(layout: *mut Layout, name: &[u8]) {
    let Some(index) = HUD_LAYOUTS.iter().position(|n| *n == name) else { return };
    let root = (*layout).root_pane;
    if root.is_null() {
        return;
    }
    let bit = 1usize << index;
    let hidden = HIDDEN_BY_MENU.load(Ordering::Acquire) & bit != 0;
    // Hidden while the menu shows, and (HUD Off) while a Training match is running; never outside Training.
    let want_hidden = crate::runtime::game::is_training_mode()
        && (super::state::MENU_SHOWN.load(Ordering::Acquire)
            || (!crate::runtime::published::hud() && crate::runtime::game::is_ready_go()));
    if want_hidden {
        super::pane::set_visible(root, false);
        if !hidden {
            HIDDEN_BY_MENU.fetch_or(bit, Ordering::AcqRel);
        }
    } else if hidden {
        super::pane::set_visible(root, true);
        HIDDEN_BY_MENU.fetch_and(!bit, Ordering::AcqRel);
    }
}