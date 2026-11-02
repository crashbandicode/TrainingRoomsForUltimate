//! Host tool: builds the plugin's `info_training` layout archive from the player's own stock archive
//! (SPEC-core-05 §4.2, SPEC-core-07 §4.2, question 2). Usage:
//!
//!   layout-builder build <stock info_training.arc> <out dir>
//!       writes `<out>/target.arc` (the plugin's archive, game bytes inside: keep it private),
//!       `<out>/info_training.tra1` (the recipe the plugin embeds: copy ranges and the changed bytes only)
//!       and `<out>/report.json`.
//!   layout-builder build-waiting <stock info_waiting.arc> <out dir>
//!       adds the same panes beneath `TrModPort` and writes `info_waiting.tra1`.
//!   layout-builder dump <bflyt> <pane>...   prints the named panes (development).
//!
//! The pane inventory is `src/model/ui/panes.rs`, shared with the runtime. The layout is flat: one container
//! under `RootPane` with text boxes cloned from a stock text box and flat rectangles cloned from a stock solid
//! picture. The 38 provenance-recorded item icons are appended to the stock BNTX; stock textures are retained.

use nx_layout_toolbox::bflyt::{read_bflyt, write_bflyt, ClonePaneSpec, PaneEdit, BFLYT};
use nx_layout_toolbox::sarc::{read_arc, write_arc};
use nx_layout_toolbox::bntx::{read_bntx, write_bntx, pipeline::{import_png_file, ImportOptions}};

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
const WAITING_PORT: panes::Name = panes::Name::new("TrModPort");
const INFLUENCED_ALPHA: u8 = 0x02;
/// 	xt1 flag bit 1: the buffer is 	ext_buf_bytes long even when the string in it is shorter.
const FORCE_ASSIGN_TEXT_LENGTH: u16 = 0x0002;

type Result<T> = std::result::Result<T, String>;

fn err<E: std::fmt::Debug>(what: &str) -> impl FnOnce(E) -> String + '_ {
    move |e| format!("{what}: {e:?}")
}

fn layout_specs(waiting: bool) -> Vec<Spec> {
    let mut specs = panes::specs();
    if waiting {
        // All three panel roots belong to the waiting host: menu, notices and
        // input log. Keep their internal parent relationships unchanged.
        for spec in &mut specs {
            if spec.parent == panes::Name::new("RootPane") {
                spec.parent = WAITING_PORT;
            }
        }
    }
    specs
}

