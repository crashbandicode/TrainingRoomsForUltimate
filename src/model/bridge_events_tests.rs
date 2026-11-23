use super::*;

fn hold(polls: u32) -> Event {
    Event::Hold { dir: Direction::Down, polls, fires: 3, jump: true }
}

fn clip() -> Clip {
    Clip { ctx: "replay", total: 200, start: 12, end: 157, head: 156, playing: true, handle: "playhead", running: true }
}

#[test]
fn clip_text_names_the_window_the_playhead_and_frames_consumed() {
    let c = clip();
    assert_eq!(c.text(), "clip ctx=replay total=200 in=12 out=157 head=156 playing=1 handle=playhead running=1");
    assert_eq!(c.head - c.start + 1, 145);
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
