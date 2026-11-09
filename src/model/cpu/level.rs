//! The mod's CPU level (rc.6): Off, or Lv 1–9 for the CPUs of native Training and of the waiting room's practice. The
//! rules are here; `runtime::features::cpu_level` reads the game and applies them.
//!
//! Facts the rules encode (13.0.5; `research/lobby-cpu-settings/`, seam rows `cpu_level.*`):
//! - A CPU's level is its AI rank (`ai+0xc0c`). The setter `main+0x6b8400(controller, rank)` and the entry's copy at
//!   `FE+0x37` are written together by every native writer. Lv 1 is rank 0; Lv 2..=9 are the table at `main+0x4470dc0`.
//! - Training's pause menu holds CPU Behavior at `+0xb4c` and Training's own level at `+0xb50` (index 0..=8). Each
//!   Training entry starts the level from the CPU's character-select level. `main+0x155c020` gives the CPUs that level's
//!   rank only for CPU (Lv. n) (2) and Control (8); every scripted behavior runs at rank 100.
//! - The waiting room's partners are entries 4..=6. Their rank is the game's own and rises in play
//!   (`main+0x151d0a0`, `0x151d370`, `0x151c2f0`, `0x151da00`).

/// The setting row's id.
pub const ROW_ID: &str = "cpu_level";
/// The row's options: Off, then Lv 1..=9.
pub const OPTIONS: usize = 10;
/// The notice plate's header.
pub const NOTICE_HEADER: &str = "CPU level";

/// 13.0.5's rank table (`main+0x4470dc0`): the ranks of Lv 2..=9.
pub const RANK_TABLE_13_0_5: [i32; 8] = [15, 21, 31, 42, 48, 60, 75, 100];

/// Training's CPU Behavior values (the pause menu's `+0xb4c`) that use Training's own level.
pub mod behavior {
    /// Stop, the value a Training entry starts with.
    pub const STOP: u32 = 1;
    /// CPU (Lv. n).
    pub const CPU_LEVEL: u32 = 2;
    /// Control: the CPU is driven by a controller, at the rank of Training's level.
    pub const CONTROL: u32 = 8;
}

/// The waiting room's CPU entries: the practice partners.
pub const LOBBY_ENTRIES: core::ops::RangeInclusive<i32> = 4..=6;

/// The setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuLevel {
    /// The game's own level everywhere, and no notice.
    Off,
    /// Lv 1..=9.
    Lv(u8),
}

impl CpuLevel {
    /// The row's value (a Single row: the first set option). Nothing set, or an option past the list, reads as Off.
    pub fn from_row(row: &[u8]) -> CpuLevel {
        match row.iter().position(|v| *v != 0) {
            Some(index @ 1..=9) => CpuLevel::Lv(index as u8),
            _ => CpuLevel::Off,
        }
    }
}

/// The AI rank of Lv `level` with the game's `table`; `None` when the table is not 13.0.5's or the level is not 1..=9.
pub fn rank_for(level: u8, table: [i32; 8]) -> Option<u8> {
    if table != RANK_TABLE_13_0_5 {
        return None;
    }
    match level {
        1 => Some(0),
        2..=9 => u8::try_from(table[usize::from(level) - 2]).ok(),
        _ => None,
    }
}

/// Training's own level from the menu's index (0..=8 is Lv 1..=9).
pub fn level_of_menu_index(index: u32) -> Option<u8> {
    (index <= 8).then_some(index as u8 + 1)
}

/// Where the game is, for the rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Where {
    /// Native Training, with the pause menu's CPU Behavior (0 while the menu is unreadable).
    Training { behavior: u32 },
    /// The waiting room's practice (LDN room or Quickplay lobby; never a match).
    Lobby,
    /// Anything else, every match included.
    Elsewhere,
}

/// The rank the mod wants on `entry`, or `None` to leave the game's own rank alone.
pub fn target(level: CpuLevel, place: Where, entry: i32, table: [i32; 8]) -> Option<u8> {
    let CpuLevel::Lv(level) = level else { return None };
    let applies = match place {
        Where::Lobby => LOBBY_ENTRIES.contains(&entry),
        Where::Training { behavior } => {
            (1..8).contains(&entry) && matches!(behavior, behavior::CPU_LEVEL | behavior::CONTROL)
        }
        Where::Elsewhere => false,
    };
    if applies { rank_for(level, table) } else { None }
}

