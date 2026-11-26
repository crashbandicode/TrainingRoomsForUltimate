use super::*;

fn hold(polls: u32) -> Event {
    Event::Hold { dir: Direction::Down, polls, fires: 3, jump: true }
}

fn clip() -> Clip {
    Clip { pos: "none", ctx: "replay", total: 200, start: 12, end: 157, cursor: 156, playing: true, handle: "playhead", running: true,
        target: "cpu", marks: 2, mark: Some((90, None)), text: MoveText::new("66 frames into Special") }
}

fn fed(p1: u32, cpu: u32, preview: bool, hides: bool) -> Fed { Fed { p1, cpu, run: [0, 0], preview, hides } }

#[test]
fn a_fed_line_names_both_counts_and_both_flags_and_fits_its_buffer() {
    let mut got = Vec::new();
    FedSeen::default().sample(fed(0, 37, true, false), |s| { got.push(s.to_owned()); 0 });
    FedSeen::default().sample(Fed { p1: u32::MAX, cpu: u32::MAX, run: [u32::MAX, u32::MAX], preview: true, hides: true }, |s| { got.push(s.to_owned()); 0 });
    assert_eq!(got, vec!["fed p1=0 cpu=37 run.p1=0 run.cpu=0 preview=1 hides=0", "fed p1=4294967295 cpu=4294967295 run.p1=4294967295 run.cpu=4294967295 preview=1 hides=1"]);
}

#[test]
fn each_rise_and_the_return_go_out_on_the_frame_that_first_sees_them() {
    let frames = [
        (100, fed(0, 0, true, true)), (101, fed(0, 0, true, true)), (102, fed(1, 0, true, true)),
        (103, fed(2, 0, true, true)), (104, fed(3, 0, true, true)), (105, fed(3, 0, false, false)),
        (106, fed(3, 0, false, false)), (107, fed(4, 0, false, false)),
    ];
    let mut seen = FedSeen::default();
    let mut got = Vec::new();
    for (frame, now) in frames {
        seen.sample(now, |s| { got.push((frame, s.to_owned())); 0 });
    }
    assert_eq!(got, vec![
        (100, "fed p1=0 cpu=0 run.p1=0 run.cpu=0 preview=1 hides=1".to_string()), (102, "fed p1=1 cpu=0 run.p1=0 run.cpu=0 preview=1 hides=1".to_string()),
        (103, "fed p1=2 cpu=0 run.p1=0 run.cpu=0 preview=1 hides=1".to_string()), (104, "fed p1=3 cpu=0 run.p1=0 run.cpu=0 preview=1 hides=1".to_string()),
        (105, "fed p1=3 cpu=0 run.p1=0 run.cpu=0 preview=0 hides=0".to_string()), (107, "fed p1=4 cpu=0 run.p1=0 run.cpu=0 preview=0 hides=0".to_string()),
    ]);
}

#[test]
fn a_line_that_goes_out_late_is_announced_as_lost() {
    let mut seen = FedSeen::default();
    let mut got = Vec::new();
    seen.sample(fed(0, 1, false, false), |s| { got.push(s.to_owned()); 1 });
    seen.sample(fed(0, 1, false, false), |s| { got.push(s.to_owned()); 1 });
    seen.sample(fed(0, 2, false, false), |s| { got.push(s.to_owned()); 0 });
    seen.sample(fed(0, 2, false, false), |_| panic!("already sent"));
    assert_eq!(got, vec!["fed p1=0 cpu=1 run.p1=0 run.cpu=0 preview=0 hides=0", "fed lost=1", "fed lost=2", "fed p1=0 cpu=2 run.p1=0 run.cpu=0 preview=0 hides=0"]);

    let mut seen = FedSeen::default();
    seen.sample(fed(1, 0, false, false), |_| 1);
    let mut calls = 0;
    seen.sample(fed(1, 0, false, false), |_| { calls += 1; if calls == 1 { 0 } else { 1 } });
    let mut got = Vec::new();
    seen.sample(fed(1, 0, false, false), |s| { got.push(s.to_owned()); 0 });
    assert_eq!(got, vec!["fed lost=1", "fed p1=1 cpu=0 run.p1=0 run.cpu=0 preview=0 hides=0"]);
}

