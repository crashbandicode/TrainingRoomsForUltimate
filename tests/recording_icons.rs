//! The floppy and the loop in the committed layout archive.
//!
//! The builder test of the same name (`tools/layout-builder`) imports the glyphs into an archive and
//! checks that the opaque texels are white. This one checks the recipes that the plugin embeds, which
//! is where rr-6's bind names have to be, and the licence carried with the source PNGs.

#[test]
fn save_and_repeat_land_in_the_built_archive() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = root.join("assets/recording-icons");
    let licence = std::fs::read_to_string(dir.join("LICENSE-Apache-2.0.txt"))
        .unwrap_or_else(|error| panic!("licence: {error}"));
    assert!(licence.contains("Apache License"), "the Apache licence is not in the asset directory");
    assert!(licence.contains("Version 2.0"));
    let source = std::fs::read_to_string(dir.join("SOURCE.txt")).unwrap_or_else(|error| panic!("source: {error}"));
    assert!(source.contains("symbols/web/save/materialsymbolsoutlined/save_24px.svg"));
    assert!(source.contains("symbols/web/repeat/materialsymbolsoutlined/repeat_24px.svg"));
    for recipe in ["assets/info_training.tra1", "assets/info_waiting.tra1"] {
        let bytes = std::fs::read(root.join(recipe)).unwrap_or_else(|error| panic!("read {recipe}: {error}"));
        for name in ["tr_icon_save", "tr_icon_repeat"] {
            assert!(
                bytes.windows(name.len()).any(|window| window == name.as_bytes()),
                "{recipe} is missing {name}, the name rr-6 binds"
            );
        }
    }
}
