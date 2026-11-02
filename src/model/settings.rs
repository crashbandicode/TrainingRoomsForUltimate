//! The settings file (SPEC-core-04, decisions S1 and S2): `sd:/ultimate/TrainingRooms/settings.json`.
//!
//! ```text
//! { "menu":          { "<setting id>": [<u8>, …], … },
//!   "defaults_menu": { "<setting id>": [<u8>, …], … },
//!   …other top-level keys of the fork (mii_preferences, stage_preferences) kept as they are }
//! ```
//!
//! Every setting is an array of small integers, one element per option in the option set's
//! declaration order (`0` not selected, `1` selected, `2..=5` a weight on a weighted row); a slider
//! is `[lower, upper]`. The shape and the ids are the fork's, checked against two real files from
//! the fork (`tests/fixtures/fork-menu-*.json`).
//!
//! Reading is field by field (S2): a field that does not parse takes its factory value, the rest of
//! the file still loads, and `Loaded::issues` lists what was wrong so the caller keeps a renamed
//! copy of the file. A key the table below does not know is kept verbatim and written back, so a
//! file written by a later build (or by the fork, for rows this build does not have yet) survives
//! a save; the retired keys `update_policy` and `menu_open_start_press` are read and dropped.
//! The factory table (`DEFS`, in `model::factory`) holds every row of the menu with a plain array
//! value; the rows whose value is not one (the fork's rosters, hats, Mii picks and stage
//! preferences) are kept as they are until the package that owns them gives them a row.

use std::fmt;

use serde::de::{Deserialize, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};

/// Top-level key holding the page id the menu opens on (Q5; SPEC-core-04 §3.1).
pub const LAST_PAGE_KEY: &str = "last_page";
/// Largest weight on a weighted row.
pub const WEIGHT_MAX: u8 = 5;
/// Largest slider value (the two damage ranges).
pub const SLIDER_MAX: u8 = 150;
/// Keys older builds wrote that nothing reads any more (SPEC-core-01 §6, SPEC-core-02 question 4;
/// `character_item` since decision N6, converted once by the first-boot import).
pub const RETIRED: [&str; 3] = ["update_policy", "menu_open_start_press", "character_item"];

/// How a row's array is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// One option selected (`1`), the rest `0`.
    Single,
    /// Each option on (`1`) or off (`0`).
    Multiple,
    /// Each option `0..=WEIGHT_MAX`.
    Weighted,
    /// `[lower, upper]`, `lower <= upper <= SLIDER_MAX`.
    Slider,
    /// The optional range of a delay row (N3): empty (no range) or `[min, max]` as **option
    /// indices** of the row's own value set, `min <= max <= RANGE_INDEX_MAX`. Absent from the file
    /// while empty.
    Range,
}

/// Largest option index a delay range may name (the delay rows have 31 options, `0..=30`).
pub const RANGE_INDEX_MAX: u8 = 30;

impl Kind {
    pub const fn max(self) -> u8 {
        match self {
            Kind::Single | Kind::Multiple => 1,
            Kind::Weighted => WEIGHT_MAX,
            Kind::Slider => SLIDER_MAX,
            Kind::Range => RANGE_INDEX_MAX,
        }
    }
}

/// One row of the factory table (SPEC-core-02 §2.7).
#[derive(Clone, Copy, Debug)]
pub struct Def {
    pub id: &'static str,
    pub kind: Kind,
    /// Number of elements in the array.
    pub len: usize,
    pub default: &'static [u8],
}

pub use super::factory::{DEFS, INPUT_DELAY, LRA_RESET, STAGE_HAZARDS};

/// The table row called `id`, with its index.
pub fn def(id: &str) -> Option<(usize, &'static Def)> {
    DEFS.iter().enumerate().find(|(_, def)| def.id == id)
}

/// Why a value or a file was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reason {
    /// The text is not JSON.
    Syntax(String),
    /// The top level is not an object.
    NotAnObject,
    /// `menu` or `defaults_menu` is present but is not an object.
    SnapshotNotAnObject,
    /// A key appears twice in one object.
    DuplicateKey,
    NotAnArray,
    /// An element is not an integer in `0..=255`.
    ElementNotAByte,
    /// An element is above the row's largest value.
    ElementAboveMax,
    /// A slider array does not have exactly two elements.
    BadSliderLength,
    /// A slider's `lower` is above its `upper` (or either is above `SLIDER_MAX`).
    BadSliderRange,
    /// `Snapshot::set` was given an id this build has no row for.
    UnknownId,
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reason::Syntax(message) => write!(f, "not JSON: {message}"),
            Reason::NotAnObject => f.write_str("top level is not an object"),
            Reason::SnapshotNotAnObject => f.write_str("not an object"),
            Reason::DuplicateKey => f.write_str("duplicate key"),
            Reason::NotAnArray => f.write_str("not an array"),
            Reason::ElementNotAByte => f.write_str("element is not an integer 0..=255"),
            Reason::ElementAboveMax => f.write_str("element above the row's largest value"),
            Reason::BadSliderLength => f.write_str("slider needs exactly two elements"),
            Reason::BadSliderRange => f.write_str("slider range is not lower <= upper <= 150"),
            Reason::UnknownId => f.write_str("no such setting"),
        }
    }
}

