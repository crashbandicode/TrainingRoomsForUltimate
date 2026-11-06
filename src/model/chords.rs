//! Button chords (SPEC-input-04): "hold any one button and press the others" on P1's controller, the
//! logical-to-physical button mapping by controller style, the request flags consumers pull, the Plus
//! rule and the glyph table. Pure: the runtime hands in the controller's bit sets once per P1 poll.
//!
//! Decisions applied: B8 (SPEC-input-04 question 1) every chord needs at least one just-pressed button,
//! so a held one-button chord fires once and not every poll; question 2 (recommendation) no request is
//! raised while any input owner is active or its 15-frame grace runs; N7 the permanent `B + D-pad Up`
//! and the two INI shortcuts (`menu_shortcuts`) open the menu with the same edge rule.

use super::menu::chord::{Chord, BUTTONS};
use super::menu_shortcuts::MenuShortcuts;

/// The logical buttons, in the order of `Chord` and the chord settings' arrays.
pub mod logical {
    pub const A: u8 = 0;
    pub const B: u8 = 1;
    pub const X: u8 = 2;
    pub const Y: u8 = 3;
    pub const L: u8 = 4;
    pub const R: u8 = 5;
    pub const ZL: u8 = 6;
    pub const ZR: u8 = 7;
    pub const DPAD_UP: u8 = 8;
    pub const DPAD_DOWN: u8 = 9;
    pub const DPAD_LEFT: u8 = 10;
    pub const DPAD_RIGHT: u8 = 11;
    pub const PLUS: u8 = 12;
    pub const MINUS: u8 = 13;
    pub const STICK_L: u8 = 14;
    pub const STICK_R: u8 = 15;
}

/// The game's 32-bit button set, bit order of the `Controller` (SPEC-input-04 §3.2).
pub mod bit {
    pub const DPAD_UP: u32 = 1 << 0;
    pub const DPAD_RIGHT: u32 = 1 << 1;
    pub const DPAD_DOWN: u32 = 1 << 2;
    pub const DPAD_LEFT: u32 = 1 << 3;
    pub const X: u32 = 1 << 4;
    pub const A: u32 = 1 << 5;
    pub const B: u32 = 1 << 6;
    pub const Y: u32 = 1 << 7;
    pub const L: u32 = 1 << 8;
    pub const R: u32 = 1 << 9;
    pub const ZL: u32 = 1 << 10;
    pub const ZR: u32 = 1 << 11;
    pub const LEFT_SL: u32 = 1 << 12;
    pub const LEFT_SR: u32 = 1 << 13;
    pub const RIGHT_SL: u32 = 1 << 14;
    pub const RIGHT_SR: u32 = 1 << 15;
    pub const STICK_L: u32 = 1 << 16;
    pub const STICK_R: u32 = 1 << 17;
    pub const L_UP: u32 = 1 << 20;
    pub const L_RIGHT: u32 = 1 << 21;
    pub const L_DOWN: u32 = 1 << 22;
    pub const L_LEFT: u32 = 1 << 23;
    pub const R_UP: u32 = 1 << 24;
    pub const R_RIGHT: u32 = 1 << 25;
    pub const R_DOWN: u32 = 1 << 26;
    pub const R_LEFT: u32 = 1 << 27;
    pub const PLUS: u32 = 1 << 18;
    pub const MINUS: u32 = 1 << 19;
    pub const REAL_DIGITAL_L: u32 = 1 << 28;
    pub const REAL_DIGITAL_R: u32 = 1 << 29;
    /// The four Plus bits are cleared in `current`, `previous`, `just_down` and `just_release`.
    pub const PLUS_MASK: u32 = PLUS;
}

/// A raw 32-bit button set of the game's `Controller` (SPEC-input-04 section 3.1; the bit order is SPEC-core-03 section 4): the reads the
/// Replay Window's controls use, with the d-pad and both sticks' digital direction bits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Buttons(pub u32);

