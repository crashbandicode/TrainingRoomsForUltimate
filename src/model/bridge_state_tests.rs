use super::*;

fn eggs_start() -> View<'static> {
    View {
        open: true,
        page: "fighters_stages",
        depth: 1,
        group: Some(22),
        cursor: 0,
        row: Some("Start"),
        row_id: "",
        action: "start.eggs",
        ack: 7,
        ack_result: "handled",
        scene: "training",
        list: "rows/22",
        list_count: 2,
        list_index: Some(0),
        list_value: ">",
        notice: Some(("Eggs-ercise", "Eggs 0   1:00")),
        rows: &["Start", "Eggs mode"],
        ..View::default()
    }
}

fn lines(text: &str) -> Vec<&str> {
    text.lines().collect()
}

#[test]
fn the_snapshot_is_fixed_order_with_schema_ack_and_focus_first_and_rows_last() {
    let mut out = String::new();
    assert!(snapshot(&eggs_start(), &mut out));
    assert_eq!(lines(&out), [
        "schema=tr.menu/1",
        "ack=7",
        "ack.result=\"handled\"",
        "focus=fighters_stages/22/0/start.eggs",
        "open=1",
        "page=\"fighters_stages\"",
        "depth=1",
        "group=22",
        "cursor=0",
        "row=\"Start\"",
        "row_id=\"\"",
        "action=\"start.eggs\"",
        "editor=\"\"",
        "editor.sel=-1",
        "editor.sel_key=\"\"",
        "keyboard=0",
        "game=\"\"",
        "game.running=0",
        "zone.step=\"\"",
        "zone.last=\"\"",
        "zone.stage=-1",
        "zone.bins=\"\"",
        "zone.y=\"\"",
        "zone.left=\"\"",
        "zone.right=\"\"",
        "zone.saved=\"\"",
        "notice.header=\"Eggs-ercise\"",
        "notice.body=\"Eggs 0   1:00\"",
        "scene=\"training\"",
        "recording.popup.open=0",
        "recording.popup.role=\"\"",
        "recording.page.role=\"\"",
        "recording.capture.role=\"\"",
        "recording.indicator=\"\"",
        "list=\"rows/22\"",
        "list.count=2",
        "list.index=0",
        "list.value=\">\"",
        "recording.preview=\"\"",
        "rows.count=2",
        "rows.complete=1",
        "rows=\"Start\",\"Eggs mode\"",
    ]);
}

#[test]
fn the_snapshot_says_a_drill_preview_hides_the_menu() {
    for role in ["drill", ""] {
        let view = View { recording_preview: role, ..View::default() };
        let mut out = String::new();
        assert!(snapshot(&view, &mut out));
        assert!(out.contains(&format!("\nrecording.preview=\"{role}\"\n")));
    }
}

#[test]
fn the_recording_popup_page_and_capture_report_independent_roles() {
    let view = View {
        recording_popup_open: true,
        recording_popup_role: "cpu",
        recording_page_role: "player",
        recording_capture_role: "player",
        ..View::default()
    };
    let mut out = String::new();
    snapshot(&view, &mut out);
    assert!(out.contains("\nopen=0\n"));
    assert!(out.contains("\nrecording.popup.open=1\n"));
    assert!(out.contains("\nrecording.popup.role=\"cpu\"\n"));
    assert!(out.contains("\nrecording.page.role=\"player\"\n"));
    assert!(out.contains("\nrecording.capture.role=\"player\"\n"));
}

#[test]
fn the_snapshot_says_which_take_the_hud_shows() {
    for role in ["player", "cpu", ""] {
        let view = View { recording_indicator: role, ..View::default() };
        let mut out = String::new();
        assert!(snapshot(&view, &mut out));
        assert!(out.contains(&format!("\nrecording.capture.role=\"\"\nrecording.indicator=\"{role}\"\n")));
    }
}

