//! Shared role claims and committed foreign-kind evidence. Values only.
use {crate::engine::mii::inventory::{MiiClass, Snapshot}, crate::engine::mii::lifetime::{Binding, Configuration}, crate::engine::mii::native_values::{Mirrors, RowIds, RECORD_LEN}, crate::engine::mii::ordinary_player::Owner};

/// Native Mii rows use their selector. Ordinary indices are placeholders; neither
/// builder nor companion accesses G for their kinds. Pointer/row identity stays exact.
pub fn current_record_claim(kind:u32,index:u16)->bool {
    index!=0 || MiiClass::from_kind(kind).is_some()
}

pub fn claims(owners: &[Option<Owner>; 8], kinds: &[Option<u32>; 8]) -> Option<[Option<u16>; 8]> {
    claims_with(false,owners,kinds)
}
pub fn claims_in(layout:crate::engine::mii::backend::Layout,owners:&[Option<Owner>;8],kinds:&[Option<u32>;8])->Option<[Option<u16>;8]> {
    claims_in_with(layout,crate::engine::mii::capacity::Extras::NONE,owners,kinds)
}
/// `claims_in` over the layout owners plus exactly the registered created entries.
/// Every present owner, extra or not, still obeys the same claim and identity rules.
pub fn claims_in_with(layout:crate::engine::mii::backend::Layout,extras:crate::engine::mii::capacity::Extras,
    owners:&[Option<Owner>;8],kinds:&[Option<u32>;8])->Option<[Option<u16>;8]> {
    if !layout.shape_with(&owners.map(|o|o.map_or(0,|o|o.entry)),extras) {return None;}
    claims_with(layout.waiting(),owners,kinds)
}
fn claims_with(native_indices:bool,owners:&[Option<Owner>;8],kinds:&[Option<u32>;8])->Option<[Option<u16>;8]> {
    let mut claims = [None; 8];
    for (id, pair) in owners.iter().zip(kinds).enumerate() {
        let (owner, kind) = match pair {
            (None, None) => continue,
            (Some(owner), Some(kind)) => (owner, *kind),
            _ => return None,
        };
        let claim = u16::from(owner.selector);
        if claim != id as u16
            || owner.alias != 0
            || (!native_indices && MiiClass::from_kind(kind).is_some() && owner.index != claim)
            || (!native_indices && owner.index != 0 && owner.index != claim)
            || [owner.entry, owner.row, owner.information, owner.object]
                .iter()
                .any(|p| *p == 0 || !p.is_multiple_of(8))
        {
            return None;
        }
        if owners[..id].iter().flatten().any(|other| {
            owner.entry == other.entry
                || owner.row == other.row
                || owner.information == other.information
                || owner.object == other.object
                || owner.selector == other.selector
        }) {
            return None;
        }
        if native_indices && owners.iter().enumerate().any(|(other_id,other)| {
            other_id!=id && other.is_some_and(|other|
                (owner.index==u16::from(other.selector) && owner.index!=0)
                || (owner.index==other.index && (owner.index!=0
                    || MiiClass::from_kind(kind).is_some()
                        && kinds[other_id].is_some_and(|k|MiiClass::from_kind(k).is_some()))))
        }) {return None;}
        claims[id] = Some(claim);
    }
    Some(claims)
}

/// Rebuilt under the current G lock; never a retained pointer granting access.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Authority {
    pub binding: Binding,
    pub thread: usize,
    pub map: [usize; 3],
    pub entries: [usize; 8],
    pub owner: Owner,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Committed {
    pub authority: Authority,
    pub original_kind: u32,
    pub kind: u32,
}
impl Committed {
    pub fn permits(self, now: Authority, row_kind: u32, kind: u32) -> bool {
        now == self.authority
            && now.binding.epoch != 0
            && now.thread != 0
            && row_kind == self.original_kind
            && kind == self.kind
    }
}

/// Kind policy after the native reader rederives the foreign owner under G. E keeps
/// its native original identity. Ordinary kinds never read G in the native builder
/// or companion; changed Mii kinds need exact authority. Registration is additive.
/// The caller still checks O/header/mirrors/claims and all per-ticket protected bytes.
/// Native 13.0.5 CSS Ice Climbers keeps the paired kind 0x6e in E+8, while
/// the ready FighterEntry's first actor is Popo (0x4b). Preserve the exact
/// original anchor; do not accept an unrelated row merely because FE is ordinary.
pub fn original_row_matches(original_kind:u32,row_kind:i8)->bool {
    i64::from(row_kind)==i64::from(original_kind)
        || (original_kind==0x4b && row_kind==0x6e)
}

pub fn foreign_kind(
    now: Authority,
    original_kind: u32,
    row_kind: i8,
    kind: u32,
    committed: Option<Committed>,
) -> bool {
    original_row_matches(original_kind,row_kind)
        && (kind == original_kind
            || MiiClass::from_kind(kind).is_none()
            || committed.is_some_and(|c| c.permits(now, original_kind, kind)))
}

/// A numbered duplicate may retain the actually qualified payload. Certifying an
/// immutable Original also needs its exact unmasked Configuration. Native callers
/// separately require descriptor equality and retain their active payload on no-op.
pub fn payload_already_active(
    active: Option<Configuration>,
    publish: Option<Configuration>,
    restore_original: bool,
) -> bool {
    if restore_original {
        active == publish
    } else {
        active.map(|c| c.canonical()) == publish.map(|c| c.canonical())
    }
}

