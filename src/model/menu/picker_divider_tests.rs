lib_unit_tests! {
use super::*;
use crate::editors::roster::RosterEditor;
use crate::engine::swap::roster_grid::{Role, RosterGrid, CELLS};
use crate::model::frame::{DivWrite, Frame, View, POP_BOX};
use crate::model::panes as p;

fn menu() -> Menu {
    Menu::new(Settings::factory(), Env::default())
}

fn frame(m: &Menu) -> Frame {
    let mut f = Frame::new();
    crate::model::screens::frame_of(m, &mut f, |_| "");
    f
}

#[derive(Debug, PartialEq)]
struct DivPane {
    shown: bool,
    rect: [f32; 4],
    color: [u8; 4],
}

fn table_rule() -> DivPane {
    let spec = p::spec(p::IDX_CONN_DIV);
    let p::Kind::Rect { color } = spec.kind else { panic!("divider must be a rectangle") };
    let pane = DivPane { shown: true, rect: [spec.x, spec.y, spec.w, spec.h], color };
    assert_eq!(pane.rect, [110.0, 15.0, 3.0, 640.0]);
    assert_eq!(pane.color, p::RULE);
    pane
}

/// The divider pane after the view applied each frame's write in order, from the pane table's start.
fn div_after(frames: &[Frame]) -> DivPane {
    let spec = p::spec(p::IDX_CONN_DIV);
    let p::Kind::Rect { color } = spec.kind else { panic!("divider must be a rectangle") };
    let mut pane = DivPane { shown: spec.visible, rect: [spec.x, spec.y, spec.w, spec.h], color };
    for f in frames {
        match f.conn_div() {
            DivWrite::Hide => pane.shown = false,
            DivWrite::Show => pane.shown = true,
            DivWrite::Rect(rect, color) => {
                pane.shown = true;
                pane.rect = rect;
                pane.color = color;
            }
        }
    }
    pane
}

fn picker_open(m: &mut Menu, editor: Editor, use_: RosterUse, frames: &mut Vec<Frame>) -> RosterEditor {
    frames.push(frame(m));
    m.open_editor(editor);
    frames.push(frame(m));
    let (cell, button) = m.take_roster_press().expect("picker seek");
    let mut roster = RosterEditor::new(use_);
    roster.apply(m, usize::from(cell), button);
    frames.push(frame(m));
    roster
}

fn cell(label: &str) -> usize {
    let kind = crate::model::fighter_choices::CHOICES.iter().find(|(_, name)| *name == label).unwrap().0;
    let grid = RosterGrid::new(Role::Cpu);
    (0..CELLS).find(|&i| grid.info(i).and_then(|c| c.kind()) == Some(kind)).unwrap()
}

fn assert_box(f: &Frame) -> [f32; 4] {
    let DivWrite::Rect(rect, color) = f.conn_div() else { panic!("picker box not written") };
    assert_eq!(Some(rect), f.pop_box());
    assert_eq!(color, POP_BOX);
    assert_ne!(rect, table_rule().rect);
    rect
}

fn picker_back_out(m: &mut Menu, frames: &mut Vec<Frame>) {
    for _ in 0..2 {
        m.press(Button::B);
        let f = frame(m);
        assert_eq!(f.conn_div(), DivWrite::Hide);
        frames.push(f);
    }
}

fn open(m: &mut Menu, title: &str, frames: &mut Vec<Frame>) {
    let row = ROWS.iter().position(|r| r.title == title).unwrap();
    let page = ROWS[row].page();
    for _ in 0..6 {
        if m.page() == page { break; }
        m.press(Button::R);
        frames.push(frame(m));
    }
    for _ in 0..=m.entry_count(page) {
        if m.focus() == Some(Entry::Row(row)) {
            m.press(Button::A);
            frames.push(frame(m));
            return;
        }
        m.press(Button::Down);
        frames.push(frame(m));
    }
    panic!("could not reach {title}");
}

fn practice_picker(game: Drill) -> (Menu, Vec<Frame>) {
    let mut m = menu();
    let mut frames = Vec::new();
    let mut roster = picker_open(&mut m, Editor::PracticeCpu(game), RosterUse::PracticeCpu(game), &mut frames);
    roster.apply(&mut m, cell("Donkey Kong"), Button::A);
    frames.push(frame(&m));
    assert_box(frames.last().unwrap());
    m.press(Button::X);
    frames.push(frame(&m));
    assert_box(frames.last().unwrap());
    assert_eq!(m.practice_cpu_pick(game).mode(), crate::model::edge_juggle::picker::Mode::Cpu(1));
    picker_back_out(&mut m, &mut frames);
    (m, frames)
}

#[test]
fn guard_the_edge_picker_gives_the_divider_back() {
    let (mut m, mut frames) = practice_picker(Drill::GuardEdge);
    open(&mut m, "Hitbox Visualization", &mut frames);
    assert!(frames.last().unwrap().popup);
    assert_eq!(div_after(&frames), table_rule());
}

#[test]
fn juggler_picker_gives_the_divider_back() {
    let (mut m, mut frames) = practice_picker(Drill::Juggler);
    open(&mut m, "Hitbox Visualization", &mut frames);
    assert!(frames.last().unwrap().popup);
    assert_eq!(div_after(&frames), table_rule());
}

#[test]
fn mash_out_picker_gives_the_divider_back() {
    let mut m = menu();
    let mut frames = Vec::new();
    let mut roster = picker_open(&mut m, Editor::MashoutPicker, RosterUse::Mashout, &mut frames);
    roster.apply(&mut m, cell("Mario"), Button::A);
    frames.push(frame(&m));
    assert_box(frames.last().unwrap());
    picker_back_out(&mut m, &mut frames);
    open(&mut m, "Hitbox Visualization", &mut frames);
    assert!(frames.last().unwrap().popup);
    assert_eq!(div_after(&frames), table_rule());
}

#[test]
fn recording_popup_after_a_picker_shows_the_divider() {
    let (mut m, mut frames) = practice_picker(Drill::GuardEdge);
    m.recording_mut().toggle_popup();
    frames.push(frame(&m));
    let last = frames.last().unwrap();
    assert!(last.popup && last.view == View::List);
    assert_eq!(div_after(&frames), table_rule());
}

#[test]
fn picker_popup_box_follows_the_popup_and_hides_without_it() {
    let mut m = menu();
    let mut frames = Vec::new();
    let mut roster = picker_open(&mut m, Editor::PracticeCpu(Drill::GuardEdge), RosterUse::PracticeCpu(Drill::GuardEdge), &mut frames);
    roster.apply(&mut m, cell("Mario"), Button::A);
    frames.push(frame(&m));
    let first = assert_box(frames.last().unwrap());
    m.press(Button::B);
    frames.push(frame(&m));
    roster.apply(&mut m, cell("Donkey Kong"), Button::A);
    frames.push(frame(&m));
    let second = assert_box(frames.last().unwrap());
    assert_ne!(first, second);
    assert_eq!(div_after(&frames).rect, second);
    picker_back_out(&mut m, &mut frames);
    assert!(!frames.last().unwrap().popup);
    assert!(!div_after(&frames).shown);
}

#[test]
fn the_view_writes_the_divider_pane_in_one_place() {
    let source = include_str!("../../runtime/ui/view.rs");
    let start = source.find("fn draw_conn_div").unwrap();
    let end = start + source[start + 3..].find("fn ").unwrap() + 3;
    let occurrences: Vec<_> = source.match_indices("IDX_CONN_DIV").map(|(i, _)| i).collect();
    assert_eq!(occurrences.iter().filter(|&&i| i < start).count(), 1);
    assert!(occurrences.iter().filter(|&&i| i >= start).all(|&i| i < end));
    assert!(occurrences.iter().any(|&i| i >= start));
}
}
