//! P0 (Surface run `tr-p23-surface-combo3`, `LANE-SURFACE-REPORT.md` "P0"): the cycle-badge port's roster marks
//! were indexed past the menu's resolved pane array (`view.rs`: `index out of bounds: the len is 311 but the index
//! is 763`, every menu open). The production renderer (`src/runtime/ui/view.rs`) runs here over a `Panes` of the
//! real length (`MENU_COUNT`, as `tick::resolve` builds it) with every pane absent, so an index past the array
//! panics in the test instead of on the console. Native structs use the same host shim as `ui_native_pause.rs`.
extern crate self as skyline;
#[path = "../src/model.rs"]
#[allow(dead_code)]
pub mod model;
pub mod nn { pub mod ui2d {
    #[repr(C)] pub struct PaneNode { pub prev: *mut PaneNode, pub next: *mut PaneNode }
    #[repr(C)] pub struct Pane {
        pub link: PaneNode, pub children_list: PaneNode, pub name: [u8; 25],
        pub alpha: u8, pub flags: u8, pub pos_x: f32, pub pos_y: f32,
        pub size_x: f32, pub size_y: f32, pub rot_z: f32,
    }
    impl Pane { pub fn set_visible(&mut self, on: bool) { self.flags = u8::from(on); } }
    pub enum PaneFlag { IsGlobalMatrixDirty = 1 }
    #[repr(C)] pub struct Picture { pub pane: Pane, pub vertex_colors: [[u8; 4]; 4] }
    #[repr(C)] pub struct TextBox {
        pub pane: Pane, pub text_buf_len: u16, pub text_len: u16, pub text_buf: *mut u16,
        pub bits: u32, pub text_position: u32,
    }
    impl TextBox { pub fn set_color(&mut self, _: u8, _: u8, _: u8, _: u8) {} }
} }
#[path = "../src/runtime/ui/pane.rs"]
#[allow(dead_code)]
mod pane;
#[path = "../src/runtime/ui/view.rs"]
mod view;
mod runtime { pub mod ui {
    pub(crate) use crate::{pane, view};
    pub mod chips {
        pub const NEUTRAL: usize = 8;
        pub fn layer() -> Option<crate::model::effect_chips::LayerWord> { None }
    }
    pub mod thumbs { pub fn is_bound(_: usize) -> bool { false } }
} }

use model::frame::{Frame, View};
use model::panes as p;
use runtime::ui::view::Panes;

fn absent() -> Panes {
    Panes { container: core::ptr::null_mut(), all: [core::ptr::null_mut(); p::MENU_COUNT] }
}

/// Every index range the renderer writes, by its constants, lies inside the resolved array; the roster grid's
/// per-role marks (badges, All-stripes, Solo Popo) included. The notification panel and the input log are the
/// input package's panes (found by name when it draws), after the menu's block.
#[test]
fn every_index_the_renderer_can_write_is_inside_the_resolved_array() {
    let ranges = [
        ("tiles", p::IDX_TILE, p::TILES),
        ("tile labels", p::IDX_TILE_LABEL, p::TILES),
        ("item icons", p::IDX_ITEM_ICON, p::ITEM_ICONS),
        ("chips", p::IDX_CHIP, p::CHIPS),
        ("chip needles", p::IDX_CHIP_NEEDLE, p::CHIPS),
        ("chip labels", p::IDX_CHIP_LABEL, p::CHIPS),
        ("chip badges", p::IDX_CHIP_BADGE, p::CHIPS),
        ("roster badges (Player, CPU)", p::IDX_TILE_BADGE, 2 * p::TILES),
        ("roster All-stripes (Player, CPU)", p::IDX_TILE_STRIPE, 2 * p::TILES),
        ("Solo Popo markers (Player, CPU)", p::IDX_TILE_SOLO, 2),
    ];
    let panes = absent();
    assert!(panes.container.is_null());
    let all = panes.all;
    for (what, first, n) in ranges {
        assert!(first + n <= all.len(), "{what}: {first}..{} past the resolved array of {}", first + n, all.len());
    }
    for single in [p::IDX_TILE_HI, p::IDX_CHIP_HI, p::IDX_TAB_HI, p::IDX_MENU] {
        assert!(single < all.len());
    }
    const { assert!(p::IDX_NOTICE >= p::MENU_COUNT && p::IDX_IV >= p::MENU_COUNT, "the input package's panes stay outside") };
}

/// The renderer itself, over a `Panes` of the real length: the hide-all path (no roster grid), the roster grid
/// with both roles' badges, stripes and Solo Popo markers on every tile, and the other views.
#[test]
fn the_renderer_runs_every_view_over_the_real_pane_array() {
    let panes = absent();
    let mut frame = Frame::new();
    unsafe { panes.draw(&frame) }; // hide-all: every roster mark hidden
    frame.view = View::Tiles;
    frame.tile_roster_grid = true;
    frame.tile_count = p::TILES as u8;
    for role in 0..2 {
        frame.tile_stripes[role] = [true; p::TILES];
        frame.tile_solo[role] = true;
        for badge in frame.tile_badges[role].iter_mut() {
            let _ = core::fmt::Write::write_str(badge, "12");
        }
    }
    frame.tile_n[3] = 0x4b; // the Ice Climbers cell carries the Solo Popo markers
    unsafe { panes.draw(&frame) };
    frame.tile_cpu = true;
    unsafe { panes.draw(&frame) };
    for view in [View::List, View::Delay, View::Overrides, View::Clip] {
        let mut other = Frame::new();
        other.view = view;
        unsafe { panes.draw(&other) };
    }
    frame.tile_roster_grid = false;
    frame.tile_stage_grid = true;
    unsafe { panes.draw(&frame) };
}

/// SOLO-S-PLACEMENT (operator 2026-11-05): on every layout the roster grid uses, the Player and CPU "S" marks never
/// intersect the tile's name label. With the portrait drawn they sit on its lower corners (covering it partly), clear
/// of the order badges above; without it the centred name stays clear too.
#[test]
fn the_solo_marks_never_cover_the_tile_name() {
    use model::menu::grid::{ConsoleMode, GridKind, GridLayout, RosterUse};
    for mode in [ConsoleMode::Docked, ConsoleMode::Handheld] {
        for use_ in [RosterUse::Cycles, RosterUse::Hats, RosterUse::PlayerPick, RosterUse::CpuPick] {
            let layout = GridLayout::new(GridKind::Roster(use_), mode);
            let cell_w = (p::ROW_RIGHT - p::ROW_LEFT) / f32::from(layout.cols);
            let cell_h = p::TILE_AREA_H / f32::from(layout.rows);
            for pictured in [true, false] {
                let (icon, label, marks) = p::roster_tile(cell_w, cell_h, pictured);
                for mark in marks {
                    assert!(!mark.overlaps(label), "{mode:?} {use_:?} pictured={pictured}: {mark:?} covers the name {label:?}");
                    if pictured {
                        assert!(mark.overlaps(icon), "the mark sits on the portrait's corner: {mark:?} {icon:?}");
                        let badge = p::Area { x: mark.x, y: 35.0, w: 52.0, h: p::MARK_H };
                        assert!(!mark.overlaps(badge), "clear of the order badge above");
                    }
                }
                assert!(!marks[0].overlaps(marks[1]), "the Player and CPU marks stay apart");
            }
        }
    }
}
