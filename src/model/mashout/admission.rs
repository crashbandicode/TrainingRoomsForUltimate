//! A required guard closure decides whether Mash may read a seed or grade a result.

// restriction: M3-READER-GUARD
pub const UNAVAILABLE_NOTICE: &str = "Mash out unavailable";

/// A newer dropped observation also invalidates older attempts still queued for grading.
pub const fn faulted(generation: u64, through: u64) -> bool {
    generation != 0 && generation <= through
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Admission {
    available: bool,
}

impl Admission {
    /// `required_usable` covers every required site, including the shared native dependencies.
    pub const fn new(required_usable: bool) -> Self {
        Self {
            available: required_usable,
        }
    }

    pub const fn available(self) -> bool {
        self.available
    }

    /// Keep native reads and grading inside this closure so an unavailable path does no work.
    pub fn with<T>(self, work: impl FnOnce() -> T) -> Option<T> {
        self.available.then(work)
    }
}

/// Preserve the existing CPU-before-original path; the admitted P1 observer runs after it instead.
pub fn start_dispatch<T>(
    p1: bool,
    original: impl FnOnce() -> T,
    cpu: impl FnOnce(),
    observe: impl FnOnce(),
) -> T {
    if p1 {
        let result = original();
        observe();
        result
    } else {
        cpu();
        original()
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
        use super::{faulted, start_dispatch, Admission, UNAVAILABLE_NOTICE};
        use crate::model::{
            guard::GuardOutcome,
            mashout::grade::{grade, Bounds, Grade, Outcome},
            sites_mashout::SITES,
        };
        use std::cell::Cell;

        #[test]
        fn mashout_fault_watermark_survives_retirement_and_retry() {
            use core::sync::atomic::{AtomicU64, Ordering};
            let through = AtomicU64::new(0);
            for generation in [4, 4, 0, 0, 5] {
                through.fetch_max(generation, Ordering::AcqRel);
                assert!(faulted(4, through.load(Ordering::Acquire)));
            }
            assert!(faulted(5, through.load(Ordering::Acquire)));
            assert!(!faulted(0, through.load(Ordering::Acquire)));
            assert!(!faulted(6, through.load(Ordering::Acquire)));
        }

        #[test]
        fn mashout_p1_observer_calls_original_once_after_no_cpu_automation() {
            let order = Cell::new(0);
            let result = start_dispatch(true,
                || { assert_eq!(order.replace(1), 0); 0x1234u64 },
                || panic!("P1 reached CPU automation"),
                || { assert_eq!(order.replace(2), 1); });
            assert_eq!((result, order.get()), (0x1234, 2));
        }

        #[test]
        fn mashout_cpu_start_retains_pre_original_order_and_return() {
            let order = Cell::new(0);
            let result = start_dispatch(false,
                || { assert_eq!(order.replace(2), 1); 0xfedcu64 },
                || { assert_eq!(order.replace(1), 0); },
                || panic!("CPU reached P1 observer"));
            assert_eq!((result, order.get()), (0xfedc, 2));
        }

        #[test]
        fn mashout_reader_failed_windows_block_seed_and_score() {
            for failed in &SITES {
                for byte in 0..64 {
                    let mut poisoned = failed.guard;
                    // A foreign first instruction makes each mutated 64-byte fixture unusable,
                    // including Called rows whose ordinary foreign hooks remain usable by policy.
                    poisoned[0] ^= 0xff;
                    poisoned[byte] ^= 0x40;
                    assert!(!failed.outcome(&poisoned).usable(), "{} byte {byte}", failed.name);
                    let admission = Admission::new(SITES.iter().all(|site| {
                        site.outcome(if site.name == failed.name { &poisoned } else { &site.guard }).usable()
                    }));
                    let reads = Cell::new(0);
                    let scores = Cell::new(0);
                    let seed = admission.with(|| {
                        reads.set(reads.get() + 1);
                        Bounds { fastest: 10, no_input: Some(90) }
                    });
                    let score = admission.with(|| {
                        scores.set(scores.get() + 1);
                        grade(30, Bounds { fastest: 10, no_input: Some(90) })
                    });
                    assert!(!admission.available());
                    assert_eq!((reads.get(), scores.get(), seed, score), (0, 0, None, None));
                }
            }
        }

        #[test]
        fn mashout_reader_stock_allows_seed_and_score() {
            let admission = Admission::new(SITES.iter().all(|site| site.outcome(&site.guard).usable()));
            let reads = Cell::new(0);
            let scores = Cell::new(0);
            let seed = admission.with(|| {
                reads.set(reads.get() + 1);
                Bounds { fastest: 10, no_input: Some(90) }
            }).unwrap();
            let score = admission.with(|| {
                scores.set(scores.get() + 1);
                grade(30, seed)
            });
            assert!(admission.available());
            assert_eq!((reads.get(), scores.get(), score), (1, 1, Some(Outcome::Graded(Grade::Good))));
        }

        #[test]
        fn mashout_reader_required_closure_blocks_work() {
            // Reader rows alone do not grant the shared clatter/CPU/native closure.
            let reader_usable = SITES.iter().all(|site| site.outcome(&site.guard).usable());
            for required in [GuardOutcome::Foreign { first_diff: 0 }, GuardOutcome::HookedByOther { at: 0, usable: false }] {
                let admission = Admission::new(reader_usable && required.usable());
                assert!(!admission.available());
                assert_eq!(admission.with(|| panic!("shared guard failure ran reader/grader")), None::<()>);
            }
            assert!(UNAVAILABLE_NOTICE.contains("unavailable"));
        }
    }
}
