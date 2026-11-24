//! A saved combo (SPEC-input-07 §4, `docs/COMBOS.md`): `sd:/ultimate/TrainingRooms/combos/<id>.json` on the generic
//! store, with `selected.txt` naming the target. An open, shareable format: read by hand from `serde_json::Value`, so
//! unknown fields are ignored and a bad field is refused with its name; a file people copy in may have any safe file
//! name, may leave out `schema_version` and `name`, and keeps its own extra fields when it is renamed.

use core::fmt;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{json, Map, Value};

use super::action::{ActionDesc, Travel};
use super::combo::{Aim, Combo, Dir, Event, Flick, Kind, LinkRec, LEGACY_STEPS, LINKS, STEPS};
use super::frame_strip::{Angle, Button, Move, Verdict};
use super::spots::Spot;
use super::store::{default_id, Record, SkippedFile, Store, StoreError};

/// The format's version. Fields added since are optional, so rc.4 (the first writer) reads every file written now.
pub const SCHEMA_VERSION: u32 = 1;
// restriction: combo-file-size — worker-side reads retain the existing 4 KiB per unit budget.
pub const MAX_FILE_BYTES: u64 = LINKS as u64 * 4096;
/// Longest name, in Unicode scalar values (the keyboard's limit, `MAX_NAME_CHARS` of macros and setups).
pub const MAX_NAME_CHARS: usize = 48;
/// The directory under the data folder.
pub const DIR: &str = "combos";
/// The start of the name of a macro file a drill owns as its take (F10). A key of 16 hex digits follows.
pub const DRILL_TAKE_PREFIX: &str = "Drill take ";

/// A take's name for `key`. The file's id comes from the name, so every take has an id of its own and a drill file
/// from another card names no take here.
pub fn drill_take_name(key: u64) -> String {
    format!("{DRILL_TAKE_PREFIX}{key:016x}")
}

/// `name` is a take's name: the prefix and exactly 16 lower-case hex digits.
pub fn is_drill_take_name(name: &str) -> bool {
    name.strip_prefix(DRILL_TAKE_PREFIX)
        .is_some_and(|key| key.len() == 16 && key.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
}
/// Longest file name a shared combo may have, in characters before `.json`.
pub const MAX_FILE_NAME_CHARS: usize = 64;

pub type ComboLibrary = Store<ComboRecord>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    Link,
    Step,
}
impl UnitKind {
    pub fn singular(self) -> &'static str {
        match self {
            Self::Link => "link",
            Self::Step => "step",
        }
    }
    pub fn plural(self) -> &'static str {
        match self {
            Self::Link => "links",
            Self::Step => "steps",
        }
    }
    pub fn name_prefix(self) -> &'static str {
        match self {
            Self::Link => "Combo",
            Self::Step => "Drill",
        }
    }
}

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
    /// The recording or chain this combo was made from. Absent on a combo saved by hand.
    pub made_from: Option<String>,
    /// The macro file that holds a directly recorded input drill's take (F10). The drill owns it: the menu lists it nowhere and deleting the drill deletes it.
    pub take: Option<String>,
    /// The JSON object the record was read from. Writing it back (a rename) changes only `name` (and adds
    /// `schema_version` when the file had none), so fields someone added by hand stay.
    source: Option<Map<String, Value>>,
}

