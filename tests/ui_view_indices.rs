//! P0 (Surface run `tr-p23-surface-combo3`, `LANE-SURFACE-REPORT.md` "P0"): the cycle-badge port's roster marks
//! were indexed past the menu's resolved pane array (`view.rs`: `index out of bounds: the len is 311 but the index
//! is 763`, every menu open). The production renderer (`src/runtime/ui/view.rs`) runs here over a `Panes` of the
//! real length (`MENU_COUNT`, as `tick::resolve` builds it) with every pane absent, so an index past the array
//! panics in the test instead of on the console. Native structs use the same host shim as `ui_native_pause.rs`.
extern crate self as skyline;
// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
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
        pub fn layer(_: usize) -> Option<crate::model::effect_chips::LayerWord> { None }
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
        ("chip layers (glow, body, streaks, core)", p::IDX_CHIP_NEEDLE, p::CHIP_LAYERS * p::CHIPS),
        ("chip labels", p::IDX_CHIP_LABEL, p::CHIPS),
        ("chip badges", p::IDX_CHIP_BADGE, p::CHIPS),
        ("roster badges (Player, CPU)", p::IDX_TILE_BADGE, 2 * p::TILES),
        ("roster All-stripes (Player, CPU)", p::IDX_TILE_STRIPE, 2 * p::TILES),
        ("Solo Popo markers (Player, CPU)", p::IDX_TILE_SOLO, 2),
        ("recording floppies", p::IDX_FLOPPY, p::FLOPPIES),
        ("recording loops", p::IDX_LOOP, p::LOOPS),
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

