//! The two spec-named core-07 host checks (SPEC-core-07 section 9, rows N10 and Q16): the SDI chip's
//! streak length (N10, x2.0) and the absence of baked or committed fighter portraits (Q16, as amended
//! for RC10: the grids draw the game's own stock icon at run time, nothing baked or committed).

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use std::path::Path;

/// `core07_sdi_chip_streak_length` (SPEC-core-07 :223, N10): the chip still's streak length is above
/// the first-render value and within a bound of the captured spark's, measured on the host render of
/// the stock effect file. The render is built outside the repo (W9's `di-sdi-extract`); the repo keeps
/// byte copies of the two Up layers under `docs/menu-mockup/captures/renders/`, pinned by sha256 in
/// `docs/menu-mockup/index.md`. The measurement itself is the committed script
/// `tools/sdi_streak_measure.py` (method and recorded numbers in its docstring: the first render's
/// 58 x 86 burst, the drawn still's x2.0 stretch, the drawn tip 0.39 of the chip against the captured
/// spark's recorded Up tip 0.36, the kept width, and the unchanged DI render), so a reviewer re-runs it.
#[test]
fn core07_sdi_chip_streak_length() {
    // The model draws the SDI layer stretched x2.0 along the vector and keeps the DI proportions.
    assert_eq!(model::effect_chips::SDI_STRETCH, 2.0, "N10: the decided stretch is x2.0");
    let sdi = model::effect_chips::place(model::frame::ChipVisual::Sdi, 90.0, 0.0, 0.0, 184.0, 256, 128);
    assert!(
        (sdi.h / (sdi.w * 128.0 / 256.0) - model::effect_chips::SDI_STRETCH).abs() < 1e-4,
        "the SDI layer stretches along the vector, not across it: {sdi:?}"
    );
    let di = model::effect_chips::place(model::frame::ChipVisual::Di, 90.0, 0.0, 0.0, 184.0, 64, 256);
    assert!((di.w / di.h - model::effect_chips::DI_WIDTH_RATIO).abs() < 1e-6, "the DI chips are unchanged: {di:?}");
    let output = std::process::Command::new("/usr/bin/python3")
        .args(["-I", "tools/sdi_streak_measure.py"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap_or_else(|error| panic!("the measurement needs /usr/bin/python3: {error}"));
    let text = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "tools/sdi_streak_measure.py failed:\n{text}");
    println!("{text}");
}

/// `core07_no_portrait_pictures` (SPEC-core-07 :224, Q16 as amended 2026-11-03 for RC10): no portrait
/// PNG in the repo or the built archive; the fork's 89 stock icons are not carried over. The runtime
/// reads each grid's picture at run time through the mod-aware loader, so what ships is only the 38
/// provenance-recorded item icons (`assets/item-icons/`, ASSET-REPRODUCTION-R15) plus the two archive
/// recipes, whose literal bytes are the plugin's own additions to the player's stock archive: they
/// must add no picture except those item textures. The review mockup (`docs/menu-mockup/`, SPEC-core-07
/// section 4.6 "Review mockup only") and the Eden run evidence (`eden/`) hold pictures but never ship.
#[test]
fn core07_no_portrait_pictures() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // (a) Every PNG in the repo lives in a folder that never ships: the 38 item icons, the review
    // mockup and the decision pictures under docs/, or the Eden run evidence under eden/.
    let mut pngs = Vec::new();
    walk(root, &mut |path| {
        if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")) {
            pngs.push(path.to_path_buf());
        }
    });
    let offenders: Vec<String> = pngs
        .iter()
        .filter(|path| !picture_evidence_folder(path, root))
        .map(|path| path.strip_prefix(root).unwrap_or(path).to_string_lossy().into_owned())
        .collect();
    assert!(offenders.is_empty(), "portrait or unblessed PNG outside the never-ships folders: {offenders:?}");
    // (b) The fork's roster-grid assets (its 89 stock icons) are not carried over.
    assert!(!root.join("scripts").join("roster-grid").exists(), "the fork's scripts/roster-grid is carried over");
    // (c) `assets/` ships exactly the item icons and the two archive recipes - nothing portrait-sized. Beside them, the
    // recipes' pane-table digest (`panes-recipe.sha256`, text, checked by tests/pane_recipe_guard.rs, never embedded).
    let assets = root.join("assets");
    let mut entries: Vec<String> = std::fs::read_dir(&assets)
        .unwrap_or_else(|error| panic!("read {}: {error}", assets.display()))
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        ["info_training.tra1", "info_waiting.tra1", "item-icons", "panes-recipe.sha256"],
        "assets/ holds more than the item icons, the archive recipes and their pane-table digest"
    );
    let icons: Vec<String> = std::fs::read_dir(assets.join("item-icons"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(icons.len(), 39, "38 item icons plus sources.json, found {icons:?}");
    assert!(icons.iter().all(|name| name == &"sources.json".to_owned() || is_item_icon(name)), "unexpected file in assets/item-icons: {icons:?}");
    // (d) The recipes' own bytes (what the built archive gains over the player's stock archive) carry
    // no picture but the item textures: no PNG magic, and no portrait/roster/stock-icon texture name.
    for recipe in ["info_training.tra1", "info_waiting.tra1"] {
        let bytes = std::fs::read(assets.join(recipe)).unwrap_or_else(|error| panic!("read {recipe}: {error}"));
        assert!(!contains(&bytes, b"\x89PNG"), "{recipe} embeds a PNG");
        for name in ["portrait", "roster_icon", "chara_", "fighter", "stock_icon"] {
            assert!(!contains_ignore_case(&bytes, name.as_bytes()), "{recipe} names a {name} texture");
        }
        assert!(contains(&bytes, b"tr_item_"), "{recipe} lost the item textures");
    }
}

/// Whether `path` is inside one of the picture folders that never ship (the check's docstring).
fn picture_evidence_folder(path: &Path, root: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else { return false };
    let Some(top) = relative.components().next() else { return false };
    let top = top.as_os_str().to_string_lossy().into_owned();
    if top == "eden" {
        return true;
    }
    if top == "assets" {
        return relative.components().nth(1).is_some_and(|part| part.as_os_str() == "item-icons");
    }
    if top == "docs" {
        // The review mockup (never ships, SPEC-core-07 section 4.6) and the decision pictures.
        let second = relative.components().nth(1).map(|part| part.as_os_str().to_string_lossy().into_owned());
        return matches!(second.as_deref(), Some("menu-mockup") | Some("decisions"));
    }
    false
}

fn is_item_icon(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("icon-") else { return false };
    let Some((number, "png")) = rest.split_once('.') else { return false };
    number.len() == 2 && number.bytes().all(|digit| digit.is_ascii_digit())
}

fn walk(dir: &Path, visit: &mut impl FnMut(&Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if name != ".git" && name != "target" && name != "tmp" {
                walk(&path, visit);
            }
        } else if name != "Cargo.lock" {
            visit(&path);
        }
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|window| window == needle)
}

fn contains_ignore_case(haystack: &[u8], needle: &[u8]) -> bool {
    let lower = |byte: u8| byte.to_ascii_lowercase();
    let needle: Vec<u8> = needle.iter().copied().map(lower).collect();
    haystack.windows(needle.len()).any(|window| window.iter().copied().map(lower).eq(needle.iter().copied()))
}
