//! R2-2 of the packages 2-5 review: an unavailable menu must never own Player 1's input. The runtime's poll and tick
//! (`runtime::ui::input::on_p1_poll`, `runtime::ui::tick`) apply `model::readiness` to atomics; this test drives the same
//! pure pieces in the same order with the input hook active: the chord adapter, the owner bit and the readiness rules,
//! through (a) a failed UI guard, (b) a refused archive (panes never resolve), (c) a layout lost while the menu is open,
//! each with repeated open attempts, and the recovery once a usable layout registers.

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::adapter::{bit, Adapter, Style};
use model::input_delay::Owners;
use model::menu::chord::Chord;
use model::readiness::{may_open, must_release, Facts, STALL_POLLS};

fn mask(bits: &[u32]) -> u32 {
    bits.iter().fold(0, |m, b| m | (1 << b))
}

/// The runtime's facts as plain fields: what the atomics hold.
#[derive(Default)]
struct World {
    installed: bool,
    layout_ready: bool,
    open: bool,
    closing: bool,
    owners: Owners,
    polls: u32,
    polls_at_tick: u32,
    adapter: Adapter,
}

impl World {
    fn facts(&self) -> Facts {
        Facts {
            installed: self.installed,
            layout_ready: self.layout_ready,
            pause_active: false,
            closing: self.closing,
            owns_input: self.owners.contains(Owners::MENU) || self.closing || self.open,
            polls_since_tick: self.polls.wrapping_sub(self.polls_at_tick),
        }
    }

    /// `on_p1_poll`: one poll of Player 1.
    fn poll(&mut self, buttons: u32) {
        self.polls += 1;
        let facts = self.facts();
        if must_release(facts) {
            self.open = false;
            self.closing = false;
            self.owners = Owners::from_bits(self.owners.bits() & !Owners::MENU.bits());
            self.adapter.resync(buttons, Style::Pro);
            return;
        }
        let step = self.adapter.poll(buttons, Style::Pro, self.open, may_open(facts), [Chord::NONE, Chord::NONE]);
        if step.open {
            self.adapter.resync(buttons, Style::Pro);
            self.owners = self.owners.with(Owners::MENU);
            self.open = true;
        }
    }

    /// The layout tick, when the layout lives: `ready` is whether it resolved the panes.
    fn tick(&mut self, ready: bool) {
        if ready {
            self.layout_ready = true;
            self.polls_at_tick = self.polls;
        } else {
            self.layout_ready = false;
            if self.facts().owns_input {
                self.open = false;
                self.closing = false;
                self.owners = Owners::from_bits(self.owners.bits() & !Owners::MENU.bits());
            }
        }
    }

    /// Hold B, press Up, release both: one open attempt.
    fn chord(&mut self) {
        self.poll(0);
        self.poll(mask(&[bit::B]));
        self.poll(mask(&[bit::B, bit::DPAD_UP]));
        self.poll(0);
    }

    fn p1_free(&self) -> bool {
        !self.owners.contains(Owners::MENU)
    }
}

#[test]
fn a_failed_ui_guard_never_lets_the_menu_own_input_however_often_the_chord_is_tried() {
    let mut world = World { installed: false, ..World::default() };
    for _ in 0..5 {
        world.chord();
        assert!(world.p1_free() && !world.open);
    }
}

#[test]
fn a_refused_archive_means_the_panes_never_resolve_and_the_menu_never_opens() {
    let mut world = World { installed: true, ..World::default() };
    for _ in 0..5 {
        world.tick(false);
        world.chord();
        assert!(world.p1_free() && !world.open, "no renderer: no menu, input stays with the game");
    }
    // A usable layout registers: the same chord now opens it.
    world.tick(true);
    world.chord();
    assert!(world.open && !world.p1_free());
}

#[test]
fn a_layout_lost_while_the_menu_is_open_releases_the_input_at_once() {
    let mut world = World { installed: true, ..World::default() };
    world.tick(true);
    world.chord();
    assert!(world.open && !world.p1_free());
    // The panes go away (archive re-injection refused, layout re-registered without them).
    world.tick(false);
    assert!(world.p1_free() && !world.open && !world.closing);
    // It recovers after a usable layout registers.
    world.tick(true);
    world.chord();
    assert!(world.open);
}

#[test]
fn a_layout_destroyed_with_the_menu_open_is_noticed_by_the_input_hook_alone() {
    let mut world = World { installed: true, ..World::default() };
    world.tick(true);
    world.chord();
    assert!(world.open);
    // No tick ever runs again (the layout is gone): the poll counter walks past the stall limit.
    for _ in 0..=STALL_POLLS {
        world.poll(0);
    }
    assert!(world.p1_free() && !world.open);
}

#[test]
fn the_close_window_is_released_too_when_the_renderer_goes_before_its_fifteen_frames_end() {
    let mut world = World { installed: true, ..World::default() };
    world.tick(true);
    world.chord();
    world.open = false;
    world.closing = true;
    world.tick(false);
    assert!(world.p1_free() && !world.closing, "ownership does not wait for a tick that cannot finish the window");
}
