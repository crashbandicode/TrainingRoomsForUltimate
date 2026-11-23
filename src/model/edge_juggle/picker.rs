//! Fighter, behavior and settings shared by the two practice games.
use core::fmt::{self, Write};

use crate::model::{fighter_choices::CHOICES, settings::Snapshot, tech_settings::selected};

use super::Drill;

/// Kinds that are forms of one fighter (the native form groups). A lib test checks this against `character_forms`.
const FORM_GROUPS: [&[u32]; 2] = [&[0x24, 0x25, 0x26], &[0x5a, 0x5b]];

fn is_form_of(selected: u32, live: u32) -> bool {
    FORM_GROUPS.iter().any(|group| group.contains(&selected) && group.contains(&live))
}

/// The selection that has been live in the CPU slot since it became the selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Settled(Option<u32>);

impl Settled {
    pub const fn new(kind: Option<u32>) -> Self {
        Self(kind)
    }

    pub const fn kind(self) -> Option<u32> {
        self.0
    }

    /// One look at the slot. `target`: the kind wanted now, if any; `live`: the kind in the slot, when known. The
    /// target settles when it is seen live and stays settled while it stays the target.
    pub fn see(self, target: Option<u32>, live: Option<u32>) -> Self {
        match target {
            Some(target) if live == Some(target) || self.0 == Some(target) => Self(Some(target)),
            _ => Self(None),
        }
    }

    pub fn is(self, selected: u32) -> bool {
        self.0 == Some(selected)
    }
}

/// Whether the CPU in the slot answers the drill's selection. The selected fighter itself always does. Another of
/// its forms does only once the selection has been in the slot (`settled`): then the change was the fighter's own
/// (Pyra's swap to Mythra), not a pick of the other form.
pub fn answers(selected: u32, live: u32, settled: bool) -> bool {
    live == selected || settled && is_form_of(selected, live)
}

/// One roster tick of the selection service.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Service {
    /// The original CPU is being restored and is not in the slot yet.
    pub switching: bool,
    /// Ask the roster for `target`.
    pub request: bool,
}

/// `selected`: the drill's choice, if any; `target`: that choice or the original to restore; `observed`: the CPU
/// kind the roster sees when it is ready for a request; `settled`: the target has been live since it became the target.
pub fn service(selected: Option<u32>, target: u32, observed: Option<u32>, settled: bool) -> Service {
    let satisfied = observed.map(|kind| answers(target, kind, settled));
    Service { switching: selected.is_none() && satisfied != Some(true), request: satisfied == Some(false) }
}

/// One roster tick of the selection service, decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tick {
    /// Nothing selected and nothing to restore.
    Idle,
    /// Outside practice (Training or a waiting room): the service forgets everything.
    Left,
    /// Another practice game owns the opponent until it stops.
    OtherGame,
    /// Compare the slot with `target` and act on `step`.
    Serve { target: u32, observed: Option<u32>, step: Service },
}

/// `selected`: the drill's choice; `original`: the fighter to restore when there is no choice; `other_game`: another
/// practice game owns the opponent (asked only when nothing is selected); `observed`: the roster's CPU kind when it
/// is ready for a request (asked only when the slot is compared). The settlement is the selection's: without a
/// selection, or outside Training, there is none, and restoring the original compares exactly.
pub fn tick(
    settled: Settled,
    selected: Option<u32>,
    original: Option<u32>,
    training: bool,
    other_game: impl FnOnce() -> bool,
    observed: impl FnOnce() -> Option<u32>,
) -> (Settled, Tick) {
    let Some(target) = selected.or(original) else { return (Settled::default(), Tick::Idle) };
    if !training {
        return (Settled::default(), Tick::Left);
    }
    let Some(choice) = selected else {
        if other_game() {
            return (Settled::default(), Tick::OtherGame);
        }
        let observed = observed();
        return (Settled::default(), Tick::Serve { target, observed, step: service(None, target, observed, false) });
    };
    let observed = observed();
    let settled = settled.see(Some(choice), observed);
    (settled, Tick::Serve { target, observed, step: service(selected, target, observed, settled.is(choice)) })
}

/// What the service compares the slot with (`tick`'s `observed`): the roster's `ready` read. Stub of 480345a6: a
/// picker's hold on the CPU's body is not asked about.
pub fn observable(_selected: Option<u32>, _held: impl FnOnce() -> bool, ready: impl FnOnce() -> Option<u32>) -> Option<u32> {
    ready()
}

pub const HERO: u32 = 0x53;
pub const FIGHTER_CHOICES: usize = CHOICES.len() + 1;
pub const MODE_CHOICES: usize = 3;
pub const DEFAULT_WINDOW: u8 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrillId {
    Zoom,
}

