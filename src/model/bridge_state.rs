//! The `tr.menu/1` snapshot TrainingRooms publishes to the optional test bridge (ssbu-hw-bridge
//! docs/PLUGIN-STATE.md v2.1): the menu's place, the open editor, the running practice game, zone editor state, the
//! current notice and the handled-input count, as fixed-order `key=value` lines under the bridge's 2 KiB bound. Also the
//! lookup schedule (at most once a second for 30 s, then off for the boot) and the publisher's change/retry rule.
//! Nothing here runs when the bridge is absent: the runtime branches on `enabled` first.

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

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
    /// The zone editor's last handed-over state (`None` before its first opening this boot), whether it is up now, and
    /// the saved zone of that state's stage.
    pub zone: Option<ZoneEditor>,
    pub zone_up: bool,
    pub zone_saved: Option<[u8; 4]>,
    /// Inputs the menu handled this boot, and how the last one ended.
    pub ack: u64,
    pub ack_result: &'a str,
    pub scene: &'a str,
    pub recording_popup_open: bool,
    pub recording_popup_role: &'a str,
    pub recording_page_role: &'a str,
    /// The role of the active capture, empty when there is no take.
    pub recording_capture_role: &'a str,
    /// The role of the take the HUD's row drew on its last pass, empty when it drew none or that pass is stale.
    pub recording_indicator: &'a str,
    pub playback_indicator: &'a str,
    /// The strip's last drawn direction marks, oldest first.
    pub strip_aims: [Option<u8>; 2],
    pub list: &'a str,
    pub list_count: usize,
    pub list_index: Option<usize>,
    pub list_value: &'a str,
    /// What hides the menu while it stays open: `drill` during an input drill's preview (F10), else empty.
    pub recording_preview: &'a str,
    pub extra: &'a str,
    /// The visible rows of the screen showing (optional, sent last).
    pub rows: &'a [&'a str],
    /// The publisher does not list the rows (the v1 limit): `rows.complete=0` whatever `rows` holds.
    pub rows_omitted: bool,
}

/// The stage spawn-zone editor for the snapshot's `zone.*` lines, as the game thread hands it over.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ZoneEditor {
    pub stage: i32,
    /// `egg_zone::Step as u8`: top, bottom, inner, outer, preview.
    pub step: u8,
    /// Top, bottom, inner, outer, as saved.
    pub bins: [u8; 4],
    /// How the editor closed: 0 not by its own buttons, `APPLIED`, `CANCELLED`.
    pub last: u8,
    /// [bottom, top] in stage units.
    pub vertical: [f32; 2],
    /// Left and right: that side's [lo, hi] when the stage has a zone there.
    pub sides: [Option<[f32; 2]>; 2],
}

impl ZoneEditor {
    pub const APPLIED: u8 = 1;
    pub const CANCELLED: u8 = 2;

    pub fn pack(self) -> [u64; 4] {
        let pair = |[lo, hi]: [f32; 2]| u64::from(lo.to_bits()) | (u64::from(hi.to_bits()) << 32);
        let head = (1u64 << 63) | (u64::from(self.last & 3) << 51) | (u64::from(self.step & 7) << 48)
            | (u64::from(self.stage as u16) << 32) | u64::from(u32::from_le_bytes(self.bins));
        [head, pair(self.vertical), self.sides[0].map_or(u64::MAX, pair), self.sides[1].map_or(u64::MAX, pair)]
    }

    pub fn unpack(words: [u64; 4]) -> Option<Self> {
        let [head, vertical, left, right] = words;
        if head == 0 {
            return None;
        }
        let pair = |word: u64| [f32::from_bits(word as u32), f32::from_bits((word >> 32) as u32)];
        Some(Self {
            stage: i32::from((head >> 32) as u16),
            step: ((head >> 48) & 7) as u8,
            bins: (head as u32).to_le_bytes(),
            last: ((head >> 51) & 3) as u8,
            vertical: pair(vertical),
            sides: [left, right].map(|word| (word != u64::MAX).then(|| pair(word))),
        })
    }
}

