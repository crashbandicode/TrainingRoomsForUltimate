//! The native fighter-entry install at main+0x653240, and its staged descriptor.
//!
//! The native routine synchronously copies a 0x70-byte, 8-byte-aligned descriptor and
//! three flag bytes; it retains neither. It is reached through its text address, which
//! `character_lifecycle::reload_trace` hooks inline, never through a trampoline. This
//! module holds values only: no hook, lock, log, allocation or retained native address.
//! It depends on `std` alone so `rustc --test` can check it outside the NRO.

use std::mem::{align_of, size_of};

pub(crate) const DESCRIPTOR_LEN: usize = 0x70;
const INSTALL_ENTRY: usize = 0x653240;
const COUNT: usize = 0x14;
const KINDS: usize = 0x18;
const ACTOR_COUNT: usize = 0x28;
const STARTING_FORM: usize = 0x43;

/// Local staging copy of a native fighter-entry descriptor. Same size and alignment as
/// the `[u64; 14]` buffer the native install reads.
#[repr(C, align(8))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Descriptor([u8; DESCRIPTOR_LEN]);

const _: () = assert!(size_of::<Descriptor>() == DESCRIPTOR_LEN && align_of::<Descriptor>() == 8);

/// Canonical group written over a copied descriptor; the fields of `Forms`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Group {
    pub count: u32,
    pub kinds: [u32; 3],
    pub actor_count: u32,
}

/// The flag bytes the existing callers pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Flags {
    /// Practice player/CPU reload of the requested group: `[1, 0, 0]`.
    Reload,
    /// Non-activating helper/source install: `[0, 0, 0]`.
    Parked,
}

impl Flags {
    #[inline]
    fn bytes(self) -> [u8; 3] {
        match self {
            Self::Reload => [1, 0, 0],
            Self::Parked => [0, 0, 0],
        }
    }
}

impl Descriptor {
    /// Copy the complete descriptor at the start of a native fighter entry.
    ///
    /// # Safety
    /// `entry` is the current callback's live native entry, readable for 0x70 bytes.
    #[inline]
    pub(crate) unsafe fn copy_from(entry: *const u8) -> Self {
        let mut descriptor = Self([0; DESCRIPTOR_LEN]);
        std::ptr::copy_nonoverlapping(entry, descriptor.0.as_mut_ptr(), DESCRIPTOR_LEN);
        descriptor
    }

    /// Re-stage a descriptor value captured earlier.
    #[cfg(any(test, feature = "native-body-bank-alpha",
        feature = "mii-ordinary-player-alpha", feature = "mii-ordinary-cpu-alpha",
        feature = "mii-managed-helper-alpha"))]
    #[inline]
    pub(crate) fn from_bytes(bytes: &[u8; DESCRIPTOR_LEN]) -> Self {
        Self(*bytes)
    }

    /// Replace the complete canonical group and request `kinds[0]` as the starting form.
    /// Every other byte, including 0x24..0x28, keeps its copied value.
    #[inline]
    pub(crate) fn request_group(&mut self, group: Group) {
        self.request_group_starting(group, 0);
    }

    /// Replace the complete canonical group and request cell `starting` as the starting form
    /// (`+0x43`, the form native activates at Ready: a PT window starts at its configured form the
    /// same way). The body bank uses it when its layout moves the target off cell 0
    /// (`CellRule::PinCell0`: Kirby or Solo Popo takes cell 0 of any window that holds it), and
    /// for a window *reuse* with the live entry's `+0x43` (whatever the Reload that built it
    /// requested; a warm member change never rewrites it).
    #[inline]
    pub(crate) fn request_group_starting(&mut self, group: Group, starting: u8) {
        self.request_group_keeping_form(group);
        self.0[STARTING_FORM] = starting;
    }

    /// Replace the complete canonical group and leave the starting-form byte as copied; the
    /// callers above set it.
    #[inline]
    fn request_group_keeping_form(&mut self, group: Group) {
        self.put(COUNT, group.count);
        for (index, kind) in group.kinds.iter().copied().enumerate() {
            self.put(KINDS + index * 4, kind);
        }
        self.put(ACTOR_COUNT, group.actor_count);
    }

    #[cfg(any(test, feature = "native-body-bank-alpha",
        feature = "mii-ordinary-player-alpha", feature = "mii-ordinary-cpu-alpha"))]
    #[inline]
    pub(crate) fn bytes(&self) -> &[u8; DESCRIPTOR_LEN] {
        &self.0
    }

    #[inline]
    fn put(&mut self, offset: usize, value: u32) {
        self.0[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
    }

    #[inline]
    fn as_native(&self) -> *const u64 {
        (self as *const Self).cast()
    }
}

/// Submit a staged descriptor to the native install.
///
/// # Safety
/// `base` is the calling module's published, fingerprint-verified main text base. The
/// caller runs inside `entry`'s native loader callback, owns the shared install gate, and
/// has completed every validation, phase, trace and state step ordered before the call.
/// This function adds no check, lock, early return or log.
#[inline]
pub(crate) unsafe fn install(base: usize, entry: *mut u8, descriptor: &Descriptor, flags: Flags) {
    let native: unsafe extern "C" fn(*mut u8, *const u64, *const u8) =
        std::mem::transmute(base + INSTALL_ENTRY);
    let flags = flags.bytes();
    native(entry, descriptor.as_native(), flags.as_ptr());
}