#[test]
fn the_indicator_is_the_take_the_hud_drew_on_a_fresh_pass() {
    use crate::model::hud_row::Take;
    let drew = |take| indicator_stamp(1000, take);
    assert_eq!(recording_indicator(drew(Some(Take::Player)), 1200, 500), "player");
    assert_eq!(recording_indicator(drew(Some(Take::Cpu)), 1500, 500), "cpu");
    assert_eq!(recording_indicator(drew(None), 1000, 500), "", "the pass drew no chip: the stock layout, the menu, no take");
    assert_eq!(recording_indicator(drew(Some(Take::Player)), 1501, 500), "", "no pass since: a waiting room without the host");
    assert_eq!(recording_indicator(0, 10, 500), "", "never drawn");
}

#[test]
fn no_focused_list_has_empty_keys_and_an_index_of_minus_one() {
    let mut out = String::new();
    snapshot(&View::default(), &mut out);
    assert!(out.contains("\nlist=\"\"\nlist.count=0\nlist.index=-1\nlist.value=\"\"\n"), "{out}");
}

#[test]
fn extra_lines_come_after_the_list_and_before_the_rows() {
    let view = View { extra: "place.first=1\nplace.role=\"cpu\"\n", ..eggs_start() };
    let mut out = String::new();
    assert!(snapshot(&view, &mut out));
    assert!(out.contains("list.value=\">\"\nrecording.preview=\"\"\nplace.first=1\nplace.role=\"cpu\"\nrows.count=2\n"));

    // Leave room for the fixed fields, but not for either long optional row.
    let extra = format!("place.long=\"{}\"\n", "x".repeat(MAX / 2));
    let long = "row".repeat(200);
    let rows = [long.as_str(), long.as_str()];
    let view = View { extra: &extra, rows: &rows, ..eggs_start() };
    assert!(!snapshot(&view, &mut out));
    assert!(out.len() <= MAX && out.contains("rows.complete=0"));
}

#[test]
fn the_page_level_has_no_group_and_strings_are_escaped() {
    let view = View { open: true, page: "cpu", row: Some("Say \"hi\"\\now\nnext"), action: "row", ..View::default() };
    let mut out = String::new();
    snapshot(&view, &mut out);
    assert!(out.contains("\nfocus=cpu/-/0/row\n"), "{out}");
    assert!(out.contains("\ngroup=-1\n"));
    assert!(out.contains("\nrow=\"Say \\\"hi\\\"\\\\now\\nnext\"\n"), "{out}");
    assert!(out.lines().all(|l| !l.is_empty()));
}

#[test]
fn an_open_editor_has_an_integer_selection_and_a_quoted_key() {
    let view = View { editor: Some("parry_saved"), editor_sel: Some(2), editor_sel_key: "rename", keyboard: true, ..eggs_start() };
    let mut out = String::new();
    snapshot(&view, &mut out);
    assert!(out.contains("\neditor=\"parry_saved\"\neditor.sel=2\neditor.sel_key=\"rename\"\nkeyboard=1\n"), "{out}");
}

fn editor_zone() -> ZoneEditor {
    ZoneEditor { stage: 3, step: 2, bins: [200, 40, 255, 8], last: ZoneEditor::APPLIED,
        vertical: [-30.0, 80.5], sides: [Some([-200.25, -90.0]), None] }
}

#[test]
fn the_zone_editor_snapshot_shows_open_values_and_closed_outcomes() {
    let zone = editor_zone();
    let mut out = String::new();
    let mut view = View { zone: Some(zone), zone_up: true, ..View::default() };
    snapshot(&view, &mut out);
    let zone_lines = |text: &str| text.lines().filter(|line| line.starts_with("zone.")).map(str::to_owned).collect::<Vec<_>>();
    assert_eq!(zone_lines(&out), [
        "zone.step=\"inner\"", "zone.last=\"\"", "zone.stage=3", "zone.bins=\"200,40,255,8\"",
        "zone.y=\"-30.00,80.50\"", "zone.left=\"-200.25,-90.00\"", "zone.right=\"\"", "zone.saved=\"\"",
    ]);
    view.zone_up = false;
    view.zone_saved = Some(zone.bins);
    snapshot(&view, &mut out);
    assert_eq!(zone_lines(&out), [
        "zone.step=\"\"", "zone.last=\"applied\"", "zone.stage=3", "zone.bins=\"\"",
        "zone.y=\"\"", "zone.left=\"\"", "zone.right=\"\"", "zone.saved=\"200,40,255,8\"",
    ]);
    for (last, expected) in [(ZoneEditor::CANCELLED, "cancelled"), (0, "")] {
        view.zone = Some(ZoneEditor { last, ..zone });
        snapshot(&view, &mut out);
        assert!(out.contains(&format!("\nzone.last=\"{expected}\"\n")), "{out}");
    }
    view.zone = None;
    view.zone_up = true;
    view.zone_saved = None;
    snapshot(&view, &mut out);
    assert!(out.contains("\nzone.step=\"\"\nzone.last=\"\"\nzone.stage=-1\n"), "{out}");
}

