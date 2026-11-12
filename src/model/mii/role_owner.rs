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
    fn placeholder_zero_does_not_claim_player_payload() {
        let mut rows = owners();
        let ordinary = [Some(9), Some(9), Some(9), Some(9), None, None, None, None];
        let expected = Some([Some(0), Some(1), Some(2), Some(3), None, None, None, None]);
        assert_eq!(claims(&rows, &ordinary), expected);
        let mut kinds = ordinary;
        kinds[0] = Some(0x49);
        assert_eq!(claims(&rows, &kinds), expected);
        kinds[1] = Some(0x4a);
        assert_eq!(claims(&rows, &kinds), None); // a real CPU Mii must use claim1
        rows[1].as_mut().unwrap().index = 1;
        assert_eq!(claims(&rows, &kinds), expected);
        rows[2].as_mut().unwrap().index = 1;
        assert_eq!(claims(&rows, &kinds), None);
        rows = owners();
        rows[3].as_mut().unwrap().selector = 2;
        assert_eq!(claims(&rows, &ordinary), None);
        rows = owners();
        rows[2].as_mut().unwrap().object = rows[0].unwrap().object;
        assert_eq!(claims(&rows, &ordinary), None);
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

    fn css_ice_climbers_row_keeps_exact_ownership_for_mii_cycles() {
        // Native13.0.5 trace: CSS IC E+8=0x6e, FE first actor0x4b/count2.
        assert!(original_row_matches(0x4b,0x6e));
        assert!(original_row_matches(0x4b,0x4b));
        assert!(!original_row_matches(0x4b,0x6d));
        assert!(!original_row_matches(0x48,0x6e));
        assert!(!original_row_matches(0x4b,-1));
        for id in [0,1] {
            let now=authority(id);
            assert!(foreign_kind(now,0x4b,0x6e,0x4b,None));
            assert!(!foreign_kind(now,0x4b,0x6e,0x48,None));
            let committed=Committed {authority:now,original_kind:0x4b,kind:0x48};
            assert!(foreign_kind(now,0x4b,0x6e,0x48,Some(committed)));
            let mut stale=now;stale.binding.epoch+=1;
            assert!(!foreign_kind(stale,0x4b,0x6e,0x48,Some(committed)));
            assert!(!foreign_kind(now,0x4b,0x6d,0x48,Some(committed)));
        }
    }

    fn ordinary_foreign_kind_preserves_original_row_anchor_for_both_roles() {
        for id in [0, 1] {
            let now = authority(id);
            let committed = Committed {
                authority: now,
                original_kind: 9,
                kind: 0x49,
            };
            for registration in [None, Some(committed)] {
                assert!(foreign_kind(now, 9, 9, 16, registration));
                assert!(!foreign_kind(now, 9, 16, 16, registration));
                assert!(!foreign_kind(now, 9, -1, 16, registration));
            }
            assert!(foreign_kind(now, 0x49, 0x49, 9, None));
            assert!(!foreign_kind(now, 9, 9, 0x49, None));
            assert!(foreign_kind(now, 9, 9, 0x49, Some(committed)));
        }
    }

    fn stale_registration_never_narrows_unchanged_foreign_kind() {
        for id in [0, 1] {
            let now = authority(id);
            let mut stale = now;
            stale.binding.epoch -= 1;
            stale.owner.object += 8;
            let committed = Committed {
                authority: stale,
                original_kind: 0x49,
                kind: 0x4a,
            };
            assert!(foreign_kind(now, 0x49, 0x49, 0x49, Some(committed)));
            assert!(foreign_kind(now, 0x49, 0x49, 16, Some(committed)));
            assert!(!foreign_kind(now, 0x49, 0x49, 0x4a, Some(committed)));
        }
    }

    fn numbered_duplicate_noop_cannot_certify_exact_original_payload() {
        let catalog = catalog();
        let saved = &catalog.configs[0];
        let original = Configuration::from_saved(saved).unwrap();
        let mut copy = saved.clone();
        copy.charinfo[0] ^= 1; // copy identity is masked by canonical deduplication
        let duplicate = Configuration::from_saved(&copy).unwrap();
        assert_eq!(original.canonical(), duplicate.canonical());
        assert_ne!(original, duplicate);
        assert!(payload_already_active(
            Some(duplicate),
            Some(original),
            false
        ));
        assert!(!payload_already_active(
            Some(duplicate),
            Some(original),
            true
        ));
        assert!(payload_already_active(Some(original), Some(original), true));
        copy.canonical.appearance.voice += 1;
        let different = Configuration::from_saved(&copy).unwrap();
        assert!(!payload_already_active(
            Some(duplicate),
            Some(different),
            false
        ));
        assert!(!payload_already_active(Some(original), None, false));
        assert!(!payload_already_active(None, Some(original), true));
        assert!(payload_already_active(None, None, true));
    }
    fn waiting_extras_keep_native_mii_indices_disjoint() {
        use {crate::engine::mii::backend::Layout, crate::engine::mii::capacity::Extras};
        for layout in [Layout::Ldn, Layout::Quickplay] {
            let mut rows = std::array::from_fn(|id| (id != 1).then_some(Owner {
                entry: 0x1000 + id * 0x100, row: 0x2000 + id * 0x100,
                information: 0x3000 + id * 0x100, object: 0x4000 + id * 0x100,
                index: 0, selector: id as u8, alias: 0,
            }));
            let extras = Extras::new_for(layout, [rows[7].unwrap().entry,
                rows[3].unwrap().entry, rows[2].unwrap().entry]).unwrap();
            let mut kinds = [Some(9), None, Some(9), Some(9), Some(0x4a), Some(9), Some(9), Some(9)];
            rows[4].as_mut().unwrap().index = 1; // native saved-Mii origin index
            assert_eq!(claims_in_with(layout, extras, &rows, &kinds),
                Some([Some(0), None, Some(2), Some(3), Some(4), Some(5), Some(6), Some(7)]));
            assert_eq!(claims_in(layout, &rows, &kinds), None);
            for foreign in [2, 3, 7] {
                let mut collision = rows; collision[foreign].as_mut().unwrap().index = 1;
                assert_eq!(claims_in_with(layout, extras, &collision, &kinds), None);
            }
            rows[0].as_mut().unwrap().index = 1; kinds[0] = Some(0x48);
            assert_eq!(claims_in_with(layout, extras, &rows, &kinds), None);
        }
    }

    fn registered_extras_obey_the_same_claims_and_only_extend_training() {
        use {crate::engine::mii::backend::Layout, crate::engine::mii::capacity::Extras};
        let mut rows = owners();
        let extra = |id: usize, index: u16| Some(Owner {
            entry: 0x1000 + id * 0x100, row: 0x2000 + id * 0x100,
            information: 0x3000 + id * 0x100, object: 0x4000 + id * 0x100,
            index, selector: id as u8, alias: 0,
        });
        rows[7] = extra(7, 7);
        rows[6] = extra(6, 0);
        let mut kinds = [Some(9), Some(9), Some(9), Some(9), None, None, None, None];
        kinds[7] = Some(9);
        kinds[6] = Some(u32::MAX); // created, not yet loaded
        let extras = Extras::new([rows[7].unwrap().entry, rows[6].unwrap().entry, 0]).unwrap();
        assert_eq!(claims_in_with(Layout::Training, extras, &rows, &kinds),
            Some([Some(0), Some(1), Some(2), Some(3), None, None, Some(6), Some(7)]));
        // Unregistered or partially registered extras stay unknown entries.
        assert_eq!(claims_in(Layout::Training, &rows, &kinds), None);
        assert_eq!(claims_in_with(Layout::Training, Extras::new([rows[7].unwrap().entry, 0, 0]).unwrap(), &rows, &kinds), None);
        // The same claim rules still reject a selector or alias mismatch on an extra.
        let mut changed = rows; changed[7].as_mut().unwrap().selector = 5;
        assert_eq!(claims_in_with(Layout::Training, extras, &changed, &kinds), None);
        changed = rows; changed[6].as_mut().unwrap().alias = 1;
        assert_eq!(claims_in_with(Layout::Training, extras, &changed, &kinds), None);
        // The four-owner result is unchanged without extras.
        let four = owners();
        let ordinary = [Some(9), Some(9), Some(9), Some(9), None, None, None, None];
        assert_eq!(claims_in_with(Layout::Training, Extras::NONE, &four, &ordinary), claims_in(Layout::Training, &four, &ordinary));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("origin_catalog_resolution_uses_unmasked_bytes_and_tokens", origin_catalog_resolution_uses_unmasked_bytes_and_tokens as fn()),
            ("placeholder_zero_does_not_claim_player_payload", placeholder_zero_does_not_claim_player_payload as fn()),
            ("changed_foreign_mii_requires_exact_current_committed_owner", changed_foreign_mii_requires_exact_current_committed_owner as fn()),
            ("css_ice_climbers_row_keeps_exact_ownership_for_mii_cycles", css_ice_climbers_row_keeps_exact_ownership_for_mii_cycles as fn()),
            ("ordinary_foreign_kind_preserves_original_row_anchor_for_both_roles", ordinary_foreign_kind_preserves_original_row_anchor_for_both_roles as fn()),
            ("stale_registration_never_narrows_unchanged_foreign_kind", stale_registration_never_narrows_unchanged_foreign_kind as fn()),
            ("numbered_duplicate_noop_cannot_certify_exact_original_payload", numbered_duplicate_noop_cannot_certify_exact_original_payload as fn()),
            ("waiting_extras_keep_native_mii_indices_disjoint", waiting_extras_keep_native_mii_indices_disjoint as fn()),
            ("registered_extras_obey_the_same_claims_and_only_extend_training", registered_extras_obey_the_same_claims_and_only_extend_training as fn()),
        ]);
    }
    }
}
