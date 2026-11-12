//! Persisted stage choice is independent of the ordered rotation's ranks.
use serde::{Deserialize, Serialize};

/// Shared with the native stage panel: paging moves the cursor, never ranks.
pub const PAGE_ROWS: usize = 4;
pub const PAGE_COLUMNS: usize = crate::engine::compat::grid::NX_SUBMENU_COLUMNS;
pub const PAGE_SIZE: usize = PAGE_ROWS * PAGE_COLUMNS;

/// Wrap pages, retaining the cell where possible and clamping on the final page.
pub(crate) fn page_cursor(selected: usize, count: usize, next: bool) -> Option<usize> {
    if count == 0 {
        return None;
    }
    let selected = selected.min(count - 1);
    let pages = count.div_ceil(PAGE_SIZE);
    let page = selected / PAGE_SIZE;
    let target = if next { (page + 1) % pages } else { (page + pages - 1) % pages };
    Some((target * PAGE_SIZE + selected % PAGE_SIZE).min(count - 1))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Rotation,
    Direct,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Preferences {
    pub mode: Mode,
    /// Native catalog ID, never a page/cell index. Validated by the catalog owner.
    pub direct_stage: u32,
    pub direct_form: crate::engine::stages::stage_forms::Form,
    pub rotation_forms: crate::engine::stages::stage_forms::Forms,
}

impl Default for Preferences {
    fn default() -> Self {
        Self { mode: Mode::Rotation, direct_stage: 311,
            direct_form: crate::engine::stages::stage_forms::Form::Normal,
            rotation_forms: crate::engine::stages::stage_forms::Forms::NORMAL }
    }
}

