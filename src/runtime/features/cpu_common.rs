//! The CPU hooks that live in the `common` script module (`lua2cpp_common.nro`): DI
//! (`FighterStatusDamage__correctDamageVectorCommon`), the tech status change
//! (`L2CFighterBase::change_status`) and the out-of-shield decision (`L2CFighterCommon::sub_guard_cont`).
//!
//! The module loads after the plugin, so the hooks install when the loader reports it. The notification
//! comes from `libnro_hook`'s `add_nro_load_hook`, looked up by symbol (the playbook's provider pattern,
//! docs/HOOK-SHARING.md): a card without the provider loses these three features and the install line
//! says `cpu_common=not_installed reason=no_nro_hook`. Each function's 64 bytes are compared with the
//! owned 13.0.5 module's (`model::sites_pkg2::COMMON_SITES`) before it is hooked; a mismatch leaves that
//! function alone. One more line in `events.log` says what was installed when the module loaded.
//!
//! ABI (checked on the owned module): the script functions take `lib::L2CValue` arguments by reference
//! and return one through x8; `L2CValue` is 16 bytes plus the one-byte tail the `smash` crate's
//! declaration uses to make the compiler follow that convention. NOT YET SWITCH-RUN.

use std::fmt::Write as _;
use std::sync::atomic::{AtomicUsize, Ordering};

use skyline::hooks::A64HookFunction;
use skyline::nn::ro::{LookupModuleSymbol, LookupSymbol};

use crate::model::guard::GUARD_BYTES;
use crate::model::sites_pkg2::COMMON_SITES;
use crate::model::storage::FixedPath;
use crate::runtime::alloc::HookScope;
use crate::runtime::features::cpu;
use crate::runtime::{clock, install};

/// `lib::L2CValue` as the scripts pass it.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct L2CValue {
    ty: u32,
    unk1: u32,
    inner: u64,
    tail: u8,
}

const L2C_BOOL: u32 = 1;
const L2C_INT: u32 = 2;

impl L2CValue {
    fn int(value: i32) -> Self {
        Self { ty: L2C_INT, unk1: 0, inner: value as i64 as u64, tail: 0 }
    }

    fn boolean(value: bool) -> Self {
        Self { ty: L2C_BOOL, unk1: 0, inner: u64::from(value), tail: 0 }
    }

    /// The integer inside an int value.
    fn as_int(&self) -> Option<i32> {
        (self.ty == L2C_INT).then_some(self.inner as i32)
    }

    fn as_bool(&self) -> bool {
        self.ty == L2C_BOOL && self.inner & 1 != 0
    }
}

// The trampolines of the three hooks and the address of the called `is_enable_passive`.
static ORIGINAL_DI: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL_CHANGE_STATUS: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL_SUB_GUARD_CONT: AtomicUsize = AtomicUsize::new(0);
static IS_ENABLE_PASSIVE: AtomicUsize = AtomicUsize::new(0);

type DiFn = extern "C" fn(usize, L2CValue) -> L2CValue;
type ChangeStatusFn = extern "C" fn(usize, L2CValue, L2CValue) -> L2CValue;
type SubGuardContFn = extern "C" fn(usize) -> L2CValue;
type IsEnablePassiveFn = extern "C" fn(usize) -> L2CValue;

extern "C" fn di_hook(fighter: usize, value: L2CValue) -> L2CValue {
    let _scope = HookScope::enter();
    // SAFETY: `fighter` is the script object the game is running; the accessor is only used when it is
    // a registered CPU body.
    unsafe { cpu::di_before(cpu::script_accessor(fighter)) };
    // SAFETY: set by `install` from the trampoline Skyline returned.
    let original: DiFn = unsafe { std::mem::transmute(ORIGINAL_DI.load(Ordering::Acquire)) };
    original(fighter, value)
}

extern "C" fn change_status_hook(fighter: usize, status: L2CValue, unk: L2CValue) -> L2CValue {
    let _scope = HookScope::enter();
    let mut status = status;
    let mut unk = unk;
    if let Some(requested) = status.as_int() {
        let passive = || {
            let address = IS_ENABLE_PASSIVE.load(Ordering::Acquire);
            // SAFETY: the function passed its guard; it takes the script object and returns a value.
            address != 0 && unsafe { std::mem::transmute::<usize, IsEnablePassiveFn>(address)(fighter) }.as_bool()
        };
        // SAFETY: as in `di_hook`.
        if let Some(new_status) = unsafe { cpu::tech_request(cpu::script_accessor(fighter), requested, passive) } {
            status = L2CValue::int(new_status);
            unk = L2CValue::boolean(true);
        }
    }
    // SAFETY: set by `install`.
    let original: ChangeStatusFn = unsafe { std::mem::transmute(ORIGINAL_CHANGE_STATUS.load(Ordering::Acquire)) };
    original(fighter, status, unk)
}