/// Exact origin selection; canonical identity alone cannot certify a native payload.
pub fn saved_origin(
    catalog: &Snapshot,
    kind: u32,
    mirrors: Mirrors,
    record: &[u8; RECORD_LEN],
) -> Option<Configuration> {
    if !catalog.is_complete() {
        return None;
    }
    catalog
        .configs
        .iter()
        .filter_map(|s| Configuration::from_saved(s).ok())
        .find(|c| {
            c.canonical().class as u32 == kind
                && c.tokens() == mirrors.tokens
                && c.charinfo().as_slice() == &record[..88]
        })
}
pub fn validate_saved(
    c: Configuration,
    index: u16,
    mirrors: Mirrors,
    rows: RowIds,
    record: &[u8; RECORD_LEN],
) -> Result<(), &'static str> {
    mirrors
        .validate(index, c.tokens())
        .map_err(|_| "saved origin mirrors differ")?;
    rows.validate(c)
        .map_err(|_| "saved origin row identity differs")?;
    if record[..88] != *c.charinfo() {
        return Err("saved origin CharInfo differs");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    fn catalog() -> Snapshot {
        use crate::engine::mii::inventory::{build_snapshot, RawSlot, TaggedId};
        build_snapshot(vec![RawSlot {
            index: 0,
            slot_id: 2622,
            name_len: 1,
            name_units: [65, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            colour: 4,
            voice: 37,
            specials: [1, 0, 0, 0],
            tag_outfit: TaggedId(0x13002b147bc59435),
            tag_hat: TaggedId(0x5a00000fa9efa5ea),
            tag_chara: TaggedId(0xc1004d15d7061f77),
            charinfo: [0; 88],
        }])
    }
    fn origin_catalog_resolution_uses_unmasked_bytes_and_tokens() {
        let mut catalog = catalog();
        let c = Configuration::from_saved(&catalog.configs[0]).unwrap();
        let t = c.tokens();
        let mirrors = Mirrors {
            index: 0,
            tokens: t,
            fe: [0, t.outfit, t.hat, t.colour, t.voice],
            d: [0, t.outfit, t.hat, t.colour],
            d_specials: t.specials.map(u32::from),
            owner_index: 0,
            owner: [t.outfit, t.hat, t.colour, t.voice].map(u32::from),
        };
        let mut record = [0xa5; RECORD_LEN];
        record[..88].copy_from_slice(c.charinfo());
        assert_eq!(saved_origin(&catalog, 0x49, mirrors, &record), Some(c));
        assert_eq!(saved_origin(&catalog, 0x4a, mirrors, &record), None);
        record[0] ^= 1; // a masked canonical hash alone would still match
        assert_eq!(saved_origin(&catalog, 0x49, mirrors, &record), None);
        record[0] ^= 1;
        let mut changed = mirrors;
        changed.tokens.voice += 1;
        assert_eq!(saved_origin(&catalog, 0x49, changed, &record), None);
        catalog.findings.push(crate::engine::mii::inventory::Finding::Source {
            slot_id: None,
            error: "unreadable".into(),
        });
        assert_eq!(saved_origin(&catalog, 0x49, mirrors, &record), None);
    }
    fn owners() -> [Option<Owner>; 8] {
        core::array::from_fn(|id| {
            (id < 4).then_some(Owner {
                entry: 0x1000 + id * 0x100,
                row: 0x2000 + id * 0x100,
                information: 0x3000 + id * 0x100,
                object: 0x4000 + id * 0x100,
                index: 0,
                selector: id as u8,
                alias: 0,
            })
        })
    }
    fn authority(id: usize) -> Authority {
        let rows = owners();
        let mut owner = rows[id].unwrap();
        owner.index = id as u16;
        Authority {
            binding: Binding {
                epoch: 4,
                owner: id as u32,
                entry: owner.entry,
                row: owner.row,
                holder: 0x5000,
                manager: 0x6000,
                vector_begin: 0x7000,
                vector_end: 0x71a0,
                record_index: id as u16,
            },
            thread: 0x8000,
            map: [0x9000, 0xa000, 0xb000],
            entries: rows.map(|o| o.map_or(0, |o| o.entry)),
            owner,
        }
    }
    fn changed_foreign_mii_requires_exact_current_committed_owner() {
        let authority = authority(1);
        let committed = Committed {
            authority,
            original_kind: 9,
            kind: 0x4a,
        };
        assert!(foreign_kind(authority, 9, 9, 0x4a, Some(committed)));
        assert!(!foreign_kind(authority, 9, 9, 0x49, Some(committed)));
        assert!(!foreign_kind(authority, 9, 0x4a, 0x4a, Some(committed)));
        let mut stale = authority;
        stale.binding.epoch += 1;
        assert!(!foreign_kind(stale, 9, 9, 0x4a, Some(committed)));
        stale = authority;
        stale.binding.vector_begin += 0x68;
        assert!(!foreign_kind(stale, 9, 9, 0x4a, Some(committed)));
        stale = authority;
        stale.owner.object += 8;
        assert!(!foreign_kind(stale, 9, 9, 0x4a, Some(committed)));
        stale = authority;
        stale.entries[2] += 8;
        assert!(!foreign_kind(stale, 9, 9, 0x4a, Some(committed)));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("origin_catalog_resolution_uses_unmasked_bytes_and_tokens", origin_catalog_resolution_uses_unmasked_bytes_and_tokens as fn()),
            ("changed_foreign_mii_requires_exact_current_committed_owner", changed_foreign_mii_requires_exact_current_committed_owner as fn()),
        ]);
    }
    }
}