impl Buttons {
    const fn has(self, mask: u32) -> bool {
        self.0 & mask != 0
    }
    pub const fn a(self) -> bool { self.has(bit::A) }
    pub const fn b(self) -> bool { self.has(bit::B) }
    pub const fn x(self) -> bool { self.has(bit::X) }
    pub const fn y(self) -> bool { self.has(bit::Y) }
    pub const fn l(self) -> bool { self.has(bit::L) }
    pub const fn r(self) -> bool { self.has(bit::R) }
    pub const fn zl(self) -> bool { self.has(bit::ZL) }
    pub const fn zr(self) -> bool { self.has(bit::ZR) }
    pub const fn dpad_up(self) -> bool { self.has(bit::DPAD_UP) }
    pub const fn dpad_down(self) -> bool { self.has(bit::DPAD_DOWN) }
    pub const fn dpad_left(self) -> bool { self.has(bit::DPAD_LEFT) }
    pub const fn dpad_right(self) -> bool { self.has(bit::DPAD_RIGHT) }
    pub const fn real_digital_l(self) -> bool { self.has(bit::REAL_DIGITAL_L) }
    pub const fn real_digital_r(self) -> bool { self.has(bit::REAL_DIGITAL_R) }
    pub const fn l_up(self) -> bool { self.has(bit::L_UP) }
    pub const fn l_right(self) -> bool { self.has(bit::L_RIGHT) }
    pub const fn l_down(self) -> bool { self.has(bit::L_DOWN) }
    pub const fn l_left(self) -> bool { self.has(bit::L_LEFT) }
    pub const fn r_up(self) -> bool { self.has(bit::R_UP) }
    pub const fn r_right(self) -> bool { self.has(bit::R_RIGHT) }
    pub const fn r_down(self) -> bool { self.has(bit::R_DOWN) }
    pub const fn r_left(self) -> bool { self.has(bit::R_LEFT) }
}

/// Controller field offsets (SPEC-input-04 §3.1).
pub mod field {
    pub const CURRENT_BUTTONS: usize = 0x8;
    pub const PREVIOUS_BUTTONS: usize = 0xc;
    pub const JUST_DOWN: usize = 0x88;
    pub const JUST_RELEASE: usize = 0x8c;
    pub const STYLE: usize = 0x9c;
}

/// Controller style (`Controller::style`, SPEC-input-04 §3.3). Only the GameCube differs.
pub const STYLE_GAMECUBE: u32 = 7;

/// The logical buttons a 32-bit button set holds under `style` (§3.3). The same mapping serves
/// `current_buttons` (held) and `just_down` (pressed).
pub fn logical_set(style: u32, bits: u32) -> u16 {
    let any = |mask: u32| bits & mask != 0;
    let gc = style == STYLE_GAMECUBE;
    let mut out = 0u16;
    let mut set = |button: u8, on: bool| {
        if on {
            out |= 1 << button;
        }
    };
    set(logical::A, any(bit::A));
    set(logical::B, any(bit::B));
    set(logical::X, any(bit::X));
    set(logical::Y, any(bit::Y));
    // L never exists on a GameCube pad; R is its Z, which arrives as `zr`.
    set(logical::L, !gc && any(bit::L));
    set(logical::R, if gc { any(bit::ZR) } else { any(bit::R) });
    set(logical::ZL, if gc { any(bit::L | bit::REAL_DIGITAL_L) } else { any(bit::ZL | bit::LEFT_SL | bit::RIGHT_SL) });
    set(logical::ZR, if gc { any(bit::R | bit::REAL_DIGITAL_R) } else { any(bit::ZR | bit::LEFT_SR | bit::RIGHT_SR) });
    set(logical::DPAD_UP, any(bit::DPAD_UP));
    set(logical::DPAD_DOWN, any(bit::DPAD_DOWN));
    set(logical::DPAD_LEFT, any(bit::DPAD_LEFT));
    set(logical::DPAD_RIGHT, any(bit::DPAD_RIGHT));
    set(logical::PLUS, any(bit::PLUS));
    // Minus has no GameCube mapping (a hardware limit: the GameCube pad has no Minus).
    set(logical::MINUS, !gc && any(bit::MINUS));
    set(logical::STICK_L, any(bit::STICK_L));
    set(logical::STICK_R, any(bit::STICK_R));
    out
}

/// One poll's controller sample in logical buttons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sample {
    pub held: u16,
    pub pressed: u16,
}

