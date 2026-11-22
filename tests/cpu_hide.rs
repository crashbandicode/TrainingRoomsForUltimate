//! Hide CPU (rc.14, SPEC-eggsercise V2.6): the rule, the restore, the edges and the waiting-room gate. The model decides
//! once per body frame (`Cpu::frame` -> `FrameOut::vanish`); `runtime::features::hide_cpu` applies it (Eden rows `hide-E34`..).

use crate::model;

use model::cpu::defence::Defence;
use model::cpu::facts::{FrameFacts, FrameOut};
use model::cpu::hide;
use model::cpu::role::{FighterKey, Scene};
use model::cpu::settings::CpuSettings;
use model::cpu::state::Cpu;
use model::eggs;
use model::menu::table::ROWS;
use model::menu::{Env, Menu};
use model::settings::{Settings, Snapshot};

const ON: [u8; 2] = [1, 0];

fn cpu(rows: &[(&str, &[u8])]) -> Cpu {
    let mut snap = Snapshot::factory();
    for (id, row) in rows {
        snap.set(id, row).unwrap();
    }
    let mut cpu = Cpu::new(CpuSettings::from_snapshot(&snap), 5);
    for fighter in &mut cpu.fighters {
        fighter.pending = 0;
    }
    cpu
}

fn body() -> FighterKey {
    FighterKey::new(1, false).unwrap()
}

/// One CPU frame of `key`, with a practice game's request `practice`.
fn frame(cpu: &mut Cpu, key: FighterKey, practice: bool) -> FrameOut {
    // A write the CPU page asks for on every frame, so a frame that ran the behaviours shows it.
    let penalty_disabled = !Defence::stale_flag_wanted(&cpu.settings);
    cpu.frame(key, &FrameFacts { penalty_disabled, request: practice, ..FrameFacts::default() })
}

fn runs_behaviours(out: &FrameOut) -> bool {
    out.stale_flag.is_some()
}

/// A hidden body's whole answer: nothing but the hide itself.
fn hidden() -> FrameOut {
    FrameOut { vanish: Some(true), ..FrameOut::default() }
}

#[test]
fn hide_cpu_rule() {
    for scene in [Scene::Training, Scene::Lobby] {
        for (row, practice) in [(false, false), (true, false), (false, true), (true, true)] {
            // rc.14 (11-15): the same rule in the waiting room's practice as in Training.
            let want = if row || practice { Some(true) } else { None };
            assert_eq!(hide::step(&mut false, row, practice), want, "{scene:?} row {row} practice {practice}");
        }
    }
    // The factory has it Off; the CPU frame follows the rule for each body.
    let mut off = cpu(&[]);
    let out = frame(&mut off, body(), false);
    assert_eq!((out.vanish, runs_behaviours(&out)), (None, true));
    let mut on = cpu(&[("hide_cpu", &ON)]);
    let out = frame(&mut on, body(), false);
    assert_eq!((out.vanish, runs_behaviours(&out)), (Some(true), false), "hidden: no CPU behaviour runs");
    assert_eq!(out, hidden());
    // Hidden every frame (the writes are renewed, so a rebuilt body cannot show).
    assert_eq!(frame(&mut on, body(), false), hidden());
}

#[test]
fn hide_cpu_restore() {
    let mut cpu = cpu(&[("hide_cpu", &ON)]);
    assert_eq!(frame(&mut cpu, body(), false), hidden());
    let before = cpu.settings;
    // The row goes Off: shown on the next frame, once, and that same frame the CPU page applies.
    let mut snap = Snapshot::factory();
    snap.set("hide_cpu", &[0, 1]).unwrap();
    cpu.publish(CpuSettings::from_snapshot(&snap), 0);
    let out = frame(&mut cpu, body(), false);
    assert_eq!((out.vanish, runs_behaviours(&out)), (Some(false), true));
    assert_eq!(frame(&mut cpu, body(), false).vanish, None);
    // Hiding never wrote a CPU setting: the only difference is the row itself.
    assert_eq!(CpuSettings { hide_cpu: true, ..cpu.settings }, before);
}

