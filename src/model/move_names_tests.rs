lib_unit_tests! {
use super::*;
use crate::model::action::ActionDesc;
use crate::model::combo::{Combo, Helper, LinkRec, Settings};
use crate::model::combo_record::ComboRecord;
use crate::model::frame_strip::{Link, Move, Shown, Tracker, Verdict};
use crate::model::frame_strip_plan::{link_moves, Text};
use crate::model::hash40::hash40;
use crate::model::recording_menu::combo_link_lines;
use std::sync::{Mutex, OnceLock};

const MASK: u64 = 0xff_ffff_ffff;
static TURN: Mutex<()> = Mutex::new(());
static TABLE: OnceLock<()> = OnceLock::new();

fn key(suffix: &str) -> u64 { hash40(&format!("ui_chara_{suffix}")) & MASK }
fn fighter(suffix: &str) -> FighterId { FighterId { chara: key(suffix), specials: [0; 4] } }

fn section(file: &mut Vec<u8>, magic: &[u8; 4], body: &[u8]) {
    file.extend_from_slice(magic);
    file.extend_from_slice(&(body.len() as u32).to_le_bytes());
    file.extend_from_slice(&[0; 8]);
    file.extend_from_slice(body);
    while !file.len().is_multiple_of(16) { file.push(0); }
}

fn message(entries: &[(String, Vec<u16>)]) -> Vec<u8> {
    let mut labels = Vec::new();
    labels.extend_from_slice(&1u32.to_le_bytes());
    labels.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    labels.extend_from_slice(&12u32.to_le_bytes());
    for (index, (label, _)) in entries.iter().enumerate() {
        labels.push(label.len() as u8);
        labels.extend_from_slice(label.as_bytes());
        labels.extend_from_slice(&(index as u32).to_le_bytes());
    }
    let mut texts = Vec::new();
    texts.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    let mut offset = 4 + entries.len() * 4;
    for (_, text) in entries {
        texts.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += (text.len() + 1) * 2;
    }
    for (_, text) in entries {
        for unit in text { texts.extend_from_slice(&unit.to_le_bytes()); }
        texts.extend_from_slice(&0u16.to_le_bytes());
    }
    let mut file = vec![0; 0x20];
    file[..8].copy_from_slice(b"MsgStdBn");
    file[8..10].copy_from_slice(&[0xff, 0xfe]);
    file[0x0c] = 1;
    file[0x0e..0x10].copy_from_slice(&2u16.to_le_bytes());
    section(&mut file, b"LBL1", &labels);
    section(&mut file, b"TXT2", &texts);
    let size = file.len() as u32;
    file[0x12..0x16].copy_from_slice(&size.to_le_bytes());
    file
}

fn texts(entries: &[(&str, &str)]) -> Vec<u8> {
    message(&entries.iter().map(|(label, text)| ((*label).to_owned(), text.encode_utf16().collect())).collect::<Vec<_>>())
}

fn shared_file() -> Vec<u8> {
    texts(&[
        ("nam_special_upper_00_alpha", "Test Rising Kick"),
        ("nam_special_upper_00_beta", "Test Lunar Lift"),
        ("nam_special_normal_00_alpha", "Test Pulse"),
        ("nam_final_arts_alpha", "Test Grand Finish"),
    ])
}

struct Guard { _guard: std::sync::MutexGuard<'static, ()> }
impl Drop for Guard {
    fn drop(&mut self) { PUBLISHED.set_on(false); }
}

fn serial() -> Guard {
    let guard = TURN.lock().unwrap_or_else(|error| error.into_inner());
    Guard { _guard: guard }
}

fn publish_table() {
    TABLE.get_or_init(|| PUBLISHED.publish(Box::new(Names::build(&shared_file()).unwrap())));
}

fn published() -> Guard {
    let guard = serial();
    publish_table();
    PUBLISHED.set_on(true);
    guard
}

fn isolated(name: &str, case: impl FnOnce()) {
    if std::env::var_os("TR_MOVE_NAMES_CHILD").is_some() {
        case();
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "model::move_names::tests::case_table", "--nocapture"])
        .env("TR_TABLE_CASE", name)
        .env("TR_MOVE_NAMES_CHILD", "1")
        .output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success() && stdout.contains("running 1 test"), "{name}: {stdout}\n{stderr}");
}

fn link(from: Move, to: Move) -> Link {
    Link {
        verdict: Verdict::True(None), hit: 0, free: None, actionable: 0, press: 0, input: 0,
        window: None, from, to, shown: [Shown::default(); 2], variants: [0; 2], owners: [0; 2], spot: None,
    }
}

fn combo(from: Move, to: Move) -> Combo {
    Combo::from_links([LinkRec::presses(from, to, Verdict::True(None), None, &[])]).unwrap()
}

fn strip_text(link: &Link) -> String {
    String::from_utf16(link_moves(link, 128).units()).unwrap()
}

fn bar_text(combo: Combo) -> String {
    let mut helper = Helper::new();
    helper.set_target(Some(combo));
    helper.step(&Tracker::new(), Settings::factory());
    let mut title = String::new();
    crate::model::combo_plan::Renderer::new().draw(
        &helper, &Text::new("Test"), None, true, (&crate::model::panes::strip_layout::C, false),
        |index, look, _| {
            if index == crate::model::combo_plan::at::TITLE {
                title = String::from_utf16(look.text.units()).unwrap();
            }
        },
    );
    title
}

fn menu_screens(record: &ComboRecord) -> crate::model::recording_menu::RecordingScreens {
    use crate::model::menu::table::COMBOS_SEQ;
    use crate::model::recording_menu::{Catalog, ComboFile, Face, RecordingScreens};
    let mut screens = RecordingScreens::new();
    screens.set_catalog(Catalog {
        combos: vec![ComboFile {
            input_only: record.combo.input_only, id: "test".into(), name: record.name.clone(),
            recorded_on: String::new(), made_from: None, take: None,
            combo: Box::new(record.combo), fighter: record.fighter, span: record.combo.len() as u32,
        }],
        ..Catalog::default()
    });
    screens.set_cursor(Face::Page, COMBOS_SEQ, 1, 1);
    screens
}

fn menu_texts(screens: &crate::model::recording_menu::RecordingScreens) -> (Vec<crate::model::recording_menu::ConnItem>, Vec<String>) {
    use crate::model::menu::table::COMBOS_SEQ;
    use crate::model::recording_menu::Face;
    (
        screens.connection_items(Face::Page, COMBOS_SEQ),
        screens.connections(Face::Page, COMBOS_SEQ).into_iter().map(|line| line.text).collect(),
    )
}