pub fn drills(fighter: Option<u32>) -> &'static [DrillId] {
    match fighter {
        Some(HERO) => &[DrillId::Zoom],
        _ => &[],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Cpu(u8),
    Perfect,
    Drill(DrillId),
}

impl Mode {
    pub fn write(self, out: &mut impl Write) -> fmt::Result {
        match self {
            Self::Cpu(level) => write!(out, "CPU {level}"),
            Self::Perfect => out.write_str("Perfect"),
            Self::Drill(DrillId::Zoom) => out.write_str("Zoom"),
        }
    }

    pub const fn stock_level(self) -> u8 {
        match self {
            Self::Cpu(level) => level,
            Self::Perfect | Self::Drill(_) => 9,
        }
    }
}

pub struct Rows {
    pub fighter: &'static str,
    pub level: &'static str,
    pub window: &'static str,
}

pub const fn rows(game: Drill) -> &'static Rows {
    match game {
        Drill::GuardEdge => &Rows {
            fighter: "edge_cpu_fighter",
            level: "edge_cpu_level",
            window: "edge_zoom_window",
        },
        Drill::Juggler => &Rows {
            fighter: "juggle_cpu_fighter",
            level: "juggle_cpu_level",
            window: "juggle_zoom_window",
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Picker {
    pub fighter: Option<u32>,
    pub level: u8,
    pub choice: usize,
}

impl Picker {
    pub const DEFAULT: Self = Self { fighter: None, level: 9, choice: 0 };

    pub fn read(snapshot: &Snapshot, game: Drill) -> Self {
        let rows = rows(game);
        let mut pick = Self {
            fighter: selected(snapshot, rows.fighter)
                .checked_sub(1)
                .and_then(|index| CHOICES.get(index))
                .map(|&(kind, _)| kind),
            level: (selected(snapshot, rows.level).min(8) + 1) as u8,
            choice: selected(snapshot, game.difficulty_rows().difficulty),
        };
        pick.select_fighter(pick.fighter);
        pick
    }

    pub fn select_fighter(&mut self, fighter: Option<u32>) {
        self.fighter = fighter;
        if self.choice >= self.count() {
            self.choice = 0;
        }
    }

    pub fn fighter_index(self) -> usize {
        CHOICES.iter().position(|&(kind, _)| Some(kind) == self.fighter).map_or(0, |i| i + 1)
    }

    pub fn count(self) -> usize {
        2 + drills(self.fighter).len()
    }

    pub fn mode_at(self, choice: usize) -> Mode {
        match choice {
            1 => Mode::Perfect,
            _ => choice.checked_sub(2)
                .and_then(|i| drills(self.fighter).get(i))
                .map_or(Mode::Cpu(self.level), |&drill| Mode::Drill(drill)),
        }
    }

    pub fn mode(self) -> Mode {
        self.mode_at(self.choice)
    }

    pub fn cycle_mode(&mut self, forward: bool) {
        let count = self.count();
        self.choice = (self.choice + if forward { 1 } else { count - 1 }) % count;
    }

    pub fn cycle_level(&mut self) {
        if matches!(self.mode(), Mode::Cpu(_)) {
            self.level = self.level % 9 + 1;
        }
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;

    #[test]
    fn form_groups_match_the_native_descriptor_table() {
        use crate::engine::swap::character_forms::Forms;
        for &(a, _) in CHOICES {
            for &(b, _) in CHOICES {
                assert_eq!(a == b || is_form_of(a, b), Forms::starting(a).same_group(b), "{a:#x}, {b:#x}");
            }
        }
    }

    #[test]
    fn a_form_change_answers_only_a_settled_selection() {
        assert!(answers(0x5a, 0x5a, false));
        assert!(!answers(0x5a, 0x5b, false));
        assert!(answers(0x5a, 0x5b, true));
        assert!(answers(0x5b, 0x5a, true));
        assert!(!answers(0x5b, 0x5a, false));
        assert!(answers(0x24, 0x26, true));
        assert!(!answers(0x24, 0x26, false));
        assert!(!answers(0x5a, 0x00, true));
        assert!(answers(0x00, 0x00, false));
    }

    #[test]
    fn the_selection_settles_when_it_is_live_and_unsettles_when_it_changes() {
        const PYRA: u32 = 0x5a;
        const MYTHRA: u32 = 0x5b;
        const MARIO: u32 = 0x00;
        let mut settled = Settled::default().see(Some(PYRA), Some(MARIO));
        assert!(!settled.is(PYRA));
        settled = settled.see(Some(PYRA), None);
        assert!(!settled.is(PYRA));
        settled = settled.see(Some(PYRA), Some(PYRA));
        assert!(settled.is(PYRA));
        settled = settled.see(Some(PYRA), Some(MYTHRA));
        assert!(settled.is(PYRA));
        settled = settled.see(Some(PYRA), None);
        assert!(settled.is(PYRA));
        settled = settled.see(Some(MYTHRA), Some(MYTHRA));
        assert!(settled.is(MYTHRA));
        assert!(!settled.is(PYRA));
        settled = settled.see(Some(PYRA), Some(MYTHRA));
        assert!(!settled.is(PYRA));
        settled = settled.see(None, None);
        assert_eq!(settled.kind(), None);
        settled = settled.see(Some(PYRA), Some(PYRA));
        assert!(settled.is(PYRA));
        settled = settled.see(Some(MARIO), Some(MYTHRA));
        assert_eq!(settled.kind(), None);
        settled = settled.see(Some(PYRA), Some(MYTHRA));
        assert!(!settled.is(PYRA));
    }

    #[test]
    fn selection_service_requests_a_picked_form_and_not_an_own_swap() {
        let no_request = Service { switching: false, request: false };
        let request = Service { switching: false, request: true };
        let live = |s: Settled, target, observed| service(Some(target), target, observed, s.see(Some(target), observed).is(target));
        let pyra = Settled::default().see(Some(0x5a), Some(0x5a));
        assert_eq!(live(pyra, 0x5a, Some(0x5b)), no_request);
        assert_eq!(live(pyra, 0x5b, Some(0x5a)), request);
        assert_eq!(live(Settled::default(), 0x5a, Some(0x5b)), request);
        assert_eq!(live(pyra, 0x5a, Some(0x00)), request);
        assert_eq!(live(Settled::default(), 0x5a, Some(0x00)), request);
        assert_eq!(live(pyra, 0x5a, None), no_request);
        assert_eq!(service(None, 0x00, Some(0x5b), false), Service { switching: true, request: true });
        assert_eq!(service(None, 0x00, None, false), Service { switching: true, request: false });
        assert_eq!(service(None, 0x00, Some(0x00), false), no_request);
    }

    #[test]
    fn a_busy_roster_keeps_the_settlement() {
        const PYRA: u32 = 0x5a;
        const MYTHRA: u32 = 0x5b;
        const MARIO: u32 = 0x00;
        let (mut settled, _) = tick(Settled::default(), Some(PYRA), Some(MARIO), true, || false, || Some(PYRA));
        assert!(settled.is(PYRA));
        let (next, step) = tick(settled, Some(PYRA), Some(MARIO), true, || false, || Some(MYTHRA));
        settled = next;
        assert_eq!(step, Tick::Serve { target: PYRA, observed: Some(MYTHRA), step: Service { switching: false, request: false } });
        for _ in 0..3 {
            let (next, step) = tick(settled, Some(PYRA), Some(MARIO), true, || false, || None);
            settled = next;
            assert_eq!(step, Tick::Serve { target: PYRA, observed: None, step: Service { switching: false, request: false } });
            assert!(settled.is(PYRA));
        }
        let (settled, step) = tick(settled, Some(PYRA), Some(MARIO), true, || false, || Some(MYTHRA));
        assert!(settled.is(PYRA));
        assert_eq!(step, Tick::Serve { target: PYRA, observed: Some(MYTHRA), step: Service { switching: false, request: false } });
    }

    #[test]
    fn another_game_ends_the_settlement() {
        const PYRA: u32 = 0x5a;
        const MYTHRA: u32 = 0x5b;
        const MARIO: u32 = 0x00;
        let (settled, _) = tick(Settled::default(), Some(PYRA), Some(MARIO), true, || false, || Some(PYRA));
        assert!(settled.is(PYRA));
        let (settled, step) = tick(settled, Some(PYRA), Some(MARIO), true, || false, || Some(MYTHRA));
        assert_eq!(step, Tick::Serve { target: PYRA, observed: Some(MYTHRA), step: Service { switching: false, request: false } });
        let (settled, step) = tick(settled, None, Some(MARIO), true, || true, || panic!("slot not compared"));
        assert_eq!(step, Tick::OtherGame);
        assert_eq!(settled.kind(), None);
        let (_, step) = tick(settled, Some(PYRA), Some(MARIO), true, || false, || Some(MYTHRA));
        assert_eq!(step, Tick::Serve { target: PYRA, observed: Some(MYTHRA), step: Service { switching: false, request: true } });
    }

    #[test]
    fn restoring_the_original_compares_exactly() {
        const PYRA: u32 = 0x5a;
        const MYTHRA: u32 = 0x5b;
        let (settled, _) = tick(Settled::default(), Some(PYRA), Some(PYRA), true, || false, || Some(PYRA));
        assert!(settled.is(PYRA));
        let (settled, step) = tick(settled, Some(PYRA), Some(PYRA), true, || false, || Some(MYTHRA));
        assert_eq!(step, Tick::Serve { target: PYRA, observed: Some(MYTHRA), step: Service { switching: false, request: false } });
        let (settled, step) = tick(settled, None, Some(PYRA), true, || false, || Some(MYTHRA));
        assert_eq!(step, Tick::Serve { target: PYRA, observed: Some(MYTHRA), step: Service { switching: true, request: true } });
        assert_eq!(settled.kind(), None);
        let (settled, step) = tick(settled, None, Some(PYRA), true, || false, || Some(PYRA));
        assert_eq!(step, Tick::Serve { target: PYRA, observed: Some(PYRA), step: Service { switching: false, request: false } });
        assert_eq!(settled.kind(), None);
    }

    #[test]
    fn the_original_is_not_asked_for_while_a_picker_holds_the_cpu() {
        const DONKEY: u32 = 0x01;
        const ZERO_SUIT: u32 = 0x20;
        // The picker was left: nothing is selected, Donkey Kong is in the slot and Zero Suit Samus is to come back.
        let mut reads = 0;
        let mut serve = |held: bool| {
            tick(Settled::default(), None, Some(ZERO_SUIT), true, || false, || {
                observable(None, || held, || {
                    reads += 1;
                    Some(DONKEY)
                })
            })
            .1
        };
        let held = serve(true);
        let let_go = serve(false);
        // The roster hands the outgoing body's place to the incoming fighter. While the picker holds Donkey Kong at
        // its picked start, the original is not asked for and the roster is not read.
        assert_eq!(held, Tick::Serve { target: ZERO_SUIT, observed: None, step: Service { switching: true, request: false } });
        // The body is back and let go: the original is asked for.
        assert_eq!(let_go, Tick::Serve { target: ZERO_SUIT, observed: Some(DONKEY), step: Service { switching: true, request: true } });
        assert_eq!(reads, 1);
    }

    #[test]
    fn a_selection_is_compared_whatever_the_picker_holds() {
        const DONKEY: u32 = 0x01;
        // The drill's own choice is in the slot and its picker holds it: the slot is read, the hold is not asked about.
        assert_eq!(observable(Some(DONKEY), || panic!("the hold is not asked about"), || Some(DONKEY)), Some(DONKEY));
        assert_eq!(observable(Some(DONKEY), || panic!("the hold is not asked about"), || None), None);
        // Nothing selected and nothing held: the roster's read, ready or not.
        assert_eq!(observable(None, || false, || Some(DONKEY)), Some(DONKEY));
        assert_eq!(observable(None, || false, || None), None);
    }

    #[test]
    fn leaving_training_ends_the_settlement() {
        const PYRA: u32 = 0x5a;
        const MYTHRA: u32 = 0x5b;
        let (settled, _) = tick(Settled::default(), Some(PYRA), Some(0x00), true, || false, || Some(PYRA));
        assert!(settled.is(PYRA));
        let (settled, step) = tick(settled, Some(PYRA), Some(0x00), false, || panic!("other game not asked"), || panic!("slot not compared"));
        assert_eq!(step, Tick::Left);
        assert_eq!(settled.kind(), None);
        let (_, step) = tick(settled, Some(PYRA), Some(0x00), true, || false, || Some(MYTHRA));
        assert_eq!(step, Tick::Serve { target: PYRA, observed: Some(MYTHRA), step: Service { switching: false, request: true } });
    }

    #[test]
    fn an_idle_service_keeps_nothing() {
        let (settled, step) = tick(Settled::new(Some(0x5a)), None, None, true, || panic!("other game not asked"), || panic!("slot not compared"));
        assert_eq!(step, Tick::Idle);
        assert_eq!(settled.kind(), None);
    }

    #[test]
    fn the_other_game_is_asked_only_without_a_selection() {
        let mut observed_calls = 0;
        let (settled, step) = tick(Settled::default(), Some(0x5a), None, true, || panic!("other game not asked"), || {
            observed_calls += 1;
            Some(0x5a)
        });
        assert_eq!(observed_calls, 1);
        assert!(settled.is(0x5a));
        assert_eq!(step, Tick::Serve { target: 0x5a, observed: Some(0x5a), step: Service { switching: false, request: false } });
    }
    }
}
