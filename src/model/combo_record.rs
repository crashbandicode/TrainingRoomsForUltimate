//! A saved combo (SPEC-input-07 §4, `docs/COMBOS.md`): `sd:/ultimate/TrainingRooms/combos/<id>.json` on the generic
//! store, with `selected.txt` naming the target. An open, shareable format: read by hand from `serde_json::Value`, so
//! unknown fields are ignored and a bad field is refused with its name; a file people copy in may have any safe file
//! name, may leave out `schema_version` and `name`, and keeps its own extra fields when it is renamed.

use core::fmt;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Map, Value};

use super::combo::{Aim, Combo, LinkRec, Step, LINKS, STEPS};
use super::frame_strip::{Button, Move, Verdict};
use super::store::{default_id, Record, SkippedFile, Store, StoreError};

/// The format's version. Fields added since are optional, so rc.4 (the first writer) reads every file written now.
pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_FILE_BYTES: u64 = 64 * 1024;
/// Longest name, in Unicode scalar values (the keyboard's limit, `MAX_NAME_CHARS` of macros and setups).
pub const MAX_NAME_CHARS: usize = 48;
/// The directory under the data folder.
pub const DIR: &str = "combos";
/// Longest file name a shared combo may have, in characters before `.json`.
pub const MAX_FILE_NAME_CHARS: usize = 64;

pub type ComboLibrary = Store<ComboRecord>;

#[derive(Debug)]
pub enum ComboError {
    Store(StoreError),
    Invalid(&'static str),
}

impl fmt::Display for ComboError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComboError::Store(err) => err.fmt(f),
            ComboError::Invalid(why) => f.write_str(why),
        }
    }
}

impl From<StoreError> for ComboError {
    fn from(err: StoreError) -> Self {
        ComboError::Store(err)
    }
}

#[derive(Clone, Debug)]
pub struct ComboRecord {
    pub name: String,
    pub combo: Combo,
    /// The name of P1's fighter when the combo was saved. Metadata only: the helper grades whatever fighter P1 is.
    pub recorded_on: Option<String>,
    /// The JSON object the record was read from. Writing it back (a rename) changes only `name` (and adds
    /// `schema_version` when the file had none), so fields someone added by hand stay.
    source: Option<Map<String, Value>>,
}

/// Two records are the same combo when their name, links and recorded fighter are (not how their files are laid out).
impl PartialEq for ComboRecord {
    fn eq(&self, other: &Self) -> bool {
        (&self.name, &self.combo, &self.recorded_on) == (&other.name, &other.combo, &other.recorded_on)
    }
}

impl Eq for ComboRecord {}

/// One row of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComboEntry {
    pub id: String,
    pub name: String,
    pub links: usize,
    pub recorded_on: Option<String>,
}

impl ComboEntry {
    /// A file the plugin did not name (someone copied it in): its id is not one the store makes.
    pub fn shared(&self) -> bool {
        !default_id(&self.id)
    }

    /// The list row's text: `Name · N links · recorded on X`.
    pub fn label(&self) -> String {
        let mut label = format!("{} · {} links", self.name, self.links);
        if let Some(fighter) = &self.recorded_on {
            label.push_str(" · recorded on ");
            label.push_str(fighter);
        }
        label
    }
}