fn assert_screen_line(screens: &crate::model::recording_menu::RecordingScreens, expected: &str) {
    let (panel, help) = menu_texts(screens);
    assert!(panel.iter().any(|line| line.text == expected && line.kind == crate::model::recording_menu::ConnKind::Link), "panel: {panel:?}");
    assert!(help.iter().any(|line| line == expected), "help: {help:?}");
}

fn assert_menu_line(record: &ComboRecord, expected: &str) {
    assert_screen_line(&menu_screens(record), expected);
}

fn the_move_names_row_reads_its_setting() {
    let mut snapshot = crate::model::settings::Snapshot::factory();
    assert!(!Settings::from_snapshot(&snapshot).move_names);
    assert!(snapshot.set(ROW_MOVE_NAMES, &[1, 0]).is_ok());
    assert!(Settings::from_snapshot(&snapshot).move_names);
    assert!(snapshot.set(ROW_MOVE_NAMES, &[0, 1]).is_ok());
    assert!(!Settings::from_snapshot(&snapshot).move_names);
    let rows = &crate::model::menu::table::ROWS;
    let at = rows.iter().position(|row| row.id == ROW_MOVE_NAMES).expect("move names row");
    assert_eq!(rows[at].title, "Move names");
    assert_eq!(rows[at - 1].id, "combo_stick");
    assert_eq!(rows[at - 2].id, "spots_combo");
    assert_eq!(rows[at].group, rows[at - 1].group);
    assert!(matches!(rows[at].control, crate::model::menu::table::Control::Toggle));
    assert_eq!(rows[at].help, "On: moves show their names. Specials and Final Smashes use\nthe game's own names. Off: the short names.");
}

fn the_toggle_off_prints_todays_names_at_every_site() {
    isolated("the_toggle_off_prints_todays_names_at_every_site", || {
    let _guard = published();
    PUBLISHED.set_on(false);
    for mv in [
        Move::Jab, Move::DashAttack, Move::SideTilt, Move::UpTilt, Move::DownTilt, Move::SideSmash,
        Move::UpSmash, Move::DownSmash, Move::Aerial, Move::NeutralSpecial, Move::SideSpecial,
        Move::UpSpecial, Move::DownSpecial, Move::Other,
    ] {
        assert_eq!(Namer::OFF.name(mv, Normal::Jab3).as_str(), mv.name());
    }
    let rec = combo(Move::Jab, Move::UpTilt);
    assert_eq!(strip_text(&link(Move::Jab, Move::UpTilt)), "Jab > Up tilt");
    assert_eq!(bar_text(rec), "1 Jab>Up tilt");
    assert_eq!(combo_link_lines(&rec, Namer::OFF), ["L1 jab → up tilt TRUE"]);
    assert_menu_line(&ComboRecord::new("Test".into(), rec, None), "L1 jab → up tilt TRUE");
    });
}

fn the_toggle_on_names_the_strip_the_bar_and_the_menu_alike() {
    isolated("the_toggle_on_names_the_strip_the_bar_and_the_menu_alike", || {
    let _guard = serial();
    let id = fighter("alpha");
    PUBLISHED.set_on(true);
    PUBLISHED.set_player(Some((1, id)));
    PUBLISHED.set_target(Some(id));
    let rec = ComboRecord::new("Test".into(), combo(Move::Jab, Move::UpSpecial), None).with_fighter(Some(id));
    let screens = menu_screens(&rec);
    assert_screen_line(&screens, "L1 Jab → Up special TRUE");
    publish_table();
    assert_eq!(strip_text(&link(Move::Jab, Move::UpSpecial)), "Jab > Test Rising Kick");
    assert_eq!(bar_text(rec.combo), "1 Jab>Test Rising Kick");
    assert_eq!(combo_link_lines(&rec.combo, Namer::recorded(rec.fighter)), ["L1 Jab → Test Rising Kick TRUE"]);
    assert_screen_line(&screens, "L1 Jab → Test Rising Kick TRUE");
    PUBLISHED.set_on(false);
    assert_screen_line(&screens, "L1 jab → up b TRUE");
    PUBLISHED.set_on(true);
    assert_screen_line(&screens, "L1 Jab → Test Rising Kick TRUE");
    });
}

fn a_special_shows_the_games_own_name_for_the_fighter() {
    let names = Names::build(&shared_file()).unwrap();
    let namer = Namer::new(true, Some(&names), Some(fighter("alpha")));
    assert_eq!(namer.name(Move::UpSpecial, Normal::Unknown).as_str(), "Test Rising Kick");
    for (mv, expected) in [(Move::SideSpecial, "Side special"), (Move::DownSpecial, "Down special")] {
        assert_eq!(namer.name(mv, Normal::Unknown).as_str(), expected);
    }
    assert_eq!(namer.name(Move::NeutralSpecial, Normal::Unknown).as_str(), "Test Pulse");
}

fn the_final_smash_name_is_the_games_own() {
    let names = Names::build(&shared_file()).unwrap();
    assert_eq!(Namer::new(true, Some(&names), Some(fighter("alpha"))).final_smash().as_str(), "Test Grand Finish");
    assert_eq!(Namer::OFF.final_smash().as_str(), "Final Smash");
    assert_eq!(Namer::new(true, None, None).final_smash().as_str(), "Final Smash");
}

fn the_names_are_the_loaded_files() {
    let one = Names::build(&texts(&[("nam_special_upper_00_alpha", "Test First")])).unwrap();
    let two = Names::build(&texts(&[("nam_special_upper_00_alpha", "Test Second")])).unwrap();
    let name = |table| Namer::new(true, Some(table), Some(fighter("alpha"))).name(Move::UpSpecial, Normal::Unknown);
    assert_eq!(name(&one).as_str(), "Test First");
    assert_eq!(name(&two).as_str(), "Test Second");
}

fn a_mii_special_takes_its_variant_and_falls_back_to_the_first() {
    let names = Names::build(&texts(&[
        ("nam_special_upper_00_alpha", "Test First"),
        ("nam_special_upper_02_alpha", "Test Third"),
    ])).unwrap();
    let mut id = fighter("alpha");
    id.specials[2] = 2;
    assert_eq!(Namer::new(true, Some(&names), Some(id)).name(Move::UpSpecial, Normal::Unknown).as_str(), "Test Third");
    id.specials[2] = 1;
    assert_eq!(Namer::new(true, Some(&names), Some(id)).name(Move::UpSpecial, Normal::Unknown).as_str(), "Test First");
}

