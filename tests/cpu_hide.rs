//! Hide CPU (rc.14, SPEC-eggsercise V2.6): the rule, the restore, the edges and the waiting-room gate. The model decides
//! once per body frame (`Cpu::frame` -> `FrameOut::vanish`); `runtime::features::hide_cpu` applies it (Eden rows `hide-E34`..).

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::cpu::defence::Defence;
use model::cpu::facts::{FrameFacts, FrameOut};
use model::cpu::hide;
use model::cpu::role::{FighterKey, Scene};
use model::cpu::settings::CpuSettings;
use model::cpu::state::Cpu;
use model::menu::table::ROWS;
use model::menu::{Env, Grey, Menu};
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
            let want = if scene == Scene::Training && (row || practice) { Some(true) } else { None };
            assert_eq!(hide::step(&mut false, scene, row, practice), want, "{scene:?} row {row} practice {practice}");
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
    // Leaving Training (or the practice scene) while hidden: the next frame of that body, in a waiting room, shows it.
    both.scene = Scene::Lobby;
    assert_eq!(frame(&mut both, body(), false).vanish, Some(false));
    // A save state load, L+R+A and opening the menu are not events of this model: no frame says anything but the rule.
}

#[test]
fn hide_cpu_waiting_room_gate() {
    // Until the LDN check has passed, a waiting room's CPU is never hidden: not by the row, not by a practice game.
    let mut lobby = cpu(&[("hide_cpu", &ON)]);
    lobby.scene = Scene::Lobby;
    for practice in [false, true] {
        let out = frame(&mut lobby, FighterKey::new(4, false).unwrap(), practice);
        assert_eq!((out.vanish, runs_behaviours(&out)), (None, true), "practice {practice}");
    }
    // The setting itself is kept for Training.
    assert!(lobby.settings.hide_cpu);
    // The row says so in a waiting room's menu (greyed, with the reason) and is a plain toggle in Training: the CPU page's
    // Level group, after CPU Level, Off by default.
    let row = ROWS.iter().position(|r| r.id == "hide_cpu").expect("the Hide CPU row");
    assert_eq!(ROWS[row - 1].id, "cpu_level");
    assert_eq!(ROWS[row].page(), model::menu::table::Page::Cpu);
    assert!(!CpuSettings::from_snapshot(&Snapshot::factory()).hide_cpu);
    let menu = |waiting_room| Menu::new(Settings::factory(), Env { waiting_room, ..Env::default() });
    assert_eq!(menu(false).grey(row), None);
    assert_eq!(menu(true).grey(row), Some(Grey::WaitingRoom));
    assert_eq!(Grey::WaitingRoom.text(), "Not in a waiting room yet");
}

#[test]
fn hide_cpu_play_refusal() {
    // Play on CPU while the CPU is hidden is refused: the notice names the row, or the game when a practice game asked.
    assert_eq!(hide::refusal(false), "The CPU is hidden: turn Hide CPU Off first");
    assert_eq!(hide::refusal(true), "The CPU is hidden during this game");
}