#[test]
fn hide_cpu_edges() {
    let climbers = [body(), FighterKey::new(1, true).unwrap()];
    // A practice game's request hides with the row Off, and ending it shows (once).
    let mut game = cpu(&[]);
    for key in climbers {
        assert_eq!(frame(&mut game, key, false).vanish, None);
        assert_eq!(frame(&mut game, key, true), hidden(), "both Ice Climbers follow the rule together");
    }
    for key in climbers {
        assert_eq!(frame(&mut game, key, false).vanish, Some(false));
        assert_eq!(frame(&mut game, key, false).vanish, None);
    }
    // With Hide CPU On the CPU stays hidden when the game ends, and shows when the row goes Off.
    let mut both = cpu(&[("hide_cpu", &ON)]);
    assert_eq!(frame(&mut both, body(), true), hidden());
    assert_eq!(frame(&mut both, body(), false), hidden());
    // A body that took the old one's place (a swap, Cycle CPU, a Kirby hat, a reset) is hidden on its first frame:
    // the state belongs to the CPU's key, not to the game's body.
    both.fighters[body().index()].kind = 7;
    assert_eq!(frame(&mut both, body(), false), hidden());
    // The waiting room's practice follows the same rule (rc.14, 11-15): with the row On a body there stays hidden.
    both.scene = Scene::Lobby;
    assert_eq!(frame(&mut both, body(), false), hidden());
    // A save state load, L+R+A and opening the menu are not events of this model: no frame says anything but the rule.
}

#[test]
fn hide_cpu_in_the_waiting_room() {
    // rc.14 (the operator, 11-15 11:35Z): Hide CPU works in the waiting room's practice as in Training. The row hides the
    // room's CPU (entry 4), a practice game's request hides it, neither shows it; the menu row is a plain toggle there.
    let room_cpu = FighterKey::new(4, false).unwrap();
    let mut lobby = cpu(&[("hide_cpu", &ON)]);
    lobby.scene = Scene::Lobby;
    assert_eq!(frame(&mut lobby, room_cpu, false), hidden(), "the row hides it");
    let mut asked = cpu(&[]);
    asked.scene = Scene::Lobby;
    assert_eq!(frame(&mut asked, room_cpu, true), hidden(), "a practice game's request hides it");
    let out = frame(&mut asked, room_cpu, false);
    assert_eq!((out.vanish, runs_behaviours(&out)), (Some(false), true), "the request over: shown, behaviours run");
    let mut neither = cpu(&[]);
    neither.scene = Scene::Lobby;
    let out = frame(&mut neither, room_cpu, false);
    assert_eq!((out.vanish, runs_behaviours(&out)), (None, true), "neither: left alone");
    // Outside practice (a match, a stale lobby flag) no CPU frame runs at all: the scene is none.
    assert_eq!(model::scene::scene(model::scene::Facts::default()), None);
    assert_eq!(model::scene::scene(model::scene::Facts { lobby_active: true, ..Default::default() }), None);
    // The row: the CPU page's Level group, after CPU Level, Off by default, a plain toggle in Training and in a room.
    let row = ROWS.iter().position(|r| r.id == "hide_cpu").expect("the Hide CPU row");
    assert_eq!(ROWS[row - 1].id, "cpu_level");
    assert_eq!(ROWS[row].page(), model::menu::table::Page::Cpu);
    assert!(!CpuSettings::from_snapshot(&Snapshot::factory()).hide_cpu);
    let menu = |waiting_room| Menu::new(Settings::factory(), Env { waiting_room, ..Env::default() });
    assert_eq!([menu(false).grey(row), menu(true).grey(row)], [None, None]);
}

/// One frame's tag write and the HUD panels hidden after it.
fn tag_and_panels(cpu: &mut Cpu, key: FighterKey, practice: bool) -> (Option<bool>, u8) {
    let out = frame(cpu, key, practice);
    (hide::tag(out.vanish), cpu.hidden_panels())
}

/// The CPU entry's panel bit (entry 1 in Training).
const CPU_PANEL: u8 = 1 << 1;

