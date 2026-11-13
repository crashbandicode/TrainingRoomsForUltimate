//! Button chords (SPEC-input-04): "hold any one button and press the others" on P1's controller, the
//! logical-to-physical button mapping by controller style, the request flags consumers pull, the Plus
//! rule and the glyph table. Pure: the runtime hands in the controller's bit sets once per P1 poll.
//!
//! Decisions applied: B8 (SPEC-input-04 question 1) every chord needs at least one just-pressed button,
//! so a held one-button chord fires once and not every poll; question 2 (recommendation) no request is
//! raised while any input owner is active or its 15-frame grace runs; N7 the permanent `B + D-pad Up`
//! and the two INI shortcuts (`menu_shortcuts`) open the menu with the same edge rule.

use super::game_speed::{Hold, HoldEvent};
use super::menu::chord::{Chord, BUTTONS};
use super::menu_shortcuts::MenuShortcuts;
use super::state::refused::{Action, Refused};

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
    #[cfg(test)]
    pub const L_UP: u32 = 1 << 20;
    pub const L_RIGHT: u32 = 1 << 21;
    #[cfg(test)]
    pub const L_DOWN: u32 = 1 << 22;
    pub const L_LEFT: u32 = 1 << 23;
    #[cfg(test)]
    pub const R_UP: u32 = 1 << 24;
    pub const R_RIGHT: u32 = 1 << 25;
    #[cfg(test)]
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
    #[cfg(test)]
    pub const fn l_up(self) -> bool { self.has(bit::L_UP) }
    pub const fn l_right(self) -> bool { self.has(bit::L_RIGHT) }
    #[cfg(test)]
    pub const fn l_down(self) -> bool { self.has(bit::L_DOWN) }
    pub const fn l_left(self) -> bool { self.has(bit::L_LEFT) }
    #[cfg(test)]
    pub const fn r_up(self) -> bool { self.has(bit::R_UP) }
    pub const fn r_right(self) -> bool { self.has(bit::R_RIGHT) }
    #[cfg(test)]
    pub const fn r_down(self) -> bool { self.has(bit::R_DOWN) }
    pub const fn r_left(self) -> bool { self.has(bit::R_LEFT) }
}

/// Controller field offsets (SPEC-input-04 §3.1).
pub mod field {
    pub const CURRENT_BUTTONS: usize = 0x8;
    pub const PREVIOUS_BUTTONS: usize = 0xc;
    /// Left stick x, y, then right stick x, y: f32, ±1.0.
    pub const STICKS: [usize; 4] = [0x10, 0x14, 0x20, 0x24];
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

/// The raw bits of `style` that hold the logical buttons `set`: the inverse of `logical_set`, for clearing a chord's
/// buttons from the controller before the game maps it.
pub fn physical_mask(style: u32, set: u16) -> u32 {
    let gc = style == STYLE_GAMECUBE;
    let has = |button: u8| set & (1 << button) != 0;
    let mut out = 0u32;
    let mut add = |button: u8, bits: u32| {
        if has(button) {
            out |= bits;
        }
    };
    add(logical::A, bit::A);
    add(logical::B, bit::B);
    add(logical::X, bit::X);
    add(logical::Y, bit::Y);
    add(logical::L, if gc { 0 } else { bit::L });
    add(logical::R, if gc { bit::ZR } else { bit::R });
    add(logical::ZL, if gc { bit::L | bit::REAL_DIGITAL_L } else { bit::ZL | bit::LEFT_SL | bit::RIGHT_SL });
    add(logical::ZR, if gc { bit::R | bit::REAL_DIGITAL_R } else { bit::ZR | bit::LEFT_SR | bit::RIGHT_SR });
    add(logical::DPAD_UP, bit::DPAD_UP);
    add(logical::DPAD_DOWN, bit::DPAD_DOWN);
    add(logical::DPAD_LEFT, bit::DPAD_LEFT);
    add(logical::DPAD_RIGHT, bit::DPAD_RIGHT);
    add(logical::PLUS, bit::PLUS);
    add(logical::MINUS, if gc { 0 } else { bit::MINUS });
    add(logical::STICK_L, bit::STICK_L);
    add(logical::STICK_R, bit::STICK_R);
    out
}

/// One poll's controller sample in logical buttons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sample {
    pub held: u16,
    pub pressed: u16,
    /// The controller is a GameCube pad. Online Deluxe reads its two triggers as L and R, which are this plugin's ZL and
    /// ZR there (`logical_set`).
    pub gamecube: bool,
    /// Online Deluxe's window shows a page with rows (`model::deluxe_overlay::rows_page_visible`): they move with D-pad
    /// Up/Down and change with ZR + Left/Right. Its Performance and Debug views have none.
    pub overlay: bool,
}

