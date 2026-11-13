//! The menu's content: the pages, groups and rows of MENU-DRAFT §2 (draft 9), as static tables. Each
//! row is bound to its setting id in the factory table (`model::factory`); a row whose value is not
//! a plain settings array (a grid, a list, an INI file) names the key or file it edits and is opened
//! as a sub-screen. Everything is `'static`: the model allocates nothing to know its layout.

use super::options as o;

/// The pages, in ring order. `Developer` is in the ring only while `developer.ini` enables it (N1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Cpu,
    Saves,
    Recording,
    FightersStages,
    DisplayControls,
    Developer,
}

impl Page {
    /// The ring, with the hidden page last.
    pub const ALL: [Page; 6] =
        [Page::Cpu, Page::Saves, Page::Recording, Page::FightersStages, Page::DisplayControls, Page::Developer];

    /// The stable id written to `last_page`.
    pub const fn id(self) -> &'static str {
        match self {
            Page::Cpu => "cpu",
            Page::Saves => "saves",
            Page::Recording => "recording",
            Page::FightersStages => "fighters_stages",
            Page::DisplayControls => "display_controls",
            Page::Developer => "developer",
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Page::Cpu => "CPU",
            Page::Saves => "Saves",
            Page::Recording => "Recording",
            Page::FightersStages => "Fighters & Stages",
            Page::DisplayControls => "Display & Controls",
            Page::Developer => "Developer",
        }
    }

    pub fn from_id(id: &str) -> Option<Page> {
        Page::ALL.into_iter().find(|page| page.id() == id)
    }

    pub const fn index(self) -> usize {
        self as usize
    }
}

/// A section header on a page. An `advanced` group is collapsed until the player opens it.
#[derive(Clone, Copy, Debug)]
pub struct Group {
    pub page: Page,
    pub title: &'static str,
    pub advanced: bool,
}

const fn g(page: Page, title: &'static str) -> Group {
    Group { page, title, advanced: false }
}

pub static GROUPS: [Group; 20] = [
    g(Page::Cpu, "Reactions"),
    g(Page::Cpu, "Defence"),
    g(Page::Cpu, "Ledge & Tech"),
    g(Page::Cpu, "Shield"),
    g(Page::Cpu, "Attacks & Movement"),
    g(Page::Cpu, "Player 1 control"),
    g(Page::Saves, "Save states"),
    g(Page::Saves, "Setup library"),
    // Recording redesign revision 8.2: the left column. The two library groups have no setting rows.
    g(Page::Recording, "Save states & Macros"),
    g(Page::Recording, "Combos & Sequences"),
    g(Page::Recording, "Replays"),
    g(Page::Recording, "Options"),
    g(Page::FightersStages, "Player 1 and CPU"),
    g(Page::FightersStages, "Extras"),
    g(Page::FightersStages, "Stages"),
    Group { page: Page::FightersStages, title: "Loading", advanced: true },
    g(Page::DisplayControls, "On screen"),
    g(Page::DisplayControls, "Controls"),
    g(Page::Developer, "Developer"),
    // rc.6: appended so no group index moves; the menu shows a page's groups in index order, so it follows "Player 1 control".
    g(Page::Cpu, "Level"),
];

// Group indices, for the rows below.
const REACT: u8 = 0;
const DEFENCE: u8 = 1;
const LEDGE: u8 = 2;
const SHIELD: u8 = 3;
const ATTACKS: u8 = 4;
const P1: u8 = 5;
const STATES: u8 = 6;
const SETUPS: u8 = 7;
/// Save states & Macros. No setting rows: the library screen fills it.
pub const SAVES_MACROS: u8 = 8;
/// Combos & Sequences. Hidden in the CPU view. No setting rows.
pub const COMBOS_SEQ: u8 = 9;
const REPLAYS: u8 = 10;
const OPTIONS: u8 = 11;
const PEOPLE: u8 = 12;
const EXTRAS: u8 = 13;
const STAGES: u8 = 14;
const LOADING: u8 = 15;
const SCREEN: u8 = 16;
const CONTROLS: u8 = 17;
const DEV: u8 = 18;
const LEVEL: u8 = 19;

