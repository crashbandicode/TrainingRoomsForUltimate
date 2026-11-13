//! The game speed bind (rc.8, `docs/design/rc8-binds.md`): the native Training speeds, the lists the bind steps
//! through, the hold (open on the chord, Left/Right steps, close on release), the pop-up's text, the Training write and
//! the waiting-room practice's slot in the game's slow table. Pure; `runtime::features::game_speed` reads and writes the
//! game.
//!
//! Facts (13.0.5, the owned main ELF; seam group `game_speed`):
//! - Training's Speed item is a `u32` at `PauseMenu+0xb54`: 1 x1, 2 1 Frame, 3 2/3, 4 1/2, 5 1/4, 6 x1.5 (labels
//!   `mel_training_speed*`). The menu setter's own path for an unchanged value stores it and writes the row's text with
//!   `0x1bbbd30(menu, value, menu+0x538)`, with no slide and no sound.
//! - The game applies it in `FUN_0155a000`'s speed code, edge-triggered against the Training mode object's `+0x194`, and
//!   only on the pause/resume side: in closed-menu play that code is skipped (Eden 2026-11-11 `out-diag6`: 0 hits in
//!   play, 137 with the menu open; QCF's RE 09:12Z). So the bind also does the apply's writes itself (`mode_apply`),
//!   to the same object, found the way the game finds it (`registry_find`).
//! - The slow table is `*(main+0x52c3610)`, 0x48 bytes: sixteen `{in_use, divisor}` byte pairs at +0x20 and a dirty
//!   byte at +0x40. A new table has every slot `{0, 1}` and dirty set. The apply frees a slot as `{0, 1}` plus dirty,
//!   and takes the first slot with `in_use` 0 as `{1, N}` plus dirty. `BattleObjectSlow::rate_request` then gives
//!   `1 / max N`.

use core::fmt::Write;

use super::chords::{deluxe_rows_take, deluxe_shoulders, glyph, logical, passes_without_l, Sample};
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
    #[cfg(test)]
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
/// The waiting-room practice's speeds: Training's five (rc.8 R2). 1/2 and 1/4 are a slot alone; 2/3 and x1.5 never raise
/// the update rate, which the room's own timers count: 2/3 is a paced slot and x1.5 an extra world step
/// (`model::lobby_speed`).
pub const LOBBY_SPEEDS: [Speed; 5] = TRAINING_SPEEDS;

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
            // A Left/Right is not a step while Online Deluxe has it: its L + R chords are held (L, or both triggers on a
            // GameCube pad, and not in the chord; rc.8 and rc.9 C3), or its open page changes a value with it (rc.9 C4,
            // narrowed in rc.12: only under its edit hold, ZR without ZL).
            let lr = Chord((1 << logical::DPAD_LEFT) | (1 << logical::DPAD_RIGHT) | self.open);
            let deluxe = deluxe_shoulders(Chord(self.open), sample) || (sample.overlay && deluxe_rows_take(lr, sample));
            let left = !deluxe && sample.pressed & (1 << logical::DPAD_LEFT) != 0;
            let right = !deluxe && sample.pressed & (1 << logical::DPAD_RIGHT) != 0;
            return match (left, right) {
                (true, false) => HoldEvent::Step(Dir::Left),
                (false, true) => HoldEvent::Step(Dir::Right),
                _ => HoldEvent::Held,
            };
        }
        if !blocked && passes_without_l(chord, sample) {
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
    /// From the root to the HUD's data object, the same chain's first five hops (`game_speed.hud_data`): the object the
    /// speed indicator's refresh runs on (x19 at `.13e5500`), whose `[[+8]+0x80]` is the pause menu.
    pub const HUD_CHAIN: [usize; 5] = [8, 0, 0xb38, 0, 0x210];
    /// `main+0x377aee0(anim, rate)`: plays one of the layout's animations, as the refresh calls it
    /// (`game_speed.anim_play`).
    pub const ANIM_PLAY_FN: usize = 0x377aee0;
    /// `main+0x1bbbd30(menu, value, pane)`: value -> text (`game_speed.speed_text`).
    pub const SPEED_TEXT_FN: usize = 0x1bbbd30;
    /// `main+0x52c3610`: the slow table's global (`game_speed.rate_request`).
    pub const SLOW_TABLE: usize = 0x52c3610;
    /// `main+0x52c3910`: the word the battle's object registry is reached through, `[[main+0x52c3910]]`
    /// (`game_speed.mode_key`).
    pub const REGISTRY: usize = 0x52c3910;
    /// `main+0x506fd10`: the Training mode object's class vtable (`game_speed.mode_vtable`).
    pub const MODE_VTABLE: usize = 0x506fd10;
    /// `main+0x593b444`: the rate code (`game_speed.rates_slow`).
    pub const RATE_CODE: usize = 0x593b444;
    /// `main+0x523d004`: game updates per second, 60/120/180 (`game_speed.rates_slow`).
    pub const UPDATE_RATE: usize = 0x523d004;
    /// `main+0x532f8b0`: the state whose `+8 -> +0x138` set and `+0x2b8 == 11` makes the apply read 0
    /// (`game_speed.mode_state`).
    pub const STATE: usize = 0x532f8b0;
    /// `main+0x52b92e0`: the static guard of the 1 Frame step bits at `main+0x52b92ec` (`game_speed.step_clear`).
    pub const STEP_GUARD: usize = 0x52b92e0;
    pub const STEP_BITS: usize = 0x52b92ec;
    /// `main+0x53398d8`: the object whose `[+8]` byte the apply clears on a change away from 1 Frame
    /// (`game_speed.step_clear`).
    pub const STEP_FLAG: usize = 0x53398d8;
}

