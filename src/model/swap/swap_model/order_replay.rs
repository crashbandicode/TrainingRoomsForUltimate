//! LEDGER-SCOPE: the two reconcile observations of the IC-pair follow-up rows, replayed against
//! the native model with the logged map counts.
//!
//! - **R-ICP3 (`icp3d`, CPU order PT → Aegis → IC):** `#200 reconcile … missing=[(ec5,1),(eef,1)]`
//!   after the PT build. Those are the `+1` keys of `0x5a/0x5b`: the `0x65dd70` table sets the bit,
//!   but the files are absent, and `0x17e2df0` inserts no node for handle `0xffffff`. The fixture
//!   `d-sg1-aegis-orphan.jsonl` line 2 already calibrates them (`missing_keys [3781, 3823]`). The
//!   ledger predicts every file present (`ledger::KeyRule`: "and its file exists"). This is
//!   reporting only. The tokens are exact: each is released by kind through `0x17e8610`, which
//!   ignores a missing key. `orphan_keys(0x5a|0x5b)` is empty, so no per-key call ever names those
//!   keys. In the Aegis-first order the Aegis tokens are already settled at every reconcile, so
//!   nothing shows.
//! - **R-ICP4 (`icp4d`, Aegis + IC pair group):** `#183 reconcile … residual=[(c78,1),(c7e,1)]`.
//!   Those are Nana's `+0`/`+6`. The ledger's registration rule for `0x4c` was empty ("never
//!   requested by the bank"), while the bank's pair-group prefetch requests it and native requests it
//!   for every pair window (`17e5bc0.c:230–235`). That was a real accounting gap in the key
//!   prediction, now fixed (`ledger::registration_rules`, `native_model::keys_registered_by_17e64d0`).
//!
//! Neither leaves a token that a later `reset_batch_permit` or `R+0x50` write could refuse on.
//! Permits and pair references count tokens, not predicted keys, and every token here settles or is
//! released by kind. The tests below replay both runs with the logged map counts.

#[cfg(test)]
mod tests {
    use crate::engine::swap::swap_model::ledger::{
        AcqToken, BatchId, GroupId, Holder, Ledger, NativeCall, NoConsumers, ReleaseRecipe, RFlags, Role,
    };
    use crate::engine::swap::swap_model::native_model::{kind_table, KindTable, RModel, G68};

    const PT: [u32; 3] = [0x24, 0x25, 0x26];
    const AEGIS: [u32; 2] = [0x5a, 0x5b];
    const PAIR: [u32; 2] = [0x4b, 0x4c];
    /// `d-sg1-aegis-orphan.jsonl` line 2: the absent `+1` files of `0x5a`/`0x5b`.
    const AEGIS_ABSENT: [u32; 2] = [0x5a * 42 + 1, 0x5b * 42 + 1];

    /// `(key, count)` pairs, as `Reconciliation::residual` / `Reconciliation::missing` carry them.
    type KeyCounts = Vec<(u32, u32)>;

    struct World {
        ledger: Ledger,
        r: RModel,
        g68: G68,
        table: KindTable,
        held: Vec<AcqToken>,
    }

    impl World {
        fn new() -> Self {
            let mut r = RModel::default();
            r.missing_keys.extend(AEGIS_ABSENT);
            World { ledger: Ledger::new(RFlags::TRAINING), r, g68: G68::default(), table: kind_table(), held: Vec::new() }
        }

        fn prefetch(&mut self, group: u8, kinds: &[(u32, ReleaseRecipe)]) -> BatchId {
            let batch = self.ledger.new_batch();
            for (kind, recipe) in kinds {
                let (token, _) = self.ledger.acquire(*kind, Holder::Prefetch(GroupId(group)), *recipe, batch, &NoConsumers, Role::Cpu).unwrap();
                self.held.push(token);
                self.r.acquire(*kind, &self.table);
            }
            batch
        }

