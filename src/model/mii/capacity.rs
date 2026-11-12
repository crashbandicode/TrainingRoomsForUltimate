//! Native created-entry contract shared by cache scheduling (re-exported by
//! `crate::engine::preload::cycle_preload_capacity`) and Mii owner checks.
//! Values only: an `Extras` never authorizes a dereference. Callers still apply
//! every physical FE/E/D/O, index, alias and mirror check to a created entry.
use crate::engine::mii::inventory::MiiClass;
use crate::engine::mii::ordinary_player::Owner;
use crate::engine::mii::backend::Layout;

/// The two parked helpers are part of each witnessed native practice layout.
pub const BASE_HELPERS: usize = 2;
/// Native entry identities in serial creation order, never menu ordinal values.
pub const EXTRA_SLOTS: [usize; 4] = [7, 6, 5, 4];
/// Waiting has native helpers 5/6 already. Extra resources use a separate
/// creation plan, admitted only while the corresponding rows and inputs are
/// unclaimed. These values alone never authorize creation or a native read.
pub const WAITING_EXTRA_SLOTS: [usize; 3] = [7, 3, 2];
/// Stable physical identities for retirement across a scene/layout transition.
/// The active budget remains the length of one plan, not this union.
pub const ALL_EXTRA_SLOTS: [usize; 6] = [7, 6, 5, 4, 3, 2];
/// Saved speculative preload budget. Renderer demand does not add a menu choice.
pub const MAX_TARGETS: usize = 5;
/// Physical candidate storage includes the fourth, purpose-created renderer.
pub const SOURCE_SLOTS: usize = BASE_HELPERS + EXTRA_SLOTS.len();

pub fn capacity_index(slot: usize) -> Option<usize> {
    ALL_EXTRA_SLOTS.iter().position(|candidate| *candidate == slot)
}

pub const fn extra_slots(layout: Layout) -> &'static [usize] {
    match layout {
        Layout::Training => &EXTRA_SLOTS,
        Layout::Ldn | Layout::Quickplay => &WAITING_EXTRA_SLOTS,
    }
}

/// One registered creation's native roots, numeric identities only: the
/// resource root `context+slot*8`, alias root `context+0xe8+slot*8`, the
/// selection row byte `+0xa` and creation descriptor word 1 (entry `+4`), as
/// recorded by the creating capacity module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Roots {
    pub entry: usize,
    pub resource: usize,
    pub alias: usize,
    pub selection: u8,
    pub descriptor_selector: u32,
}

/// Raw current facts of one created entry. The native caller reads them from
/// the current FE/E/D/O and owner-table roots under its lock; values only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtraFacts {
    /// FE `+0xe0` and E `+0x14`.
    pub entry_owner: u32,
    pub row_owner: u32,
    /// FE `+4`, descriptor word 1.
    pub descriptor_selector: u32,
    /// FE `+0x46`, D `+0x72` and O `+0x68`; the row index is `Owner::index`.
    pub entry_index: u16,
    pub information_index: u16,
    pub object_index: u16,
    /// FE `+0x18` and O `+0x58`.
    pub entry_kind: u32,
    pub object_kind: u32,
    /// Owner table `+slot*8` and `+0xe8+slot*8`, and E `+0xa`.
    pub resource: usize,
    pub alias_root: usize,
    pub selection: u8,
}

/// Registered created entries by `EXTRA_SLOTS` position (0 = absent): a
/// creation-order prefix of distinct nonzero entry identities, with their
/// registered roots when built by the owning registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Extras {
    layout: Layout,
    entries: [usize; EXTRA_SLOTS.len()],
    /// `None` for identity-only values; `owned_roots` and `admit` refuse them.
    roots: [Option<Roots>; EXTRA_SLOTS.len()],
}

impl Extras {
    pub const NONE: Self = Self {
        layout: Layout::Training,
        entries: [0; EXTRA_SLOTS.len()],
        roots: [None; EXTRA_SLOTS.len()],
    };

    /// Entry identities only (value policy and tests). `owned_roots` and
    /// `admit` refuse every extra of such a value.
    #[cfg(test)]
    pub fn new<const N: usize>(entries: [usize; N]) -> Option<Self> {
        Self::new_for(Layout::Training, entries)
    }

    pub fn new_for<const N: usize>(layout: Layout, entries: [usize; N]) -> Option<Self> {
        if N > EXTRA_SLOTS.len() { return None; }
        let mut padded = [0; EXTRA_SLOTS.len()];
        padded[..N].copy_from_slice(&entries);
        let entries = padded;
        let count = entries.iter().take_while(|entry| **entry != 0).count();
        if count > extra_slots(layout).len() || entries[count..].iter().any(|entry| *entry != 0)
            || (0..count).any(|index| entries[..index].contains(&entries[index]))
        {
            return None;
        }
        Some(Self {
            layout,
            entries,
            roots: [None; EXTRA_SLOTS.len()],
        })
    }

    /// The owning registry's records, including their roots.
    #[cfg(test)]
    pub fn registered<const N: usize>(records: [Option<Roots>; N]) -> Option<Self> {
        Self::registered_for(Layout::Training, records)
    }