/// Game thread to worker without a lock: a read that meets a write gets nothing rather than half of it.
pub struct ZoneCell {
    seq: AtomicU32,
    words: [AtomicU64; 4],
}

impl ZoneCell {
    pub const fn new() -> Self {
        Self { seq: AtomicU32::new(0), words: [const { AtomicU64::new(0) }; 4] }
    }

    pub fn write(&self, words: [u64; 4]) {
        let odd = self.seq.load(Ordering::SeqCst).wrapping_add(1);
        self.seq.store(odd, Ordering::SeqCst);
        for (slot, value) in self.words.iter().zip(words) {
            slot.store(value, Ordering::SeqCst);
        }
        self.seq.store(odd.wrapping_add(1), Ordering::SeqCst);
    }

    pub fn read(&self) -> Option<[u64; 4]> {
        let seq = self.seq.load(Ordering::SeqCst);
        if seq & 1 != 0 {
            return None;
        }
        let words = self.words.each_ref().map(|word| word.load(Ordering::SeqCst));
        (self.seq.load(Ordering::SeqCst) == seq).then_some(words)
    }
}

impl Default for ZoneCell {
    fn default() -> Self {
        Self::new()
    }
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
    let editing = view.zone.filter(|_| view.zone_up);
    let step = editing.map_or("", |zone| match zone.step {
        0 => "top", 1 => "bottom", 2 => "inner", 3 => "outer", 4 => "preview", _ => "",
    });
    push_str(out, "zone.step", step);
    let last = if editing.is_some() { "" } else { view.zone.map_or("", |zone| match zone.last {
        ZoneEditor::APPLIED => "applied", ZoneEditor::CANCELLED => "cancelled", _ => "",
    }) };
    push_str(out, "zone.last", last);
    let _ = writeln!(out, "zone.stage={}", view.zone.map_or(-1, |zone| zone.stage));
    if let Some(zone) = editing {
        let [top, bottom, inner, outer] = zone.bins;
        let _ = writeln!(out, "zone.bins=\"{top},{bottom},{inner},{outer}\"");
        let [bottom, top] = zone.vertical;
        let _ = writeln!(out, "zone.y=\"{bottom:.2},{top:.2}\"");
        for (key, side) in [("zone.left", zone.sides[0]), ("zone.right", zone.sides[1])] {
            if let Some([lo, hi]) = side {
                let _ = writeln!(out, "{key}=\"{lo:.2},{hi:.2}\"");
            } else {
                push_str(out, key, "");
            }
        }
    } else {
        for key in ["zone.bins", "zone.y", "zone.left", "zone.right"] {
            push_str(out, key, "");
        }
    }
    if let Some([top, bottom, inner, outer]) = view.zone.filter(|z| z.stage >= 0).and(view.zone_saved) {
        let _ = writeln!(out, "zone.saved=\"{top},{bottom},{inner},{outer}\"");
    } else {
        push_str(out, "zone.saved", "");
    }
    let (header, body) = view.notice.unwrap_or(("", ""));
    push_str(out, "notice.header", header);
    push_str(out, "notice.body", body);
    push_str(out, "scene", view.scene);
    let _ = writeln!(out, "recording.popup.open={}", u8::from(view.recording_popup_open));
    push_str(out, "recording.popup.role", view.recording_popup_role);
    push_str(out, "recording.page.role", view.recording_page_role);
    push_str(out, "recording.capture.role", view.recording_capture_role);
    push_str(out, "recording.indicator", view.recording_indicator);
    push_str(out, "playback.indicator", view.playback_indicator);
    out.push_str("strip.aims=\"");
    let mut separator = "";
    for dir in view.strip_aims.into_iter().flatten() {
        if let Some(name) = ["R", "UR", "U", "UL", "L", "DL", "D", "DR"].get(usize::from(dir)) {
            out.push_str(separator);
            out.push_str(name);
            separator = " ";
        }
    }
    out.push_str("\"\n");
    push_str(out, "list", view.list);
    let _ = writeln!(out, "list.count={}", view.list_count);
    let _ = writeln!(out, "list.index={}", view.list_index.map_or(-1, |index| index as i64));
    push_str(out, "list.value", view.list_value);
    push_str(out, "recording.preview", view.recording_preview);
    out.push_str(view.extra);
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

/// The snapshot's `list` keys for `menu`: its focused list, with a table row's value written as the page draws it
/// (`frame::write_value`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListView {
    pub id: String,
    pub count: usize,
    pub index: Option<usize>,
    pub value: String,
}

