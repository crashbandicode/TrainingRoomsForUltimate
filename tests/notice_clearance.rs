//! Notice slot versus the frame strip. Geometry comes from `panes::specs()` (back-to-front draw order).

#[path = "../src/model/ui/panes.rs"]
#[allow(dead_code)]
mod panes;
#[path = "../tools/layout-builder/src/pane_table.rs"]
mod pane_table;

use panes::{Kind, Name, Spec, IDX_IV, IDX_NOTICE, IDX_ORBS};

/// Plate padding `p23/notice-plates` adds around each notice text. This branch's boxes must leave room for it.
const PLATE_PAD: f32 = 16.0;
const GAP: f32 = 8.0;

fn world(specs: &[Spec], index: usize) -> (f32, f32) {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut at = index;
    for _ in 0..8 {
        x += specs[at].x;
        y += specs[at].y;
        let parent = specs[at].parent;
        if parent.as_str() == "RootPane" || parent.as_str() == "TrModPort" {
            break;
        }
        match specs.iter().position(|spec| spec.name == parent) {
            Some(next) => at = next,
            None => break,
        }
    }
    (x, y)
}

fn pixel(kind: &Kind) -> bool {
    matches!(kind, Kind::Rect { .. } | Kind::Text { .. })
}

fn pad_of(kind: &Kind) -> f32 {
    if matches!(kind, Kind::Text { .. }) { PLATE_PAD } else { 0.0 }
}

fn separated(notice: &Spec, nx: f32, ny: f32, other: &Spec, ox: f32, oy: f32) -> bool {
    let pad = pad_of(&notice.kind);
    let n_left = nx - notice.w / 2.0 - pad;
    let n_right = nx + notice.w / 2.0 + pad;
    let n_bottom = ny - notice.h / 2.0 - pad;
    let n_top = ny + notice.h / 2.0 + pad;
    let o_left = ox - other.w / 2.0;
    let o_right = ox + other.w / 2.0;
    let o_bottom = oy - other.h / 2.0;
    let o_top = oy + other.h / 2.0;
    n_right + GAP <= o_left || o_right + GAP <= n_left || n_top + GAP <= o_bottom || o_top + GAP <= n_bottom
}

fn notice_indexes(specs: &[Spec]) -> Vec<usize> {
    let root = Name::new("TrModDisp0");
    specs
        .iter()
        .enumerate()
        .filter(|(index, spec)| *index != IDX_NOTICE && pixel(&spec.kind) && (spec.name == root || spec.parent == root || ancestor_is_notice(specs, *index)))
        .map(|(index, _)| index)
        .collect()
}

fn ancestor_is_notice(specs: &[Spec], index: usize) -> bool {
    let root = Name::new("TrModDisp0");
    let mut at = index;
    for _ in 0..8 {
        if specs[at].name == root {
            return true;
        }
        let parent = specs[at].parent;
        match specs.iter().position(|spec| spec.name == parent) {
            Some(next) => at = next,
            None => return false,
        }
    }
    false
}

fn strip_indexes(specs: &[Spec]) -> Vec<usize> {
    (IDX_IV..IDX_ORBS).filter(|&index| pixel(&specs[index].kind)).collect()
}

fn assert_clear(specs: &[Spec], label: &str) {
    let notices = notice_indexes(specs);
    let strip = strip_indexes(specs);
    assert!(!notices.is_empty() && !strip.is_empty(), "{label}");
    for &notice in &notices {
        let (nx, ny) = world(specs, notice);
        for &other in &strip {
            let (ox, oy) = world(specs, other);
            assert!(
                separated(&specs[notice], nx, ny, &specs[other], ox, oy),
                "{label}: {:?} is covered by later or overlapping {:?}",
                specs[notice].name,
                specs[other].name
            );
            if other > notice {
                assert!(
                    separated(&specs[notice], nx, ny, &specs[other], ox, oy),
                    "{label}: {:?} draws over {:?}",
                    specs[other].name,
                    specs[notice].name
                );
            }
        }
        let top = ny + specs[notice].h / 2.0 + pad_of(&specs[notice].kind);
        assert!(top <= 540.0 && ny - specs[notice].h / 2.0 - pad_of(&specs[notice].kind) >= -540.0, "{label}: {:?} leaves the screen", specs[notice].name);
    }
}

fn waiting(specs: &[Spec]) -> Vec<Spec> {
    let port = Name::new("TrModPort");
    let orbs = panes::name_of(panes::IDX_ORBS);
    let root = Name::new("RootPane");
    specs
        .iter()
        .cloned()
        .map(|mut spec| {
            if spec.parent == root && spec.name != orbs {
                spec.parent = port;
            }
            spec
        })
        .collect()
}

#[test]
fn no_notice_pane_is_covered_by_the_strip() {
    let specs = panes::specs();
    assert_clear(&specs, "training");
    for &notice in &notice_indexes(&specs) {
        assert!(strip_indexes(&specs).iter().any(|&other| other > notice), "the strip still draws after the notice");
    }
}

#[test]
fn the_strip_keeps_its_panes_when_a_notice_shows() {
    let specs = panes::specs();
    let container = &specs[IDX_IV];
    assert_eq!(container.name.as_str(), "TrIv");
    assert_eq!((container.x, container.y, container.w, container.h), (0.0, panes::iv::STRIP_Y, panes::iv::STRIP_W, panes::iv::STRIP_H));
    assert_eq!(container.parent.as_str(), "RootPane");
    let backdrop = &specs[IDX_IV + panes::iv::at::BACKDROP];
    assert!(matches!(backdrop.kind, Kind::Rect { .. }) && backdrop.visible);
    let bar = &specs[panes::IDX_CB];
    assert_eq!(bar.parent.as_str(), "TrIv");
    assert_eq!(bar.y, panes::cb::BAR_Y);
    assert_clear(&specs, "strip content");
}

#[test]
fn both_layouts_keep_notices_clear_of_the_strip() {
    let training = panes::specs();
    assert_clear(&training, "training");
    let waiting_specs = waiting(&training);
    assert_eq!(waiting_specs[IDX_NOTICE].parent.as_str(), "TrModPort");
    assert_eq!(waiting_specs[IDX_IV].parent.as_str(), "TrModPort");
    assert_eq!(waiting_specs[IDX_NOTICE + 1].parent.as_str(), "TrModDisp0");
    assert_clear(&waiting_specs, "waiting");
    let committed = std::fs::read_to_string(format!("{}/assets/panes-recipe.sha256", env!("CARGO_MANIFEST_DIR")))
        .unwrap()
        .trim()
        .to_owned();
    assert_eq!(pane_table::digest(&training), committed, "rebuild both layout recipes from this pane table");
    for recipe in ["assets/info_training.tra1", "assets/info_waiting.tra1"] {
        let path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), recipe);
        let bytes = std::fs::read(&path).unwrap_or_else(|err| panic!("{path}: {err}"));
        for name in ["TrModDisp0", "TrModDisp0Header", "TrModDisp0Txt", "TrIv"] {
            assert!(bytes.windows(name.len()).any(|window| window == name.as_bytes()), "{recipe} is missing {name}");
        }
    }
}
