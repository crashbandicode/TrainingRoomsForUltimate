//! Portable practice setups: the value-only pair of saved fighter states one save-state slot holds, as a
//! named, versioned library file (`sd:/ultimate/TrainingRooms/setups/<id>.json`).
//!
//! Carried from the fork's `training_mod_consts/src/practice_setup.rs` (stage m5o, ours-new;
//! CARRY-OVER.md): `SetupRecord`, `FighterSetup`, their validation, the error type, the listing row and
//! the `Record` implementation over the shared `model::store::Store`. Clean-room changes: hand-written
//! JSON in place of serde derive (the reader is strict about the required fields, tolerant about the
//! optional ones); the value types are this crate's `record::{Charge, Steve}` and `HeldItem`; the Mii
//! identity is an opaque JSON value (package 6 types it), so the check that its class matches the fighter
//! kind is package 6's, while a Mii fighter without an identity is still refused; a record is a plain
//! struct with `to_value` / `from_value`.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use super::held_item::HeldItem;
use super::json::Writer;
use super::record::{charge_from_json, steve_from_json, write_charge, write_steve, Charge, Marker, SavedState, Steve};
use crate::model::store::{Listing, Record, Store, StoreError};

/// Current setup file schema.
pub const SETUP_SCHEMA_VERSION: u32 = 1;
/// Largest setup file read. A compact setup is about 2 KB.
pub const MAX_SETUP_FILE_BYTES: u64 = 64 * 1024;
/// Highest fighter kind a saved state may name: `FIGHTER_KIND_TRAIL` (Sora), the last playable kind in
/// 13.0.x.
pub const MAX_FIGHTER_KIND: i32 = 0x5d;
/// Inclusive damage bound. The game caps displayed damage at 999.9%.
pub const MAX_PERCENT: f32 = 1000.0;
/// Longest setup name, in scalars.
pub const MAX_NAME_CHARS: usize = 48;
/// Longest game-version tag, in bytes.
pub const MAX_GAME_VERSION_BYTES: usize = 32;
/// The three Mii class kinds (`MIIFIGHTER`, `MIISWORDSMAN`, `MIIGUNNER`).
pub const MII_KINDS: std::ops::RangeInclusive<i32> = 0x48..=0x4a;

/// Which half of the pair a value belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Player,
    Cpu,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Player => "Player 1",
            Self::Cpu => "CPU",
        })
    }
}

/// One role's saved values. Nothing here refers to a live object.
#[derive(Clone, Debug, PartialEq)]
pub struct FighterSetup {
    /// Raw game fighter kind, `0..=MAX_FIGHTER_KIND`.
    pub fighter_kind: i32,
    pub x: f32,
    pub y: f32,
    pub percent: f32,
    /// Facing: `1.0` right, `-1.0` left.
    pub lr: f32,
    /// Raw game situation kind (ground/air/cliff), admitted by the runtime.
    pub situation_kind: i32,
    pub charge: Charge,
    pub steve_state: Option<Steve>,
    /// Player 1 only in current saves; the CPU stores `None`.
    pub held_item: Option<HeldItem>,
    /// Exact Mii identity for this role (opaque until package 6).
    pub mii: Option<Value>,
}

fn invalid(role: Role, why: &'static str) -> SetupError {
    SetupError::InvalidFighter { role, why }
}

impl FighterSetup {
    /// The values of a saved record (no execution state).
    pub fn of_saved(saved: &SavedState) -> Self {
        Self {
            fighter_kind: saved.fighter_kind,
            x: saved.x,
            y: saved.y,
            percent: saved.percent,
            lr: saved.lr,
            situation_kind: saved.situation_kind,
            charge: saved.charge,
            steve_state: saved.steve,
            held_item: saved.held_item,
            mii: saved.mii.clone(),
        }
    }

    /// A slot record holding these values, the load machine idle (`NoAction`) and the stage the
    /// setup was saved on.
    pub fn to_saved(&self, stage: Option<i32>) -> SavedState {
        SavedState {
            x: self.x,
            y: self.y,
            percent: self.percent,
            lr: self.lr,
            situation_kind: self.situation_kind,
            marker: Marker::NoAction,
            fighter_kind: self.fighter_kind,
            charge: self.charge,
            steve: self.steve_state,
            held_item: self.held_item,
            saved_stage: stage,
            mii: self.mii.clone(),
        }
    }

