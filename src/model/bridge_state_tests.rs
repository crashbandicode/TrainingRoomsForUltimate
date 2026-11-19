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
        "notice.header=\"Eggs-ercise\"",
        "notice.body=\"Eggs 0   1:00\"",
        "scene=\"training\"",
        "rows.count=2",
        "rows.complete=1",
        "rows=\"Start\",\"Eggs mode\"",
    ]);
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
