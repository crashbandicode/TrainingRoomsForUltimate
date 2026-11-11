//! Exact identity receipts: one `Identity` captured at preflight and one `verify(before, now,
//! ExpectedMutation)` for every compare site (WORKPLAN §2.6 5a; SYNTHESIS agree-2; Astra §7
//! `ExpectedMutation`; Fable §2.5 per-byte writer table as the implementation).
//!
//! Facts the rules encode:
//! - The outgoing object header `R+0x54..0x68` is a **capture**, never derived from the FE words
//!   (D-PTA1-RCA §0/§7: after a same-R group exchange the shared resource keeps the window's kinds
//!   while `FE+0x18..` names the exchanged group; PTA116T #658 `header=[1, 9, 1, 5, 0]` under FE
//!   words `[0x21, 0x1e, -1]`).
//! - A bank group exchange rewrites exactly the entry's count (`FE+0x14`), kinds (`FE+0x18..0x24`)
//!   and actor count (`FE+0x28`) and pins the resulting body addresses/IDs (C116-GSFIX D1; consts
//!   `GroupExchange::new`); every other byte, the header, entry/owner/epoch/R stay equal.
//! - Training CPU Behavior (Stop, Attack, level, Control) natively rewrites the live bytes
//!   `FE+0x10..0x14` (controller, `-1..=8`), `FE+0x34` (operation), `FE+0x35` (control type,
//!   `0..=0x27`), `FE+0x36` (AI+0xc10) and `FE+0x37` (rank, `0..=100`) on the live entry
//!   (C120-FIX; C104 #336..#342 `ffffffff>00000001, 01>00, 01>24, 100>21`); a change is qualified only
//!   when **both** the old and the new value are inside the native domain
//!   (`ordinary_cpu::live_behavior_only_change`, Astra review 2.1), and an unchanged byte keeps any value.
//! - A native PT/Aegis form change (Down+B) moves the entry's selected member (`E+0x5918`) under a
//!   bank record without a mod request (SPECIAL-GROUPS §9a; Astra review item 6): only that field may
//!   change, to the selected cell.
//! - An exact Mii publication (WP-M2) rewrites the group layout plus the Mii bytes and owner
//!   identity the native setup owns; its value rules are the settle proof's (`ExactBankSettle`),
//!   so here only the fields it may touch are freed and everything else stays exact.
//!
//! A mismatch names the first offending field (`Mismatch`); a per-owner mismatch fails that
//! owner's claim only — the caller never widens tolerance to pass another owner (Astra review 2.1).

use crate::engine::mii::lifetime::Configuration;
use crate::engine::mii::ordinary_cpu::live_behavior_only_change;

pub const DESCRIPTOR_LEN: usize = 0x70;
pub const BODIES: usize = 3;

/// Descriptor offsets of the group layout a bank exchange rewrites (consts `GroupExchange::new`).
pub const LAYOUT_COUNT: usize = 0x14;
pub const LAYOUT_KINDS: usize = 0x18;
pub const LAYOUT_ACTORS: usize = 0x28;
/// Descriptor bytes an exact Mii publication owns (MII-GROUPS-IMPL §1: `FE+0x41..0x55`).
pub const MII_BYTES: core::ops::Range<usize> = 0x41..0x56;

/// The identity captured once at preflight (header under the pool lock) and re-read at every
/// later proof. Fields a site cannot observe are zero on both sides and therefore equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity {
    pub entry: usize,
    pub owner: u32,
    pub epoch: u64,
    pub resource: usize,
    pub object: usize,
    /// `R+0x54..0x68` as captured.
    pub header: [u32; 5],
    pub descriptor: [u8; DESCRIPTOR_LEN],
    /// `O+0x68`.
    pub index: u16,
    /// `O+0x6c..0x7c`.
    pub tokens: [u32; 4],
    /// Body address and native id per cell.
    pub bodies: [(usize, u32); BODIES],
    /// `E+0x5918`, the selected member.
    pub active: u8,
}
impl Identity {
    pub const ZERO: Identity = Identity {
        entry: 0, owner: 0, epoch: 0, resource: 0, object: 0, header: [0; 5], descriptor: [0; DESCRIPTOR_LEN],
        index: 0, tokens: [0; 4], bodies: [(0, 0); BODIES], active: 0,
    };
}