impl Sample {
    pub fn of(style: u32, current: u32, just_down: u32) -> Self {
        Self {
            held: logical_set(style, current),
            pressed: logical_set(style, just_down),
            gamecube: style == STYLE_GAMECUBE,
            overlay: false,
        }
    }

    /// The same sample with Online Deluxe's window state for this poll.
    pub const fn with_overlay(self, overlay: bool) -> Self {
        Self { overlay, ..self }
    }
}

/// Both triggers: Online Deluxe's L + R on a GameCube pad.
const TRIGGERS: u16 = (1 << logical::ZL) | (1 << logical::ZR);

/// Online Deluxe's L + R chords are being held and `chord` is not one of them: L held on most controllers (rc.8), both
/// triggers held on a GameCube pad (rc.9 C3). A chord that holds those buttons itself is the player's own.
pub fn deluxe_shoulders(chord: Chord, sample: Sample) -> bool {
    let l = 1u16 << logical::L;
    let l_held = sample.held & l != 0 && chord.0 & l == 0;
    let triggers = sample.gamecube && sample.held & TRIGGERS == TRIGGERS && chord.0 & TRIGGERS != TRIGGERS;
    l_held || triggers
}

/// rc.12: whether this poll's press is the Save or the Load chord (it would raise the request if nothing stood it down).
fn save_or_load_press(chords: &ChordSet, sample: Sample) -> Option<Action> {
    [(Action::Save, chords.save_state), (Action::Load, chords.load_state)]
        .into_iter()
        .find(|(_, chord)| passes(*chord, sample))
        .map(|(action, _)| action)
}

/// Online Deluxe 0.10.5's `selection_held`: while one of these is held, D-pad Up/Down move none of its rows (a GameCube
/// pad's triggers arrive as this plugin's ZL and ZR, its Z as R).
const DELUXE_SHOULDERS: u16 = (1 << logical::ZL) | (1 << logical::ZR) | (1 << logical::R) | (1 << logical::L);

/// rc.12 (the console's Save and Load, 2026-11-13): whether an Online Deluxe page with rows acts on this chord's D-pad,
/// by Deluxe 0.10.5's own rules (`overlay.rs`): Up/Down move its row only while none of ZL, ZR, R and L is held;
/// Left/Right change its value only under its edit hold, ZR without ZL (on a GameCube pad its Z, this plugin's R). The
/// chord's own buttons count as held.
pub fn deluxe_rows_take(chord: Chord, sample: Sample) -> bool {
    let held = chord.0 | sample.held;
    let has = |button: u8| held & (1 << button) != 0;
    let up_down = (1u16 << logical::DPAD_UP) | (1 << logical::DPAD_DOWN);
    let left_right = (1u16 << logical::DPAD_LEFT) | (1 << logical::DPAD_RIGHT);
    let edit_hold = if sample.gamecube { has(logical::R) } else { has(logical::ZR) && !has(logical::ZL) };
    (chord.0 & up_down != 0 && held & DELUXE_SHOULDERS == 0) || (chord.0 & left_right != 0 && edit_hold)
}

/// Whether a chord row passes beside Online Deluxe (rc.8 and rc.9, `PLUGIN-SHORTCUT-CONFLICTS.md`):
/// - C4 (narrowed in rc.12): no row whose D-pad Deluxe acts on while it shows a page with rows (`deluxe_rows_take`);
/// - C3: on a GameCube pad, no row with ZL or ZR, and no rc.8 row, while both triggers are held (Deluxe's L + R there);
/// - rc.8 (`rc8_row`, Input display and Game speed): not while L is held (Deluxe's L + R + D-pad Down holds R + Down).
pub fn passes_row(chord: Chord, sample: Sample, rc8_row: bool) -> bool {
    if !passes(chord, sample) || (sample.overlay && deluxe_rows_take(chord, sample)) {
        return false;
    }
    let l = 1u16 << logical::L;
    let gc_triggers = sample.gamecube && sample.held & TRIGGERS == TRIGGERS && chord.0 & TRIGGERS != TRIGGERS;
    if gc_triggers && (rc8_row || chord.0 & TRIGGERS != 0) {
        return false;
    }
    !(rc8_row && sample.held & l != 0 && chord.0 & l == 0)
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

/// rc.8's two rows (Input display, Game speed) pass only while L is not held, unless the chord holds L itself: Online
/// Deluxe's overlay opens with L + R + D-pad Down, which contains the Input display chord R + D-pad Down (the docs lane,
/// 2026-11-11 19:57Z: opening that overlay flipped the input display). rc.9 adds C3 and C4 (`passes_row`).
pub fn passes_without_l(chord: Chord, sample: Sample) -> bool {
    passes_row(chord, sample, true)
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
    /// rc.8: the input display row flips (Off / Frame strip).
    InputDisplay,
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
    /// rc.8: toggles the input display (`docs/design/rc8-binds.md`).
    pub input_display: Chord,
    /// rc.8: the game speed hold (`model::game_speed::Hold`).
    pub game_speed: Chord,
    pub shortcuts: MenuShortcuts,
}

impl ChordSet {
    /// The factory chords: ZL + D-pad Down/Up (save/load), ZR + D-pad Down/Up (record/play on CPU),
    /// ZL + D-pad Left/Right (cycle CPU/player), ZR + D-pad Left/Right (play on Player 1 / recording
    /// pop-up), R + D-pad Down/Up (input display / game speed, rc.8), and the default shortcuts.
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
            input_display: two(logical::R, logical::DPAD_DOWN),
            game_speed: two(logical::R, logical::DPAD_UP),
            shortcuts: super::menu_shortcuts::DEFAULT_SHORTCUTS,
        }
    }

    /// The ten chord rows of a settings snapshot. The shortcuts are kept from `self` (they live in
    /// the INI, not the settings). A snapshot without one of the later rows (`play_player`,
    /// `recording_popup`, rc.8's `input_display_toggle` and `game_speed`) keeps its factory chord.
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
            input_display: row("input_display_toggle", self.input_display),
            game_speed: row("game_speed", self.game_speed),
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
    /// The logical buttons of the chords that passed and are still held: the game does not see them.
    swallow: u16,
    /// What this poll keeps from the game: `swallow` with the buttons released on this poll (their release edge).
    hidden: u16,
    /// rc.8: the game speed hold and what the last poll did to it.
    hold: Hold,
    hold_event: HoldEvent,
    /// rc.12: the Save or Load press this poll did not raise, and why (`take_refused`).
    refused: Option<(Action, Refused)>,
}

