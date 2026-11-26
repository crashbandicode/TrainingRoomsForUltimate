//! A clip editor's transient checkpoint. Numbered slots and persistence are not
//! part of this owner; native adapters supply freshly qualified identities.
use super::record::SavedState;

pub fn playing_index(player: bool) -> usize { usize::from(!player) }

/// What a preview's start does about a CPU the scrub hold has just let go (`scrub_track::Block::settling`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Settle {
    /// On the ground and idle: the checkpoint's own condition.
    Ready,
    /// In a state the game always leaves for the ground: the start stays asked.
    Wait,
    /// It will not settle: the start is refused, as a start with no release before it is.
    Refuse,
}

/// The CPU on this frame, as the game reports it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Body {
    pub ground: bool,
    pub air: bool,
    pub idle: bool,
    /// The Fall status: what the hold put the body in, and what it is in until it lands.
    pub falling: bool,
    /// A landing status.
    pub landing: bool,
    /// Dead, waiting to come back, or on the rebirth platform.
    pub gone: bool,
    /// In the air with a floor under its point by the shared stage rule: it lands.
    pub floor: bool,
}

/// No clock: every state waited on is one the game leaves by itself, and every other state refuses at once.
pub fn settle(body: Body) -> Settle {
    let _ = body;
    Settle::Refuse
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Missing,
    FightersChanged,
    OwnedJson,
}

pub struct Checkpoint<I> {
    saved: Option<(I, [SavedState; 2])>,
}

impl<I: Copy + PartialEq> Default for Checkpoint<I> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I: Copy + PartialEq> Checkpoint<I> {
    pub const fn new() -> Self {
        Self { saved: None }
    }

    pub fn captured(&self) -> bool {
        self.saved.is_some()
    }

    /// The adapter captures once after both fighters settle. Retrying or editing
    /// a crop must not replace that anchor with positions from the last preview.
    /// JSON is a worker-owned representation and cannot enter this game owner.
    pub fn capture(&mut self, identity: I, pair: [SavedState; 2]) -> Result<(), Error> {
        if let Some((saved, _)) = &self.saved {
            return if *saved == identity {
                Ok(())
            } else {
                Err(Error::FightersChanged)
            };
        }
        if pair.iter().any(|record| record.mii.is_some()) {
            return Err(Error::OwnedJson);
        }
        self.saved = Some((identity, pair));
        Ok(())
    }

    /// Only a successfully captured JSON-free pair is copied here. Both start
    /// and stop use this same checkpoint; current positions are not an input.
    pub fn restore(&self, identity: &I) -> Result<[SavedState; 2], Error> {
        let (saved, pair) = self.saved.as_ref().ok_or(Error::Missing)?;
        if saved != identity {
            return Err(Error::FightersChanged);
        }
        Ok(pair.clone())
    }