fn normals_take_their_common_names_from_the_engine_common_animation() {
    let cases = [
        ("attack_11", Normal::Jab1, "Jab 1"), ("attack_12", Normal::Jab2, "Jab 2"),
        ("attack_13", Normal::Jab3, "Jab 3"), ("attack_100_start", Normal::RapidJab, "Rapid jab"),
        ("attack_100", Normal::RapidJab, "Rapid jab"), ("attack_100_end", Normal::RapidJabFinisher, "Rapid jab finisher"),
        ("attack_dash", Normal::DashAttack, "Dash attack"),
        ("attack_s3_hi", Normal::ForwardTilt, "Forward tilt"), ("attack_s3_s", Normal::ForwardTilt, "Forward tilt"),
        ("attack_s3_lw", Normal::ForwardTilt, "Forward tilt"), ("attack_hi3", Normal::UpTilt, "Up tilt"),
        ("attack_lw3", Normal::DownTilt, "Down tilt"), ("attack_s4_hi", Normal::ForwardSmash, "Forward smash"),
        ("attack_s4_s", Normal::ForwardSmash, "Forward smash"), ("attack_s4_lw", Normal::ForwardSmash, "Forward smash"),
        ("attack_s4_hold", Normal::ForwardSmash, "Forward smash"), ("attack_hi4", Normal::UpSmash, "Up smash"),
        ("attack_hi4_hold", Normal::UpSmash, "Up smash"), ("attack_lw4", Normal::DownSmash, "Down smash"),
        ("attack_lw4_hold", Normal::DownSmash, "Down smash"), ("attack_air_n", Normal::NeutralAir, "Neutral air"),
        ("attack_air_f", Normal::ForwardAir, "Forward air"), ("attack_air_b", Normal::BackAir, "Back air"),
        ("attack_air_hi", Normal::UpAir, "Up air"), ("attack_air_lw", Normal::DownAir, "Down air"),
        ("catch", Normal::Grab, "Grab"), ("catch_dash", Normal::DashGrab, "Dash grab"),
        ("catch_turn", Normal::PivotGrab, "Pivot grab"), ("catch_attack", Normal::Pummel, "Pummel"),
        ("throw_f", Normal::ForwardThrow, "Forward throw"), ("throw_b", Normal::BackThrow, "Back throw"),
        ("throw_hi", Normal::UpThrow, "Up throw"), ("throw_lw", Normal::DownThrow, "Down throw"),
        ("cliff_attack_quick", Normal::LedgeAttack, "Ledge attack"), ("down_attack_u", Normal::GetupAttack, "Getup attack"),
        ("down_attack_d", Normal::GetupAttack, "Getup attack"), ("slip_attack", Normal::TripAttack, "Trip attack"),
    ];
    for (motion, normal, name) in cases {
        assert_eq!(Normal::of(hash40(motion)), normal, "{motion}");
        assert_eq!(normal.name(), Some(name));
        assert_eq!(Namer::new(true, None, None).name(Move::Other, normal).as_str(), name);
    }
    assert_eq!(Normal::Unknown.name(), None);
}

fn the_angle_stays_the_arrow_after_the_name() {
    isolated("the_angle_stays_the_arrow_after_the_name", || {
    let _guard = published();
    let mut l = link(Move::Jab, Move::SideTilt);
    l.shown[1].dirs[0] = Some(2);
    PUBLISHED.set_on(false);
    assert_eq!(strip_text(&l), "Jab > Side tilt ↑");
    PUBLISHED.set_on(true);
    assert_eq!(strip_text(&l), "Jab > Forward tilt ↑");
    });
}

fn an_animation_outside_the_list_keeps_the_status_familys_name() {
    assert_eq!(Normal::of(hash40("invented_motion")), Normal::Unknown);
    let on = Namer::new(true, None, None);
    for (mv, expected) in [
        (Move::Jab, "Jab"), (Move::DashAttack, "Dash attack"), (Move::SideTilt, "Forward tilt"),
        (Move::UpTilt, "Up tilt"), (Move::DownTilt, "Down tilt"), (Move::SideSmash, "Forward smash"),
        (Move::UpSmash, "Up smash"), (Move::DownSmash, "Down smash"), (Move::Aerial, "Aerial"),
        (Move::NeutralSpecial, "Neutral special"), (Move::SideSpecial, "Side special"),
        (Move::UpSpecial, "Up special"), (Move::DownSpecial, "Down special"), (Move::Other, "Move"),
    ] {
        assert_eq!(on.name(mv, Normal::Unknown).as_str(), expected);
    }
}

fn the_names_module_names_no_fighter() {
    let source = include_str!("move_names.rs").to_ascii_lowercase();
    for banned in ["mario", "pikachu", "miifighter", "eflame", "fighter_kind_"] {
        assert!(!source.contains(banned), "{banned}");
    }
}

fn the_fighter_is_matched_by_the_hash_of_its_ui_chara_label() {
    let names = Names::build(&shared_file()).unwrap();
    let id = hash40("ui_chara_alpha") & MASK;
    assert_eq!(names.special(id, Slot::Up, 0), Some("Test Rising Kick"));
    assert_eq!(names.special(id | (0xffu64 << 40), Slot::Up, 0), Some("Test Rising Kick"));
    assert_eq!(names.special(key("gamma"), Slot::Up, 0), None);
}

fn the_names_are_loaded_by_the_menu_worker_and_read_without_a_lock() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let read = |path: &str| std::fs::read_to_string(root.join(path)).unwrap_or_default();
    let loader = read("src/runtime/features/move_names.rs");
    let worker = read("src/runtime/ui/worker.rs");
    let boot = read("src/runtime/boot.rs");
    assert!(loader.contains("threads::run_transient(ThreadId::MoveNames,")
        || loader.contains("threads::run_transient(crate::model::thread_stacks::ThreadId::MoveNames,"));
    assert!(worker.contains("move_names::service(")
        || boot.contains("set_tick(crate::runtime::features::move_names::service")
        || boot.contains("set_tick(move_names::service"));
    for forbidden in ["std::fs", "Seek", "GetFileSize"] { assert!(!loader.contains(forbidden)); }
    let source = read("src/model/move_names.rs");
    assert!(!source.contains(".lock()"));
    for view in ["pub fn live(", "pub fn target(", "pub fn recorded("] { assert!(source.contains(view)); }
    assert!(read("src/model/frame_strip_plan.rs").contains("Namer::live("));
    assert!(read("src/model/combo_plan.rs").contains("Namer::target("));
    assert!(read("src/model/recording_menu.rs").contains("Namer::recorded("));
}