/// Two records are the same combo when their name, links and recorded fighter are (not how their files are laid out).
impl PartialEq for ComboRecord {
    fn eq(&self, other: &Self) -> bool {
        (&self.name, &self.combo, &self.recorded_on, &self.made_from, &self.take)
            == (
                &other.name,
                &other.combo,
                &other.recorded_on,
                &other.made_from,
                &other.take,
            )
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
        && !id.chars().any(|c| {
            c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
}

/// A name as the list shows it: control characters dropped, trimmed, at most 48 characters; `None` when nothing is left.
fn shown_name(text: &str) -> Option<String> {
    let kept: String = text.chars().filter(|c| !c.is_control()).collect();
    clean_name(&kept)
}

impl ComboRecord {
    pub fn unit_kind(&self) -> UnitKind {
        if self.combo.input_only {
            UnitKind::Step
        } else {
            UnitKind::Link
        }
    }
    pub fn unit_count(&self) -> u32 {
        self.combo.len() as u32
    }
    /// A crop has different inputs, so it does not keep the original drill's take.
    pub fn crop_units(&self, start: u32, end: u32) -> Result<Self, ComboError> {
        if start >= end || end > self.unit_count() {
            return Err(ComboError::Invalid("crop units"));
        }
        let origin = if self.combo.input_only { self.combo.get(start as usize).unwrap().events().next().unwrap().offset } else { 0 };
        let mut combo = Combo::from_links(self.combo.links().skip(start as usize).take((end-start) as usize).map(|link| {
            let mut link = *link;
            for event in link.events.iter_mut().flatten() { event.offset -= origin; }
            link
        })).unwrap();
        combo.input_only = self.combo.input_only;
        for old in start as usize..end as usize {
            for event in 0..STEPS {
                if let Some(travel) = self.combo.travel(old, event) {
                    combo.attach_travel(old - start as usize, event, travel);
                }
            }
        }
        let mut record = Self::new(self.name.clone(), combo, self.recorded_on.clone());
        record.made_from = self.made_from.clone();
        Ok(record)
    }

    /// A combo the plugin saves now (no file yet).
    pub fn new(name: String, combo: Combo, recorded_on: Option<String>) -> Self {
        ComboRecord {
            name,
            combo,
            recorded_on,
            made_from: None,
            take: None,
            source: None,
        }
    }

    /// Made from a recording or a chain. The combo file keeps that id.
    pub fn with_made_from(mut self, source: impl Into<String>) -> Self {
        self.made_from = Some(source.into());
        self
    }

    /// Give a directly recorded drill ownership of the macro file holding its take.
    pub fn with_take(mut self, id: impl Into<String>) -> Self {
        self.take = Some(id.into());
        self
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
        let version = match (self.combo.input_only, self.combo.has_aim()) {
            (false, false) => SCHEMA_VERSION,
            (true, false) => 2,
            (false, true) => 3,
            (true, true) => 4,
        };
        if let Some(source) = &self.source {
            let mut object = source.clone();
            object.insert("schema_version".into(), json!(version));
            object.insert("name".into(), json!(self.name));
            return Value::Object(object);
        }
        let links: Vec<Value> = self
            .combo
            .links()
            .enumerate()
            .map(|(k, l)| {
                let holds = if self.combo.input_only { self.combo.holds(k) } else { l.holds() };
                if self.combo.input_only {
                    return json!({"events": events_value(l, &holds, Some(&self.combo.travels(k)))});
                }
                let mut link = json!({
                    "from": move_key(l.from),
                    "to": move_key(l.to),
                    "verdict": match l.verdict {
                        Verdict::True(link) => json!({"kind": "true", "link": link}),
                        Verdict::Drop(link) => json!({"kind": "drop", "link": link}),
                    },
                    "window": l.window.map(|(a, b)| json!([a, b])),
                    "steps": l.legacy_presses().filter_map(|event| match event.kind {
                        Kind::Press(button) => Some(json!({
                            "button": button_key(button),
                            "aim": key_of(&AIMS, event.dir.legacy_aim()),
                            "offset": event.offset,
                        })),
                        _ => None,
                    }).collect::<Vec<_>>(),
                });
                if let Some(spot) = l.spot {
                    link["spot"] = json!(spot.name().to_ascii_lowercase());
                }
                if l.described() {
                    link["from_shown"] = description_value(l.desc(0));
                    link["to_shown"] = description_value(l.desc(1));
                }
                if !l.presses_only {
                    link["events"] = Value::Array(events_value(l, &holds, None));
                    if let Some(dir) = l.drift {
                        link["drift"] = json!(dir_key(dir));
                    }
                }
                link
            })
            .collect();
        let mut value = json!({"schema_version": version, "name": self.name});
        value[if self.combo.input_only { "input_steps" } else { "links" }] = Value::Array(links);
        if self.combo.input_only { value["kind"] = json!("input_drill"); }
        if let Some(fighter) = &self.recorded_on {
            value["recorded_on"] = json!(fighter);
        }
        if let Some(source) = &self.made_from {
            value["made_from"] = json!(source);
        }
        if let Some(take) = &self.take {
            value["take"] = json!(take);
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
            Some(version) if matches!(version.as_u64(), Some(1..=4)) => {}
            Some(_) => return Err("schema_version"),
        }
        let name = match object.get("name") {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(text)) => shown_name(text).unwrap_or_default(),
            Some(_) => return Err("name"),
        };
        let shown = |key| object.get(key).and_then(Value::as_str).and_then(shown_name);
        let recorded_on = shown("recorded_on");
        let made_from = shown("made_from");
        let take = object.get("take").and_then(Value::as_str).filter(|id| shareable_id(id)).map(str::to_owned);
        let version = object.get("schema_version").and_then(Value::as_u64).unwrap_or(1);
        let input = matches!(version, 2 | 4);
        let aims = matches!(version, 3 | 4);
        if input && object.get("kind").and_then(Value::as_str) != Some("input_drill") { return Err("kind"); }
        let key = if input { "input_steps" } else { "links" };
        let links = object.get(key).and_then(Value::as_array).ok_or(key)?;
        if links.is_empty() || links.len() > LINKS { return Err(key); }
        let mut previous = 0;
        let mut combo = Combo::from_links(links.iter().map(|value| {
            if !input { return link_of(value, aims); }
            let events = value.get("events").and_then(Value::as_array).ok_or("events")?;
            if events.is_empty() || events.len() > STEPS { return Err("events"); }
            let mut link = LinkRec::INPUT;
            for (slot, event) in link.events.iter_mut().zip(events) {
                let event = parse_event(event.as_object().ok_or("event")?, aims)?;
                if event.offset < previous { return Err("offset"); }
                previous = event.offset;
                *slot = Some(event);
            }
            Ok(link)
        }).collect::<Result<Vec<_>,_>>()?).ok_or(key)?;
        combo.input_only = input;
        if input && combo.get(0).unwrap().events().next().unwrap().offset != 0 { return Err("offset"); }
        if input {
            for (link, value) in links.iter().enumerate() {
                let events = value.get("events").and_then(Value::as_array).ok_or("events")?;
                for (index, value) in events.iter().enumerate() {
                    if matches!(combo.get(link).and_then(|link| link.events[index]), Some(Event { kind: Kind::Aim(_), .. }))
                        && value.get("path").is_some()
                    {
                        let travel = parse_travel(value.as_object().ok_or("event")?)?;
                        combo.attach_travel(link, index, travel);
                    }
                }
            }
            combo.join_split_steps();
        }
        Ok(ComboRecord {
            name,
            combo,
            recorded_on,
            made_from,
            take,
            source: Some(object.clone()),
        })
    }
}

fn int(object: &Map<String, Value>, key: &'static str) -> Result<i32, &'static str> {
    object
        .get(key)
        .and_then(int_value)
        .ok_or(key)
}

fn link_of(value: &Value, aims: bool) -> Result<LinkRec, &'static str> {
    let o = value.as_object().ok_or("link")?;
    let key = |k: &'static str| o.get(k).and_then(Value::as_str).ok_or(k);
    let verdict = o
        .get("verdict")
        .and_then(Value::as_object)
        .ok_or("verdict")?;
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
            let at = |i: usize| {
                int_value(&pair[i])
                    .ok_or("window")
            };
            Some((at(0)?, at(1)?))
        }
        _ => return Err("window"),
    };
    let from = move_of(key("from")?).ok_or("from")?;
    let to = move_of(key("to")?).ok_or("to")?;
    let (events, presses_only) = match o.get("events") {
        Some(Value::Array(events_in)) => {
            if events_in.len() > STEPS {
                return Err("events");
            }
            if let Some(steps) = o.get("steps") {
                let steps = steps.as_array().ok_or("steps")?;
                if steps.len() > LEGACY_STEPS {
                    return Err("steps");
                }
                for step in steps {
                    parse_step(step.as_object().ok_or("step")?)?;
                }
            }
            let mut events = [None; STEPS];
            for (slot, event) in events.iter_mut().zip(events_in) {
                *slot = Some(parse_event(event.as_object().ok_or("event")?, aims)?);
            }
            (events, false)
        }
        Some(_) => return Err("events"),
        None => {
            let steps_in = o.get("steps").and_then(Value::as_array).ok_or("steps")?;
            if steps_in.len() > LEGACY_STEPS {
                return Err("steps");
            }
            let mut events = [None; STEPS];
            for (slot, step) in events.iter_mut().zip(steps_in) {
                let (button, aim, offset) = parse_step(step.as_object().ok_or("step")?)?;
                *slot = Some(Event::press(button, aim, offset));
            }
            (events, true)
        }
    };
    let drift = match o.get("drift") {
        None | Some(Value::Null) => None,
        Some(Value::String(text)) => Some(dir_of(text).ok_or("drift")?),
        Some(_) => return Err("drift"),
    };
    let from_shown = o.get("from_shown").map(|value| parse_description(value, from, "from_shown")).transpose()?;
    let to_shown = o.get("to_shown").map(|value| parse_description(value, to, "to_shown")).transpose()?;
    let mut rec = LinkRec {
        from,
        to,
        verdict,
        window,
        events,
        presses_only,
        drift,
        spot: o
            .get("spot")
            .and_then(Value::as_str)
            .and_then(Spot::from_key),
        shown: [0; 3],
    };
    if let (Some(from), Some(to)) = (from_shown, to_shown) {
        rec.from = from.mv;
        rec.to = to.mv;
        rec.set_descriptions([from, to]);
    }
    Ok(rec)
}

