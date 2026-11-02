//! The fork's `replay_preview` as package 6's carried files name it: the clip preview's session is package 4's
//! `runtime::macros::replay_preview` (carried); this forwards to that one owner.

pub fn occupied() -> bool {
    crate::runtime::macros::replay_preview::occupied()
}
