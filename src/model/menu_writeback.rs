//! The typed menu's writeback (`runtime::compat::menu`, rc.8 §6). Game threads (`store`, serialised by `EDITED`) and the publishing
//! path (`after_publish`, under `published::PUBLISHING`) each hand their value to the menu worker through a wait-free triple buffer
//! of their own, stamped from one counter as it is written. The worker reads both and adopts the newest fresh value it has not
//! adopted, as the last write to the one `PENDING` mutex won before. That mutex was taken on a game thread (the native service's
//! poll: `sync_selection` -> `menu::store`) while the menu worker could hold it: the rc.6 freeze class.

/// Of the values the reader just took (`(value, fresh)`, one per writer side, in place), the newest fresh one stamped after
/// `adopted`. By reference: the menu worker's stack does not hold copies of the menu.
pub fn newest<'a, T>(read: &[(&'a Option<(u64, T)>, bool)], adopted: u64) -> Option<&'a (u64, T)> {
    read.iter()
        .filter(|(_, fresh)| *fresh)
        .filter_map(|(value, _)| value.as_ref())
        .filter(|(stamp, _)| *stamp > adopted)
        .max_by_key(|(stamp, _)| *stamp)
}
