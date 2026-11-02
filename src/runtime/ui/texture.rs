//! A game-file texture on a picture pane, through the game's own UI texture loader (stream u4; SPEC-core-07 §4.5, §4.6,
//! decisions "read at run time, never baked").
//!
//! The plugin never reads, decodes or uploads a BNTX. It asks the game for the texture of a file path: the game's file-resource
//! service (the layer ARCropolis hooks, so a mod's replacement file is what loads) loads the file, and the game's UI texture update
//! builds the GPU texture, its view and its registered descriptor slot. What the plugin owns is one `shared_ptr` reference to the
//! game's UI texture, taken by `load` and dropped by `release` (the game's own `release` sequence). `bind` points a picture pane's
//! TexMap at the texture's embedded texture-info with the game's own override function (which keeps the pane's previous texture),
//! `unbind` restores it with the game's reset function.
//!
//! Threads: every call runs on the UI job thread, inside the layout tick, exactly where the game's own UI code makes the same calls.
//! Nothing here allocates on the plugin's heap (the game's allocator makes the UI texture), so the tick stays at `heap_allocations=0`.
//! A texture is released a few frames after its pane stopped using it (`retire` and `tick`): the GPU may still read the last
//! frame's descriptor.
//!
//! Site guards (every called function and relied instruction, 64 bytes each): `sites_u4::SITES`, compared by `install`.

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering};

use hos_lock::Mutex;
use skyline::nn::ui2d::Pane;

use crate::model::guard::GuardOutcome;
use crate::model::hash40::hash40;
use crate::model::sites_u4::{self, SITES};
use crate::model::sites_u4b;
use crate::model::sites_ui::decode_adrp_ldr;
use crate::model::ui_texture::{self as facts, RetireQueue, TextureFacts};
use crate::runtime::diag_log::diagnostic;
use crate::runtime::game;

/// Every guard passed; set by `install`.
static USABLE: AtomicBool = AtomicBool::new(false);
/// Absolute address of the file-resource service's global slot, decoded from the lookup's own `adrp`/`ldr` words.
static SERVICE_SLOT_ADDRESS: AtomicUsize = AtomicUsize::new(0);
/// The UI frame counter `tick` advances.
static FRAME: AtomicU64 = AtomicU64::new(0);

/// Frames a retired texture waits after it was unbound.
pub use facts::RETIRE_DELAY_FRAMES;
use facts::RETIRE_SLOTS;
static RETIRED: Mutex<RetireQueue<Texture, RETIRE_SLOTS>> = Mutex::new(RetireQueue::new(RETIRE_DELAY_FRAMES));

/// One reference to the game's UI texture: the shared pointer's holder and control block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Texture {
    holder: usize,
    control: usize,
}

/// Where a loaded texture stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Readiness {
    /// The file is loading or the texture is being built: poll again next frame.
    Loading,
    /// The texture view is built and its descriptor registered: bind it.
    Ready,
}

type FindHandle = extern "C" fn(*mut u32, u64);
type Make = extern "C" fn(*mut usize, *const u32);
type Poll = extern "C" fn(*mut u8);
type ReleaseHandle = extern "C" fn(*mut u8, u32);
type Bind = extern "C" fn(*mut Pane, *const u8, i32);
type Clear = extern "C" fn(*mut Pane);
type ReleaseWeak = extern "C" fn(*mut u8);
type OnZero = extern "C" fn(*mut u8);
/// `register(u64* slot_out, texture_view*, user) -> bool` (`sites_u4b::UI_REGISTER_VIEW_OFFSET`).
type Register = extern "C" fn(*mut u64, *const u8, usize) -> bool;

fn at(offset: usize) -> usize {
    game::text_base() + offset
}

