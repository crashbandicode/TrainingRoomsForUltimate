//! Lobby-stage preferences. Catalog indices are stable; ranks store selection order.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
pub use crate::engine::stages::stage_forms::{Form, Forms, Selection};

// Owned ui_stage_db.prc: can_select=true, sorted by native disp_order.
// Labels join name_id to msg_name+us_en.msbt's nam_stg1_* keys; stage IDs
// join stage_place_id to the native form-0 table. Waiting Room is our default;
// Training is appended separately because it is not a normal selectable stage.
pub const STAGES: &[(u32, &str)] = &[
    (311, "Waiting Room"),
    (0, "Battlefield"),
    (347, "Small Battlefield"),
    (2, "Big Battlefield"),
    (3, "Final Destination"),
    (5, "Peach's Castle"),
    (8, "Kongo Jungle"),
    (11, "Hyrule Castle"),
    (14, "Super Happy Tree"),
    (17, "Dream Land"),
    (20, "Saffron City"),
    (23, "Mushroom Kingdom"),
    (26, "Princess Peach's Castle"),
    (29, "Rainbow Cruise"),
    (32, "Kongo Falls"),
    (35, "Jungle Japes"),
    (38, "Great Bay"),
    (41, "Temple"),
    (62, "Brinstar"),
    (47, "Yoshi's Island (Melee)"),
    (44, "Yoshi's Story"),
    (50, "Fountain of Dreams"),
    (53, "Green Greens"),
    (56, "Corneria"),
    (59, "Venom"),
    (68, "Pokémon Stadium"),
    (65, "Onett"),
    (80, "Mushroom Kingdom II"),
    (71, "Brinstar Depths"),
    (77, "Big Blue"),
    (74, "Fourside"),
    (83, "Delfino Plaza"),
    (134, "Mushroomy Kingdom"),
    (125, "Figure-8 Circuit"),
    (104, "WarioWare, Inc."),
    (92, "Bridge of Eldin"),
    (122, "Norfair"),
    (128, "Frigate Orpheon"),
    (86, "Yoshi's Island"),
    (110, "Halberd"),
    (89, "Lylat Cruise"),
    (107, "Pokémon Stadium 2"),
    (137, "Port Town Aero Dive"),
    (101, "Castle Siege"),
    (131, "Distant Planet"),
    (95, "Smashville"),
    (116, "New Pork City"),
    (119, "Summit"),
    (98, "Skyworld"),
    (113, "Shadow Moses Island"),
    (140, "Luigi's Mansion"),
    (143, "Pirate Ship"),
    (146, "Spear Pillar"),
    (149, "75 m"),
    (152, "Mario Bros."),
    (155, "Hanenbow"),
    (158, "Green Hill Zone"),
    (161, "3D Land"),
    (164, "Golden Plains"),
    (167, "Paper Mario"),
    (170, "Gerudo Valley"),
    (173, "Spirit Train"),
    (218, "Dream Land GB"),
    (176, "Unova Pokémon League"),
    (179, "Prism Tower"),
    (224, "Mute City SNES"),
    (215, "Magicant"),
    (182, "Arena Ferox"),
    (185, "Reset Bomb Forest"),
    (188, "Tortimer Island"),
    (221, "Balloon Fight"),
    (200, "Living Room"),
    (203, "Find Mii"),
    (206, "Tomodachi Life"),
    (209, "PictoChat 2"),
    (227, "Mushroom Kingdom U"),
    (230, "Mario Galaxy"),
    (233, "Mario Circuit"),
    (236, "Skyloft"),
    (239, "The Great Cave Offensive"),
    (242, "Kalos Pokémon League"),
    (245, "Coliseum"),
    (278, "Flat Zone X"),
    (248, "Palutena's Temple"),
    (251, "Gamer"),
    (254, "Garden of Hope"),
    (257, "Town and City"),
    (260, "Wii Fit Studio"),
    (191, "Boxing Ring"),
    (197, "Gaur Plain"),
    (281, "Duck Hunt"),
    (263, "Wrecking Crew"),
    (266, "Pilotwings"),
    (269, "Wuhu Island"),
    (272, "Windy Hill Zone"),
    (212, "Wily Castle"),
    (275, "PAC-LAND"),
    (287, "Super Mario Maker"),
    (284, "Suzaku Castle"),
    (290, "Midgar"),
    (293, "Umbra Clock Tower"),
    (305, "New Donk City Hall"),
    (302, "Great Plateau Tower"),
    (296, "Moray Towers"),
    (299, "Dracula's Castle"),
    (328, "Mementos"),
    (332, "Yggdrasil's Altar"),
    (335, "Spiral Mountain"),
    (338, "King of Fighters Stadium"),
    (341, "Garreg Mach Monastery"),
    (344, "Spring Stadium"),
    (348, "Minecraft World"),
    (351, "Northern Cave"),
    (355, "Cloud Sea of Alrest"),
    (358, "Mishima Dojo"),
    (361, "Hollow Bastion"),
    // Append special practice stages so existing persisted catalog indices stay stable.
    (308, "Training"),
];