fn the_move_names_loader_runs_on_a_transient_stack_sized_for_the_arc_read() {
    use crate::model::thread_stacks::{self, ThreadId};
    let row = thread_stacks::of(ThreadId::MoveNames);
    let probe = thread_stacks::of(ThreadId::Probe);
    assert!(row.bytes >= row.required());
    assert_eq!(row.bytes, probe.bytes, "the size the same read was measured on");
    assert_eq!(row.external_bound, probe.external_bound);
    assert!(row.static_bound > 0 && row.static_bound != probe.static_bound, "its own entry's path, from its ELF");
    assert!(thread_stacks::transient(ThreadId::MoveNames));
    let source = include_str!("../runtime/threads.rs");
    assert!(source.contains("ThreadId::MoveNames => None"));
}

fn the_kept_table_is_one_small_block_and_the_file_is_not_kept() {
    let mut entries = Vec::new();
    for n in 0..89 {
        for label in ["normal_00", "side_00", "upper_00", "lower_00"] {
            entries.push((format!("nam_special_{label}_fiction{n}"), "abcdefghijklmnopqrstuvwx".to_owned()));
        }
        entries.push((format!("nam_final_arts_fiction{n}"), "abcdefghijklmnopqrstuvwx".to_owned()));
    }
    let borrowed: Vec<_> = entries.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let mut file = texts(&borrowed);
    let names = Names::build(&file).unwrap();
    let source = include_str!("move_names.rs");
    let declaration = source.split("pub struct Names").nth(1).unwrap_or("").split(['}', ';']).next().unwrap_or("");
    assert_eq!(declaration.matches("Box<[u8]>").count(), 1);
    assert_eq!(declaration.matches("Box<").count(), 1);
    assert!(!declaration.contains("Vec<") && !declaration.contains("String"));
    assert!(names.bytes() > 0 && names.bytes() < 16 * 1024);
    file.fill(0);
    drop(file);
    assert_eq!(names.special(key("fiction88"), Slot::Final, 0), Some("abcdefghijklmnopqrstuvwx"));
}