/// Compare every row of `sites_u4::SITES` with the bytes now in the game; the helper is usable only when all pass.
pub fn install() -> [GuardOutcome; SITES.len()] {
    let mut found = [GuardOutcome::Stock; SITES.len()];
    for (slot, site) in found.iter_mut().zip(SITES.iter()) {
        *slot = site.outcome(&game::read_window(site.offset));
    }
    let all = found.iter().all(|o| o.usable());
    let mut slot = 0usize;
    if all {
        // Words 10 and 11 of the lookup's window are the `adrp`/`ldr` pair that names the service slot.
        let window = game::read_window(sites_u4::UI_TEX_FIND_HANDLE_OFFSET);
        let first = sites_u4::UI_TEX_SERVICE_SLOT_WORDS_OFFSET - sites_u4::UI_TEX_FIND_HANDLE_OFFSET;
        let word = |at: usize| u32::from_le_bytes([window[at], window[at + 1], window[at + 2], window[at + 3]]);
        let (adrp, ldr) = (word(first), word(first + 4));
        let pc = at(sites_u4::UI_TEX_SERVICE_SLOT_WORDS_OFFSET) as u64;
        // The slot must be the one the constant names (the decode is position independent).
        let want = at(facts::SERVICE_SLOT) as u64;
        slot = match decode_adrp_ldr(pc, adrp, ldr) {
            Some(address) if address == want => address as usize,
            _ => 0,
        };
    }
    SERVICE_SLOT_ADDRESS.store(slot, Ordering::Release);
    USABLE.store(all && slot != 0, Ordering::Release);
    diagnostic(format_args!("[Texture] install usable={} guards={found:?} service_slot={slot:#x}", all && slot != 0));
    found
}

pub fn usable() -> bool {
    // R2-1: also every function the helper calls must be granted by the boot (the native-call layer's rule); `install` compares the
    // same rows again later, so this is the boot's answer for them.
    USABLE.load(Ordering::Acquire) && SITES.iter().all(|site| game::granted(site.offset))
}

/// The path's file-resource hash (`hash40` of the archive-relative path, lower case, no mount prefix).
pub fn path_hash(path: &str) -> u64 {
    hash40(path)
}

/// Ask the game for the texture of the file with this hash40: `None` when the helper is not usable or the file is not in the
/// game's resource table. The file loads in the background; poll with `poll` until it is ready.
///
/// # Safety
/// On the UI job thread, inside the layout tick.
pub unsafe fn load(hash: u64) -> Option<Texture> {
    if !usable() {
        return None;
    }
    let find: FindHandle = core::mem::transmute::<usize, FindHandle>(at(sites_u4::UI_TEX_FIND_HANDLE_OFFSET));
    let make: Make = core::mem::transmute::<usize, Make>(at(sites_u4::UI_TEX_MAKE_OFFSET));
    let mut handle = facts::HANDLE_NONE;
    find(&mut handle, hash);
    if handle == facts::HANDLE_NONE {
        return None;
    }
    // The lookup took a reference on the handle; the UI texture takes its own, so ours is dropped once it exists (the game's loader does
    // the same).
    let mut pair = [0usize; 2];
    make(pair.as_mut_ptr(), &handle);
    if let Some(service) = service() {
        let release_handle: ReleaseHandle = core::mem::transmute::<usize, ReleaseHandle>(at(sites_u4::UI_TEX_RELEASE_HANDLE_OFFSET));
        release_handle(service, handle);
    }
    let texture = Texture { holder: pair[0], control: pair[1] };
    if texture.holder == 0 || texture.control == 0 || texture.object().is_null() {
        // A pair the game did not build the way the guards say: leave it (never call through an unknown vtable).
        return None;
    }
    // The holder and the control block must carry the game's own vtables before anything is called through them.
    if read_usize(texture.holder) != at(facts::HOLDER_VTABLE) || read_usize(texture.control) != at(facts::CONTROL_VTABLE) {
        return None;
    }
    Some(texture)
}

