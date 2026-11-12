//! IC-PAIR-STALL (Yoga `swon0` step 82, M4d release): the CPU's Aegis and Pokémon Trainer groups
//! were built by Phase B on one `R`, then the CPU went to the Ice Climbers pair-lead window `[4b,4c,5c]`
//! on the same `R`. The pair rule's `R+0x50` write was refused (`other batches [3] hold keys on R`,
//! #1508), and the 3-body window never completed (B119 state-11 class).
//!
//! FACTs:
//! - The object kinds have native tails. The PT trainer object calls `0x17e8610(R, 0x72)` at
//!   `0x34cb1ec`; the Aegis object calls `0x17e8610(R, 0x75)` at `0x33f45e4`. Both run inside the
//!   Phase B build. Evidence: the Aegis post-build release took the map 9 → 8, removing only its
//!   orphan `0x1338` (#1466); after the PT build the map was 0 (#1483).
//! - The bank's post-build object release ran only while `R+0x18 != 0`. For PT the map was already
//!   empty, so the `0x72` token was never released or settled, and stayed in batch 3 (the plan's
//!   group 1 = PT, #1429).
//! - The pair-lead window's `prepare_resources` asks the ledger for a reset-batch permit, which
//!   refuses while any other batch holds a token.
//!
//! The fix keeps native the authority. A post-build object token on an empty map is settled with no
//! native call. A configure on an `R` that native picked as free (empty map) settles anything the
//! ledger still holds for it. This module checks the shape end to end against the native model.

lib_unit_tests! {
#[cfg(test)]
mod tests {
    use crate::engine::swap::swap_model::ledger::{
        AcqToken, BatchId, GroupId, Holder, Ledger, NativeCall, NoConsumers, ReleaseRecipe, RFlags, Role,
    };
    use crate::engine::swap::swap_model::native_model::{kind_table, KindTable, RModel, G68};

    const AEGIS: [u32; 2] = [0x5a, 0x5b];
    const AEGIS_OBJECT: u32 = 0x75;
    const PT: [u32; 3] = [0x24, 0x25, 0x26];
    const PT_OBJECT: u32 = 0x72;

    struct World {
        ledger: Ledger,
        r: RModel,
        g68: G68,
        table: KindTable,
        /// Tokens the ledger minted and the bank still holds (`r_handle`'s `held`).
        held: Vec<AcqToken>,
    }

    impl World {
        fn new() -> Self {
            World { ledger: Ledger::new(RFlags::TRAINING), r: RModel::default(), g68: G68::default(), table: kind_table(), held: Vec::new() }
        }

        /// A Phase B group's prefetch: a construction token per member, the object kind owned.
        fn prefetch(&mut self, group: u8, members: &[u32], object: u32) -> BatchId {
            let batch = self.ledger.new_batch();
            for (kind, recipe) in members.iter().map(|k| (*k, ReleaseRecipe::ConstructionTail))
                .chain(std::iter::once((object, ReleaseRecipe::OwnedPerKind))) {
                let (token, _) = self.ledger.acquire(kind, Holder::Prefetch(GroupId(group)), recipe, batch, &NoConsumers, Role::Cpu).unwrap();
                self.held.push(token);
                self.r.acquire(kind, &self.table);
            }
            batch
        }

        fn take(&mut self, kind: u32, batch: BatchId) -> Option<AcqToken> {
            let i = self.held.iter().position(|t| t.kind() == kind && t.batch() == batch)?;
            Some(self.held.remove(i))
        }

        fn apply(&mut self, calls: Vec<NativeCall>) {
            for call in calls {
                match call {
                    NativeCall::ReleaseKey(key) => self.r.release_key(key.value()),
                    NativeCall::ReleaseKind { kind } => self.r.release_kind(kind, None, &mut self.g68),
                    other => panic!("unexpected {:?}", other),
                }
            }
        }

        /// The synchronous build. Each body's tail (`0x60dec4`) and the object's own native tail
        /// release their kinds (the last one resets an emptied R). The ledger settles the
        /// constructions. Then the bank's post-build object step runs: `fixed` is the rule; today it
        /// is skipped on an empty map.
        fn build(&mut self, batch: BatchId, members: &[u32], object: u32, fixed: bool) {
            for kind in members {
                self.r.release_kind(*kind, None, &mut self.g68);
                let _construction = self.take(*kind, batch);
                assert!(self.ledger.construction_settled(*kind, batch));
            }
            self.r.release_kind(object, None, &mut self.g68);
            let token = self.take(object, batch).expect("object token held");
            if self.r.map_nodes() != 0 {
                let calls = self.ledger.release(token, &NoConsumers, Role::Cpu).unwrap();
                self.apply(calls);
            } else if fixed {
                assert!(self.ledger.settle(token));
            } else {
                self.held.push(token);
            }
        }

        /// `r_handle::settle_stale_on_empty`: a configure on an R native found empty settles every
        /// held token. Returns how many existed.
        fn configure_on_empty(&mut self) -> usize {
            assert_eq!(self.r.map_nodes(), 0);
            let mut settled = 0;
            for token in std::mem::take(&mut self.held) {
                if self.ledger.settle(token) { settled += 1; }
            }
            settled
        }

        fn balanced(&self) -> bool { self.ledger.reconcile(&self.r.snapshot()).balanced() }
    }

    /// The swon0 sequence. Prefetch Aegis (group 0) and PT (group 1). Build Aegis: the map stays
    /// non-empty, so the post-build release returns the orphan `0x1338`. Build PT: the map is empty
    /// afterwards. Then the IC pair window asks for its reset-batch permit.
    fn run(fixed: bool, configure_rule: bool) -> (World, Result<(), Vec<BatchId>>, BatchId, usize) {
        let mut world = World::new();
        let aegis = world.prefetch(0, &AEGIS, AEGIS_OBJECT);
        let pt = world.prefetch(1, &PT, PT_OBJECT);
        world.build(aegis, &AEGIS, AEGIS_OBJECT, fixed);
        assert!(world.balanced(), "after the Aegis build: {:?}", world.ledger.reconcile(&world.r.snapshot()));
        world.build(pt, &PT, PT_OBJECT, fixed);
        assert_eq!(world.r.map_nodes(), 0, "the PT build empties R (#1483 map=0)");
        let stale = if configure_rule { world.configure_on_empty() } else { 0 };
        let window = world.ledger.new_batch();
        let permit = world.ledger.reset_batch_permit(window).map(|_| ());
        (world, permit, pt, stale)
    }

    fn settling_the_object_on_an_empty_map_keeps_ledger_and_native_equal_and_admits_the_pair() {
        let (world, permit, _, _) = run(true, false);
        assert!(world.balanced(), "{:?}", world.ledger.reconcile(&world.r.snapshot()));
        assert!(world.ledger.tokens().is_empty());
        assert_eq!(permit, Ok(()));
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("settling_the_object_on_an_empty_map_keeps_ledger_and_native_equal_and_admits_the_pair", settling_the_object_on_an_empty_map_keeps_ledger_and_native_equal_and_admits_the_pair as fn()),
        ]);
    }
}
}