fn parse_description(value: &Value, mv: Move, key: &'static str) -> Result<ActionDesc, &'static str> {
    let object = value.as_object().ok_or(key)?;
    let exact = match object.get("move") {
        None => mv,
        Some(value) => {
            let found = value.as_str().and_then(move_of).ok_or(key)?;
            if mv != Move::Other || !matches!(found, Move::Grab | Move::Throw | Move::FinalSmash) { return Err(key); }
            found
        }
    };
    let of = match object.get("of") {
        None => None,
        Some(value) => Some(value.as_str().and_then(move_of).ok_or(key)?),
    };
    let angle = match object.get("angle") {
        None => None,
        Some(Value::String(angle)) if angle == "up" => Some(Angle::Up),
        Some(Value::String(angle)) if angle == "down" => Some(Angle::Down),
        Some(_) => return Err(key),
    };
    let normal = match object.get("variant") {
        None => 0,
        Some(value) => value.as_u64().filter(|&normal| normal <= 31).ok_or(key)? as u8,
    };
    Ok(ActionDesc { mv: exact, of, angle, normal, parts: [None; 2] })
}

fn description_value(desc: ActionDesc) -> Value {
    let mut object = Map::new();
    if matches!(desc.mv, Move::Grab | Move::Throw | Move::FinalSmash) {
        object.insert("move".into(), json!(key_of(&MOVES, desc.mv)));
    }
    if let Some(of) = desc.of {
        object.insert("of".into(), json!(key_of(&MOVES, of)));
    }
    if let Some(angle) = desc.angle {
        object.insert("angle".into(), json!(match angle { Angle::Up => "up", Angle::Down => "down" }));
    }
    if desc.normal != 0 {
        object.insert("variant".into(), json!(desc.normal));
    }
    Value::Object(object)
}

fn parse_step(step: &Map<String, Value>) -> Result<(Button, Aim, i32), &'static str> {
    let name = |k: &'static str| step.get(k).and_then(Value::as_str).ok_or(k);
    Ok((
        button_of(name("button")?).ok_or("button")?,
        value_of(&AIMS, name("aim")?).ok_or("aim")?,
        int(step, "offset")?,
    ))
}

