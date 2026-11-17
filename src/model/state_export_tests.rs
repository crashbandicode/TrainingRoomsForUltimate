lib_unit_tests! {
use super::*;
use core::sync::atomic::AtomicU64;

fn ring(slots: &mut [Slot]) -> Ring {
    Ring { abi: ABI, slot_count: slots.len() as u32, head: AtomicU64::new(0), frame: AtomicU64::new(7), slots: slots.as_mut_ptr() }
}

fn slots(n: usize) -> Vec<Slot> {
    (0..n).map(|_| Slot { seq: AtomicU64::new(0), frame: 0, uptime_ms: 0, len: 0, truncated: 0, bytes: [0; SLOT_BYTES] }).collect()
}

fn state_export_layout_is_pinned() {
    assert_eq!(core::mem::size_of::<Slot>(), 32 + SLOT_BYTES);
    assert_eq!(core::mem::size_of::<Ring>(), 32);
    assert_eq!(core::mem::size_of::<FighterFrame>(), 40);
    assert_eq!((FRAME_NANA, FRAME_PLAYER), (1, 2));
    assert_eq!(core::mem::size_of::<Table>(), 40);
    assert_eq!((REQ_SET_APPLY, REQ_SAVE, REQ_LOAD, REQ_ITEM, OK, REFUSED_SCENE, BAD_REQUEST), (1, 2, 3, 4, 0, -1, -2));
}

fn state_export_ring_passes_lines_in_order_with_the_frame() {
    let mut s = slots(4);
    let r = ring(&mut s);
    let mut out = [0u8; SLOT_BYTES];
    let mut next = 0;
    unsafe {
        assert!(matches!(r.pop(&mut next, &mut out), Pop::Empty));
        r.push(10, b"[Ui] press A", false);
        r.frame.store(8, core::sync::atomic::Ordering::Relaxed);
        r.push(11, b"[TrainingRooms] state saved slot=1 entry=0 (file queued)", false);
        let Pop::Line { len, frame, uptime_ms, truncated } = r.pop(&mut next, &mut out) else { panic!("first line") };
        assert_eq!((&out[..len], frame, uptime_ms, truncated), (&b"[Ui] press A"[..], 7, 10, false));
        let Pop::Line { len, frame, .. } = r.pop(&mut next, &mut out) else { panic!("second line") };
        assert_eq!((&out[..len], frame), (&b"[TrainingRooms] state saved slot=1 entry=0 (file queued)"[..], 8));
        assert!(matches!(r.pop(&mut next, &mut out), Pop::Empty));
    }
}

fn state_export_ring_counts_lapped_lines_and_truncation() {
    let mut s = slots(2);
    let r = ring(&mut s);
    let mut out = [0u8; SLOT_BYTES];
    let mut next = 0;
    unsafe {
        for i in 0..5u8 {
            r.push(u64::from(i), &[b'a' + i], false);
        }
        assert!(matches!(r.pop(&mut next, &mut out), Pop::Lost(3)));
        let Pop::Line { len, .. } = r.pop(&mut next, &mut out) else { panic!("d") };
        assert_eq!(&out[..len], b"d");
        r.push(9, &[b'x'; SLOT_BYTES + 3], false);
        let _ = r.pop(&mut next, &mut out);
        let Pop::Line { len, truncated, .. } = r.pop(&mut next, &mut out) else { panic!("long") };
        assert_eq!((len, truncated), (SLOT_BYTES, true));
    }
}

fn state_export_offers_only_the_oracle_families_and_each_notice_once() {
    assert!(exported(b"[TrainingRooms] cpu_event camera_setup game=[0.0, 0.0, 0.0] class=Flat stage=3 write=None"));
    assert!(exported(b"[TrainingRooms] state saved slot=1 entry=0 (file queued)"));
    assert!(exported(b"[Ui] press A page=3 cursor=0 depth=1 screen=Page row=None recording=false"));
    assert!(!exported(b"[TrainingRooms] state pos entry=0 kind=0 x=0 y=0 lr=1 dmg=0 status=0 situation=0"));
    assert!(!exported(b"[GroupSwap] demand owner=0"));
    let last = AtomicU64::new(0);
    let notice = b"[TrainingRooms] notice_draw header=\"Save state\" body=\"Saved\" color=[1.0]";
    assert!(notice_changed(notice, &last));
    assert!(!notice_changed(notice, &last));
    assert!(notice_changed(b"[TrainingRooms] notice_draw header=\"Save state\" body=\"Loaded\" color=[1.0]", &last));
    assert!(notice_changed(b"[Ui] menu opened", &last));
}

#[test]
fn case_table() {
    crate::test_table::run(&[
        ("state_export_layout_is_pinned", state_export_layout_is_pinned as fn()),
        ("state_export_ring_passes_lines_in_order_with_the_frame", state_export_ring_passes_lines_in_order_with_the_frame as fn()),
        ("state_export_ring_counts_lapped_lines_and_truncation", state_export_ring_counts_lapped_lines_and_truncation as fn()),
        ("state_export_offers_only_the_oracle_families_and_each_notice_once", state_export_offers_only_the_oracle_families_and_each_notice_once as fn()),
    ]);
}
}