/// One entry's override, the pilot's arm/release rule per entry: the native rank it replaced and the rank it wrote.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EntryRank {
    native: Option<u8>,
    applied: Option<u8>,
}

impl EntryRank {
    pub const fn new() -> Self {
        Self { native: None, applied: None }
    }

    /// One frame: `target` from [`target`], `current` the entry's AI rank now. Returns the rank to write, if any.
    pub fn step(&mut self, target: Option<u8>, current: u8) -> Option<u8> {
        match target {
            Some(rank) => {
                if current == rank {
                    return None;
                }
                // Anything but our own last write is the game's rank: remember it, then write ours.
                if self.applied != Some(current) {
                    self.native = Some(current);
                }
                self.applied = Some(rank);
                Some(rank)
            }
            None => {
                let applied = self.applied.take()?;
                let native = self.native.take();
                // Give back only while our rank still holds; a native write since then wins.
                if current == applied { native } else { None }
            }
        }
    }

    /// Our rank is on the entry (Off then has something to give back).
    pub const fn holds(&self) -> bool {
        self.applied.is_some()
    }

    /// The entry is gone (its fighter or the scene ended): nothing to give back.
    pub fn forget(&mut self) {
        *self = Self::new();
    }
}

/// The Training session clock: a session lasts while Training's player entry is present on consecutive polls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SessionClock {
    current: Option<u64>,
    next: u64,
}

impl SessionClock {
    pub const fn new() -> Self {
        Self { current: None, next: 1 }
    }

    /// One poll: the current session's id, or `None` outside a Training session (the lobby, a match, a menu).
    pub fn poll(&mut self, place: Where, player_present: bool) -> Option<u64> {
        if !(player_present && matches!(place, Where::Training { .. })) {
            self.current = None;
            return None;
        }
        if self.current.is_none() {
            self.current = Some(self.next);
            self.next = self.next.wrapping_add(1).max(1);
        }
        self.current
    }
}

/// A mismatch to show: the mod's level and Training's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Notice {
    pub level: u8,
    pub menu: u8,
}

/// The once-per-Training-session notice rule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoticeGate {
    judged: Option<u64>,
}

impl NoticeGate {
    pub const fn new() -> Self {
        Self { judged: None }
    }

    /// The last Training session judged (the diag line reports each new one).
    pub const fn judged(&self) -> Option<u64> {
        self.judged
    }

    /// One poll: `session` from the clock, the setting, and Training's own level (`None` while unreadable).
    pub fn poll(&mut self, session: Option<u64>, level: CpuLevel, menu_level: Option<u8>) -> Option<Notice> {
        let session = session?;
        if self.judged == Some(session) {
            return None;
        }
        // The session's start is the first poll that can read Training's level.
        let menu = menu_level?;
        self.judged = Some(session);
        match level {
            CpuLevel::Lv(level) if level != menu => Some(Notice { level, menu }),
            _ => None,
        }
    }
}