extern "C" fn sub_guard_cont_hook(fighter: usize) -> L2CValue {
    let _scope = HookScope::enter();
    // SAFETY: set by `install`.
    let original: SubGuardContFn = unsafe { std::mem::transmute(ORIGINAL_SUB_GUARD_CONT.load(Ordering::Acquire)) };
    let result = original(fighter);
    // SAFETY: as in `di_hook`.
    if let Some(escape) = unsafe { cpu::oos_after(cpu::script_accessor(fighter)) } {
        // The status change goes through the script object's own change_status (the hook's original).
        let change = ORIGINAL_CHANGE_STATUS.load(Ordering::Acquire);
        if change != 0 {
            // SAFETY: set by `install` from the trampoline Skyline returned.
            let change: ChangeStatusFn = unsafe { std::mem::transmute(change) };
            change(fighter, L2CValue::int(escape), L2CValue::boolean(true));
        }
    }
    result
}

/// The nro-load provider's registration function: it takes the callback (`fn(&NroInfo)`, a Rust-ABI
/// function pointer) as one pointer-sized argument.
type AddNroLoadHook = extern "C" fn(usize);

/// Ask the provider to tell us when the `common` module loads. `None` when the hook is registered (the
/// install happens at load), otherwise the install line's reason.
pub fn register() -> Option<&'static str> {
    let mut address = 0usize;
    // SAFETY: the name is a NUL-terminated literal; the loader writes one address.
    unsafe { LookupSymbol(&mut address, b"add_nro_load_hook\0".as_ptr()) };
    if address == 0 {
        return Some("no_nro_hook");
    }
    // SAFETY: the provider exports `add_nro_load_hook(callback)` (docs/HOOK-SHARING.md).
    let add: AddNroLoadHook = unsafe { std::mem::transmute(address) };
    add(on_nro_load as *const () as usize);
    Some("awaiting_common_nro")
}

fn is_common(name: &str) -> bool {
    name == "common" || name == "lua2cpp_common" || name.ends_with("lua2cpp_common.nro")
}

extern "Rust" fn on_nro_load(info: &skyline::nro::NroInfo) {
    if is_common(info.name) {
        install(info);
    }
}

/// Compare each function's 64 bytes with the owned module's; only when every one matches are the hooks
/// written (all or nothing: the three hooks share state). One line says what happened.
fn install(info: &skyline::nro::NroInfo) {
    let mut line = FixedPath::<640>::new();
    let _ = write!(line, "{} plugin=training_rooms/{} cpu_common_load module={}", clock::uptime_ms(), env!("CARGO_PKG_VERSION"), info.name);
    let mut addresses = [0usize; COMMON_SITES.len()];
    let mut all = true;
    for (slot, site) in addresses.iter_mut().zip(&COMMON_SITES) {
        let mut symbol = [0u8; 160];
        let name = site.symbol.as_bytes();
        let mut address = 0usize;
        if name.len() < symbol.len() {
            symbol[..name.len()].copy_from_slice(name);
            // SAFETY: NUL-terminated stack buffer; the module pointer is the loader's.
            unsafe { LookupModuleSymbol(&mut address, info.module as *const _ as *const _, symbol.as_ptr()) };
        }
        let usable = address != 0 && {
            let mut window = [0u8; GUARD_BYTES];
            // SAFETY: the address is a function the loader just resolved inside the module's text.
            unsafe { std::ptr::copy_nonoverlapping(address as *const u8, window.as_mut_ptr(), GUARD_BYTES) };
            site.outcome(&window).usable()
        };
        let _ = write!(line, " {}={}", site.name, if usable { "guard_ok" } else if address == 0 { "missing" } else { "guard_refused" });
        *slot = if usable { address } else { 0 };
        all &= usable;
    }
    if all {
        // The called function first, so a hook that fires the moment it is written finds it.
        for (site, address) in COMMON_SITES.iter().zip(addresses) {
            if !site.hooked {
                IS_ENABLE_PASSIVE.store(address, Ordering::Release);
            }
        }
        for (site, address) in COMMON_SITES.iter().zip(addresses) {
            if !site.hooked {
                continue;
            }
            let (replacement, original): (usize, &AtomicUsize) = match site.name {
                "common_di_hook" => (di_hook as *const () as usize, &ORIGINAL_DI),
                "common_change_status_hook" => (change_status_hook as *const () as usize, &ORIGINAL_CHANGE_STATUS),
                _ => (sub_guard_cont_hook as *const () as usize, &ORIGINAL_SUB_GUARD_CONT),
            };
            let mut trampoline: *mut std::ffi::c_void = std::ptr::null_mut();
            // SAFETY: `address` passed its guard and is a function entry; the replacement has the same
            // signature as the original.
            unsafe { A64HookFunction(address as *const _, replacement as *const _, &mut trampoline) };
            original.store(trampoline as usize, Ordering::Release);
        }
    }
    let _ = write!(line, " cpu_common={}", if all { "installed" } else { "not_installed" });
    install::write_text(line.as_str());
}
