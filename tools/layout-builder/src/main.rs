//! Host tool: builds the plugin's `info_training` layout archive from the player's own stock archive
//! (SPEC-core-05 §4.2, SPEC-core-07 §4.2, question 2). Usage:
//!
//!   layout-builder build <stock info_training.arc> <out dir>
//!       writes `<out>/target.arc` (the plugin's archive, game bytes inside: keep it private),
//!       `<out>/info_training.tra1` (the recipe the plugin embeds: copy ranges and the changed bytes only)
//!       and `<out>/report.json`.
//!   layout-builder dump <bflyt> <pane>...   prints the named panes (development).
//!
//! The pane inventory is `src/model/ui/panes.rs`, shared with the runtime. The layout is flat: one container
//! under `RootPane` with text boxes cloned from a stock text box and flat rectangles cloned from a stock solid
//! picture, so no texture is added and no stock texture changes.

use nx_layout_toolbox::bflyt::{read_bflyt, write_bflyt, ClonePaneSpec, PaneEdit, BFLYT};
use nx_layout_toolbox::sarc::{read_arc, write_arc};

#[path = "../../../src/model/archive_recipe.rs"]
#[allow(dead_code)]
mod archive_recipe;
#[path = "../../../src/model/ui/panes.rs"]
#[allow(dead_code)]
mod panes;

use panes::{Align, Kind, Spec};

const BFLYT_MEMBER: &str = "blyt/info_training.bflyt";
/// A stock `pic1` over the plain white texture (`com_white16`), vertex colours white.
const RECT_TEMPLATE: &str = "pic_bg_mode_main_00";
/// A stock text box in the `nintendo64` font with a plain white material.
const TEXT_TEMPLATE: &str = "set_txt_help_00";
const INFLUENCED_ALPHA: u8 = 0x02;

type Result<T> = std::result::Result<T, String>;

fn err<E: std::fmt::Debug>(what: &str) -> impl FnOnce(E) -> String + '_ {
    move |e| format!("{what}: {e:?}")
}

fn add_container(layout: &mut BFLYT) -> Result<()> {
    layout
        .clone_pane(&ClonePaneSpec {
            template: "RootPane".into(),
            new_name: panes::ROOT.as_str().into(),
            parent: Some("RootPane".into()),
            visible: Some(false),
            ..Default::default()
        })
        .map_err(err("container"))?;
    let root = layout.find_pane_mut(panes::ROOT.as_str()).ok_or("container missing")?;
    root.flag |= INFLUENCED_ALPHA;
    Ok(())
}

fn add_pane(layout: &mut BFLYT, spec: &Spec) -> Result<()> {
    let template = match spec.kind {
        Kind::Rect { .. } => RECT_TEMPLATE,
        Kind::Text { .. } => TEXT_TEMPLATE,
    };
    let name = spec.name.as_str();
    layout
        .clone_pane(&ClonePaneSpec {
            template: template.into(),
            new_name: name.into(),
            parent: Some(panes::ROOT.as_str().into()),
            translate_x: Some(spec.x),
            translate_y: Some(spec.y),
            translate_z: Some(0.0),
            width: Some(spec.w),
            height: Some(spec.h),
            alpha: Some(255),
            visible: Some(spec.visible),
            ..Default::default()
        })
        .map_err(err(name))?;
    layout.set_pane(name, &PaneEdit::default()).map_err(err(name))?;
    let pane = layout.find_pane_mut(name).ok_or("pane missing")?;
    pane.flag |= INFLUENCED_ALPHA;
    match spec.kind {
        Kind::Rect { color } => {
            let picture = pane.picture.as_mut().ok_or("not a picture")?;
            for vertex in &mut picture.vertex_colors {
                vertex.r = color[0];
                vertex.g = color[1];
                vertex.b = color[2];
                vertex.a = color[3];
            }
        }
        Kind::Text { align, font, chars, color } => {
            let text = pane.text.as_mut().ok_or("not a text box")?;
            let (position, alignment) = match align {
                // Origin: horizontal * 3 + vertical (centre, left, right; centre, top, bottom); lines align alike.
                Align::Left => (3, 1),
                Align::Center => (0, 2),
                Align::Right => (6, 3),
            };
            text.text_position = position;
            text.text_alignment = alignment;
            text.font_size.x = font.0;
            text.font_size.y = font.1;
            for c in &mut text.text_cols {
                c.r = color[0];
                c.g = color[1];
                c.b = color[2];
                c.a = color[3];
            }
            // The string buffer (`chars` UTF-16 units, empty), then the text id the stock box carries.
            let buffer = usize::from(chars) * 2;
            let padded = buffer.next_multiple_of(4);
            text.text_buf_bytes = buffer as u16;
            text.text_str_bytes = 2;
            text.text_id_offset = text.text_str_offset + padded as u32;
            let mut trailing = vec![0u8; padded];
            trailing.extend_from_slice(&[b'@', 0, 0, 0]);
            text.trailing = trailing;
        }
    }
    Ok(())
}