impl Sample {
    pub fn of(style: u32, current: u32, just_down: u32) -> Self {
        Self { held: logical_set(style, current), pressed: logical_set(style, just_down) }
    }
}

/// Whether `chord` passes on `sample` (§2.2 with the B8 edge rule): there is a held button `H` in the
/// chord such that every other button of the chord was pressed on this poll, and at least one button of
/// the chord was pressed. An empty chord never passes; extra buttons do not disqualify.
pub fn passes(chord: Chord, sample: Sample) -> bool {
    let set = chord.0;
    if set == 0 || set & sample.pressed == 0 {
        return false;
    }
    (0..BUTTONS as u8).any(|h| {
        let hb = 1u16 << h;
        set & hb != 0 && sample.held & hb != 0 && (set & !hb) & !sample.pressed == 0
    })
}

/// A request a consumer pulls (SPEC-input-04 §4 "pending-request flags").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    SaveState,
    LoadState,
    InputRecord,
    InputPlayback,
    CpuCycle,
    PlayerCycle,
    OpenMenu,
    PlayPlayer,
    RecordingPopup,
}

impl Request {
    const fn bit(self) -> u16 {
        1u16 << self as u8
    }
}

/// The configured chords.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChordSet {
    pub save_state: Chord,
    pub load_state: Chord,
    pub input_record: Chord,
    pub input_playback: Chord,
    pub cpu_cycle: Chord,
    pub player_cycle: Chord,
    pub play_player: Chord,
    pub recording_popup: Chord,
    pub shortcuts: MenuShortcuts,
}

impl ChordSet {
    /// The factory chords: ZL + D-pad Down/Up (save/load), ZR + D-pad Down/Up (record/play on CPU),
    /// ZL + D-pad Left/Right (cycle CPU/player), ZR + D-pad Left/Right (play on Player 1 / recording
    /// pop-up), and the default shortcuts.
    pub const fn factory() -> Self {
        const fn two(a: u8, b: u8) -> Chord {
            Chord((1 << a) | (1 << b))
        }
        Self {
            save_state: two(logical::ZL, logical::DPAD_DOWN),
            load_state: two(logical::ZL, logical::DPAD_UP),
            input_record: two(logical::ZR, logical::DPAD_DOWN),
            input_playback: two(logical::ZR, logical::DPAD_UP),
            cpu_cycle: two(logical::ZL, logical::DPAD_LEFT),
            player_cycle: two(logical::ZL, logical::DPAD_RIGHT),
            play_player: two(logical::ZR, logical::DPAD_LEFT),
            recording_popup: two(logical::ZR, logical::DPAD_RIGHT),
            shortcuts: super::menu_shortcuts::DEFAULT_SHORTCUTS,
        }
    }

    /// The eight chord rows of a settings snapshot. The shortcuts are kept from `self` (they live in
    /// the INI, not the settings). A snapshot without `play_player` or `recording_popup` keeps those
    /// two factory chords.
    pub fn with_rows(self, snapshot: &super::settings::Snapshot) -> Self {
        let row = |id: &str, fallback: Chord| snapshot.get(id).map_or(fallback, Chord::from_row);
        Self {
            save_state: row("save_state_save", self.save_state),
            load_state: row("save_state_load", self.load_state),
            input_record: row("input_record", self.input_record),
            input_playback: row("input_playback", self.input_playback),
            cpu_cycle: row("cpu_cycle", self.cpu_cycle),
            player_cycle: row("player_cycle", self.player_cycle),
            play_player: row("play_player", self.play_player),
            recording_popup: row("recording_popup", self.recording_popup),
            shortcuts: self.shortcuts,
        }
    }
}

impl Default for ChordSet {
    fn default() -> Self {
        Self::factory()
    }
}

/// The request flags and the detection step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Detector {
    pending: u16,
}

impl Detector {
    pub const fn new() -> Self {
        Self { pending: 0 }
    }