/// The sub-screens that edit something the model does not own the content of (the fork's grids,
/// lists and panels arrive with package 6; the libraries and the replay window with package 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Editor {
    /// One situation (dropdown of 13) and its set of actions.
    SituationOverrides,
    /// The six chord rows.
    ButtonChords,
    /// Menu shortcut 1 (`0`) or 2 (`1`): the chord editor writing `button_config.ini` (N7).
    ShortcutSlot(u8),
    PortraitPlayer,
    PortraitCpu,
    CharacterCycles,
    KirbyHats,
    GiveItem,
    /// Mii Brawler (`0`), Swordfighter (`1`), Gunner (`2`).
    MiiList(u8),
    StageGrid,
    /// Change Stage: the stage grid in Direct mode (`GridKind::StageChange`).
    ChangeStage,
    SetupSave,
    SetupLibrary,
    NamedLibrary,
    SaveSlotToLibrary,
    EditPlacement,
    ReplayWindow,
    /// The combo helper (SPEC-input-07 §6): the draft name, saving the last combo, the saved combos, the target.
    ComboName,
    ComboSave,
    ComboLibrary,
    ComboTarget,
}

/// A read-only row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Info {
    /// `B + D-pad Up`, fixed.
    PermanentShortcut,
    /// Shown instead of the two fallback rows while the standalone CPU pilot is on the card.
    PilotControl,
}

/// How a row is edited.
#[derive(Clone, Copy, Debug)]
pub enum Control {
    /// `[On, Off]`; A flips it.
    Toggle,
    /// One option of a list: Left / Right cycle in place (wrapping), A opens the list (Q2).
    Dropdown(&'static [&'static str]),
    /// Yes / No / Random over a weighted `[True, False]` row; Random is both set.
    YesNoRandom,
    /// One value of an arithmetic run (`first`, `first + step`, ..): Left / Right step it (clamped), A
    /// opens the list.
    Number { first: u16, step: u16 },
    /// A set edited in a popup checklist.
    MultiSelect(&'static [&'static str]),
    /// A checklist and a range over the same values (N3); the other greys while one holds a value.
    MultiSelectRange { first: u16, step: u16, range_id: &'static str },
    /// Two handles, `lower <= upper`, greyed unless the paired dropdown reads Random.
    Slider,
    /// Chips: 3 x 3 relative to the opponent plus Left / Right; `single` = exactly one chip.
    DirectionGrid { single: bool },
    /// The seven swap-group switches in one checklist.
    SwapGroups,
    /// `replay_capture.ini`'s switch (Q7): an INI-backed toggle, not a settings row.
    ReplayCapture,
    Editor(Editor),
    Info(Info),
}

/// When a row exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vis {
    Always,
    /// Only while the standalone CPU pilot is absent (Q11).
    PilotAbsent,
    /// Only while the standalone CPU pilot is present (Q11).
    PilotPresent,
}

/// One row of the layout.
#[derive(Clone, Copy, Debug)]
pub struct Row {
    /// Index into `GROUPS`.
    pub group: u8,
    pub title: &'static str,
    /// The setting id (or the key or file the editor edits); empty when the row has none.
    pub id: &'static str,
    pub control: Control,
    pub vis: Vis,
    /// Tagged *advanced* next to its label (N4); only a group collapses.
    pub advanced: bool,
    /// The two-line help panel: lines separated by `\n`.
    pub help: &'static str,
}

const fn r(group: u8, title: &'static str, id: &'static str, control: Control, help: &'static str) -> Row {
    Row { group, title, id, control, vis: Vis::Always, advanced: false, help }
}

const fn delay(group: u8, title: &'static str, id: &'static str, step: u16, range_id: &'static str, help: &'static str) -> Row {
    r(group, title, id, Control::MultiSelectRange { first: 0, step, range_id }, help)
}

