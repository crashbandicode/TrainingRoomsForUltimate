//! The factory table: every setting row of the menu with its wire shape and factory value
//! (SPEC-core-02 §2.7 and §2.8, SPEC-core-04 §3.1; one array element per option in the option set's
//! declaration order, so the order below each id is the on-disk order).
//!
//! Ids and shapes of the upstream rows are the ones of the spec tables and of the two real fork files
//! in `tests/fixtures/`. The rows the fork added carry the fork's own ids, read from the fork's
//! ours-new sources (`training_mod_consts/src/cycle_preload_config.rs`, `menu_json.rs`,
//! `p1_cpu_menu_tests.rs`, `character_items.rs`): see CARRY-OVER.md. Rows whose value is not a plain
//! array (the fighter rosters, the Kirby hats, the Mii picks, the stage preferences, the item grant)
//! have no row here: they stay kept-as-they-are keys until the package that owns them.
//!
//! Option sets (index 0 first):
//! * `OnOff` is `[On, Off]`; a toggle's factory value is `[1,0]` (On) or `[0,1]` (Off).
//! * `Direction` (11) is Out, Up Out, Up, Up In, In, Down In, Down, Down Out, Neutral, Left, Right.
//! * `Action` (30) is Air Dodge .. Dash Attack, Playback Slot 1..5 (labels in `menu::options`).
//! * `Delay` (31) is 0..=30; `MedDelay` (31) is 0, 5 .. 150; `LongDelay` (31) is 0, 10 .. 300.

use super::settings::{Def, Kind};

/// `len` elements, all `0` except the ones listed, which are `1`.
const fn ones<const N: usize>(at: &[usize]) -> [u8; N] {
    let mut row = [0u8; N];
    let mut i = 0;
    while i < at.len() {
        row[at[i]] = 1;
        i += 1;
    }
    row
}

macro_rules! row {
    ($id:literal, $kind:ident, $len:literal, $at:expr) => {
        Def { id: $id, kind: Kind::$kind, len: $len, default: &ones::<$len>(&$at) }
    };
}

macro_rules! range_row {
    ($id:literal) => {
        Def { id: $id, kind: Kind::Range, len: 0, default: &[] }
    };
}

/// Index of `input_delay` in `DEFS`.
pub const INPUT_DELAY: usize = 0;
/// Index of `lra_reset` in `DEFS`.
pub const LRA_RESET: usize = 1;
/// Index of `stage_hazards` in `DEFS`.
pub const STAGE_HAZARDS: usize = 2;

/// Number of rows.
pub const COUNT: usize = 105;