#[test]
fn hide_cpu_tag_follows_the_body() {
    // The row: the tag (the game's two cursor flags) goes with the body on its first hidden frame, on every hidden frame
    // after (a rebuilt body cannot show it), and comes back on the frame the body is shown; untouched otherwise.
    let mut cpu = cpu(&[("hide_cpu", &ON)]);
    assert_eq!(tag_and_panels(&mut cpu, body(), false), (Some(false), CPU_PANEL));
    assert_eq!(tag_and_panels(&mut cpu, body(), false), (Some(false), CPU_PANEL));
    // A reset or swap rebuilds the body's state: its first frame hides tag and panel again.
    cpu.clear_session();
    assert_eq!(cpu.hidden_panels(), 0);
    assert_eq!(tag_and_panels(&mut cpu, body(), false), (Some(false), CPU_PANEL));
    let mut snap = Snapshot::factory();
    snap.set("hide_cpu", &[0, 1]).unwrap();
    cpu.publish(CpuSettings::from_snapshot(&snap), 0);
    assert_eq!(tag_and_panels(&mut cpu, body(), false), (Some(true), 0), "shown: the tag is back on the same frame");
    assert_eq!(tag_and_panels(&mut cpu, body(), false), (None, 0), "then the game owns it again");
    // A practice game's request (CPU during a game: Hide): the same, both Ice Climbers on the one entry.
    let mut game = cpu_with_factory();
    for key in [body(), FighterKey::new(1, true).unwrap()] {
        assert_eq!(tag_and_panels(&mut game, key, true), (Some(false), CPU_PANEL));
    }
    for key in [body(), FighterKey::new(1, true).unwrap()] {
        assert_eq!(tag_and_panels(&mut game, key, false).0, Some(true));
    }
    assert_eq!(game.hidden_panels(), 0);
    // Never shown or hidden: no tag write, no panel.
    let mut off = cpu_with_factory();
    assert_eq!(tag_and_panels(&mut off, body(), false), (None, 0));
}

#[test]
fn hide_cpu_tag_in_the_waiting_room() {
    // rc.14 (11-15): the waiting room hides the CPU as Training does, and its tag and its panel go with the body (the room's
    // CPU is entry 4: panel bit 4).
    let mut lobby = cpu(&[("hide_cpu", &ON)]);
    lobby.scene = Scene::Lobby;
    for practice in [false, true] {
        assert_eq!(tag_and_panels(&mut lobby, FighterKey::new(4, false).unwrap(), practice), (Some(false), 1 << 4), "practice {practice}");
    }
    // Hidden in Training, then the waiting room: it stays hidden there, tag and panel too.
    let mut both = cpu(&[("hide_cpu", &ON)]);
    assert_eq!(tag_and_panels(&mut both, body(), false), (Some(false), CPU_PANEL));
    both.scene = Scene::Lobby;
    assert_eq!(tag_and_panels(&mut both, body(), false), (Some(false), CPU_PANEL));
}

#[test]
fn hide_cpu_room_card_mapping() {
    // A waiting room shows its CPU as a partner card of `info_waiting` (`tp_00`..`tp_02`), not as an `info_melee` part: the
    // room's hidden CPU (entry 4) hides its card, found as the parent of the CPU's partner portrait.
    let mut room = cpu(&[("hide_cpu", &ON)]);
    room.scene = Scene::Lobby;
    let room_cpu = FighterKey::new(4, false).unwrap();
    frame(&mut room, room_cpu, false);
    assert!(hide::room_panel(room.hidden_panels()), "the room's CPU is hidden: its card hides");
    // Training's CPU (entry 1) is never a room card, and neither is Player 1.
    let mut training = cpu(&[("hide_cpu", &ON)]);
    frame(&mut training, body(), false);
    assert!(!hide::room_panel(training.hidden_panels()));
    assert!(!hide::room_panel(1 << 0));
    // Shown again: the card comes back (the same tick rule as the panels).
    let mut snap = Snapshot::factory();
    snap.set("hide_cpu", &[0, 1]).unwrap();
    room.publish(CpuSettings::from_snapshot(&snap), 0);
    frame(&mut room, room_cpu, false);
    assert!(!hide::room_panel(room.hidden_panels()));
    // Only a partner card is ever hidden: the pane's name is checked (NUL-padded as the game stores it).
    for name in [&b"tp_00"[..], b"tp_01", b"tp_02", b"tp_02\0\0\0\0"] {
        assert!(hide::room_card(name), "{name:?}");
    }
    for name in [&b"tp_03"[..], b"tp_0", b"tp_000", b"set_rep_tp_00", b"info_tp", b"players", b"p5", b"RootPane", b""] {
        assert!(!hide::room_card(name), "{name:?}");
    }
}

