//! The `tr.menu/1` snapshot TrainingRooms publishes to the optional test bridge (ssbu-hw-bridge
//! docs/PLUGIN-STATE.md v2.1): the menu's place, the open editor, the running practice game, the current notice and the
//! handled-input count, as fixed-order `key=value` lines under the bridge's 2 KiB bound. Also the lookup schedule
//! (at most once a second for 30 s, then off for the boot) and the publisher's change/retry rule. Nothing here runs
//! when the bridge is absent: the runtime branches on `enabled` first.

/// The provider's name, schema id and version (the wire's `schema=tr.menu/1`).
pub const NAME: &[u8] = b"tr.menu";
pub const SCHEMA_ID: u32 = 1;
pub const SCHEMA_VERSION: u32 = 1;
/// The bridge's per-publish bound.
pub const MAX: usize = 2048;

/// What the snapshot shows (borrowed from the worker's session for one build).
#[derive(Clone, Copy, Debug, Default)]
pub struct View<'a> {
    pub open: bool,
    pub page: &'a str,
    pub depth: usize,
    /// `GROUPS` index of the sub-screen showing, when one is.
    pub group: Option<u8>,
    pub cursor: usize,
    pub row: Option<&'a str>,
    /// The row's setting id (empty when it has none).
    pub row_id: &'a str,
    /// A stable id for the focused row's action (`start.eggs`, `game.btt`, the row id, or `row`).
    pub action: &'a str,
    pub editor: Option<&'a str>,
    pub editor_sel: Option<i64>,
    pub editor_sel_key: &'a str,
    pub keyboard: bool,
    pub game: Option<&'a str>,
    pub game_running: bool,
    pub notice: Option<(&'a str, &'a str)>,
    /// Inputs the menu handled this boot, and how the last one ended.
    pub ack: u64,
    pub ack_result: &'a str,
    pub scene: &'a str,
    /// The visible rows of the screen showing (optional, sent last).
    pub rows: &'a [&'a str],
}

/// The snapshot into `out` (cleared first). Returns false when the rows did not all fit (`rows.complete=0`).
pub fn snapshot(view: &View<'_>, out: &mut String) -> bool {
    let _ = view;
    out.clear();
    true
}

/// The bridge lookup schedule: due at most once a second, and only in the first 30 s after the first look.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Discovery {
    started_ms: Option<u64>,
    next_ms: u64,
    pub state: Search,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Search {
    #[default]
    Searching,
    Found,
    Disabled,
}

impl Discovery {
    pub const WINDOW_MS: u64 = 30_000;
    pub const EVERY_MS: u64 = 1_000;

    /// A lookup is due now.
    pub fn due(&mut self, now_ms: u64) -> bool {
        let _ = now_ms;
        false
    }

    pub fn found(&mut self) {}

    /// The table was there but unusable (bad header, a null pointer, registration refused): off for the boot.
    pub fn disable(&mut self) {}

    /// Snapshot work is wanted (discovery still running, or the bridge was found).
    pub fn enabled(&self) -> bool {
        false
    }
}

/// The change and retry rule: publish a changed snapshot, and keep it dirty until a publish returns 0.
#[derive(Clone, Copy, Debug, Default)]
pub struct Publisher {
    last: u64,
    dirty: bool,
}

impl Publisher {
    pub fn wants(&self, snapshot: &[u8]) -> bool {
        let _ = snapshot;
        false
    }

    pub fn published(&mut self, snapshot: &[u8], result: i32) {
        let _ = (snapshot, result);
    }
}

#[cfg(test)]
#[path = "bridge_state_tests.rs"]
mod tests;
