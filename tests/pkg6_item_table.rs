//! The character-item table (`ALL_CHAR_ITEMS`, `src/runtime/lobby/training/character_specific.rs`): one row per `ITEM_NAMES` label,
//! in its order (the saved item choice indexes the table directly), each row naming the fighter its label names and the same game
//! constants package 3 resolves for that choice. The constants' VALUES are the game's own and are not in the main image (the
//! table `lib::lua_bind_get_value` searches is built at run time): the diag build dumps them (`[ItemTable]`, Eden row).

mod common;

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

#[path = "../src/model/items/character_items.rs"]
#[allow(dead_code)]
mod items;

/// `(ITEM_NAMES label prefix, the fighter constant its entries name)`.
const OWNERS: [(&str, &str); 18] = [
    ("Banjo & Kazooie", "FIGHTER_KIND_BUDDY"),
    ("Bowser Jr.", "FIGHTER_KIND_KOOPAJR"),
    ("Daisy", "FIGHTER_KIND_DAISY"),
    ("Diddy Kong", "FIGHTER_KIND_DIDDY"),
    ("King K. Rool", "FIGHTER_KIND_KROOL"),
    ("Link", "FIGHTER_KIND_LINK"),
    ("Mega Man", "FIGHTER_KIND_ROCKMAN"),
    ("Pac-Man", "FIGHTER_KIND_PACMAN"),
    ("Peach", "FIGHTER_KIND_PEACH"),
    ("Richter", "FIGHTER_KIND_RICHTER"),
    ("R.O.B.", "FIGHTER_KIND_ROBOT"),
    ("Robin", "FIGHTER_KIND_REFLET"),
    ("Sheik", "FIGHTER_KIND_SHEIK"),
    ("Simon", "FIGHTER_KIND_SIMON"),
    ("Snake", "FIGHTER_KIND_SNAKE"),
    ("Toon Link", "FIGHTER_KIND_TOONLINK"),
    ("Villager", "FIGHTER_KIND_MURABITO"),
    ("Young Link", "FIGHTER_KIND_YOUNGLINK"),
];

/// The rows of the table (`rows!`), as the identifiers each names: `(fighter, item kind, variation)`.
fn table_rows() -> Vec<(String, Option<String>, Option<String>)> {
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/runtime/lobby/training/character_specific.rs")).unwrap();
    let start = source.find("macro_rules! rows").unwrap();
    let body = &source[start..];
    let end = body.find("        };").unwrap();
    body[..end]
        .lines()
        .filter_map(|line| line.trim().strip_prefix("$m!(").and_then(|rest| rest.strip_suffix("),")))
        .map(|args| {
            let parts: Vec<&str> = args.split(',').map(str::trim).collect();
            let fighter = parts[0].to_string();
            match parts.as_slice() {
                [_] => (fighter, None, None),
                [_, kind] => (fighter, Some((*kind).to_string()), None),
                [_, "", variation] => (fighter, None, Some((*variation).to_string())),
                [_, kind, variation] => (fighter, Some((*kind).to_string()), Some((*variation).to_string())),
                _ => panic!("unreadable row {args}"),
            }
        })
        .collect()
}

#[test]
fn the_table_has_one_row_per_item_name_and_each_row_belongs_to_the_named_fighter() {
    let rows = table_rows();
    assert_eq!(rows.len(), items::ITEM_NAMES.len());
    for (index, ((fighter, _, _), label)) in rows.iter().zip(items::ITEM_NAMES).enumerate() {
        let owner = OWNERS.iter().find(|(prefix, _)| label.starts_with(&format!("{prefix}:")));
        let (_, expected) = owner.unwrap_or_else(|| panic!("no owner for {label}"));
        assert_eq!(fighter, expected, "row {index} ({label})");
    }
}

/// Package 3 resolved the same 38 choices on its own (`model::state::items::spec_for`, SPEC-state-04 section 2.1, proven by its Eden
/// rows): the item kind, the owner fighter and the variation constant of every entry name the same game constants as package 3's.
#[test]
fn the_table_names_the_constants_package_3_resolves_for_each_choice() {
    use model::state::ids::NAMES;
    use model::state::items::{spec_for, Variation};
    let name = |c: model::state::ids::C| NAMES[c as usize];
    for (choice, (fighter, kind, variation)) in table_rows().iter().enumerate() {
        let spec = spec_for(choice as u8).unwrap_or_else(|| panic!("package 3 has no spec for choice {choice}"));
        // Items any fighter can make have no source in package 3's spec: the table names the fighter the label names.
        if let Some(source) = spec.source {
            assert_eq!(fighter, name(source), "choice {choice}: owner fighter");
        }
        match (spec.turnip, spec.kind) {
            // The turnips and the banana are generated through the article kinds the grant names itself.
            (Some(_), _) => assert!(kind.is_none(), "choice {choice}: a turnip names no item kind in the table"),
            (None, kind_of_spec) if name(kind_of_spec) == "ITEM_KIND_BANANA" => assert!(kind.is_none(), "choice {choice}: banana"),
            (None, kind_of_spec) => assert_eq!(kind.as_deref(), Some(name(kind_of_spec)), "choice {choice}: item kind"),
        }
        if let Variation::Constant(constant) = spec.variation {
            assert_eq!(variation.as_deref(), Some(name(constant)), "choice {choice}: variation constant");
        }
    }
}

/// The turnip faces the saved snapshots store (`valid_variation`: Smile 0..=4, Winky 5, Dot-Eyes 6, Stitch-Face 7) are the
/// `ITEM_VARIATION_*DAIKON_1/6/7/8` constants of the table; the constants' values are dumped by the diag build (`[ItemTable]`).
#[test]
fn the_turnip_rows_name_the_four_selectable_faces_in_order() {
    let rows = table_rows();
    for (first, who) in [(2, "DAISY"), (22, "PEACH")] {
        for (offset, face) in ["1", "6", "7", "8"].iter().enumerate() {
            let expected = format!("ITEM_VARIATION_{who}DAIKON_{face}");
            assert_eq!(rows[first + offset].2.as_deref(), Some(expected.as_str()), "choice {}", first + offset);
        }
    }
}