/// The HUD's speed indicator (the stopwatch: `info_training`'s `in_speed`/`out_speed`, text `set_txt_speed`), as the game's
/// refresh drives it (`.13e5500`–`.13e5544`).
pub mod hud {
    /// The HUD data object's `+0x231`: the indicator is shown.
    pub const SHOWN: usize = 0x231;
    /// The pause menu's `+0x168`: `in_speed`, which shows it.
    pub const SHOW_ANIM: usize = 0x168;
    /// The pause menu's `+0x178`: `out_speed`, which hides it.
    pub const HIDE_ANIM: usize = 0x178;
    /// The HUD's state handlers sit at `+0x40 + 16 * state`: the refresh is state 6's (`0x13e5470`), the Training menu
    /// state the Speed is edited in. Play (state 5, `0x13e5110`) never runs it.
    #[cfg(test)]
    pub const STATE_TABLE: usize = 0x40;
    #[cfg(test)]
    pub const REFRESH_STATE: usize = 6;
}

/// One change of the HUD's speed indicator: the animation played on the pause menu and the shown flag after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Icon {
    pub anim: usize,
    pub shown: bool,
}

/// The game's indicator refresh (`.13e5500`–`.13e5544`) for the menu's value and the shown flag: any value but x1 shows
/// it (`in_speed`, flag set), x1 hides it (`out_speed`, flag cleared), each only when the flag says otherwise. The game
/// runs it only in its menu state, so the bind runs it after a closed-menu step; the next menu visit finds it done.
pub fn hud_icon(value: u32, shown: bool) -> Option<Icon> {
    match (shown, value == Speed::X1.value()) {
        (false, false) => Some(Icon { anim: hud::SHOW_ANIM, shown: true }),
        (true, true) => Some(Icon { anim: hud::HIDE_ANIM, shown: false }),
        _ => None,
    }
}

