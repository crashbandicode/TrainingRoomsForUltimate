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

use nx_layout_toolbox::bflyt::{read_bflyt, write_bflyt, ClonePaneSpec, Color8, PaneEdit, BFLYT};
use nx_layout_toolbox::sarc::{read_arc, write_arc};
use nx_layout_toolbox::bntx::{read_bntx, write_bntx, TextureFormat, pipeline::{import_dds, import_png_file, ImportOptions}};
use nx_layout_toolbox::dds::Dds;

#[path = "../../../src/model/archive_recipe.rs"]
#[allow(dead_code)]
mod archive_recipe;
#[path = "../../../src/model/ui/panes.rs"]
#[allow(dead_code)]
mod panes;
mod pane_table;

use panes::{Align, Kind, Spec};

const BFLYT_MEMBER: &str = "blyt/info_training.bflyt";
/// A stock `pic1` over the plain white texture (`com_white16`), vertex colours white.
const RECT_TEMPLATE: &str = "pic_bg_mode_main_00";
/// A stock text box in the `nintendo64` font with a plain white material.
const TEXT_TEMPLATE: &str = "set_txt_help_00";
const TEXT_FONT: &str = "nintendo64";
const OWNED_TEXT_MATERIAL: &str = "tr_text_neutral";
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
    // Resolve the font by identity in this archive, never by Training's index.
    let text_font = if matches!(spec.kind, Kind::Text { .. }) {
        Some(u16::try_from(layout.fonts.iter().position(|name| name == TEXT_FONT)
            .ok_or("qualified text font missing")?).map_err(err("font index"))?)
    } else {
        None
    };
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
    if text_font.is_some() {
        layout.set_pane(name, &PaneEdit {
            scale_x: Some(1.0), scale_y: Some(1.0),
            bind_material: waiting.then(|| OWNED_TEXT_MATERIAL.into()),
            ..Default::default()
        }).map_err(err(name))?;
    }
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
            text.font_index = text_font.ok_or("text font missing")?;
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

/// The orb disc's side in pixels.
const ORB_DISC: u32 = 64;

/// The hitbox orbs' texture (RGBA8, white): a disc whose opacity rises from a translucent middle to a brighter rim and fades
/// over its last pixels, so a tinted pane reads as a sphere's outline; 4 x 4 samples per pixel for a smooth edge.
fn orb_disc() -> Vec<u8> {
    let alpha = |d: f32| -> f32 {
        if d >= 1.0 {
            return 0.0;
        }
        let rim = ((d - 0.70) / 0.30).max(0.0);
        let a = 0.30 + 0.58 * rim * rim;
        if d > 0.94 { a * (1.0 - d) / 0.06 } else { a }
    };
    let size = ORB_DISC as f32;
    let centre = (size - 1.0) / 2.0;
    let mut out = Vec::with_capacity((ORB_DISC * ORB_DISC * 4) as usize);
    for y in 0..ORB_DISC {
        for x in 0..ORB_DISC {
            let mut total = 0.0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let dx = x as f32 + (sx as f32 + 0.5) / 4.0 - 0.5 - centre;
                    let dy = y as f32 + (sy as f32 + 0.5) / 4.0 - 0.5 - centre;
                    total += alpha((dx * dx + dy * dy).sqrt() / (size / 2.0));
                }
            }
            out.extend_from_slice(&[255, 255, 255, (255.0 * total / 16.0).round() as u8]);
        }
    }
    out
}

