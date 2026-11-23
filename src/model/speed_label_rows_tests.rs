use crate::engine::seams::native_seams::{rows_speed_label::ROWS, Seam, SeamKind};
use crate::model::game_speed::{glyph_style, GLYPH_STYLE, LABEL_ROWS};

fn row(label: &str) -> &'static Seam {
    ROWS.iter().find(|row| row.label == label).unwrap_or_else(|| panic!("{label}"))
}

fn words(row: &Seam) -> Vec<u32> {
    row.guard.as_chunks::<4>().0.iter().map(|bytes| u32::from_le_bytes(*bytes)).collect()
}

fn offset(w: u32, opcode: u32, base: u32, target: u32) -> usize {
    assert_eq!(w & 0xffc0_03ff, opcode | (base << 5) | target);
    (((w >> 10) & 0xfff) * 4) as usize
}

fn branch(pc: usize, word: u32) -> Option<usize> {
    (word >> 26 == 0b100101).then(|| (pc as i64 + (((word & 0x03ff_ffff) as i64) << 38 >> 38) * 4) as usize)
}

fn adrp_add(pc: usize, adrp: u32, add: u32) -> Option<usize> {
    if adrp & 0x9f00_0000 != 0x9000_0000 || add & 0xffc0_0000 != 0x9100_0000 {
        return None;
    }
    let register = adrp & 31;
    if (add & 31) != register || ((add >> 5) & 31) != register {
        return None;
    }
    let imm = (((adrp >> 29) & 3) | (((adrp >> 5) & 0x7_ffff) << 2)) as i64;
    Some(((pc as i64 & !0xfff) + ((imm << 43) >> 43) * 0x1000) as usize + ((add >> 10) & 0xfff) as usize)
}

#[test]
fn the_glyph_styles_are_the_games_table() {
    let values = words(row("speed_label.glyph_styles"));
    assert_eq!(&values[..7], &[1, 2, 3, 4, 5, 6, 6]);
    for style in 1..=7 {
        assert_eq!(glyph_style(style), values[style as usize - 1]);
    }
    assert_eq!((glyph_style(0), glyph_style(8)), (0, 0));
}

#[test]
fn the_frame_label_reads_the_menus_glyph_style() {
    assert_eq!(offset(words(row("speed_label.frame_text"))[0], 0xb940_0000, 22, 8), GLYPH_STYLE);
}

#[test]
fn the_frame_label_sets_both_texts_with_the_button_code() {
    let row = row("speed_label.frame_set");
    let words = words(row);
    assert!(words.windows(2).enumerate().any(|(i, pair)|
        pair[0] == 0x2a14_03e3 && branch(row.offset + 4 * (i + 1), pair[1]) == Some(0x377a280)));
    assert!(words.iter().any(|word| *word & 0xffc0_03ff == 0xb940_02c8 && offset(*word, 0xb940_0000, 22, 8) == GLYPH_STYLE));
    assert!(words.iter().any(|word| *word & 0xffc0_03ff == 0xb900_02a8 && offset(*word, 0xb900_0000, 21, 8) == 0x25c));
}

#[test]
fn the_menus_open_step_stores_the_glyph_style_of_the_controller() {
    let source = words(row("speed_label.style_source"));
    assert_eq!(offset(source[15], 0xb940_0000, 8, 8), crate::model::chords::field::STYLE);
    let store = row("speed_label.style_store");
    let words = words(store);
    assert!(words.windows(2).enumerate().any(|(i, pair)|
        adrp_add(store.offset + 4 * i, pair[0], pair[1]) == Some(row("speed_label.glyph_styles").offset)));
    assert!(words.iter().enumerate().any(|(i, word)| branch(store.offset + 4 * i, *word) == Some(0x1bb8b20)));
    assert_eq!(offset(words[15], 0xb900_0000, 21, 22), GLYPH_STYLE);
}

#[test]
fn every_label_row_is_gated() {
    assert_eq!(ROWS.iter().map(|row| row.label).collect::<Vec<_>>(), LABEL_ROWS);
    for row in ROWS {
        assert_eq!(row.group, "speed_label");
        assert_eq!(row.kind, SeamKind::Relied);
        assert!(row.compared());
    }
}