fn a_malformed_message_file_is_refused_whole() {
    let valid = texts(&[("nam_special_upper_00_alpha", "Test Rising Kick")]);
    let full = Names::build(&valid).unwrap();
    for n in 0..valid.len() {
        let result = std::panic::catch_unwind(|| Names::build(&valid[..n]));
        assert!(result.is_ok(), "truncation {n} panicked");
        if let Ok(candidate) = result.unwrap() {
            assert_eq!(candidate.special(key("alpha"), Slot::Up, 0), full.special(key("alpha"), Slot::Up, 0), "truncation {n}");
        }
    }
    for (at, bytes) in [(0, vec![0]), (8, vec![0xfe, 0xff]), (0x0c, vec![0]), (0x38, u32::MAX.to_le_bytes().to_vec())] {
        let mut bad = valid.clone();
        bad[at..at + bytes.len()].copy_from_slice(&bytes);
        assert!(std::panic::catch_unwind(|| Names::build(&bad)).unwrap().is_err(), "offset {at}");
    }
    let mut bad = valid.clone();
    let index = 0x20 + 16 + 12 + 1 + "nam_special_upper_00_alpha".len();
    bad[index..index + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(std::panic::catch_unwind(|| Names::build(&bad)).unwrap().is_err(), "text index");
    let bad_tag = message(&[("nam_special_upper_00_alpha".into(), vec![0x0e, 0, 0, 0xffff])]);
    assert!(std::panic::catch_unwind(|| Names::build(&bad_tag)).unwrap().is_err(), "tag size");
}

fn the_kept_byte_budget_includes_the_index() {
    // 64 distinct entries of 7 index bytes and 249 text bytes each.
    let mut entries: Vec<_> = (0..64)
        .map(|i| (format!("nam_final_arts_fiction{i}"), vec![0x61; 249]))
        .collect();
    assert_eq!(Names::build(&message(&entries)).unwrap().bytes(), 16 * 1024);
    entries[63].1.push(0x62);
    assert!(Names::build(&message(&entries)).is_err(), "one byte over the retained-table budget");
}

fn the_work_budget_counts_all_labels_and_only_selected_texts() {
    let selected = "nam_final_arts_alpha";
    let ignored = "unrelated";
    let file = texts(&[(ignored, "not decoded"), (selected, "A🌟")]);
    let cost = (5 + ignored.len()) + (5 + selected.len()) + 8; // Four UTF-16 units including NUL.
    let names = Names::build_with_work_budget(&file, cost).unwrap();
    assert_eq!(names.special(key("alpha"), Slot::Final, 0), Some("A🌟"));
    assert!(Names::build_with_work_budget(&file, cost - 1).is_err());
}

fn control_tags_are_dropped_from_a_name() {
    let mut text: Vec<u16> = "テスト".encode_utf16().collect();
    text.extend_from_slice(&[0x0e, 1, 2, 4, 0x304b, 0x306a, 0x0f, 1, 2]);
    text.extend("名".encode_utf16());
    let names = Names::build(&message(&[("nam_special_upper_00_alpha".into(), text)])).unwrap();
    assert_eq!(names.special(key("alpha"), Slot::Up, 0), Some("テスト名"));
}

fn the_help_texts_are_not_names() {
    let names = Names::build(&texts(&[
        ("nam_final_arts_help_alpha", "Test help"),
        ("nam_final_arts_alpha", "Test Finish"),
        ("nam_special_upper_00_help_beta", "Test help"),
    ])).unwrap();
    assert_eq!(names.special(key("alpha"), Slot::Final, 0), Some("Test Finish"));
    assert_eq!(names.special(key("help_alpha"), Slot::Final, 0), None);
    assert_eq!(names.special(key("help_beta"), Slot::Up, 0), None);
}

fn a_long_name_is_cut_at_a_character_boundary() {
    let source = format!("{}é{}", "x".repeat(95), "z".repeat(24));
    let names = Names::build(&texts(&[("nam_special_upper_00_alpha", &source)])).unwrap();
    let cut = Namer::new(true, Some(&names), Some(fighter("alpha"))).name(Move::UpSpecial, Normal::Unknown);
    assert_eq!(cut.as_str(), "x".repeat(95));
    assert!(cut.as_str().is_char_boundary(cut.as_str().len()));
}

fn a_take_recorded_on_one_fighter_keeps_its_names_under_another() {
    isolated("a_take_recorded_on_one_fighter_keeps_its_names_under_another", || {
    let _guard = published();
    let a = fighter("alpha");
    PUBLISHED.set_player(Some((2, fighter("beta"))));
    let record = ComboRecord::new("Test".into(), combo(Move::Jab, Move::UpSpecial), None).with_fighter(Some(a));
    assert_menu_line(&record, "L1 Jab → Test Rising Kick TRUE");
    let read: ComboRecord = serde_json::from_str(&serde_json::to_string(&record).unwrap()).unwrap();
    assert_eq!(read.fighter, Some(a));
    assert_menu_line(&read, "L1 Jab → Test Rising Kick TRUE");
    });
}

fn the_bar_names_its_target_by_the_fighter_it_was_recorded_on() {
    isolated("the_bar_names_its_target_by_the_fighter_it_was_recorded_on", || {
    let _guard = published();
    PUBLISHED.set_player(Some((2, fighter("beta"))));
    PUBLISHED.set_target(Some(fighter("alpha")));
    assert_eq!(Namer::target().name(Move::UpSpecial, Normal::Unknown).as_str(), "Test Rising Kick");
    assert_eq!(bar_text(combo(Move::Jab, Move::UpSpecial)), "1 Jab>Test Rising Kick");
    PUBLISHED.set_target(None);
    assert_eq!(Namer::target().name(Move::UpSpecial, Normal::Unknown).as_str(), "Up special");
    assert_eq!(bar_text(combo(Move::Jab, Move::UpSpecial)), "1 Jab>Up special");
    });
}

fn a_file_without_a_recorded_fighter_shows_the_common_names() {
    isolated("a_file_without_a_recorded_fighter_shows_the_common_names", || {
    let _guard = published();
    PUBLISHED.set_player(Some((1, fighter("alpha"))));
    let original = ComboRecord::new("Test".into(), combo(Move::Jab, Move::UpSpecial), None);
    let record: ComboRecord = serde_json::from_str(&serde_json::to_string(&original).unwrap()).unwrap();
    assert_eq!(record.fighter, None);
    assert_menu_line(&record, "L1 Jab → Up special TRUE");
    });
}

fn the_recorded_fighter_is_an_optional_key_older_builds_ignore() {
    let id = fighter("alpha");
    let original = ComboRecord::new("Test".into(), combo(Move::Jab, Move::UpSpecial), None);
    let baseline = serde_json::to_value(&original).unwrap();
    let record = original.with_fighter(Some(id));
    assert_ne!(record, ComboRecord::new("Test".into(), record.combo, None));
    let value = serde_json::to_value(&record).unwrap();
    assert_eq!(value["schema_version"], baseline["schema_version"]);
    assert_eq!(value["recorded_fighter"]["ui_chara"], format!("{:x}", id.chara));
    assert_eq!(value["recorded_fighter"]["specials"], serde_json::json!([0, 0, 0, 0]));
    let mut read: ComboRecord = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(read.fighter, Some(id));
    read.name = "Renamed".into();
    let renamed: ComboRecord = serde_json::from_value(serde_json::to_value(&read).unwrap()).unwrap();
    assert_eq!(renamed.fighter, Some(id));
    assert_eq!(renamed.crop_units(0, 1).unwrap().fighter, Some(id));
    let mut absent = value.clone();
    absent.as_object_mut().unwrap().remove("recorded_fighter");
    assert_eq!(serde_json::from_value::<ComboRecord>(absent).unwrap().fighter, None);
    for malformed in [
        serde_json::json!(null), serde_json::json!("bad"),
        serde_json::json!({"ui_chara":"oops","specials":[0,0,0,0]}),
        serde_json::json!({"ui_chara":format!("{:x}", id.chara),"specials":[0,0,0,3]}),
        serde_json::json!({"ui_chara":format!("{:x}", id.chara),"specials":[0,0,0]}),
        serde_json::json!({"ui_chara":"10000000000","specials":[0,0,0,0]}),
    ] {
        let mut bad = value.clone();
        bad["recorded_fighter"] = malformed;
        assert_eq!(serde_json::from_value::<ComboRecord>(bad).unwrap().fighter, None);
    }
}

fn a_save_takes_the_identity_only_of_the_fighter_the_take_was_recorded_on() {
    let id = fighter("alpha");
    assert_eq!(recorded_identity(Some((3, id)), 3), Some(id));
    assert_eq!(recorded_identity(Some((3, id)), 4), None);
    assert_eq!(recorded_identity(None, 3), None);
    assert!(include_str!("rr_host.rs").contains("recorded_identity("));
}

fn the_variant_numbers_are_fixed_and_fit_five_bits() {
    let variants = [
        Normal::Unknown, Normal::Jab1, Normal::Jab2, Normal::Jab3, Normal::RapidJab,
        Normal::RapidJabFinisher, Normal::DashAttack, Normal::ForwardTilt, Normal::UpTilt,
        Normal::DownTilt, Normal::ForwardSmash, Normal::UpSmash, Normal::DownSmash,
        Normal::NeutralAir, Normal::ForwardAir, Normal::BackAir, Normal::UpAir, Normal::DownAir,
        Normal::Grab, Normal::DashGrab, Normal::PivotGrab, Normal::Pummel, Normal::ForwardThrow,
        Normal::BackThrow, Normal::UpThrow, Normal::DownThrow, Normal::LedgeAttack,
        Normal::GetupAttack, Normal::TripAttack,
    ];
    let numbers: [u8; 29] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
        15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
    ];
    for (variant, number) in variants.into_iter().zip(numbers) {
        assert_eq!(variant as u8, number);
        assert_eq!(Normal::from_u8(number) as u8, number);
    }
    for number in [29, 31, 255] { assert_eq!(Normal::from_u8(number), Normal::Unknown); }
    assert!((Normal::TripAttack as u8) < 32);
}