    /// Editor close or actual entry retirement, never an ordinary crop change.
    pub fn clear(&mut self) {
        self.saved = None;
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    fn pair(x: f32) -> [SavedState; 2] {
        std::array::from_fn(|role| SavedState {
            fighter_kind: 7 + role as i32,
            x: if role == 0 { x } else { -x },
            percent: 17.0 + role as f32,
            saved_stage: Some(8),
            ..SavedState::never_saved()
        })
    }

    fn a_released_cpu_is_waited_for_by_its_state_and_never_by_a_clock() {
        let on_ground = Body { ground: true, ..Body::default() };
        let in_air = Body { air: true, ..Body::default() };
        assert_eq!(settle(Body { idle: true, ..on_ground }), Settle::Ready);
        // The game always leaves these: the released body's Fall on the ground, a landing, a fall onto a floor.
        assert_eq!(settle(Body { falling: true, ..on_ground }), Settle::Wait);
        assert_eq!(settle(Body { landing: true, ..on_ground }), Settle::Wait);
        assert_eq!(settle(Body { floor: true, falling: true, ..in_air }), Settle::Wait);
        assert_eq!(settle(Body { floor: true, ..in_air }), Settle::Wait);
        // It will not settle: no floor to land on, the ledge (neither ground nor air), busy on the ground.
        assert_eq!(settle(Body { falling: true, ..in_air }), Settle::Refuse);
        assert_eq!(settle(Body::default()), Settle::Refuse);
        assert_eq!(settle(on_ground), Settle::Refuse);
        assert_eq!(settle(Body { idle: true, ..in_air }), Settle::Refuse, "idle in the air is not the gate's condition");
        // Dead or coming back refuses whatever else is read.
        for body in [Body { idle: true, ..on_ground }, Body { floor: true, ..in_air }, Body { landing: true, ..on_ground }] {
            assert_eq!(settle(Body { gone: true, ..body }), Settle::Refuse);
        }
        // The game side keeps the start asked while it waits, and takes it only then.
        let runtime = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/runtime/features/states/preview.rs")).unwrap_or_default();
        let wait = runtime.find("if waiting != 0 && waiting != 3 && settling() == Some(Settle::Wait) {").expect("a start waits for the released CPU");
        assert!(wait < runtime.find("let ask = ASK.swap(0, Ordering::AcqRel);").unwrap(), "the ask stays while it waits");
        assert!(!runtime.contains("Instant") && !runtime.contains("elapsed"), "no clock");
    }

    fn restarts_crop_edits_and_stop_keep_first_checkpoint_until_close() {
        let mut checkpoint = Checkpoint::new();
        let identity = ([100usize, 200], [7, 8], 8);
        assert_eq!(checkpoint.restore(&identity), Err(Error::Missing));
        checkpoint.capture(identity, pair(42.0)).unwrap();
        for x in [5.0, 90.0, -17.0] {
            checkpoint.capture(identity, pair(x)).unwrap();
            assert_eq!(checkpoint.restore(&identity).unwrap(), pair(42.0));
        }
        checkpoint.clear();
        assert!(!checkpoint.captured());
        checkpoint.capture(identity, pair(-9.0)).unwrap();
        assert_eq!(checkpoint.restore(&identity).unwrap(), pair(-9.0));
    }

    fn a_full_pair_comes_back_at_the_start_the_finish_and_a_stop_and_a_retired_home_gives_nothing() {
        use crate::model::state::held_item::HeldItem;
        use crate::model::state::record::{Charge, Steve};
        let mut checkpoint = Checkpoint::new();
        let identity = ([100usize, 200], [7, 8], 8);
        let mut full = pair(42.0);
        full[0].steve = Some(Steve {
            mat_iron: 12, mat_diamond: 4, pick_mat: 4, pick_durability: 31.5,
            ..Steve::START
        });
        full[0].held_item = Some(HeldItem::Item {
            choice: 29, variation: 7, lifetime: 100, lifetime_max: 600, fuse: -1, fuse_max: -1,
        });
        full[1].charge = Charge { int_x: Some(17), float_x: Some(3.5), has_charge: Some(true), ..Charge::default() };
        checkpoint.capture(identity, full.clone()).unwrap();
        for _ in 0..3 {
            assert_eq!(checkpoint.restore(&identity).unwrap(), full);
        }
        let different = ([100usize, 201], [7, 8], 8);
        assert_eq!(checkpoint.restore(&different), Err(Error::FightersChanged));
        checkpoint.clear();
        assert_eq!(checkpoint.restore(&identity), Err(Error::Missing));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("restarts_crop_edits_and_stop_keep_first_checkpoint_until_close", restarts_crop_edits_and_stop_keep_first_checkpoint_until_close as fn()),
            ("a_released_cpu_is_waited_for_by_its_state_and_never_by_a_clock", a_released_cpu_is_waited_for_by_its_state_and_never_by_a_clock as fn()),
            ("a_full_pair_comes_back_at_the_start_the_finish_and_a_stop_and_a_retired_home_gives_nothing", a_full_pair_comes_back_at_the_start_the_finish_and_a_stop_and_a_retired_home_gives_nothing as fn()),
        ]);
    }
    }
}