fn parse_event(event: &Map<String, Value>, aims: bool) -> Result<Event, &'static str> {
    if event.get("hold").is_some_and(|hold| hold.as_u64().is_none()) { return Err("hold"); }
    let name = |k: &'static str| event.get(k).and_then(Value::as_str).ok_or(k);
    let dir = dir_of(name("dir")?).ok_or("dir")?;
    let offset = int(event, "offset")?;
    let kind = match name("kind")? {
        "press" => Kind::Press(button_of(name("button")?).ok_or("button")?),
        "release" => Kind::Release(button_of(name("button")?).ok_or("button")?),
        "cstick" => Kind::CStick,
        "flick" => Kind::Flick(value_of(&FLICKS, name("flick")?).ok_or("flick")?),
        "aim" if aims => match event.get("part").and_then(Value::as_u64) {
            Some(part @ 1..=2) => Kind::Aim(part as u8),
            _ => return Err("event"),
        },
        _ => return Err("event"),
    };
    Ok(Event { kind, dir, offset })
}

fn parse_travel(event: &Map<String, Value>) -> Result<Travel, &'static str> {
    let raw = event.get("path").and_then(Value::as_array).filter(|path| path.len() == 8).ok_or("path")?;
    let mut path = [(0_i8, 0_i8); 8];
    for (slot, point) in path.iter_mut().zip(raw) {
        let pair = point.as_array().filter(|pair| pair.len() == 2).ok_or("path")?;
        let coordinate = |value: &Value| {
            int_value(value).filter(|value| (-127..=127).contains(value)).map(|value| value as i8).ok_or("path")
        };
        *slot = (coordinate(&pair[0])?, coordinate(&pair[1])?);
    }
    let angle = |value: Option<&Value>, key: &'static str| -> Result<u16, &'static str> {
        let value = value.and_then(Value::as_f64).filter(|value| *value >= 0.0 && *value < 360.0).ok_or(key)?;
        Ok(((value * 10.0).round() as u16) % 3600)
    };
    let heading = angle(event.get("heading"), "heading")?;
    let distance = event.get("distance").and_then(Value::as_f64).filter(|value| *value >= 0.0).ok_or("distance")?;
    let distance = (distance * 10.0).round().clamp(0.0, f64::from(u16::MAX)) as u16;
    let stick = match event.get("stick") {
        None => None,
        Some(value) => {
            let stick = value.as_object().ok_or("stick")?;
            let angle = angle(stick.get("angle"), "stick")?;
            let deflection = stick.get("deflection").and_then(Value::as_f64)
                .filter(|value| *value >= 0.0 && *value <= 100.0).ok_or("stick")?;
            Some((angle, deflection.round() as u8))
        }
    };
    Ok(Travel { stick, heading, distance, path })
}