// Forms persists one entry for each stable catalog index.
const _: () = assert!(STAGES.len() == 117);

// Explicit [normal, omega, battlefield] native IDs from the recorded 13.0.5
// table (0x454a7e8, stride72), joined by base_stage_id and table form, not offsets.
// Special stages below have missing forms; do not infer IDs from adjacency.
const THREE_FORM_STAGES: &[[u32; 3]] = &[
    [5, 6, 7], [8, 9, 10], [11, 12, 13], [14, 15, 16], [17, 18, 19],
    [20, 21, 22], [23, 24, 25], [26, 27, 28], [29, 30, 31], [32, 33, 34],
    [35, 36, 37], [38, 39, 40], [41, 42, 43], [44, 45, 46], [47, 48, 49],
    [50, 51, 52], [53, 54, 55], [56, 57, 58], [59, 60, 61], [62, 63, 64],
    [65, 66, 67], [68, 69, 70], [71, 72, 73], [74, 75, 76], [77, 78, 79],
    [80, 81, 82], [83, 84, 85], [86, 87, 88], [89, 90, 91], [92, 93, 94],
    [95, 96, 97], [98, 99, 100], [101, 102, 103], [104, 105, 106],
    [107, 108, 109], [110, 111, 112], [113, 114, 115], [116, 117, 118],
    [119, 120, 121], [122, 123, 124], [125, 126, 127], [128, 129, 130],
    [131, 132, 133], [134, 135, 136], [137, 138, 139], [140, 141, 142],
    [143, 144, 145], [146, 147, 148], [149, 150, 151], [152, 153, 154],
    [155, 156, 157], [158, 159, 160], [161, 162, 163], [164, 165, 166],
    [167, 168, 169], [170, 171, 172], [173, 174, 175], [176, 177, 178],
    [179, 180, 181], [182, 183, 184], [185, 186, 187], [188, 189, 190],
    [191, 192, 193], [197, 198, 199], [200, 201, 202], [203, 204, 205],
    [206, 207, 208], [209, 210, 211], [212, 213, 214], [215, 216, 217],
    [218, 219, 220], [221, 222, 223], [224, 225, 226], [227, 228, 229],
    [230, 231, 232], [233, 234, 235], [236, 237, 238], [239, 240, 241],
    [242, 243, 244], [245, 246, 247], [248, 249, 250], [251, 252, 253],
    [254, 255, 256], [257, 258, 259], [260, 261, 262], [263, 264, 265],
    [266, 267, 268], [269, 270, 271], [272, 273, 274], [275, 276, 277],
    [278, 279, 280], [281, 282, 283], [284, 285, 286], [287, 288, 289],
    [290, 291, 292], [293, 294, 295], [296, 297, 298], [299, 300, 301],
    [302, 303, 304], [305, 306, 307], [328, 329, 330], [332, 333, 334],
    [335, 336, 337], [338, 339, 340], [341, 342, 343], [344, 345, 346],
    [348, 349, 350], [351, 352, 353], [355, 356, 357], [358, 359, 360],
    [361, 362, 363],
];

pub fn native_forms(stage: u32) -> [Option<u32>; 3] {
    match stage {
        0 => [Some(0), Some(1), None],
        3 => [Some(3), None, Some(4)],
        2 | 347 | 308 | 311 => [Some(stage), None, None],
        _ => THREE_FORM_STAGES.iter().find(|row| row[0] == stage)
            .map_or([None; 3], |row| row.map(Some)),
    }
}