#[test]
fn hide_cpu_panel_rule() {
    // The panel parts of `info_melee` go by entry: Player 1's is `p1` and is never one this rule hides.
    assert_eq!(hide::panel_part(0), Some(*b"p1"));
    assert_eq!(hide::panel_part(1), Some(*b"p2"));
    assert_eq!(hide::panel_part(7), Some(*b"p8"));
    assert_eq!(hide::panel_part(8), None);
    assert_eq!(hide::panels([false, false, true, false, false, true]), (1 << 1) | (1 << 2));
    // Each `info_melee` tick: hidden on every tick while wanted, visible once after, nothing for a panel it never hid.
    let mut hidden = false;
    let ticks: Vec<_> = [false, true, true, false, false].iter().map(|&wanted| hide::panel_write(wanted, &mut hidden)).collect();
    assert_eq!(ticks, [None, Some(false), Some(false), Some(true), None]);
}

#[test]
fn hide_cpu_panel_follows_the_entry() {
    // The panel is the hidden body's own entry's part, whatever entry the CPU has: never an assumed `p2`.
    for entry in 1..8u8 {
        let mut cpu = cpu(&[("hide_cpu", &ON)]);
        let key = FighterKey::new(i32::from(entry), false).unwrap();
        assert_eq!(tag_and_panels(&mut cpu, key, false), (Some(false), 1 << entry), "entry {entry}");
        let parts: Vec<_> = (0..8).filter(|e| cpu.hidden_panels() & (1 << e) != 0).filter_map(hide::panel_part).collect();
        assert_eq!(parts, [[b'p', b'1' + entry]], "entry {entry}: only its own part");
        // Ice Climbers' Nana is the same entry: the same part.
        let nana = FighterKey::new(i32::from(entry), true).unwrap();
        assert_eq!(tag_and_panels(&mut cpu, nana, false), (Some(false), 1 << entry));
    }
    // Two CPU entries hidden at once: both parts, and Player 1's (`p1`) never.
    let mut two = cpu(&[("hide_cpu", &ON)]);
    for entry in [1, 3] {
        frame(&mut two, FighterKey::new(entry, false).unwrap(), false);
    }
    assert_eq!(two.hidden_panels(), (1 << 1) | (1 << 3));
    assert_eq!(two.hidden_panels() & 1, 0);
}

fn cpu_with_factory() -> Cpu {
    cpu(&[])
}

#[test]
fn hide_cpu_play_refusal() {
    // Play on CPU while the CPU is hidden is refused: the notice names the row, or the game when a practice game asked.
    assert_eq!(hide::refusal(false), "The CPU is hidden: turn Hide CPU Off first");
    assert_eq!(hide::refusal(true), "The CPU is hidden during this game");
}

/// One setting of the practice games, as the plugin's frame sees it: CPU during a game (Hide), an Eggs-ercise run on, the CPU as
/// its target, Break the Targets running.
#[derive(Clone, Copy, Debug)]
struct Game {
    practice_hide: bool,
    running: bool,
    cpu_target: bool,
    btt: bool,
}

impl Game {
    /// The request the game makes of the CPU body: the one decision (`eggs::hides_cpu`) the body, tag and panel all follow.
    fn request(self) -> bool {
        eggs::hides_cpu(self.practice_hide, self.running, self.cpu_target, self.btt)
    }
}

