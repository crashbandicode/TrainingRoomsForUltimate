//! Diag only: the development probe of SPEC-core-07 §4.5 unknowns U1-U3 (which route reads a game file with a
//! mod's replacement applied) and V5 (a ranged read of the large effect file). Never in the shipped NRO.
//!
//! ARCropolis (when loaded) exports `arcrop_api_version`, `arcrop_register_event_callback`,
//! `arcrop_get_decompressed_size` and `arcrop_load_file` (audit: `artifacts/enhancements/arc-stage-assets/REPORT.md`).
//! The callback is registered in the plugin's early `main` (events are not replayed); a worker thread waits for
//! `ModFilesystemMounted` and then reads a few known paths through each route and logs what each returned.

use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::model::archive_recipe::fingerprint;
use crate::model::hash40::hash40;
use crate::runtime::diag_log::diagnostic;

static READY: AtomicBool = AtomicBool::new(false);
const MOD_FILESYSTEM_MOUNTED: u32 = 1;

fn lookup(name: &[u8]) -> usize {
    let mut address = 0usize;
    // SAFETY: a plain symbol lookup with a NUL-terminated name.
    let result = unsafe { skyline::nn::ro::LookupSymbol(&mut address, name.as_ptr()) };
    if result == 0 {
        address
    } else {
        0
    }
}

extern "C" fn mounted(event: u32) {
    if event == MOD_FILESYSTEM_MOUNTED {
        READY.store(true, Ordering::Release);
    }
}

type SizeFn = extern "C" fn(u64, *mut usize) -> bool;
type LoadFn = extern "C" fn(u64, *mut u8, usize, *mut usize) -> bool;

pub fn install() {
    let version = lookup(b"arcrop_api_version\0");
    let register = lookup(b"arcrop_register_event_callback\0");
    let size = lookup(b"arcrop_get_decompressed_size\0");
    let load = lookup(b"arcrop_load_file\0");
    if version == 0 || register == 0 || size == 0 || load == 0 {
        diagnostic(format_args!("[Probe] arcropolis exports absent version={version:#x} register={register:#x} size={size:#x} load={load:#x}"));
        return;
    }
    // SAFETY: the exports have the ABI of the audit; the version record is two u32.
    let (major, minor) = unsafe {
        let api: extern "C" fn() -> *const [u32; 2] = core::mem::transmute::<usize, extern "C" fn() -> *const [u32; 2]>(version);
        let v = api();
        if v.is_null() {
            (0, 0)
        } else {
            ((*v)[0], (*v)[1])
        }
    };
    diagnostic(format_args!("[Probe] arcropolis api {major}.{minor}"));
    if major != 1 || minor < 9 {
        return;
    }
    // SAFETY: as above.
    unsafe {
        let register: extern "C" fn(u32, extern "C" fn(u32)) = core::mem::transmute::<usize, extern "C" fn(u32, extern "C" fn(u32))>(register);
        register(MOD_FILESYSTEM_MOUNTED, mounted);
    }
    let _ = std::thread::Builder::new().name("tr-probe".into()).stack_size(256 * 1024).spawn(move || worker(size, load));
}

fn worker(size: usize, load: usize) {
    for _ in 0..1800 {
        if READY.load(Ordering::Acquire) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    if !READY.load(Ordering::Acquire) {
        diagnostic(format_args!("[Probe] ModFilesystemMounted never came"));
        return;
    }
    diagnostic(format_args!("[Probe] ModFilesystemMounted"));
    // SAFETY: the audited ABI.
    let size: SizeFn = unsafe { core::mem::transmute::<usize, SizeFn>(size) };
    let load: LoadFn = unsafe { core::mem::transmute::<usize, LoadFn>(load) };
    for path in [
        "ui/replace/stage/stage_1/stage_1_end.bntx",
        "ui/replace/stage/stage_1/stage_1_battlefield.bntx",
        "ui/replace_patch/stage/stage_1/stage_1_battlefields.bntx",
        "ui/message/msg_name.msbt",
        "effect/system/common/ef_common.eff",
    ] {
        probe_path(path, size, load);
    }
}

fn magic_of(bytes: &[u8]) -> String {
    bytes.iter().take(8).map(|b| if b.is_ascii_graphic() { *b as char } else { '.' }).collect()
}

fn probe_path(path: &str, size: SizeFn, load: LoadFn) {
    let hash = hash40(path);
    let mut reported = 0usize;
    let known = size(hash, &mut reported);
    diagnostic(format_args!("[Probe] path={path} hash={hash:#x} arcrop_size known={known} size={reported}"));
    // U1: arcrop_load_file from this thread (a bounded read; the effect file is far too large to load whole).
    if known && reported > 0 && reported <= 4 << 20 {
        let mut buffer = vec![0u8; reported];
        let mut out = 0usize;
        let ok = load(hash, buffer.as_mut_ptr(), buffer.len(), &mut out);
        diagnostic(format_args!(
            "[Probe] route=arcrop_load_file ok={ok} out={out} magic={} fnv={:016x}",
            magic_of(&buffer),
            fingerprint(&buffer[..out.min(buffer.len())])
        ));
    }
    // U2 and the vanilla view: the two mounts through the standard file API.
    for mount in ["mods:/", "arc:/"] {
        let full = format!("{mount}{path}");
        match std::fs::File::open(&full) {
            Ok(mut file) => {
                let len = file.metadata().map(|m| m.len()).unwrap_or(0);
                let mut head = [0u8; 8];
                let read = file.read(&mut head).unwrap_or(0);
                // V5: a ranged read from the middle of the file.
                let mut ranged = 0usize;
                if len > 0x2000 && file.seek(SeekFrom::Start(0x1000)).is_ok() {
                    let mut chunk = [0u8; 4096];
                    ranged = file.read(&mut chunk).unwrap_or(0);
                }
                diagnostic(format_args!(
                    "[Probe] route={mount} open=ok len={len} magic={} head_read={read} ranged_read={ranged}",
                    magic_of(&head)
                ));
            }
            Err(error) => diagnostic(format_args!("[Probe] route={mount} open=err {error}")),
        }
    }
}