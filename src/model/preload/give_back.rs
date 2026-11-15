//! rc.14 mem-headroom (F2, the primary 2026-11-15 ~06:20Z): which speculative helper entries give their memory back to a
//! cycle group held for it. Not carried.
//!
//! A cycle group whose prefetch the process cannot hold waits (`GroupState::Held(Hold::Memory)`). Parked groups that
//! left the cycle go first (`body_bank::capacity_release`). With none left, the speculative helpers the plugin created
//! (Kirby's preloaded hats first, then fighter look-ahead, on created entries) hold the room; they kept it for the whole
//! scene before (Eden smoke2r-2: four created entries, about 250 MB, and "Loaded 7/9 (2 waiting for memory)" until the
//! lane changed). The farthest of them are marked and retired when the pause opens, through the pause-time retire
//! (`model::pause_helper`); never mid-game. This module decides; it never touches memory.

/// One created helper entry that holds a loaded source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// The FighterManager slot of the created entry (4-7).
    pub slot: u8,
    /// Its source's place in the preload plan, 0 the nearest; None when the plan no longer wants it.
    pub place: Option<u8>,
    /// What its load costs the process (`memory::helper_cost`, the measured upper bound).
    pub cost: u64,
    /// Nothing uses it: a Ready source, no load in flight, no lease (a hat on Kirby, an item, a portrait), no visible
    /// request for its kind.
    pub idle: bool,
}

/// The slots (one bit per slot) to retire at the next pause for a group short of `shortfall` bytes: idle helpers, the
/// ones the plan no longer wants first, then from the farthest place, until their costs cover the shortfall. None of
/// them when every idle helper together would not cover it: they stay loaded (what doesn't fit stays cold, what fits
/// stays hot).
pub fn choose(candidates: &[Candidate], shortfall: u64) -> u8 {
    let _ = (candidates, shortfall);
    0
}

/// At the pause: the marked slots retired now. Only while the cycle still waits for memory, and only slots still idle.
pub fn at_pause(marked: u8, idle: u8, cycle_waits: bool) -> u8 {
    let _ = (marked, idle, cycle_waits);
    0
}

/// One helper entry in the census: its slot, its source's kind, Ready, leased, marked to give its memory back.
#[cfg(any(test, feature = "diag"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Helper {
    pub slot: u8,
    pub kind: u32,
    pub ready: bool,
    pub leased: bool,
    pub marked: bool,
}

/// The diag build's per-consumer memory line after each `[BodyBank] ready` (the primary 2026-11-15: so the final pass can
/// measure where the process budget goes): the process charge, the fighter pool, each role's bank bodies (window and
/// groups), Training's seats and the created helpers, and the module references the bank holds.
#[cfg(any(test, feature = "diag"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Census {
    pub owner: u32,
    pub headroom: Option<u64>,
    pub committed: Option<u64>,
    /// (live, capacity) of the game's fighter pool.
    pub pool: Option<(u16, u32)>,
    /// Per role (Player, CPU): (window bodies, group bodies).
    pub bank: [(u8, u8); 2],
    /// Seats 2 and 3, then the created entries, in slot order.
    pub helpers: [Option<Helper>; 8],
    /// Per role: kinds whose fighter module a kept group pins (`module_pins`).
    pub pins: [u8; 2],
    /// Groups whose holder keeps its object kind's module (`object_module`).
    pub objects: u8,
}

#[cfg(any(test, feature = "diag"))]
impl std::fmt::Display for Census {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[MemoryCensus] owner={} headroom={:?} committed={:?}", self.owner, self.headroom, self.committed)?;
        match self.pool {
            Some((live, capacity)) => write!(f, " pool={}/{}", live, capacity)?,
            None => f.write_str(" pool=na")?,
        }
        write!(f, " bank=p1:{}+{},cpu:{}+{}", self.bank[0].0, self.bank[0].1, self.bank[1].0, self.bank[1].1)?;
        for (label, seats) in [(" seats=[", true), (" created=[", false)] {
            f.write_str(label)?;
            let mut first = true;
            for helper in self.helpers.iter().flatten().filter(|h| matches!(h.slot, 2 | 3) == seats) {
                if !first { f.write_str(", ")?; }
                first = false;
                write!(f, "{}:{:#x} {}", helper.slot, helper.kind, if helper.ready { "ready" } else { "loading" })?;
                if helper.leased { f.write_str(" leased")?; }
                if helper.marked { f.write_str(" marked")?; }
            }
            f.write_str("]")?;
        }
        write!(f, " pins=p1:{},cpu:{} objects={}", self.pins[0], self.pins[1], self.objects)
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    const MB: u64 = 1 << 20;