/// The rows this build knows, in menu order of their first appearance in SPEC-core-02 §2.7. The
/// first three keep the indices the earlier packages use.
pub static DEFS: [Def; COUNT] = [
    row!("input_delay", Single, 31, [0]),
    row!("lra_reset", Single, 2, [0]),
    row!("stage_hazards", Single, 2, [1]),
    // Mash Settings (draft 9: the CPU page).
    row!("mash_state", Weighted, 30, []),
    row!("follow_up", Weighted, 30, []),
    // Hitstun, Shieldstun, Tumble, Clatter.
    row!("mash_triggers", Multiple, 17, [0, 1, 3, 7]),
    row!("attack_angle", Weighted, 3, []),
    row!("throw_state", Weighted, 5, [0]),
    row!("throw_delay", Weighted, 31, []),
    row!("pummel_delay", Weighted, 31, []),
    // [True, False]
    row!("falling_aerials", Weighted, 2, [1]),
    row!("full_hop", Weighted, 2, [0]),
    row!("aerial_delay", Weighted, 31, []),
    row!("fast_fall", Weighted, 2, [1]),
    row!("fast_fall_delay", Weighted, 31, []),
    row!("oos_offset", Weighted, 31, []),
    row!("reaction_time", Weighted, 31, []),
    // Override Settings: the 13 situations.
    row!("ledge_neutral_override", Weighted, 30, []),
    row!("ledge_roll_override", Weighted, 30, []),
    row!("ledge_jump_override", Weighted, 30, []),
    row!("ledge_attack_override", Weighted, 30, []),
    row!("tech_action_override", Weighted, 30, []),
    row!("clatter_override", Weighted, 30, []),
    row!("tumble_override", Weighted, 30, []),
    row!("hitstun_override", Weighted, 30, []),
    row!("parry_override", Weighted, 30, []),
    row!("shieldstun_override", Weighted, 30, []),
    row!("footstool_override", Weighted, 30, []),
    row!("landing_override", Weighted, 30, []),
    row!("trump_override", Weighted, 30, []),
    // Defensive Settings.
    row!("air_dodge_dir", Weighted, 11, []),
    row!("di_state", Weighted, 11, []),
    row!("sdi_state", Weighted, 11, []),
    row!("sdi_strength", Single, 4, [0]),
    row!("clatter_strength", Single, 4, [0]),
    // Neutral Getup, Roll, Jump, Getup Attack.
    row!("ledge_state", Weighted, 10, [0, 1, 2, 3]),
    row!("ledge_delay", Weighted, 31, []),
    row!("tech_state", Weighted, 4, [0, 1, 2, 3]),
    row!("miss_tech_state", Weighted, 4, [0, 1, 2, 3]),
    row!("shield_state", Single, 4, [0]),
    row!("shield_tilt", Single, 11, []),
    // [On, Off]: Crouch Off, Dodge Staling On, Hide Tech Animations Off.
    row!("crouch", Single, 2, [1]),
    row!("stale_dodges", Single, 2, [0]),
    row!("tech_hide", Single, 2, [1]),
    // Input Recording.
    row!("recording_slot", Single, 5, [0]),
    row!("record_trigger", Single, 2, [0]),
    // 60, 90, 120, 150 ...: 150 is index 3.
    row!("recording_duration", Single, 19, [3]),
    row!("recording_crop", Single, 2, [0]),
    row!("playback_button_slots", Weighted, 5, [0]),
    row!("hitstun_playback", Single, 3, [0]),
    row!("save_state_playback", Weighted, 5, []),
    row!("playback_mash", Single, 2, [0]),
    row!("playback_loop", Single, 2, [1]),
    // Button chords: 16 buttons A B X Y L R ZL ZR Up Down Left Right Plus Minus LStick RStick.
    row!("save_state_save", Multiple, 16, [6, 9]),
    row!("save_state_load", Multiple, 16, [6, 8]),
    row!("input_record", Multiple, 16, [7, 9]),
    row!("input_playback", Multiple, 16, [7, 8]),
    row!("cpu_cycle", Multiple, 16, [6, 10]),
    row!("player_cycle", Multiple, 16, [6, 11]),
    // Save States.
    row!("save_state_mirroring", Single, 3, [0]),
    row!("save_state_autoload", Single, 2, [1]),
    row!("save_damage_cpu", Single, 3, [0]),
    Def { id: "save_damage_limits_cpu", kind: Kind::Slider, len: 2, default: &[0, 150] },
    row!("save_damage_player", Single, 3, [0]),
    Def { id: "save_damage_limits_player", kind: Kind::Slider, len: 2, default: &[0, 150] },
    row!("save_state_enable", Single, 2, [0]),
    row!("save_state_slot", Single, 5, [0]),
    row!("randomize_slots", Weighted, 5, []),
    row!("buff_state", Multiple, 18, []),
    // Misc Settings.
    row!("frame_advantage", Single, 2, [1]),
    row!("hitbox_vis", Single, 2, [1]),
    // Off, Frame strip: the frame strip (SPEC-input-06 §3.1; it replaced `input_display` and `input_display_status`).
    row!("input_display_strip", Single, 2, [1]),
    row!("hud", Single, 2, [0]),
    // Draft 9 (SPEC-core-04 §3.1): Off by default.
    row!("respawn_invincibility", Single, 2, [1]),
    row!("settings_override_playback", Single, 2, [1]),
    row!("dev_reload_config", Single, 2, [1]),
    row!("dev_debug_hooks", Single, 2, [1]),
    // Preload Samples: the fork's id (SPEC-core-02 §2.8 proposed `preload_samples`; see CARRY-OVER.md).
    row!("cycle_preload_measurements", Single, 2, [1]),
    // The fork's own rows: Macro Source (Numbered Slots, Named Library), Fighter Prompt (On),
    // Player Swapping, Lobby Practice, Training Stage Rotation, Solo Popo (all Off).
    row!("macro_source", Single, 2, [0]),
    row!("fighter_prompt", Single, 2, [0]),
    row!("trio_practice", Single, 2, [1]),
    row!("lobby_practice", Single, 2, [1]),
    row!("training_stage_rotation", Single, 2, [1]),
    row!("solo_popo_original", Single, 2, [1]),
    row!("p1_cpu_control", Single, 2, [1]),
    // P1 CPU Level 1..=9: 3 is index 2.
    row!("p1_cpu_level", Single, 9, [2]),
    // Cycle Preload: Off, 1..=5 (the built-in maximum, 5, is index 5).
    row!("cycle_preload_slots", Single, 6, [5]),
    // Fighters, Hats, Both: Both.
    row!("cycle_preload_sources", Single, 3, [2]),
    // Ready Fighter Bodies: Off, 2, 3 (the native maximum, 3).
    row!("cycle_preload_bank_members", Single, 3, [2]),
    // Ready Fighters 2..=12: 8 is index 6.
    row!("cycle_preload_ready_fighters", Single, 11, [6]),
    // Group Swap and the seven group switches, all On.
    row!("cycle_preload_group_swap", Single, 2, [0]),
    row!("cycle_preload_ic_pair_bank", Single, 2, [0]),
    row!("cycle_preload_original_bank", Single, 2, [0]),
    row!("cycle_preload_solo_popo_groups", Single, 2, [0]),
    row!("cycle_preload_ic_pair_groups", Single, 2, [0]),
    row!("cycle_preload_native_groups", Single, 2, [0]),
    row!("cycle_preload_pair_groups", Single, 2, [0]),
    row!("cycle_preload_mii_groups", Single, 2, [0]),
    // The optional range of each delay row (N3), as option indices; absent while empty.
    range_row!("aerial_delay_range"),
    range_row!("fast_fall_delay_range"),
    range_row!("throw_delay_range"),
    range_row!("pummel_delay_range"),
    range_row!("ledge_delay_range"),
    range_row!("oos_offset_range"),
    range_row!("reaction_time_range"),
];
