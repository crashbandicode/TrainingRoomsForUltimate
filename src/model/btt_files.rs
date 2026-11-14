//! Break the Targets' saved layouts on the menu's side (`artifacts/rc14-eggsercise/SPEC.md` §V2.5 "Saved layouts"): the lines and
//! footer of the Saved layouts list. Pure and host-tested; the screen is `runtime::ui::layouts`, the files are
//! `runtime::features::btt_files` (the writer thread).

use super::btt::Slot;

/// A line of the Saved layouts list: `Slot 2 · 6 targets` or `Slot 3 · empty`. The plugin has no calendar date to show.
pub fn slot_label(index: usize, slot: Option<&Slot>) -> String {
    match slot {
        None => format!("Slot {} · empty", index + 1),
        Some(s) => format!("Slot {} · {} target{}", index + 1, s.n, if s.n == 1 { "" } else { "s" }),
    }
}

/// The list's footer: its first line (the focused row) and this screen's buttons.
pub fn layouts_footer(library_footer: &str) -> String {
    format!("{}\nA Load · Y Delete · B Return", library_footer.lines().next().unwrap_or(""))
}

#[cfg(test)]
#[path = "btt_files_tests.rs"]
mod tests;