/// Check one row's JSON value against its table row and return the elements: a longer array loses
/// its extra elements, a shorter one leaves the tail off (SPEC-core-04 §3.2, SPEC-core-02 §5).
pub fn parse_row(def: &Def, value: &Value) -> Result<Vec<u8>, Reason> {
    let Value::Array(items) = value else {
        return Err(Reason::NotAnArray);
    };
    let mut elements = Vec::with_capacity(items.len());
    for item in items {
        let byte = item.as_u64().and_then(|n| u8::try_from(n).ok()).ok_or(Reason::ElementNotAByte)?;
        if byte > def.kind.max() {
            return Err(Reason::ElementAboveMax);
        }
        elements.push(byte);
    }
    if def.kind == Kind::Range && elements.is_empty() {
        return Ok(elements);
    }
    if matches!(def.kind, Kind::Slider | Kind::Range) {
        let &[lower, upper] = elements.as_slice() else {
            return Err(Reason::BadSliderLength);
        };
        if lower > upper {
            return Err(Reason::BadSliderRange);
        }
        return Ok(elements);
    }
    elements.resize(def.len, 0);
    Ok(elements)
}

/// One of the two snapshots: the current settings, or the player's saved defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    /// One entry per `DEFS` row, same order.
    rows: Vec<Vec<u8>>,
    /// Keys this build does not know, kept verbatim.
    extra: Map<String, Value>,
}

impl Snapshot {
    /// Every known row at its factory value.
    pub fn factory() -> Self {
        Self { rows: DEFS.iter().map(|def| def.default.to_vec()).collect(), extra: Map::new() }
    }

    /// A known row's elements.
    pub fn get(&self, id: &str) -> Option<&[u8]> {
        def(id).map(|(index, _)| self.rows[index].as_slice())
    }

    /// Replace a known row (the menu's commit). The value is checked like a file's.
    pub fn set(&mut self, id: &str, elements: &[u8]) -> Result<(), Reason> {
        let (index, def) = def(id).ok_or(Reason::UnknownId)?;
        let value = Value::Array(elements.iter().map(|b| Value::from(*b)).collect());
        self.rows[index] = parse_row(def, &value)?;
        Ok(())
    }

    /// The keys this build keeps without knowing them.
    pub fn extra(&self) -> &Map<String, Value> {
        &self.extra
    }

    /// Input Delay in frames: the first selected option; an empty selection means 0
    /// (SPEC-input-03 §2.1).
    pub fn input_delay(&self) -> u8 {
        let row = &self.rows[INPUT_DELAY];
        row.iter().position(|v| *v != 0).map_or(0, |i| i.min(30) as u8)
    }

    /// L+R+A Reset: On when the first option (`On`) is set. Neither option set reads as Off.
    pub fn lra_reset(&self) -> bool {
        self.rows[LRA_RESET][0] != 0
    }

    /// Stage Hazards: On when the first option (`On`) is set.
    pub fn stage_hazards(&self) -> bool {
        self.rows[STAGE_HAZARDS][0] != 0
    }

    fn to_value(&self) -> Value {
        let mut object = self.extra.clone();
        for (def, row) in DEFS.iter().zip(&self.rows) {
            // An empty delay range is "no range": the key is absent (SPEC-core-04 §3.1).
            if def.kind == Kind::Range && row.is_empty() {
                continue;
            }
            object.insert(def.id.to_owned(), Value::Array(row.iter().map(|b| Value::from(*b)).collect()));
        }
        Value::Object(object)
    }
}

/// The whole file.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub menu: Snapshot,
    pub defaults: Snapshot,
    /// Other top-level keys, kept verbatim (the fork's `mii_preferences`, `stage_preferences`).
    pub extra: Map<String, Value>,
}

impl Settings {
    pub fn factory() -> Self {
        Self { menu: Snapshot::factory(), defaults: Snapshot::factory(), extra: Map::new() }
    }

    /// The page the menu was last left on (`last_page`, SPEC-core-04 §3.1): a top-level string, kept
    /// with the other top-level keys. `None` when absent or not a string.
    pub fn last_page(&self) -> Option<&str> {
        self.extra.get(LAST_PAGE_KEY).and_then(Value::as_str)
    }