/// A UI texture over a texture view the game already built (stream u4b: the DI / SDI effect textures, which live in `ef_common.eff`'s
/// `BNTX`, built in place by the particle system). The game's own constructor makes the UI texture with no file handle (the
/// constructor's `0xffffff` case: nothing is queued, nothing is loaded), the game's own registration function puts the view in the
/// shared descriptor pool for this texture (the call the UI texture update makes for a file texture), and the three fields the update
/// would have written are written here: slot (by the registration), size and the "registered" byte, so the game's destroy releases the
/// slot. `None` when the helper is not usable or the pool is full (nothing is left behind).
///
/// # Safety
/// On the UI job thread, inside the layout tick; `view` is a live texture view of the game (the effect resource's) that outlives the
/// returned texture, and `width` and `height` are its texture's size.
pub unsafe fn load_resident(view: usize, width: u16, height: u16) -> Option<Texture> {
    if !usable() || !game::granted(sites_u4b::UI_REGISTER_VIEW_OFFSET) || view == 0 {
        return None;
    }
    let make: Make = core::mem::transmute::<usize, Make>(at(sites_u4::UI_TEX_MAKE_OFFSET));
    let register: Register = core::mem::transmute::<usize, Register>(at(sites_u4b::UI_REGISTER_VIEW_OFFSET));
    let none = facts::HANDLE_NONE;
    let mut pair = [0usize; 2];
    make(pair.as_mut_ptr(), &none);
    let texture = Texture { holder: pair[0], control: pair[1] };
    if texture.holder == 0 || texture.control == 0 || texture.object().is_null() {
        return None;
    }
    if read_usize(texture.holder) != at(facts::HOLDER_VTABLE) || read_usize(texture.control) != at(facts::CONTROL_VTABLE) {
        return None;
    }
    let object = texture.object() as usize;
    // The constructor left the texture empty: no handle, no pending job, no state object. Anything else is not the layout the guards say.
    if !facts::is_ready(texture.facts()) {
        release(texture);
        return None;
    }
    let mut slot = facts::SLOT_UNSET;
    let registered = register(&mut slot, view as *const u8, 0);
    if !registered {
        release(texture);
        return None;
    }
    ((object + facts::TEXTURE_SLOT) as *mut u64).write_unaligned(slot);
    ((object + facts::TEXTURE_WIDTH) as *mut u16).write_unaligned(width);
    ((object + facts::TEXTURE_HEIGHT) as *mut u16).write_unaligned(height);
    ((object + facts::TEXTURE_REGISTERED) as *mut u8).write_unaligned(1);
    Some(texture)
}

/// The file-resource service object (`*main+0x5332f20`), or `None` before the game created it.
unsafe fn service() -> Option<*mut u8> {
    let slot = SERVICE_SLOT_ADDRESS.load(Ordering::Acquire) as *const *mut u8;
    if slot.is_null() {
        return None;
    }
    let service = slot.read_volatile();
    (!service.is_null()).then_some(service)
}

unsafe fn read_usize(address: usize) -> usize {
    (address as *const usize).read_unaligned()
}

impl Texture {
    /// The shared pointer's two words, for a holder that keeps it in atomics (the chips page: no lock on the UI thread).
    pub fn to_words(self) -> (usize, usize) {
        (self.holder, self.control)
    }

    /// The texture `to_words` gave, or `None` for the empty pair.
    ///
    /// # Safety
    /// The words come from `to_words` of a texture that was not released since.
    pub unsafe fn from_words(holder: usize, control: usize) -> Option<Texture> {
        (holder != 0 && control != 0).then_some(Texture { holder, control })
    }

    /// The game's 0x68-byte UI texture object (`[holder + 8]`).
    fn object(&self) -> *mut u8 {
        // SAFETY: the holder was built by the game's constructor (checked by `load`) and lives while the shared pointer does.
        unsafe { read_usize(self.holder + facts::HOLDER_TEXTURE) as *mut u8 }
    }

    /// The texture-info a picture pane's TexMap takes (the object's embedded descriptor).
    pub fn descriptor(&self) -> *const u8 {
        self.object().wrapping_add(facts::TEXTURE_DESCRIPTOR)
    }

    /// The fields the game's own bind reads, for the readiness test and the diagnostics.
    pub unsafe fn facts(&self) -> TextureFacts {
        let object = self.object() as usize;
        let state = read_usize(object + facts::TEXTURE_STATE);
        TextureFacts {
            handle: (object as *const u32).read_unaligned(),
            job: read_usize(object + facts::TEXTURE_JOB) as u64,
            has_state: state != 0,
            state: if state != 0 { ((state + facts::STATE_FIELD) as *const u32).read_unaligned() } else { 0 },
        }
    }

