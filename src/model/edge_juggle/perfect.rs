//! Perfect-mode recovery and escape decisions. The runtime applies only pad inputs.

pub type Point = [f32; 2];
pub type Segment = [Point; 2];

/// The foot heights that leave enough room below a closed main-stage outline.
/// The caller passes live segments from one walk in the current stage.
pub fn corridor(
    outline: &[Segment],
    ledges: [Point; 2],
    blast_bottom: f32,
    body_clearance: f32,
    fall_budget: f32,
) -> Option<[f32; 2]> {
    let [[left, left_y], [right, right_y]] = ledges;
    if ![left, left_y, right, right_y, blast_bottom, body_clearance, fall_budget]
        .into_iter().all(f32::is_finite)
        || left >= right || body_clearance <= 0.0 || fall_budget < 0.0 || outline.is_empty()
    {
        return None;
    }
    let mut underside = left_y.min(right_y);
    for &[a, b] in outline {
        if !a.into_iter().chain(b).all(f32::is_finite) {
            return None;
        }
        let ([x0, y0], [x1, y1]) = if a[0] <= b[0] { (a, b) } else { (b, a) };
        let start = x0.max(left);
        let end = x1.min(right);
        if start <= end {
            let low = if x0 == x1 {
                y0.min(y1)
            } else {
                let at = |x| y0 + (y1 - y0) * ((x - x0) / (x1 - x0));
                at(start).min(at(end))
            };
            underside = underside.min(low);
        }
    }
    let low = blast_bottom + fall_budget;
    let high = underside - body_clearance;
    (low.is_finite() && high.is_finite() && low < high).then_some([low, high])
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    #[test]
    fn opposite_ledge_requires_a_full_corridor_above_the_blast_zone() {
        let ledges = [[-70., 0.], [70., 0.]];
        let fd = [
            [ledges[0], ledges[1]],
            [ledges[0], [0., -32.]],
            [[0., -32.], ledges[1]],
        ];
        assert_eq!(corridor(&fd, ledges, -105., 12., 15.), Some([-90., -44.]));
        let wedge = [fd[0], [ledges[0], [0., -110.]], [[0., -110.], ledges[1]]];
        assert_eq!(corridor(&wedge, ledges, -105., 12., 15.), None);
        assert_eq!(corridor(&fd, ledges, -60., 18., 15.), None);
        assert_eq!(corridor(&[], ledges, -105., 12., 15.), None);
    }
    }
}