/// rc.13 E1 x1: each chip has four layer panes (glow, body, streaks, core, back to front), one block of `CHIPS` per
/// layer, between the chip tiles and the labels; every one is a hidden rect at the chip block's place, as the one
/// pointer was.
#[test]
fn the_four_chip_layers_lie_between_the_tiles_and_the_labels() {
    assert_eq!(p::CHIP_LAYERS, 4);
    assert_eq!(p::IDX_CHIP_NEEDLE, p::IDX_CHIP + p::CHIPS);
    assert_eq!(p::IDX_CHIP_LABEL, p::IDX_CHIP_NEEDLE + p::CHIP_LAYERS * p::CHIPS);
    let first = p::spec(p::IDX_CHIP_NEEDLE);
    for index in p::IDX_CHIP_NEEDLE..p::IDX_CHIP_LABEL {
        let spec = p::spec(index);
        assert!(matches!(spec.kind, p::Kind::Rect { .. }) && !spec.visible, "{index}: {spec:?}");
        assert_eq!((spec.x, spec.y, spec.w, spec.h, spec.parent), (first.x, first.y, first.w, first.h, first.parent));
        assert!((0..p::COUNT).all(|other| other == index || p::name_of(other) != spec.name), "{index}: name taken twice");
    }
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

/// rc.13: every give-item tile has its picture (Wario's bike, tile 38, included): one icon pane per tile, and the
/// layout builder's `assets/item-icons/icon-NN.png` for each.
#[test]
fn every_item_tile_has_an_icon() {
    assert_eq!(p::ITEM_ICONS, model::menu::grid::ITEM_TILES);
    for i in 0..p::ITEM_ICONS {
        let png = format!("{}/assets/item-icons/icon-{i:02}.png", env!("CARGO_MANIFEST_DIR"));
        assert!(std::path::Path::new(&png).is_file(), "{png}");
    }
}

/// The zone editor's crop window (design 3) over shim panes of the real length: entering hides every menu pane, the
/// draw shows only its rectangles, and leaving puts every pane back to the table's starting visibility, also when the
/// layout was resolved again with the editor up and the window entered twice (review 2026-11-22 17:53:48Z).
#[test]
fn the_crop_window_leaves_every_menu_pane_as_the_table_starts_it() {
    use model::egg_zone::{CropRect, CROP_RECTS};
    use nn::ui2d::Picture;

    let mut pictures: Vec<Picture> = (0..p::MENU_COUNT).map(|_| unsafe { core::mem::zeroed() }).collect();
    let mut all = [core::ptr::null_mut(); p::MENU_COUNT];
    for (i, picture) in pictures.iter_mut().enumerate() {
        picture.pane.flags = u8::from(p::spec(i).visible);
        all[i] = &mut picture.pane;
    }
    let panes = Panes { container: all[0], all };
    let mut rects = [None; CROP_RECTS];
    rects[0] = Some(CropRect { x: 10.0, y: 20.0, w: 30.0, h: 40.0, color: [1, 2, 3, 4] });
    rects[5] = Some(CropRect { x: -50.0, y: 60.0, w: 70.0, h: 80.0, color: [5, 6, 7, 8] });

    unsafe { panes.crop_enter(); }
    for (i, picture) in pictures.iter().enumerate().skip(1) {
        assert_eq!(picture.pane.flags & 1, 0, "{} should be hidden on enter", p::name_of(i).as_str());
    }
    unsafe { panes.crop_draw(&rects) };
    for (i, picture) in pictures.iter().enumerate().skip(1) {
        let visible = i == p::IDX_TILE_STRIPE || i == p::IDX_TILE_STRIPE + 5;
        assert_eq!(picture.pane.flags & 1 != 0, visible, "{} visibility during crop", p::name_of(i).as_str());
    }
    for offset in [0, 5] {
        let rect = rects[offset].unwrap();
        let pane = &pictures[p::IDX_TILE_STRIPE + offset].pane;
        assert_eq!((pane.pos_x, pane.pos_y, pane.size_x, pane.size_y), (rect.x, rect.y, rect.w, rect.h));
    }
    unsafe { panes.crop_enter(); panes.crop_leave(); }
    for (i, picture) in pictures.iter().enumerate().skip(1) {
        assert_eq!(picture.pane.flags & 1 != 0, p::spec(i).visible, "{} visibility after leave", p::name_of(i).as_str());
    }
}

/// The crop window's state through a layout resolved again (review 2026-11-22 19:00:21Z): a registration with the
/// editor up, the editor ended, and the regular calculate resolving the same panes; the editor still up through the
/// second resolve; and a new layout's panes while the window is up.
#[test]
fn the_crop_window_is_put_back_when_the_layout_is_resolved_again() {
    use model::egg_zone::{CropRect, CROP_RECTS};
    use nn::ui2d::Picture;
    use view::Crop;

    fn fixture() -> (Vec<Picture>, Panes) {
        let mut pictures: Vec<Picture> = (0..p::MENU_COUNT).map(|_| unsafe { core::mem::zeroed() }).collect();
        let mut all = [core::ptr::null_mut(); p::MENU_COUNT];
        for (i, picture) in pictures.iter_mut().enumerate() {
            picture.pane.flags = u8::from(p::spec(i).visible);
            all[i] = &mut picture.pane;
        }
        (pictures, Panes { container: all[0], all })
    }
    fn check(pictures: &[Picture], table: bool, stage: &str) {
        for (i, picture) in pictures.iter().enumerate().skip(1) {
            assert_eq!(picture.pane.flags & 1 != 0, table && p::spec(i).visible,
                "{} visibility {stage}", p::name_of(i).as_str());
        }
    }
    let (pictures, a) = fixture();
    let b = Panes { container: a.container, all: a.all };
    let mut rects = [None; CROP_RECTS];
    rects[0] = Some(CropRect { x: 10.0, y: 20.0, w: 30.0, h: 40.0, color: [1, 2, 3, 4] });
    rects[5] = Some(CropRect { x: -50.0, y: 60.0, w: 70.0, h: 80.0, color: [5, 6, 7, 8] });
    let mut crop = Crop::new();
    unsafe { crop.resolved(Some(&a)); crop.pass(&a, true, Some(&rects)); }
    for (i, picture) in pictures.iter().enumerate().skip(1) {
        let visible = i == p::IDX_TILE_STRIPE || i == p::IDX_TILE_STRIPE + 5;
        assert_eq!(picture.pane.flags & 1 != 0, visible, "{} visibility during crop", p::name_of(i).as_str());
    }
    unsafe { crop.resolved(Some(&b)); crop.pass(&b, false, None); }
    check(&pictures, true, "after the editor ended and the layout was resolved again");

    let mut crop = Crop::new();
    unsafe {
        crop.resolved(Some(&a));
        crop.pass(&a, true, Some(&rects));
        crop.resolved(Some(&b));
        crop.pass(&b, true, None);
    }
    check(&pictures, false, "after resolving again while editing");
    unsafe { crop.pass(&b, false, None) };
    check(&pictures, true, "after leaving");

    unsafe { crop.pass(&a, true, Some(&rects)) };
    let (other_pictures, c) = fixture();
    unsafe { crop.resolved(Some(&c)) };
    check(&other_pictures, true, "on a newly resolved layout");
    unsafe { crop.pass(&c, true, None) };
    check(&other_pictures, false, "after entering on the new layout");
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

// The pane traversal uses the same host types as the production renderer probe above.
#[path = "../src/test_table.rs"]
mod test_table;
mod ui_pane_boundary;

// The byte meter needs the System allocator for unrelated test allocations.
mod hook_heap_meter;