#[test]
fn zone_editor_packing_round_trips_and_reserves_zero() {
    for zone in [
        editor_zone(),
        ZoneEditor { stage: 308, step: 4, bins: [0, 255, 64, 32], last: ZoneEditor::CANCELLED,
            vertical: [-137.85, -2.25], sides: [Some([-250.0, -90.5]), Some([10.0, 120.25])] },
        ZoneEditor { stage: 0, step: 0, bins: [0; 4], last: 0, vertical: [0.0; 2], sides: [None; 2] },
    ] {
        let packed = zone.pack();
        assert_ne!(packed[0], 0);
        assert_eq!(ZoneEditor::unpack(packed), Some(zone));
    }
    assert_eq!(ZoneEditor::unpack([0; 4]), None);
}

#[test]
fn zone_cell_reads_complete_states_and_declines_an_in_progress_write() {
    let cell = ZoneCell::new();
    assert_eq!(cell.read(), Some([0; 4]));
    let words = editor_zone().pack();
    cell.write(words);
    assert_eq!(cell.read(), Some(words));
    cell.seq.store(1, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(cell.read(), None);
}

#[test]
fn long_zone_lines_and_optional_rows_stay_under_the_snapshot_bound() {
    let zone = ZoneEditor { stage: i16::MAX.into(), step: 4, bins: [255; 4], last: 0,
        vertical: [f32::MIN, f32::MAX], sides: [Some([f32::MIN, f32::MAX]); 2] };
    let long: Vec<String> = (0..200).map(|i| format!("Row number {i} with a long \"title\"")).collect();
    let rows: Vec<&str> = long.iter().map(String::as_str).collect();
    let view = View { zone: Some(zone), zone_up: true, zone_saved: Some([255; 4]), rows: &rows, ..eggs_start() };
    let mut out = String::new();
    assert!(!snapshot(&view, &mut out));
    assert!(out.len() <= MAX, "{}", out.len());
    assert!(out.contains("\nzone.step=\"preview\"\n"), "{out}");
}

#[test]
fn rows_that_do_not_fit_are_cut_and_marked_incomplete_under_the_bound() {
    let long: Vec<String> = (0..200).map(|i| format!("Row number {i} with a long \"title\"")).collect();
    let rows: Vec<&str> = long.iter().map(String::as_str).collect();
    let view = View { rows: &rows, ..eggs_start() };
    let mut out = String::new();
    assert!(!snapshot(&view, &mut out));
    assert!(out.len() <= MAX, "{}", out.len());
    assert!(out.contains("\nrows.count=200\nrows.complete=0\nrows="));
    assert!(out.starts_with("schema=tr.menu/1\nack=7\n"), "the gate keys always fit");
}

#[test]
fn lookup_is_due_at_most_once_a_second_for_thirty_seconds_then_off_for_the_boot() {
    let mut d = Discovery::default();
    assert!(d.enabled());
    assert!(d.due(100));
    assert!(!d.due(500));
    assert!(d.due(1_100));
    assert!(!d.due(1_200));
    assert!(d.due(29_000));
    assert!(!d.due(30_200), "past the window");
    assert_eq!(d.state, Search::Disabled);
    assert!(!d.enabled());
    let mut found = Discovery::default();
    assert!(found.due(0));
    found.found();
    assert!(!found.due(5_000), "no more lookups once found");
    assert!(found.enabled());
    let mut bad = Discovery::default();
    bad.due(0);
    bad.disable();
    assert!(!bad.enabled() && !bad.due(2_000));
}


#[test]
fn a_busy_registration_retries_neither_too_early_nor_after_success() {
    let mut search = Discovery::default();
    assert!(search.due(0));
    assert_eq!(search.registration(-6), None);
    assert!(!search.due(999));
    assert!(search.due(1_000), "the bridge registry was briefly busy");
    assert_eq!(search.registration(0), Some(0));
    assert!(!search.due(2_000));
    assert_eq!(search.state, Search::Found);

    let mut rejected = Discovery::default();
    assert!(rejected.due(0));
    assert_eq!(rejected.registration(-5), None);
    assert!(!rejected.enabled());

    let mut exhausted = Discovery::default();
    assert!(exhausted.due(0));
    assert_eq!(exhausted.registration(-6), None);
    assert!(!exhausted.due(30_001));
    assert!(!exhausted.enabled());
}

#[test]
fn a_changed_snapshot_is_wanted_and_a_dropped_publish_stays_dirty_until_it_lands() {
    let mut p = Publisher::default();
    assert!(p.wants(b"a=1"));
    p.published(b"a=1", 0);
    assert!(!p.wants(b"a=1"), "unchanged and landed");
    assert!(p.wants(b"a=2"));
    p.published(b"a=2", 1); // dropped (the bridge's reader held the buffer)
    assert!(p.wants(b"a=2"), "kept dirty after a drop");
    p.published(b"a=2", 0);
    assert!(!p.wants(b"a=2"));
}

#[test]
fn scene_is_the_practice_layouts_fresh_pass_only() {
    assert_eq!(scene_name(1000 << 2 | 1, 1200, 500), "training");
    assert_eq!(scene_name(1000 << 2 | 2, 1500, 500), "lobby");
    assert_eq!(scene_name(1000 << 2 | 1, 1501, 500), "", "a stale pass is no scene");
    assert_eq!(scene_name(0, 10, 500), "", "never passed");
}

#[test]
fn omitted_rows_are_marked_incomplete() {
    let mut out = String::new();
    let view = View { rows: &[], rows_omitted: true, ..eggs_start() };
    assert!(!snapshot(&view, &mut out));
    assert!(out.contains("rows.count=0\nrows.complete=0\nrows="), "{out}");
}

#[test]
fn the_header_must_declare_exactly_this_table_and_eight_slots() {
    let ok = |size: u32, slots: u32| header_usable(u32::from_le_bytes(*b"BRST"), 1, size, 1, MAX as u32, slots, 48);
    assert!(ok(48, 8));
    assert!(!ok(56, 8), "a larger declared table is not this ABI");
    assert!(!ok(40, 8), "nor a smaller one");
    assert!(!ok(48, 1), "one slot is not the contract");
    assert!(!ok(48, 16));
    assert!(!header_usable(0, 1, 48, 1, MAX as u32, 8, 48), "magic");
}

#[test]
fn a_notice_is_reported_only_while_the_hud_shows_it() {
    use crate::model::notify::{Body, Queue};
    let mut queue = Queue::default();
    queue.post("Eggs", Body::copied("Eggs 1"), 3, [255; 4]);
    let shown = |out: &crate::model::notify::TickOut, painted: (&'static str, &'static str)| shown_notice(out.visible, painted);
    let first = queue.tick(true, false);
    assert!(first.visible && first.draw.is_some());
    assert_eq!(shown(&first, ("Eggs", "Eggs 1")), Some(("Eggs", "Eggs 1")));
    let hidden = queue.tick(true, true);
    assert_eq!(shown(&hidden, ("Eggs", "Eggs 1")), None, "the menu open hides it");
    let again = queue.tick(true, false);
    assert!(again.draw.is_none(), "unchanged: nothing is repainted");
    assert_eq!(shown(&again, ("Eggs", "Eggs 1")), Some(("Eggs", "Eggs 1")), "visible and unchanged: kept");
    let mut last = queue.tick(true, false);
    for _ in 0..4 {
        last = queue.tick(true, false);
    }
    assert_eq!(shown(&last, ("Eggs", "Eggs 1")), None, "expired");
    assert_eq!((ack_result(true), ack_result(false)), ("handled", "ignored"));
}
