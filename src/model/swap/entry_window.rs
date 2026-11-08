//! P-HOT B (`artifacts/phase23/LANE-HOT-SLOTS-REPORT.md` §12; operator 2026-11-06: "Yes pursue B"): the window built
//! into Training's own start, the way the game builds Pokémon Trainer's three bodies. Values only: the native header
//! extension (H1, `main+0x66de18`), the creation rewrite (H3, `main+0x678174`) and the binding (H4, `0x654dc0`) are
//! the adapter's, behind their own fingerprint rows.

use super::body_bank::{member_lane, Members};
use super::character_forms::Forms;
use super::swap_model::capabilities::{bankable_kind, pins_cell0};

const NONE: u32 = u32::MAX;

/// What H1 reads of a fighter's resource owner R at `0x66de18`, after the scene's configure stored its header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeaderFacts {
    /// Native Training (never online, never a waiting room).
    pub training: bool,
    /// `R+0x54`.
    pub owner: u32,
    /// `R+0x4c`.
    pub flags: u8,
    /// `R+0x80`: the PT/Aegis object kind, else −1.
    pub object: u32,
    /// `R+0x58`, `+0x5c`, `+0x60`.
    pub kinds: [u32; 3],
}

/// The original H1 may extend (§12.2 H1's guards): a visible role (owner 0 or 1) in native Training, R not yet
/// requested (bit 0 clear), one ordinary body (no object kind, `+0x5c`/`+0x60` −1) of a bankable kind (not a Mii,
/// not the IC pair or Solo Popo, not PT/Aegis).
pub fn extendable(facts: HeaderFacts) -> Option<u32> {
    let [original, second, third] = facts.kinds;
    (facts.training && facts.owner <= 1 && facts.flags & 1 == 0 && facts.object == NONE && second == NONE
        && third == NONE && original != NONE && bankable_kind(original) && !matches!(original, 0x4b | 0x4c))
        .then_some(original)
}

/// Extra bodies a role's start-up window may take from the Ready Fighters budget: both roles share it evenly, the
/// two active fighters counted, never more than the bodies setting allows.
pub fn extras_allowed(ready_fighters: u8, bank_members: u8) -> usize {
    if bank_members < 2 {
        return 0;
    }
    (usize::from(ready_fighters).saturating_sub(2) / 2).min(usize::from(bank_members) - 1)
}

/// The window for a role whose scene header names `original` (its slot 0): the original at cell 0 (native's own
/// primary request, `R+0x58`) and the next members of the role's numbered cycle, at most `extras`. A member that pins
/// cell 0 (Kirby, Solo Popo) ahead of a non-pinned original is skipped, and the next one is taken. None (the scene
/// stays native's one body): no extra member, or an original that is not one ordinary bankable body.
pub fn entry_window(original: u32, numbered: &[u32], extras: usize, solo_popo_member: bool) -> Option<Members> {
    if extras == 0 || !bankable_kind(original) {
        return None;
    }
    let (lane, count) = member_lane(numbered, solo_popo_member);
    let mut kept = [NONE; 5];
    let mut n = 0;
    for kind in lane[..count].iter().copied().filter(|kind| pins_cell0(original) || !pins_cell0(*kind)) {
        if n < kept.len() {
            kept[n] = kind;
            n += 1;
        }
    }
    let members = Members::from_forms(Forms::starting(original), &kept[..n], extras)?;
    (members.kinds()[0] == original).then_some(members)
}

/// The words H1 writes into R's header and H3 into the creation descriptor (count `+0x14`, kinds `+0x18..`, actor
/// count `+0x28`, active `+0x43` = 0: the original's cell).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Words {
    pub count: u32,
    pub kinds: [u32; 3],
    pub actor_count: u32,
    pub active: u8,
}

impl Words {
    pub fn of(members: &Members) -> Self {
        let mut kinds = [NONE; 3];
        kinds[..members.count()].copy_from_slice(&members.kinds()[..members.count()]);
        Self { count: members.count() as u32, kinds, actor_count: members.actor_count(), active: 0 }
    }
}

/// H3: the creation descriptor is rewritten only when it is native's single form of the window's original:
/// `(1, [members[0], −1, −1])`. Anything else (another caller's kind −1 creates, a group, an already-rewritten
/// window, another fighter) is left as it is and the pending window is dropped.
pub fn creation_matches(native: Words, window: Words) -> bool {
    native.count == 1 && native.kinds == [window.kinds[0], NONE, NONE]
}

/// A window published by H1, before the entry exists: keyed by owner and its words, for the scene epoch it was made
/// in. H4 binds it to the first constructed entry of that owner whose descriptor shows exactly these words, in the
/// same epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pending<E> {
    pub owner: u32,
    pub words: Words,
    pub epoch: E,
}

impl<E: Copy + PartialEq> Pending<E> {
    pub fn binds(&self, owner: u32, words: Words, epoch: E) -> bool {
        self.owner == owner && self.words == words && self.epoch == epoch
    }
}