    /// One P1 poll. `blocked` is true while any input owner other than this stage's own is active (the
    /// vanilla pause menu, the plugin menu and its 15-frame grace, a modal panel, the replay editor, the
    /// native waiting-room UI): no request is raised then, and requests already pending stay
    /// (SPEC-input-04 §5.3). Returns the set of chords that passed this poll as a bit set of
    /// `Request`s for the runtime's diagnostics.
    pub fn poll(&mut self, chords: &ChordSet, sample: Sample, blocked: bool) -> u16 {
        if blocked {
            return 0;
        }
        let mut passed = 0u16;
        for (request, chord) in [
            (Request::SaveState, chords.save_state),
            (Request::LoadState, chords.load_state),
            (Request::InputRecord, chords.input_record),
            (Request::InputPlayback, chords.input_playback),
            (Request::CpuCycle, chords.cpu_cycle),
            (Request::PlayerCycle, chords.player_cycle),
            (Request::PlayPlayer, chords.play_player),
            (Request::RecordingPopup, chords.recording_popup),
        ] {
            if self.pending & request.bit() == 0 && passes(chord, sample) {
                self.pending |= request.bit();
                passed |= request.bit();
            }
        }
        let open = chords.shortcuts.passes(|b| sample.held & (1 << b) != 0, |b| sample.pressed & (1 << b) != 0);
        if self.pending & Request::OpenMenu.bit() == 0 && open {
            self.pending |= Request::OpenMenu.bit();
            passed |= Request::OpenMenu.bit();
        }
        passed
    }

    /// `combo_passes`: whether the request is pending; clears it.
    pub fn consume(&mut self, request: Request) -> bool {
        let was = self.pending & request.bit() != 0;
        self.pending &= !request.bit();
        was
    }

    /// Observe without consuming (the character loader asks whether P1 is mid-request).
    /// The CPU cycle chord passes in `sample`: the controller that operates the CPU (native CPU Behavior = Control) reads its own
    /// sample. Stateless: no pending flag is touched, so this never mixes with Player 1's chord detection.
    pub fn operator_cycle(chords: &ChordSet, sample: Sample) -> bool {
        passes(chords.cpu_cycle, sample)
    }

    pub fn is_pending(&self, request: Request) -> bool {
        self.pending & request.bit() != 0
    }

    pub fn has_pending_request(&self) -> bool {
        self.pending != 0
    }

    /// Session clear: scene departure, hot swap, the waiting-room UI becoming busy.
    pub fn clear_session(&mut self) {
        self.pending = 0;
    }
}

/// Whether the Plus bits are cleared from this poll's controller (SPEC-input-04 §4, §6.1): never for a
/// plain Plus (it goes to the game's pause), only while a modal panel owns the input or when an INI
/// shortcut that includes Plus was accepted this poll.
pub const fn clear_plus_bits(modal_owner: bool, plus_shortcut_accepted: bool) -> bool {
    modal_owner || plus_shortcut_accepted
}

/// Whether an accepted OpenMenu this poll came through a shortcut that contains Plus (the pause press
/// is then consumed).
pub fn plus_shortcut_accepted(chords: &ChordSet, sample: Sample) -> bool {
    let plus = Chord(1 << logical::PLUS);
    chords.shortcuts.slots().iter().any(|c| c.0 & plus.0 != 0 && passes(*c, sample))
}

/// The font glyph (a private-use code point of the game font) of a logical button for a style
/// (SPEC-input-04 §3.4); `None` where the GameCube pad has none.
pub fn glyph(button: u8, style: u32) -> Option<u16> {
    use logical::*;
    let gc = style == STYLE_GAMECUBE;
    Some(match (button, gc) {
        (A, _) => 0xE0E0,
        // Upstream has no distinct B glyph; it shows the A glyph.
        (B, _) => 0xE0E0,
        (X, false) => 0xE0E2,
        (X, true) => 0xE206,
        (Y, false) => 0xE0E3,
        (Y, true) => 0xE207,
        (L, false) => 0xE0E4,
        (L, true) => return None,
        (R, false) => 0xE0E5,
        (R, true) => 0xE205,
        (ZL, false) => 0xE0E6,
        (ZL, true) => 0xE204,
        (ZR, false) => 0xE0E7,
        (ZR, true) => 0xE208,
        (DPAD_UP, false) => 0xE079,
        (DPAD_DOWN, false) => 0xE07A,
        (DPAD_LEFT, false) => 0xE07B,
        (DPAD_RIGHT, false) => 0xE07C,
        (DPAD_UP, true) => 0xE209,
        (DPAD_DOWN, true) => 0xE20A,
        (DPAD_LEFT, true) => 0xE20B,
        (DPAD_RIGHT, true) => 0xE20C,
        (PLUS, false) => 0xE0EF,
        (PLUS, true) => 0xE20D,
        (MINUS, false) => 0xE0F0,
        (MINUS, true) => return None,
        (STICK_L, false) => 0xE104,
        (STICK_R, false) => 0xE105,
        _ => return None,
    })
}

