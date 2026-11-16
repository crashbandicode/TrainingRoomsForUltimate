//! A typed-menu read distinguishes unavailable edits from absent or obsolete edits.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuBusy;

/// The existing typed-menu precedence, shared by blocking and fallible readers.
pub fn current<T: Copy>(generation: u32, edit: Option<(u32, T)>, published: impl FnOnce() -> T) -> T {
    match edit {
        Some((at, menu)) if at == generation => menu,
        _ => published(),
    }
}

/// `None` outside means the edit lock was busy; `Some(None)` means no edit exists.
/// Never consult published/default values when the edit could not be inspected.
pub fn try_current<T: Copy>(
    generation: u32,
    edit: Option<Option<(u32, T)>>,
    published: impl FnOnce() -> T,
) -> Result<T, MenuBusy> {
    Ok(current(generation, edit.ok_or(MenuBusy)?, published))
}
