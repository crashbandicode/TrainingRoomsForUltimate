//! rc.14 Break the Targets' camera (SPEC V2.5, `camera`; acceptance row 31 has the Eden row): the fixed camera is switched on
//! by state, restored when the mode ends, and the manager is read only through a pointer that reads as an object.

lib_unit_tests! {
use super::*;

/// The camera manager's mode as the game reports it: 1 normal, 4 Training's Fixed, 8 Zoom.
const NORMAL: i32 = 1;
const ZOOM: i32 = 8;

/// Where the mode takes the game's camera and reads the stage's camera range (the normal placing box): native Training and an
/// active waiting-room practice (rc.14, 11-15: the LDN pass showed both local to the console); not in a match, not while a
/// fighter swap owns the scene, not on a lobby flag left over from an earlier room.
fn btt_room_uses_the_fixed_camera_and_the_normal_box() {
    use crate::model::scene::Facts;
    let training = Facts { native_training: true, ..Facts::default() };
    let room = Facts { lobby_active: true, waiting_room: true, ..Facts::default() };
    assert_eq!([own_camera(training), own_camera(room)], [true; 2], "Training and the waiting room's practice");
    let stale = Facts { lobby_active: true, ..Facts::default() };
    let busy = Facts { busy: true, ..room };
    assert_eq!([own_camera(Facts::default()), own_camera(stale), own_camera(busy)], [false; 3], "outside practice");
}

fn btt_camera_switches_by_state() {
    let mut cam = Camera::IDLE;
    // Not wanted, and the game on its own camera: nothing to do, nothing held.
    assert_eq!((cam.step(false, Some(NORMAL)), cam.holding()), (None, false));
    // Wanted: switch to the Fixed camera, remembering the mode before; the hooks are told to stand down from this step on.
    assert_eq!((cam.step(true, Some(NORMAL)), cam.holding()), (Some(FIXED), true));
    // The game reports the Fixed camera now: nothing more to do.
    for _ in 0..30 {
        assert_eq!(cam.step(true, Some(FIXED)), None);
    }
    // Not wanted any more: back to the mode it was, at once, and the hold ends before the call.
    assert_eq!((cam.step(false, Some(FIXED)), cam.holding()), (Some(NORMAL), false));
    assert_eq!(cam.step(false, Some(NORMAL)), None);
    // From the Zoom camera back to the Zoom camera.
    let mut cam = Camera::IDLE;
    assert_eq!(cam.step(true, Some(ZOOM)), Some(FIXED));
    assert_eq!(cam.step(false, Some(FIXED)), Some(ZOOM));
}

fn btt_camera_reasserts_at_a_rate_and_keeps_the_first_mode() {
    let mut cam = Camera::IDLE;
    assert_eq!(cam.step(true, Some(NORMAL)), Some(FIXED));
    // The game's own Training Camera option moved it to Zoom: asked again, but not every frame.
    let calls = (0..RETRY * 3).filter(|_| cam.step(true, Some(ZOOM)) == Some(FIXED)).count();
    assert_eq!(calls, 3, "one call every {RETRY} frames");
    // The mode remembered is the first one seen, not the Zoom the game moved it to.
    assert_eq!(cam.step(false, Some(FIXED)), Some(NORMAL));
    // Someone else changed it to Zoom while we hold nothing: leave it alone.
    assert_eq!(cam.step(false, Some(ZOOM)), None);
}

fn btt_camera_stands_down_without_a_manager() {
    let mut cam = Camera::IDLE;
    cam.step(true, Some(NORMAL));
    // The scene ended (no manager): nothing to restore, and the hooks are free again.
    assert_eq!((cam.step(true, None), cam.holding()), (None, false));
    assert_eq!(cam.step(false, Some(FIXED)), None, "nothing was held across the scene");
    // Already Fixed: remember that exact mode too, even though no native switch is needed.
    let mut cam = Camera::IDLE;
    assert_eq!((cam.step(true, Some(FIXED)), cam.holding()), (None, true));
    assert_eq!(cam.step(false, Some(FIXED)), None);
}

fn btt_camera_restores_each_placement_exit() {
    use crate::model::scene::Facts;
    let practice = Facts { native_training: true, ..Facts::default() };
    for before in [NORMAL, ZOOM, FIXED] {
        for phase in [Phase::Off, Phase::Ready, Phase::Running, Phase::Done] {
            let mut cam = Camera::IDLE;
            assert_eq!(cam.step(wants_camera(Phase::Placing, false, practice), Some(before)), (before != FIXED).then_some(FIXED));
            assert_eq!(cam.step(wants_camera(phase, false, practice), Some(FIXED)), (before != FIXED).then_some(before));
            assert!(!cam.holding(), "{phase:?}");
        }
        for (covered, facts) in [(true, practice), (false, Facts { busy: true, ..practice }), (false, Facts::default())] {
            let mut cam = Camera::IDLE;
            cam.step(true, Some(before));
            assert_eq!(cam.step(wants_camera(Phase::Placing, covered, facts), Some(FIXED)), (before != FIXED).then_some(before));
            assert!(!cam.holding());
        }
    }
    let mut cam = Camera::IDLE;
    cam.step(true, Some(NORMAL));
    assert_eq!(cam.step(false, Some(ZOOM)), Some(NORMAL), "an intervening mode cannot replace the saved mode");
}

fn btt_camera_manager_pointer_is_checked_before_it_is_read() {
    // A manager pointer is aligned and not in the first pages.
    assert!(pointer_ok(0x0000_0042_1234_5678 & !7));
    for bad in [0, 1, 0x20, 0xffff, 0x1_0004, 0x1_0001] {
        assert!(!pointer_ok(bad), "{bad:#x}");
    }
}
fn btt_camera_owners_preserve_mode_through_handover() {
    fn call(owners: &CameraRequests, cam: &mut Camera, mode: &mut i32, owner: u8, want: bool) -> Option<i32> {
        owners.request(owner, want);
        let act = cam.step(owners.camera_wanted(false), Some(*mode));
        if let Some(to) = act { *mode = to; }
        act
    }
    let bits = [BTT_PLACEMENT_OWNER, TECH_CAMERA_OWNER, EGG_ZONE_OWNER, EDGE_JUGGLE_OWNER, TECH_PLACEMENT_OWNER];
    for before in [NORMAL, ZOOM, FIXED] {
        for owner in bits {
            let owners = CameraRequests::idle();
            let (mut cam, mut mode) = (Camera::IDLE, before);
            assert_eq!(call(&owners, &mut cam, &mut mode, owner, false), None);
            assert!(!owners.camera_wanted(false));
            assert_eq!(call(&owners, &mut cam, &mut mode, owner, true), (before != FIXED).then_some(FIXED));
            for _ in 0..60 { assert_eq!(call(&owners, &mut cam, &mut mode, owner, true), None); }
            call(&owners, &mut cam, &mut mode, owner, false);
            assert_eq!(mode, before);
            assert!(!cam.holding());
        }
        // Both, then each direction of handover: the remaining owner keeps Fixed and the first saved mode.
        for (first, second) in bits.into_iter().flat_map(|first| {
            bits.into_iter().filter(move |&second| first != second).map(move |second| (first, second))
        }) {
            let owners = CameraRequests::idle();
            let (mut cam, mut mode) = (Camera::IDLE, before);
            call(&owners, &mut cam, &mut mode, first, true);
            assert_eq!(call(&owners, &mut cam, &mut mode, second, true), None);
            assert_eq!(call(&owners, &mut cam, &mut mode, first, false), None);
            assert_eq!(mode, FIXED);
            assert!(cam.holding());
            for _ in 0..60 { assert_eq!(call(&owners, &mut cam, &mut mode, second, true), None); }
            call(&owners, &mut cam, &mut mode, second, false);
            assert_eq!(mode, before);
            assert!(!cam.holding());
        }
    }
    let mut cam = Camera::IDLE;
    let owners = CameraRequests::idle();
    let mut mode = FIXED;
    assert_eq!(call(&owners, &mut cam, &mut mode, 1, true), None);
    mode = ZOOM;
    assert_eq!(call(&owners, &mut cam, &mut mode, 1, false), Some(FIXED), "an original Fixed mode is preserved too");
}

fn each_placement_owner_holds_the_cpu_and_releases_the_previous_view() {
    let editors = [BTT_PLACEMENT_OWNER, TECH_PLACEMENT_OWNER, EGG_ZONE_OWNER, EDGE_JUGGLE_OWNER];
    for owner in editors {
        for (closed, covered, invalidated, mode) in [
            (true, false, false, Some(FIXED)), // Apply, Back
            (false, true, false, Some(FIXED)), // Menu interruption
            (false, false, true, Some(FIXED)), // Stage change and native reset
            (true, false, false, None), // Scene departure retires the manager
        ] {
            let owners = CameraRequests::idle();
            let mut camera = Camera::IDLE;
            assert!(!owners.placement(false));
            assert_eq!(camera.step(owners.camera_wanted(false), Some(ZOOM)), None);

            owners.request(owner, true);
            assert!(owners.placement(false));
            assert_eq!(camera.step(owners.camera_wanted(false), Some(ZOOM)), Some(FIXED));
            assert!(camera.holding());

            if closed { owners.request(owner, false); }
            if invalidated {
                owners.interrupt();
                assert!(!owners.placement(false));
            }
            assert_eq!(camera.step(owners.camera_wanted(covered), mode), mode.map(|_| ZOOM));
            assert!(!camera.holding());
            assert!(!owners.placement(covered));
            assert_eq!(camera.step(false, Some(ZOOM)), None);
        }
    }
}

fn running_tech_camera_does_not_hold_the_cpu() {
    let owners = CameraRequests::idle();
    owners.request(TECH_CAMERA_OWNER, true);
    assert!(owners.camera_wanted(false));
    assert!(!owners.placement(false));
    owners.request(TECH_PLACEMENT_OWNER, true);
    assert!(owners.placement(false));
    owners.interrupt();
    assert!(owners.camera_wanted(false));
    assert!(!owners.placement(false));
}

/// The cases of this module in one table.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("btt_room_uses_the_fixed_camera_and_the_normal_box", btt_room_uses_the_fixed_camera_and_the_normal_box as fn()),
        ("btt_camera_switches_by_state", btt_camera_switches_by_state as fn()),
        ("btt_camera_reasserts_at_a_rate_and_keeps_the_first_mode", btt_camera_reasserts_at_a_rate_and_keeps_the_first_mode as fn()),
        ("btt_camera_stands_down_without_a_manager", btt_camera_stands_down_without_a_manager as fn()),
        ("btt_camera_restores_each_placement_exit", btt_camera_restores_each_placement_exit as fn()),
        ("btt_camera_owners_preserve_mode_through_handover", btt_camera_owners_preserve_mode_through_handover as fn()),
        ("each_placement_owner_holds_the_cpu_and_releases_the_previous_view", each_placement_owner_holds_the_cpu_and_releases_the_previous_view as fn()),
        ("running_tech_camera_does_not_hold_the_cpu", running_tech_camera_does_not_hold_the_cpu as fn()),
        ("btt_camera_manager_pointer_is_checked_before_it_is_read", btt_camera_manager_pointer_is_checked_before_it_is_read as fn()),
    ]);
}
}