fn add_pane(layout: &mut BFLYT, spec: &Spec, waiting: bool) -> Result<()> {
    let template = match spec.kind {
        Kind::Null => "RootPane",
        Kind::Rect { .. } => if waiting { "pic_bg_main_00" } else { RECT_TEMPLATE },
        Kind::Text { .. } => if waiting { "set_txt_status" } else { TEXT_TEMPLATE },
    };
    let name = spec.name.as_str();
    layout
        .clone_pane(&ClonePaneSpec {
            template: template.into(),
            new_name: name.into(),
            parent: Some(spec.parent.as_str().into()),
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
        Kind::Null => {}
        Kind::Rect { color } => {
            let picture = pane.picture.as_mut().ok_or("not a picture")?;
            let color = panes::linear(color);
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
            let color = panes::linear(color);
            // The string buffer keeps the length the layout gives it, not the length of the empty string it starts with.
            text.text_box_flag |= FORCE_ASSIGN_TEXT_LENGTH;
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
            // The string is declared as long as the buffer (all NULs, so it reads as empty): the game sizes the live
            // buffer from the string it was given.
            text.text_str_bytes = buffer as u16;
            // No text id: with one the game takes the live text from its message tables and sizes the buffer
            // from that (empty) result.
            text.text_id_offset = 0;
            text.trailing = vec![0u8; padded];
        }
    }
    Ok(())
}

fn build(stock_path: &str, out: &str, waiting: bool) -> Result<()> {
    let stock = std::fs::read(stock_path).map_err(err("read stock"))?;
    let mut arc = read_arc(&stock).map_err(err("read arc"))?;
    // The writer must keep what it was given: an untouched archive round-trips to the same bytes.
    let untouched = write_arc(&arc).map_err(err("write untouched"))?;
    let member = if waiting { "blyt/info_waiting.bflyt" } else { BFLYT_MEMBER };
    let recipe_name = if waiting { "info_waiting" } else { "info_training" };
    let index = arc.position(member).ok_or("requested layout not in the archive")?;
    let mut layout = read_bflyt(&arc.files[index].data).map_err(err("read bflyt"))?;
    let specs = layout_specs(waiting);
    if waiting {
        // The waiting archive already has com_white16, a plain UV0 material and
        // the game's text font. Build our own flat panes from those stock templates;
        // no Training archive, imported artwork or BNTX rewrite is needed.
        layout.clone_pane(&ClonePaneSpec {
            template: "RootPane".into(), new_name: "TrModPort".into(),
            parent: Some("RootPane".into()), visible: Some(true),
            ..Default::default()
        }).map_err(err("waiting port"))?;
    }
    for spec in &specs {
        add_pane(&mut layout, spec, waiting)?;
    }
    // These are the approved owned-game item renders, not baked fighter/stage portraits.
    // Keep the stock archive private; the distributed recipe contains only the delta.
    let textures = arc.position("timg/__Combined.bntx").ok_or("stock combined texture missing")?;
    let mut bntx = read_bntx(&arc.files[textures].data).map_err(err("read stock textures"))?;
    let options = ImportOptions { srgb: true, ..Default::default() };
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/item-icons");
    for i in 0..panes::ITEM_ICONS {
        let name = panes::name_of(panes::IDX_ITEM_ICON + i);
        let texture = format!("tr_item_{i:02}");
        import_png_file(&mut bntx, &texture, &assets.join(format!("icon-{i:02}.png")), &options)
            .map_err(err("import owned item icon"))?;
        let material_index = layout.find_pane(name.as_str()).and_then(|p| p.picture.as_ref())
            .ok_or("item picture missing")?.material_index;
        let template = layout.materials.get(usize::from(material_index)).ok_or("item template material missing")?.name.clone();
        layout.add_texture_ref(&texture);
        layout.add_material_from_template(&template, &texture, Some(&texture)).map_err(err("item material"))?;
        layout.set_pane(name.as_str(), &PaneEdit { bind_material: Some(texture), ..Default::default() })
            .map_err(err("item picture material"))?;
    }
    arc.files[textures].data = write_bntx(&bntx).map_err(err("write item textures"))?;
    arc.files[index].data = write_bflyt(&layout).map_err(err("write bflyt"))?;
    let target = write_arc(&arc).map_err(err("write arc"))?;
    let recipe = archive_recipe::make(&stock, &target);
    let rebuilt = archive_recipe::build(&stock, &recipe).map_err(|e| e.to_string())?;
    if rebuilt != target {
        return Err("the recipe does not rebuild the target".into());
    }
    std::fs::create_dir_all(out).map_err(err("out dir"))?;
    std::fs::write(format!("{out}/target.arc"), &target).map_err(err("write target"))?;
    std::fs::write(format!("{out}/{recipe_name}.tra1"), &recipe).map_err(err("write recipe"))?;
    let report = serde_json::json!({
        "stock_bytes": stock.len(),
        "stock_fnv": format!("{:016x}", archive_recipe::fingerprint(&stock)),
        "untouched_roundtrip_identical": untouched == stock,
        "untouched_roundtrip_bytes": untouched.len(),
        "target_bytes": target.len(),
        "target_fnv": format!("{:016x}", archive_recipe::fingerprint(&target)),
        "recipe_bytes": recipe.len(),
        "panes_added": specs.len() + usize::from(waiting),
        "item_icons": panes::ITEM_ICONS,
    });
    std::fs::write(format!("{out}/report.json"), serde_json::to_string_pretty(&report).unwrap() + "\n")
        .map_err(err("write report"))?;
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waiting_host_contains_every_panel_without_changing_internal_parents() {
        let training = layout_specs(false);
        let waiting = layout_specs(true);
        assert_eq!(training.len(), waiting.len());
        let mut roots = 0;
        for (a, b) in training.iter().zip(&waiting) {
            assert_eq!(a.name, b.name);
            if a.parent == panes::Name::new("RootPane") {
                assert_eq!(b.parent, WAITING_PORT);
                roots += 1;
            } else {
                assert_eq!(a.parent, b.parent);
            }
        }
        assert_eq!(roots, 3, "menu, notices and input log");
        for root in [panes::IDX_MENU, panes::IDX_NOTICE, panes::IDX_LOG] {
            assert_eq!(waiting[root].parent, WAITING_PORT);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let result = match args.get(1).map(String::as_str) {
        Some("build") if args.len() == 4 => build(&args[2], &args[3], false),
        Some("build-waiting") if args.len() == 4 => build(&args[2], &args[3], true),
        Some("dump") if args.len() >= 4 => {
            let layout = read_bflyt(&std::fs::read(&args[2]).expect("read bflyt")).expect("parse");
            for name in &args[3..] {
                println!("{name}: {:#?}", layout.find_pane(name));
            }
            Ok(())
        }
        _ => Err("usage: layout-builder build|build-waiting <stock.arc> <out dir> | dump <bflyt> <pane>...".into()),
    };
    if let Err(message) = result {
        eprintln!("layout-builder: {message}");
        std::process::exit(1);
    }
}