fn events_value(link: &LinkRec, holds: &[Option<u32>; STEPS], travels: Option<&[Option<Travel>; STEPS]>) -> Vec<Value> {
    link.events.iter().enumerate().filter_map(|(i, event)| {
        event.map(|event| event_value(event, holds[i], travels.and_then(|travels| travels[i])))
    }).collect()
}

fn event_value(event: Event, hold: Option<u32>, travel: Option<Travel>) -> Value {
    let mut value =
        json!({"kind": kind_key(event.kind), "dir": dir_key(event.dir), "offset": event.offset});
    match event.kind {
        Kind::Press(button) | Kind::Release(button) => {
            value["button"] = json!(button_key(button));
            if let (Kind::Release(_), Some(hold)) = (event.kind, hold) {
                value["hold"] = json!(hold);
            }
        }
        Kind::Flick(flick) => value["flick"] = json!(key_of(&FLICKS, flick)),
        Kind::CStick => {}
        Kind::Aim(part) => {
            value["part"] = json!(part);
            if let Some(travel) = travel {
                if let Some((angle, deflection)) = travel.stick {
                    value["stick"] = json!({"angle": f64::from(angle) / 10.0, "deflection": deflection});
                }
                value["heading"] = json!(f64::from(travel.heading) / 10.0);
                value["distance"] = json!(f64::from(travel.distance) / 10.0);
                value["path"] = json!(travel.path);
            }
        }
    }
    value
}

const MOVES: [(Move, &str); 17] = [
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
    (Move::Grab, "grab"),
    (Move::Throw, "throw"),
    (Move::FinalSmash, "final_smash"),
];
const BUTTONS: [(Button, &str); 5] = [
    (Button::Attack, "attack"),
    (Button::Special, "special"),
    (Button::Jump, "jump"),
    (Button::Shield, "shield"),
    (Button::Grab, "grab"),
];
const AIMS: [(Aim, &str); 4] = [
    (Aim::Neutral, "neutral"),
    (Aim::Up, "up"),
    (Aim::Down, "down"),
    (Aim::Side, "side"),
];