    pub fn registered_for<const N: usize>(layout: Layout, records: [Option<Roots>; N]) -> Option<Self> {
        let mut extras = Self::new_for(layout, records.map(|record| record.map_or(0, |record| record.entry)))?;
        for (index, record) in records.iter().enumerate() {
            if let Some(record) = record {
                if record.resource == 0 || record.alias == 0 {
                    return None;
                }
                extras.roots[index] = Some(*record);
            }
        }
        Some(extras)
    }

    fn registered_roots(self, slot: usize) -> Result<Roots, &'static str> {
        let Some(index) = extra_slots(self.layout).iter().position(|value| *value == slot)
            .filter(|index| self.entries[*index] != 0) else {
            return Err("extra slot is not a registered creation");
        };
        self.roots[index].ok_or("extra registered roots unavailable")
    }

    /// Compare this creation's complete registration without treating another
    /// slot's later creation as replacement of this source's own roots.
    pub fn same_registration(self,other:Self,slot:usize)->bool {
        self.layout==other.layout && matches!((self.registered_roots(slot),other.registered_roots(slot)),
            (Ok(a),Ok(b)) if a==b)
    }

    /// A registered extra's current native roots must equal its registration.
    /// Raw values only; the caller reads them from current roots under its lock.
    pub fn owned_roots(
        self,
        slot: usize,
        resource: usize,
        alias: usize,
        selection: u8,
    ) -> Result<(), &'static str> {
        let roots = self.registered_roots(slot)?;
        if resource != roots.resource {
            return Err("extra resource root differs from its registration");
        }
        if alias != roots.alias {
            return Err("extra alias root differs from its registration");
        }
        if selection != roots.selection {
            return Err("extra selection byte differs from its registration");
        }
        Ok(())
    }

    /// The one admission rule for a registered created entry beside the layout
    /// owners, shared by the Player0 and CPU selector owner walks (helper proofs
    /// never admit an extra). Every check is an exact equality:
    /// - roots, owner ids, owner object, indices and kinds as for any owner;
    /// - descriptor word 1 equals the **registered creation descriptor** word.
    ///   Native creation does not mirror the row selector there (C98b entry7:
    ///   word 0, row selector 7), and every capacity load preflight keeps the
    ///   creation descriptor except kind/count/actor words, byte 0x43 and the
    ///   reset reference pair. A layout owner instead requires word 1 == its
    ///   row selector; this specialization never applies to unknown entries.
    /// - then the ordinary-only claim facts of `extra_owner`.
    pub fn admit(self, slot: usize, owner: &Owner, facts: ExtraFacts) -> Result<(), &'static str> {
        self.owned_roots(slot, facts.resource, facts.alias_root, facts.selection)?;
        let roots = self.registered_roots(slot)?;
        if owner.entry != roots.entry {
            return Err("extra entry differs from its registration");
        }
        if facts.entry_owner != slot as u32 || facts.row_owner != slot as u32 {
            return Err("extra entry/row owner differs from its slot");
        }
        if owner.object != facts.alias_root {
            return Err("extra owner object is not its registered alias root");
        }
        if facts.descriptor_selector != roots.descriptor_selector {
            return Err("extra descriptor selector differs from its creation descriptor");
        }
        if facts.entry_index != owner.index
            || facts.information_index != owner.index
            || facts.object_index != owner.index
        {
            return Err("extra index mirrors differ");
        }
        if facts.entry_kind != facts.object_kind {
            return Err("extra entry/object kind differs");
        }
        extra_owner(slot, owner, facts.entry_kind)
    }

    pub fn count(self) -> usize {
        self.entries.iter().filter(|entry| **entry != 0).count()
    }

    pub fn entry(self, slot: usize) -> Option<usize> {
        extra_slots(self.layout).iter().position(|value| *value == slot)
            .map(|index| self.entries[index])
            .filter(|entry| *entry != 0)
    }

    /// `(slot, entry)` for each registered creation, in creation order.
    pub fn slots(self) -> impl Iterator<Item = (usize, usize)> {
        extra_slots(self.layout)
            .iter().copied()
            .zip(self.entries)
            .filter(|(_, entry)| *entry != 0)
    }

    /// Never reinterpret one scene's registration as another scene's authority.
    pub fn matches(self, layout: Layout) -> bool {
        self.count() == 0 || self.layout == layout
    }

    /// The four-owner view of a physical map. Each registered slot must hold
    /// exactly its registered entry and is zeroed; every other slot is kept, so an
    /// unknown entry still fails the layout shape.
    pub fn view(self, physical: &[usize; 8]) -> Option<[usize; 8]> {
        let mut view = *physical;
        for (slot, entry) in self.slots() {
            if physical[slot] != entry {
                return None;
            }
            view[slot] = 0;
        }
        Some(view)
    }

    /// The physical map implied by a view; the inverse of `view`.
    pub fn physical(self, view: &[usize; 8]) -> [usize; 8] {
        let mut physical = *view;
        for (slot, entry) in self.slots() {
            physical[slot] = entry;
        }
        physical
    }
}

