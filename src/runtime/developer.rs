//! Shipped hidden Developer-page debug behavior (SPEC-core-01, N1).
//! Boot checks the complete closure before installing; callbacks never do I/O.
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use crate::model::cpu::hash::lua_bind_hash;
use crate::model::developer_debug::{self, DEBUG_SET_FLOAT_OFFSET, SITES};
use crate::model::guard::GuardOutcome;
use crate::model::install_line::{SiteReport, Written};
use crate::model::settings::Snapshot;
use crate::runtime::alloc::{HookId, HookScope};
use crate::runtime::{game, published, scene};

static INSTALLED: AtomicBool = AtomicBool::new(false);
static DEBUG_ON: AtomicBool = AtomicBool::new(false);
static KIRBY: AtomicI32 = AtomicI32::new(-1);

pub fn publish(snapshot: &Snapshot) {
    use crate::model::developer_ini::{control_enabled, Control};
    DEBUG_ON.store(control_enabled(snapshot, published::developer_page(), Control::DebugHooks), Ordering::Release);
}

/// Before any hook writes a neighboring WorkModule window.
pub fn check() -> [GuardOutcome; 2] {
    std::array::from_fn(|i| SITES[i].outcome(&game::read_window(SITES[i].offset)))
}

pub fn install(page_enabled: bool, closure_usable: bool) -> bool {
    if !page_enabled || !closure_usable { return false; }
    skyline::install_hooks!(debug_set_float);
    INSTALLED.store(true, Ordering::Release);
    true
}

pub fn reports(found: &[GuardOutcome; 2], installed: bool) -> [SiteReport; 2] {
    std::array::from_fn(|i| SiteReport {
        site: SITES[i].name, found: found[i],
        written: if i == 0 && installed { Written::Far } else if i == 1 && found[i].usable() {
            Written::Checked
        } else { Written::Skipped },
    })
}

unsafe fn eligible(accessor: usize) -> bool {
    if !INSTALLED.load(Ordering::Acquire) || !DEBUG_ON.load(Ordering::Acquire)
        || !scene::is_training() || !game::natives::is_fighter(accessor) {
        return false;
    }
    let mut kirby = KIRBY.load(Ordering::Acquire);
    if kirby < 0 {
        let Some(value) = game::lua_const(lua_bind_hash("FIGHTER_KIND_KIRBY")) else { return false };
        if value < 0 { return false; }
        KIRBY.store(value, Ordering::Release);
        kirby = value;
    }
    game::natives::fighter_kind(accessor) == Some(kirby)
}

unsafe fn observe(accessor: usize) {
    // Existing diag hook statistics distinguish an actual probe from entry-only coverage.
    #[cfg(feature = "diag")]
    let _probe = HookScope::enter_hook(HookId::DeveloperBackshield);
    let _ = game::natives::backshield_visible(accessor);
}

#[skyline::hook(offset = DEBUG_SET_FLOAT_OFFSET)]
unsafe fn debug_set_float(accessor: usize, value: f32, id: i32) {
    let _scope = HookScope::enter_hook(HookId::DeveloperFloat);
    developer_debug::write_then_observe(
        || original!()(accessor, value, id),
        || eligible(accessor),
        || observe(accessor),
    );
}