fn live_strip_resets_share_the_identity_invalidation_boundary() {
    let native = include_str!("../runtime/features/input_display.rs");
    for signature in ["fn session_clear()", "fn fighter_initialised()", "fn sample_frame("] {
        let body = native.split_once(signature).unwrap().1.split_once("\n}").unwrap().0;
        assert!(body.contains("clear_live_strip("), "{signature} resets the timeline without expiring P1's identity");
    }
}

fn same_kind_after_a_strip_reset_reads_the_new_loadout() {
    let names = Published::new();
    let mut strip = Tracker::new();
    let kind = 0x49;
    let first = FighterId { specials: [0, 1, 2, 0], ..fighter("mii") };
    let second = FighterId { specials: [2, 0, 1, 2], ..first };
    names.observe_player(Some(kind), |_| Some(first));
    let retained = CaptureOwner::new(kind, recorded_identity(names.player(), kind));
    names.set_target(retained.fighter());
    names.observe_player(Some(kind), |_| panic!("stable frames must not reread native work values"));
    strip.clear_live_strip(&names);
    assert_eq!(names.player(), None, "no frame from the new body yet");
    names.observe_player(Some(kind), |_| None);
    assert_eq!(names.player(), None, "unavailable native identity cannot reuse an old Mii");
    names.observe_player(Some(kind), |_| Some(second));
    assert_eq!(names.player(), Some((kind, second)));
    assert_eq!((retained.fighter(), names.target()), (Some(first), Some(first)));
}

fn the_file_is_read_only_after_the_mod_filesystem_is_mounted() {
    assert_eq!(load_step(false, false, None, false, 0, 0), LoadStep::Idle);
    assert_eq!(load_step(true, true, None, false, 0, 0), LoadStep::Idle);
    assert_eq!(load_step(true, false, None, false, 0, 0), LoadStep::GiveUp);
    assert_eq!(load_step(true, false, Some(false), false, 0, 90_000), LoadStep::Wait);
    assert_eq!(load_step(true, false, Some(true), false, 0, 0), LoadStep::Try);
    assert_eq!(load_step(true, false, Some(true), false, 1, 1_999), LoadStep::Wait);
    assert_eq!(load_step(true, false, Some(true), false, 1, 2_000), LoadStep::Try);
    assert_eq!(load_step(true, false, Some(true), false, 3, 2_000), LoadStep::GiveUp);
    assert_eq!(load_step(true, false, Some(false), false, 3, 0), LoadStep::GiveUp);
    assert_eq!(load_step(true, false, Some(true), true, 1, 2_000), LoadStep::GiveUp);
    assert_eq!(load_step(true, false, Some(false), true, 0, 0), LoadStep::GiveUp);
    let loader = include_str!("../runtime/features/move_names.rs");
    assert!(loader.contains("arcrop_register_event_callback"));
    let service = loader.split("pub fn service()").nth(1).unwrap().split("#[derive(Debug)]").next().unwrap();
    assert!(service.contains("load_step("));
    let (before, after) = service.split_once("LoadStep::Try =>").unwrap();
    assert!(!before.contains("run_transient("));
    assert_eq!(after.matches("run_transient(").count(), 1);
    assert!(include_str!("../runtime/boot.rs").contains("move_names::register("));
}

