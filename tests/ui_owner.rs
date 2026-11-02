//! R2 of the package-1 review for the plugin menu (SPEC-core-03 §2.3, SPEC-input-03 §5.1): the menu's owner bit has
//! a real producer. The runtime input hook sets `Owners::MENU` when the adapter says a chord passed and the layout tick
//! clears it after the 15 real frames that follow a close; while it is set the delay stage outputs a neutral record
//! and empties its queue. This test drives the same pure pieces in that order (adapter -> owner bit -> delay queue).

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::adapter::{bit, Adapter, Style};
use model::frame::CLOSE_FRAMES;
use model::input_delay::{DelayQueue, MappedInputs, Owners};
use model::menu::chord::Chord;
use model::menu::input::Button;

fn mask(bits: &[u32]) -> u32 {
    bits.iter().fold(0, |m, b| m | (1 << b))
}

fn press(n: u32) -> MappedInputs {
    MappedInputs { buttons: n, ..MappedInputs::NEUTRAL }
}

#[test]
fn the_menu_owns_p1_from_the_chord_until_fifteen_frames_after_the_close() {
    let none = [Chord::NONE, Chord::NONE];
    let mut adapter = Adapter::new();
    let mut queue = DelayQueue::new();
    let mut owners = Owners::NONE;
    let mut open = false;
    let mut closing: Option<u32> = None;

    // Gameplay: the chord is not held yet, the delay queue passes the record on after its delay.
    assert!(!adapter.poll(mask(&[bit::B]), Style::Pro, open, true, none).open);
    assert_eq!(queue.poll(0, owners, press(1)), press(1));

    // B + D-pad Up: the hook's producer sets the owner; the same poll's delay stage now outputs neutral.
    let step = adapter.poll(mask(&[bit::B, bit::DPAD_UP]), Style::Pro, open, true, none);
    assert!(step.open);
    adapter.resync(mask(&[bit::B, bit::DPAD_UP]), Style::Pro);
    open = true;
    owners = owners.with(Owners::MENU);
    assert_eq!(queue.poll(3, owners, press(7)), MappedInputs::NEUTRAL);
    assert!(queue.is_empty(), "the queue is emptied while the menu owns the input");

    // Open: presses reach the menu, the game still sees neutral.
    adapter.poll(0, Style::Pro, open, true, none);
    let step = adapter.poll(mask(&[bit::B]), Style::Pro, open, true, none);
    assert_eq!(step.presses().collect::<Vec<_>>(), [Button::B]);
    for _ in 0..4 {
        assert_eq!(queue.poll(3, owners, press(9)), MappedInputs::NEUTRAL);
    }

    // The menu closed (the worker's close): the owner stays for the close window, then the tick clears it.
    open = false;
    closing = closing.or(Some(0));
    let mut frames = 0;
    while let Some(count) = closing {
        // Chords are refused while closing, so the closing B cannot reopen the menu.
        assert!(!adapter.poll(mask(&[bit::B, bit::DPAD_UP]), Style::Pro, open, false, none).open);
        assert_eq!(queue.poll(0, owners, press(1)), MappedInputs::NEUTRAL, "frame {count} of the window");
        frames += 1;
        closing = if count + 1 >= CLOSE_FRAMES {
            owners = Owners::from_bits(owners.bits() & !Owners::MENU.bits());
            None
        } else {
            Some(count + 1)
        };
    }
    assert_eq!(frames, CLOSE_FRAMES);
    assert_eq!(queue.poll(0, owners, press(1)), press(1), "gameplay owns the input again");
}