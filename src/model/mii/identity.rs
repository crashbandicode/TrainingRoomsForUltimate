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

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::mii::inventory::{Appearance, Canonical, MiiClass, MiiConfig, TaggedId, CHARINFO_LEN, CHARINFO_MASK_END};

    /// The `lifetime` tests' sample configuration (a Swordfighter preset), rebuilt here.
    fn configuration() -> Configuration {
        let face = 7u8;
        let mut info = [face; CHARINFO_LEN];
        info[..CHARINFO_MASK_END].fill(1);
        let mut masked = info;
        masked[..CHARINFO_MASK_END].fill(0);
        let canonical = Canonical {
            class: MiiClass::Swordfighter,
            appearance: Appearance {
                charinfo_masked: masked, specials: [face % 3, 0, 0, 0], colour: 4, voice: 37,
                outfit: 0x14_7bc5_9435, hat: 0x0f_a9ef_a5ea, chara: MiiClass::Swordfighter.chara_hash40(),
            },
        };
        Configuration::from_saved(&MiiConfig {
            slot_id: 107, slot_index: 0, name: String::new(), charinfo: info,
            raw_tags: [TaggedId(0x13002b147bc59435), TaggedId(0x5a00000fa9efa5ea), TaggedId(0xc1004d15d7061f77)],
            canonical, copies: vec![],
        }).unwrap()
    }

    fn pta116t_before() -> Identity {
        // PTA116T #658: the CPU's exchanged group [0x21, 0x1e] (count 2, actor 1) over the window
        // bank [9, 1, 5]; the shared R header is still the window's [1, 9, 1, 5, 0].
        let mut descriptor = [0u8; DESCRIPTOR_LEN];
        GroupLayout { count: 2, kinds: [0x21, 0x1e, u32::MAX], actors: 1 }.apply(&mut descriptor);
        descriptor[0x10..0x14].copy_from_slice(&(-1i32).to_le_bytes());
        descriptor[0x34] = 1; descriptor[0x35] = 1; descriptor[0x37] = 100;
        Identity { entry: 0x11af92c000, owner: 1, epoch: 7, resource: 0x2000, object: 0x11afee3200, header: [1, 9, 1, 5, 0],
            descriptor, index: 0, tokens: [0; 4], bodies: [(0x90000, 1), (0xa0000, 2), (0, 0)], active: 0 }
    }

    fn exchanged_group_header_is_its_own_capture_not_the_descriptor_words() {
        let before = pta116t_before();
        let now = before;
        assert_eq!(verify(&before, &now, ExpectedMutation::None), Ok(()), "the prequeue compares capture against capture");
        // The old predicate expected [selector, FE+0x18, FE+0x1c, FE+0x20, FE+0x41] = [1, 0x21, 0x1e, -1, 0].
        let derived = Identity { header: [1, 0x21, 0x1e, u32::MAX, 0], ..before };
        assert_eq!(verify(&derived, &now, ExpectedMutation::None), Err(Mismatch { field: Field::Header(1), expected: 0x21, actual: 9 }),
            "D-PTA1: a derived header is a mismatch, and the first word that differs is named");
        for word in 0..5 {
            let mut foreign = now; foreign.header[word] ^= 0x100;
            assert_eq!(verify(&before, &foreign, ExpectedMutation::None).unwrap_err().field, Field::Header(word as u8));
            assert_eq!(verify(&before, &foreign, ExpectedMutation::NativeBehavior).unwrap_err().field, Field::Header(word as u8), "no mutation frees the header except an exact Mii setup");
        }
        let mut other_object = now; other_object.object += 0x100;
        assert_eq!(verify(&before, &other_object, ExpectedMutation::None).unwrap_err().field, Field::Object);
        let mut other_index = now; other_index.index = 3;
        assert_eq!(verify(&before, &other_index, ExpectedMutation::None).unwrap_err().field, Field::Index);
        let mut other_tokens = now; other_tokens.tokens[2] = 9;
        assert_eq!(verify(&before, &other_tokens, ExpectedMutation::None).unwrap_err().field, Field::Tokens(2));
    }

    fn configure_skipped_admits_exactly_the_installed_r68_and_tokens() {
        // W-M2-4: the gate installed R+0x68 = FE+0x46 of the incoming Mii and the configured tokens on
        // the skip path; the receipt proves those values and refuses any other change.
        let before = pta116t_before();
        let mut now = before; now.index = 7; now.tokens = [0x10b, 0x2, 0x33, 0x0];
        let allowed = ExpectedMutation::ConfigureSkipped { index: 7, tokens: [0x10b, 0x2, 0x33, 0x0] };
        assert_eq!(verify(&before, &now, allowed), Ok(()));
        assert_eq!(verify(&before, &now, ExpectedMutation::None).unwrap_err().field, Field::Index, "without the receipt the install is a mismatch");
        let mut stale = now; stale.index = 0;
        assert_eq!(verify(&before, &stale, allowed), Err(Mismatch { field: Field::Index, expected: 7, actual: 0 }), "a stale R+0x68 is the defect");
        let mut other_token = now; other_token.tokens[2] = 0x34;
        assert_eq!(verify(&before, &other_token, allowed).unwrap_err().field, Field::Tokens(2));
        let mut header = now; header.header[1] ^= 1;
        assert_eq!(verify(&before, &header, allowed).unwrap_err().field, Field::Header(1), "the skip path writes nothing else");
        let mut body = now; body.bodies[0].1 += 1;
        assert_eq!(verify(&before, &body, allowed).unwrap_err().field, Field::Body(0));
    }

    fn training_menu_live_bytes_are_tolerated_only_in_domain_and_nothing_else_is() {
        let before = pta116t_before();
        // C120 S2 #934 / C104 #336..#342: Behavior=Control writes ffffffff>00000001, 01>00, 01>24, 100>21; FE+0x36 as written.
        let mut now = before;
        now.descriptor[0x10..0x14].copy_from_slice(&1i32.to_le_bytes());
        now.descriptor[0x34] = 0; now.descriptor[0x35] = 0x24; now.descriptor[0x36] ^= 0x5a; now.descriptor[0x37] = 21;
        assert_eq!(verify(&before, &now, ExpectedMutation::NativeBehavior), Ok(()));
        assert_eq!(verify(&now, &before, ExpectedMutation::NativeBehavior), Ok(()), "releasing Control is live too");
        assert_eq!(verify(&before, &now, ExpectedMutation::None).unwrap_err().field, Field::Descriptor(0x10), "a Track proof admits no writer");
        // Out of domain on the new side …
        let mut rank = now; rank.descriptor[0x37] = 101;
        assert_eq!(verify(&before, &rank, ExpectedMutation::NativeBehavior), Err(Mismatch { field: Field::Descriptor(0x37), expected: 100, actual: 101 }));
        let mut pad = now; pad.descriptor[0x10..0x14].copy_from_slice(&9i32.to_le_bytes());
        assert_eq!(verify(&before, &pad, ExpectedMutation::NativeBehavior).unwrap_err().field, Field::Descriptor(0x10));
        let mut control = now; control.descriptor[0x35] = 0x28;
        assert_eq!(verify(&before, &control, ExpectedMutation::NativeBehavior).unwrap_err().field, Field::Descriptor(0x35));
        // … and on the old side (Astra review 2.1: rank 101 → 21 must refuse).
        let mut odd_base = before; odd_base.descriptor[0x37] = 101;
        assert_eq!(verify(&odd_base, &now, ExpectedMutation::NativeBehavior).unwrap_err().field, Field::Descriptor(0x37));
        let mut unchanged_odd = before; unchanged_odd.descriptor[0x37] = 101;
        let mut same = unchanged_odd; same.descriptor[0x34] = 0;
        assert_eq!(verify(&unchanged_odd, &same, ExpectedMutation::NativeBehavior), Ok(()), "an unchanged byte keeps any value");
        // The descriptor count written to 255 is a foreign change even under NativeBehavior (Astra review 2.1).
        let mut count = now; count.descriptor[0x14] = 255;
        assert_eq!(verify(&before, &count, ExpectedMutation::NativeBehavior), Err(Mismatch { field: Field::Descriptor(0x14), expected: 2, actual: 255 }));
        // Every non-live byte flipped alone is refused and named.
        for offset in 0..DESCRIPTOR_LEN {
            if is_live_behavior_byte(offset) { continue; }
            let mut flipped = now; flipped.descriptor[offset] ^= 0xff;
            assert_eq!(verify(&before, &flipped, ExpectedMutation::NativeBehavior).unwrap_err().field, Field::Descriptor(offset as u8), "offset {offset:#x}");
        }
        // Bodies and the active member are never behaviour.
        let mut body = now; body.bodies[1].0 += 8;
        assert_eq!(verify(&before, &body, ExpectedMutation::NativeBehavior).unwrap_err().field, Field::Body(1));
        let mut active = now; active.active = 1;
        assert_eq!(verify(&before, &active, ExpectedMutation::NativeBehavior).unwrap_err().field, Field::Active);
    }

    fn a_group_exchange_permits_exactly_the_minted_layout_and_bodies() {
        // GSFIX D1: window [3, 7, 8] exchanged for the group [10, 11, 21]; R header stays [1, 3, 7, 8, 2].
        let mut before = pta116t_before();
        before.header = [1, 3, 7, 8, 2];
        GroupLayout { count: 3, kinds: [3, 7, 8], actors: 1 }.apply(&mut before.descriptor);
        before.descriptor[0x41] = 2;
        before.bodies = [(0x3000, 1), (0x4000, 262144), (0x5000, 327680)];
        let layout = GroupLayout { count: 3, kinds: [10, 11, 21], actors: 1 };
        let bodies = [(0x6000, 393216), (0x7000, 458752), (0x8000, 524288)];
        let mut after = before;
        layout.apply(&mut after.descriptor);
        after.bodies = bodies;
        let exchange = ExpectedMutation::GroupExchange { layout, bodies };
        assert_eq!(verify(&before, &after, exchange), Ok(()));
        assert_eq!(GroupLayout::of(&after.descriptor), layout);
        assert_eq!(verify(&before, &after, ExpectedMutation::None).unwrap_err().field, Field::Descriptor(0x18), "the exchange is the only writer of the words");
        let mut header = after; header.header[1] = 10;
        assert_eq!(verify(&before, &header, exchange), Err(Mismatch { field: Field::Header(1), expected: 3, actual: 10 }), "the header is never re-derived from the words");
        let mut other_bodies = after; other_bodies.bodies[2].1 += 1;
        assert_eq!(verify(&before, &other_bodies, exchange).unwrap_err().field, Field::Body(2));
        let mut other_layout = after;
        GroupLayout { count: 2, kinds: [10, 11, u32::MAX], actors: 1 }.apply(&mut other_layout.descriptor);
        assert_eq!(verify(&before, &other_layout, exchange).unwrap_err().field, Field::Descriptor(0x14));
        let mut flag = after; flag.descriptor[0x41] ^= 1;
        assert_eq!(verify(&before, &flag, exchange).unwrap_err().field, Field::Descriptor(0x41));
        // After the exchange the receipt admits only live behaviour (C118 D5 / C120 D5).
        let mut toggled = after;
        toggled.descriptor[0x10..0x14].copy_from_slice(&1i32.to_le_bytes());
        toggled.descriptor[0x34] = 0; toggled.descriptor[0x35] = 0x24; toggled.descriptor[0x37] = 21;
        assert_eq!(verify(&after, &toggled, ExpectedMutation::NativeBehavior), Ok(()));
        assert_eq!(verify(&after, &toggled, ExpectedMutation::None).unwrap_err().field, Field::Descriptor(0x10));
    }

    fn a_native_form_selection_changes_only_the_selected_member() {
        let before = pta116t_before();
        let mut now = before; now.active = 1;
        assert_eq!(verify(&before, &now, ExpectedMutation::NativeFormSelection(1)), Ok(()));
        assert_eq!(verify(&before, &now, ExpectedMutation::None).unwrap_err().field, Field::Active);
        assert_eq!(verify(&before, &now, ExpectedMutation::NativeFormSelection(2)), Err(Mismatch { field: Field::Active, expected: 2, actual: 1 }));
        let mut out_of_range = before; out_of_range.active = 3;
        assert_eq!(verify(&before, &out_of_range, ExpectedMutation::NativeFormSelection(3)).unwrap_err().field, Field::Active);
        let mut plus_bytes = now; plus_bytes.descriptor[0x18] ^= 1;
        assert_eq!(verify(&before, &plus_bytes, ExpectedMutation::NativeFormSelection(1)).unwrap_err().field, Field::Descriptor(0x18), "the form change rewrites no descriptor word");
        assert_eq!(verify(&before, &before, ExpectedMutation::NativeFormSelection(1)), Ok(()), "an unchanged selection is admitted too");
    }

    fn an_exact_mii_publication_frees_only_what_the_setup_owns() {
        let before = pta116t_before();
        let layout = GroupLayout { count: 1, kinds: [0x4a, u32::MAX, u32::MAX], actors: 1 };
        let configuration = configuration();
        let allowed = ExpectedMutation::ExactMii { layout, configuration };
        let mut now = before;
        layout.apply(&mut now.descriptor);
        now.header = [1, 0x4a, u32::MAX, u32::MAX, 0];
        now.index = 5; now.tokens = [1, 2, 3, 4];
        for offset in MII_BYTES { now.descriptor[offset] = 0x5a; }
        now.bodies = [(0x9000, 7), (0, 0), (0, 0)];
        assert_eq!(verify(&before, &now, allowed), Ok(()));
        let mut foreign = now; foreign.descriptor[0x30] ^= 1;
        assert_eq!(verify(&before, &foreign, allowed).unwrap_err().field, Field::Descriptor(0x30));
        let mut other_entry = now; other_entry.entry += 8;
        assert_eq!(verify(&before, &other_entry, allowed).unwrap_err().field, Field::Entry);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("exchanged_group_header_is_its_own_capture_not_the_descriptor_words", exchanged_group_header_is_its_own_capture_not_the_descriptor_words as fn()),
            ("configure_skipped_admits_exactly_the_installed_r68_and_tokens", configure_skipped_admits_exactly_the_installed_r68_and_tokens as fn()),
            ("training_menu_live_bytes_are_tolerated_only_in_domain_and_nothing_else_is", training_menu_live_bytes_are_tolerated_only_in_domain_and_nothing_else_is as fn()),
            ("a_group_exchange_permits_exactly_the_minted_layout_and_bodies", a_group_exchange_permits_exactly_the_minted_layout_and_bodies as fn()),
            ("a_native_form_selection_changes_only_the_selected_member", a_native_form_selection_changes_only_the_selected_member as fn()),
            ("an_exact_mii_publication_frees_only_what_the_setup_owns", an_exact_mii_publication_frees_only_what_the_setup_owns as fn()),
        ]);
    }
    }
}