    /// Run the game's UI texture update (it builds the GPU texture once the file is loaded) and report whether it is ready to bind.
    ///
    /// # Safety
    /// On the UI job thread, inside the layout tick.
    pub unsafe fn poll(&self) -> Readiness {
        if !usable() {
            return Readiness::Loading;
        }
        let poll: Poll = core::mem::transmute::<usize, Poll>(at(sites_u4::UI_TEX_POLL_OFFSET));
        poll(self.object());
        if facts::is_ready(self.facts()) {
            Readiness::Ready
        } else {
            Readiness::Loading
        }
    }
}

/// Point a picture pane's TexMap 0 at a ready texture. Returns false (and changes nothing) when it is not ready.
///
/// # Safety
/// On the UI job thread; `pane` is a live picture pane with at least one TexMap, and the texture stays alive until `unbind`.
pub unsafe fn bind(pane: *mut Pane, texture: &Texture) -> bool {
    if !usable() || pane.is_null() || !facts::is_ready(texture.facts()) {
        return false;
    }
    let bind: Bind = core::mem::transmute::<usize, Bind>(at(sites_u4::UI_TEX_BIND_OFFSET));
    bind(pane, texture.descriptor(), 0);
    true
}

/// Restore the pane's own texture (the game's override reset: both indices, and its cached shared texture).
///
/// # Safety
/// On the UI job thread; `pane` is the pane `bind` was called with, still alive.
pub unsafe fn unbind(pane: *mut Pane) {
    if !usable() || pane.is_null() {
        return;
    }
    let clear: Clear = core::mem::transmute::<usize, Clear>(at(sites_u4::UI_TEX_CLEAR_OFFSET));
    clear(pane);
}

/// Drop the plugin's reference at once: the game's shared-pointer release (decrement; the last owner runs `on_zero_shared`, which
/// frees the GPU texture, the descriptor slot and the loaded copy, then `__release_weak`). The pane must have been unbound first
/// and the GPU done with it: use `retire`.
///
/// # Safety
/// On the UI job thread; `texture` is not used again.
pub unsafe fn release(texture: Texture) {
    if !usable() || read_usize(texture.control) != at(facts::CONTROL_VTABLE) {
        return;
    }
    let count = &*((texture.control + facts::CONTROL_COUNT) as *const AtomicI64);
    if count.fetch_sub(1, Ordering::AcqRel) != 0 {
        return;
    }
    let on_zero: OnZero = core::mem::transmute::<usize, OnZero>(read_usize(read_usize(texture.control) + facts::CONTROL_ON_ZERO));
    on_zero(texture.control as *mut u8);
    let release_weak: ReleaseWeak = core::mem::transmute::<usize, ReleaseWeak>(at(sites_u4::UI_TEX_RELEASE_WEAK_OFFSET));
    release_weak(texture.control as *mut u8);
}

/// Queue an unbound texture for release after `RETIRE_DELAY_FRAMES` frames of `tick`. False when the queue is full (the caller
/// releases it itself or keeps it).
pub fn retire(texture: Texture) -> bool {
    RETIRED.lock().push(texture, FRAME.load(Ordering::Acquire))
}

/// One UI frame: advance the clock and release the retired textures whose delay passed. Call once per frame from the layout tick
/// (one layout's tick, on the UI job thread).
///
/// # Safety
/// On the UI job thread, inside the layout tick.
pub unsafe fn tick() {
    if !usable() {
        return;
    }
    let now = FRAME.fetch_add(1, Ordering::AcqRel) + 1;
    loop {
        let due = RETIRED.lock().pop_due(now);
        match due {
            Some(texture) => release(texture),
            None => break,
        }
    }
}

/// How many textures wait for their release (diagnostics and tests of the page lifecycle).
pub fn retired_waiting() -> usize {
    RETIRED.lock().len()
}
