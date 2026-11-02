//! The fork's `replay_preview`: whether the replay editor holds the practice entries (package 4's recorder owns the fact).

pub fn occupied() -> bool {
    crate::runtime::features::recorder::replay_editor_occupied()
}