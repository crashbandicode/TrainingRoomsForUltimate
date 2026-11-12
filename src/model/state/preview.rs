//! A clip editor's transient checkpoint. Numbered slots and persistence are not
//! part of this owner; native adapters supply freshly qualified identities.
use super::record::SavedState;

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

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("restarts_crop_edits_and_stop_keep_first_checkpoint_until_close", restarts_crop_edits_and_stop_keep_first_checkpoint_until_close as fn()),
        ]);
    }
    }
}
