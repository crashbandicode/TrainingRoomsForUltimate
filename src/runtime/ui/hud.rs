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

pub(super) unsafe fn hud_rule(layout: *mut Layout, name: &[u8]) {
    let Some(index) = HUD_LAYOUTS.iter().position(|n| *n == name) else { return };
    let root = (*layout).root_pane;
    if root.is_null() {
        return;
    }
    let bit = 1usize << index;
    let menu = super::state::MENU_SHOWN.load(Ordering::Acquire);
    let hidden = HIDDEN_BY_MENU.load(Ordering::Acquire) & bit != 0;
    if menu && !hidden {
        super::pane::set_visible(root, false);
        HIDDEN_BY_MENU.fetch_or(bit, Ordering::AcqRel);
    } else if !menu && hidden {
        super::pane::set_visible(root, true);
        HIDDEN_BY_MENU.fetch_and(!bit, Ordering::AcqRel);
    } else if menu {
        // Another writer (the game's own HUD rules) may show it again while the menu is up.
        super::pane::set_visible(root, false);
    }
}