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
    /// The publisher does not list the rows (the v1 limit): `rows.complete=0` whatever `rows` holds.
    pub rows_omitted: bool,
}

/// The snapshot into `out` (cleared first). Returns false when the rows did not all fit (`rows.complete=0`).
pub fn snapshot(view: &View<'_>, out: &mut String) -> bool {
    use core::fmt::Write as _;
    out.clear();
    let group = view.group.map_or(-1, i32::from);
    let focus_group = view.group.map_or_else(|| "-".to_string(), |g| g.to_string());
    let _ = writeln!(out, "schema=tr.menu/{SCHEMA_VERSION}");
    let _ = writeln!(out, "ack={}", view.ack);
    push_str(out, "ack.result", view.ack_result);
    let _ = writeln!(out, "focus={}/{focus_group}/{}/{}", view.page, view.cursor, view.action);
    let _ = writeln!(out, "open={}", u8::from(view.open));
    push_str(out, "page", view.page);
    let _ = writeln!(out, "depth={}", view.depth);
    let _ = writeln!(out, "group={group}");
    let _ = writeln!(out, "cursor={}", view.cursor);
    push_str(out, "row", view.row.unwrap_or(""));
    push_str(out, "row_id", view.row_id);
    push_str(out, "action", view.action);
    push_str(out, "editor", view.editor.unwrap_or(""));
    let _ = writeln!(out, "editor.sel={}", view.editor_sel.unwrap_or(-1));
    push_str(out, "editor.sel_key", view.editor_sel_key);
    let _ = writeln!(out, "keyboard={}", u8::from(view.keyboard));
    push_str(out, "game", view.game.unwrap_or(""));
    let _ = writeln!(out, "game.running={}", u8::from(view.game_running));
    let (header, body) = view.notice.unwrap_or(("", ""));
    push_str(out, "notice.header", header);
    push_str(out, "notice.body", body);
    push_str(out, "scene", view.scene);
    // The optional rows come last: as many as fit under the bound, and a mark when some did not.
    let mut rows = String::new();
    let mut shown = 0;
    let fixed = out.len() + "rows.count=\nrows.complete=0\nrows=".len() + 8;
    for row in view.rows {
        let mut item = String::new();
        if shown > 0 {
            item.push(',');
        }
        quote(&mut item, row);
        if fixed + rows.len() + item.len() > MAX {
            break;
        }
        rows.push_str(&item);
        shown += 1;
    }
    let complete = !view.rows_omitted && shown == view.rows.len();
    let _ = writeln!(out, "rows.count={}", view.rows.len());
    let _ = writeln!(out, "rows.complete={}", u8::from(complete));
    let _ = write!(out, "rows={rows}");
    complete
}

/// The table header the provider accepts (ssbu-hw-bridge PLUGIN-STATE.md §2): this magic and ABI, exactly the declared
/// size of the table it knows, one reader, the publish bound, and exactly the contract's eight provider slots.
pub fn header_usable(magic: u32, abi: u32, struct_size: u32, readers: u32, max_payload: u32, max_providers: u32, table_size: usize) -> bool {
    magic == u32::from_le_bytes(*b"BRST")
        && abi == 1
        && struct_size as usize == table_size
        && readers == 1
        && max_payload as usize == MAX
        && max_providers == PROVIDER_SLOTS
}

/// The contract's provider slot count.
pub const PROVIDER_SLOTS: u32 = 8;

/// `ack.result` for the handler's own outcome.
pub fn ack_result(handled: bool) -> &'static str {
    if handled { "handled" } else { "ignored" }
}

/// The notice the snapshot reports: the last painted one while the HUD shows the slot, none while it is hidden (menu
/// open) or after the notice expired.
pub fn shown_notice<'a>(visible: bool, painted: (&'a str, &'a str)) -> Option<(&'a str, &'a str)> {
    (visible && !(painted.0.is_empty() && painted.1.is_empty())).then_some(painted)
}

/// `scene` from the UI tick's stamp (`uptime_ms << 2 | kind`): "training", "lobby", or "" when the practice layout has
/// not passed for more than `fresh_ms`.
pub fn scene_name(stamp: u64, now_ms: u64, fresh_ms: u64) -> &'static str {
    if stamp == 0 || now_ms.saturating_sub(stamp >> 2) > fresh_ms {
        return "";
    }
    match stamp & 3 {
        1 => "training",
        2 => "lobby",
        _ => "",
    }
}

fn push_str(out: &mut String, key: &str, value: &str) {
    out.push_str(key);
    out.push('=');
    quote(out, value);
    out.push('\n');
}

/// `"…"` with `\"`, `\\` and `\n` escaped: one line whatever the text.
fn quote(out: &mut String, value: &str) {
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            c => out.push(c),
        }
    }
    out.push('"');
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
        if self.state != Search::Searching {
            return false;
        }
        let started = *self.started_ms.get_or_insert(now_ms);
        if now_ms.saturating_sub(started) > Self::WINDOW_MS {
            self.state = Search::Disabled;
            return false;
        }
        if now_ms < self.next_ms {
            return false;
        }
        self.next_ms = now_ms + Self::EVERY_MS;
        true
    }

    pub fn found(&mut self) {
        self.state = Search::Found;
    }

    /// The table was there but unusable (bad header, a null pointer, registration refused): off for the boot.
    pub fn disable(&mut self) {
        self.state = Search::Disabled;
    }

    /// Snapshot work is wanted (discovery still running, or the bridge was found).
    pub fn enabled(&self) -> bool {
        self.state != Search::Disabled
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
        self.dirty || hash(snapshot) != self.last
    }

    /// The bridge's answer to the publish of `snapshot`: 0 landed; anything else keeps it dirty for the next tick.
    pub fn published(&mut self, snapshot: &[u8], result: i32) {
        if result == 0 {
            self.last = hash(snapshot);
            self.dirty = false;
        } else {
            self.dirty = true;
        }
    }
}

/// FNV-1a (0 is kept for "nothing published yet").
fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3)) | 1
}

#[cfg(test)]
#[path = "bridge_state_tests.rs"]
mod tests;
