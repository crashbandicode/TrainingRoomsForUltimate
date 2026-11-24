use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU64, Ordering};

use crate::model::move_names::{load_step, FighterId, LoadStep, Names, PUBLISHED};
use crate::model::thread_stacks::ThreadId;
use crate::runtime::{game, threads};

pub mod ids {
    crate::model::cpu::ids::game_constants! {
        Neutral = "FIGHTER_INSTANCE_WORK_ID_INT_CUSTOMIZE_SPECIAL_N_NO",
        Side = "FIGHTER_INSTANCE_WORK_ID_INT_CUSTOMIZE_SPECIAL_S_NO",
        Up = "FIGHTER_INSTANCE_WORK_ID_INT_CUSTOMIZE_SPECIAL_HI_NO",
        Down = "FIGHTER_INSTANCE_WORK_ID_INT_CUSTOMIZE_SPECIAL_LW_NO",
    }
}

static LAST_KIND: AtomicI32 = AtomicI32::new(-1);
static ARCROPOLIS: AtomicBool = AtomicBool::new(false);
static MOUNTED: AtomicBool = AtomicBool::new(false);
static GIVE_UP: AtomicBool = AtomicBool::new(false);
static ATTEMPTS: AtomicU8 = AtomicU8::new(0);
static LAST_TRY: AtomicU64 = AtomicU64::new(0);
const MOD_FILESYSTEM_MOUNTED: u32 = 1;
const PATH: &[u8] = b"arc:/ui/message/msg_name.msbt\0";

fn lookup(name: &[u8]) -> usize {
    let mut address = 0usize;
    // SAFETY: names passed here are static and NUL-terminated.
    if unsafe { skyline::nn::ro::LookupSymbol(&mut address, name.as_ptr()) } == 0 { address } else { 0 }
}

extern "C" fn mounted(event: u32) {
    if event == MOD_FILESYSTEM_MOUNTED {
        MOUNTED.store(true, Ordering::Release);
    }
}

pub fn register() {
    ARCROPOLIS.store(false, Ordering::Release);
    let version = lookup(b"arcrop_api_version\0");
    let register = lookup(b"arcrop_register_event_callback\0");
    if version == 0 || register == 0 { return; }
    // SAFETY: the two exports have the ABI used by the ARCropolis probe.
    let (major, minor) = unsafe {
        let api: extern "C" fn() -> *const [u32; 2] = core::mem::transmute::<usize, extern "C" fn() -> *const [u32; 2]>(version);
        let version = api();
        if version.is_null() { (0, 0) } else { ((*version)[0], (*version)[1]) }
    };
    if major != 1 || minor < 9 { return; }
    // SAFETY: the registered callback and event use the audited ARCropolis ABI.
    unsafe {
        let register: extern "C" fn(u32, extern "C" fn(u32)) = core::mem::transmute::<usize, extern "C" fn(u32, extern "C" fn(u32))>(register);
        register(MOD_FILESYSTEM_MOUNTED, mounted);
    }
    ARCROPOLIS.store(true, Ordering::Release);
}

/// Called by the menu worker, outside its locks. True while a retry is due.
pub fn service() -> bool {
    if GIVE_UP.load(Ordering::Acquire) { return false; }
    let arcropolis = ARCROPOLIS.load(Ordering::Acquire).then(|| MOUNTED.load(Ordering::Acquire));
    let attempts = ATTEMPTS.load(Ordering::Acquire);
    let now = crate::runtime::clock::uptime_ms();
    match load_step(PUBLISHED.is_on(), PUBLISHED.table().is_some(), arcropolis, false, attempts, now.saturating_sub(LAST_TRY.load(Ordering::Acquire))) {
        LoadStep::Idle => false,
        LoadStep::Wait => true,
        LoadStep::GiveUp => {
            GIVE_UP.store(true, Ordering::Release);
            false
        }
        LoadStep::Try => {
            ATTEMPTS.store(attempts + 1, Ordering::Release);
            let result = threads::run_transient(ThreadId::MoveNames, load);
            LAST_TRY.store(crate::runtime::clock::uptime_ms(), Ordering::Release);
            #[cfg(feature = "diag")]
            match &result {
                Err(error) => crate::runtime::diag_log::diagnostic(format_args!("[MoveNames] thread failed: {error}")),
                Ok(Err(error)) => crate::runtime::diag_log::diagnostic(format_args!("[MoveNames] load failed: {error:?}")),
                Ok(Ok(())) => {}
            }
            let failed = matches!(&result, Ok(Err(LoadError::Malformed)));
            let published = matches!(&result, Ok(Ok(())));
            match load_step(true, published, arcropolis, failed, attempts + 1, 0) {
                LoadStep::Idle => false,
                LoadStep::GiveUp => {
                    GIVE_UP.store(true, Ordering::Release);
                    false
                }
                LoadStep::Wait | LoadStep::Try => true,
            }
        }
    }
}