/// The Training mode object (`FUN_0155a000`'s `this`, x25 there), as the speed code uses it.
pub mod mode {
    /// `+0x194`: the value the game applied last.
    pub const APPLIED: usize = 0x194;
    /// `+0x1b4`: its slot in the slow table, `NO_SLOT` for none.
    pub const SLOT: usize = 0x1b4;
    pub const NO_SLOT: u8 = 0xff;
    /// The mode object's key in the battle's object registry: the game's per-update lookup (`.135ca40`–`.135ca48`)
    /// and the battle setup's registration (`.14ed3b4`–`.14ed3c0`). Not `0x1308abad79`: that key holds another object
    /// (vtable main+0x5070380, Eden 2026-11-11 `out-mode`).
    pub const KEY: u64 = 0x10_2259_0708;
    /// The registry hashes and compares the low 40 bits (`and x8, x2, #0xffffffffff`).
    pub const KEY_MASK: u64 = 0xff_ffff_ffff;
    /// The registry node: next at `+0`, hash at `+8`, key at `+0x10`, the object at `+0x18`.
    pub const NODE_HASH: usize = 8;
    pub const NODE_KEY: usize = 0x10;
    pub const NODE_OBJECT: usize = 0x18;
    /// The registry: buckets at `+0`, their count at `+8`.
    pub const MAP_COUNT: usize = 8;
    /// The state the apply reads 0 in (`[[main+0x532f8b0]+8]+0x2b8`).
    pub const STATE_OFF: u8 = 11;
    pub const STATE_SET: usize = 0x138;
    pub const STATE_KIND: usize = 0x2b8;
    /// The 1 Frame step bit at `main+0x52b92ec` the apply always clears.
    pub const STEP_BIT: u32 = 1 << 2;
    /// The registry walk's bound (a bucket chain is a handful of nodes).
    pub const WALK: usize = 64;
}

/// The battle's object registry lookup the game itself uses (`FUN_015435c0`, a libc++ `unordered_map` find), read-only:
/// `map` is `[[main+0x52c3910]]`, `read` gives the pointer-sized word at an address (`None` when it is no object field).
/// Returns the object stored for `key`, the Training mode object for `mode::KEY` in Training.
pub fn registry_find(map: usize, key: u64, read: &dyn Fn(usize) -> Option<u64>) -> Option<usize> {
    let buckets = read(map)?;
    let n = read(map + mode::MAP_COUNT)?;
    if n == 0 {
        return None;
    }
    let hash = key & mode::KEY_MASK;
    let index = |h: u64| if n & (n - 1) == 0 { h & (n - 1) } else { h % n };
    let bucket = index(hash);
    let head = read((buckets as usize).checked_add(8 * bucket as usize)?)?;
    if head == 0 {
        return None;
    }
    let mut node = read(head as usize)?;
    for _ in 0..mode::WALK {
        if node == 0 {
            return None;
        }
        let node_hash = read(node as usize + mode::NODE_HASH)?;
        if node_hash == hash {
            if (read(node as usize + mode::NODE_KEY)? ^ key) & mode::KEY_MASK == 0 {
                return Some(read(node as usize + mode::NODE_OBJECT)? as usize);
            }
        } else if index(node_hash) != bucket {
            return None;
        }
        node = read(node as usize)?;
    }
    None
}

/// What the game's speed code does for a speed: the rate code at `main+0x593b444`, the updates per second at
/// `main+0x523d004` and the slow divisor it takes a slot for (jump table `0x44f0204`, `game_speed.rates_*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rate {
    pub code: u32,
    pub updates: u32,
    pub divisor: Option<u8>,
}

impl Speed {
    pub const fn rate(self) -> Rate {
        match self {
            Speed::X1 | Speed::OneFrame => Rate { code: 0, updates: 60, divisor: None },
            Speed::TwoThirds => Rate { code: 4, updates: 120, divisor: Some(3) },
            Speed::Half => Rate { code: 0, updates: 60, divisor: Some(2) },
            Speed::Quarter => Rate { code: 0, updates: 60, divisor: Some(4) },
            Speed::OneAndHalf => Rate { code: 6, updates: 180, divisor: Some(2) },
        }
    }
}

/// The writes of one Training apply, beside the slot bytes `mode_apply` made in the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModeApply {
    /// `+0x194`.
    pub value: u32,
    /// `+0x1b4` (`mode::NO_SLOT` without one).
    pub slot: u8,
    pub code: u32,
    pub updates: u32,
    /// The mode object's previous slot, freed in the table.
    pub freed: Option<u8>,
    /// Whether the table's dirty byte must be set (last, after every other write).
    pub dirty: bool,
}

