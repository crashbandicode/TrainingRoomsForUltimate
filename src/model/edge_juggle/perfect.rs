//! Perfect-mode recovery and escape decisions. The runtime applies only pad inputs.

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
