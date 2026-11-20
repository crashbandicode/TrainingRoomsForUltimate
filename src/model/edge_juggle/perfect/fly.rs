//! Measured multi-jump crossing for Jigglypuff.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::edge_juggle::perfect::{RouteFacts, Segment};

    const LEFT: [f32; 2] = [-79.986, 0.0];
    const RIGHT: [f32; 2] = [79.989, 0.0];
    const RING: [Segment; 8] = [
        [LEFT, [-79.986, -8.319]],
        [[-79.986, -8.319], [-64.395, -25.111]],
        [[-64.395, -25.111], [-41.396, -40.099]],
        [[-41.396, -40.099], [41.416, -40.118]],
        [[41.416, -40.118], [64.387, -25.103]],
        [[64.387, -25.103], [79.989, -8.319]],
        [[79.989, -8.319], RIGHT],
        [RIGHT, LEFT],
    ];

    fn facts(position: [f32; 2]) -> RouteFacts<'static> {
        RouteFacts {
            stage: (3, 0),
            position,
            player: [-75.0, 0.0],
            player_reach: 9.0,
            ledges: [LEFT, RIGHT],
            blast: [-230.0, 230.0, 185.0, -140.0],
            outline: &RING,
            body_clearance: 9.5,
            ledge_distance: 10.0,
            fall_budget: 0.98,
            reach: [130.0, 20.0],
        }
    }

    fn live(position: [f32; 2], velocity: [f32; 2]) -> FlyState {
        FlyState { position, velocity, remaining: 5, can_jump: true, frame: 0 }
    }

    lib_unit_tests! {
    #[test]
    fn puff_requires_the_measured_region_and_all_five_jumps() {
        let start = live([-100.0, -28.0], [0.0; 2]);
        let safe = facts(start.position);
        assert!(PuffFlight::start(&safe, start).is_some());
        assert!(PuffFlight::start(&safe, FlyState { remaining: 4, ..start }).is_none());
        assert!(PuffFlight::start(&safe, FlyState { velocity: [1.0, 0.0], ..start }).is_none());
        assert!(PuffFlight::start(&RouteFacts { stage: (4, 0), ..safe }, start).is_none());
        let too_low = live([-130.0, -70.0], [0.0; 2]);
        assert!(PuffFlight::start(&facts(too_low.position), too_low).is_none());
    }

    #[test]
    fn puff_stays_below_the_live_hull_and_steers_at_far_hang_height() {
        let start = live([-90.0, 30.0], [0.0; 2]);
        let mut flight = PuffFlight::start(&facts(start.position), start).unwrap();
        let first = flight.step(start).unwrap();
        assert_eq!(first.stick, Some([0.0, 0.0]));
        assert_eq!(first.press, 0);

        let glide = FlyState { frame: 60, ..live([-90.0, -33.0], [0.0, -0.98]) };
        assert_eq!(flight.step(glide).unwrap().stick, Some([1.0, 0.0]));
        let under = FlyState { frame: 100, ..live([-30.0, -74.0], [1.332, -0.98]) };
        assert_eq!(flight.step(under).unwrap().press, 0);
        let room = FlyState { frame: 105, position: [-30.0, -78.5], ..under };
        let jump = flight.step(room).unwrap();
        assert_eq!(jump.press, crate::model::edge_juggle::perfect::JUMP);
        assert_eq!(jump.stick, Some([1.0, 0.0]));
        assert_eq!(flight.step(room).unwrap(), jump);
        let rising = FlyState { frame: 106, remaining: 4, velocity: [0.914, 1.597], ..room };
        assert_eq!(flight.step(rising).unwrap().press, 0);

        let beside = FlyState {
            frame: 180,
            remaining: 2,
            ..live([83.2, -22.0], [0.0, -0.98])
        };
        let climb = flight.step(beside).unwrap();
        assert_eq!(climb.stick, Some([1.0, 0.0]));
        assert_eq!(climb.press, crate::model::edge_juggle::perfect::JUMP);
    }
    }
}