fn build(stock_path: &str, out: &str) -> Result<()> {
    let stock = std::fs::read(stock_path).map_err(err("read stock"))?;
    let mut arc = read_arc(&stock).map_err(err("read arc"))?;
    // The writer must keep what it was given: an untouched archive round-trips to the same bytes.
    let untouched = write_arc(&arc).map_err(err("write untouched"))?;
    let index = arc.position(BFLYT_MEMBER).ok_or("info_training.bflyt not in the archive")?;
    let mut layout = read_bflyt(&arc.files[index].data).map_err(err("read bflyt"))?;
    add_container(&mut layout)?;
    let specs = panes::specs();
    for spec in &specs {
        add_pane(&mut layout, spec)?;
    }
    arc.files[index].data = write_bflyt(&layout).map_err(err("write bflyt"))?;
    let target = write_arc(&arc).map_err(err("write arc"))?;
    let recipe = archive_recipe::make(&stock, &target);
    let rebuilt = archive_recipe::build(&stock, &recipe).map_err(|e| e.to_string())?;
    if rebuilt != target {
        return Err("the recipe does not rebuild the target".into());
    }
    std::fs::create_dir_all(out).map_err(err("out dir"))?;
    std::fs::write(format!("{out}/target.arc"), &target).map_err(err("write target"))?;
    std::fs::write(format!("{out}/info_training.tra1"), &recipe).map_err(err("write recipe"))?;
    let report = serde_json::json!({
        "stock_bytes": stock.len(),
        "stock_fnv": format!("{:016x}", archive_recipe::fingerprint(&stock)),
        "untouched_roundtrip_identical": untouched == stock,
        "untouched_roundtrip_bytes": untouched.len(),
        "target_bytes": target.len(),
        "target_fnv": format!("{:016x}", archive_recipe::fingerprint(&target)),
        "recipe_bytes": recipe.len(),
        "panes_added": specs.len() + 1,
    });
    std::fs::write(format!("{out}/report.json"), serde_json::to_string_pretty(&report).unwrap() + "\n")
        .map_err(err("write report"))?;
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let result = match args.get(1).map(String::as_str) {
        Some("build") if args.len() == 4 => build(&args[2], &args[3]),
        Some("dump") if args.len() >= 4 => {
            let layout = read_bflyt(&std::fs::read(&args[2]).expect("read bflyt")).expect("parse");
            for name in &args[3..] {
                println!("{name}: {:#?}", layout.find_pane(name));
            }
            Ok(())
        }
        _ => Err("usage: layout-builder build <stock.arc> <out dir> | dump <bflyt> <pane>...".into()),
    };
    if let Err(message) = result {
        eprintln!("layout-builder: {message}");
        std::process::exit(1);
    }
}