/// The plate's body: "Lv 7 from TrainingRooms (Training menu: Lv 3)" under [`NOTICE_HEADER`].
pub fn notice_body(notice: Notice, out: &mut impl core::fmt::Write) -> core::fmt::Result {
    write!(out, "Lv {} from TrainingRooms (Training menu: Lv {})", notice.level, notice.menu)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::menu::table::{Control, Page, Vis, ROWS};
    use crate::model::settings::{self, Kind, Settings};

    const T: [i32; 8] = RANK_TABLE_13_0_5;
    const RANKS: [u8; 9] = [0, 15, 21, 31, 42, 48, 60, 75, 100];
    const PLACES: [Where; 4] =
        [Where::Training { behavior: behavior::CPU_LEVEL }, Where::Training { behavior: behavior::CONTROL }, Where::Lobby, Where::Elsewhere];

    fn row_of(index: usize) -> Vec<u8> {
        let mut row = vec![0u8; OPTIONS];
        row[index] = 1;
        row
    }

    fn body(notice: Notice) -> String {
        let mut text = String::new();
        notice_body(notice, &mut text).unwrap();
        text
    }

    #[test]
    fn cpu_level_row_defaults_to_off_and_round_trips() {
        let (_, def) = settings::def(ROW_ID).expect("the cpu_level row");
        assert_eq!((def.kind, def.len), (Kind::Single, OPTIONS));
        assert_eq!(def.default, row_of(0).as_slice(), "factory Off");
        let mut file = Settings::factory();
        assert_eq!(CpuLevel::from_row(file.menu.get(ROW_ID).unwrap()), CpuLevel::Off);
        file.menu.set(ROW_ID, &row_of(7)).unwrap();
        let loaded = settings::parse(&file.to_json());
        assert!(!loaded.corrupt(), "{:?}", loaded.issues);
        assert_eq!(CpuLevel::from_row(loaded.settings.menu.get(ROW_ID).unwrap()), CpuLevel::Lv(7));
        assert_eq!(CpuLevel::from_row(loaded.settings.defaults.get(ROW_ID).unwrap()), CpuLevel::Off, "saved defaults stay Off");
    }

    #[test]
    fn cpu_level_reads_off_and_levels_from_the_row() {
        assert_eq!(CpuLevel::from_row(&row_of(0)), CpuLevel::Off);
        for level in 1..=9u8 {
            assert_eq!(CpuLevel::from_row(&row_of(usize::from(level))), CpuLevel::Lv(level));
        }
        assert_eq!(CpuLevel::from_row(&[0; OPTIONS]), CpuLevel::Off, "nothing set");
        assert_eq!(CpuLevel::from_row(&[]), CpuLevel::Off, "no row");
        let mut long = vec![0u8; 12];
        long[11] = 1;
        assert_eq!(CpuLevel::from_row(&long), CpuLevel::Off, "an option past the list");
    }

    #[test]
    fn cpu_level_row_is_a_level_dropdown_on_the_cpu_page() {
        let row = ROWS.iter().find(|row| row.id == ROW_ID).expect("the CPU Level row");
        assert_eq!(row.title, "CPU Level");
        assert_eq!(row.page(), Page::Cpu);
        assert_eq!(row.vis, Vis::Always);
        let Control::Dropdown(options) = row.control else { panic!("{:?}", row.control) };
        assert_eq!(options, ["Off", "1", "2", "3", "4", "5", "6", "7", "8", "9"]);
        assert_eq!(options.len(), OPTIONS);
        // The CPU page's other level row has the same control and the same level labels.
        let p1 = ROWS.iter().find(|row| row.id == "p1_cpu_level").unwrap();
        let Control::Dropdown(p1_options) = p1.control else { panic!() };
        assert_eq!(&options[1..], p1_options);
        assert_eq!(p1.page(), Page::Cpu);
    }

    #[test]
    fn ranks_match_the_native_table() {
        for level in 1..=9u8 {
            assert_eq!(rank_for(level, T), Some(RANKS[usize::from(level) - 1]), "Lv {level}");
        }
        assert_eq!(rank_for(0, T), None);
        assert_eq!(rank_for(10, T), None);
        let mut other = T;
        other[3] += 1;
        assert_eq!(rank_for(5, other), None, "another table refuses every level");
        for index in 0..9u32 {
            assert_eq!(level_of_menu_index(index), Some(index as u8 + 1));
        }
        assert_eq!(level_of_menu_index(9), None);
    }

    #[test]
    fn off_never_targets_a_rank() {
        for place in PLACES {
            for entry in -1..9 {
                assert_eq!(target(CpuLevel::Off, place, entry, T), None, "{place:?} entry {entry}");
            }
        }
    }

    #[test]
    fn lobby_partners_get_the_level() {
        for entry in LOBBY_ENTRIES {
            assert_eq!(target(CpuLevel::Lv(7), Where::Lobby, entry, T), Some(60), "entry {entry}");
            assert_eq!(target(CpuLevel::Lv(1), Where::Lobby, entry, T), Some(0), "entry {entry}");
        }
        for entry in [0, 1, 2, 3, 7] {
            assert_eq!(target(CpuLevel::Lv(7), Where::Lobby, entry, T), None, "entry {entry} is no partner");
        }
        assert_eq!(target(CpuLevel::Lv(7), Where::Lobby, 4, [0; 8]), None, "not the 13.0.5 table");
    }

    #[test]
    fn training_override_applies_where_training_uses_its_level() {
        for behavior in [behavior::CPU_LEVEL, behavior::CONTROL] {
            for entry in 1..8 {
                assert_eq!(target(CpuLevel::Lv(7), Where::Training { behavior }, entry, T), Some(60), "{behavior} entry {entry}");
                assert_eq!(target(CpuLevel::Lv(9), Where::Training { behavior }, entry, T), Some(100));
            }
        }
    }

    #[test]
    fn scripted_training_behaviors_keep_the_native_rank() {
        for behavior in [0, behavior::STOP, 3, 4, 5, 6, 7, 9] {
            for entry in 1..8 {
                assert_eq!(target(CpuLevel::Lv(7), Where::Training { behavior }, entry, T), None, "{behavior} entry {entry}");
            }
        }
    }

    #[test]
    fn player_entry_is_never_a_target() {
        for place in PLACES {
            for level in 1..=9 {
                for entry in [0, -1, 8, 9] {
                    assert_eq!(target(CpuLevel::Lv(level), place, entry, T), None, "{place:?} Lv {level} entry {entry}");
                }
            }
        }
    }

    #[test]
    fn nothing_applies_in_a_match() {
        for level in 1..=9 {
            for entry in -1..9 {
                assert_eq!(target(CpuLevel::Lv(level), Where::Elsewhere, entry, T), None);
            }
        }
        let mut clock = SessionClock::new();
        assert_eq!(clock.poll(Where::Elsewhere, true), None, "a match is never a Training session");
    }

    #[test]
    fn override_returns_after_a_native_change() {
        let mut entry = EntryRank::new();
        assert_eq!(entry.step(Some(60), 21), Some(60), "Training's Lv 3 becomes the mod's Lv 7");
        assert_eq!(entry.step(Some(60), 60), None, "held");
        // The pause menu (or the lobby's own rank-up) writes a new native rank: the override returns the next frame.
        assert_eq!(entry.step(Some(60), 31), Some(60));
        assert_eq!(entry.step(Some(60), 60), None);
        // The mod's level changes (Lv 7 to Lv 9): our own 60 is not the game's rank.
        assert_eq!(entry.step(Some(100), 60), Some(100));
        assert_eq!(entry.step(Some(100), 100), None);
        // ... and Off gives back the latest native rank.
        assert_eq!(entry.step(None, 100), Some(31));
    }

    #[test]
    fn switching_to_off_restores_the_native_rank() {
        let mut entry = EntryRank::new();
        assert_eq!(entry.step(None, 21), None, "Off from the start writes nothing");
        assert_eq!(entry.step(Some(60), 21), Some(60));
        assert_eq!(entry.step(None, 60), Some(21), "Off: Training's own rank comes back");
        assert_eq!(entry.step(None, 21), None, "once");
        // A native change after the override wins: nothing is written over it.
        assert_eq!(entry.step(Some(60), 21), Some(60));
        assert_eq!(entry.step(None, 100), None, "the behavior changed to a scripted one: the game's 100 stays");
        assert_eq!(entry.step(None, 100), None);
        // A level of 0 is a real rank (Lv 1), not "unset".
        let mut low = EntryRank::new();
        assert_eq!(low.step(Some(100), 0), Some(100));
        assert_eq!(low.step(None, 100), Some(0));
        low.step(Some(60), 0);
        low.forget();
        assert_eq!(low.step(None, 60), None, "forgotten: nothing to give back");
    }

    #[test]
    fn notice_shows_once_per_training_session() {
        let mut gate = NoticeGate::new();
        assert_eq!(gate.poll(Some(1), CpuLevel::Lv(7), None), None, "Training's level not readable yet");
        assert_eq!(gate.poll(Some(1), CpuLevel::Lv(7), Some(3)), Some(Notice { level: 7, menu: 3 }));
        for _ in 0..3 {
            assert_eq!(gate.poll(Some(1), CpuLevel::Lv(7), Some(3)), None);
        }
    }

    #[test]
    fn notice_not_again_after_a_mid_session_change() {
        let mut gate = NoticeGate::new();
        assert!(gate.poll(Some(4), CpuLevel::Lv(7), Some(3)).is_some());
        assert_eq!(gate.poll(Some(4), CpuLevel::Lv(7), Some(5)), None, "Training's level changed");
        assert_eq!(gate.poll(Some(4), CpuLevel::Lv(9), Some(5)), None, "the mod's level changed");
        assert_eq!(gate.poll(Some(4), CpuLevel::Lv(9), Some(3)), None);
    }

    #[test]
    fn notice_again_on_the_next_training_entry() {
        let mut gate = NoticeGate::new();
        let mut clock = SessionClock::new();
        let first = clock.poll(Where::Training { behavior: behavior::STOP }, true);
        assert!(gate.poll(first, CpuLevel::Lv(7), Some(3)).is_some());
        // The character select: no Training entries.
        assert_eq!(gate.poll(clock.poll(Where::Training { behavior: 0 }, false), CpuLevel::Lv(7), None), None);
        let second = clock.poll(Where::Training { behavior: behavior::STOP }, true);
        assert_ne!(second, first);
        assert_eq!(gate.poll(second, CpuLevel::Lv(7), Some(3)), Some(Notice { level: 7, menu: 3 }));
        assert_eq!(gate.poll(second, CpuLevel::Lv(7), Some(3)), None);
    }

    #[test]
    fn same_level_gives_no_notice() {
        let mut gate = NoticeGate::new();
        assert_eq!(gate.poll(Some(1), CpuLevel::Lv(7), Some(7)), None);
        assert_eq!(gate.poll(Some(1), CpuLevel::Lv(7), Some(3)), None, "a later change in the session is not the start");
        assert_eq!(gate.poll(Some(2), CpuLevel::Lv(3), Some(3)), None);
    }

    #[test]
    fn off_never_posts_the_notice() {
        let mut gate = NoticeGate::new();
        for menu in 1..=9 {
            assert_eq!(gate.poll(Some(u64::from(menu)), CpuLevel::Off, Some(menu)), None);
        }
        assert_eq!(gate.poll(Some(9), CpuLevel::Lv(7), Some(9)), None, "Off at the start judged the session");
        assert_eq!(gate.poll(Some(10), CpuLevel::Off, Some(3)), None);
        assert_eq!(gate.poll(Some(10), CpuLevel::Lv(7), Some(3)), None, "switched on mid-session: not the start");
    }

    #[test]
    fn lobby_never_posts_a_notice() {
        let mut gate = NoticeGate::new();
        let mut clock = SessionClock::new();
        for _ in 0..5 {
            let session = clock.poll(Where::Lobby, true);
            assert_eq!(session, None, "the lobby is no Training session");
            assert_eq!(gate.poll(session, CpuLevel::Lv(7), Some(3)), None);
        }
    }

    #[test]
    fn notice_body_names_both_levels() {
        assert_eq!(NOTICE_HEADER, "CPU level");
        assert_eq!(body(Notice { level: 7, menu: 3 }), "Lv 7 from TrainingRooms (Training menu: Lv 3)");
        assert_eq!(body(Notice { level: 1, menu: 9 }), "Lv 1 from TrainingRooms (Training menu: Lv 9)");
        assert!(body(Notice { level: 9, menu: 1 }).len() <= crate::model::notify::BODY_MAX);
    }

    #[test]
    fn session_clock_starts_a_new_session_after_a_gap() {
        let mut clock = SessionClock::new();
        let training = Where::Training { behavior: behavior::STOP };
        let a = clock.poll(training, true);
        assert!(a.is_some());
        assert_eq!(clock.poll(Where::Training { behavior: behavior::CPU_LEVEL }, true), a, "a behavior change keeps the session");
        assert_eq!(clock.poll(Where::Training { behavior: 0 }, true), a, "the pause menu keeps the entries");
        assert_eq!(clock.poll(training, false), None, "no player entry: the session ended");
        let b = clock.poll(training, true);
        assert!(b.is_some() && b != a);
        assert_eq!(clock.poll(Where::Lobby, true), None);
        let c = clock.poll(training, true);
        assert!(c.is_some() && c != b, "the lobby in between ends it too");
    }
}