#[test]
fn the_fed_state_survives_its_words() {
    let mut seen = FedSeen::default();
    assert_eq!(FedSeen::from_words(seen.words()), seen);
    seen.sample(Fed { run: [5, u32::MAX], ..fed(u32::MAX, 7, true, false) }, |_| 1);
    seen.sample(Fed { run: [5, u32::MAX], ..fed(u32::MAX, 7, true, true) }, |_| 1);
    assert_eq!(FedSeen::from_words(seen.words()), seen);
    assert_ne!(seen, FedSeen::default());
    seen.sample(Fed { run: [7, 9], ..fed(3, u32::MAX, false, true) }, |_| 0);
    assert_eq!(FedSeen::from_words(seen.words()), seen);
}

#[test]
fn a_load_is_a_line_of_its_own_and_a_skipped_poll_is_announced_only_when_something_changed() {
    let at = |p1: u32| Fed { p1, run: [1, 0], ..Fed::default() };
    let mut seen = FedSeen::default();
    let mut got = Vec::new();
    seen.sample(at(0), |s| { got.push(s.to_owned()); 0 });
    seen.sample(at(1), |s| { got.push(s.to_owned()); 0 });
    seen.skip();
    seen.sample(at(1), |_| panic!("nothing changed"));
    seen.sample(at(2), |s| { got.push(s.to_owned()); 0 });
    seen.skip();
    seen.sample(at(4), |s| { got.push(s.to_owned()); 0 });
    assert_eq!(got, vec![
        "fed p1=0 cpu=0 run.p1=1 run.cpu=0 preview=0 hides=0", "fed p1=1 cpu=0 run.p1=1 run.cpu=0 preview=0 hides=0",
        "fed p1=2 cpu=0 run.p1=1 run.cpu=0 preview=0 hides=0", "fed lost=1", "fed p1=4 cpu=0 run.p1=1 run.cpu=0 preview=0 hides=0",
    ]);
}

#[test]
fn the_fed_sample_is_first_in_player_ones_poll_and_the_worker_never_sends_it() {
    let input = include_str!("../runtime/ui/input.rs");
    let poll = &input[input.find("pub fn on_p1_poll(").unwrap()..];
    let sample = poll.find("crate::runtime::bridge_state::note_fed();").unwrap();
    assert!(sample < poll.find("read_record(holder)").unwrap());
    let bridge = include_str!("../runtime/bridge_state.rs");
    let events = &bridge[bridge.find("fn events(").unwrap()..];
    let events = &events[..events.find("\n    }\n").unwrap()];
    assert!(!events.contains("fed"));
}

#[test]
fn clip_text_names_the_window_and_the_editor_cursor() {
    let c = clip();
    assert_eq!(c.text(), "clip ctx=replay total=200 in=12 out=157 cursor=156 playing=1 handle=playhead running=1 \
        target=cpu marks=2 mark=90-none move=\"66 frames into Special\"");
}


#[test]
fn a_clip_event_is_sent_once_per_change_and_retried_until_it_lands() {
    let mut seen = ClipSeen::default();
    let mut attempts = Vec::new();
    seen.send(Some(clip()), |s| { attempts.push(s.to_owned()); 1 });
    seen.send(Some(clip()), |s| { attempts.push(s.to_owned()); 0 });
    seen.send(Some(clip()), |_| panic!("already sent"));
    seen.send(None, |s| { attempts.push(s.to_owned()); 0 });
    seen.send(None, |_| panic!("closed already sent"));
    assert_eq!(attempts, vec![clip().text(), clip().text(), "clip closed".to_string()]);
    let mut seen = ClipSeen::default();
    seen.send(Some(clip()), |_| 1);
    let mut after_change = Vec::new();
    seen.send(None, |s| { after_change.push(s.to_owned()); 0 });
    assert_eq!(after_change, vec!["clip lost=1".to_string(), "clip closed".to_string()]);
}

