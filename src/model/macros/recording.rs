//! Portable macro data model: per-frame mapped controls, the start context a
//! macro assumes, and a relative placement that is edited independently of the
//! frames.

use std::{fmt, io};

use serde::{Deserialize, Serialize};

/// Ordinary recordings retain the original schema and serialized shape.
pub const SCHEMA_VERSION: u32 = 1;
/// Exact Mii macros must refuse on older readers that would discard their identity.
pub const MII_SCHEMA_VERSION: u32 = 2;
/// Gameplay frame rate assumed when the caller does not supply one.
pub const DEFAULT_FPS: u32 = 60;
/// Longest macro accepted: 10 minutes of gameplay at 60 fps.
///
/// A CPU reaction drill lasts seconds and the legacy recorder holds 600 frames,
/// so this is far beyond any practical replay window. It bounds a macro to
/// 288 KiB in memory (8 bytes per frame) and about 3 MiB of compact JSON
/// (87 bytes per worst-case frame), which a console parses comfortably.
pub const MAX_FRAMES: usize = 36_000;
/// Longest display name, counted in Unicode scalar values.
pub const MAX_NAME_CHARS: usize = 48;
/// Longest free-form game version tag, in bytes.
pub const MAX_GAME_VERSION_BYTES: usize = 32;

pub use crate::model::trim::InputFrame;

/// Fighter context a macro assumes at its first frame.
///
/// `situation` and `status` are raw game enum values captured by the runtime;
/// this portable crate does not know them, and 0 means "not captured" exactly as
/// in the save-state default. A macro is not an arbitrary full-game save state.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StartState {
    pub fighter_kind: i32,
    pub costume: u32,
    /// Sign gives the facing direction (1.0 = right, -1.0 = left).
    pub facing: f32,
    pub situation: i32,
    pub status: i32,
    pub speed_x: f32,
    pub speed_y: f32,
}

impl Default for StartState {
    /// Mirrors the save-state default: no fighter, facing right, at rest.
    fn default() -> Self {
        Self {
            fighter_kind: -1,
            costume: 0,
            facing: 1.0,
            situation: 0,
            status: 0,
            speed_x: 0.0,
            speed_y: 0.0,
        }
    }
}

impl StartState {
    /// Neutral context (no speed, uncaptured situation/status) for a known fighter.
    pub fn neutral(fighter_kind: i32, costume: u32, facing: f32) -> Self {
        Self {
            fighter_kind,
            costume,
            facing,
            ..Self::default()
        }
    }

    fn validate(&self) -> Result<(), MacroError> {
        if self.fighter_kind < 0 {
            return Err(MacroError::MissingFighter);
        }
        finite(self.facing, "start.facing")?;
        finite(self.speed_x, "start.speed_x")?;
        finite(self.speed_y, "start.speed_y")
    }
}

/// What the runtime does with the recorded momentum when the macro starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MomentumMode {
    /// Start with the `StartState` speeds (rising/falling momentum kept).
    #[default]
    PreserveRecorded,
    /// Start at rest; ordinary physics applies afterwards (not a permanent hover).
    AtRest,
}

/// Start position relative to the CPU's restored position, in world units.
///
/// The offset is always applied to the freshly restored anchor, never to a
/// previous result, so repeated reset/play trials cannot accumulate it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub delta_x: f32,
    pub delta_y: f32,
    pub momentum: MomentumMode,
}

impl Placement {
    pub fn validate(&self) -> Result<(), MacroError> {
        finite(self.delta_x, "placement.delta_x")?;
        finite(self.delta_y, "placement.delta_y")
    }