    /// Record the page the menu is left on; the next close writes it.
    pub fn set_last_page(&mut self, page_id: &str) {
        self.extra.insert(LAST_PAGE_KEY.to_owned(), Value::from(page_id));
    }

    /// The file's contents: pretty-printed JSON, for hand editing (SPEC-core-04 question 6). Key
    /// order is not part of the format.
    pub fn to_json(&self) -> String {
        let mut object = self.extra.clone();
        object.insert("menu".to_owned(), self.menu.to_value());
        object.insert("defaults_menu".to_owned(), self.defaults.to_value());
        serde_json::to_string_pretty(&Value::Object(object)).unwrap_or_default()
    }
}

/// One thing that was wrong in a file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    /// Where: empty for the file, `menu`, `defaults_menu`, or `menu.<id>` / `defaults_menu.<id>`.
    pub at: String,
    pub reason: Reason,
}

/// What reading a file produced.
#[derive(Clone, Debug, PartialEq)]
pub struct Loaded {
    pub settings: Settings,
    /// Empty for a clean file. Otherwise the caller keeps a renamed copy (S2) and logs these.
    pub issues: Vec<Issue>,
}

impl Loaded {
    pub fn corrupt(&self) -> bool {
        !self.issues.is_empty()
    }
}

/// The fields of one JSON object, in order, with every repeated key noted.
#[derive(Default)]
struct Entries {
    items: Vec<(String, Value)>,
    duplicates: Vec<String>,
}

impl Entries {
    fn put(&mut self, key: String, value: Value) {
        if self.items.iter().any(|(k, _)| *k == key) {
            self.duplicates.push(key);
        } else {
            self.items.push((key, value));
        }
    }
}

struct EntriesVisitor;

impl<'de> Visitor<'de> for EntriesVisitor {
    type Value = Entries;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Entries, A::Error> {
        let mut entries = Entries::default();
        while let Some(key) = map.next_key::<String>()? {
            let value = map.next_value::<Value>()?;
            entries.put(key, value);
        }
        Ok(entries)
    }
}

/// A snapshot value: its fields when it is an object, otherwise just the fact that it is not.
enum Nested {
    Object(Entries),
    Other,
}

struct NestedVisitor;

impl<'de> Visitor<'de> for NestedVisitor {
    type Value = Nested;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any JSON value")
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Nested, A::Error> {
        EntriesVisitor.visit_map(map).map(Nested::Object)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Nested, A::Error> {
        while seq.next_element::<IgnoredAny>()?.is_some() {}
        Ok(Nested::Other)
    }

    fn visit_bool<E>(self, _: bool) -> Result<Nested, E> {
        Ok(Nested::Other)
    }

    fn visit_i64<E>(self, _: i64) -> Result<Nested, E> {
        Ok(Nested::Other)
    }

    fn visit_u64<E>(self, _: u64) -> Result<Nested, E> {
        Ok(Nested::Other)
    }

    fn visit_f64<E>(self, _: f64) -> Result<Nested, E> {
        Ok(Nested::Other)
    }

    fn visit_str<E>(self, _: &str) -> Result<Nested, E> {
        Ok(Nested::Other)
    }

    fn visit_unit<E>(self) -> Result<Nested, E> {
        Ok(Nested::Other)
    }

    fn visit_none<E>(self) -> Result<Nested, E> {
        Ok(Nested::Other)
    }

    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Nested, D::Error> {
        d.deserialize_any(NestedVisitor)
    }
}

impl<'de> Deserialize<'de> for Nested {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(NestedVisitor)
    }
}

/// The top-level object: the two snapshots with their duplicates noted, every other key verbatim.
#[derive(Default)]
struct Top {
    menu: Option<Nested>,
    defaults: Option<Nested>,
    others: Entries,
    /// `menu` or `defaults_menu` appeared more than once.
    repeated: Vec<&'static str>,
}

struct TopVisitor;

impl<'de> Visitor<'de> for TopVisitor {
    type Value = Top;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Top, A::Error> {
        let mut top = Top::default();
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "menu" => {
                    let nested = map.next_value::<Nested>()?;
                    if top.menu.replace(nested).is_some() {
                        top.repeated.push("menu");
                    }
                }
                "defaults_menu" => {
                    let nested = map.next_value::<Nested>()?;
                    if top.defaults.replace(nested).is_some() {
                        top.repeated.push("defaults_menu");
                    }
                }
                _ => {
                    let value = map.next_value::<Value>()?;
                    top.others.put(key, value);
                }
            }
        }
        Ok(top)
    }
}

impl<'de> Deserialize<'de> for Top {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_map(TopVisitor)
    }
}