fn key_of<T: PartialEq + Copy>(table: &[(T, &'static str)], value: T) -> &'static str {
    table
        .iter()
        .find(|(v, _)| *v == value)
        .map_or("", |(_, k)| k)
}

fn value_of<T: Copy>(table: &[(T, &'static str)], key: &str) -> Option<T> {
    table.iter().find(|(_, k)| *k == key).map(|(v, _)| *v)
}

pub fn move_key(m: Move) -> &'static str {
    key_of(&MOVES, m.legacy())
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

const DIRS: [(Dir, &str); 9] = [
    (Dir::Neutral, "neutral"),
    (Dir::Forward, "forward"),
    (Dir::ForwardUp, "forward_up"),
    (Dir::Up, "up"),
    (Dir::BackUp, "back_up"),
    (Dir::Back, "back"),
    (Dir::BackDown, "back_down"),
    (Dir::Down, "down"),
    (Dir::ForwardDown, "forward_down"),
];
const FLICKS: [(Flick, &str); 3] = [
    (Flick::FastFall, "fast_fall"),
    (Flick::Dash, "dash"),
    (Flick::TapJump, "tap_jump"),
];

fn dir_key(dir: Dir) -> &'static str {
    key_of(&DIRS, dir)
}
fn dir_of(key: &str) -> Option<Dir> {
    value_of(&DIRS, key)
}
fn kind_key(kind: Kind) -> &'static str {
    match kind {
        Kind::Press(_) => "press",
        Kind::Release(_) => "release",
        Kind::CStick => "cstick",
        Kind::Flick(_) => "flick",
        Kind::Aim(_) => "aim",
    }
}

impl Serialize for ComboRecord {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_value().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ComboRecord {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        ComboRecord::from_value(&value)
            .map_err(|field| D::Error::custom(format!("bad combo field: {field}")))
    }
}

impl Record for ComboRecord {
    type Error = ComboError;
    type Summary = ComboEntry;
    const SCHEMA_VERSIONS: &'static [u32] = &[1, 2, 3, 4];
    const MAX_FILE_BYTES: u64 = MAX_FILE_BYTES;

    fn validate(&self) -> Result<(), ComboError> {
        ComboRecord::validate(self)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn summary(&self, id: &str) -> ComboEntry {
        ComboEntry {
            id: id.to_owned(),
            name: self.name.clone(),
            links: self.combo.len(),
            recorded_on: self.recorded_on.clone(),
        }
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
    let file = if first.file.is_empty() {
        "A combo file"
    } else {
        first.file.as_str()
    };
    let mut note = format!("{file} skipped: {}", first.reason);
    if skipped.len() > 1 {
        note.push_str(&format!(" (+{} more)", skipped.len() - 1));
    }
    Some(note)
}

/// "Combo N" with the lowest N no entry uses (SPEC-input-07 §5).
pub fn default_name(entries: &[ComboEntry]) -> String {
    super::store::lowest_free(
        |n| format!("Combo {n}"),
        entries.iter().map(|e| e.name.as_str()),
    )
}

/// A typed name, trimmed; `None` when nothing is left.
pub fn clean_name(typed: &str) -> Option<String> {
    let name: String = typed.trim().chars().take(MAX_NAME_CHARS).collect();
    (!name.is_empty()).then_some(name)
}

/// Saved combos footer. The named library's first line is the focused row's full label
/// (`Name · N links · recorded on X`); the second line is this list's controls.
pub fn saved_combos_footer(library_footer: &str) -> String {
    let label = library_footer.lines().next().unwrap_or("");
    format!("{label}\nA Target · X Rename · Y Delete · B Return")
}

/// Whether opening the list should post, repeat, or clear the skipped-file notice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkipPost {
    Same,
    Post(String),
    Clear,
}

/// The two standing notices for Saved combos. `0x2B` names a file the list could not use.
/// `0x2A` is the action note (the target note, and the other Combos results).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComboNotices {
    posted: Option<String>,
}

impl ComboNotices {
    pub const SKIPPED_ID: u8 = 0x2B;
    pub const ACTION_ID: u8 = 0x2A;

    #[cfg(test)]
    pub fn new() -> Self {
        Self { posted: None }
    }

    /// Opening the list. The skipped-file notice posts only when its text changes, so a later
    /// open of the same set does not reset the 300-frame length.
    pub fn post_skipped(&mut self, skipped: &[SkippedFile]) -> SkipPost {
        match skipped_note(skipped) {
            Some(text) if self.posted.as_deref() == Some(text.as_str()) => SkipPost::Same,
            Some(text) => {
                self.posted = Some(text.clone());
                SkipPost::Post(text)
            }
            None => {
                if self.posted.take().is_some() {
                    SkipPost::Clear
                } else {
                    SkipPost::Same
                }
            }
        }
    }
}

/// An action note (the target note, and the other Combos results) replaces the skipped-file
/// notice. The open menu freezes whichever standing notice is in front, so leaving `0x2B` queued
/// would keep the target note off the screen.
#[cfg(test)]
pub fn apply_combo_action(queue: &mut super::notify::Queue, text: &str) {
    queue.clear_id(ComboNotices::SKIPPED_ID);
    queue.upsert(
        ComboNotices::ACTION_ID,
        "Combos",
        super::notify::Body::copied(text),
        180,
        super::notify::DEFAULT_COLOR,
    );
}

#[cfg(test)]
#[path = "combo_record_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "combo_record_share_tests.rs"]
mod share_tests;

#[cfg(test)]
#[path = "combo_notice_tests.rs"]
mod notice_tests;

fn int_value(value: &Value) -> Option<i32> {
    value.as_i64().and_then(|v| i32::try_from(v).ok())
}
