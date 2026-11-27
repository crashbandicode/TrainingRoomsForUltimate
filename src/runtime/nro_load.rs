//! TrainingRooms' own module-load observation (no provider plugin): an inline hook on the success path of the game's only `nn::ro::LoadModule` call (`sites_pkg2::NRO_LOADED_OFFSET`). It hands the module's name and the loader's `Module` to the `common` consumer (`cpu_common`) and the `item` consumer (the roster's turnip identity). Nothing here or below it waits on a plugin thread's lock or allocates.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::model::{nro_load, sites_pkg2};
use crate::runtime::alloc::HookScope;

static INSTALLED: AtomicBool = AtomicBool::new(false);

/// Whether boot installed the hook (its site's guard passed).
pub fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

/// Boot calls this right after it installs the hook.
pub fn mark_installed() {
    INSTALLED.store(true, Ordering::Release);
}

/// At the site x25 is the `nn::ro::Module` the game just loaded (`LoadModule` returned 0); the game marks it loaded after this.
#[skyline::hook(offset = sites_pkg2::NRO_LOADED_OFFSET, inline)]
pub unsafe fn nro_loaded_hook(ctx: &mut skyline::hooks::InlineCtx) {
    let _scope = HookScope::enter();
    let module = ctx.registers[25].x() as *mut skyline::nn::ro::Module;
    if module.is_null() {
        return;
    }
    let field = &*core::ptr::addr_of!((*module).Name);
    let Some(name) = nro_load::module_name(field) else { return };
    crate::runtime::features::cpu_common::on_nro_load(name, module);
    if crate::runtime::roster::running() {
        crate::runtime::roster::on_nro_load(name, module);
    }
}