pub fn native_stage(selection: Selection) -> Result<u32, &'static str> {
    if !STAGES.iter().any(|(stage, _)| *stage == selection.stage) {
        return Err("stage outside native selectable catalog");
    }
    native_forms(selection.stage)[selection.form as usize]
        .ok_or("form unavailable for this stage")
}

pub fn form_catalog() -> Vec<[Option<u32>; 3]> {
    STAGES.iter().map(|(stage, _)| native_forms(*stage)).collect()
}

/// Validate all stored forms, including unranked cells, before publishing settings.
pub fn validate_forms(forms: &Forms) -> Result<(), &'static str> {
    if forms.as_slice().len() != STAGES.len() { return Err("stage form catalog mismatch"); }
    for ((stage, _), form) in STAGES.iter().zip(forms.as_slice()) {
        native_stage(Selection { stage: *stage, form: *form })?;
    }
    Ok(())
}

pub fn validate_preferences(prefs: &crate::engine::stages::stage_menu::Preferences) -> Result<(), &'static str> {
    native_stage(Selection { stage: prefs.direct_stage, form: prefs.direct_form })?;
    validate_forms(&prefs.rotation_forms)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LobbyStageOrder {
    all: bool,
    ranks: [u8; STAGES.len()],
}

impl Default for LobbyStageOrder {
    fn default() -> Self {
        Self::NATIVE
    }
}

impl LobbyStageOrder {
    pub const NATIVE: Self = Self {
        all: false,
        ranks: {
            let mut ranks = [0; STAGES.len()];
            ranks[0] = 1;
            ranks
        },
    };

    pub fn stages(self) -> Vec<u32> {
        if self.all {
            return STAGES.iter().map(|(id, _)| *id).collect();
        }
        let mut selected: Vec<_> = self
            .ranks
            .iter()
            .zip(STAGES)
            .filter(|(rank, _)| **rank != 0)
            .collect();
        selected.sort_unstable_by_key(|(rank, _)| **rank);
        selected.into_iter().map(|(_, (id, _))| *id).collect()
    }

    pub fn selections(self, forms: &Forms) -> Result<Vec<Selection>, &'static str> {
        validate_forms(forms)?;
        self.stages().into_iter().map(|stage| {
            let index = STAGES.iter().position(|(id, _)| *id == stage)
                .ok_or("stage outside native selectable catalog")?;
            Ok(Selection { stage, form: forms.get(index).ok_or("stage form index outside catalog")? })
        }).collect()
    }

}

impl Serialize for LobbyStageOrder {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        std::iter::once(u8::from(self.all))
            .chain(self.ranks)
            .collect::<Vec<_>>()
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for LobbyStageOrder {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let values = Vec::<u8>::deserialize(deserializer)?;
        // The first full catalog had 116 stages, before Training was appended.
        if !matches!(values.len(), 4 | 117) && values.len() != STAGES.len() + 1 {
            return Err(serde::de::Error::custom("invalid lobby stage catalog"));
        }
        if values[0] > 1 {
            return Err(serde::de::Error::custom("invalid lobby stage catalog"));
        }
        let mut ranks = [0; STAGES.len()];
        if values.len() == 4 {
            // Initial development preferences used Waiting Room/Battlefield/FD.
            for (old, stage) in [311, 0, 3].iter().enumerate() {
                let index = STAGES.iter().position(|(id, _)| id == stage).unwrap();
                ranks[index] = values[old + 1];
            }
        } else {
            ranks[..values.len() - 1].copy_from_slice(&values[1..]);
        }
        let mut selected: Vec<_> = ranks.iter().copied().filter(|rank| *rank != 0).collect();
        selected.sort_unstable();
        let all = values[0] == 1;
        if (all && !selected.is_empty())
            || (!all
                && (selected.is_empty() || !selected.iter().copied().eq(1..=selected.len() as u8)))
        {
            return Err(serde::de::Error::custom(
                "choose All or a contiguous lobby stage order",
            ));
        }
        Ok(Self { all, ranks })
    }
}