/// Every setting a game can be in: a CPU target needs a run, and Hide CPU On makes the target the disc (`eggs::choices`).
fn games(row: bool) -> Vec<Game> {
    let mut all = Vec::new();
    for practice_hide in [false, true] {
        for running in [false, true] {
            for cpu_target in [false, true] {
                for btt in [false, true] {
                    if (cpu_target && !running) || (cpu_target && row) || (running && btt) {
                        continue;
                    }
                    all.push(Game { practice_hide, running, cpu_target, btt });
                }
            }
        }
    }
    all
}

fn cpu_with_row(row: bool) -> Cpu {
    let mut cpu = cpu(&[]);
    set_row(&mut cpu, row);
    cpu
}

fn set_row(cpu: &mut Cpu, row: bool) {
    let mut snap = Snapshot::factory();
    snap.set("hide_cpu", &[[0, 1], [1, 0]][usize::from(row)]).unwrap();
    cpu.publish(CpuSettings::from_snapshot(&snap), 0);
}

#[test]
fn hide_cpu_target_cpu_keeps_tag_and_panel() {
    // Eggs-ercise on, Target CPU, CPU during a game Hide, Hide CPU Off: the CPU is the target, so the body, its tag and its
    // panel all stay. No frame hides it and none writes the tag or the panel.
    let target = Game { practice_hide: true, running: true, cpu_target: true, btt: false };
    assert!(!target.request(), "the target wins over CPU during a game");
    let mut cpu = cpu_with_row(false);
    for _ in 0..3 {
        let out = frame(&mut cpu, body(), target.request());
        assert_eq!((out.vanish, hide::tag(out.vanish), cpu.hidden_panels()), (None, None, 0));
        assert!(runs_behaviours(&out), "shown: the CPU's behaviours run");
    }
    // The same game with the disc as the target hides body, tag and panel together.
    let disc = Game { cpu_target: false, ..target };
    assert_eq!(tag_and_panels(&mut cpu, body(), disc.request()), (Some(false), CPU_PANEL));
    // The menu closes on Target CPU while the CPU is hidden: it is shown at once, its tag back on and its panel clear.
    assert_eq!(tag_and_panels(&mut cpu, body(), target.request()), (Some(true), 0));
    assert_eq!(tag_and_panels(&mut cpu, body(), target.request()), (None, 0), "then the game owns the tag again");
}

#[test]
fn hide_cpu_body_tag_panel_share_one_decision() {
    // Every pair of consecutive states (the Hide CPU row x a practice game's setting), each pair from a fresh CPU. After a frame
    // the body is hidden exactly when the row or the game's request says so; its tag is written off, and its panel is hidden,
    // in exactly those frames; the tag is written on only in the frame that shows a body that was hidden (and then the panel
    // is clear); and a frame that leaves the body alone leaves the tag alone. One decision, read three ways.
    let states: Vec<(bool, Game)> = [false, true].into_iter().flat_map(|row| games(row).into_iter().map(move |g| (row, g))).collect();
    assert_eq!(states.len(), 14, "8 settings with the row Off, 6 with it On (its CPU target is the disc)");
    for &(row_a, game_a) in &states {
        for &(row_b, game_b) in &states {
            let mut cpu = cpu_with_row(row_a);
            let mut was_hidden = false;
            for (row, game) in [(row_a, game_a), (row_b, game_b), (row_b, game_b)] {
                set_row(&mut cpu, row);
                let out = frame(&mut cpu, body(), game.request());
                let what = format!("row {row} {game:?}, hidden before: {was_hidden}");
                let wanted = row || (game.practice_hide && ((game.running && !game.cpu_target) || game.btt));
                let hidden = out.vanish == Some(true);
                assert_eq!(hidden, wanted, "the body: {what}");
                assert_eq!(hide::tag(out.vanish) == Some(false), hidden, "the tag is off with the body: {what}");
                assert_eq!(cpu.hidden_panels() & CPU_PANEL != 0, hidden, "the panel is hidden with the body: {what}");
                match hide::tag(out.vanish) {
                    Some(true) => assert!(was_hidden && !hidden && cpu.hidden_panels() == 0, "the tag comes on with the show: {what}"),
                    None => assert!(!was_hidden && !hidden, "no tag write only for a body left alone: {what}"),
                    Some(false) => {}
                }
                was_hidden = hidden;
            }
        }
    }
}