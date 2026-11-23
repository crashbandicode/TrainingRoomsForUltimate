use super::*;
use crate::model::recording_menu::Face;

/// The list the cursor rests in now (the bridge snapshot's `list` keys, `model::bridge_state`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FocusList {
    pub id: String,
    /// The places the cursor can rest on.
    pub count: usize,
    /// The cursor's place among them.
    pub index: usize,
    /// The table row under the cursor, when the entry is one (its value is the page's `write_value`).
    pub row: Option<usize>,
    /// The entry's own value where it is not a table row (a Recording file screen's row).
    pub value: String,
}

impl Menu {
    pub fn focus_list(&self) -> Option<FocusList> {
        if self.recording.popup_open() {
            return self.recording.focus_list(Face::Popup, 0, true);
        }
        let screen = self.screen().copied();
        if screen.is_none() && self.page == Page::Recording {
            let group = self.focus().map(group_of);
            let on_library = group.is_some_and(|g| g == usize::from(table::SAVES_MACROS) || g == usize::from(table::COMBOS_SEQ));
            let which = group.unwrap_or(usize::from(table::SAVES_MACROS)) as u8;
            if let Some(list) = self.recording.focus_list(Face::Page, which, on_library) {
                return Some(list);
            }
        }
        let (id, count, index, row) = match screen {
            None => {
                let cursor = self.cursor();
                let mut count = 0;
                let mut index = None;
                self.scan(self.page, |at, entry| {
                    if self.focusable(entry) {
                        if at == cursor { index = Some(count); }
                        count += 1;
                    }
                    true
                });
                ("page".to_owned(), count, index?, self.focused_row())
            }
            Some(Screen::Rows { group, cursor }) => {
                (format!("rows/{group}"), self.sub_rows(group).count(), cursor, self.sub_row(group, cursor))
            }
            Some(Screen::List { row, cursor } | Screen::Checklist { row, cursor }) => {
                (format!("popup/{}", ROWS[row].id), self.option_count(row), cursor, None)
            }
            Some(Screen::Delay { row, cursor, .. }) => {
                (format!("popup/{}", ROWS[row].id), self.get(ROWS[row].id).len() + 2, cursor, None)
            }
            Some(Screen::SwapGroups { cursor }) => ("swap_groups".to_owned(), SWAP_GROUP_IDS.len(), cursor, None),
            Some(Screen::Overrides { situation, cursor }) => {
                (format!("overrides/{situation}"), options::ACTIONS.len() + 1, cursor, None)
            }
            Some(Screen::Chords { cursor }) => ("chords".to_owned(), CHORD_ROWS.len(), cursor, None),
            _ => return None,
        };
        (count != 0).then(|| FocusList { id, count, index: index.min(count - 1), row, value: String::new() })
    }
}
