//! The game speed bind (rc.8, `docs/design/rc8-binds.md`): the native Training speeds, the lists the bind steps
//! through, the hold (open on the chord, Left/Right steps, close on release), the pop-up's text, the Training write and
//! the waiting-room practice's slot in the game's slow table. Pure; `runtime::features::game_speed` reads and writes the
//! game.
//!
//! Facts (13.0.5, the owned main ELF; seam group `game_speed`):
//! - Training's Speed item is a `u32` at `PauseMenu+0xb54`: 1 x1, 2 1 Frame, 3 2/3, 4 1/2, 5 1/4, 6 x1.5 (labels
//!   `mel_training_speed*`). The menu setter's own path for an unchanged value stores it and writes the row's text with
//!   `0x1bbbd30(menu, value, menu+0x538)`, with no slide and no sound. The game's per-update apply reads the field and
//!   does the rest (rate and slot), edge-triggered.
//! - The slow table is `*(main+0x52c3610)`, 0x48 bytes: sixteen `{in_use, divisor}` byte pairs at +0x20 and a dirty
//!   byte at +0x40. A new table has every slot `{0, 1}` and dirty set. The apply frees a slot as `{0, 1}` plus dirty,
//!   and takes the first slot with `in_use` 0 as `{1, N}` plus dirty. `BattleObjectSlow::rate_request` then gives
//!   `1 / max N`.

use core::fmt::Write;

use super::chords::{glyph, logical, passes, Sample};
use super::menu::chord::Chord;
use super::storage::FixedPath;

/// A value of Training's Speed item, as the game stores it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Speed {
    X1 = 1,
    OneFrame = 2,
    TwoThirds = 3,
    Half = 4,
    Quarter = 5,
    OneAndHalf = 6,
}

impl Speed {
    pub const ALL: [Speed; 6] = [Speed::X1, Speed::OneFrame, Speed::TwoThirds, Speed::Half, Speed::Quarter, Speed::OneAndHalf];

    pub const fn value(self) -> u32 {
        self as u32
    }

    pub fn from_value(value: u32) -> Option<Speed> {
        Speed::ALL.into_iter().find(|speed| speed.value() == value)
    }

    /// The menu's text for the value (`mel_training_speed*`).
    pub const fn label(self) -> &'static str {
        match self {
            Speed::X1 => "x1",
            Speed::OneFrame => "1 Frame",
            Speed::TwoThirds => "2/3",
            Speed::Half => "1/2",
            Speed::Quarter => "1/4",
            Speed::OneAndHalf => "x1.5",
        }
    }

    /// The slow divisor the game's apply requests for this speed with the update rate left at 60: 1/2 and 1/4. 2/3 and
    /// x1.5 also change the update rate, so the slot alone cannot give them.
    pub const fn slot_divisor(self) -> Option<u8> {
        match self {
            Speed::Half => Some(2),
            Speed::Quarter => Some(4),
            _ => None,
        }
    }
}

/// Training's speeds the bind offers, slowest first. 1 Frame stays with the native menu: it is a pause-and-step mode.
pub const TRAINING_SPEEDS: [Speed; 5] = [Speed::Quarter, Speed::Half, Speed::TwoThirds, Speed::X1, Speed::OneAndHalf];
/// The waiting-room practice's speeds: the ones the slot gives alone (2/3 and x1.5 also raise the update rate, which the
/// room's own timers count).
pub const LOBBY_SPEEDS: [Speed; 3] = [Speed::Quarter, Speed::Half, Speed::X1];

/// Where the bind runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Training,
    Lobby,
}

