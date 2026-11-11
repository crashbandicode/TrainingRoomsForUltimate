//! Option labels, in the declaration order of each option set (the order of the arrays on disk,
//! SPEC-core-02 §2.7). Static tables: nothing here allocates. The sentences are this plugin's own.

pub static ACTIONS: [&str; 30] = [
    "Air Dodge", "Jump", "Shield", "Spot Dodge", "Roll Forwards", "Roll Backwards", "Neutral Air", "Forward Air",
    "Back Air", "Up Air", "Down Air", "Neutral Special", "Side Special", "Up Special", "Down Special", "Forward Smash",
    "Up Smash", "Down Smash", "Jab", "Forward Tilt", "Up Tilt", "Down Tilt", "Grab", "Dash", "Dash Attack",
    "Playback Slot 1", "Playback Slot 2", "Playback Slot 3", "Playback Slot 4", "Playback Slot 5",
];

pub static MASH_TRIGGERS: [&str; 17] = [
    "Hitstun", "Shieldstun", "Parry", "Tumble", "Landing", "Ledge Trump", "Footstool", "Clatter", "Ledge Option",
    "Tech Option", "Mistech Option", "Grounded", "Airborne", "Distance: Close", "Distance: Mid", "Distance: Far",
    "Always",
];

pub static ATTACK_ANGLE: [&str; 3] = ["Neutral", "Up", "Down"];
pub static THROW: [&str; 5] = ["None", "Forward Throw", "Backward Throw", "Up Throw", "Down Throw"];
pub static LEDGE_OPTIONS: [&str; 10] = [
    "Neutral Getup", "Roll", "Jump", "Getup Attack", "Wait", "Playback Slot 1", "Playback Slot 2", "Playback Slot 3",
    "Playback Slot 4", "Playback Slot 5",
];
pub static TECH: [&str; 4] = ["No Tech", "Roll Forwards", "Roll Backwards", "Tech In Place"];
pub static MISTECH: [&str; 4] = ["Neutral Getup", "Getup Attack", "Roll Forwards", "Roll Backwards"];
pub static SHIELD: [&str; 4] = ["None", "Infinite", "Hold", "Constant"];
pub static STRENGTH: [&str; 4] = ["None", "Normal", "Medium", "High"];
pub static MIRRORING: [&str; 3] = ["None", "Alternate", "Random"];
pub static SAVE_DAMAGE: [&str; 3] = ["Default", "Save State", "Random Value"];
pub static SLOTS: [&str; 5] = ["Slot 1", "Slot 2", "Slot 3", "Slot 4", "Slot 5"];
pub static HITSTUN_PLAYBACK: [&str; 3] = ["As Hitstun Ends", "As Hitstop Ends", "As Hitstop Begins"];
/// The loop counts X cycles, and the Options row shows. The factory row stays length 2.
pub static LOOP_COUNTS: [&str; 6] = ["∞", "1", "2", "3", "5", "10"];
pub static RECORD_TRIGGER: [&str; 2] = ["Button Combination", "Save State Load"];
pub static DURATIONS: [&str; 19] = [
    "60", "90", "120", "150", "180", "210", "240", "270", "300", "330", "360", "390", "420", "450", "480", "510", "540",
    "570", "600",
];
pub static BUFFS: [&str; 18] = [
    "Accelerate", "Oomph", "Psyche Up", "Bounce", "Arsene", "Deep Breathing", "Limit", "KO Punch", "1-Winged Angel",
    "Jump", "Speed", "Shield", "Buster", "Smash", "Power Dragon", "Mini Waft", "Half Waft", "Full Waft",
];
pub static INPUT_DISPLAY: [&str; 2] = ["Off", "Frame strip"];
/// SPEC-input-07 §6: frames from the recorded press that still count as on time, and what follows an attempt.
pub static COMBO_TOLERANCE: [&str; 3] = ["Exact", "1 frame", "2 frames"];
pub static COMBO_AFTER: [&str; 2] = ["Keep playing", "Load save state"];
pub static MACRO_SOURCE: [&str; 2] = ["Numbered Slots", "Named Library"];
pub static CYCLE_SLOTS: [&str; 6] = ["Off", "1", "2", "3", "4", "5"];
pub static SOURCES: [&str; 3] = ["Fighters", "Hats", "Both"];
pub static BODIES: [&str; 3] = ["Off", "2", "3"];
pub static LEVELS: [&str; 9] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];
/// CPU Level: Off (the game's own level), then the nine levels of `LEVELS`.
pub static CPU_LEVELS: [&str; 10] = ["Off", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
/// Yes / No / Random over a `[True, False]` weighted row: `Random` is both set.
pub static YES_NO_RANDOM: [&str; 3] = ["Yes", "No", "Random"];

/// The eleven directions, array order (SPEC-core-02 §2.7).
pub static DIRECTIONS: [&str; 11] =
    ["Out", "Up Out", "Up", "Up In", "In", "Down In", "Down", "Down Out", "Neutral", "Left", "Right"];

/// The seven swap-group switches (MENU-DRAFT §2): each is its own On/Off setting.
pub static SWAP_GROUPS: [&str; 7] = [
    "Ice Climbers pair bank", "Original fighter bank", "Solo Popo groups", "Ice Climbers pair groups",
    "PT / Aegis groups", "Ice Climbers pair group", "Mii groups",
];
/// The setting ids of `SWAP_GROUPS`, same order.
pub static SWAP_GROUP_IDS: [&str; 7] = [
    "cycle_preload_ic_pair_bank", "cycle_preload_original_bank", "cycle_preload_solo_popo_groups",
    "cycle_preload_ic_pair_groups", "cycle_preload_native_groups", "cycle_preload_pair_groups",
    "cycle_preload_mii_groups",
];

/// The 13 situations of the override editor, with the setting id of each (SPEC-core-02 §2.7).
pub static SITUATIONS: [(&str, &str); 13] = [
    ("Ledge Neutral Getup", "ledge_neutral_override"),
    ("Ledge Roll", "ledge_roll_override"),
    ("Ledge Jump", "ledge_jump_override"),
    ("Ledge Attack", "ledge_attack_override"),
    ("Tech Action", "tech_action_override"),
    ("Clatter", "clatter_override"),
    ("Tumble", "tumble_override"),
    ("Hitstun", "hitstun_override"),
    ("Parry", "parry_override"),
    ("Shieldstun", "shieldstun_override"),
    ("Footstool", "footstool_override"),
    ("Landing", "landing_override"),
    ("Ledge Trump", "trump_override"),
];

/// The ten chord rows of the *Button chords* editor, with their setting ids (rc.8 added the last two:
/// `docs/design/rc8-binds.md`).
pub static CHORD_ROWS: [(&str, &str); 10] = [
    ("Save State Save", "save_state_save"),
    ("Save State Load", "save_state_load"),
    ("Record", "input_record"),
    ("Play on CPU", "input_playback"),
    ("Play on Player 1", "play_player"),
    ("Recording pop-up", "recording_popup"),
    ("Cycle Player", "player_cycle"),
    ("Cycle CPU", "cpu_cycle"),
    ("Input display", "input_display_toggle"),
    ("Game speed", "game_speed"),
];
