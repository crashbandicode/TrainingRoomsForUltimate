lib_unit_tests! {
use super::*;
use core::sync::atomic::AtomicU64;

fn ring(slots: &mut [Slot]) -> Ring {
    Ring {
        abi: ABI,
        slot_count: slots.len() as u32,
        head: AtomicU64::new(0),
        frame: AtomicU64::new(7),
        dropped: AtomicU64::new(0),
        slots: slots.as_mut_ptr(),
    }
}

fn slots(n: usize) -> Vec<Slot> {
    (0..n).map(|_| Slot { state: AtomicU64::new(0), frame: 0, uptime_ms: 0, len: 0, truncated: 0, bytes: [0; SLOT_BYTES] }).collect()
}

fn state_export_layout_is_pinned() {
    assert_eq!(core::mem::size_of::<Slot>(), 32 + SLOT_BYTES);
    assert_eq!(core::mem::size_of::<Ring>(), 40);
    assert_eq!(core::mem::size_of::<FighterFrame>(), 40);
    assert_eq!(core::mem::size_of::<Table>(), 40);
    assert_eq!((FRAME_NANA, FRAME_PLAYER), (1, 2));
    assert_eq!((FREE, WRITING, READY, READING, state(0, READY)), (0, 1, 2, 3, 6));
    assert_eq!((REQ_SET_APPLY, REQ_SAVE, REQ_LOAD, REQ_ITEM, OK, REFUSED_SCENE, BAD_REQUEST, REFUSED_SETTINGS), (1, 2, 3, 4, 0, -1, -2, -3));
    assert_eq!((REQ_SETTLED, REQ_RELOAD_SAVES, BUSY), (5, 6, -4));
}

fn state_export_ring_passes_lines_in_order_with_the_frame() {
    let mut s = slots(4);
    let r = ring(&mut s);
    let mut out = [0u8; SLOT_BYTES];
    let mut next = 0;
    unsafe {
        assert!(matches!(r.pop(&mut next, &mut out), Pop::Empty));
        assert!(r.push(10, b"[Ui] press A", false));
        r.frame.store(8, core::sync::atomic::Ordering::Relaxed);
        assert!(r.push(11, b"[TrainingRooms] state saved slot=1 entry=0 (file queued)", false));
        let Pop::Line { len, frame, uptime_ms, truncated } = r.pop(&mut next, &mut out) else { panic!("first line") };
        assert_eq!((&out[..len], frame, uptime_ms, truncated), (&b"[Ui] press A"[..], 7, 10, false));
        let Pop::Line { len, frame, .. } = r.pop(&mut next, &mut out) else { panic!("second line") };
        assert_eq!((&out[..len], frame), (&b"[TrainingRooms] state saved slot=1 entry=0 (file queued)"[..], 8));
        assert!(matches!(r.pop(&mut next, &mut out), Pop::Empty));
        assert_eq!(s[0].state.load(core::sync::atomic::Ordering::Relaxed), state(0, FREE), "a read slot is free again");
    }
}

fn state_export_ring_counts_lapped_lines_and_truncation() {
    let mut s = slots(2);
    let r = ring(&mut s);
    let mut out = [0u8; SLOT_BYTES];
    let mut next = 0;
    unsafe {
        for i in 0..5u8 {
            assert!(r.push(u64::from(i), &[b'a' + i], false), "a ready, unread line of an older lap may be overwritten");
        }
        assert!(matches!(r.pop(&mut next, &mut out), Pop::Lost(3)));
        let Pop::Line { len, .. } = r.pop(&mut next, &mut out) else { panic!("d") };
        assert_eq!(&out[..len], b"d");
        assert!(r.push(9, &[b'x'; SLOT_BYTES + 3], false));
        let _ = r.pop(&mut next, &mut out);
        let Pop::Line { len, truncated, .. } = r.pop(&mut next, &mut out) else { panic!("long") };
        assert_eq!((len, truncated), (SLOT_BYTES, true));
    }
}

/// Review finding 1: a producer one lap ahead must never write a slot the reader is copying (v1 wrote it and let the
/// reader discover a changed sequence after the copy).
fn state_export_producer_drops_instead_of_writing_a_slot_being_read() {
    let mut s = slots(2);
    let r = ring(&mut s);
    unsafe {
        assert!(r.push(1, b"old", false));
        r.head.store(2, core::sync::atomic::Ordering::Relaxed);
        s[0].state.store(state(0, READING), core::sync::atomic::Ordering::Relaxed);
        assert!(!r.push(2, b"new", false));
        assert_eq!(&s[0].bytes[..3], b"old", "the reader's slot is untouched");
        assert_eq!(s[0].state.load(core::sync::atomic::Ordering::Relaxed), state(0, READING));
        assert_eq!(r.dropped.load(core::sync::atomic::Ordering::Relaxed), 1);
    }
}

/// Producer-producer wrap: a delayed writer still owns its slot; the writer one lap later drops.
fn state_export_producer_drops_instead_of_sharing_a_slot_with_a_writer() {
    let mut s = slots(2);
    let r = ring(&mut s);
    unsafe {
        s[0].state.store(state(0, WRITING), core::sync::atomic::Ordering::Relaxed);
        r.head.store(2, core::sync::atomic::Ordering::Relaxed);
        assert!(!r.push(2, b"later lap", false));
        assert_eq!(s[0].state.load(core::sync::atomic::Ordering::Relaxed), state(0, WRITING));
        assert_eq!(r.dropped.load(core::sync::atomic::Ordering::Relaxed), 1);
        // Without a lap: the reader waits for the writer of its own ticket and never copies a slot being written.
        r.head.store(1, core::sync::atomic::Ordering::Relaxed);
        let mut out = [0u8; SLOT_BYTES];
        let mut next = 0;
        assert!(matches!(r.pop(&mut next, &mut out), Pop::Empty), "the reader waits for the writer, it never copies");
        assert_eq!(next, 0);
        s[0].state.store(state(0, READY), core::sync::atomic::Ordering::Release);
        assert!(matches!(r.pop(&mut next, &mut out), Pop::Line { .. }));
    }
}

/// A newer lap already owns the slot: the reader counts the line lost without copying; a dropped ticket can be skipped.
fn state_export_reader_skips_newer_laps_and_dropped_tickets() {
    let mut s = slots(2);
    let r = ring(&mut s);
    let mut out = [0u8; SLOT_BYTES];
    unsafe {
        r.head.store(3, core::sync::atomic::Ordering::Relaxed);
        s[0].state.store(state(2, WRITING), core::sync::atomic::Ordering::Relaxed);
        let mut next = 0;
        assert!(matches!(r.pop(&mut next, &mut out), Pop::Lost(1)));
        assert_eq!(next, 1);
        assert!(matches!(r.pop(&mut next, &mut out), Pop::Empty), "ticket 1 was dropped by its producer");
        r.skip(&mut next);
        assert_eq!(next, 2);
    }
}

/// Many producers and one reader at once: every line the reader takes is whole (each line is one repeated byte).
fn state_export_ring_lines_stay_whole_under_concurrent_producers() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    let mut s = slots(8);
    let r = Arc::new(ring(&mut s));
    let done = Arc::new(AtomicBool::new(false));
    struct Shared(Arc<Ring>);
    unsafe impl Send for Shared {}
    let producers: Vec<_> = (0..4u8)
        .map(|p| {
            let ring = Shared(r.clone());
            std::thread::spawn(move || {
                let ring = ring;
                for i in 0..20_000u32 {
                    let byte = b'a' + ((u32::from(p) * 7 + i) % 26) as u8;
                    let len = 1 + (i as usize * 37) % (SLOT_BYTES - 1);
                    unsafe { ring.0.push(u64::from(i), &vec![byte; len], false) };
                }
            })
        })
        .collect();
    let reader = {
        let ring = Shared(r.clone());
        let done = done.clone();
        std::thread::spawn(move || {
            let ring = ring;
            let (mut next, mut out, mut whole, mut idle) = (0u64, [0u8; SLOT_BYTES], 0u64, 0u32);
            loop {
                match unsafe { ring.0.pop(&mut next, &mut out) } {
                    Pop::Line { len, .. } => {
                        assert!(out[..len].iter().all(|b| *b == out[0]), "a torn line");
                        whole += 1;
                        idle = 0;
                    }
                    Pop::Lost(_) => idle = 0,
                    Pop::Empty => {
                        idle += 1;
                        if idle > 1000 && next < ring.0.head.load(Ordering::Acquire) {
                            ring.0.skip(&mut next);
                            idle = 0;
                        } else if done.load(Ordering::Acquire) && next >= ring.0.head.load(Ordering::Acquire) {
                            return whole;
                        }
                    }
                }
            }
        })
    };
    for p in producers {
        p.join().unwrap();
    }
    done.store(true, Ordering::Release);
    assert!(reader.join().unwrap() > 0);
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
        ("state_export_producer_drops_instead_of_writing_a_slot_being_read", state_export_producer_drops_instead_of_writing_a_slot_being_read as fn()),
        ("state_export_producer_drops_instead_of_sharing_a_slot_with_a_writer", state_export_producer_drops_instead_of_sharing_a_slot_with_a_writer as fn()),
        ("state_export_reader_skips_newer_laps_and_dropped_tickets", state_export_reader_skips_newer_laps_and_dropped_tickets as fn()),
        ("state_export_ring_lines_stay_whole_under_concurrent_producers", state_export_ring_lines_stay_whole_under_concurrent_producers as fn()),
        ("state_export_offers_only_the_oracle_families_and_each_notice_once", state_export_offers_only_the_oracle_families_and_each_notice_once as fn()),
    ]);
}
}
