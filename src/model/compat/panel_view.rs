//! The one list view the carried Mii catalog menu fills (the fork's `macro_panel::View`, a plain title, rows, cursor and footer).
//! Written for this repository, not carried: the clean room's list screen (package 5) draws it.

/// A titled page of rows with a cursor and a footer line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct View {
    pub title: String,
    pub footer: String,
    pub labels: Vec<String>,
    pub selected: usize,
}