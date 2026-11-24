lib_unit_tests! {

use crate::model::hud_row::{hud_row, hud_row_box, Take, REC_W, TEXT_UNITS};
use crate::model::notice_box;
use crate::model::panes::{self as p, Kind};

#[test]
fn a_take_shows_rec_in_the_roles_colour() {
    for (role, text, color) in [
        (Take::Player, "REC P1", [255, 83, 92, 255]),
        (Take::Cpu, "REC CPU", [75, 164, 255, 255]),
    ] {
        let row = hud_row(Some(role), [false; 2], "").unwrap();
        assert_eq!(row.text.as_str(), text);
        assert_eq!((row.color, row.chip, row.take, row.line), (color, color, Some(role), false));
    }
}

#[test]
fn no_take_and_no_line_hides_the_row() {
    assert!(hud_row(None, [false; 2], "").is_none());
}

#[test]
fn the_targets_line_alone_is_drawn_as_before() {
    for line in ["10/10 0:07.12", "3/10   A place  B undo  X start"] {
        for strip in [false, true] {
            for bar in [false, true] {
                let row = hud_row(None, [false; 2], line).unwrap();
                assert_eq!(row.text.as_str(), line);
                assert_eq!((row.color, row.chip, row.take, row.line), ([236, 238, 242, 255], [255; 4], None, true));
                assert_eq!(hud_row_box(&row, strip, bar), p::btt_hud_box(line, strip, bar));
            }
        }
    }
}

#[test]
fn a_take_during_the_targets_run_shares_the_line() {
    let row = hud_row(Some(Take::Player), [false; 2], "10/10 0:07.12").unwrap();
    assert_eq!(row.text.as_str(), "REC P1 · 10/10 0:07.12");
    assert_eq!((row.color, row.chip, row.take, row.line), ([255, 83, 92, 255], [255, 83, 92, 255], Some(Take::Player), true));
    assert_eq!(hud_row_box(&row, true, true), p::btt_hud_box("REC P1 · 10/10 0:07.12", true, true));
    for line in ["x".repeat(96), "🟦".repeat(96)] {
        let row = hud_row(Some(Take::Cpu), [false; 2], &line).unwrap();
        assert!(row.text.as_str().starts_with("REC CPU · "));
        assert_eq!(row.text.as_str().chars().count(), TEXT_UNITS);
        assert!(row.line);
    }
}

#[test]
fn the_rec_row_is_short_and_holds_its_text() {
    for role in [Take::Player, Take::Cpu] {
        let row = hud_row(Some(role), [false; 2], "").unwrap();
        for strip in [false, true] {
            for bar in [false, true] {
                let rect = hud_row_box(&row, strip, bar);
                assert_eq!(rect, [-936.0, 504.0, 140.0, 48.0]);
                assert_eq!(rect[0], p::NOTICE_LEFT);
            }
        }
    }
    let Kind::Text { font, .. } = p::spec(p::IDX_BTT + 3).kind else { panic!("BTT text pane") };
    assert!(notice_box::measure("REC CPU".encode_utf16(), font.0).0 <= 0.9 * (REC_W - 64.0));
}

#[test]
fn a_notice_stacks_under_the_rec_row() {
    let rec = hud_row_box(&hud_row(Some(Take::Player), [false; 2], "").unwrap(), false, false);
    let old = p::btt_hud_box("10/10 0:07.12", false, false);
    let shift = notice_box::stack(59.0, Some(rec));
    assert_eq!(shift, notice_box::stack(59.0, Some(old)));
    assert!(shift < 0.0);
}

#[test]
fn the_rec_row_keeps_clear_of_the_lobbys_panels() {
    fn meet(a: [f32; 4], b: [f32; 4]) -> bool {
        a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
    }
    let leave_battle = [-960.0, 447.0, -543.0, 540.0];
    let waiting_pill = [588.0, 450.0, 937.0, 522.0];
    let right_column = [697.0, -30.0, 930.0, 184.0];
    let damage_panels = [685.0, 12.0, 925.0, 331.0];
    for (drop, native) in [
        (p::LOBBY_DROP, vec![leave_battle, waiting_pill, right_column]),
        (0.0, vec![damage_panels]),
    ] {
        for role in [Take::Player, Take::Cpu] {
            for strip in [false, true] {
                for bar in [false, true] {
                    let [x, y, w, h] = hud_row_box(&hud_row(Some(role), [false; 2], "").unwrap(), strip, bar);
                    let chip = [x, y - h / 2.0 - drop, x + w, y + h / 2.0 - drop];
                    let c = p::strip_layout::C;
                    let [sx, sy, sw, sh] = c.strip.bounds(true, bar);
                    let strip_box = [sx - sw / 2.0, c.strip.origin_y + sy - drop - sh / 2.0,
                                     sx + sw / 2.0, c.strip.origin_y + sy - drop + sh / 2.0];
                    assert!(!strip || !meet(chip, strip_box));
                    for panel in &native {
                        assert!(!meet(chip, *panel), "drop {drop}: {chip:?} meets {panel:?}");
                    }
                }
            }
        }
    }
}

#[test]
fn the_hud_draw_shows_the_open_take_in_training_and_the_waiting_room() {
    let source = include_str!("../../runtime/features/hud_draw.rs");
    let body = |name: &str| {
        let start = source.find(name).unwrap_or_else(|| panic!("missing {name}"));
        &source[start..start + source[start..].find("\n}\n").expect("end of function")]
    };
    let draw = body("pub unsafe fn draw(");
    let row = draw.find("hud_row::hud_row(").expect("the HUD builds the recording row");
    assert!(row > draw.find("if !waiting {").expect("Training-only branch"), "the row is shared by both scenes");
    assert!(draw.contains("rr_host::capture_take()"), "the row reads the open take");
    assert!(draw.contains("(!menu_open).then(|| hud_row::hud_row("), "the menu hides the row");
    assert!(draw.contains("let drawn = write_btt(") && draw.contains("bridge_state::note_recording_indicator(drawn)"),
            "the bridge reads what the row's writer drew");
    let write = body("unsafe fn write_btt(");
    assert!(write.contains("set_text_color") && write.contains("set_rect_color") && write.contains("p::IDX_BTT + 2"),
            "the text and chip use the row's colours");
    assert!(write.contains("-> Option<hud_row::Take>") && write.contains("if container.is_null() { return None; }"),
            "no row in the layout (the stock layout): nothing drawn");
    assert!(write.contains("row.take.filter(|_| !picture.is_null() && !text.is_null())"),
            "a take counts only with its chip and text panes");
    let bridge = include_str!("../../runtime/bridge_state.rs");
    assert!(bridge.contains("m::indicator_stamp(crate::runtime::clock::uptime_ms(), drawn)"), "each pass is stamped");
    assert!(bridge.contains("m::recording_indicator(RECORDING_INDICATOR.load(Ordering::Relaxed), now_ms, SCENE_FRESH_MS)"),
            "a stale pass reads as no chip");
    let view = include_str!("../../runtime/ui/view.rs");
    assert!(!view.contains("[255, 83, 92, 255]") && !view.contains("[75, 164, 255, 255]"));
    assert!(view.contains("hud_row::PLAYER_COLOR") && view.contains("hud_row::CPU_COLOR"),
            "the roster uses the same role colours");
}
}