/// The owner generation a bank's group exchange must show. H1 stamps the window with the generation current while the
/// new scene is configured; that scene's own scene change opens the next one after the bank's step. A window the bank
/// step kept for the new scene (`kept`) takes the new generation; any other bank keeps its stamp, so a group exchange
/// from an earlier scene stays refused.
pub fn scene_generation(stamped: u64, current: u64, kept: bool) -> u64 {
    if kept { current } else { stamped }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MARIO: u32 = 0x0;
    const DK: u32 = 0x1;
    const LINK: u32 = 0x2;
    const KIRBY: u32 = 0x6;
    const LITTLE_MAC: u32 = 0x34;
    const STEVE: u32 = 0x58;

    fn facts(owner: u32, kinds: [u32; 3]) -> HeaderFacts {
        HeaderFacts { training: true, owner, flags: 0, object: NONE, kinds }
    }

    /// tr-p23-hot-measure-3 at `0x66de18`: the CPU `{flags 0, owner 1, [40,−1,−1], object −1}` is extendable; PT
    /// `{owner 0, [24,25,26], object 0x24}` is not (the game builds its group itself).
    #[test]
    fn h1_extends_only_one_ordinary_body_of_a_visible_role_in_training() {
        assert_eq!(extendable(facts(1, [0x40, NONE, NONE])), Some(0x40));
        assert_eq!(extendable(facts(0, [STEVE, NONE, NONE])), Some(STEVE));
        assert_eq!(extendable(HeaderFacts { object: 0x24, ..facts(0, [0x24, 0x25, 0x26]) }), None, "Pokémon Trainer");
        assert_eq!(extendable(HeaderFacts { object: 0x5a, ..facts(0, [0x5a, 0x5b, NONE]) }), None, "Aegis");
        assert_eq!(extendable(facts(0, [0x4b, 0x4c, NONE])), None, "the IC pair");
        assert_eq!(extendable(facts(0, [0x4b, NONE, NONE])), None, "Solo Popo");
        for mii in [0x48, 0x49, 0x4a] {
            assert_eq!(extendable(facts(1, [mii, NONE, NONE])), None, "{mii:#x}");
        }
        assert_eq!(extendable(facts(2, [0x40, NONE, NONE])), None, "a parked helper");
        assert_eq!(extendable(facts(7, [0x40, NONE, NONE])), None, "an extra");
        assert_eq!(extendable(HeaderFacts { flags: 1, ..facts(1, [0x40, NONE, NONE]) }), None, "already requested");
        assert_eq!(extendable(HeaderFacts { training: false, ..facts(1, [0x40, NONE, NONE]) }), None, "not Training");
    }

    /// Ready Fighters is shared by both roles, the two active fighters counted; the bodies setting caps a window.
    #[test]
    fn both_start_up_windows_share_ready_fighters() {
        assert_eq!(extras_allowed(10, 3), 2, "10: two windows of three");
        assert_eq!(extras_allowed(6, 3), 2);
        assert_eq!(extras_allowed(4, 3), 1, "4: two windows of two");
        assert_eq!(extras_allowed(3, 3), 0);
        assert_eq!(extras_allowed(2, 3), 0, "2: today's path");
        assert_eq!(extras_allowed(12, 2), 1, "two bodies");
        assert_eq!(extras_allowed(12, 0), 0, "bodies off");
    }

    /// tr-p23-hot-measure-1: the CPU entered as Little Mac with the cycle Mario, DK, Link, …: the window is Little Mac
    /// and his next two, Little Mac at cell 0 (native's own primary request) and active.
    #[test]
    fn the_start_up_window_is_the_original_and_its_next_members() {
        let members = entry_window(LITTLE_MAC, &[MARIO, DK, LINK], 2, false).unwrap();
        assert_eq!(&members.kinds()[..members.count()], &[LITTLE_MAC, MARIO, DK]);
        let words = Words::of(&members);
        assert_eq!((words.count, words.kinds, words.active), (3, [LITTLE_MAC, MARIO, DK], 0));
        let two = entry_window(STEVE, &[MARIO, DK], 1, false).unwrap();
        assert_eq!(&two.kinds()[..two.count()], &[STEVE, MARIO]);
    }

    /// The original stays at cell 0: a Kirby member ahead of an ordinary original is skipped; a Kirby original keeps
    /// cell 0 itself.
    #[test]
    fn the_original_keeps_cell_0_and_a_pinned_member_waits() {
        let members = entry_window(MARIO, &[KIRBY, DK, LINK], 2, false).unwrap();
        assert_eq!(&members.kinds()[..members.count()], &[MARIO, DK, LINK]);
        let kirby = entry_window(KIRBY, &[MARIO, DK], 2, false).unwrap();
        assert_eq!(&kirby.kinds()[..kirby.count()], &[KIRBY, MARIO, DK]);
    }

    /// No window: no room, no bankable next member, or an original that is not one ordinary body.
    #[test]
    fn no_start_up_window_without_room_members_or_an_ordinary_original() {
        assert_eq!(entry_window(MARIO, &[DK, LINK], 0, false), None, "no room");
        assert_eq!(entry_window(MARIO, &[], 2, false), None, "no next member");
        assert_eq!(entry_window(MARIO, &[0x48, 0x49], 2, false), None, "Miis are never window members");
        for original in [0x48, 0x24, 0x5a, 0x4b] {
            assert_eq!(entry_window(original, &[MARIO, DK], 2, false), None, "{original:#x}");
        }
    }

    /// H3: only native's single form of the window's original is rewritten; H4: the window binds only to the same
    /// owner's entry showing the written words in the same scene.
    #[test]
    fn the_creation_is_rewritten_and_bound_only_for_its_own_window() {
        let window = Words::of(&entry_window(LITTLE_MAC, &[MARIO, DK], 2, false).unwrap());
        let native = Words { count: 1, kinds: [LITTLE_MAC, NONE, NONE], actor_count: 1, active: 0 };
        assert!(creation_matches(native, window));
        assert!(!creation_matches(Words { kinds: [NONE; 3], ..native }, window), "kind -1 creates");
        assert!(!creation_matches(Words { kinds: [MARIO, NONE, NONE], ..native }, window), "another fighter");
        assert!(!creation_matches(window, window), "already a window");
        let pending = Pending { owner: 1, words: window, epoch: 7u64 };
        assert!(pending.binds(1, window, 7));
        assert!(!pending.binds(0, window, 7), "another owner");
        assert!(!pending.binds(1, native, 7), "the native words");
        assert!(!pending.binds(1, window, 8), "a later scene");
    }

    /// rc6-hot-xps1 (2026-11-08, XPS): H1 stamps the owner generation of the scene being configured, and that scene's
    /// own `scene_changed` advances it afterwards. A window kept for the new scene follows it; nothing else does.
    #[test]
    fn a_kept_start_up_window_joins_its_scenes_owner_generation() {
        let (at_h1, after_the_change) = (4u64, 5u64);
        assert_eq!(scene_generation(at_h1, after_the_change, true), after_the_change, "kept for the new scene");
        assert_eq!(scene_generation(at_h1, after_the_change, false), at_h1,
            "departed or retired: the old stamp, so an exchange from an earlier scene stays refused");
        assert_eq!(scene_generation(after_the_change, after_the_change, true), after_the_change, "already current");
    }

    /// Primary 12:40Z (cov-rest `covrest-02`, rc.5 diag): the CPU cycle Mario, Mii Brawler, Swordfighter, Gunner, Link
    /// fell back the same way on its first press, a Mii group exchange out of the CPU's start-up window (the Miis are
    /// groups: `cycle_members` leaves them out of the window). Any group exchange starts from the receipt
    /// `group_owner` mints only while the bank's stamp is the owner's current generation, and a Mii exchange keeps that
    /// generation. With the stamp the kept window adopts, the Mii exchange is proven; with the stamp H1 left, there is
    /// no receipt to start from.
    #[test]
    fn a_cpu_mii_group_exchange_out_of_a_kept_window_after_the_scene_change() {
        use crate::engine::swap::body_bank::{GroupExchange, GroupOwner};
        let (at_h1, after_the_change) = (1u64, 2u64);
        let window = entry_window(MARIO, &[LINK, DK], 2, false).unwrap();
        let receipt = |stamp: u64| (stamp == after_the_change).then(|| {
            let mut descriptor = [0u8; 0x70];
            descriptor[0x14..0x18].copy_from_slice(&(window.count() as u32).to_le_bytes());
            for (i, kind) in window.kinds().iter().enumerate() {
                descriptor[0x18 + i * 4..0x1c + i * 4].copy_from_slice(&kind.to_le_bytes());
            }
            descriptor[0x28] = 1;
            GroupOwner { entry: 0x1000, owner: 1, epoch: after_the_change, resource: 0x2000,
                header: [1, MARIO, LINK, DK, 0], descriptor, bodies: [(0x3000, 1), (0x4000, 262144), (0x5000, 327680)] }
        });
        assert!(receipt(at_h1).is_none(), "the stamp H1 left: no receipt, the fallback rc6-hot-xps1 and covrest-02 saw");
        let before = receipt(scene_generation(at_h1, after_the_change, true)).expect("the kept window's receipt");
        let brawler = Members::mii_group(0x48).unwrap();
        let mii_header = [1, 0x48, NONE, NONE, 0];
        let mut after = GroupOwner { header: mii_header, ..before };
        after.descriptor[0x14..0x18].copy_from_slice(&1u32.to_le_bytes());
        after.descriptor[0x18..0x24].copy_from_slice(&[0x48, 0, 0, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
        after.descriptor[0x28..0x2c].copy_from_slice(&1u32.to_le_bytes());
        assert!(GroupExchange::new_with_header(before, after, brawler, mii_header).is_some(), "the Mii group moves in");
        assert!(GroupExchange::new_with_header(before, GroupOwner { epoch: at_h1, ..after }, brawler, mii_header).is_none(),
            "a generation that differs across the exchange refuses");
    }
}