/// Whether a chord cannot pass on this style (the menu greys the row: SPEC-input-04 question 3).
pub fn unusable_on(chord: Chord, style: u32) -> bool {
    style == STYLE_GAMECUBE && chord.0 & ((1 << logical::L) | (1 << logical::MINUS)) != 0
}

#[cfg(test)]
mod tests {
    use super::logical::{self, A, B, DPAD_LEFT, DPAD_RIGHT, DPAD_UP, ZR};
    use super::{passes, ChordSet, Detector, Request, Sample};
    use crate::model::factory::DEFS;
    use crate::model::menu::chord::Chord;
    use crate::model::menu::options::CHORD_ROWS;
    use crate::model::menu_shortcuts::{DEFAULT_OPEN_MENU_CONFIG, DEFAULT_SHORTCUTS};
    use crate::model::settings::{self, Snapshot};

    fn bits(buttons: &[u8]) -> u16 {
        buttons.iter().fold(0u16, |set, button| set | (1 << button))
    }

    fn sample(held: &[u8], pressed: &[u8]) -> Sample {
        Sample { held: bits(held), pressed: bits(pressed) }
    }

    fn row(at: &[usize]) -> [u8; 16] {
        let mut out = [0u8; 16];
        for index in at {
            out[*index] = 1;
        }
        out
    }

    fn factory_row(id: &str) -> &[u8] {
        DEFS.iter().find(|def| def.id == id).unwrap_or_else(|| panic!("{id} missing")).default
    }

    #[test]
    fn the_eight_chords_keep_todays_defaults() {
        let eight: [(&str, [usize; 2], &str); 8] = [
            ("save_state_save", [6, 9], "Save State Save"),
            ("save_state_load", [6, 8], "Save State Load"),
            ("input_record", [7, 9], "Record"),
            ("input_playback", [7, 8], "Play on CPU"),
            ("play_player", [7, 10], "Play on Player 1"),
            ("recording_popup", [7, 11], "Recording pop-up"),
            ("player_cycle", [6, 11], "Cycle Player"),
            ("cpu_cycle", [6, 10], "Cycle CPU"),
        ];
        for (id, at, label) in eight {
            assert_eq!(factory_row(id), row(&at), "{id}");
            let (shown, row_id) = CHORD_ROWS.into_iter().find(|(_, row_id)| *row_id == id).unwrap();
            assert_eq!((shown, row_id), (label, id));
        }
        assert_eq!(CHORD_ROWS.len(), eight.len());
        let set = ChordSet::factory();
        assert_eq!(set.save_state, Chord::from_row(factory_row("save_state_save")));
        assert_eq!(set.load_state, Chord::from_row(factory_row("save_state_load")));
        assert_eq!(set.input_record, Chord::from_row(factory_row("input_record")));
        assert_eq!(set.input_playback, Chord::from_row(factory_row("input_playback")));
        assert_eq!(set.play_player, Chord::from_row(factory_row("play_player")));
        assert_eq!(set.recording_popup, Chord::from_row(factory_row("recording_popup")));
        assert_eq!(set.player_cycle, Chord::from_row(factory_row("player_cycle")));
        assert_eq!(set.cpu_cycle, Chord::from_row(factory_row("cpu_cycle")));
        assert_eq!(set.shortcuts, DEFAULT_SHORTCUTS);
        assert_eq!(set.shortcuts.slots()[0], Chord::DEFAULT_SHORTCUT_1);
        assert!(set.shortcuts.slots()[1].is_empty());
        assert_eq!(DEFAULT_OPEN_MENU_CONFIG, Chord(bits(&[B, DPAD_UP])));
        let mut detector = Detector::new();
        detector.poll(&set, sample(&[B, DPAD_UP], &[B]), false);
        assert!(detector.consume(Request::OpenMenu));
        let mut detector = Detector::new();
        detector.poll(&set, sample(&[logical::MINUS], &[logical::MINUS]), false);
        assert!(detector.consume(Request::OpenMenu));
    }

