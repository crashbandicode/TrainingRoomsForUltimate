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
    // Already on the Fixed camera when asked (the game's own option): nothing switched, nothing to restore.
    let mut cam = Camera::IDLE;
    assert_eq!((cam.step(true, Some(FIXED)), cam.holding()), (None, false));
    assert_eq!(cam.step(false, Some(FIXED)), None);
}

fn btt_camera_manager_pointer_is_checked_before_it_is_read() {
    // A manager pointer is aligned and not in the first pages.
    assert!(pointer_ok(0x0000_0042_1234_5678 & !7));
    for bad in [0, 1, 0x20, 0xffff, 0x1_0004, 0x1_0001] {
        assert!(!pointer_ok(bad), "{bad:#x}");
    }
}
/// The cases of this module in one table.
#[test]
fn case_table() {
    crate::test_table::run(&[
        ("btt_room_uses_the_fixed_camera_and_the_normal_box", btt_room_uses_the_fixed_camera_and_the_normal_box as fn()),
        ("btt_camera_switches_by_state", btt_camera_switches_by_state as fn()),
        ("btt_camera_reasserts_at_a_rate_and_keeps_the_first_mode", btt_camera_reasserts_at_a_rate_and_keeps_the_first_mode as fn()),
        ("btt_camera_stands_down_without_a_manager", btt_camera_stands_down_without_a_manager as fn()),
        ("btt_camera_manager_pointer_is_checked_before_it_is_read", btt_camera_manager_pointer_is_checked_before_it_is_read as fn()),
    ]);
}
}