/// The game's speed code for a change of the applied value to `speed` (`.155b5b8`–`.155bbb8`), which the game runs only
/// on the pause/resume side: free the mode object's slot `slot` if it is in use (`{0, 1}`), then take the first free
/// slot for the speed's divisor as `{1, N}` (none when all 16 are in use, as native). `applied`/`slot` are the object's
/// `+0x194`/`+0x1b4`. The dirty byte is not written here: the caller sets it last. `None` when `speed` is applied already.
pub fn mode_apply(speed: Speed, applied: u32, slot: u8, table: &mut (impl Bytes + ?Sized)) -> Option<ModeApply> {
    if applied == speed.value() {
        return None;
    }
    let mut out = ModeApply { value: speed.value(), slot: mode::NO_SLOT, code: 0, updates: 0, freed: None, dirty: false };
    if slot != mode::NO_SLOT && usize::from(slot) < table::SLOTS {
        let at = table::SLOTS_AT + 2 * usize::from(slot);
        if table.get(at) != 0 {
            table.set(at, 0);
            table.set(at + 1, 1);
            out.freed = Some(slot);
            out.dirty = true;
        }
    }
    let rate = speed.rate();
    (out.code, out.updates) = (rate.code, rate.updates);
    if let Some(divisor) = rate.divisor {
        if let Some(free) = (0..table::SLOTS).find(|i| table.get(table::SLOTS_AT + 2 * i) == 0) {
            table.set(table::SLOTS_AT + 2 * free, 1);
            table.set(table::SLOTS_AT + 2 * free + 1, divisor);
            out.slot = free as u8;
            out.dirty = true;
        }
    }
    Some(out)
}

/// Every row the bind relies on; all must pass the boot gate or the speed bind is off.
pub const SEAM_ROWS: [&str; 27] = [
    "game_speed.speed_store_text",
    "game_speed.speed_text",
    "game_speed.training_chain",
    "game_speed.training_value",
    "game_speed.slot_free",
    "game_speed.slot_take",
    "game_speed.hud_icon",
    "game_speed.hud_icon_show",
    "game_speed.hud_state",
    "game_speed.hud_data",
    "game_speed.anim_play",
    "game_speed.rate_request",
    "game_speed.max_divisor",
    "game_speed.table_create",
    "game_speed.table_destroy",
    "game_speed.mode_key",
    "game_speed.mode_find",
    "game_speed.mode_walk",
    "game_speed.mode_found",
    "game_speed.mode_vtable",
    "game_speed.mode_gate",
    "game_speed.mode_state",
    "game_speed.step_clear",
    "game_speed.rates_slow",
    "game_speed.rates_half",
    "game_speed.rates_fast",
    "game_speed.slot_divisors",
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
    #[cfg(test)]
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
#[cfg(test)]
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

/// Rewrite the divisor of `slot` from `from` to `to` and set dirty, only while it still holds `{1, from}` (the same
/// bytes `take` writes). Returns whether it wrote.
pub fn retarget(table: &mut (impl Bytes + ?Sized), slot: u8, from: u8, to: u8) -> bool {
    let at = table::SLOTS_AT + 2 * usize::from(slot);
    if usize::from(slot) >= table::SLOTS || table.get(at) != 1 || table.get(at + 1) != from {
        return false;
    }
    table.set(at + 1, to);
    table.set(table::DIRTY_AT, 1);
    true
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
    /// 2/3's pace rewrote the held slot's divisor (the held record shows the new one).
    Paced(Held),
}

/// The lobby's slot keeper (decisions 7 and 8): edge-triggered, one slot at most. 2/3's slot is paced (`pace`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LobbySlot {
    held: Option<Held>,
    /// The held slot is 2/3's: `pace` rewrites its divisor, and `step` keeps it whatever divisor it holds.
    paced: bool,
}

impl LobbySlot {
    pub const fn new() -> Self {
        Self { held: None, paced: false }
    }

    pub const fn held(&self) -> Option<Held> {
        self.held
    }