    /// Finite, in-range values for one role. Situation-kind membership is the runtime's check.
    pub fn validate(&self, role: Role) -> Result<(), SetupError> {
        if !(0..=MAX_FIGHTER_KIND).contains(&self.fighter_kind) {
            return Err(invalid(role, "fighter kind is outside the roster range"));
        }
        for (value, why) in [
            (self.x, "x position is not finite"),
            (self.y, "y position is not finite"),
            (self.percent, "damage is not finite"),
            (self.lr, "facing is not finite"),
        ] {
            if !value.is_finite() {
                return Err(invalid(role, why));
            }
        }
        if !(0.0..=MAX_PERCENT).contains(&self.percent) {
            return Err(invalid(role, "damage is outside 0 to 1000"));
        }
        if self.lr != 1.0 && self.lr != -1.0 {
            return Err(invalid(role, "facing must be -1 or 1"));
        }
        self.charge.validate().map_err(|why| invalid(role, why))?;
        if let Some(steve) = &self.steve_state {
            steve.validate().map_err(|why| invalid(role, why))?;
        }
        if self.held_item.is_some_and(|item| !item.valid()) {
            return Err(invalid(role, "held item snapshot is not valid"));
        }
        if self.mii.is_none() && MII_KINDS.contains(&self.fighter_kind) {
            return Err(invalid(role, "exact Mii identity missing; save this slot again"));
        }
        Ok(())
    }

    fn write(&self, out: &mut Writer) {
        out.begin_object();
        out.field_int("fighter_kind", i64::from(self.fighter_kind));
        out.field_f32("x", self.x);
        out.field_f32("y", self.y);
        out.field_f32("percent", self.percent);
        out.field_f32("lr", self.lr);
        out.field_int("situation_kind", i64::from(self.situation_kind));
        out.key("charge");
        write_charge(&self.charge, out);
        if let Some(steve) = &self.steve_state {
            out.key("steve_state");
            write_steve(steve, out);
        }
        if let Some(item) = self.held_item {
            out.key("held_item");
            item.write_json(out);
        }
        if let Some(mii) = &self.mii {
            out.key("mii");
            out.value(mii);
        }
        out.end_object();
    }

    fn from_value(value: &Value) -> Result<Self, &'static str> {
        let object = value.as_object().ok_or("a fighter is not an object")?;
        let float = |key: &str| {
            object.get(key).and_then(Value::as_f64).map(|v| v as f32).ok_or("a fighter field is missing or not a number")
        };
        let int = |key: &str| {
            object
                .get(key)
                .and_then(Value::as_i64)
                .and_then(|v| i32::try_from(v).ok())
                .ok_or("a fighter field is missing or not an integer")
        };
        Ok(Self {
            fighter_kind: int("fighter_kind")?,
            x: float("x")?,
            y: float("y")?,
            percent: float("percent")?,
            lr: float("lr")?,
            situation_kind: int("situation_kind")?,
            charge: charge_from_json(object.get("charge").ok_or("a fighter has no charge")?),
            steve_state: object.get("steve_state").and_then(steve_from_json),
            held_item: object.get("held_item").and_then(HeldItem::from_json),
            mii: object.get("mii").filter(|v| !v.is_null()).cloned(),
        })
    }
}

/// A named, versioned, portable practice setup: both roles of one slot.
#[derive(Clone, Debug, PartialEq)]
pub struct SetupRecord {
    pub schema_version: u32,
    pub name: String,
    /// Free-form tag of the game version it was saved on; may be empty.
    pub game_version: String,
    /// Stage the positions were saved on, when the runtime knew it. Advisory: the runtime decides
    /// whether to request that stage or refuse a load.
    pub stage_id: Option<i32>,
    pub player: FighterSetup,
    pub cpu: FighterSetup,
}

impl SetupRecord {
    /// Builds a validated record; the name is trimmed.
    pub fn new(
        name: impl Into<String>,
        game_version: impl Into<String>,
        stage_id: Option<i32>,
        player: FighterSetup,
        cpu: FighterSetup,
    ) -> Result<Self, SetupError> {
        let name: String = name.into();
        let record = Self {
            schema_version: SETUP_SCHEMA_VERSION,
            name: name.trim().to_owned(),
            game_version: game_version.into(),
            stage_id,
            player,
            cpu,
        };
        record.validate()?;
        Ok(record)
    }

    /// Everything the library requires before it writes or accepts a file.
    pub fn validate(&self) -> Result<(), SetupError> {
        if self.schema_version != SETUP_SCHEMA_VERSION {
            return Err(SetupError::Store(StoreError::UnsupportedVersion { found: self.schema_version }));
        }
        check_name(&self.name)?;
        if self.game_version.len() > MAX_GAME_VERSION_BYTES {
            return Err(SetupError::Invalid("game version is too long"));
        }
        if self.game_version.chars().any(char::is_control) {
            return Err(SetupError::Invalid("game version contains control characters"));
        }
        if self.stage_id.is_some_and(|stage| stage < 0) {
            return Err(SetupError::Invalid("stage id is negative"));
        }
        self.player.validate(Role::Player)?;
        self.cpu.validate(Role::Cpu)
    }

    /// Renames the setup (trimmed and validated); the fighters are untouched.
    pub fn set_name(&mut self, name: &str) -> Result<(), SetupError> {
        let name = name.trim();
        check_name(name)?;
        self.name = name.to_owned();
        Ok(())
    }

    pub fn role(&self, role: Role) -> &FighterSetup {
        match role {
            Role::Player => &self.player,
            Role::Cpu => &self.cpu,
        }
    }