impl Place {
    pub const fn speeds(self) -> &'static [Speed] {
        match self {
            Place::Training => &TRAINING_SPEEDS,
            Place::Lobby => &LOBBY_SPEEDS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Left,
    Right,
}

/// One step from `current` in `list`: Left is the slower neighbour, Right the faster one; the ends stop. A speed the
/// list does not hold (Training's 1 Frame) steps from x1's place.
pub fn step(list: &[Speed], current: Speed, dir: Dir) -> Speed {
    let Some(last) = list.len().checked_sub(1) else { return current };
    let at = list.iter().position(|s| *s == current).or_else(|| list.iter().position(|s| *s == Speed::X1)).unwrap_or(0);
    let next = match dir {
        Dir::Left => at.saturating_sub(1),
        Dir::Right => (at + 1).min(last),
    };
    list[next]
}

/// What a poll did to the hold.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HoldEvent {
    /// Closed, and still closed.
    #[default]
    Idle,
    /// The chord passed: the pop-up opens. The press that opened it is not a step.
    Opened,
    /// Open, nothing new.
    Held,
    Step(Dir),
    /// No chord button is held any more (or an input owner took the poll): the pop-up goes.
    Closed,
}

/// D-pad Left and Right: the step buttons while the hold is open.
pub const STEP_BUTTONS: u16 = (1 << logical::DPAD_LEFT) | (1 << logical::DPAD_RIGHT);

/// The speed hold on P1's controller: open while any button of the chord that opened it is held.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hold {
    /// The chord's buttons while open, 0 while closed.
    open: u16,
}

impl Hold {
    pub const fn new() -> Self {
        Self { open: 0 }
    }

    pub const fn is_open(self) -> bool {
        self.open != 0
    }

    /// One P1 poll. `blocked`: an input owner holds the poll (the pause menu, the plugin's menu, the waiting-room UI):
    /// the hold closes and never opens.
    pub fn poll(&mut self, chord: Chord, sample: Sample, blocked: bool) -> HoldEvent {
        if self.open != 0 {
            if blocked || sample.held & self.open == 0 {
                self.open = 0;
                return HoldEvent::Closed;
            }
            let left = sample.pressed & (1 << logical::DPAD_LEFT) != 0;
            let right = sample.pressed & (1 << logical::DPAD_RIGHT) != 0;
            return match (left, right) {
                (true, false) => HoldEvent::Step(Dir::Left),
                (false, true) => HoldEvent::Step(Dir::Right),
                _ => HoldEvent::Held,
            };
        }
        if !blocked && passes(chord, sample) {
            self.open = chord.0;
            return HoldEvent::Opened;
        }
        HoldEvent::Idle
    }

    /// The logical buttons the game must not see while the hold is open: the chord's and the step buttons.
    pub const fn hidden(self) -> u16 {
        if self.open != 0 {
            self.open | STEP_BUTTONS
        } else {
            0
        }
    }

    /// Session clear: the hold closes without an event.
    pub fn clear(&mut self) {
        self.open = 0;
    }
}

/// The pop-up's title.
pub const POPUP_HEADER: &str = "Game speed";
/// The standing notice the pop-up is (`notices::upsert`/`clear`).
pub const POPUP_ID: u8 = 0x2C;
/// The pop-up's length in frames: renewed by every step, cleared on release.
pub const POPUP_FRAMES: u16 = 600;

/// The pop-up's body: the speed between the D-pad Left and Right glyphs of the controller's style (the game font's own
/// private-use glyphs, `chords::glyph`).
pub fn popup_body(speed: Speed, style: u32) -> FixedPath<32> {
    let mut out = FixedPath::<32>::new();
    let mark = |button: u8| glyph(button, style).and_then(|code| char::from_u32(u32::from(code)));
    let (left, right) = (mark(logical::DPAD_LEFT).unwrap_or('<'), mark(logical::DPAD_RIGHT).unwrap_or('>'));
    let _ = write!(out, "{left} {} {right}", speed.label());
    out
}

/// The Training write (decision 6): the field and the row's text pane, both offsets from `PauseMenu`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrainingWrite {
    pub field: usize,
    pub value: u32,
    pub text_pane: usize,
}

/// `PauseMenu+0xb54`, the Speed item's value.
pub const SPEED_FIELD: usize = 0xb54;
/// `PauseMenu+0x538`, the Speed row's value pane (the text `0x1bbbd30` writes).
pub const SPEED_TEXT_PANE: usize = 0x538;

