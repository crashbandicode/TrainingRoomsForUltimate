//! The transition target as the planner must see it (Astra REDESIGN §6.1, adopted in
//! `redesign/fable/REVIEW-OF-ASTRA.md` §1 item 9, WORKPLAN §2.2): an exact `Forms`, not a kind.
//!
//! FACT (FINAL-INT §6, B123-pre Ally row 4/O1): the group → pair-lead window return took a 3.2 s
//! canonical Reload because the request reached the planner as the kind `0x4b` and
//! `member_forms(0x4b)` is always Solo Popo, so the normal pair could never be group-reachable.
//! A [`Target`] carries the form, so "Original Ice Climbers pair" and "numbered Solo Popo" are two
//! different targets of the same kind, and a group serves a target only in the target's own form.

use super::capabilities::{capabilities, form_class, request_forms, Capabilities, FormClass};
use crate::engine::swap::body_bank::Members;
use crate::engine::swap::character_forms::Forms;
use crate::engine::mii::lifetime::Configuration;

/// The selectable member index of `members` that serves `kind` in `forms` (WP-I, WORKPLAN §2.13;
/// FINAL-INT §6 as a value rule): the group's own form of the kind must be the requested one
/// (`Members::forms_of`: the pair for the Popo of a parked pair-lead window or a pair group, Solo
/// for a Solo-led group's Popo, the native form for a PT/Aegis member, the single form otherwise).
/// `member_forms(kind)` alone never yields the pair, which is why the group → pair-lead window
/// return took the canonical Reload on B123-pre (Ally S4 L7002, 3.2 s). The adapter's
/// `group_reachable`, `switch` and the ready hint all use this one rule.
pub fn group_serves(members: Members, kind: u32, forms: Forms) -> Option<u8> {
    members.index_of(kind).filter(|_| members.forms_of(kind) == forms)
}

/// Where a request came from. Slot 0 is the original; numbered slots are the cycle lanes; a
/// direct pick (grid or menu) and a replay switch carry no slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    Original,
    Numbered(u8),
    Direct,
    Replay,
}

/// One requested fighter: the selection, its exact form (the pair or Solo for `0x4b`, the native
/// group expansion for a transformation form, the single body otherwise) and, for a Mii, the
/// exact configuration when one is staged. `kind` is `form.kinds[0]`, kept for the callers that
/// still key on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    pub selection: Selection,
    pub form: Forms,
    pub kind: u32,
    pub exact: Option<Configuration>,
}

impl Target {
    pub fn new(selection: Selection, form: Forms, exact: Option<Configuration>) -> Self {
        Self { selection, form, kind: form.kinds[0], exact }
    }

    /// From the descriptor words of a request (`+0x14` count, `+0x18..+0x24` kinds, `+0x28`
    /// actors): the canonical form they spell, or `None` for words that spell no form
    /// ([`request_forms`]). A target is never built from a kind alone.
    pub fn from_request(selection: Selection, count: u32, kinds: [u32; 3], actors: u32,
        exact: Option<Configuration>) -> Option<Self> {
        request_forms(count, kinds, actors).map(|form| Self::new(selection, form, exact))
    }

    pub fn class(&self) -> FormClass { form_class(self.kind, self.form) }

    pub fn capabilities(&self) -> Capabilities { capabilities(self.class()) }

    /// The descriptor words this target requests: `(count, kinds, actors)`.
    pub fn request_words(&self) -> (u32, [u32; 3], u32) {
        (self.form.count, self.form.kinds, self.form.actor_count)
    }

    /// Same physical fighter: same form and, for a Mii, same exact configuration. The selection
    /// (original vs numbered vs direct) is not part of it; the form is, so the original pair and a
    /// numbered Solo Popo are different fighters of one kind.
    pub fn same_fighter(&self, other: &Target) -> bool {
        self.form == other.form && self.exact == other.exact
    }

    pub fn is_pair(&self) -> bool { self.class() == FormClass::IcPair }

    pub fn is_solo_popo(&self) -> bool { self.class() == FormClass::SoloPopo }

    /// [`group_serves`] for this target: the member index of `members` that is this fighter.
    pub fn served_by(&self, members: Members) -> Option<u8> { group_serves(members, self.kind, self.form) }
}