    fn helper(slot: u8, place: Option<u8>, cost_mb: u64, idle: bool) -> Candidate {
        Candidate { slot, place, cost: cost_mb * MB, idle }
    }

    /// Eden smoke2r-2 a2-p4 (#6769): the Aegis group `[5a, 5b]` was held at 136 MB of headroom against a need of 226,
    /// with four created entries loaded (`[MiiHelper]` #6576, slots 4-7). The helpers the plan no longer wants go first,
    /// then the farthest place; they stop once their costs cover the shortfall; a helper in use never goes; and nothing
    /// goes when every idle helper together would not make the room.
    fn the_farthest_idle_helpers_go_first_until_the_shortfall_is_covered() {
        let helpers = [helper(4, Some(0), 62, true), helper(5, Some(2), 52, true), helper(6, None, 31, true),
            helper(7, Some(1), 40, true)];
        // 90 MB short: the stale slot 6 (31), then the farthest place, slot 5 (52): 83, not yet; then slot 7 (40).
        assert_eq!(choose(&helpers, 90 * MB), 1 << 6 | 1 << 5 | 1 << 7);
        // 30 MB short: the stale helper alone covers it.
        assert_eq!(choose(&helpers, 30 * MB), 1 << 6);
        // A helper in use (a hat on Kirby, a load in flight) never goes; the next farthest takes its turn.
        let worn = [helper(4, Some(0), 62, true), helper(5, Some(2), 52, false), helper(6, None, 31, true)];
        assert_eq!(choose(&worn, 60 * MB), 1 << 6 | 1 << 4);
        // Every idle helper together is 93 MB: a 100 MB shortfall frees none of them (they could not make the room).
        assert_eq!(choose(&worn, 100 * MB), 0);
        assert_eq!(choose(&[], 10 * MB), 0);
    }

    /// The pause retires the marked helpers that are still idle, only while the cycle still waits for the room; a helper
    /// used since its mark (Kirby put its hat on) stays, and nothing is retired once the group found room another way.
    /// Nothing else retires a helper: no mid-game retire.
    fn a_marked_helper_is_retired_at_the_pause_only_while_the_cycle_waits_and_it_is_idle() {
        let marked = 1 << 5 | 1 << 6;
        assert_eq!(at_pause(marked, 1 << 5 | 1 << 6 | 1 << 7, true), 1 << 5 | 1 << 6);
        assert_eq!(at_pause(marked, 1 << 6, true), 1 << 6, "slot 5 was put to use after its mark");
        assert_eq!(at_pause(marked, 1 << 5 | 1 << 6, false), 0, "the group found room: nothing goes");
        assert_eq!(at_pause(0, 0xf0, true), 0, "nothing marked, nothing retired");
    }

    /// The census names every consumer the final pass measures (diag build only).
    fn the_memory_census_names_every_consumer() {
        let mut helpers = [None; 8];
        helpers[2] = Some(Helper { slot: 2, kind: 0x4a, ready: true, leased: false, marked: false });
        helpers[3] = Some(Helper { slot: 3, kind: 0x45, ready: false, leased: false, marked: false });
        helpers[4] = Some(Helper { slot: 4, kind: 0x18, ready: true, leased: true, marked: false });
        helpers[6] = Some(Helper { slot: 6, kind: 0x10, ready: true, leased: false, marked: true });
        let census = Census { owner: 1, headroom: Some(143 * MB), committed: Some(3000 * MB), pool: Some((13, 24)),
            bank: [(3, 1), (1, 3)], helpers, pins: [0, 2], objects: 1 };
        assert_eq!(census.to_string(), format!("[MemoryCensus] owner=1 headroom=Some({}) committed=Some({}) pool=13/24 \
            bank=p1:3+1,cpu:1+3 seats=[2:0x4a ready, 3:0x45 loading] created=[4:0x18 ready leased, 6:0x10 ready marked] \
            pins=p1:0,cpu:2 objects=1", 143 * MB, 3000 * MB));
    }

    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("the_farthest_idle_helpers_go_first_until_the_shortfall_is_covered", the_farthest_idle_helpers_go_first_until_the_shortfall_is_covered as fn()),
            ("a_marked_helper_is_retired_at_the_pause_only_while_the_cycle_waits_and_it_is_idle", a_marked_helper_is_retired_at_the_pause_only_while_the_cycle_waits_and_it_is_idle as fn()),
            ("the_memory_census_names_every_consumer", the_memory_census_names_every_consumer as fn()),
        ]);
    }
    }
}