pub fn list_view(menu: &super::menu::Menu) -> ListView {
    let Some(list) = menu.focus_list() else { return ListView::default() };
    let value = if let Some(row) = list.row {
        let mut text = super::frame::Text::<72>::new();
        super::frame::write_value(menu, row, &mut text);
        text.string_lossy()
    } else {
        list.value
    };
    ListView { id: list.id, count: list.count, index: Some(list.index), value }
}

/// The `list` keys the worker publishes: the model's focused list while the menu's own screens take the presses, else none.
pub fn published_list(menu: Option<&super::menu::Menu>, model_takes_presses: bool) -> ListView {
    menu.filter(|_| model_takes_presses).map(list_view).unwrap_or_default()
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
/// Another plugin has the Bridge registry's registration slot (State API v1).
const REGISTER_BUSY: i32 = -6;

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

/// The HUD row's stamp for one pass: `uptime_ms << 2 | take` (0 the pass drew no take, 1 Player 1's, 2 the CPU's), the
/// scene stamp's form.
pub fn indicator_stamp(now_ms: u64, drawn: Option<super::hud_row::Take>) -> u64 {
    now_ms << 2
        | match drawn {
            None => 0,
            Some(super::hud_row::Take::Player) => 1,
            Some(super::hud_row::Take::Cpu) => 2,
        }
}

/// `recording.indicator` from that stamp: the role of the take the HUD drew, while its last pass is no older than
/// `fresh_ms`. Empty when that pass drew none (no take, the menu over the HUD, a layout without the row's panes) and
/// when no pass ran since (a waiting room without the plugin's host, a scene change).
pub fn recording_indicator(stamp: u64, now_ms: u64, fresh_ms: u64) -> &'static str {
    if now_ms.saturating_sub(stamp >> 2) > fresh_ms {
        return "";
    }
    match stamp & 3 {
        1 => super::hud_row::Take::Player.key(),
        2 => super::hud_row::Take::Cpu.key(),
        _ => "",
    }
}

pub fn playback_indicator_stamp(now_ms: u64, drawn: [bool; 2]) -> u64 {
    (now_ms << 2) | u64::from(drawn[0]) | (u64::from(drawn[1]) << 1)
}

pub fn playback_indicator(stamp: u64, now_ms: u64, fresh_ms: u64) -> &'static str {
    if now_ms.saturating_sub(stamp >> 2) > fresh_ms {
        return "";
    }
    match stamp & 3 {
        1 => "player",
        2 => "cpu",
        3 => "both",
        _ => "",
    }
}

/// A HUD pass and its direction marks, oldest first.
pub fn aims_stamp(now_ms: u64, drawn: [Option<u8>; 2]) -> u64 {
    let nibble = |dir: Option<u8>| u64::from(dir.filter(|dir| *dir < 8).map_or(0, |dir| dir + 1));
    (now_ms << 8) | nibble(drawn[0]) | (nibble(drawn[1]) << 4)
}

/// The direction marks on a fresh HUD pass, oldest first.
pub fn strip_aims(stamp: u64, now_ms: u64, fresh_ms: u64) -> [Option<u8>; 2] {
    if stamp == 0 || now_ms.saturating_sub(stamp >> 8) > fresh_ms {
        return [None; 2];
    }
    let dir = |nibble: u64| (1..=8).contains(&nibble).then(|| (nibble - 1) as u8);
    [dir(stamp & 15), dir((stamp >> 4) & 15)]
}

pub(crate) fn push_str(out: &mut String, key: &str, value: &str) {
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

    /// Classify the bridge's registration result within this lookup schedule.
    pub fn registration(&mut self, result: i32) -> Option<i32> {
        if result == REGISTER_BUSY {
            return None;
        }
        if result < 0 {
            self.disable();
            return None;
        }
        self.found();
        Some(result)
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