const TOGGLE: Control = Control::Toggle;

/// The rows in page order, then group order, then the order they appear in (MENU-DRAFT §2).
pub static ROWS: [Row; 92] = [
    // CPU > Reactions
    r(REACT, "Mash", "mash_state", Control::MultiSelect(&o::ACTIONS),
      "Actions the CPU performs as soon as it can.\nWith several ticked it picks one at random."),
    r(REACT, "Followup", "follow_up", Control::MultiSelect(&o::ACTIONS),
      "Actions the CPU performs after a Mash action.\nWith several ticked it picks one at random."),
    r(REACT, "Mash Triggers", "mash_triggers", Control::MultiSelect(&o::MASH_TRIGGERS),
      "The situations that make the CPU mash.\nTick every situation that should trigger it."),
    r(REACT, "Situation overrides", "", Control::Editor(Editor::SituationOverrides),
      "Pick a situation, then the actions the CPU takes in it.\nIt applies without its Mash Trigger, except Shieldstun."),
    // CPU > Defence
    r(DEFENCE, "Airdodge Direction", "air_dodge_dir", Control::DirectionGrid { single: false },
      "Where the CPU air dodges. In and Out are relative to you.\nNothing selected: the CPU does not choose."),
    r(DEFENCE, "DI Direction", "di_state", Control::DirectionGrid { single: false },
      "Where the CPU directionally influences. In and Out follow you.\nNothing selected: the CPU DIs as the game decides."),
    r(DEFENCE, "SDI Direction", "sdi_state", Control::DirectionGrid { single: false },
      "Where the CPU smash-directionally influences.\nNothing selected: the CPU does not SDI."),
    r(DEFENCE, "SDI Strength", "sdi_strength", Control::Dropdown(&o::STRENGTH),
      "How often the CPU SDIs during hitlag.\nNone turns SDI off."),
    r(DEFENCE, "Clatter Strength", "clatter_strength", Control::Dropdown(&o::STRENGTH),
      "How hard the CPU mashes out of grabs, bury and similar.\nNone: it does not."),
    r(DEFENCE, "Dodge Staling", "stale_dodges", TOGGLE,
      "On: the CPU's dodges go stale like a player's.\nOff: every dodge is fresh."),
    // CPU > Ledge & Tech
    r(LEDGE, "Ledge Options", "ledge_state", Control::MultiSelect(&o::LEDGE_OPTIONS),
      "What the CPU does on the ledge.\nWith several ticked it picks one at random."),
    delay(LEDGE, "Ledge Delay", "ledge_delay", 10, "ledge_delay_range",
          "Frames the CPU waits before its ledge option.\nTick values, or set a minimum and a maximum."),
    r(LEDGE, "Tech Options", "tech_state", Control::MultiSelect(&o::TECH),
      "How the CPU techs a hard landing.\nWith several ticked it picks one at random."),
    r(LEDGE, "Mistech Options", "miss_tech_state", Control::MultiSelect(&o::MISTECH),
      "What the CPU does after it fails to tech.\nWith several ticked it picks one at random."),
    r(LEDGE, "Hide Tech Animations", "tech_hide", TOGGLE,
      "Hides the CPU during a tech, a tech roll and a missed tech,\nso its choice cannot be read. Getups stay visible."),
    // CPU > Shield
    r(SHIELD, "Shield", "shield_state", Control::Dropdown(&o::SHIELD),
      "How the CPU shields. Infinite never breaks, Hold keeps it up,\nConstant keeps shielding without end."),
    r(SHIELD, "Shield Tilt", "shield_tilt", Control::DirectionGrid { single: true },
      "Where the CPU tilts its shield. In and Out are relative to you.\nOne chip, or none for no tilt."),
    delay(SHIELD, "OoS Offset", "oos_offset", 1, "oos_offset_range",
          "Hits the shield takes before the CPU acts out of shield.\nTick values, or set a minimum and a maximum."),
    delay(SHIELD, "Reaction Time", "reaction_time", 1, "reaction_time_range",
          "Frames the CPU waits before acting out of shield.\nTick values, or set a minimum and a maximum."),
    // CPU > Attacks & Movement
    r(ATTACKS, "Attack Angle", "attack_angle", Control::MultiSelect(&o::ATTACK_ANGLE),
      "The angle of the CPU's angled attacks.\nWith several ticked it picks one at random."),
    r(ATTACKS, "Throw", "throw_state", Control::MultiSelect(&o::THROW),
      "What the CPU does after it grabs you.\nWith several ticked it picks one at random."),
    delay(ATTACKS, "Throw Delay", "throw_delay", 5, "throw_delay_range",
          "Frames the CPU holds you before throwing.\nTick values, or set a minimum and a maximum."),
    delay(ATTACKS, "Pummel Delay", "pummel_delay", 5, "pummel_delay_range",
          "Frames after a grab before the CPU pummels. Tick values or a range.\nIt pummels until its throw, or with Throw None until the grab ends."),
    r(ATTACKS, "Falling Aerials", "falling_aerials", Control::YesNoRandom,
      "Whether the CPU's aerials come while it is falling.\nRandom picks either way each time."),
    delay(ATTACKS, "Aerial Delay", "aerial_delay", 1, "aerial_delay_range",
          "Frames the CPU waits before an aerial.\nTick values, or set a minimum and a maximum."),
    r(ATTACKS, "Full Hop", "full_hop", Control::YesNoRandom,
      "Whether the CPU's jumps are full hops or short hops,\naerials included. Random picks either way each time."),
    r(ATTACKS, "Fast Fall", "fast_fall", Control::YesNoRandom,
      "Whether the CPU fast falls during a jump.\nRandom picks either way each time."),
    delay(ATTACKS, "Fast Fall Delay", "fast_fall_delay", 1, "fast_fall_delay_range",
          "Frames the CPU waits before fast falling.\nTick values, or set a minimum and a maximum."),
    r(ATTACKS, "Crouch", "crouch", TOGGLE, "On: the CPU crouches in place.\nOff: it stands."),
    // CPU > Player 1 control (the fallback exists only without the pilot)
    Row { vis: Vis::PilotPresent, ..r(P1, "P1 control", "", Control::Info(Info::PilotControl),
      "The CPU pilot plugin controls Player 1.\nIts settings are on its own menu page.") },
    Row { vis: Vis::PilotAbsent, ..r(P1, "P1 CPU Control", "p1_cpu_control", TOGGLE,
      "On: the CPU's AI plays Player 1 once the menu closes.\nOff: you play Player 1.") },
    Row { vis: Vis::PilotAbsent, ..r(P1, "P1 CPU Level", "p1_cpu_level", Control::Dropdown(&o::LEVELS),
      "The level of the AI that plays Player 1.\nHigher is stronger.") },
    // Saves > Save states
    r(STATES, "Save States", "save_state_enable", TOGGLE,
      "On: the save and load chords work.\nOff: they show that Save States is Off."),
    r(STATES, "Mirroring", "save_state_mirroring", Control::Dropdown(&o::MIRRORING),
      "Flips the loaded state left to right.\nAlternate flips every load; Random flips by chance."),
    r(STATES, "Auto-load on death", "save_state_autoload", TOGGLE,
      "On: a save state loads when anyone dies.\nOff: you load it yourself."),
    r(STATES, "Player damage on load", "save_damage_player", Control::Dropdown(&o::SAVE_DAMAGE),
      "Player 1's damage after a load: the default, the saved\nvalue, or a random value in the range below."),
    r(STATES, "Player damage range", "save_damage_limits_player", Control::Slider,
      "The range a random value for Player 1 is drawn from.\nUsed only when damage on load is Random Value."),
    r(STATES, "CPU damage on load", "save_damage_cpu", Control::Dropdown(&o::SAVE_DAMAGE),
      "The CPU's damage after a load: the default, the saved\nvalue, or a random value in the range below."),
    r(STATES, "CPU damage range", "save_damage_limits_cpu", Control::Slider,
      "The range a random value for the CPU is drawn from.\nUsed only when damage on load is Random Value."),
    r(STATES, "Buffs on load", "buff_state", Control::MultiSelect(&o::BUFFS),
      "Character buffs applied after a load.\nShulk's arts and Wario's wafts: one ticked is picked at random."),
    // Saves > Setup library (operator 2026-11-05: it ships again)
    r(SETUPS, "Save setup to library...", "setup_library_save", Control::Editor(Editor::SetupSave),
      "Keeps what Load would load now as a named setup:\nnormally your bound save. Restore it from the library."),
    r(SETUPS, "Setup library", "", Control::Editor(Editor::SetupLibrary),
      "The saved setups. A binds a setup to the slot and\nrestores it; Y binds it without restoring."),
    // Recording > Replays. The library groups above have no rows (revision 8.2).
    r(REPLAYS, "Ask before switching fighter", "fighter_prompt", TOGGLE,
      "On: a replay, macro, chain or combo drill on another fighter\nasks first. Off: the fighter you have stays, without asking."),
    r(REPLAYS, "Edit placement", "", Control::Editor(Editor::EditPlacement),
      "Moves where the active macro starts after its warp,\nfor the side the Recording pop-up is set to."),
    r(REPLAYS, "Replay window", "", Control::Editor(Editor::ReplayWindow),
      "Saved match captures: pick a fighter and an input window\nto turn into a recording."),
    r(REPLAYS, "Replay capture", "", Control::ReplayCapture,
      "On: capture match inputs when saving a replay.\nThis choice is saved automatically."),
    // Recording > Options. The loop row shows the six counts; the factory value stays length 2.
    r(OPTIONS, "Crop trailing neutral", "recording_crop", TOGGLE,
      "Has no effect on Record: a recording keeps every frame\nup to the stop. Use Crop on a macro's screen to cut it."),
    r(OPTIONS, "Hitstun timing", "hitstun_playback", Control::Dropdown(&o::HITSTUN_PLAYBACK),
      "When playback starts after the CPU is hit."),
    r(OPTIONS, "Mash interrupts playback", "playback_mash", TOGGLE,
      "On: a mash action stops a playing recording.\nOff: the recording plays to its end."),
    r(OPTIONS, "Loop", "playback_loop", Control::Dropdown(&o::LOOP_COUNTS),
      "How many times the active macro plays.\nX cycles this while the cursor is in MACROS."),
    Row { advanced: true, ..r(OPTIONS, "Settings override playback", "settings_override_playback", TOGGLE,
      "Off: a playing recording wins over the DI, SDI, air-dodge,\ntech and throw settings. On: the settings override it.") },
    r(OPTIONS, "Combo helper", "combo_helper", TOGGLE,
      "On: each attempt at the target is graded under the\nframe strip, link by link."),
    r(OPTIONS, "On-time tolerance", "combo_tolerance", Control::Dropdown(&o::COMBO_TOLERANCE),
      "How many frames from the recorded press\nstill count as on time."),
    r(OPTIONS, "After an attempt", "combo_after", Control::Dropdown(&o::COMBO_AFTER),
      "Keep playing, or load the save state to retry\nfrom the same spot."),
    // Fighters & Stages > Player 1 and CPU
    r(PEOPLE, "Player Swapping", "trio_practice", TOGGLE,
      "On: the Player 1 cycle chord changes your fighter.\nOff: only a Player Character pick or a combo's Switch does."),
    r(PEOPLE, "Player Character", "player_character", Control::Editor(Editor::PortraitPlayer),
      "Pick Player 1's fighter from the portrait grid.\nIt loads whether Player Swapping is On or Off."),
    r(PEOPLE, "CPU Character", "cpu_character", Control::Editor(Editor::PortraitCpu),
      "Pick the CPU's fighter from the portrait grid."),
    r(PEOPLE, "Character Cycles", "player_roster", Control::Editor(Editor::CharacterCycles),
      "The order the cycle chords step through, for Player 1\n(red) and the CPU (blue). ZR switches role."),
    r(PEOPLE, "Solo Popo (original Ice Climbers)", "solo_popo_original", TOGGLE,
      "On: Player 1's original Ice Climbers pick is Popo alone."),
    // Fighters & Stages > Extras
    r(EXTRAS, "Kirby Hats", "player_kirby_hats", Control::Editor(Editor::KirbyHats),
      "The hat order Kirby copies, for each role."),
    r(EXTRAS, "Give item", "character_item_grant", Control::Editor(Editor::GiveItem),
      "A picks an item for Player 1. It arrives when the menu\ncloses; a later pick replaces the earlier one."),
    r(EXTRAS, "Mii Brawler", "mii_brawler_config", Control::Editor(Editor::MiiList(0)),
      "The Mii used for the Brawler: a saved Mii, a replay Mii\nor a new one."),
    r(EXTRAS, "Mii Swordfighter", "mii_sword_config", Control::Editor(Editor::MiiList(1)),
      "The Mii used for the Swordfighter: a saved Mii, a replay Mii\nor a new one."),
    r(EXTRAS, "Mii Gunner", "mii_gunner_config", Control::Editor(Editor::MiiList(2)),
      "The Mii used for the Gunner: a saved Mii, a replay Mii\nor a new one."),
    // Fighters & Stages > Stages
    r(STAGES, "Change Stage", "", Control::Editor(Editor::ChangeStage),
      "A on a stage goes there now and closes the menu.\nY picks the stage's form first."),
    r(STAGES, "Lobby Practice", "lobby_practice", TOGGLE,
      "On: practice runs in the waiting room too.\nOff: only in Training."),
    r(STAGES, "Lobby Stages", "lobby_stages", Control::Editor(Editor::StageGrid),
      "The stages, in the order practice steps through them.\nY cycles a form. X switches to Direct: one fixed stage."),
    r(STAGES, "Training Stage Rotation", "training_stage_rotation", TOGGLE,
      "On: Training moves through the Lobby Stages order.\nThe factory list is only the Waiting Room. Off: it stays."),
    r(STAGES, "Stage Hazards", "stage_hazards", TOGGLE,
      "On: stage hazards are active in Training. Off: they are not.\nWith Lobby Practice On, this sets the waiting room's too."),
    // Phase 2 N11 (docs/ROADMAP.md): the waiting room's item spawner.
    r(STAGES, "Lobby Items", "lobby_items", TOGGLE,
      "On: items appear in the waiting room as usual.\nOff: no items appear there."),
    r(STAGES, "Respawn on platform", "respawn_invincibility", TOGGLE,
      "On: respawn on the revival platform, invincible as usual.\nOff: instant respawn. Greyed while a death loads a save state."),
    // Fighters & Stages > Loading (advanced)
    r(LOADING, "Cycle Preload", "cycle_preload_slots", Control::Dropdown(&o::CYCLE_SLOTS),
      "How many fighters per role are kept ready for swaps.\nOff loads each fighter when you swap."),
    r(LOADING, "Preload", "cycle_preload_sources", Control::Dropdown(&o::SOURCES),
      "What is kept ready: fighters, Kirby hats, or both."),
    r(LOADING, "Ready Fighters", "cycle_preload_ready_fighters", Control::Number { first: 2, step: 1 },
      "The total number of fighters kept ready across roles."),
    r(LOADING, "Ready Hats", "cycle_preload_ready_hats", Control::Number { first: 0, step: 1 },
      "How many Kirby hats are kept ready, apart from the fighters.\nA hat of a ready fighter shares its body."),
    r(LOADING, "Ready Fighter Bodies", "cycle_preload_bank_members", Control::Dropdown(&o::BODIES),
      "How many bodies a ready fighter keeps.\nOff, two or three; some groups need three."),
    r(LOADING, "Group Swap", "cycle_preload_group_swap", TOGGLE,
      "The master switch of the group swaps below.\nOff turns every group off."),
    r(LOADING, "Swap groups", "", Control::SwapGroups,
      "Which swaps are done as a group. An entry is greyed,\nwith the reason, while what it needs is off."),
    // Display & Controls > On screen
    r(SCREEN, "HUD", "hud", TOGGLE, "On: the game's HUD shows as usual.\nOff: Training hides damage, pointers and off-screen bubbles."),
    r(SCREEN, "Hitbox Visualization", "hitbox_vis", TOGGLE, "On: hitboxes, grab boxes and hurtboxes (yellow) are drawn.\nOff: nothing is drawn."),
    r(SCREEN, "Frame Advantage", "frame_advantage", TOGGLE,
      "On: the frame advantage after each exchange is shown."),
    r(SCREEN, "Input Display", "input_display_strip", Control::Dropdown(&o::INPUT_DISPLAY),
      "Frame strip: the last second of P1 and CPU frames,\nhow each follow-up linked and the frame advantage."),
    r(SCREEN, "Input Delay", "input_delay", Control::Number { first: 0, step: 1 },
      "Frames Player 1's inputs are held back, 0 to 30.\nChords are not delayed."),
    // Display & Controls > Controls
    r(CONTROLS, "Button chords", "", Control::Editor(Editor::ButtonChords),
      "The chords for save, load, record, playback and the\ncycles. Hold one button and press the others."),
    r(CONTROLS, "L+R+A Reset", "lra_reset", TOGGLE,
      "On: holding L, R and A resets the training room.\nOff: the chord does nothing."),
    r(CONTROLS, "Menu shortcut 1", "", Control::Editor(Editor::ShortcutSlot(0)),
      "A chord that opens this menu. Hold one button and press\nthe others. The change applies at once."),
    r(CONTROLS, "Menu shortcut 2", "", Control::Editor(Editor::ShortcutSlot(1)),
      "A second chord that opens this menu, or disabled.\nThe change applies at once."),
    r(CONTROLS, "Permanent shortcut", "", Control::Info(Info::PermanentShortcut),
      "B and D-pad Up always open this menu.\nThis one cannot be changed."),
    // Developer (hidden unless developer.ini enables it)
    r(DEV, "Preload Samples", "cycle_preload_measurements", TOGGLE,
      "Writes sparse timing and memory samples of preloads\nto the diagnostics file."),
    r(DEV, "Reload developer config", "dev_reload_config", TOGGLE,
      "While on, holding L+R+A on Player 1 re-reads the\ndeveloper config file every frame."),
    r(DEV, "Debug hooks", "dev_debug_hooks", TOGGLE,
      "Turns on the debug watch on the Kirby backshield\nwork float."),
    // CPU > Level (rc.6; its group is the last in `GROUPS`, so its row is the last here)
    r(LEVEL, "CPU Level", "cpu_level", Control::Dropdown(&o::CPU_LEVELS),
      "The level of the CPUs in Training and the lobby.\nOff: the game's own level."),
];

/// Number of `ROWS` of a page that are visible with the pilot absent (and with it present, minus
/// one: the info row replaces the two fallback rows).
#[cfg(test)]
pub fn row_count(page: Page, pilot_present: bool) -> usize {
    ROWS.iter().filter(|row| GROUPS[usize::from(row.group)].page == page && row.visible(pilot_present)).count()
}

impl Row {
    pub fn page(&self) -> Page {
        GROUPS[usize::from(self.group)].page
    }

    /// Whether the row exists for this install.
    pub fn visible(&self, pilot_present: bool) -> bool {
        match self.vis {
            Vis::Always => true,
            Vis::PilotAbsent => !pilot_present,
            Vis::PilotPresent => pilot_present,
        }
    }

    /// The help panel's two lines.
    pub fn help_lines(&self) -> (&'static str, &'static str) {
        self.help.split_once('\n').unwrap_or((self.help, ""))
    }
}