fn only_selected_message_texts_are_decoded_and_compacted() {
    let long = format!("{}é{}", "x".repeat(254), "y".repeat(20));
    let entries = [
        ("unrelated_label".to_owned(), vec![0xd800]),
        ("nam_special_upper_00_alpha".to_owned(), long.encode_utf16().collect()),
        ("nam_special_upper_16_alpha".to_owned(), vec![0xd800]),
    ];
    let names = Names::build(&message(&entries)).unwrap();
    assert_eq!(names.bytes(), 7 + 254);
    assert_eq!(names.special(key("alpha"), Slot::Up, 0), Some("x".repeat(254).as_str()));
    assert_eq!(names.special(key("alpha"), Slot::Up, 16), None);
    let mut unrelated_index = message(&entries);
    let index = 0x20 + 16 + 12 + 1 + "unrelated_label".len();
    unrelated_index[index..index + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(Names::build(&unrelated_index).is_ok());
    let bad = message(&[("nam_special_upper_00_alpha".into(), vec![0xd800])]);
    assert!(Names::build(&bad).is_err());
}

fn written(desc: ActionDesc, namer: Namer<'_>) -> String {
    let mut text = String::new();
    desc.write(&mut text, false, namer).unwrap();
    text
}

fn the_action_models_variant_is_the_engine_common_animations() {
    for motion in [
        "attack_11", "attack_12", "attack_13", "attack_100", "attack_100_end", "attack_dash", "attack_s3_hi",
        "attack_hi4", "attack_air_b", "catch", "catch_dash", "catch_turn", "catch_attack", "throw_f", "throw_lw",
        "cliff_attack_quick", "down_attack_u", "slip_attack", "invented_motion",
    ] {
        assert_eq!(crate::model::action::variant(hash40(motion)), Normal::of(hash40(motion)) as u8, "{motion}");
    }
}

fn the_strip_names_the_variant_the_game_chose() {
    isolated("the_strip_names_the_variant_the_game_chose", || {
    let _guard = published();
    let mut l = link(Move::Jab, Move::Grab);
    l.variants = [Normal::Jab2 as u8, Normal::DashGrab as u8];
    PUBLISHED.set_on(false);
    assert_eq!(strip_text(&l), "Jab > Grab");
    PUBLISHED.set_on(true);
    assert_eq!(strip_text(&l), "Jab 2 > Dash grab");
    l.variants = [0; 2];
    assert_eq!(strip_text(&l), "Jab > Grab");
    });
}

fn a_saved_link_keeps_its_variant_for_the_bar_and_the_menu() {
    isolated("a_saved_link_keeps_its_variant_for_the_bar_and_the_menu", || {
    let _guard = published();
    let mut rec = LinkRec::presses(Move::Jab, Move::Throw, Verdict::True(None), None, &[]);
    rec.set_descriptions([
        ActionDesc::of(Move::Jab, Shown::default(), Normal::Jab3 as u8),
        ActionDesc::of(Move::Throw, Shown::default(), Normal::ForwardThrow as u8),
    ]);
    let combo = Combo::from_links([rec]).unwrap();
    assert_eq!(combo_link_lines(&combo, Namer::recorded(None)), ["L1 Jab 3 → Forward throw TRUE"]);
    assert_eq!(bar_text(combo), "1 Jab 3>Forward throw");
    let record = ComboRecord::new("Test".into(), combo, None);
    let read: ComboRecord = serde_json::from_str(&serde_json::to_string(&record).unwrap()).unwrap();
    assert_eq!(combo_link_lines(&read.combo, Namer::recorded(None)), ["L1 Jab 3 → Forward throw TRUE"]);
    PUBLISHED.set_on(false);
    assert_eq!(combo_link_lines(&read.combo, Namer::recorded(None)), ["L1 jab → throw TRUE"]);
    assert_eq!(bar_text(combo), "1 Jab>Throw");
    });
}

fn the_final_smash_shows_the_games_name_for_the_fighter() {
    let names = Names::build(&shared_file()).unwrap();
    let desc = ActionDesc::of(Move::FinalSmash, Shown::default(), 0);
    assert_eq!(written(desc, Namer::new(true, Some(&names), Some(fighter("alpha")))), "Test Grand Finish");
    assert_eq!(written(desc, Namer::new(true, Some(&names), None)), "Final Smash");
    assert_eq!(written(desc, Namer::new(true, Some(&names), Some(fighter("beta")))), "Final Smash");
    assert_eq!(written(desc, Namer::OFF), "Final Smash");
}

fn the_toggle_off_prints_the_action_models_text() {
    for (mv, normal) in [
        (Move::Jab, Normal::Jab2), (Move::Grab, Normal::DashGrab), (Move::Throw, Normal::ForwardThrow),
        (Move::FinalSmash, Normal::Unknown), (Move::UpSpecial, Normal::Unknown), (Move::Aerial, Normal::BackAir),
    ] {
        assert_eq!(written(ActionDesc::of(mv, Shown::default(), normal as u8), Namer::OFF), mv.name());
    }
    let mut up = Shown::default();
    up.dirs[0] = Some(2);
    assert_eq!(written(ActionDesc::of(Move::SideTilt, up, Normal::ForwardTilt as u8), Namer::OFF), "Side tilt ↑");
    assert_eq!(written(ActionDesc::of(Move::SideTilt, up, Normal::ForwardTilt as u8), Namer::new(true, None, None)), "Forward tilt ↑");
}

fn a_strip_on_screen_takes_the_toggle_on_its_next_draw() {
    isolated("a_strip_on_screen_takes_the_toggle_on_its_next_draw", || {
    use crate::model::frame_strip::Sample;
    use crate::model::frame_strip_plan::{at, Renderer, StripDraw};
    use crate::model::input_delay::MappedInputs;
    use crate::model::panes::strip_layout::C;
    let _guard = published();
    let stun = Sample { hitstun: true, reaction: Some(50.0), ..Sample::default() };
    let hit = Sample { stop: true, ..stun };
    let mut tracker = Tracker::new();
    tracker.step(Sample { attack: Some(Move::DashAttack), status: 0x29, ..Sample::default() }, hit, MappedInputs::NEUTRAL);
    tracker.step(Sample { actionable: true, ..Sample::default() }, stun, MappedInputs::NEUTRAL);
    tracker.step(
        Sample { attack: Some(Move::Jab), status: 0x27, motion_frame: 8.0, motion: hash40("attack_12"), ..Sample::default() },
        stun, MappedInputs::NEUTRAL,
    );
    tracker.step(
        Sample { attack: Some(Move::Jab), status: 0x28, motion: hash40("attack_100"), ..Sample::default() },
        hit, MappedInputs::NEUTRAL,
    );
    let link = tracker.latest_link().expect("a dash attack into a rapid jab");
    let on = strip_text(&link);
    PUBLISHED.set_on(false);
    let off = strip_text(&link);
    assert!(on.ends_with("Rapid jab") && off.ends_with("Jab") && on != off, "{on} / {off}");
    // The LAST row's newest moves (Link history On) and the folded LINK line (Off): each on one renderer, as on screen.
    for (history_enabled, pane) in [(true, at::MOVE_TEXT), (false, at::READ_SUB + 1)] {
        let mut renderer = Renderer::new();
        let draw = |renderer: &mut Renderer| {
            let mut wrote = None;
            let (writes, _) = renderer.draw_with_spec(
                &tracker, StripDraw { shown: true, back: None, spots: false, parry: None, history_enabled, cells: false, spec: &C },
                |index, look, changed| if index == pane && changed.text { wrote = Some(String::from_utf16(look.text.units()).unwrap()) },
            );
            (writes, wrote)
        };
        let kept = |renderer: &Renderer| renderer.written(pane).map(|look| String::from_utf16(look.text.units()).unwrap());
        PUBLISHED.set_on(false);
        let (_, first) = draw(&mut renderer);
        assert!(first.as_deref().is_some_and(|text| text.ends_with(off.as_str())), "pane {pane}: {first:?}");
        assert_eq!(draw(&mut renderer), (0, None), "pane {pane}: an unchanged draw writes nothing");
        PUBLISHED.set_on(true);
        let (_, named) = draw(&mut renderer);
        assert!(named.as_deref().is_some_and(|text| text.ends_with(on.as_str())), "pane {pane}: {named:?}");
        assert_eq!(kept(&renderer), named, "pane {pane}: the cache holds the names");
        assert_eq!(draw(&mut renderer), (0, None), "pane {pane}: the names stay on an unchanged draw");
        PUBLISHED.set_on(false);
        let (_, back) = draw(&mut renderer);
        assert_eq!(back, first, "pane {pane}: Off writes the first text again");
        assert_eq!(kept(&renderer), first);
        assert_eq!(draw(&mut renderer), (0, None));
    }
    });
}

fn a_bar_on_screen_takes_the_toggle_on_its_next_draw() {
    isolated("a_bar_on_screen_takes_the_toggle_on_its_next_draw", || {
    use crate::model::combo_plan::{at, Renderer};
    let _guard = published();
    let mut rec = LinkRec::presses(Move::Jab, Move::Throw, Verdict::True(None), None, &[]);
    rec.set_descriptions([
        ActionDesc::of(Move::Jab, Shown::default(), Normal::Jab3 as u8),
        ActionDesc::of(Move::Throw, Shown::default(), Normal::ForwardThrow as u8),
    ]);
    let mut helper = Helper::new();
    helper.set_target(Some(Combo::from_links([rec]).unwrap()));
    helper.step(&Tracker::new(), Settings::factory());
    let mut renderer = Renderer::new();
    let draw = |renderer: &mut Renderer| {
        let mut wrote = None;
        let (writes, _) = renderer.draw(
            &helper, &Text::new("Test"), None, true, (&crate::model::panes::strip_layout::C, false),
            |index, look, changed| if index == at::TITLE && changed.text { wrote = Some(String::from_utf16(look.text.units()).unwrap()) },
        );
        (writes, wrote)
    };
    PUBLISHED.set_on(false);
    assert_eq!(draw(&mut renderer).1.as_deref(), Some("1 Jab>Throw"));
    assert_eq!(draw(&mut renderer), (0, None), "an unchanged draw writes nothing");
    PUBLISHED.set_on(true);
    assert_eq!(draw(&mut renderer).1.as_deref(), Some("1 Jab 3>Forward throw"));
    assert_eq!(draw(&mut renderer), (0, None), "the names stay on an unchanged draw");
    PUBLISHED.set_on(false);
    assert_eq!(draw(&mut renderer).1.as_deref(), Some("1 Jab>Throw"));
    assert_eq!(draw(&mut renderer), (0, None));
    });
}

#[test]
fn case_table() {
    crate::test_table::run(&[
        ("the_move_names_row_reads_its_setting", the_move_names_row_reads_its_setting as fn()),
        ("the_toggle_off_prints_todays_names_at_every_site", the_toggle_off_prints_todays_names_at_every_site as fn()),
        ("the_toggle_on_names_the_strip_the_bar_and_the_menu_alike", the_toggle_on_names_the_strip_the_bar_and_the_menu_alike as fn()),
        ("a_special_shows_the_games_own_name_for_the_fighter", a_special_shows_the_games_own_name_for_the_fighter as fn()),
        ("the_final_smash_name_is_the_games_own", the_final_smash_name_is_the_games_own as fn()),
        ("the_names_are_the_loaded_files", the_names_are_the_loaded_files as fn()),
        ("a_mii_special_takes_its_variant_and_falls_back_to_the_first", a_mii_special_takes_its_variant_and_falls_back_to_the_first as fn()),
        ("normals_take_their_common_names_from_the_engine_common_animation", normals_take_their_common_names_from_the_engine_common_animation as fn()),
        ("the_angle_stays_the_arrow_after_the_name", the_angle_stays_the_arrow_after_the_name as fn()),
        ("an_animation_outside_the_list_keeps_the_status_familys_name", an_animation_outside_the_list_keeps_the_status_familys_name as fn()),
        ("the_names_module_names_no_fighter", the_names_module_names_no_fighter as fn()),
        ("the_fighter_is_matched_by_the_hash_of_its_ui_chara_label", the_fighter_is_matched_by_the_hash_of_its_ui_chara_label as fn()),
        ("the_names_are_loaded_by_the_menu_worker_and_read_without_a_lock", the_names_are_loaded_by_the_menu_worker_and_read_without_a_lock as fn()),
        ("the_move_names_loader_runs_on_a_transient_stack_sized_for_the_arc_read", the_move_names_loader_runs_on_a_transient_stack_sized_for_the_arc_read as fn()),
        ("the_kept_table_is_one_small_block_and_the_file_is_not_kept", the_kept_table_is_one_small_block_and_the_file_is_not_kept as fn()),
        ("a_malformed_message_file_is_refused_whole", a_malformed_message_file_is_refused_whole as fn()),
        ("the_kept_byte_budget_includes_the_index", the_kept_byte_budget_includes_the_index as fn()),
        ("the_work_budget_counts_all_labels_and_only_selected_texts", the_work_budget_counts_all_labels_and_only_selected_texts as fn()),
        ("control_tags_are_dropped_from_a_name", control_tags_are_dropped_from_a_name as fn()),
        ("the_help_texts_are_not_names", the_help_texts_are_not_names as fn()),
        ("a_long_name_is_cut_at_a_character_boundary", a_long_name_is_cut_at_a_character_boundary as fn()),
        ("a_take_recorded_on_one_fighter_keeps_its_names_under_another", a_take_recorded_on_one_fighter_keeps_its_names_under_another as fn()),
        ("the_bar_names_its_target_by_the_fighter_it_was_recorded_on", the_bar_names_its_target_by_the_fighter_it_was_recorded_on as fn()),
        ("a_file_without_a_recorded_fighter_shows_the_common_names", a_file_without_a_recorded_fighter_shows_the_common_names as fn()),
        ("the_recorded_fighter_is_an_optional_key_older_builds_ignore", the_recorded_fighter_is_an_optional_key_older_builds_ignore as fn()),
        ("a_save_takes_the_identity_only_of_the_fighter_the_take_was_recorded_on", a_save_takes_the_identity_only_of_the_fighter_the_take_was_recorded_on as fn()),
        ("the_variant_numbers_are_fixed_and_fit_five_bits", the_variant_numbers_are_fixed_and_fit_five_bits as fn()),
        ("live_strip_resets_share_the_identity_invalidation_boundary", live_strip_resets_share_the_identity_invalidation_boundary as fn()),
        ("same_kind_after_a_strip_reset_reads_the_new_loadout", same_kind_after_a_strip_reset_reads_the_new_loadout as fn()),
        ("the_file_is_read_only_after_the_mod_filesystem_is_mounted", the_file_is_read_only_after_the_mod_filesystem_is_mounted as fn()),
        ("only_selected_message_texts_are_decoded_and_compacted", only_selected_message_texts_are_decoded_and_compacted as fn()),
        ("the_action_models_variant_is_the_engine_common_animations", the_action_models_variant_is_the_engine_common_animations as fn()),
        ("the_strip_names_the_variant_the_game_chose", the_strip_names_the_variant_the_game_chose as fn()),
        ("a_saved_link_keeps_its_variant_for_the_bar_and_the_menu", a_saved_link_keeps_its_variant_for_the_bar_and_the_menu as fn()),
        ("the_final_smash_shows_the_games_name_for_the_fighter", the_final_smash_shows_the_games_name_for_the_fighter as fn()),
        ("the_toggle_off_prints_the_action_models_text", the_toggle_off_prints_the_action_models_text as fn()),
        ("a_strip_on_screen_takes_the_toggle_on_its_next_draw", a_strip_on_screen_takes_the_toggle_on_its_next_draw as fn()),
        ("a_bar_on_screen_takes_the_toggle_on_its_next_draw", a_bar_on_screen_takes_the_toggle_on_its_next_draw as fn()),
    ]);
}
}