fn add_waiting_text_material(layout: &mut BFLYT) -> Result<()> {
    if layout.materials.iter().any(|m| m.name == OWNED_TEXT_MATERIAL) {
        return Err("owned text material already exists".into());
    }
    // Preserve the complete stock text material, including undecoded render
    // state. Material::default erased the v8 word at +0x20 (0x08040200)
    // in UI20's invisible text. Only our clone's name and tint may change.
    let index = layout.find_pane("set_txt_status").and_then(|p| p.text.as_ref())
        .ok_or("waiting text template missing")?.material_index;
    let mut material = layout.materials.get(usize::from(index))
        .ok_or("waiting text material missing")?.clone();
    material.name = OWNED_TEXT_MATERIAL.into();
    material.black_color = Color8 { r: 0, g: 0, b: 0, a: 0 };
    material.white_color = Color8 { r: 252, g: 252, b: 252, a: 255 };
    layout.materials.push(material);
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
        add_waiting_text_material(&mut layout)?;
        // The waiting archive already has com_white16, a plain UV0 material and
        // the game's text font. Build our own flat panes from those stock templates;
        // no Training archive or upstream artwork is used; item textures are appended below.
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
    // The hitbox visualizer's own orbs: one disc texture drawn here (`orb_disc`, the plugin's own artwork, no picture file),
    // one material, every orb picture bound to it; the panes' vertex colours tint it.
    const ORB_TEXTURE: &str = "tr_orb_disc";
    let disc = Dds {
        format: TextureFormat::R8G8B8A8Unorm, width: ORB_DISC, height: ORB_DISC, depth: 1, mip_count: 1, array_count: 1,
        is_cube: false, data: orb_disc(),
    };
    import_dds(&mut bntx, ORB_TEXTURE, &disc, None).map_err(err("import orb disc"))?;
    let first_orb = panes::name_of(panes::IDX_ORB);
    let orb_template_index = layout.find_pane(first_orb.as_str()).and_then(|p| p.picture.as_ref())
        .ok_or("orb picture missing")?.material_index;
    let orb_template = layout.materials.get(usize::from(orb_template_index)).ok_or("orb template material missing")?.name.clone();
    layout.add_texture_ref(ORB_TEXTURE);
    layout.add_material_from_template(&orb_template, ORB_TEXTURE, Some(ORB_TEXTURE)).map_err(err("orb material"))?;
    for i in 0..panes::ORBS {
        let name = panes::name_of(panes::IDX_ORB + i);
        layout.set_pane(name.as_str(), &PaneEdit { bind_material: Some(ORB_TEXTURE.into()), ..Default::default() })
            .map_err(err("orb picture material"))?;
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
    // The pane table this recipe was built from (both layouts share it): committed as assets/panes-recipe.sha256 and
    // checked by the plugin's tests/pane_recipe_guard.rs, so a pane-table change without a rebuild fails the gates.
    let pane_table = pane_table::digest(&panes::specs());
    std::fs::write(format!("{out}/panes-recipe.sha256"), format!("{pane_table}\n")).map_err(err("write pane digest"))?;
    let report = serde_json::json!({
        "pane_table_sha256": pane_table,
        "builder": pane_table::BUILDER,
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
    #[ignore = "requires operator-owned stock arcs in UI_STOCK_FIXTURE"]
    fn owned_text_style_preserves_stock_and_resolves_reordered_fonts() {
        let dir = std::env::var("UI_STOCK_FIXTURE").expect("set UI_STOCK_FIXTURE");
        for waiting in [false, true] {
            let kind = if waiting { "waiting" } else { "training" };
            let stock = std::fs::read(format!("{dir}/info_{kind}.arc")).unwrap();
            let arc = read_arc(&stock).unwrap();
            let member = arc.position(&format!("blyt/info_{kind}.bflyt")).unwrap();
            let mut layout = read_bflyt(&arc.files[member].data).unwrap();
            // Original failing control: the real waiting clone inherits green
            // mapping and half-scale even though nominal font size is correct.
            if waiting {
                let template = layout.find_pane("set_txt_status").unwrap();
                assert_eq!((template.scale.x,template.scale.y),(0.5,0.52));
                let material = &layout.materials[usize::from(template.text.as_ref().unwrap().material_index)];
                assert_eq!((material.white_color.r,material.white_color.g,material.white_color.b),(0,184,3));
            }
            let before = format!("{:?}", layout.root_pane);
            let materials = format!("{:?}", layout.materials);
            let stock_count = layout.materials.len();
            if waiting { add_waiting_text_material(&mut layout).unwrap(); }
            assert_eq!(before,format!("{:?}",layout.root_pane));
            assert_eq!(materials,format!("{:?}",&layout.materials[..stock_count]));
            // Move the qualified font to another index in this isolated fixture.
            let font = layout.fonts.iter().position(|f| f == TEXT_FONT).unwrap();
            layout.fonts.swap(0,font);
            let mut spec = panes::specs().into_iter().find(|s| matches!(s.kind,Kind::Text { .. })).unwrap();
            spec.parent = panes::Name::new("RootPane");
            add_pane(&mut layout,&spec,waiting).unwrap();
            let pane = layout.find_pane(spec.name.as_str()).unwrap();
            assert_eq!((pane.scale.x,pane.scale.y),(1.0,1.0));
            let text = pane.text.as_ref().unwrap();
            assert_eq!(text.font_index,0);
            assert_eq!(layout.fonts[usize::from(text.font_index)],TEXT_FONT);
            let material = &layout.materials[usize::from(text.material_index)];
            assert_eq!((material.white_color.r,material.white_color.g,material.white_color.b),(252,252,252));
            if waiting { assert_eq!(material.name,OWNED_TEXT_MATERIAL); }
            if waiting {
                let template = layout.find_pane("set_txt_status").unwrap().text.as_ref().unwrap();
                let stock = &layout.materials[usize::from(template.material_index)];
                assert_eq!(stock.flags_unknown, 0x0804_0200, "actual UI20 stock render-state word");
                let mut expected = stock.clone();
                expected.name = OWNED_TEXT_MATERIAL.into();
                expected.black_color = Color8 { r: 0, g: 0, b: 0, a: 0 };
                expected.white_color = Color8 { r: 252, g: 252, b: 252, a: 255 };
                assert_eq!(format!("{material:?}"), format!("{expected:?}"),
                    "all non-tint render state must survive cloning");
            }
            assert_eq!(materials,format!("{:?}",&layout.materials[..stock_count]));
            layout.fonts.clear();
            assert_eq!(add_pane(&mut layout,&spec,waiting),Err("qualified text font missing".into()));
        }
    }

    /// `pane_table::BUILDER` names this builder's version and the pinned layout library revision.
    #[test]
    fn the_builder_identity_matches_the_manifest() {
        let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
        let rev = manifest.split("rev = \"").nth(1).and_then(|s| s.split('"').next()).expect("pinned toolbox rev");
        assert_eq!(pane_table::BUILDER, format!("layout-builder {}; nx-layout-toolbox {rev}", env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn the_sha256_matches_the_standard_vectors() {
        let hex = |d: [u8; 32]| d.iter().map(|b| format!("{b:02x}")).collect::<String>();
        assert_eq!(hex(pane_table::sha256(b"")), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(hex(pane_table::sha256(b"abc")), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        let two_blocks = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        assert_eq!(hex(pane_table::sha256(two_blocks)), "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");
    }

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
        assert_eq!(roots, 4, "menu, notices, input log and the hitbox orbs");
        for root in [panes::IDX_MENU, panes::IDX_NOTICE, panes::IDX_LOG, panes::IDX_ORBS] {
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