    /// Target position for the given restored anchor position.
    pub fn apply(&self, anchor: (f32, f32)) -> (f32, f32) {
        (anchor.0 + self.delta_x, anchor.1 + self.delta_y)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ActionMark {
    pub start: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<u32>,
    #[serde(rename = "move", default, skip_serializing_if = "Option::is_none")]
    pub mv: Option<String>,
    /// The move's variant as a combo link's description stores it (`move_names::Normal`); 0 is none and is not written.
    #[serde(default, skip_serializing_if = "is_no_variant")]
    pub variant: u8,
    /// A side tilt's or side smash's angle, `up` or `down`, as a combo link's description stores it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle: Option<String>,
}

fn is_no_variant(variant: &u8) -> bool {
    *variant == 0
}

/// A named, versioned, portable CPU macro.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MacroRecording {
    pub schema_version: u32,
    pub name: String,
    /// Free-form tag of the game version it was recorded on; may be empty.
    pub game_version: String,
    pub start: StartState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mii: Option<crate::engine::mii::lifetime::Configuration>,
    pub placement: Placement,
    pub frames: Vec<InputFrame>,
    #[serde(default, skip_serializing_if = "crate::model::trim::Trim::is_off")]
    pub trim: crate::model::trim::Trim,
    #[serde(default, deserialize_with = "lenient_actions", skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ActionMark>,
    /// The fighter the take was recorded on, as a combo file's `recorded_fighter` stores it: its specials are named by
    /// it whoever Player 1 is. Absent in a take saved before the key existed.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "crate::model::combo_record::fighter_key")]
    pub recorded_fighter: Option<crate::model::move_names::FighterId>,
    /// Save id this macro starts from. Absent means the file never had one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_save: Option<String>,
    /// With an id, On warps once and Off keeps the id. Default On. Omitted from JSON when On.
    #[serde(default = "start_save_switch_on", skip_serializing_if = "is_switch_on")]
    pub start_save_on: bool,
    /// Where both fighters were on each frame, as a match capture keeps it (`ReplayFrame` with no input): the
    /// recorded fighter first, then the other one. One entry per frame from the first; shorter than the frames when
    /// recording ran out of memory for it. It is never in the macro's own file: the library keeps it beside it
    /// (`positions`), and only the clip editor's load reads it back.
    #[serde(skip)]
    pub track: Option<Vec<[super::ReplayFrame; 2]>>,
}

impl MacroRecording {
    /// Builds a validated recording with the default (zero, preserve) placement.
    pub fn new(
        name: impl Into<String>,
        game_version: impl Into<String>,
        start: StartState,
        frames: Vec<InputFrame>,
    ) -> Result<Self, MacroError> {
        let name: String = name.into();
        let recording = Self {
            schema_version: SCHEMA_VERSION,
            name: name.trim().to_owned(),
            game_version: game_version.into(),
            start,
            mii: None,
            placement: Placement::default(),
            frames,
            trim: crate::model::trim::Trim::Off,
            actions: Vec::new(),
            recorded_fighter: None,
            start_save: None,
            start_save_on: true,
            track: None,
        };
        recording.validate()?;
        Ok(recording)
    }

    /// Checks everything `Library` requires before it writes or accepts a file.
    pub fn validate(&self) -> Result<(), MacroError> {
        self.validate_header()?;
        // The positions a take carries: an entry per frame from the first, as far as they go (not at all, for a crop
        // past their end), every number finite.
        let fits = self.track.as_ref().is_none_or(|track| track.len() <= self.frames.len()
            && track.iter().flatten().all(|frame| [frame.x, frame.y, frame.start.facing, frame.start.speed_x, frame.start.speed_y].iter().all(|value| value.is_finite())));
        if !fits {
            return Err(MacroError::InvalidTrack);
        }
        self.validate_frame_count()
    }

    pub(super) fn validate_header(&self) -> Result<(), MacroError> {
        if !matches!(self.schema_version, SCHEMA_VERSION | MII_SCHEMA_VERSION) {
            return Err(MacroError::UnsupportedVersion {
                found: self.schema_version,
            });
        }
        check_name(&self.name)?;
        if self.game_version.len() > MAX_GAME_VERSION_BYTES {
            return Err(MacroError::InvalidGameVersion("is too long"));
        }
        if self.game_version.chars().any(char::is_control) {
            return Err(MacroError::InvalidGameVersion(
                "contains control characters",
            ));
        }
        self.start.validate()?;
        match (self.schema_version, self.mii) {
            (SCHEMA_VERSION, None) => {},
            (MII_SCHEMA_VERSION, Some(configuration))
                if configuration.canonical().class.kind() == self.start.fighter_kind as u32 => {},
            _ => return Err(MacroError::InvalidMii("schema, fighter class and exact configuration disagree")),
        }
        self.placement.validate()
    }

    pub(super) fn validate_frame_count(&self) -> Result<(), MacroError> {
        if self.frames.is_empty() {
            return Err(MacroError::EmptyFrames);
        }
        if self.frames.len() > MAX_FRAMES {
            return Err(MacroError::TooManyFrames {
                frames: self.frames.len(),
                max: MAX_FRAMES,
            });
        }
        Ok(())
    }

    /// Attach owned identity after qualified replay resolution. Ordinary files
    /// retain schema1 and their existing JSON representation.
    pub fn set_mii(&mut self, configuration: crate::engine::mii::lifetime::Configuration) -> Result<(), MacroError> {
        if configuration.canonical().class.kind() != self.start.fighter_kind as u32 {
            return Err(MacroError::InvalidMii("configuration belongs to a different fighter class"));
        }
        self.mii = Some(configuration);
        self.schema_version = MII_SCHEMA_VERSION;
        Ok(())
    }

    /// Attach the positions a take recorded (one entry per frame from the first), or drop them.
    pub fn set_track(&mut self, track: Option<Vec<[super::ReplayFrame; 2]>>) {
        self.track = track;
    }

    /// Keep frames `range` of the recording, with the same slice of its track.
    pub fn keep(&mut self, range: std::ops::Range<usize>) {
        self.frames.truncate(range.end);
        self.frames.drain(..range.start);
        if let Some(track) = &mut self.track {
            track.truncate(range.end);
            track.drain(..range.start.min(track.len()));
        }
    }