/// The native addresses the runtime forms, each decoded from the guarded words by the host tests.
pub mod native {
    /// `main+0x52c3420`: Training's state root (`game_speed.training_chain`, as `cpu_level.training_rank_choice`).
    pub const TRAINING_ROOT: usize = 0x52c3420;
    /// From the root to the pause menu: `[[[[[[root]+8]]+0xb38]]+0x210]+8]+0x80`.
    pub const PANEL_CHAIN: [usize; 7] = [8, 0, 0xb38, 0, 0x210, 8, 0x80];
    /// `main+0x1bbbd30(menu, value, pane)`: value -> text (`game_speed.speed_text`).
    pub const SPEED_TEXT_FN: usize = 0x1bbbd30;
    /// `main+0x52c3610`: the slow table's global (`game_speed.rate_request`).
    pub const SLOW_TABLE: usize = 0x52c3610;
}

/// Every row the bind relies on; all must pass the boot gate or the speed bind is off.
pub const SEAM_ROWS: [&str; 11] = [
    "game_speed.speed_store_text",
    "game_speed.speed_text",
    "game_speed.training_chain",
    "game_speed.training_value",
    "game_speed.slot_free",
    "game_speed.slot_take",
    "game_speed.hud_icon",
    "game_speed.rate_request",
    "game_speed.max_divisor",
    "game_speed.table_create",
    "game_speed.table_destroy",
];

/// What the UI tick writes for a wanted Training speed: the native store and then the text, only when the game's own
/// pause menu is closed (its setter writes the same pane while it is open) and the value is one the game knows.
pub fn training_write(wanted: u32, current: u32, pause_open: bool) -> Option<TrainingWrite> {
    let speed = Speed::from_value(wanted)?;
    (!pause_open && wanted != current).then_some(TrainingWrite { field: SPEED_FIELD, value: speed.value(), text_pane: SPEED_TEXT_PANE })
}

/// The slow table's layout (`*(main+0x52c3610)`).
pub mod table {
    pub const SLOTS_AT: usize = 0x20;
    pub const SLOTS: usize = 16;
    pub const DIRTY_AT: usize = 0x40;
    pub const SIZE: usize = 0x48;
}

/// Byte access to the slow table: a host buffer in tests, the game's memory on the Switch.
pub trait Bytes {
    fn get(&self, at: usize) -> u8;
    fn set(&mut self, at: usize, value: u8);
}

impl<const N: usize> Bytes for [u8; N] {
    fn get(&self, at: usize) -> u8 {
        self[at]
    }
    fn set(&mut self, at: usize, value: u8) {
        self[at] = value;
    }
}

/// A table as the game creates it (`.1345fdc`): every slot `{0, 1}`, dirty set.
pub fn fresh_table() -> [u8; table::SIZE] {
    let mut out = [0u8; table::SIZE];
    for slot in 0..table::SLOTS {
        out[table::SLOTS_AT + 2 * slot + 1] = 1;
    }
    out[table::DIRTY_AT] = 1;
    out
}

/// Take the first slot not in use as `{1, divisor}` and set dirty, in the order the game's apply takes its own
/// (`.155bba4` in_use, `.155bbb0` divisor, `.155bbb4` dirty). `None` when every slot is in use (nothing is written).
pub fn take(table: &mut (impl Bytes + ?Sized), divisor: u8) -> Option<u8> {
    let slot = (0..table::SLOTS).find(|slot| table.get(table::SLOTS_AT + 2 * slot) == 0)?;
    table.set(table::SLOTS_AT + 2 * slot, 1);
    table.set(table::SLOTS_AT + 2 * slot + 1, divisor);
    table.set(table::DIRTY_AT, 1);
    Some(slot as u8)
}