/// Whether a file name (without `.json`) may hold a combo: 1 to 64 characters, no path or drive separator, no
/// character a FAT card cannot store, no control character, no leading dot and no surrounding spaces.
pub fn shareable_id(id: &str) -> bool {
    let chars = id.chars().count();
    (1..=MAX_FILE_NAME_CHARS).contains(&chars)
        && !id.starts_with('.')
        && id.trim() == id
        && !id.chars().any(|c| c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
}

/// A name as the list shows it: control characters dropped, trimmed, at most 48 characters; `None` when nothing is left.
fn shown_name(text: &str) -> Option<String> {
    let kept: String = text.chars().filter(|c| !c.is_control()).collect();
    clean_name(&kept)
}

impl ComboRecord {
    /// A combo the plugin saves now (no file yet).
    pub fn new(name: String, combo: Combo, recorded_on: Option<String>) -> Self {
        ComboRecord { name, combo, recorded_on, source: None }
    }

    pub fn validate(&self) -> Result<(), ComboError> {
        let chars = self.name.trim().chars().count();
        if chars == 0 || chars > MAX_NAME_CHARS || self.name.chars().any(char::is_control) {
            return Err(ComboError::Invalid("a combo name is 1 to 48 characters"));
        }
        if self.combo.is_empty() {
            return Err(ComboError::Invalid("a combo has at least one link"));
        }
        Ok(())
    }

    fn to_value(&self) -> Value {
        if let Some(source) = &self.source {
            let mut object = source.clone();
            object.insert("schema_version".into(), json!(SCHEMA_VERSION));
            object.insert("name".into(), json!(self.name));
            return Value::Object(object);
        }
        let links: Vec<Value> = self
            .combo
            .links()
            .map(|l| {
                json!({
                    "from": move_key(l.from),
                    "to": move_key(l.to),
                    "verdict": match l.verdict {
                        Verdict::True(link) => json!({"kind": "true", "link": link}),
                        Verdict::Drop(link) => json!({"kind": "drop", "link": link}),
                    },
                    "window": l.window.map(|(a, b)| json!([a, b])),
                    "steps": l.steps().map(|s| json!({
                        "button": button_key(s.button), "aim": aim_key(s.aim), "offset": s.offset,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();
        let mut value = json!({"schema_version": SCHEMA_VERSION, "name": self.name, "links": links});
        if let Some(fighter) = &self.recorded_on {
            value["recorded_on"] = json!(fighter);
        }
        value
    }

    /// Tolerant where a person may have typed the file, strict where a wrong value would grade the wrong thing:
    /// `schema_version` may be left out (it is 1), `name` may be left out or blank (the file name is used,
    /// `Record::adopt`), `recorded_on` is kept only when it is text, and unknown fields are ignored; the links must
    /// be exact.
    fn from_value(value: &Value) -> Result<ComboRecord, &'static str> {
        let object = value.as_object().ok_or("not an object")?;
        match object.get("schema_version") {
            None => {}
            Some(version) if version.as_u64() == Some(u64::from(SCHEMA_VERSION)) => {}
            Some(_) => return Err("schema_version"),
        }
        let name = match object.get("name") {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(text)) => shown_name(text).unwrap_or_default(),
            Some(_) => return Err("name"),
        };
        let recorded_on = object.get("recorded_on").and_then(Value::as_str).and_then(shown_name);
        let links = object.get("links").and_then(Value::as_array).ok_or("links")?;
        if links.is_empty() || links.len() > LINKS {
            return Err("links");
        }
        let combo = Combo::from_links(links.iter().map(link_of).collect::<Result<Vec<_>, _>>()?).ok_or("links")?;
        Ok(ComboRecord { name, combo, recorded_on, source: Some(object.clone()) })
    }
}

fn int(object: &Map<String, Value>, key: &'static str) -> Result<i32, &'static str> {
    object.get(key).and_then(Value::as_i64).and_then(|v| i32::try_from(v).ok()).ok_or(key)
}

fn link_of(value: &Value) -> Result<LinkRec, &'static str> {
    let o = value.as_object().ok_or("link")?;
    let key = |k: &'static str| o.get(k).and_then(Value::as_str).ok_or(k);
    let verdict = o.get("verdict").and_then(Value::as_object).ok_or("verdict")?;
    let link = match verdict.get("link") {
        None | Some(Value::Null) => None,
        Some(_) => Some(int(verdict, "link")?),
    };
    let verdict = match verdict.get("kind").and_then(Value::as_str) {
        Some("true") => Verdict::True(link),
        Some("drop") => Verdict::Drop(link.ok_or("verdict")?),
        _ => return Err("verdict"),
    };
    let window = match o.get("window") {
        None | Some(Value::Null) => None,
        Some(Value::Array(pair)) if pair.len() == 2 => {
            let at = |i: usize| pair[i].as_i64().and_then(|v| i32::try_from(v).ok()).ok_or("window");
            Some((at(0)?, at(1)?))
        }
        _ => return Err("window"),
    };
    let steps_in = o.get("steps").and_then(Value::as_array).ok_or("steps")?;
    if steps_in.len() > STEPS {
        return Err("steps");
    }
    let mut steps = [None; STEPS];
    for (slot, step) in steps.iter_mut().zip(steps_in) {
        let s = step.as_object().ok_or("step")?;
        let name = |k: &'static str| s.get(k).and_then(Value::as_str).ok_or(k);
        *slot = Some(Step {
            button: button_of(name("button")?).ok_or("button")?,
            aim: aim_of(name("aim")?).ok_or("aim")?,
            offset: int(s, "offset")?,
        });
    }
    Ok(LinkRec {
        from: move_of(key("from")?).ok_or("from")?,
        to: move_of(key("to")?).ok_or("to")?,
        verdict,
        window,
        steps,
    })
}

const MOVES: [(Move, &str); 14] = [
    (Move::Jab, "jab"),
    (Move::DashAttack, "dash_attack"),
    (Move::SideTilt, "side_tilt"),
    (Move::UpTilt, "up_tilt"),
    (Move::DownTilt, "down_tilt"),
    (Move::SideSmash, "side_smash"),
    (Move::UpSmash, "up_smash"),
    (Move::DownSmash, "down_smash"),
    (Move::Aerial, "aerial"),
    (Move::NeutralSpecial, "neutral_special"),
    (Move::SideSpecial, "side_special"),
    (Move::UpSpecial, "up_special"),
    (Move::DownSpecial, "down_special"),
    (Move::Other, "other"),
];
const BUTTONS: [(Button, &str); 5] =
    [(Button::Attack, "attack"), (Button::Special, "special"), (Button::Jump, "jump"), (Button::Shield, "shield"), (Button::Grab, "grab")];
const AIMS: [(Aim, &str); 4] = [(Aim::Neutral, "neutral"), (Aim::Up, "up"), (Aim::Down, "down"), (Aim::Side, "side")];

fn key_of<T: PartialEq + Copy>(table: &[(T, &'static str)], value: T) -> &'static str {
    table.iter().find(|(v, _)| *v == value).map_or("", |(_, k)| k)
}

fn value_of<T: Copy>(table: &[(T, &'static str)], key: &str) -> Option<T> {
    table.iter().find(|(_, k)| *k == key).map(|(v, _)| *v)
}

pub fn move_key(m: Move) -> &'static str {
    key_of(&MOVES, m)
}
fn move_of(key: &str) -> Option<Move> {
    value_of(&MOVES, key)
}
fn button_key(b: Button) -> &'static str {
    key_of(&BUTTONS, b)
}
fn button_of(key: &str) -> Option<Button> {
    value_of(&BUTTONS, key)
}
fn aim_key(a: Aim) -> &'static str {
    key_of(&AIMS, a)
}
fn aim_of(key: &str) -> Option<Aim> {
    value_of(&AIMS, key)
}

impl Serialize for ComboRecord {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_value().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ComboRecord {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        ComboRecord::from_value(&value).map_err(|field| D::Error::custom(format!("bad combo field: {field}")))
    }
}

impl Record for ComboRecord {
    type Error = ComboError;
    type Summary = ComboEntry;
    const SCHEMA_VERSIONS: &'static [u32] = &[SCHEMA_VERSION];
    const MAX_FILE_BYTES: u64 = MAX_FILE_BYTES;

    fn validate(&self) -> Result<(), ComboError> {
        ComboRecord::validate(self)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn summary(&self, id: &str) -> ComboEntry {
        ComboEntry { id: id.to_owned(), name: self.name.clone(), links: self.combo.len(), recorded_on: self.recorded_on.clone() }
    }

    /// Shared files may have any safe name (`shareable_id`), not only the slugs the store makes.
    fn accepts_id(id: &str) -> bool {
        shareable_id(id)
    }

    /// A file without a usable `name` is called by its file name.
    fn adopt(&mut self, id: &str) {
        if self.name.is_empty() {
            self.name = shown_name(id).unwrap_or_default();
        }
    }
}

/// The notice for files the list could not use: the first one's name and why, and how many more. `None` when there are none.
pub fn skipped_note(skipped: &[SkippedFile]) -> Option<String> {
    let first = skipped.first()?;
    let file = if first.file.is_empty() { "A combo file" } else { first.file.as_str() };
    let mut note = format!("{file} skipped: {}", first.reason);
    if skipped.len() > 1 {
        note.push_str(&format!(" (+{} more)", skipped.len() - 1));
    }
    Some(note)
}

/// "Combo N" with the lowest N no entry uses (SPEC-input-07 §5).
pub fn default_name(entries: &[ComboEntry]) -> String {
    (1..).map(|n| format!("Combo {n}")).find(|name| !entries.iter().any(|e| e.name.eq_ignore_ascii_case(name))).unwrap_or_default()
}

/// A typed name, trimmed; `None` when nothing is left.
pub fn clean_name(typed: &str) -> Option<String> {
    let name: String = typed.trim().chars().take(MAX_NAME_CHARS).collect();
    (!name.is_empty()).then_some(name)
}

#[cfg(test)]
#[path = "combo_record_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "combo_record_share_tests.rs"]
mod share_tests;
