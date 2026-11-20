//! The bridge state's `ack.result`: `Menu::press_outcome` reports whether the arm that took the press acted.

use super::*;

fn menu() -> Menu {
    Menu::new(Settings::factory(), Env::default())
}

#[test]
fn down_on_the_reset_confirmation_is_ignored_and_a_and_b_are_handled() {
    let mut m = menu();
    // Up from the first entry wraps to the page's last line, the reset entry.
    for _ in 0..200 {
        if matches!(m.focus(), Some(Entry::ResetPage(_))) {
            break;
        }
        assert!(m.press_outcome(Button::Up), "moving the cursor is handled");
    }
    assert!(matches!(m.focus(), Some(Entry::ResetPage(_))));
    assert!(m.press_outcome(Button::A), "A opens the confirmation");
    assert_eq!(m.screen(), Some(&Screen::ConfirmReset));
    assert!(!m.press_outcome(Button::Down), "Down on ConfirmReset takes the explicit no-op arm");
    assert_eq!(m.screen(), Some(&Screen::ConfirmReset));
    assert!(m.press_outcome(Button::B), "B backs out");
    assert_eq!(m.screen(), None);
}

#[test]
fn a_same_focus_value_change_is_handled_and_zr_on_a_page_is_ignored() {
    let mut m = menu();
    let mut tries = 0;
    while !m.focused_row().is_some_and(|r| matches!(ROWS[r].control, Control::Toggle)) {
        assert!(m.press_outcome(Button::Down));
        tries += 1;
        assert!(tries < 200, "a toggle row on the first page");
    }
    let (row, cursor) = (m.focused_row(), m.cursor());
    let before = m.get(ROWS[row.unwrap()].id).to_vec();
    assert!(m.press_outcome(Button::Right), "a value change keeps the focus and is handled");
    assert_eq!((m.focused_row(), m.cursor()), (row, cursor));
    assert_ne!(m.get(ROWS[row.unwrap()].id).to_vec(), before);
    assert!(!m.press_outcome(Button::ZR), "ZR on a page does nothing");
}

#[test]
fn left_right_and_y_on_the_trailing_reset_entry_are_ignored() {
    let mut m = menu();
    for _ in 0..200 {
        if matches!(m.focus(), Some(Entry::ResetPage(_))) {
            break;
        }
        assert!(m.press_outcome(Button::Up));
    }
    assert!(matches!(m.focus(), Some(Entry::ResetPage(_))) && m.focused_row().is_none());
    assert!(!m.press_outcome(Button::Left), "nothing to change on the Reset entry");
    assert!(!m.press_outcome(Button::Right));
    assert!(!m.press_outcome(Button::Y), "nothing to reset");
    assert!(matches!(m.focus(), Some(Entry::ResetPage(_))), "the focus stayed");
}
