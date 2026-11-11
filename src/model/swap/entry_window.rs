//! P-HOT B (`artifacts/phase23/LANE-HOT-SLOTS-REPORT.md` §12; operator 2026-11-06: "Yes pursue B"): the window built
//! into Training's own start, the way the game builds Pokémon Trainer's three bodies. Values only: the native header
//! extension (H1, `main+0x66de18`), the creation rewrite (H3, `main+0x678174`) and the binding (H4, `0x654dc0`) are
//! the adapter's, behind their own fingerprint rows.

use super::body_bank::{member_lane, Members};
use super::character_forms::Forms;
use super::swap_model::capabilities::{bankable_kind, pins_cell0};

const NONE: u32 = u32::MAX;
const POPO: u32 = 0x4b;
const NANA: u32 = 0x4c;

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
/// requested (bit 0 clear), and either:
/// - no object kind, `+0x60` −1, and one ordinary body (`+0x5c` −1) of a bankable kind (not a Mii, not Solo Popo), or
///   the normal Ice Climbers pair (rc.6 item 3: native's header is Popo with Nana as its secondary; [`entry_window`]
///   makes it the pair-lead window);
/// - or a whole native group in its own order with its lead as the object kind (rc.6 item 3, a session that starts on
///   Pokémon Trainer or Pyra/Mythra: [`entry_window`] makes the group its own window lead).
pub fn extendable(facts: HeaderFacts) -> Option<u32> {
    let [original, second, third] = facts.kinds;
    let fresh = facts.training && facts.owner <= 1 && facts.flags & 1 == 0;
    let plain = facts.object == NONE && third == NONE;
    let one_body = plain && second == NONE && original != NONE && bankable_kind(original) && !matches!(original, POPO | NANA);
    let pair = plain && original == POPO && second == NANA;
    let native = native_lead(original).is_some_and(|group| group.kinds() == facts.kinds && facts.object == original);
    (fresh && (one_body || pair || native)).then_some(original)
}

/// The native group `original` leads in its own order (`Members::native_group`: Pokémon Trainer from Squirtle,
/// Pyra/Mythra from Pyra), or None.
fn native_lead(original: u32) -> Option<Members> {
    Members::native_group(original).filter(|group| group.kinds()[0] == original)
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
/// cell 0 (Kirby, Solo Popo) ahead of a non-pinned original is skipped, and the next one is taken. The Ice Climbers
/// pair (`original` Popo, from [`extendable`]'s pair header) gets the pair-lead window the Reload path builds for a
/// pair target (`Members::from_pair`: Nana and the first eligible member, two extra bodies). None (the scene stays
/// native's): no room or no eligible member, or an original that is neither one ordinary bankable body nor the pair.
pub fn entry_window(original: u32, numbered: &[u32], extras: usize, solo_popo_member: bool) -> Option<Members> {
    // A native group lead is its own window: no extra cell, and its bodies are native's own.
    if let Some(group) = native_lead(original) {
        return Some(group);
    }
    if original == POPO {
        let (lane, count) = member_lane(numbered, solo_popo_member);
        return Members::from_pair(&lane[..count], extras);
    }
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

/// H3: the creation descriptor is rewritten only when it is native's own form of the window's original: the single
/// form `(1, [members[0], −1, −1])`, or for a pair-lead window native's pair (`PAIR`, two actors, active 0). Anything
/// else (another caller's kind −1 creates, a group, an already-rewritten window, another fighter, Solo Popo) is left
/// as it is and the pending window is dropped.
pub fn creation_matches(native: Words, window: Words) -> bool {
    if window.kinds[..2] == [POPO, NANA] {
        return native == PAIR;
    }
    if keeps_native_creation(window) {
        return Words { active: 0, ..native } == window;
    }
    native.count == 1 && native.kinds == [window.kinds[0], NONE, NONE]
}

/// A native-group lead window ([`entry_window`]): H3 leaves native's creation as it is (its words are the window's,
/// and its active byte is native's starting form), so nothing is written.
pub fn keeps_native_creation(window: Words) -> bool {
    native_lead(window.kinds[0]).is_some_and(|group| Words::of(&group) == window)
}

/// Native's normal Ice Climbers creation: Popo and Nana, two actors, Popo active.
const PAIR: Words = Words { count: 2, kinds: [POPO, NANA, NONE], actor_count: 2, active: 0 };

/// The original's own request when its scene began with a start-up window. The origin both Mii adapters capture is the
/// live entry after H3, so it carries the window's words; staged as "back to the original", native was asked to reload
/// ordinary bodies whose resources only the start-up bank requests (rc6-hot-xps2: a construction read an unloaded
/// resource and its thread died) and the bank planner could not serve the original warm. The request is native's
/// single form H3 started from ([`creation_matches`]): count 1, `[original, −1, −1]`, one actor, active 0, every other
/// byte as captured. A pair-lead start-up window `[Popo, Nana, X]` goes back to native's pair (`PAIR`). Any other
/// words are native's own (one body, PT, the IC pair, Aegis, a Mii) and stay.
pub fn original_request(descriptor: [u8; 0x70], original: u32) -> [u8; 0x70] {
    let word = |at: usize| u32::from_le_bytes([descriptor[at], descriptor[at + 1], descriptor[at + 2], descriptor[at + 3]]);
    let words = Words { count: word(0x14), kinds: [word(0x18), word(0x1c), word(0x20)], actor_count: word(0x28),
        active: descriptor[0x43] };
    let form = if start_up_words(words, original) {
        Words { count: 1, kinds: [original, NONE, NONE], actor_count: 1, active: 0 }
    } else if original == POPO && pair_lead_words(words) {
        PAIR
    } else {
        return descriptor;
    };
    let mut request = descriptor;
    request[0x14..0x18].copy_from_slice(&form.count.to_le_bytes());
    for (i, kind) in form.kinds.iter().enumerate() {
        request[0x18 + i * 4..0x1c + i * 4].copy_from_slice(&kind.to_le_bytes());
    }
    request[0x28..0x2c].copy_from_slice(&form.actor_count.to_le_bytes());
    request[0x43] = form.active;
    request
}

/// The words H3 writes for a pair-lead window ([`entry_window`] for Popo): Popo, Nana and one bankable extra, two
/// actors, Popo active.
fn pair_lead_words(words: Words) -> bool {
    let extra = words.kinds[2];
    words == Words { count: 3, kinds: [POPO, NANA, extra], actor_count: 2, active: 0 }
        && extra != NONE && bankable_kind(extra) && !matches!(extra, POPO | NANA)
}

/// The words H3 writes for `original` ([`entry_window`], [`Words::of`]): two or three single-body bankable members
/// (never the IC pair), the original at cell 0, one actor, active 0.
fn start_up_words(words: Words, original: u32) -> bool {
    let count = words.count as usize;
    (2..=3).contains(&count) && words.kinds[0] == original && words.actor_count == 1 && words.active == 0
        && words.kinds[..count].iter().all(|kind| *kind != NONE && bankable_kind(*kind) && !matches!(*kind, 0x4b | 0x4c))
        && words.kinds[count..].iter().all(|kind| *kind == NONE)
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