impl Detector {
    pub const fn new() -> Self {
        Self { pending: 0, swallow: 0, hidden: 0, hold: Hold::new(), hold_event: HoldEvent::Idle, refused: None }
    }

    /// The Save or Load press the last poll did not raise, and why; once (rc.12, "no silent no-ops").
    pub fn take_refused(&mut self) -> Option<(Action, Refused)> {
        self.refused.take()
    }

    /// What the last poll did to the game speed hold (the runtime opens, steps and closes the pop-up from it).
    pub fn hold_event(&self) -> HoldEvent {
        self.hold_event
    }

    /// The logical buttons this poll keeps from the game: those of a chord that passed, from the poll it passed until
    /// the poll each is released (Eden rc5-opus-final-diag, 2026-11-08: the Play chord's ZR reached the game and P1
    /// shielded for 10 frames). The runtime clears them from the controller before the game maps it.
    pub fn hidden(&self) -> u16 {
        self.hidden
    }

    fn hold_back(&mut self, fired: u16, sample: Sample) {
        self.hidden = self.swallow | fired;
        self.swallow = self.hidden & sample.held;
    }

    /// One P1 poll. `blocked` is true while any input owner other than this stage's own is active (the
    /// vanilla pause menu, the plugin menu and its 15-frame grace, a modal panel, the replay editor, the
    /// native waiting-room UI): no request is raised then, and requests already pending stay
    /// (SPEC-input-04 §5.3). Returns the set of chords that passed this poll as a bit set of
    /// `Request`s for the runtime's diagnostics.
    pub fn poll(&mut self, chords: &ChordSet, sample: Sample, blocked: bool) -> u16 {
        let was_open = self.hold.is_open();
        self.hold_event = self.hold.poll(chords.game_speed, sample, blocked);
        self.refused = None;
        if blocked {
            self.refused = save_or_load_press(chords, sample).map(|action| (action, Refused::Owner));
            self.hold_back(0, sample);
            return 0;
        }
        // rc.8: the game speed hold owns the poll it opens on, every poll it stays open and the poll it closes on. No
        // other chord fires then (ZL + D-pad Left would cycle the CPU), and its chord's buttons and D-pad Left/Right stay
        // from the game until each is released.
        if was_open || self.hold.is_open() {
            self.refused = save_or_load_press(chords, sample).map(|action| (action, Refused::SpeedHold));
            self.hold_back(self.hold.hidden(), sample);
            return 0;
        }
        let mut passed = 0u16;
        let mut fired = 0u16;
        for (request, chord) in [
            (Request::SaveState, chords.save_state),
            (Request::LoadState, chords.load_state),
            (Request::InputRecord, chords.input_record),
            (Request::InputPlayback, chords.input_playback),
            (Request::CpuCycle, chords.cpu_cycle),
            (Request::PlayerCycle, chords.player_cycle),
            (Request::PlayPlayer, chords.play_player),
            (Request::RecordingPopup, chords.recording_popup),
            (Request::InputDisplay, chords.input_display),
        ] {
            let pass = passes_row(chord, sample, request == Request::InputDisplay);
            if !pass {
                let action = match request {
                    Request::SaveState => Some(Action::Save),
                    Request::LoadState => Some(Action::Load),
                    _ => None,
                };
                if let Some(action) = action.filter(|_| passes(chord, sample)) {
                    let deluxe = sample.overlay && deluxe_rows_take(chord, sample);
                    self.refused = Some((action, if deluxe { Refused::DeluxeRows } else { Refused::GameCubeTriggers }));
                }
                continue;
            }
            fired |= chord.0;
            if self.pending & request.bit() == 0 {
                self.pending |= request.bit();
                passed |= request.bit();
            }
        }
        self.hold_back(fired, sample);
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

    #[cfg(test)]
    pub fn is_pending(&self, request: Request) -> bool {
        self.pending & request.bit() != 0
    }

    pub fn has_pending_request(&self) -> bool {
        self.pending != 0
    }

    /// Session clear: scene departure, hot swap, the waiting-room UI becoming busy. The speed hold closes too (the
    /// runtime takes the pop-up down).
    pub fn clear_session(&mut self) {
        self.pending = 0;
        self.hold.clear();
        self.hold_event = HoldEvent::Idle;
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
#[cfg(test)]
pub fn unusable_on(chord: Chord, style: u32) -> bool {
    style == STYLE_GAMECUBE && chord.0 & ((1 << logical::L) | (1 << logical::MINUS)) != 0
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::logical::{self, A, B, DPAD_LEFT, DPAD_RIGHT, DPAD_UP, ZR};
    use super::{passes, passes_without_l, ChordSet, Detector, Request, Sample};
    use crate::model::factory::DEFS;
    use crate::model::menu::chord::Chord;
    use crate::model::menu::options::CHORD_ROWS;
    use crate::model::menu_shortcuts::{DEFAULT_OPEN_MENU_CONFIG, DEFAULT_SHORTCUTS};
    use crate::model::settings::{self, Snapshot};

    fn bits(buttons: &[u8]) -> u16 {
        buttons.iter().fold(0u16, |set, button| set | (1 << button))
    }

    fn sample(held: &[u8], pressed: &[u8]) -> Sample {
        Sample { held: bits(held), pressed: bits(pressed), ..Sample::default() }
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
    fn the_ten_chords_keep_todays_defaults_and_add_two() {
        let ten: [(&str, [usize; 2], &str); 10] = [
            ("save_state_save", [6, 9], "Save State Save"),
            ("save_state_load", [6, 8], "Save State Load"),
            ("input_record", [7, 9], "Record"),
            ("input_playback", [7, 8], "Play on CPU"),
            ("play_player", [7, 10], "Play on Player 1"),
            ("recording_popup", [7, 11], "Recording pop-up"),
            ("player_cycle", [6, 11], "Cycle Player"),
            ("cpu_cycle", [6, 10], "Cycle CPU"),
            // rc.8: R + D-pad Down / Up, a third family beside ZL and ZR.
            ("input_display_toggle", [5, 9], "Input display"),
            ("game_speed", [5, 8], "Game speed"),
        ];
        for (id, at, label) in ten {
            assert_eq!(factory_row(id), row(&at), "{id}");
            let (shown, row_id) = CHORD_ROWS.into_iter().find(|(_, row_id)| *row_id == id).unwrap();
            assert_eq!((shown, row_id), (label, id));
        }
        assert_eq!(CHORD_ROWS.len(), ten.len());
        // No two rows share a factory chord, and the new ones are usable on a GameCube pad (R is its Z).
        let chords: Vec<Chord> = ten.iter().map(|(id, ..)| Chord::from_row(factory_row(id))).collect();
        for (i, a) in chords.iter().enumerate() {
            assert!(chords[i + 1..].iter().all(|b| b != a), "{} shares its chord", ten[i].0);
        }
        assert!(!super::unusable_on(ChordSet::factory().input_display, super::STYLE_GAMECUBE));
        assert!(!super::unusable_on(ChordSet::factory().game_speed, super::STYLE_GAMECUBE));
        let set = ChordSet::factory();
        assert_eq!(set.input_display, Chord::from_row(factory_row("input_display_toggle")));
        assert_eq!(set.game_speed, Chord::from_row(factory_row("game_speed")));
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

    /// Eden rc5-opus-final-diag (2026-11-08): the Play on Player 1 chord (ZR + D-pad Left, pressed together) reached
    /// the game as ZR, and P1 shielded for 10 frames. The chord's buttons are hidden from the poll it passes until the
    /// poll each one is released.
    #[test]
    fn a_chords_buttons_are_hidden_from_the_game_until_released() {
        let set = ChordSet::factory();
        let mut detector = Detector::new();
        assert_eq!(detector.poll(&set, sample(&[ZR, DPAD_LEFT], &[ZR, DPAD_LEFT]), false), Request::PlayPlayer.bit());
        assert_eq!(detector.hidden(), bits(&[ZR, DPAD_LEFT]));
        assert!(detector.consume(Request::PlayPlayer));
        // D-pad Left goes first: it stays hidden on its release poll; ZR is hidden until its own.
        assert_eq!(detector.poll(&set, sample(&[ZR], &[]), false), 0);
        assert_eq!(detector.hidden(), bits(&[ZR, DPAD_LEFT]));
        assert_eq!(detector.poll(&set, sample(&[ZR], &[]), false), 0);
        assert_eq!(detector.hidden(), bits(&[ZR]));
        assert_eq!(detector.poll(&set, sample(&[], &[]), false), 0);
        assert_eq!(detector.hidden(), bits(&[ZR]), "the release poll hides the release");
        assert_eq!(detector.poll(&set, sample(&[], &[]), false), 0);
        assert_eq!(detector.hidden(), 0);
        // A plain ZR is no chord: the game sees the shield.
        assert_eq!(detector.poll(&set, sample(&[ZR], &[ZR]), false), 0);
        assert_eq!(detector.hidden(), 0);
        // A held chord button stays hidden while an owner blocks the chords.
        assert_eq!(detector.poll(&set, sample(&[ZR, DPAD_UP], &[ZR, DPAD_UP]), false), Request::InputPlayback.bit());
        assert_eq!(detector.poll(&set, sample(&[ZR, DPAD_UP], &[]), true), 0);
        assert_eq!(detector.hidden(), bits(&[ZR, DPAD_UP]));
    }

    #[test]
    fn input_display_chord_raises_its_request() {
        let set = ChordSet::factory();
        let mut detector = Detector::new();
        let toggle = sample(&[logical::R, logical::DPAD_DOWN], &[logical::DPAD_DOWN]);
        assert_eq!(detector.poll(&set, toggle, true), 0, "not while an owner holds the poll");
        assert_eq!(detector.poll(&set, toggle, false), Request::InputDisplay.bit());
        assert_eq!(detector.hidden(), bits(&[logical::R, logical::DPAD_DOWN]), "R does not shield");
        assert!(detector.consume(Request::InputDisplay));
        assert!(!detector.consume(Request::InputDisplay));
        assert_eq!(detector.poll(&set, sample(&[logical::R, logical::DPAD_DOWN], &[]), false), 0, "held: once");
        assert!(!passes(set.input_display, sample(&[logical::ZL, logical::DPAD_DOWN], &[logical::DPAD_DOWN])), "ZL + Down saves");
        // Online Deluxe's overlay chord L + R + D-pad Down holds R + D-pad Down: the toggle stays put (2026-11-11 19:57Z).
        let mut detector = Detector::new();
        let overlay = sample(&[logical::L, logical::R, logical::DPAD_DOWN], &[logical::DPAD_DOWN]);
        assert_eq!(detector.poll(&set, overlay, false), 0, "L + R + Down is Online Deluxe's overlay");
        let overlay_at_once = sample(&[logical::L, logical::R, logical::DPAD_DOWN], &[logical::L, logical::R, logical::DPAD_DOWN]);
        assert_eq!(detector.poll(&set, overlay_at_once, false), 0, "nor all three on one poll");
        assert!(passes_without_l(set.input_display, toggle));
        // A chord row that holds L itself still passes with L held.
        let with_l = Chord(bits(&[logical::L, logical::DPAD_DOWN]));
        assert!(passes_without_l(with_l, sample(&[logical::L, logical::DPAD_DOWN], &[logical::DPAD_DOWN])));
        // A row from the settings replaces the factory chord.
        let mut menu = Snapshot::factory();
        menu.set("input_display_toggle", &row(&[logical::X as usize, logical::Y as usize])).unwrap();
        let set = ChordSet::factory().with_rows(&menu);
        assert_eq!(set.input_display, Chord(bits(&[logical::X, logical::Y])));
        assert_eq!(set.game_speed, ChordSet::factory().game_speed);
    }

    /// The L exclusion is the two rc.8 rows' only: every other chord, and the menu shortcuts, fire with L held exactly
    /// as without it.
    #[test]
    fn only_the_rc8_rows_mind_l() {
        let set = ChordSet::factory();
        let l = 1u16 << logical::L;
        let rows = [
            (Request::SaveState, set.save_state),
            (Request::LoadState, set.load_state),
            (Request::InputRecord, set.input_record),
            (Request::InputPlayback, set.input_playback),
            (Request::CpuCycle, set.cpu_cycle),
            (Request::PlayerCycle, set.player_cycle),
            (Request::PlayPlayer, set.play_player),
            (Request::RecordingPopup, set.recording_popup),
        ];
        for (request, chord) in rows {
            assert_eq!(chord.0 & l, 0, "{request:?}'s factory chord has no L");
            let plain = Sample { held: chord.0, pressed: chord.0, ..Sample::default() };
            let with_l = Sample { held: chord.0 | l, pressed: chord.0, ..Sample::default() };
            let without = Detector::new().poll(&set, plain, false);
            assert_eq!(without & request.bit(), request.bit(), "{request:?} fires");
            assert_eq!(Detector::new().poll(&set, with_l, false), without, "{request:?} with L held");
        }
        // The permanent B + D-pad Up opens the menu with L held too.
        let b_up = sample(&[B, DPAD_UP], &[DPAD_UP]);
        let without = Detector::new().poll(&set, b_up, false);
        assert_eq!(without & Request::OpenMenu.bit(), Request::OpenMenu.bit());
        let b_up_l = sample(&[logical::L, B, DPAD_UP], &[DPAD_UP]);
        assert_eq!(Detector::new().poll(&set, b_up_l, false), without);
        // The two rc.8 rows: not with L, as before without it.
        for chord in [set.input_display, set.game_speed] {
            let plain = Sample { held: chord.0, pressed: chord.0, ..Sample::default() };
            assert!(passes_without_l(chord, plain) && passes(chord, plain));
            assert!(!passes_without_l(chord, Sample { held: chord.0 | l, pressed: chord.0, ..Sample::default() }));
        }
    }

    /// rc.9 C4 (`PLUGIN-SHORTCUT-CONFLICTS.md`), narrowed in rc.12: while Online Deluxe shows a page with rows, a row
    /// stands down only where Deluxe acts on its D-pad (Deluxe 0.10.5 `overlay.rs`): Up/Down with none of ZL, ZR, R, L
    /// held, and Left/Right under its edit hold, ZR without ZL (ZR + Right on its CPU Pilot page handed P1 to the CPU as
    /// TR opened the Recording pop-up). The console of 2026-11-13 kept Deluxe's Settings page open through Training and
    /// a waiting room, and Save and Load (ZL + D-pad Down/Up, which Deluxe ignores) did nothing, silently. A row that
    /// stands down hides none of its buttons; a Save or Load press that does says why (`take_refused`). The menu
    /// shortcuts and a row without the D-pad are unchanged; the Performance view has no rows.
    #[test]
    fn deluxe_window_stands_down_only_the_rows_it_takes() {
        use crate::model::state::refused::{Action, Refused};
        use crate::model::deluxe_overlay::{
            rows_page_visible, DeluxeOverlayStateV1, DELUXE_ROWS_PAGE, DELUXE_WINDOW_VISIBLE,
        };
        unsafe extern "C" fn rows_page(out: *mut DeluxeOverlayStateV1) -> bool {
            unsafe { out.write(DeluxeOverlayStateV1 { size: 8, flags: DELUXE_WINDOW_VISIBLE | DELUXE_ROWS_PAGE }) };
            true
        }
        unsafe extern "C" fn performance(out: *mut DeluxeOverlayStateV1) -> bool {
            unsafe { out.write(DeluxeOverlayStateV1 { size: 8, flags: DELUXE_WINDOW_VISIBLE }) };
            true
        }
        let set = ChordSet::factory();
        let save = sample(&[logical::ZL, logical::DPAD_DOWN], &[logical::DPAD_DOWN]);
        let on_rows = save.with_overlay(unsafe { rows_page_visible(Some(rows_page)) });
        let mut detector = Detector::new();
        assert_eq!(detector.poll(&set, on_rows, false), Request::SaveState.bit(), "a rows page: Save state fires");
        assert_eq!(detector.take_refused(), None);
        let load = sample(&[logical::ZL, logical::DPAD_UP], &[logical::DPAD_UP]).with_overlay(true);
        assert_eq!(Detector::new().poll(&set, load, false), Request::LoadState.bit(), "a rows page: Load state fires");
        let on_performance = save.with_overlay(unsafe { rows_page_visible(Some(performance)) });
        assert_eq!(Detector::new().poll(&set, on_performance, false), Request::SaveState.bit(), "Performance: it fires");
        let rows = [
            (Request::SaveState, set.save_state),
            (Request::LoadState, set.load_state),
            (Request::InputRecord, set.input_record),
            (Request::InputPlayback, set.input_playback),
            (Request::CpuCycle, set.cpu_cycle),
            (Request::PlayerCycle, set.player_cycle),
            (Request::PlayPlayer, set.play_player),
            (Request::RecordingPopup, set.recording_popup),
            (Request::InputDisplay, set.input_display),
        ];
        for (request, chord) in rows {
            let press = Sample { held: chord.0, pressed: chord.0, ..Sample::default() };
            // ZR + Left/Right are Deluxe's value edits; every other factory row's D-pad is left alone by it.
            let taken = matches!(request, Request::PlayPlayer | Request::RecordingPopup);
            let mut detector = Detector::new();
            let fired = detector.poll(&set, press.with_overlay(true), false);
            if taken {
                assert_eq!(fired, 0, "{request:?} on a rows page");
                assert_eq!(detector.hidden(), 0, "{request:?}: the game and Deluxe see the press");
            } else {
                assert_eq!(fired & request.bit(), request.bit(), "{request:?} fires on a rows page");
            }
            assert_eq!(Detector::new().poll(&set, press, false) & request.bit(), request.bit(), "{request:?} without it");
        }
        // A Save rebound onto a D-pad Deluxe moves its rows with stands down there, and says so.
        let mut menu = Snapshot::factory();
        menu.set("save_state_save", &row(&[logical::X as usize, DPAD_UP as usize])).unwrap();
        let rebound = ChordSet::factory().with_rows(&menu);
        let mut detector = Detector::new();
        assert_eq!(detector.poll(&rebound, sample(&[logical::X, DPAD_UP], &[DPAD_UP]).with_overlay(true), false), 0);
        assert_eq!(detector.take_refused(), Some((Action::Save, Refused::DeluxeRows)));
        assert_eq!(detector.take_refused(), None, "once");
        // The other refusals of a Save or Load press: an input owner, both GameCube triggers, the speed hold.
        let mut detector = Detector::new();
        assert_eq!(detector.poll(&set, sample(&[logical::ZL, DPAD_UP], &[DPAD_UP]), true), 0);
        assert_eq!(detector.take_refused(), Some((Action::Load, Refused::Owner)));
        let gc = Sample { gamecube: true, ..sample(&[logical::ZL, logical::ZR, logical::DPAD_DOWN], &[logical::DPAD_DOWN]) };
        let mut detector = Detector::new();
        assert_eq!(detector.poll(&set, gc, false) & Request::SaveState.bit(), 0);
        assert_eq!(detector.take_refused(), Some((Action::Save, Refused::GameCubeTriggers)));
        let mut detector = Detector::new();
        detector.poll(&set, sample(&[logical::R, DPAD_UP], &[DPAD_UP]), false);
        assert_eq!(detector.poll(&set, sample(&[logical::R, logical::ZL, DPAD_UP], &[logical::ZL]), false), 0);
        assert_eq!(detector.take_refused(), Some((Action::Load, Refused::SpeedHold)));
        // A press that is not the chord (ZL held, nothing new) is no refusal.
        let mut detector = Detector::new();
        detector.poll(&set, sample(&[logical::ZL], &[]), true);
        assert_eq!(detector.take_refused(), None);
        let mut detector = Detector::new();
        detector.poll(&set, sample(&[logical::R, DPAD_UP], &[DPAD_UP]).with_overlay(true), false);
        // R + D-pad Up holds a shoulder, so Deluxe moves no row: the speed hold opens beside the page (rc.12).
        assert_eq!(detector.hold_event(), crate::model::game_speed::HoldEvent::Opened, "the speed hold opens");
        let mut detector = Detector::new();
        detector.poll(&set, sample(&[B, DPAD_UP], &[DPAD_UP]).with_overlay(true), false);
        assert!(detector.consume(Request::OpenMenu), "B + D-pad Up still opens the menu");
        let mut detector = Detector::new();
        detector.poll(&set, sample(&[logical::MINUS], &[logical::MINUS]).with_overlay(true), false);
        assert!(detector.consume(Request::OpenMenu), "Minus still opens the menu");
        let mut menu = Snapshot::factory();
        menu.set("input_display_toggle", &row(&[logical::X as usize, logical::Y as usize])).unwrap();
        let x_y = ChordSet::factory().with_rows(&menu);
        let press = sample(&[logical::X, logical::Y], &[logical::Y]).with_overlay(true);
        assert_eq!(Detector::new().poll(&x_y, press, false), Request::InputDisplay.bit(), "a row without the D-pad fires");
    }

    /// rc.9 C3: on a GameCube pad Online Deluxe's L + R is both triggers, this plugin's ZL + ZR, so L + R + D-pad Down
    /// saved a state and started a recording at once. No ZL/ZR row and no rc.8 row fires while both are held; one
    /// trigger works as before, and a Pro Controller holding ZL + ZR is unchanged. C10: Deluxe's L + R + Left/Right
    /// (its peer cycle) fires no row on either pad.
    #[test]
    fn gamecube_triggers_are_deluxes_l_and_r() {
        use logical::{DPAD_DOWN, R, ZL};
        let set = ChordSet::factory();
        let gc = |held: &[u8], pressed: &[u8]| Sample { gamecube: true, ..sample(held, pressed) };
        for dir in [DPAD_DOWN, DPAD_UP, DPAD_LEFT, DPAD_RIGHT] {
            let mut detector = Detector::new();
            assert_eq!(detector.poll(&set, gc(&[ZL, ZR, dir], &[dir]), false), 0, "both triggers + {dir}");
            assert_eq!(detector.hidden(), 0);
        }
        assert_eq!(Detector::new().poll(&set, gc(&[ZL, DPAD_DOWN], &[DPAD_DOWN]), false), Request::SaveState.bit());
        assert_eq!(Detector::new().poll(&set, gc(&[ZR, DPAD_DOWN], &[DPAD_DOWN]), false), Request::InputRecord.bit());
        assert_eq!(Detector::new().poll(&set, gc(&[R, DPAD_DOWN], &[DPAD_DOWN]), false), Request::InputDisplay.bit());
        assert_eq!(Detector::new().poll(&set, gc(&[ZL, ZR, R, DPAD_DOWN], &[DPAD_DOWN]), false), 0, "Z + Down in it");
        // The pad's own bits: both triggers arrive as the game's L and R (`logical_set`, style 7).
        let raw = Sample::of(super::STYLE_GAMECUBE, super::bit::L | super::bit::R | super::bit::DPAD_DOWN, super::bit::DPAD_DOWN);
        assert_eq!(Detector::new().poll(&set, raw, false), 0);
        let both = Request::SaveState.bit() | Request::InputRecord.bit();
        assert_eq!(Detector::new().poll(&set, sample(&[ZL, ZR, DPAD_DOWN], &[DPAD_DOWN]), false), both, "Pro: unchanged");
        for dir in [DPAD_LEFT, DPAD_RIGHT] {
            assert_eq!(Detector::new().poll(&set, sample(&[logical::L, R, dir], &[dir]), false), 0, "C10: L + R + {dir}");
        }
    }

    #[test]
    fn hold_hides_the_chord_and_step_buttons_and_blocks_other_chords() {
        use crate::model::game_speed::{Dir, HoldEvent};
        use logical::{DPAD_LEFT, DPAD_RIGHT, DPAD_UP, R, ZL};
        let set = ChordSet::factory();
        let mut detector = Detector::new();
        assert_eq!(detector.poll(&set, sample(&[R, DPAD_UP], &[DPAD_UP]), false), 0);
        assert_eq!(detector.hold_event(), HoldEvent::Opened);
        assert_eq!(detector.hidden(), bits(&[R, DPAD_UP, DPAD_LEFT, DPAD_RIGHT]));
        // ZL + Left while the pop-up is open steps the speed; it does not cycle the CPU.
        assert_eq!(detector.poll(&set, sample(&[R, ZL, DPAD_LEFT], &[DPAD_LEFT]), false), 0);
        assert_eq!(detector.hold_event(), HoldEvent::Step(Dir::Left));
        assert!(!detector.is_pending(Request::CpuCycle));
        assert_eq!(detector.hidden() & bits(&[R, DPAD_LEFT, DPAD_RIGHT]), bits(&[R, DPAD_LEFT, DPAD_RIGHT]));
        assert_eq!(detector.poll(&set, sample(&[R, DPAD_RIGHT], &[DPAD_RIGHT]), false), 0);
        assert_eq!(detector.hold_event(), HoldEvent::Step(Dir::Right));
        // R released while Right is still held: closed, and Right stays from the game until its release.
        assert_eq!(detector.poll(&set, sample(&[DPAD_RIGHT], &[]), false), 0);
        assert_eq!(detector.hold_event(), HoldEvent::Closed);
        assert_ne!(detector.hidden() & bits(&[DPAD_RIGHT]), 0, "no taunt on the way out");
        assert_eq!(detector.poll(&set, sample(&[], &[]), false), 0);
        assert_eq!(detector.poll(&set, sample(&[], &[]), false), 0);
        assert_eq!(detector.hidden(), 0);
        // After the hold, the other chords are back.
        assert_eq!(detector.poll(&set, sample(&[ZL, DPAD_LEFT], &[DPAD_LEFT]), false), Request::CpuCycle.bit());
        // A session clear closes the hold without an event.
        let mut detector = Detector::new();
        detector.poll(&set, sample(&[R, DPAD_UP], &[DPAD_UP]), false);
        detector.clear_session();
        assert_eq!(detector.hold_event(), HoldEvent::Idle);
        assert_eq!(detector.poll(&set, sample(&[R, DPAD_UP], &[]), false), 0);
        assert_eq!(detector.hold_event(), HoldEvent::Idle, "held, not newly pressed: it does not reopen");
    }

    #[test]
    fn hidden_buttons_are_the_controllers_own_bits_for_its_style() {
        use super::{bit, logical_set, physical_mask, STYLE_GAMECUBE};
        let zr_left = bits(&[ZR, DPAD_LEFT]);
        assert_eq!(physical_mask(5, zr_left), bit::ZR | bit::LEFT_SR | bit::RIGHT_SR | bit::DPAD_LEFT);
        assert_eq!(physical_mask(STYLE_GAMECUBE, zr_left), bit::R | bit::REAL_DIGITAL_R | bit::DPAD_LEFT);
        for style in [5, STYLE_GAMECUBE] {
            for button in 0..16u8 {
                let mask = physical_mask(style, 1 << button);
                if mask != 0 {
                    assert_eq!(logical_set(style, mask), 1 << button, "style {style} button {button}");
                }
            }
        }
    }
    }
}