#[test]
fn a_refused_open_and_close_are_reported_as_a_gap_after_the_reopen() {
    let mut seen = ClipSeen::default();
    let mut got = Vec::new();
    seen.send(Some(clip()), |s| { got.push(s.to_owned()); 1 });
    seen.send(None, |s| { got.push(s.to_owned()); 1 });
    seen.send(Some(clip()), |s| { got.push(s.to_owned()); 0 });
    seen.send(Some(clip()), |_| panic!("already sent"));
    assert_eq!(got, vec![clip().text(), "clip lost=1".to_string(), "clip lost=2".to_string(), clip().text()]);

    let mut seen = ClipSeen::default();
    seen.send(Some(clip()), |_| 1);
    let mut calls = 0;
    seen.send(None, |_| { calls += 1; if calls == 1 { 0 } else { 1 } });
    let mut got = Vec::new();
    seen.send(None, |s| { got.push(s.to_owned()); 0 });
    seen.send(None, |_| panic!("already sent"));
    assert_eq!(got, vec!["clip closed".to_string()]);
}

#[test]
fn room_panel_events_name_the_row_and_the_applied_pair() {
    assert_eq!(Event::RoomChange { row: -2, index: 3 }.text(), "room.panel row=-2 index=3");
    assert_eq!(Event::RoomApplied { partners: 2, behavior: 34 }.text(), "room.panel applied partners=2 behavior=34");
}

#[test]
fn queued_events_go_out_in_order_and_a_refused_one_is_sent_again() {
    let mut out = Outbox::default();
    out.accept(hold(52), 10);
    out.accept(hold(52), 11);
    out.accept(Event::RoomChange { row: 2, index: 3 }, 12);
    let mut got = Vec::new();
    out.flush(13, |s| { got.push(s.to_owned()); if got.len() == 2 { 1 } else { 0 } });
    out.flush(14, |s| { got.push(s.to_owned()); 0 });
    assert_eq!(got, vec![hold(52).text(), hold(52).text(), hold(52).text(), "room.panel row=2 index=3".to_string()]);
}

#[test]
fn old_or_overflowing_events_are_counted_as_lost() {
    let mut out = Outbox::default();
    for n in 0..17 { out.accept(hold(n), 100); }
    out.lost(2);
    let mut got = Vec::new();
    out.flush(101, |s| { got.push(s.to_owned()); 1 });
    out.flush(102, |s| { got.push(s.to_owned()); 0 });
    assert_eq!(got[0], "events lost=3");
    assert_eq!(got[1], "events lost=3");
    assert_eq!(got[2], hold(1).text());
    assert_eq!(got.len(), 18);

    out.accept(hold(3), 100);
    out.accept(hold(4), 2100);
    let mut got = Vec::new();
    out.flush(2101, |s| { got.push(s.to_owned()); 0 });
    assert_eq!(got, vec!["events lost=1".to_string(), hold(4).text()]);

    out.accept(hold(6), 100);
    let mut got = Vec::new();
    out.flush(2100, |s| { got.push(s.to_owned()); 0 });
    assert_eq!(got, vec![hold(6).text()]);
}

#[test]
fn the_move_text_is_bounded_and_never_needs_an_escape() {
    let quoted = MoveText::new(&"a\"b\\".repeat(30));
    assert!(quoted.as_str().len() == 48 && !quoted.as_str().contains(['"', '\\']), "{}", quoted.as_str());
    assert_eq!(MoveText::new(&"→".repeat(40)).as_str().chars().count(), 16);
    assert_eq!(Clip { mark: None, ..clip() }.text().split(" mark=").nth(1), Some("none move=\"66 frames into Special\""));
}