    /// Legacy Mii files remain readable; no preferred Mii can stand in for their
    /// absent identity. The caller still needs native exact-target completion.
    pub fn playback_target(&self) -> Result<crate::engine::mii::replay::Target, MacroError> {
        self.validate()?;
        if let Some(configuration) = self.mii {
            Ok(crate::engine::mii::replay::Target::mii(configuration))
        } else if crate::engine::mii::inventory::MiiClass::from_kind(self.start.fighter_kind as u32).is_some() {
            Err(MacroError::InvalidMii("this older macro has no exact Mii configuration"))
        } else {
            Ok(crate::engine::mii::replay::Target::fighter(self.start.fighter_kind as u32))
        }
    }

    /// Replaces the placement metadata only; the frames are never touched.
    pub fn set_placement(&mut self, placement: Placement) -> Result<(), MacroError> {
        placement.validate()?;
        self.placement = placement;
        Ok(())
    }

    /// Renames the macro (trimmed and validated); the frames are never touched.
    #[cfg(test)]
    pub fn set_name(&mut self, name: &str) -> Result<(), MacroError> {
        let name = name.trim();
        check_name(name)?;
        self.name = name.to_owned();
        Ok(())
    }

    /// Stores a start-save id and turns the switch On. `None` clears the id.
    pub fn set_start_save(&mut self, id: Option<&str>) -> Result<(), MacroError> {
        match id {
            None => {
                self.start_save = None;
                self.start_save_on = true;
            }
            Some(id) => {
                if !crate::model::store::default_id(id) {
                    return Err(MacroError::InvalidId(id.to_owned()));
                }
                self.start_save = Some(id.to_owned());
                self.start_save_on = true;
            }
        }
        Ok(())
    }

    /// Off keeps the id. No id means there is no switch to flip.
    #[cfg(test)]
    pub fn set_start_save_on(&mut self, on: bool) {
        if self.start_save.is_some() {
            self.start_save_on = on;
        }
    }
}

/// The marks are optional detail: a key of the wrong shape reads as no marks, and the macro still loads.
fn lenient_actions<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<ActionMark>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or_default())
}

fn start_save_switch_on() -> bool {
    true
}

fn is_switch_on(on: &bool) -> bool {
    *on
}

fn check_name(name: &str) -> Result<(), MacroError> {
    if name.trim().is_empty() {
        return Err(MacroError::InvalidName("is empty"));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(MacroError::InvalidName("is too long"));
    }
    if name.chars().any(char::is_control) {
        return Err(MacroError::InvalidName("contains control characters"));
    }
    Ok(())
}

fn finite(value: f32, field: &'static str) -> Result<(), MacroError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(MacroError::NonFinite(field))
    }
}

/// Every failure this crate reports.
#[derive(Debug)]
pub enum MacroError {
    UnsupportedVersion {
        found: u32,
    },
    /// A take's positions are longer than its frames, or hold a number that is not finite.
    InvalidTrack,
    InvalidName(&'static str),
    InvalidGameVersion(&'static str),
    InvalidMii(&'static str),
    /// `start.fighter_kind` is negative, i.e. the macro names no fighter.
    MissingFighter,
    NonFinite(&'static str),
    EmptyFrames,
    TooManyFrames {
        frames: usize,
        max: usize,
    },
    FileTooLarge {
        bytes: u64,
        max: u64,
    },
    InvalidId(String),
    NotFound(String),
    /// Every candidate id derived from the name is already taken.
    NoFreeId,
    InvalidWindow(&'static str),
    Parse(serde_json::Error),
    Io(io::Error),
}

impl fmt::Display for MacroError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion { found } => write!(
                f,
                "unsupported schema version {found} (supported: {SCHEMA_VERSION} and {MII_SCHEMA_VERSION})"
            ),
            Self::InvalidTrack => f.write_str("the positions do not match the frames"),
            Self::InvalidName(why) => write!(f, "invalid name: {why}"),
            Self::InvalidGameVersion(why) => write!(f, "invalid game version: {why}"),
            Self::InvalidMii(why) => write!(f, "Mii macro unavailable: {why}"),
            Self::MissingFighter => write!(f, "start state names no fighter"),
            Self::NonFinite(field) => write!(f, "{field} is not a finite number"),
            Self::EmptyFrames => write!(f, "macro has no frames"),
            Self::TooManyFrames { frames, max } => {
                write!(f, "macro has {frames} frames (maximum {max})")
            }
            Self::FileTooLarge { bytes, max } => {
                write!(f, "file is {bytes} bytes (maximum {max})")
            }
            Self::InvalidId(id) => write!(f, "invalid macro id {id:?}"),
            Self::NotFound(id) => write!(f, "no macro with id {id:?}"),
            Self::NoFreeId => write!(f, "no free macro id for this name"),
            Self::InvalidWindow(why) => write!(f, "invalid window: {why}"),
            Self::Parse(err) => write!(f, "malformed macro file: {err}"),
            Self::Io(err) => write!(f, "i/o error: {err}"),
        }
    }
}

impl std::error::Error for MacroError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(err) => Some(err),
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for MacroError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for MacroError {
    fn from(err: serde_json::Error) -> Self {
        Self::Parse(err)
    }
}

#[cfg(test)]
mod tests;
