//! Which waiting room may open the native editor. Local Wireless is a separate
//! owner from Training and from Quickplay; it is not admitted by widening either.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Admission {
    Training,
    Quickplay,
    LocalWireless,
}

/// `mode` is the native mode word, classified by the shared waiting policy.
/// Local Wireless waiting is exactly `0x02010000` and uses parent vtable
/// `0x50b8028`, not Training `0x50bad50` or Quickplay parent `0x50b9000`.
pub(super) fn admit(
    native_training: bool, lobby_active: bool, mode: u32,
) -> Result<Admission, &'static str> {
    if native_training {
        return Ok(Admission::Training);
    }
    if lobby_active && crate::engine::mii::backend::Layout::quickplay_waiting_mode(mode) {
        return Ok(Admission::Quickplay);
    }
    if lobby_active && mode == 0x02010000 {
        return Ok(Admission::LocalWireless);
    }
    Err("Enter Training, Quickplay, or Local Wireless waiting practice to edit a Mii")
}