        fn take(&mut self, kind: u32, batch: BatchId) -> AcqToken {
            let i = self.held.iter().position(|t| t.kind() == kind && t.batch() == batch).expect("held");
            self.held.remove(i)
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

        /// A native special group's Phase B. The bodies' tails and the object's own tail release
        /// their kinds. Then the bank's post-build object step runs (release on a non-empty map,
        /// settle on an empty one).
        fn build_special(&mut self, batch: BatchId, members: &[u32], object: u32) {
            for kind in members {
                self.r.release_kind(*kind, None, &mut self.g68);
                let _construction = self.take(*kind, batch);
                assert!(self.ledger.construction_settled(*kind, batch));
            }
            self.r.release_kind(object, None, &mut self.g68);
            let token = self.take(object, batch);
            if self.r.map_nodes() != 0 {
                let calls = self.ledger.release(token, &NoConsumers, Role::Cpu).unwrap();
                self.apply(calls);
            } else {
                assert!(self.ledger.settle(token));
            }
        }

        /// The teardown's key step: every token of `batch` released by kind.
        fn drain(&mut self, batch: BatchId) {
            while let Some(i) = self.held.iter().position(|t| t.batch() == batch) {
                let token = self.held.remove(i);
                let calls = self.ledger.release(token, &NoConsumers, Role::Cpu).unwrap();
                self.apply(calls);
            }
        }

        /// `(residual, missing)`.
        fn reconcile(&self) -> (KeyCounts, KeyCounts) {
            let r = self.ledger.reconcile(&self.r.snapshot());
            (r.residual, r.missing)
        }

        /// The next pair window's configure on the emptied R, then its reset-batch permit.
        fn pair_window_permit(&mut self) -> (usize, bool) {
            assert_eq!(self.r.map_nodes(), 0);
            let mut stale = 0;
            for token in std::mem::take(&mut self.held) {
                if self.ledger.settle(token) { stale += 1; }
            }
            let window = self.ledger.new_batch();
            (stale, self.ledger.reset_batch_permit(window).is_ok())
        }
    }

    fn special(kinds: &[u32], object: u32) -> Vec<(u32, ReleaseRecipe)> {
        kinds.iter().map(|k| (*k, ReleaseRecipe::ConstructionTail)).chain(std::iter::once((object, ReleaseRecipe::OwnedPerKind))).collect()
    }

    /// `icp3d`: PT group 0, then Aegis group 1 (#154–#170: map 8, then 15), PT built first.
    #[test]
    fn icp3d_pt_first_misses_only_the_absent_aegis_files_and_strands_nothing() {
        let mut world = World::new();
        let pt = world.prefetch(0, &special(&PT, 0x72));
        assert_eq!(world.r.map_nodes(), 8);
        let aegis = world.prefetch(1, &special(&AEGIS, 0x75));
        assert_eq!(world.r.map_nodes(), 15, "#169/#170 map=15: nine predicted, two files absent");
        world.build_special(pt, &PT, 0x72);
        assert_eq!(world.r.map_nodes(), 7, "#196/#199 map=7");
        assert_eq!(world.reconcile(), (vec![], vec![(0x5a * 42 + 1, 1), (0x5b * 42 + 1, 1)]), "#200: missing=[(ec5,1),(eef,1)]");
        world.build_special(aegis, &AEGIS, 0x75);
        assert_eq!(world.r.map_nodes(), 0, "#228 map=0");
        assert_eq!(world.reconcile(), (vec![], vec![]), "#229 balanced");
        assert_eq!(world.pair_window_permit(), (0, true), "no stale token; the IC refs are written (#266)");
    }

    /// `icp4d`: Aegis group 0 (map 7), then the IC pair group 1 `[4b, 4c]` (map 11), Aegis built,
    /// then the mod reload drains the pair group.
    #[test]
    fn icp4d_nana_s_keys_are_predicted_and_the_pair_group_drains_clean() {
        let mut world = World::new();
        let aegis = world.prefetch(0, &special(&AEGIS, 0x75));
        assert_eq!(world.r.map_nodes(), 7, "#146 map=7");
        let pair = world.prefetch(1, &PAIR.iter().map(|k| (*k, ReleaseRecipe::PairReset { references: 2 })).collect::<Vec<_>>());
        assert_eq!(world.r.map_nodes(), 11, "#152 map=11: Popo {{0,6}} and Nana {{0,6}}");
        world.build_special(aegis, &AEGIS, 0x75);
        assert_eq!(world.r.map_nodes(), 4, "#182 map=4");
        assert_eq!(world.reconcile(), (vec![], vec![]), "#183 was residual=[(c78,1),(c7e,1)]; Nana is now predicted");
        let predicted: Vec<u32> = world.ledger.predicted().into_iter().map(|(k, _)| k).collect();
        assert_eq!(predicted, vec![0xc4e, 0xc54, 0xc78, 0xc7e]);
        world.drain(pair);
        assert_eq!(world.r.map_nodes(), 0, "#206 map_after=0");
        assert_eq!(world.reconcile(), (vec![], vec![]));
        assert_eq!(world.pair_window_permit(), (0, true), "#254 refs 2->3");
    }
}
