//! Exact card regression: unavailable menu edits defer hooks, never invent eligibility.
#[path = "../src/model/menu_read.rs"]
mod menu_read;

use menu_read::{try_current, MenuBusy};

#[test]
fn successful_menu_reads_match_existing_generation_precedence() {
    for generation in [0, 1, 7, u32::MAX] {
        for edit in [None, Some((generation, false)), Some((generation, true)), Some((generation.wrapping_add(1), true))] {
            for published in [false, true] {
                let old = match edit {
                    Some((at, value)) if at == generation => value,
                    _ => published,
                };
                assert_eq!(try_current(generation, Some(edit), || published), Ok(old));
            }
        }
    }
}

#[test]
fn held_edits_never_fall_back_to_stale_on_or_stale_off() {
    for (published, edited) in [(true, false), (false, true)] {
        let edit = hos_lock::Mutex::new(Some((7, edited)));
        let held = edit.lock();
        let result = try_current(7, edit.try_lock().map(|value| *value), || {
            panic!("busy edit must not consult published {published}")
        });
        assert_eq!(result, Err(MenuBusy));
        drop(held);
        assert_eq!(try_current(7, edit.try_lock().map(|value| *value), || published), Ok(edited));
        assert_eq!(try_current(8, edit.try_lock().map(|value| *value), || published), Ok(published));
    }
}