fn snapshot_from(label: &'static str, nested: Option<Nested>, issues: &mut Vec<Issue>) -> Snapshot {
    let mut snapshot = Snapshot::factory();
    let entries = match nested {
        None => return snapshot,
        Some(Nested::Other) => {
            issues.push(Issue { at: label.to_owned(), reason: Reason::SnapshotNotAnObject });
            return snapshot;
        }
        Some(Nested::Object(entries)) => entries,
    };
    for (id, value) in entries.items {
        if RETIRED.contains(&id.as_str()) {
            continue;
        }
        match def(&id) {
            Some((index, def)) => match parse_row(def, &value) {
                Ok(row) => snapshot.rows[index] = row,
                Err(reason) => issues.push(Issue { at: format!("{label}.{id}"), reason }),
            },
            None => {
                snapshot.extra.insert(id, value);
            }
        }
    }
    for id in entries.duplicates {
        issues.push(Issue { at: format!("{label}.{id}"), reason: Reason::DuplicateKey });
        // A key that was given twice has no single value to trust: a known row takes its factory value.
        if let Some((index, def)) = def(&id) {
            snapshot.rows[index] = def.default.to_vec();
        }
    }
    snapshot
}

/// Read the file's contents. Never fails: whatever cannot be read takes its factory value and is
/// listed in `Loaded::issues`.
pub fn parse(text: &str) -> Loaded {
    let mut issues = Vec::new();
    let top = match serde_json::from_str::<Top>(text) {
        Ok(top) => top,
        Err(error) => {
            let reason = if error.is_data() { Reason::NotAnObject } else { Reason::Syntax(error.to_string()) };
            issues.push(Issue { at: String::new(), reason });
            return Loaded { settings: Settings::factory(), issues };
        }
    };
    for label in &top.repeated {
        issues.push(Issue { at: (*label).to_owned(), reason: Reason::DuplicateKey });
    }
    for key in &top.others.duplicates {
        issues.push(Issue { at: key.clone(), reason: Reason::DuplicateKey });
    }
    let menu = snapshot_from("menu", top.menu, &mut issues);
    let defaults = snapshot_from("defaults_menu", top.defaults, &mut issues);
    let mut extra = Map::new();
    for (key, value) in top.others.items {
        if !RETIRED.contains(&key.as_str()) {
            extra.insert(key, value);
        }
    }
    Loaded { settings: Settings { menu, defaults, extra }, issues }
}

/// `parse` for the bytes of a file: text that is not UTF-8 is a corrupt file like any other.
pub fn parse_bytes(bytes: &[u8]) -> Loaded {
    match std::str::from_utf8(bytes) {
        Ok(text) => parse(text),
        Err(error) => Loaded {
            settings: Settings::factory(),
            issues: vec![Issue { at: String::new(), reason: Reason::Syntax(format!("not UTF-8: {error}")) }],
        },
    }
}

/// Close of the menu, as far as the file is concerned (SPEC-core-04 §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseReason {
    /// The player closed the menu.
    Closed,
    /// Every controller was unplugged: nothing is written (SPEC-core-03 §2.3).
    AllControllersUnplugged,
}

/// Whether closing the menu for `reason` writes the file.
pub const fn writes_on_close(reason: CloseReason) -> bool {
    matches!(reason, CloseReason::Closed)
}

/// The steps of a commit, in order: the globals features read are replaced before the write is
/// queued, so features see the change from the next frame (SPEC-core-04 §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitStep {
    Publish,
    QueueWrite,
}

pub const COMMIT_ORDER: [CommitStep; 2] = [CommitStep::Publish, CommitStep::QueueWrite];

/// How the file is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteMode {
    /// A writer thread with a 64 KiB stack takes the jobs.
    Thread,
    /// The thread could not be created: write on the caller (SPEC-core-04 §6 "Thread failure").
    Synchronous,
}

/// The stack of the writer thread and the boot-time loader thread (SPEC-core-04 §5, §8).
pub const WRITER_STACK_BYTES: usize = 64 * 1024;
pub const LOADER_STACK_BYTES: usize = 128 * 1024;

pub const fn write_mode(writer_thread_started: bool) -> WriteMode {
    if writer_thread_started {
        WriteMode::Thread
    } else {
        WriteMode::Synchronous
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_round_trips() {
        let factory = Settings::factory();
        assert_eq!(parse(&factory.to_json()).settings, factory);
    }

    #[test]
    fn a_bad_row_defaults_and_is_reported() {
        let loaded = parse(r#"{"menu":{"lra_reset":"x","stage_hazards":[1,0]}}"#);
        assert_eq!(loaded.issues.len(), 1);
        assert!(loaded.settings.menu.lra_reset());
        assert!(loaded.settings.menu.stage_hazards());
    }
}