    #[test]
    fn old_settings_without_the_new_chords_still_load() {
        let text = r#"{
            "menu": {
                "save_state_save": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,1,0],
                "save_state_load": [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1],
                "input_record": [1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                "input_playback": [0,1,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
                "cpu_cycle": [0,0,1,0,0,0,0,0,0,0,0,0,0,0,0,0],
                "player_cycle": [0,0,0,1,0,0,0,0,0,0,0,0,0,0,0,0]
            }
        }"#;
        let loaded = settings::parse(text);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let factory = Snapshot::factory();
        let menu = &loaded.settings.menu;
        assert_eq!(menu.get("play_player"), factory.get("play_player"));
        assert_eq!(menu.get("recording_popup"), factory.get("recording_popup"));
        assert_eq!(menu.get("play_player"), Some(row(&[7usize, 10]).as_slice()));
        assert_eq!(menu.get("recording_popup"), Some(row(&[7usize, 11]).as_slice()));
        assert_eq!(menu.get("save_state_save"), Some(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0][..]));
        assert_eq!(menu.get("save_state_load"), Some(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1][..]));
        assert_eq!(menu.get("input_record"), Some(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0][..]));
        assert_eq!(menu.get("input_playback"), Some(&[0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0][..]));
        assert_eq!(menu.get("cpu_cycle"), Some(&[0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0][..]));
        assert_eq!(menu.get("player_cycle"), Some(&[0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0][..]));
        assert_eq!(loaded.settings.defaults.get("play_player"), factory.get("play_player"));
        assert_eq!(loaded.settings.defaults.get("recording_popup"), factory.get("recording_popup"));
        let chords = ChordSet::factory().with_rows(menu);
        assert_eq!(chords.play_player, ChordSet::factory().play_player);
        assert_eq!(chords.recording_popup, ChordSet::factory().recording_popup);
        assert_eq!(chords.save_state, Chord::from_row(menu.get("save_state_save").unwrap()));
        assert_ne!(chords.save_state, ChordSet::factory().save_state);
    }

    #[test]
    fn the_new_chords_raise_their_requests() {
        let set = ChordSet::factory();
        let play = sample(&[ZR, DPAD_LEFT], &[DPAD_LEFT]);
        let popup = sample(&[ZR, DPAD_RIGHT], &[DPAD_RIGHT]);
        assert!(passes(set.play_player, play));
        assert!(passes(set.recording_popup, popup));
        assert!(!passes(set.play_player, popup));
        assert!(!passes(set.input_playback, play), "ZR + Left is not Play on CPU");
        assert!(!passes(set.input_record, popup), "ZR + Right is not Record");
        assert!(!passes(set.play_player, sample(&[ZR, DPAD_LEFT], &[])));
        assert!(!passes(set.play_player, sample(&[DPAD_LEFT], &[DPAD_LEFT])));
        assert!(passes(set.play_player, sample(&[ZR, DPAD_LEFT, A], &[DPAD_LEFT])));
        assert!(passes(set.recording_popup, sample(&[ZR, DPAD_RIGHT], &[ZR])));

        let mut detector = Detector::new();
        assert_eq!(detector.poll(&set, play, true), 0);
        assert_eq!(detector.poll(&set, play, false), Request::PlayPlayer.bit());
        assert!(detector.consume(Request::PlayPlayer));
        assert!(!detector.consume(Request::PlayPlayer));
        assert_eq!(detector.poll(&set, sample(&[ZR, DPAD_LEFT], &[]), false), 0);
        assert_eq!(detector.poll(&set, popup, false), Request::RecordingPopup.bit());
        assert!(detector.consume(Request::RecordingPopup));
    }
}
