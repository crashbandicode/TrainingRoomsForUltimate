//! Grades verified escape times by the fraction of possible time saved.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    pub fastest: u32,
    pub no_input: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade {
    Poor,
    Ok,
    Good,
    Great,
    Optimal,
}

impl Grade {
    pub fn label(self) -> &'static str {
        match self {
            Self::Poor => "Poor",
            Self::Ok => "OK",
            Self::Good => "Good",
            Self::Great => "Great",
            Self::Optimal => "Optimal",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Graded(Grade),
    BelowOptimum,
    NeverEnds,
    NoSaving,
    InvalidBounds,
}

pub fn grade(elapsed: u32, bounds: Bounds) -> Outcome {
    if bounds.fastest == 0 || bounds.no_input.is_some_and(|end| end < bounds.fastest) {
        return Outcome::InvalidBounds;
    }
    if elapsed < bounds.fastest {
        return Outcome::BelowOptimum;
    }
    let Some(end) = bounds.no_input else {
        return Outcome::NeverEnds;
    };
    if end == bounds.fastest {
        return Outcome::NoSaving;
    }
    if elapsed == bounds.fastest {
        return Outcome::Graded(Grade::Optimal);
    }
    // Signed widening preserves negative savings after the baseline. Each
    // product is at most u32::MAX * 100 in magnitude, well inside i64.
    let saved = (i64::from(end) - i64::from(elapsed)) * 100;
    let possible = i64::from(end) - i64::from(bounds.fastest);
    Outcome::Graded(if saved >= possible * 90 {
        Grade::Great
    } else if saved >= possible * 75 {
        Grade::Good
    } else if saved >= possible * 50 {
        Grade::Ok
    } else {
        Grade::Poor
    })
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
        use super::{grade, Bounds, Grade, Outcome};

        const ORDINARY: Bounds = Bounds {
            fastest: 10,
            no_input: Some(90),
        };

        #[test]
        fn mashout_grade_threshold_edges() {
            for (elapsed, expected) in [
                (10, Grade::Optimal),
                (11, Grade::Great),
                (17, Grade::Great),
                (18, Grade::Great),
                (19, Grade::Good),
                (29, Grade::Good),
                (30, Grade::Good),
                (31, Grade::Ok),
                (49, Grade::Ok),
                (50, Grade::Ok),
                (51, Grade::Poor),
                (90, Grade::Poor),
                (91, Grade::Poor),
                (u32::MAX, Grade::Poor),
            ] {
                assert_eq!(
                    grade(elapsed, ORDINARY),
                    Outcome::Graded(expected),
                    "{elapsed}"
                );
            }
        }

        #[test]
        fn mashout_grade_fractional_edges() {
            let bounds = Bounds {
                fastest: 10,
                no_input: Some(91),
            };
            for (elapsed, expected) in [
                (18, Grade::Great),
                (19, Grade::Good),
                (30, Grade::Good),
                (31, Grade::Ok),
                (50, Grade::Ok),
                (51, Grade::Poor),
            ] {
                assert_eq!(grade(elapsed, bounds), Outcome::Graded(expected));
            }
        }

        #[test]
        fn mashout_grade_invalid_bounds() {
            for bounds in [
                Bounds {
                    fastest: 0,
                    no_input: None,
                },
                Bounds {
                    fastest: 0,
                    no_input: Some(0),
                },
                Bounds {
                    fastest: 0,
                    no_input: Some(90),
                },
                Bounds {
                    fastest: 10,
                    no_input: Some(9),
                },
                Bounds {
                    fastest: u32::MAX,
                    no_input: Some(0),
                },
            ] {
                for elapsed in [0, 10, u32::MAX] {
                    assert_eq!(grade(elapsed, bounds), Outcome::InvalidBounds);
                }
            }
        }

        #[test]
        fn mashout_grade_below_optimum() {
            for no_input in [None, Some(10), Some(90)] {
                for elapsed in [0, 9] {
                    assert_eq!(
                        grade(
                            elapsed,
                            Bounds {
                                fastest: 10,
                                no_input
                            }
                        ),
                        Outcome::BelowOptimum
                    );
                }
            }
        }

        #[test]
        fn mashout_grade_no_deadline() {
            for elapsed in [10, 11, 90, u32::MAX] {
                assert_eq!(
                    grade(
                        elapsed,
                        Bounds {
                            fastest: 10,
                            no_input: None
                        }
                    ),
                    Outcome::NeverEnds
                );
            }
        }

        #[test]
        fn mashout_grade_no_saving() {
            for elapsed in [10, 11, u32::MAX] {
                assert_eq!(
                    grade(
                        elapsed,
                        Bounds {
                            fastest: 10,
                            no_input: Some(10)
                        }
                    ),
                    Outcome::NoSaving
                );
            }
            assert_eq!(
                grade(
                    u32::MAX,
                    Bounds {
                        fastest: u32::MAX,
                        no_input: Some(u32::MAX)
                    }
                ),
                Outcome::NoSaving
            );
        }

        #[test]
        fn mashout_grade_verified_deadline() {
            // An a=0 profile with a verified external deadline has finite bounds.
            assert_eq!(grade(10, ORDINARY), Outcome::Graded(Grade::Optimal));
            assert_eq!(grade(18, ORDINARY), Outcome::Graded(Grade::Great));
        }

        #[test]
        fn mashout_grade_large_values() {
            let bounds = Bounds {
                fastest: 1,
                no_input: Some(4_000_000_001),
            };
            for (elapsed, expected) in [
                (400_000_001, Grade::Great),
                (400_000_002, Grade::Good),
                (1_000_000_001, Grade::Good),
                (1_000_000_002, Grade::Ok),
                (2_000_000_001, Grade::Ok),
                (2_000_000_002, Grade::Poor),
                (u32::MAX, Grade::Poor),
            ] {
                assert_eq!(grade(elapsed, bounds), Outcome::Graded(expected));
            }
            let bounds = Bounds {
                fastest: u32::MAX - 80,
                no_input: Some(u32::MAX),
            };
            assert_eq!(
                grade(u32::MAX - 80, bounds),
                Outcome::Graded(Grade::Optimal)
            );
            assert_eq!(grade(u32::MAX - 72, bounds), Outcome::Graded(Grade::Great));
            assert_eq!(grade(u32::MAX - 71, bounds), Outcome::Graded(Grade::Good));
        }

        #[test]
        fn mashout_grade_labels() {
            for (value, expected) in [
                (Grade::Poor, "Poor"),
                (Grade::Ok, "OK"),
                (Grade::Good, "Good"),
                (Grade::Great, "Great"),
                (Grade::Optimal, "Optimal"),
            ] {
                assert_eq!(value.label(), expected);
            }
        }
    }
}