/// The entry words a group exchange writes (`Members` projected to values).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupLayout {
    pub count: u32,
    pub kinds: [u32; BODIES],
    pub actors: u32,
}
impl GroupLayout {
    pub fn of(descriptor: &[u8; DESCRIPTOR_LEN]) -> GroupLayout {
        let word = |at: usize| u32::from_le_bytes([descriptor[at], descriptor[at + 1], descriptor[at + 2], descriptor[at + 3]]);
        GroupLayout { count: word(LAYOUT_COUNT), kinds: [word(LAYOUT_KINDS), word(LAYOUT_KINDS + 4), word(LAYOUT_KINDS + 8)], actors: word(LAYOUT_ACTORS) }
    }
    pub fn apply(self, descriptor: &mut [u8; DESCRIPTOR_LEN]) {
        descriptor[LAYOUT_COUNT..LAYOUT_COUNT + 4].copy_from_slice(&self.count.to_le_bytes());
        for (i, kind) in self.kinds.iter().enumerate() {
            descriptor[LAYOUT_KINDS + i * 4..LAYOUT_KINDS + i * 4 + 4].copy_from_slice(&kind.to_le_bytes());
        }
        descriptor[LAYOUT_ACTORS..LAYOUT_ACTORS + 4].copy_from_slice(&self.actors.to_le_bytes());
    }
}

/// The one mutation a phase admits (Astra §7). Callers cannot compose or widen it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpectedMutation {
    None,
    /// A bank exchange: exactly this layout on the entry and exactly these bodies.
    GroupExchange { layout: GroupLayout, bodies: [(usize, u32); BODIES] },
    /// An exact Mii publication (WP-M2): this layout, the Mii bytes, the owner header, index and
    /// tokens are the setup's; the settle proof holds their values.
    ExactMii { layout: GroupLayout, configuration: Configuration },
    /// Training CPU Behavior: the five live bytes, each inside its native domain on both sides.
    NativeBehavior,
    /// Native PT/Aegis Down+B: only the selected member changes, to this cell.
    NativeFormSelection(u8),
    /// W-M2-4 root fix (WP-R DECISION §R+0x68): the request was served without native configuration
    /// (`R+0x4c` bit0 at `0x6510f4`, configure `0x651334–0x65137c` skipped), so the gate detour itself
    /// installed the configure-equivalent words — `R+0x68` (the incoming Mii's record, `FE+0x46`) and the
    /// tokens `R+0x6c..0x7c` from `identity_view::configured_mii_words(descriptor)`. The receipt admits
    /// exactly those values and nothing else (header, descriptor, bodies unchanged).
    ConfigureSkipped { index: u16, tokens: [u32; 4] },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Entry,
    Owner,
    Epoch,
    Resource,
    Object,
    Header(u8),
    Descriptor(u8),
    Index,
    Tokens(u8),
    Body(u8),
    Active,
}

/// The first field that differs outside the admitted mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mismatch {
    pub field: Field,
    pub expected: u64,
    pub actual: u64,
}

fn differ(field: Field, expected: u64, actual: u64) -> Result<(), Mismatch> {
    if expected == actual { Ok(()) } else { Err(Mismatch { field, expected, actual }) }
}

/// The five live-behaviour bytes (C120-FIX).
pub fn is_live_behavior_byte(offset: usize) -> bool { (0x10..0x14).contains(&offset) || (0x34..=0x37).contains(&offset) }