    /// One P1 frame. `wanted` is the lobby speed the bind set; `bytes` reaches `now.table` (the runtime builds it only
    /// in the practice, for a non-zero table). Takes only in the practice while the waiting-room UI is free; a held slot
    /// whose speed no longer applies is released (`release`) in the practice. Outside it nothing is written: the record
    /// is dropped (a match's table can reuse the practice table's block with the game's own `{1, N}` at that slot; worker
    /// B's R8 finding). The teardown's `release` has freed it before then.
    pub fn step(&mut self, wanted: Speed, now: LobbyNow, bytes: Option<&mut dyn Bytes>) -> LobbyStep {
        let slot = super::lobby_speed::LobbyPlan::of(wanted).slot().filter(|_| now.in_practice && !now.ui_busy && now.table != 0);
        if let Some(held) = self.held {
            if !now.in_practice {
                self.held = None;
                self.paced = false;
                return LobbyStep::Dropped(held);
            }
            let keep = now.table == held.table
                && match slot {
                    Some((_, true)) => self.paced,
                    Some((divisor, false)) => !self.paced && divisor == held.divisor,
                    None => false,
                };
            if keep {
                return LobbyStep::Nothing;
            }
            return self.release(now.table, bytes);
        }
        let (Some((divisor, paced)), Some(table)) = (slot, bytes) else { return LobbyStep::Nothing };
        match take(table, divisor) {
            Some(slot) => {
                let held = Held { table: now.table, slot, divisor };
                self.held = Some(held);
                self.paced = paced;
                LobbyStep::Taken(held)
            }
            None => LobbyStep::NoFreeSlot,
        }
    }

    /// 2/3's pace, on the waiting area's update after its slow step (`current_n`, `sub`: the slow state's current
    /// divisor and sub-counter): at a period's end the held paced slot gets the divisor `lobby_speed::paced_divisor`
    /// asks for, written only in the table it was taken from and only while it still holds `{1, divisor}`. Anything
    /// else (no paced slot, inside a period, another table, a slot the game took back) writes nothing; `step` and
    /// `release` deal with the record.
    pub fn pace(&mut self, current_n: u8, sub: u8, table_now: usize, bytes: Option<&mut dyn Bytes>) -> LobbyStep {
        let Some(held) = self.held.filter(|_| self.paced) else { return LobbyStep::Nothing };
        let Some(want) = super::lobby_speed::paced_divisor(current_n, sub) else { return LobbyStep::Nothing };
        if want == held.divisor || table_now == 0 || table_now != held.table {
            return LobbyStep::Nothing;
        }
        let Some(table) = bytes else { return LobbyStep::Nothing };
        if !retarget(table, held.slot, held.divisor, want) {
            return LobbyStep::Nothing;
        }
        let held = Held { divisor: want, ..held };
        self.held = Some(held);
        LobbyStep::Paced(held)
    }

    /// The practice's teardown, before the native one can reuse the table: the session clear on the practice P1's
    /// destroy, the scene's exit and any scene change (a match found shows no busy edge first, Eden 2026-11-11 08:20Z),
    /// and a frame whose speed no longer applies. The slot is freed with a write only while `table_now`
    /// (`*(main+0x52c3610)` now) is still the table it was taken from and the slot still holds `{1, divisor}`; otherwise
    /// the record goes without a write.
    pub fn release(&mut self, table_now: usize, bytes: Option<&mut dyn Bytes>) -> LobbyStep {
        self.paced = false;
        let Some(held) = self.held.take() else { return LobbyStep::Nothing };
        let freed = table_now != 0 && table_now == held.table && bytes.is_some_and(|table| free(table, held.slot, held.divisor));
        if freed {
            LobbyStep::Freed(held)
        } else {
            LobbyStep::Dropped(held)
        }
    }

    /// Forget the slot without a write.
    #[cfg(test)]
    pub fn forget(&mut self) -> Option<Held> {
        self.paced = false;
        self.held.take()
    }
}

#[cfg(test)]
#[path = "game_speed_tests.rs"]
mod tests;
