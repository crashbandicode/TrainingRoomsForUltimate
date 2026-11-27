//! The game's successful module-load observation.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::model::{nro_load, sites_pkg2};
use crate::runtime::alloc::HookScope;

static INSTALLED: AtomicBool = AtomicBool::new(false);

pub fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

pub fn mark_installed() {
    INSTALLED.store(true, Ordering::Release);
}

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