    /// The JSON value the store writes.
    pub fn to_value(&self) -> Value {
        let mut out = Writer::new();
        self.write(&mut out);
        serde_json::from_str(&out.finish()).unwrap_or(Value::Null)
    }

    fn write(&self, out: &mut Writer) {
        out.begin_object();
        out.field_int("schema_version", i64::from(self.schema_version));
        out.field_str("name", &self.name);
        out.field_str("game_version", &self.game_version);
        if let Some(stage) = self.stage_id {
            out.field_int("stage_id", i64::from(stage));
        }
        out.key("player");
        self.player.write(out);
        out.key("cpu");
        self.cpu.write(out);
        out.end_object();
    }

    pub fn from_value(value: &Value) -> Result<Self, &'static str> {
        let object: &Map<String, Value> = value.as_object().ok_or("not an object")?;
        let version = object
            .get("schema_version")
            .and_then(Value::as_u64)
            .and_then(|v| u32::try_from(v).ok())
            .ok_or("no schema_version")?;
        Ok(Self {
            schema_version: version,
            name: object.get("name").and_then(Value::as_str).ok_or("no name")?.to_owned(),
            game_version: object.get("game_version").and_then(Value::as_str).ok_or("no game_version")?.to_owned(),
            stage_id: object.get("stage_id").and_then(Value::as_i64).and_then(|v| i32::try_from(v).ok()),
            player: FighterSetup::from_value(object.get("player").ok_or("no player")?)?,
            cpu: FighterSetup::from_value(object.get("cpu").ok_or("no cpu")?)?,
        })
    }
}

impl Serialize for SetupRecord {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to_value().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SetupRecord {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        Self::from_value(&value).map_err(serde::de::Error::custom)
    }
}

fn check_name(name: &str) -> Result<(), SetupError> {
    if name.trim().is_empty() {
        return Err(SetupError::Invalid("name is empty"));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(SetupError::Invalid("name is too long"));
    }
    if name.chars().any(char::is_control) {
        return Err(SetupError::Invalid("name contains control characters"));
    }
    Ok(())
}

/// Listing row for one stored setup; enough to pick, label or refuse it without reading the payload
/// again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupEntry {
    pub id: String,
    pub name: String,
    pub game_version: String,
    pub player_kind: i32,
    pub cpu_kind: i32,
    pub stage_id: Option<i32>,
    /// Whether the player and CPU halves carry an exact Mii configuration.
    pub exact_mii: [bool; 2],
}

/// Every failure the setup library reports.
#[derive(Debug)]
pub enum SetupError {
    /// Shared store failure: I/O, id, size, parse or version.
    Store(StoreError),
    /// A record-level field (schema, name, version, stage) is unacceptable.
    Invalid(&'static str),
    /// One role's saved values are unacceptable.
    InvalidFighter { role: Role, why: &'static str },
}

impl fmt::Display for SetupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(StoreError::UnsupportedVersion { found }) => {
                write!(f, "unsupported setup schema version {found} (supported: {SETUP_SCHEMA_VERSION})")
            }
            Self::Store(StoreError::Parse(err)) => write!(f, "malformed setup file: {err}"),
            Self::Store(StoreError::InvalidId(id)) => write!(f, "invalid setup id {id:?}"),
            Self::Store(StoreError::NotFound(id)) => write!(f, "no setup with id {id:?}"),
            Self::Store(StoreError::NoFreeId) => f.write_str("no free setup id for this name"),
            Self::Store(err) => write!(f, "{err}"),
            Self::Invalid(why) => write!(f, "invalid setup: {why}"),
            Self::InvalidFighter { role, why } => write!(f, "invalid setup: {role} {why}"),
        }
    }
}

impl std::error::Error for SetupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Store(err) => Some(err),
            _ => None,
        }
    }
}

impl From<StoreError> for SetupError {
    fn from(err: StoreError) -> Self {
        Self::Store(err)
    }
}

impl Record for SetupRecord {
    type Error = SetupError;
    type Summary = SetupEntry;
    const SCHEMA_VERSIONS: &'static [u32] = &[SETUP_SCHEMA_VERSION];
    const MAX_FILE_BYTES: u64 = MAX_SETUP_FILE_BYTES;

    fn validate(&self) -> Result<(), SetupError> {
        SetupRecord::validate(self)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn summary(&self, id: &str) -> SetupEntry {
        SetupEntry {
            id: id.to_owned(),
            name: self.name.clone(),
            game_version: self.game_version.clone(),
            player_kind: self.player.fighter_kind,
            cpu_kind: self.cpu.fighter_kind,
            stage_id: self.stage_id,
            exact_mii: [self.player.mii.is_some(), self.cpu.mii.is_some()],
        }
    }
}

/// The setup library, normally rooted at `<storage root>/setups`.
pub type SetupLibrary = Store<SetupRecord>;
/// A scan of the setup library.
pub type SetupListing = Listing<SetupEntry>;
