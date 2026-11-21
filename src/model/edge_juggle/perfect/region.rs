//! Reachability bitmaps baked by the per-fighter recovery model.

pub(super) fn contains(rows: &[u32], outside: f32, height: f32) -> bool {
    if !(0.0 < outside && outside < 120.0 && height.is_finite() && height <= 40.0) {
        return false;
    }
    let column = (outside / 4.0) as u32;
    let row = ((40.0 - height) / 4.0) as usize;
    rows.get(row).is_some_and(|&mask| mask & (1 << column) != 0)
}

#[cfg(test)]
mod tests {
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