/// Compare two descriptors under `allowed`, naming the first offending byte.
pub fn verify_descriptor(before: &[u8; DESCRIPTOR_LEN], now: &[u8; DESCRIPTOR_LEN], allowed: ExpectedMutation) -> Result<(), Mismatch> {
    let first_diff = |expected: &[u8; DESCRIPTOR_LEN], skip: &dyn Fn(usize) -> bool| -> Result<(), Mismatch> {
        for offset in 0..DESCRIPTOR_LEN {
            if skip(offset) || expected[offset] == now[offset] { continue; }
            return Err(Mismatch { field: Field::Descriptor(offset as u8), expected: u64::from(expected[offset]), actual: u64::from(now[offset]) });
        }
        Ok(())
    };
    match allowed {
        ExpectedMutation::None | ExpectedMutation::NativeFormSelection(_) | ExpectedMutation::ConfigureSkipped { .. } => first_diff(before, &|_| false),
        ExpectedMutation::GroupExchange { layout, .. } => {
            let mut expected = *before;
            layout.apply(&mut expected);
            first_diff(&expected, &|_| false)
        }
        ExpectedMutation::ExactMii { layout, .. } => {
            let mut expected = *before;
            layout.apply(&mut expected);
            first_diff(&expected, &|offset| MII_BYTES.contains(&offset))
        }
        ExpectedMutation::NativeBehavior => {
            if live_behavior_only_change(before, now) { return Ok(()); }
            // Name the byte: a non-live byte that changed, else the live byte whose native domain failed
            // on either side of its change (`expected` is the domain's upper bound; an unchanged byte
            // keeps any value, as in `live_behavior_only_change`).
            first_diff(before, &is_live_behavior_byte)?;
            let word = |d: &[u8; DESCRIPTOR_LEN]| [d[0x10], d[0x11], d[0x12], d[0x13]];
            if word(before) != word(now) {
                for d in [now, before] {
                    if !(-1..=8).contains(&i32::from_le_bytes(word(d))) {
                        return Err(Mismatch { field: Field::Descriptor(0x10), expected: 8, actual: u64::from(u32::from_le_bytes(word(d))) });
                    }
                }
            }
            for (offset, max) in [(0x35usize, 0x27u8), (0x37, 100)] {
                if before[offset] != now[offset] {
                    for d in [now, before] {
                        if d[offset] > max {
                            return Err(Mismatch { field: Field::Descriptor(offset as u8), expected: u64::from(max), actual: u64::from(d[offset]) });
                        }
                    }
                }
            }
            Err(Mismatch { field: Field::Descriptor(0x37), expected: u64::from(before[0x37]), actual: u64::from(now[0x37]) })
        }
    }
}

/// Field by field: equal, or changed only as `allowed` admits. Names the first offending field.
pub fn verify(before: &Identity, now: &Identity, allowed: ExpectedMutation) -> Result<(), Mismatch> {
    differ(Field::Entry, before.entry as u64, now.entry as u64)?;
    differ(Field::Owner, u64::from(before.owner), u64::from(now.owner))?;
    differ(Field::Epoch, before.epoch, now.epoch)?;
    differ(Field::Resource, before.resource as u64, now.resource as u64)?;
    differ(Field::Object, before.object as u64, now.object as u64)?;
    let header_free = matches!(allowed, ExpectedMutation::ExactMii { .. });
    if !header_free {
        for i in 0..5 { differ(Field::Header(i as u8), u64::from(before.header[i]), u64::from(now.header[i]))?; }
    }
    verify_descriptor(&before.descriptor, &now.descriptor, allowed)?;
    if !header_free {
        // `R+0x68` (`index`) and the tokens: unchanged, or exactly the configure-equivalent words the gate
        // installed on the skip path (W-M2-4).
        let (index, tokens) = match allowed {
            ExpectedMutation::ConfigureSkipped { index, tokens } => (index, tokens),
            _ => (before.index, before.tokens),
        };
        differ(Field::Index, u64::from(index), u64::from(now.index))?;
        for (i, (want, got)) in tokens.iter().zip(now.tokens.iter()).enumerate() {
            differ(Field::Tokens(i as u8), u64::from(*want), u64::from(*got))?;
        }
    }
    match allowed {
        ExpectedMutation::GroupExchange { bodies, .. } => {
            for (i, (want, got)) in bodies.iter().zip(now.bodies.iter()).enumerate() {
                differ(Field::Body(i as u8), want.0 as u64, got.0 as u64)?;
                differ(Field::Body(i as u8), u64::from(want.1), u64::from(got.1))?;
            }
        }
        ExpectedMutation::ExactMii { .. } => {}
        _ => {
            for i in 0..BODIES {
                differ(Field::Body(i as u8), before.bodies[i].0 as u64, now.bodies[i].0 as u64)?;
                differ(Field::Body(i as u8), u64::from(before.bodies[i].1), u64::from(now.bodies[i].1))?;
            }
        }
    }
    match allowed {
        ExpectedMutation::NativeFormSelection(selected) => {
            if now.active != before.active && now.active != selected {
                return Err(Mismatch { field: Field::Active, expected: u64::from(selected), actual: u64::from(now.active) });
            }
            if usize::from(now.active) >= BODIES {
                return Err(Mismatch { field: Field::Active, expected: u64::from(selected), actual: u64::from(now.active) });
            }
            Ok(())
        }
        _ => differ(Field::Active, u64::from(before.active), u64::from(now.active)),
    }
}