/// Strict facts a created entry's current owner must satisfy before Mii code
/// accepts it beside the four layout owners. These are the claim rules applied
/// to every present owner, checked first so a refusal names its predicate. An
/// extra is never a Mii owner, selector destination or helper ordinal.
pub fn extra_owner(slot: usize, owner: &Owner, kind: u32) -> Result<(), &'static str> {
    if capacity_index(slot).is_none() {
        return Err("extra slot outside the created-entry order");
    }
    if MiiClass::from_kind(kind).is_some() {
        return Err("extra entry holds a Mii class");
    }
    if usize::from(owner.selector) != slot {
        return Err("extra row selector differs from its slot");
    }
    if owner.alias != 0 {
        return Err("extra owner alias is nonzero");
    }
    if owner.index != 0 && usize::from(owner.index) != slot {
        return Err("extra row index is neither 0 nor its slot");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    /// C98b live read-only probe of created entry7 after an ordinary Fox load
    /// (`preload168/extra7-mirrors-c98b.json`), addresses normalized: FE+e0=7,
    /// E+14=7, FE+4=0 while E+a7=7, all indices 0, FE/O kind 7, alias byte 0,
    /// selection 2, resource root == alias root == O.
    fn c98b_extra7() -> (Extras, Owner, ExtraFacts) {
        let (entry, object) = (0x7000, 0x7700);
        let extras = Extras::registered([
            Some(Roots {
                entry,
                resource: object,
                alias: object,
                selection: 2,
                descriptor_selector: 0,
            }),
            None,
            None,
        ])
        .unwrap();
        let owner = Owner {
            entry,
            row: 0x7800,
            information: entry + 0x110,
            object,
            index: 0,
            selector: 7,
            alias: 0,
        };
        let facts = ExtraFacts {
            entry_owner: 7,
            row_owner: 7,
            descriptor_selector: 0,
            entry_index: 0,
            information_index: 0,
            object_index: 0,
            entry_kind: 7,
            object_kind: 7,
            resource: object,
            alias_root: object,
            selection: 2,
        };
        (extras, owner, facts)
    }

    fn extra_admission_names_each_unknown_mismatch() {
        let (extras, owner, facts) = c98b_extra7();
        let refuse = |owner: Owner, facts: ExtraFacts| extras.admit(7, &owner, facts).unwrap_err();
        assert_eq!(
            extras.admit(6, &owner, facts),
            Err("extra slot is not a registered creation")
        );
        assert_eq!(
            refuse(
                owner,
                ExtraFacts {
                    resource: facts.resource + 8,
                    ..facts
                }
            ),
            "extra resource root differs from its registration"
        );
        assert_eq!(
            refuse(
                owner,
                ExtraFacts {
                    alias_root: facts.alias_root + 8,
                    ..facts
                }
            ),
            "extra alias root differs from its registration"
        );
        assert_eq!(
            refuse(
                owner,
                ExtraFacts {
                    selection: 1,
                    ..facts
                }
            ),
            "extra selection byte differs from its registration"
        );
        assert_eq!(
            refuse(Owner { entry: owner.entry + 8, ..owner }, facts),
            "extra entry differs from its registration"
        );
        assert_eq!(
            refuse(
                owner,
                ExtraFacts {
                    entry_owner: 0,
                    ..facts
                }
            ),
            "extra entry/row owner differs from its slot"
        );
        assert_eq!(
            refuse(
                owner,
                ExtraFacts {
                    row_owner: 6,
                    ..facts
                }
            ),
            "extra entry/row owner differs from its slot"
        );
        assert_eq!(
            refuse(
                Owner {
                    object: owner.object + 8,
                    ..owner
                },
                facts
            ),
            "extra owner object is not its registered alias root"
        );
        for facts in [
            ExtraFacts {
                entry_index: 7,
                ..facts
            },
            ExtraFacts {
                information_index: 7,
                ..facts
            },
            ExtraFacts {
                object_index: 7,
                ..facts
            },
        ] {
            assert_eq!(refuse(owner, facts), "extra index mirrors differ");
        }
        assert_eq!(
            refuse(
                owner,
                ExtraFacts {
                    object_kind: 8,
                    ..facts
                }
            ),
            "extra entry/object kind differs"
        );
        assert_eq!(
            refuse(
                owner,
                ExtraFacts {
                    entry_kind: 0x48,
                    object_kind: 0x48,
                    ..facts
                }
            ),
            "extra entry holds a Mii class"
        );
        assert_eq!(
            refuse(
                Owner {
                    selector: 0,
                    ..owner
                },
                facts
            ),
            "extra row selector differs from its slot"
        );
        assert_eq!(
            refuse(Owner { alias: 1, ..owner }, facts),
            "extra owner alias is nonzero"
        );
        assert_eq!(
            refuse(
                Owner { index: 2, ..owner },
                ExtraFacts {
                    entry_index: 2,
                    information_index: 2,
                    object_index: 2,
                    ..facts
                }
            ),
            "extra row index is neither 0 nor its slot"
        );
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("extra_admission_names_each_unknown_mismatch", extra_admission_names_each_unknown_mismatch as fn()),
        ]);
    }
    }
}
