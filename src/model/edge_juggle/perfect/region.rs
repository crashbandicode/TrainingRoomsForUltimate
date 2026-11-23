//! Reachability bitmaps baked by the per-fighter recovery model.

use super::{line_height, ordered_x, Point, Segment};

pub(super) fn contains(rows: &[u32], outside: f32, height: f32) -> bool {
    if !(0.0 < outside && outside < 120.0 && height.is_finite() && height <= 40.0) {
        return false;
    }
    let column = (outside / 4.0) as u32;
    let row = ((40.0 - height) / 4.0) as usize;
    rows.get(row).is_some_and(|&mask| mask & (1 << column) != 0)
}

/// Main-stage lower collision ring as measured on normal FD, Battlefield and Waiting Room.
pub(super) fn has_common_underside([near, far]: [Point; 2], bottom: f32, outline: &[Segment]) -> bool {
    ((far[0] - near[0]) - 159.975).abs() < 0.12
        && (far[1] - near[1]).abs() < 0.12
        && (bottom - near[1] + 140.0).abs() < 0.12
        && [
            (1.0, -9.396),
            (15.591, -25.111),
            (38.590, -40.099),
            (79.9875, -40.109),
            (121.402, -40.118),
            (144.373, -25.103),
            (158.975, -9.395),
        ]
        .into_iter()
        .all(|(offset, expected)| {
            let x = near[0] + offset;
            let lower = outline
                .iter()
                .filter_map(|&line| {
                    let [start, end] = ordered_x(line);
                    (start..=end).contains(&x).then(|| line_height(line, x)[0])
                })
                .reduce(f32::min);
            lower.is_some_and(|y| (y - near[1] - expected).abs() < 0.12)
        })
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    #[test]
    fn cells_use_near_relative_distances_and_descending_height() {
        let mut rows = [0; 44];
        rows[1] = 1 << 5;
        rows[43] = 1 << 29;
        assert!(contains(&rows, 20.0, 36.0));
        assert!(contains(&rows, 23.999, 32.001));
        assert!(contains(&rows, 119.9, -135.999));
        for (x, y) in [
            (19.999, 36.0),
            (24.0, 36.0),
            (20.0, 36.001),
            (20.0, 32.0),
            (119.9, -136.0),
        ] {
            assert!(!contains(&rows, x, y), "outside cell ({x}, {y})");
        }
    }

    #[test]
    fn a_compact_map_declines_heights_after_its_last_row() {
        let two_rows = [0, 1 << 5];
        assert!(contains(&two_rows, 20.0, 36.0));
        assert!(!contains(&two_rows, 20.0, 32.0));
    }

    #[test]
    fn malformed_or_invalid_coordinates_never_match_reserved_bits() {
        let rows = [u32::MAX; 44];
        for (x, y) in [
            (120.0, 0.0),
            (124.0, 0.0),
            (f32::MAX, 0.0),
            (-0.1, 0.0),
            (0.0, 0.0),
            (8.0, 40.001),
            (8.0, f32::NAN),
            (f32::INFINITY, 0.0),
        ] {
            assert!(!contains(&rows, x, y), "invalid cell ({x}, {y})");
        }
    }
    }
}