/// Free `slot` as the game's apply frees its own (`{0, 1}` and dirty), only while it still holds `{1, divisor}`: a slot
/// the game has since freed or reused is left alone. Returns whether it wrote.
pub fn free(table: &mut (impl Bytes + ?Sized), slot: u8, divisor: u8) -> bool {
    let at = table::SLOTS_AT + 2 * usize::from(slot);
    if usize::from(slot) >= table::SLOTS || table.get(at) != 1 || table.get(at + 1) != divisor {
        return false;
    }
    table.set(at, 0);
    table.set(at + 1, 1);
    table.set(table::DIRTY_AT, 1);
    true
}

/// The waiting-room practice right now, as the runtime reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LobbyNow {
    /// The lobby engine is active and the live mode words are the waiting room: the practice battle is the one running.
    pub in_practice: bool,
    /// The native waiting-room UI is busy (a match found, a dialog): the round may be next.
    pub ui_busy: bool,
    /// The address `*(main+0x52c3610)` holds now (0 without a table).
    pub table: usize,
}

/// The slot the plugin holds: in which table, which slot, which divisor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Held {
    pub table: usize,
    pub slot: u8,
    pub divisor: u8,
}

/// What a lobby step did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LobbyStep {
    Nothing,
    Taken(Held),
    Freed(Held),
    /// The record was dropped without a write: the table is not the one taken from (or there is none), or the slot no
    /// longer holds `{1, divisor}`.
    Dropped(Held),
    /// Every slot was in use: no slot, the speed stays x1 until one frees.
    NoFreeSlot,
}

/// The lobby's slot keeper (decisions 7 and 8): edge-triggered, one slot at most.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LobbySlot {
    held: Option<Held>,
}

impl LobbySlot {
    pub const fn new() -> Self {
        Self { held: None }
    }

    pub const fn held(&self) -> Option<Held> {
        self.held
    }

    /// One P1 frame. `wanted` is the lobby speed the bind set; `bytes` reaches `now.table` (the runtime builds it only
    /// for a non-zero table). Takes only in the practice while the waiting-room UI is free; a held slot whose speed no
    /// longer applies is released (`release`), also on a frame after the scene left the practice.
    pub fn step(&mut self, wanted: Speed, now: LobbyNow, bytes: Option<&mut dyn Bytes>) -> LobbyStep {
        let divisor = wanted.slot_divisor().filter(|_| now.in_practice && !now.ui_busy && now.table != 0);
        if let Some(held) = self.held {
            if divisor == Some(held.divisor) && now.table == held.table {
                return LobbyStep::Nothing;
            }
            return self.release(now.table, bytes);
        }
        let (Some(divisor), Some(table)) = (divisor, bytes) else { return LobbyStep::Nothing };
        match take(table, divisor) {
            Some(slot) => {
                let held = Held { table: now.table, slot, divisor };
                self.held = Some(held);
                LobbyStep::Taken(held)
            }
            None => LobbyStep::NoFreeSlot,
        }
    }

    /// The practice's teardown, before the native one can reuse the table: the session clear on the practice P1's
    /// destroy, the scene's exit and any scene change (a match found shows no busy edge first, Eden 2026-11-11 08:20Z),
    /// and a frame whose speed no longer applies. The slot is freed with a write only while `table_now`
    /// (`*(main+0x52c3610)` now) is still the table it was taken from and the slot still holds `{1, divisor}`; otherwise
    /// the record goes without a write.
    pub fn release(&mut self, table_now: usize, bytes: Option<&mut dyn Bytes>) -> LobbyStep {
        let Some(held) = self.held.take() else { return LobbyStep::Nothing };
        let freed = table_now != 0 && table_now == held.table && bytes.is_some_and(|table| free(table, held.slot, held.divisor));
        if freed {
            LobbyStep::Freed(held)
        } else {
            LobbyStep::Dropped(held)
        }
    }

    /// Forget the slot without a write.
    pub fn forget(&mut self) -> Option<Held> {
        self.held.take()
    }
}

#[cfg(test)]
#[path = "game_speed_tests.rs"]
mod tests;