#[derive(Debug)]
enum LoadError { Unreadable, Malformed }

#[inline(never)]
fn load() -> Result<(), LoadError> {
    #[cfg(feature = "diag")]
    let started = crate::runtime::clock::uptime_ms();
    let mut handle = skyline::nn::fs::FileHandle { handle: 0 };
    // SAFETY: PATH is static and NUL-terminated; mode 1 is read.
    let opened = unsafe { skyline::nn::fs::OpenFile(&mut handle, PATH.as_ptr(), 1) };
    if opened != 0 || handle.handle == 0 { return Err(LoadError::Unreadable); }
    let read = (|| {
        let mut header = [0u8; 32];
        read_exact(handle, 0, &mut header)?;
        let length = u32::from_le_bytes(header[0x12..0x16].try_into().map_err(|_| LoadError::Malformed)?) as usize;
        if !(32..=2 * 1024 * 1024).contains(&length) { return Err(LoadError::Malformed); }
        let mut bytes = vec![0; length];
        bytes[..32].copy_from_slice(&header);
        read_exact(handle, 32, &mut bytes[32..])?;
        Ok(bytes)
    })();
    // SAFETY: this handle was opened above and is closed exactly once.
    unsafe { skyline::nn::fs::CloseFile(handle) };
    let bytes = read?;
    #[cfg(feature = "diag")]
    let file_bytes = bytes.len();
    let names = Names::build(&bytes).map_err(|_| LoadError::Malformed)?;
    drop(bytes);
    #[cfg(feature = "diag")]
    crate::runtime::diag_log::diagnostic(format_args!(
        "[MoveNames] loaded bytes={file_bytes} names={} kept={} ms={}",
        names.entry_count(), names.bytes(), crate::runtime::clock::uptime_ms().saturating_sub(started)
    ));
    PUBLISHED.publish(Box::new(names));
    Ok(())
}

fn read_exact(handle: skyline::nn::fs::FileHandle, offset: usize, buffer: &mut [u8]) -> Result<(), LoadError> {
    let mut filled = 0usize;
    while filled < buffer.len() {
        let mut got = 0u64;
        // SAFETY: the destination is the remaining part of the buffer; reads progress forward from offset.
        let result = unsafe {
            skyline::nn::fs::ReadFile1(&mut got, handle, (offset + filled) as i64, buffer[filled..].as_mut_ptr(), (buffer.len() - filled) as u64)
        };
        let got = usize::try_from(got).map_err(|_| LoadError::Unreadable)?;
        if result != 0 || got == 0 || got > buffer.len() - filled { return Err(LoadError::Unreadable); }
        filled += got;
    }
    Ok(())
}

/// Record P1's kind and identity without allocating or taking a lock.
///
/// # Safety
/// `accessor` is Player 1's live module accessor on the game thread.
pub unsafe fn note_player(accessor: usize, kind: Option<i32>, cleared: bool) {
    let Some(kind) = kind.filter(|kind| *kind >= 0) else {
        LAST_KIND.store(-1, Ordering::Release);
        PUBLISHED.set_player(None);
        return;
    };
    if !cleared && LAST_KIND.load(Ordering::Acquire) == kind { return; }
    // SAFETY: the roster function checks its published seam before calling it.
    let Some(chara) = (unsafe { crate::runtime::roster::character_roster::ui_chara_hash(kind as u32) }) else {
        LAST_KIND.store(-1, Ordering::Release);
        PUBLISHED.set_player(None);
        return;
    };
    let mut table = ids::Table::unresolved();
    // SAFETY: the lookup checks its own guard.
    table.resolve(|key| unsafe { game::lua_const(key) });
    let specials = [ids::C::Neutral, ids::C::Side, ids::C::Up, ids::C::Down].map(|id| {
        if !table.resolved(id) { return 0; }
        // SAFETY: the accessor is live and the native checks its own guard.
        let value = unsafe { game::natives::get_int(accessor, table.get(id)) };
        if (0..=2).contains(&value) { value as u8 } else { 0 }
    });
    PUBLISHED.set_player(Some((kind, FighterId { chara, specials })));
    LAST_KIND.store(kind, Ordering::Release);
}
