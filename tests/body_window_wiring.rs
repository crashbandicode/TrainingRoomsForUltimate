//! The body window's teardown reads (rc.6 ship blocker, `final-ally-72344fd9`, 2026-11-10 00:56Z): Training's finalize into the
//! Mii editor ran `native_body_bank::retire_window`, which took every *planned* member's slot (`entry+0x4160 + i*8`) for a body. A
//! slot past the window's size (`+0x4150`) is uninitialised or left over; that one held float bytes, `0xb31caf33bf800000`, and the
//! reads of its id, kind, vtable and state faulted 8 times (a crash on a console). The runtime is Switch-only, so this reads its
//! text; the cell rule itself is `native_fields::inline_elements` (host-tested in the library).

fn retire_window() -> &'static str {
    let text = include_str!("../src/runtime/roster/native_body_bank.rs");
    let start = text.find("unsafe fn retire_window(").expect("retire_window");
    let body = &text[start..];
    &body[..body.find("\n}\n").expect("retire_window's end")]
}

#[test]
fn the_teardown_reads_only_the_cells_the_window_holds() {
    let retire = retire_window();
    assert!(retire.contains("window_cells(entry as usize)"), "retire_window no longer reads the window's own cells");
    assert!(!retire.contains("0x4160"), "retire_window reads a planned member's slot again");
}

#[test]
fn the_teardown_checks_a_cells_type_before_its_fields() {
    let retire = retire_window();
    let vtable = retire.find("FIGHTER_VTABLE").expect("the fighter vtable check");
    for field in ["fields::read::<u32>(body, 8)", "fields::read::<u32>(body, 0xc)", "fields::read::<u8>(body, 0x3a)"] {
        let at = retire.find(field).unwrap_or_else(|| panic!("{field}"));
        assert!(vtable < at, "{field} is read before the body's vtable is checked");
    }
}

#[test]
fn the_pair_traces_read_cell_0_through_the_window() {
    let text = include_str!("../src/runtime/roster/native_body_bank.rs");
    assert!(!text.contains("fields::child(entry as usize, 0x4160).and_then"), "a trace follows cell 0 past the window's size");
    assert!(text.contains("fields::inline_elements::<NATIVE_MEMBERS>(entry, WINDOW)"));
}
