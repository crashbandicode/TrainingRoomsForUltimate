//! Host-testable named-macro panels and restore/playback sequencing.
//!
//! The Switch runtime draws [`View`] on the existing 8×4 training menu and
//! performs disk, character-load, and controller I/O. This module does not
//! touch game pointers or a replay browser.

use {crate::engine::macros::InputFrame, crate::engine::macros::MacroError, crate::engine::macros::MacroRecording, crate::engine::macros::MomentumMode, crate::engine::macros::Placement, crate::engine::macros::StartState, crate::engine::macros::MAX_NAME_CHARS};

use {crate::engine::compat::grid::NX_SUBMENU_COLUMNS, crate::engine::compat::grid::NX_SUBMENU_ROWS};

mod fighter;
pub use fighter::{gate as fighter_gate, Choice as FighterChoice, Gate as FighterGate, Origin as FighterOrigin};
use crate::model::recording_menu::ViewRole;

const PAGE: usize = NX_SUBMENU_ROWS * NX_SUBMENU_COLUMNS;
/// Matches the legacy recorder's flick-jump bit. Other buttons count as input.
const FLICK_JUMP: u32 = 0x8000;
const STICK_NEUTRAL: i8 = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pad {
    Left,
    Right,
    Up,
    Down,
    Confirm,
    Back,
    /// Y: backspace while naming.
    Extra,
    /// X: rename the row of a caller-titled library (SPEC-input-07 §4); nothing elsewhere.
    Rename,
}

/// Native keyboard header for a numbered or named CPU recording.
pub const MACRO_KEYBOARD_TITLE: &str = "Name macro";
/// Native keyboard header for a setup draft. Callers may pass another literal.
pub const SETUP_KEYBOARD_TITLE: &str = "Name setup";
/// The combo helper's names (SPEC-input-07 §5).
pub const COMBO_KEYBOARD_TITLE: &str = "Name combo";

/// Caller-owned naming session. The TUI stores the token and keyboard title, not the payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NameContext {
    pub token: u64,
    pub title: &'static str,
}

/// Whether a recording may be saved as schema 1, with an exact Mii, or not at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MiiIdentity {
    /// Fighter is not a Mii. Schema 1 is the right file.
    Ordinary,
    /// Exact configuration from record start, and its class matches the fighter.
    Exact,
    /// Mii fighter whose exact configuration was missing or belonged to another class.
    Unknown,
}

/// `observed_kind` is `configuration.canonical().class.kind()` when a configuration
/// was actually read. The source (saved, original, or discovered) is not a separate case.
pub fn mii_identity(fighter_kind: i32, observed_kind: Option<u32>) -> MiiIdentity {
    let Some(class) = crate::engine::mii::inventory::MiiClass::from_kind(fighter_kind as u32) else {
        return MiiIdentity::Ordinary;
    };
    match observed_kind {
        Some(kind) if kind == class.kind() => MiiIdentity::Exact,
        _ => MiiIdentity::Unknown,
    }
}

pub fn export_mii_refusal(identity: MiiIdentity) -> Option<&'static str> {
    match identity {
        MiiIdentity::Unknown => Some(
            "Exact Mii was not available when recording started. Record this slot again.",
        ),
        MiiIdentity::Ordinary | MiiIdentity::Exact => None,
    }
}

/// Numbered CPU slots hold this many frames. Longer named files stay playable.
pub const NUMBERED_SLOT_FRAMES: usize = 600;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotAdmit {
    Ready,
    Empty,
    OverLimit { frames: usize, limit: usize },
}

/// Full copy, or the original length left untouched. Never a shortened vec.
pub fn copy_numbered_frames(frames: &[InputFrame]) -> Result<Vec<InputFrame>, SlotAdmit> {
    match admit_numbered(frames.len()) {
        SlotAdmit::Ready => Ok(frames.to_vec()),
        other => Err(other),
    }
}

pub fn admit_numbered(frame_count: usize) -> SlotAdmit {
    if frame_count == 0 {
        SlotAdmit::Empty
    } else if frame_count > NUMBERED_SLOT_FRAMES {
        SlotAdmit::OverLimit { frames: frame_count, limit: NUMBERED_SLOT_FRAMES }
    } else {
        SlotAdmit::Ready
    }
}

/// `slot` is the captured facing/status. `None` keeps the shared recorder value
/// and does not park it. A captured slot parks the shared pair so playback can
/// restore it instead of leaving the imported facing behind.
pub fn engage_slot_origin(
    shared: (f32, i32),
    parked: Option<(f32, i32)>,
    slot: Option<(f32, i32)>,
) -> ((f32, i32), Option<(f32, i32)>) {
    match slot {
        Some(origin) => (origin, parked.or(Some(shared))),
        None => (shared, parked),
    }
}

pub fn release_slot_origin(shared: (f32, i32), parked: Option<(f32, i32)>) -> ((f32, i32), Option<(f32, i32)>) {
    match parked {
        Some(saved) => (saved, None),
        None => (shared, None),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    None,
    Notify(String),
    Save(Box<MacroRecording>),
    Overwrite {
        id: String,
        recording: Box<MacroRecording>,
    },
    Choose(String),
    /// Y on the named CPU library. A remains [`Effect::Choose`].
    BindToSlot { id: String },
    /// Y on a caller-titled library. The runtime owns the meaning of the row.
    Alternate { id: String },
    /// X on a caller-titled library: the runtime renames the row.
    Rename { id: String },
    /// A on a caller-titled library opened with [`MacroUi::open_library_forward`].
    /// The runtime owns the row; the named-macro loader never sees it.
    ForwardChoose { id: String },
    /// Keyboard confirmed for a payload the runtime still owns.
    NameConfirmed { context: NameContext, name: String },
    NeedCharacter(i32),
    /// Result of a prompt opened for a replay clip or track. `None` is Cancel.
    /// A named-library prompt is applied inside [`MacroUi`] and never reports here.
    ClipFighter(Option<FighterChoice>),
    /// Result of a prompt opened for a chosen combo (Player 1's fighter). `None` is Cancel.
    ComboFighter(Option<FighterChoice>),
    /// Result of a prompt opened for a recording, a combo drill, or a chain's macro 1. `None` is Cancel.
    RecordingFighter(Option<FighterChoice>),
    /// Keep on Edit placement for a role's active macro: Play uses this offset until the game closes; the file is
    /// unchanged.
    KeepPlacement { id: String, placement: Placement },
}

/// Everything the runtime knows about the selection being loaded. The panel
/// never reads native state; `matches` comes from the exact readiness predicate.
#[derive(Clone, Debug)]
pub struct LoadContext {
    /// Live CPU kind, if a CPU exists.
    pub cpu_kind: Option<i32>,
    /// The recorded identity is already the settled CPU (same class with a
    /// different exact Mii is not a match).
    pub matches: bool,
    /// Persistent Fighter Prompt setting. False keeps the current CPU silently.
    pub prompt_enabled: bool,
    /// This build can load the recorded identity (an exact Mii needs the Mii
    /// alpha). False offers nothing to switch to, so the current CPU is kept.
    pub can_switch: bool,
    /// Selection revision the runtime will compare before applying the choice.
    pub revision: u64,
    pub recorded_label: String,
    pub current_label: String,
}

impl LoadContext {
    /// The gate for `recording`. A Switch needs an exact recorded target that this
    /// build can load; an older Mii file without its exact Mii has none.
    pub fn gate(&self, recording: &MacroRecording) -> FighterGate {
        let switchable = self.can_switch && recording.playback_target().is_ok();
        fighter::gate(self.prompt_enabled, switchable, self.matches)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryRow {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub title: String,
    pub footer: String,
    pub labels: Vec<String>,
    pub selected: usize,
}

#[derive(Clone, Copy, Debug)]
enum Key {
    Char(char),
    Backspace,
    Clear,
    Next,
    Prev,
    Save,
    Cancel,
    Place,
    Momentum,
    Nudge(i16, i16),
    Keep,
    Write,
    SaveAs,
    Back,
    Empty,
}

/// Which CPU a loaded macro's inputs are for. The file itself is never changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Plays {
    /// The recorded fighter (and exact Mii) must be the settled CPU.
    Recorded,
    /// An explicit Switch choice waits once for its requested fighter, then
    /// inputs follow the current CPU; later manual swaps are never undone.
    SwitchOnce,
    /// Whatever CPU is settled on stage when playback starts.
    Current,
}

struct Loaded {
    id: Option<String>,
    recording: MacroRecording,
    plays: Plays,
}

/// The role's active macro on Edit placement, and where its start save puts that role.
struct Editing {
    id: String,
    recording: MacroRecording,
}

/// A selection waiting for the user's fighter choice. Nothing is committed yet,
/// so Cancel leaves the previously loaded macro and the CPU untouched.
struct FighterScreen {
    prompt: fighter::Prompt,
    /// Live CPU kind when the prompt opened, for Keep's readiness.
    cpu_kind: Option<i32>,
    /// Named library only. A clip prompt keeps its candidate in the editor.
    candidate: Option<(String, MacroRecording)>,
    /// Library page to return to on Cancel.
    prior: Option<Box<LibraryPage>>,
}

// One allocation owns the large macro payload; the small name/caret state stays
// inline, including while parked. External naming needs no payload allocation.
struct MacroNameBody {
    frames: Vec<InputFrame>,
    start: StartState,
    placement: Placement,
    version: String,
    mii: Option<crate::engine::mii::lifetime::Configuration>,
}

enum NameBody {
    Macro(Box<MacroNameBody>),
    /// Name and caret only. The setup or other draft stays with the caller.
    External { token: u64, summary: String, title: &'static str },
}

struct NameDraft {
    name: String,
    /// Native caret in UTF-16 units; independent of the terminal grid selection.
    native_cursor: usize,
    page: usize,
    cursor: usize,
    body: NameBody,
    /// Header passed to the system keyboard, including after a draft is resumed.
    keyboard_title: &'static str,
}

struct PlaceDraft {
    working: Placement,
    cursor: usize,
    /// Return to the save dialog instead of closing.
    from_save: bool,
}

struct LibraryPage {
    rows: Vec<LibraryRow>,
    skipped: usize,
    cursor: usize,
    title: String,
    /// Y label. Absent means Y does nothing.
    alternate: Option<String>,
    /// True copies into a numbered CPU slot. False reports [`Effect::Alternate`].
    binds_slot: bool,
}

enum Screen {
    Closed,
    Save(NameDraft),
    Place(Box<PlaceDraft>),
    Library(Box<LibraryPage>),
    Fighter(Box<FighterScreen>),
}

/// Menu, naming, placement, and the restore/character sequence for one macro.
pub struct MacroUi {
    screen: Screen,
    loaded: Option<Loaded>,
    /// Edit placement's file while its page is open (primary 2026-11-08 #70 (a)); the named selection is not changed.
    editing: Option<Editing>,
    /// Library page the last A choice came from, for a Cancelled fighter prompt.
    picked: Option<Box<LibraryPage>>,
    /// Id and name of a library load a fighter prompt just committed, until the
    /// runtime has remembered it.
    prompt_commit: Option<(String, String)>,
    /// Name dialog parked while its placement page is open.
    held_save: Option<NameDraft>,
    /// CPU position from the latest save-state restore, before the offset.
    anchor: Option<(f32, f32)>,
    anchor_generation: Option<u64>,
    edit_anchor: Option<(f32, f32)>,
    pending_kind: Option<i32>,
    claimed: bool,
    character_ready: bool,
    playback_on: bool,
    playback_frame: usize,
    playback_len: usize,
    emitted: Option<InputFrame>,
    emitted_tick: Option<u64>,
}

impl MacroUi {
    pub const fn new() -> Self {
        Self {
            screen: Screen::Closed,
            loaded: None,
            editing: None,
            picked: None,
            prompt_commit: None,
            held_save: None,
            anchor: None,
            anchor_generation: None,
            edit_anchor: None,
            pending_kind: None,
            claimed: false,
            character_ready: false,
            playback_on: false,
            playback_frame: 0,
            playback_len: 0,
            emitted: None,
            emitted_tick: None,
        }
    }

    pub fn is_open(&self) -> bool {
        !matches!(self.screen, Screen::Closed)
    }

    pub fn playback_allowed(&self) -> bool {
        self.character_ready && !self.is_open() && self.loaded.is_some()
    }

    pub fn loaded_id(&self) -> Option<&str> {
        self.loaded.as_ref().and_then(|loaded| loaded.id.as_deref())
    }

    pub fn loaded_frames(&self) -> Option<&[InputFrame]> {
        self.loaded
            .as_ref()
            .map(|loaded| loaded.recording.frames.as_slice())
    }

    pub fn loaded_facing(&self) -> Option<f32> {
        self.loaded
            .as_ref()
            .map(|loaded| loaded.recording.start.facing)
    }

    pub fn loaded_start(&self) -> Option<StartState> {
        self.loaded.as_ref().map(|loaded| loaded.recording.start)
    }

    pub fn loaded_target(&self) -> Option<crate::engine::mii::replay::Target> {
        self.loaded.as_ref()?.recording.playback_target().ok()
    }

    /// Invalidate only unsubmitted intent/readiness; the native adapter owns an
    /// installation already in flight. Used when switching away to a clip.
    pub fn invalidate_character_request(&mut self) {
        self.character_ready = false;
        self.claimed = false;
        // A macro playing on the current CPU never requests a character.
        self.pending_kind = if self.plays_current() {
            None
        } else {
            self.loaded_target().map(|target| target.kind() as i32)
        };
    }

    pub fn loaded_placement(&self) -> Option<Placement> {
        self.loaded
            .as_ref()
            .map(|loaded| loaded.recording.placement)
    }

    /// Native frontends supply their system keyboard instead of the terminal grid.
    pub fn name_prompt(&self) -> Option<&str> {
        match &self.screen { Screen::Save(draft) => Some(&draft.name), _ => None }
    }

    pub fn native_name_cursor(&self) -> Option<usize> {
        match &self.screen { Screen::Save(draft) => Some(draft.native_cursor), _ => None }
    }

    /// Header for the open name draft. Unchanged by caret checkpoints and practice invalidation.
    pub fn native_name_title(&self) -> Option<&'static str> {
        match &self.screen {
            Screen::Save(draft) => Some(draft.keyboard_title),
            _ => None,
        }
    }

    /// Token of the caller-owned name draft on screen, open or parked.
    pub fn external_name_token(&self) -> Option<u64> {
        match &self.screen {
            Screen::Save(draft) => match draft.body {
                NameBody::External { token, .. } => Some(token),
                NameBody::Macro(_) => None,
            },
            _ => None,
        }
    }

    /// Checkpoint an unfinished native edit, without confirmation or any I/O
    /// effect. Empty/whitespace drafts are valid; final save validation is separate.
    /// Rejected text/caret leaves the entire existing draft untouched.
    pub fn checkpoint_native_name(&mut self, name: &str, cursor_utf16: usize) -> bool {
        let Screen::Save(draft) = &mut self.screen else { return false };
        if name.chars().count() > MAX_NAME_CHARS || name.chars().any(char::is_control) {
            return false;
        }
        let units: Vec<u16> = name.encode_utf16().collect();
        if cursor_utf16 > units.len() || String::from_utf16(&units[..cursor_utf16]).is_err() {
            return false;
        }
        draft.name = name.to_owned();
        draft.native_cursor = cursor_utf16;
        true
    }

    pub fn native_name_view(&self) -> Option<View> {
        let Screen::Save(draft) = &self.screen else { return None };
        let footer = match &draft.body {
            NameBody::Macro(body) => format!(
                "{} · {}f · {:.2}s\nA: system keyboard   Y: placement   B: return",
                shown_name(&draft.name), body.frames.len(), body.frames.len() as f32 / 60.0),
            NameBody::External { summary, .. } => format!(
                "{} · {summary}\nA: system keyboard   B: return",
                shown_name(&draft.name)),
        };
        Some(View { title: draft.keyboard_title.into(), selected: 0,
            labels: vec!["Name and save".into()], footer })
    }

    /// Validation failure leaves the full draft available for correction/retry.
    pub fn confirm_native_name(&mut self, name: String) -> Effect {
        let Screen::Save(draft) = &mut self.screen else { return Effect::None };
        draft.native_cursor = name.encode_utf16().count();
        draft.name = name;
        self.finish_save()
    }

    pub fn native_name_placement(&mut self) -> Effect { self.open_place_from_save() }

    pub fn view(&self) -> Option<View> {
        let mut labels = vec![String::new(); PAGE];
        let (title, footer, selected) = match &self.screen {
            Screen::Closed => return None,
            Screen::Fighter(screen) => return Some(screen.prompt.view()),
            Screen::Save(draft) => {
                fill_keys(&mut labels, name_keys(draft.page));
                let detail = match &draft.body {
                    NameBody::Macro(body) => format!(
                        "Fighter {} · {}f · {:.2}s. A confirms, B cancels, Y deletes.",
                        body.start.fighter_kind, body.frames.len(), body.frames.len() as f32 / 60.0),
                    NameBody::External { summary, .. } => format!(
                        "{summary}. A confirms, B cancels, Y deletes."),
                };
                (
                    draft.keyboard_title.to_owned(),
                    format!(
                        "Name: {}    page {}/2\n{detail}",
                        shown_name(&draft.name),
                        draft.page + 1,
                    ),
                    draft.cursor,
                )
            }
            Screen::Place(draft) => {
                fill_keys(&mut labels, place_keys());
                (
                    "Placement".to_owned(),
                    format!(
                        "Offset X {:.2}  Y {:.2}  {}\nAnchor is {}. Inputs stay recorded.",
                        draft.working.delta_x,
                        draft.working.delta_y,
                        momentum_label(draft.working.momentum),
                        if self.editing.is_some() { "where the start save puts this role" } else { "the restored CPU position" }
                    ),
                    draft.cursor,
                )
            }
            Screen::Library(page) => {
                let start = (page.cursor / PAGE) * PAGE;
                for (slot, label) in labels.iter_mut().enumerate() {
                    if let Some(row) = page.rows.get(start + slot) {
                        *label = clip_label(&row.label);
                    }
                }
                let pages = page.rows.len().div_ceil(PAGE).max(1);
                let selected_label = page
                    .rows
                    .get(page.cursor)
                    .map(|row| row.label.as_str())
                    .unwrap_or("No recordings");
                // A reads the file and arms it for playback; it does not start it.
                // Y copies the inputs into the numbered Record Slot named by `label`.
                let action = match (&page.alternate, page.binds_slot) {
                    (Some(label), true) => format!("A Load from file, Y Copy to {label}, B closes."),
                    (Some(label), false) => format!("A loads, Y {label}, B closes."),
                    (None, _) => "A loads, B closes.".to_owned(),
                };
                (
                    page.title.clone(),
                    format!(
                        "{selected_label}\npage {}/{pages}  skipped {}    {action}",
                        page.cursor / PAGE + 1,
                        page.skipped
                    ),
                    page.cursor % PAGE,
                )
            }
        };
        Some(View {
            title,
            footer,
            labels,
            selected,
        })
    }

    /// Opens the save dialog. An empty or fighter-less take is refused.
    pub fn offer_save(
        &mut self,
        mut frames: Vec<InputFrame>,
        start: StartState,
        crop: bool,
        version: &str,
    ) -> Effect {
        if crop {
            crop_neutral_tail(&mut frames);
        }
        if frames.is_empty() || start.fighter_kind < 0 {
            return Effect::Notify("Recording was empty".to_owned());
        }
        self.offer_save_with_mii(frames, start, false, version, None)
    }

    /// Same dialog as [`Self::offer_save`], retaining an exact Mii captured with the take.
    pub fn offer_save_with_mii(
        &mut self,
        mut frames: Vec<InputFrame>,
        start: StartState,
        crop: bool,
        version: &str,
        mii: Option<crate::engine::mii::lifetime::Configuration>,
    ) -> Effect {
        if crop {
            crop_neutral_tail(&mut frames);
        }
        if frames.is_empty() || start.fighter_kind < 0 {
            return Effect::Notify("Recording was empty".to_owned());
        }
        self.open_name(NameDraft {
            name: "Recording".to_owned(),
            native_cursor: "Recording".encode_utf16().count(),
            page: 0,
            cursor: 31,
            body: NameBody::Macro(Box::new(MacroNameBody {
                frames,
                start,
                placement: Placement::default(),
                version: clip_version(version),
                mii,
            })),
            keyboard_title: MACRO_KEYBOARD_TITLE,
        });
        Effect::None
    }

    /// Opens the keyboard for a caller-owned draft. No macro frames are invented.
    pub fn offer_name(&mut self, context: NameContext, default_name: &str, summary: &str) -> Effect {
        if default_name.chars().count() > MAX_NAME_CHARS || default_name.chars().any(char::is_control) {
            return Effect::Notify(MacroError::InvalidName("is too long").to_string());
        }
        if context.title.is_empty() || context.title.chars().any(char::is_control) {
            return Effect::Notify("Keyboard title is empty".to_owned());
        }
        self.open_name(NameDraft {
            name: default_name.to_owned(),
            native_cursor: default_name.encode_utf16().count(),
            page: 0,
            cursor: 31,
            body: NameBody::External {
                token: context.token,
                summary: summary.to_owned(),
                title: context.title,
            },
            keyboard_title: context.title,
        });
        Effect::None
    }

    fn open_name(&mut self, draft: NameDraft) {
        self.playback_on = false;
        self.held_save = None;
        self.screen = Screen::Save(draft);
    }

    pub fn open_library(&mut self, rows: Vec<LibraryRow>, skipped: usize) {
        self.open_library_titled(rows, skipped, "Named Library", None);
    }

    /// `alternate` is the Record Slot label, for example "Record Slot 2".
    /// `None` leaves Y inert. Y emits [`Effect::BindToSlot`].
    pub fn open_library_titled(
        &mut self,
        rows: Vec<LibraryRow>,
        skipped: usize,
        title: impl Into<String>,
        alternate: Option<String>,
    ) {
        self.open_listed(rows, skipped, title.into(), alternate, true);
    }

    /// Same list as [`Self::open_library_titled`]. Y emits [`Effect::Alternate`]
    /// and the footer uses `alternate` as the action words after "Y".
    pub fn open_library_forward(
        &mut self,
        rows: Vec<LibraryRow>,
        skipped: usize,
        title: impl Into<String>,
        alternate: Option<String>,
    ) {
        self.open_listed(rows, skipped, title.into(), alternate, false);
    }

    fn open_listed(
        &mut self,
        rows: Vec<LibraryRow>,
        skipped: usize,
        title: String,
        alternate: Option<String>,
        binds_slot: bool,
    ) {
        self.playback_on = false;
        self.picked = None;
        self.screen = Screen::Library(Box::new(LibraryPage {
            rows,
            skipped,
            cursor: 0,
            title,
            alternate,
            binds_slot,
        }));
    }

    /// The supplied reference must come from the Training restore, never live movement.
    pub fn open_editor(&mut self, restored_reference: Option<(f32, f32)>) -> bool {
        if self.loaded.is_none() {
            return false;
        }
        let Some(anchor) = self.anchor.or(restored_reference) else {
            return false;
        };
        let working = self.loaded.as_ref().unwrap().recording.placement;
        self.edit_anchor = Some(anchor);
        self.playback_on = false;
        self.screen = Screen::Place(Box::new(PlaceDraft {
            working,
            cursor: 1,
            from_save: false,
        }));
        true
    }

    /// Edit placement on a role's active macro, the file Play plays (primary 2026-11-08 #70 (a)). `anchor` is where the
    /// macro's start save puts that role, the warp Play makes; the offset is added to it. The named selection and the
    /// CPU are left alone.
    pub fn open_placement(&mut self, id: String, recording: MacroRecording, anchor: (f32, f32)) -> bool {
        if recording.validate().is_err() {
            return false;
        }
        let working = recording.placement;
        self.editing = Some(Editing { id, recording });
        self.edit_anchor = Some(anchor);
        self.playback_on = false;
        self.held_save = None;
        self.screen = Screen::Place(Box::new(PlaceDraft {
            working,
            cursor: 1,
            from_save: false,
        }));
        true
    }

    pub fn on_pad(&mut self, pad: Pad) -> Effect {
        if matches!(self.screen, Screen::Fighter(_)) {
            return self.fighter_pad(pad);
        }
        match pad {
            Pad::Left => self.shift(0, -1),
            Pad::Right => self.shift(0, 1),
            Pad::Up => self.shift(-1, 0),
            Pad::Down => self.shift(1, 0),
            Pad::Confirm => self.confirm(),
            Pad::Back => self.back(),
            Pad::Extra => self.extra(),
            Pad::Rename => self.rename(),
        }
    }

    /// X on a caller-titled library that does not bind a slot: the row goes to the runtime to rename.
    fn rename(&mut self) -> Effect {
        let Screen::Library(page) = &self.screen else { return Effect::None };
        if page.alternate.is_none() || page.binds_slot {
            return Effect::None;
        }
        let Some(id) = page.rows.get(page.cursor).map(|row| row.id.clone()) else { return Effect::None };
        self.screen = Screen::Closed;
        Effect::Rename { id }
    }

    /// Install a macro that was just written or loaded. Closes any panel.
    pub fn commit_loaded(
        &mut self,
        id: String,
        recording: MacroRecording,
        cpu_kind: Option<i32>,
    ) -> Result<Effect, MacroError> {
        recording.validate()?;
        self.install(Some(id), recording, cpu_kind)
    }

    /// Validated recording from the replay-export stream. No selector is created.
    pub fn import_recording(
        &mut self,
        recording: MacroRecording,
        cpu_kind: Option<i32>,
    ) -> Result<Effect, MacroError> {
        recording.validate()?;
        self.install(None, recording, cpu_kind)
    }

    /// One character-load request per explicit load or retry. Readiness loss alone
    /// never re-arms it, so changing the CPU does not undo the user's choice.
    pub fn claim_character_request(&mut self) -> Option<i32> {
        if self.claimed || self.character_ready {
            return None;
        }
        let kind = self.pending_kind?;
        self.claimed = true;
        Some(kind)
    }

    pub fn observe_cpu(&mut self, kind: Option<i32>, busy: bool) {
        self.observe_exact_cpu(kind, None, busy);
    }

    /// Configuration is supplied only after the runtime's exact selection ack.
    pub fn observe_exact_cpu(&mut self, kind: Option<i32>, configuration: Option<crate::engine::mii::lifetime::Configuration>, busy: bool) {
        if self.plays_current() {
            // Only the recorded-identity match is bypassed: the CPU must still
            // exist and be settled (not mid-install) before inputs play.
            self.character_ready = !busy && kind.is_some();
            self.pending_kind = None;
            self.claimed = false;
            return;
        }
        let target = self.loaded_target();
        self.character_ready = !busy && target.is_some_and(|target|
            kind.is_some_and(|kind| target.ready(kind as u32, configuration)));
        if self.character_ready {
            self.pending_kind = None;
            self.claimed = false;
            if let Some(loaded) = self.loaded.as_mut() {
                if loaded.plays == Plays::SwitchOnce { loaded.plays = Plays::Current; }
            }
        }
    }

    /// Absolute CPU position for this restore. The same generation keeps the
    /// first anchor, so a later call with the already-offset position does not
    /// add the delta again.
    pub fn restored_target(
        &mut self,
        generation: u64,
        saved_anchor: (f32, f32),
    ) -> Option<(f32, f32)> {
        if self.anchor_generation != Some(generation) {
            self.anchor = Some(saved_anchor);
            self.anchor_generation = Some(generation);
        }
        let placement = self.loaded.as_ref()?.recording.placement;
        let anchor = self.anchor?;
        Some(placement.apply(anchor))
    }

    /// Airborne recordings retain their situation at a grounded/zero-offset anchor.
    pub fn restore_in_air(&self) -> bool {
        self.loaded.as_ref().is_some_and(|loaded| {
            loaded.recording.start.situation == 2
                || loaded.recording.start.speed_y != 0.0
                || loaded.recording.placement.delta_y != 0.0
        })
    }

    /// Speeds to apply at playback frame zero, once Wait/Fall is ready.
    /// AtRest is explicit: drift while the other fighter restores is not retained.
    pub fn ready_velocity(&self, settled: bool) -> Option<(f32, f32)> {
        let loaded = self.loaded.as_ref()?;
        if !settled {
            return None;
        }
        if loaded.recording.placement.momentum == MomentumMode::AtRest {
            return Some((0.0, 0.0));
        }
        Some((
            loaded.recording.start.speed_x,
            loaded.recording.start.speed_y,
        ))
    }

    pub fn marker(&self) -> Option<(f32, f32)> {
        let Screen::Place(draft) = &self.screen else {
            return None;
        };
        let anchor = if self.editing.is_some() { self.edit_anchor } else { self.anchor.or(self.edit_anchor) }?;
        Some(draft.working.apply(anchor))
    }

    pub fn arm_playback(&mut self) -> bool {
        if !self.playback_allowed() {
            return false;
        }
        let len = self.loaded.as_ref().unwrap().recording.frames.len();
        self.playback_on = len > 0;
        self.playback_frame = 0;
        self.playback_len = len;
        self.emitted = None;
        self.emitted_tick = None;
        self.playback_on
    }

    /// Returns the input for `tick`. A repeated tick does not advance.
    pub fn feed_playback(&mut self, tick: u64) -> Option<InputFrame> {
        if self.emitted_tick == Some(tick) {
            return self.emitted;
        }
        if !self.playback_on {
            return None;
        }
        let frame = self
            .loaded
            .as_ref()?
            .recording
            .frames
            .get(self.playback_frame)
            .copied();
        self.emitted = frame;
        self.emitted_tick = Some(tick);
        if self.playback_frame + 1 >= self.playback_len {
            self.playback_on = false;
        } else {
            self.playback_frame += 1;
        }
        frame
    }

    pub fn playback_frame(&self) -> usize {
        self.playback_frame
    }

    /// `copy_playback` into a caller-owned buffer (cleared, then filled; no allocation when its capacity suffices).
    pub fn copy_playback_into(&self, out: &mut Vec<InputFrame>) -> Option<(f32, i32)> {
        if !self.playback_allowed() {
            return None;
        }
        let recording = &self.loaded.as_ref()?.recording;
        if recording.frames.is_empty() {
            return None;
        }
        out.clear();
        out.extend_from_slice(&recording.frames);
        Some((recording.start.facing, recording.start.status))
    }

    /// Facing, status, and frames for one playback. Empty when the fighter is not ready.
    pub fn copy_playback(&self) -> Option<(f32, i32, Vec<InputFrame>)> {
        if !self.playback_allowed() {
            return None;
        }
        let recording = &self.loaded.as_ref()?.recording;
        if recording.frames.is_empty() {
            return None;
        }
        Some((
            recording.start.facing,
            recording.start.status,
            recording.frames.clone(),
        ))
    }

    /// Put an unsaved recording back on screen after the disk write failed.
    pub fn restore_draft(&mut self, recording: MacroRecording) {
        self.screen = Screen::Save(NameDraft {
            name: recording.name.clone(),
            native_cursor: recording.name.encode_utf16().count(),
            page: 0,
            cursor: 31,
            body: NameBody::Macro(Box::new(MacroNameBody {
                frames: recording.frames.clone(),
                start: recording.start,
                placement: recording.placement,
                version: recording.game_version.clone(),
                mii: recording.mii,
            })),
            keyboard_title: MACRO_KEYBOARD_TITLE,
        });
        self.playback_on = false;
    }

    pub fn close_transient(&mut self) {
        self.screen = Screen::Closed;
        self.editing = None;
        self.picked = None;
        self.prompt_commit = None;
        self.held_save = None;
        self.clear_anchor();
    }

    /// Keep exact unfinished panels and draft values across a waiting match, but
    /// invalidate the old stage anchor and all playback/character acknowledgments.
    /// A fighter prompt is about the old stage's CPU: it is retired, not parked.
    pub fn invalidate_practice_context(&mut self) {
        self.clear_anchor();
        self.drop_fighter_prompt();
        self.picked = None;
        self.character_ready = false;
        self.claimed = false;
    }

    fn clear_anchor(&mut self) {
        self.playback_on = false;
        self.emitted = None;
        self.emitted_tick = None;
        self.anchor = None;
        self.anchor_generation = None;
        self.edit_anchor = None;
    }

    fn install(
        &mut self,
        id: Option<String>,
        recording: MacroRecording,
        cpu_kind: Option<i32>,
    ) -> Result<Effect, MacroError> {
        let kind = recording.start.fighter_kind;
        self.loaded = Some(Loaded { id, recording, plays: Plays::Recorded });
        self.screen = Screen::Closed;
        self.picked = None;
        self.held_save = None;
        self.playback_on = false;
        self.playback_frame = 0;
        self.playback_len = 0;
        let target = self.loaded_target();
        if target.is_none() {
            self.character_ready = false;
            self.pending_kind = None;
            self.claimed = false;
            return Ok(Effect::Notify("This macro has no exact Mii configuration".into()));
        }
        if cpu_kind == Some(kind) && target.is_some_and(|target| target.configuration().is_none()) {
            self.character_ready = true;
            self.pending_kind = None;
            self.claimed = false;
            Ok(Effect::None)
        } else {
            self.character_ready = false;
            self.pending_kind = Some(kind);
            self.claimed = false;
            Ok(Effect::NeedCharacter(kind))
        }
    }

    /// Load a recording the user explicitly selected. The fighter gate decides
    /// whether it commits at once (same fighter, or Keep), or waits for a choice.
    /// Nothing is committed, and the previous macro stays loaded, until then.
    pub fn offer_load(
        &mut self,
        id: String,
        recording: MacroRecording,
        context: LoadContext,
    ) -> Result<Effect, MacroError> {
        recording.validate()?;
        match context.gate(&recording) {
            FighterGate::Proceed => Ok(self.install_current(Some(id), recording, context.cpu_kind)),
            FighterGate::Keep => {
                let legacy = recording.playback_target().is_err() && !context.matches;
                let notice = legacy.then(|| {
                    Effect::Notify("Older Mii macro: playing on the current fighter".into())
                });
                let effect = self.install_current(Some(id), recording, context.cpu_kind);
                Ok(notice.unwrap_or(effect))
            }
            FighterGate::Prompt => {
                let prior = self.picked.take();
                self.playback_on = false;
                self.held_save = None;
                self.screen = Screen::Fighter(Box::new(FighterScreen {
                    prompt: fighter::Prompt::new(
                        FighterOrigin::Named,
                        // A library load plays on the CPU (`LoadContext::cpu_kind`).
                        ViewRole::Cpu,
                        context.revision,
                        context.recorded_label,
                        context.current_label,
                    ),
                    cpu_kind: context.cpu_kind,
                    candidate: Some((id, recording)),
                    prior,
                }));
                Ok(Effect::None)
            }
        }
    }

    /// Prompt for a replay clip or track, which previews on the CPU. The clip editor keeps the candidate and
    /// receives [`Effect::ClipFighter`]. Refused while another panel is open.
    pub fn open_fighter_prompt(
        &mut self,
        revision: u64,
        recorded_label: String,
        current_label: String,
    ) -> bool {
        self.open_fighter_prompt_for(FighterOrigin::Clip, ViewRole::Cpu, revision, recorded_label, current_label)
    }

    /// Prompt for a caller that keeps its own candidate: a replay clip or track
    /// ([`Effect::ClipFighter`]) or a chosen combo ([`Effect::ComboFighter`]).
    /// `side` plays the inputs and `current_label` is its fighter. Refused while another panel is open.
    pub fn open_fighter_prompt_for(
        &mut self,
        origin: FighterOrigin,
        side: ViewRole,
        revision: u64,
        recorded_label: String,
        current_label: String,
    ) -> bool {
        if origin == FighterOrigin::Named || !matches!(self.screen, Screen::Closed) {
            return false;
        }
        self.playback_on = false;
        self.screen = Screen::Fighter(Box::new(FighterScreen {
            prompt: fighter::Prompt::new(origin, side, revision, recorded_label, current_label),
            cpu_kind: None,
            candidate: None,
            prior: None,
        }));
        true
    }

    /// The library load a fighter prompt just committed, once. The runtime then
    /// remembers it as the selected macro; a Cancelled or stale prompt yields none.
    pub fn take_prompt_commit(&mut self) -> Option<(String, String)> {
        self.prompt_commit.take()
    }

    /// Origin and selection revision of the prompt on screen, if any.
    pub fn fighter_prompt(&self) -> Option<(FighterOrigin, u64)> {
        match &self.screen {
            Screen::Fighter(screen) => Some((screen.prompt.origin(), screen.prompt.revision())),
            _ => None,
        }
    }

    /// Retire a prompt whose selection or scene ended. Never applies a choice and
    /// never changes the loaded macro.
    pub fn drop_fighter_prompt(&mut self) -> bool {
        if !matches!(self.screen, Screen::Fighter(_)) {
            return false;
        }
        self.screen = Screen::Closed;
        true
    }

    /// Plays on the settled CPU, whatever it is. No character request is created,
    /// and the file's recorded fighter and Mii are untouched.
    fn install_current(
        &mut self,
        id: Option<String>,
        recording: MacroRecording,
        cpu_kind: Option<i32>,
    ) -> Effect {
        self.loaded = Some(Loaded { id, recording, plays: Plays::Current });
        self.screen = Screen::Closed;
        self.picked = None;
        self.held_save = None;
        self.playback_on = false;
        self.playback_frame = 0;
        self.playback_len = 0;
        self.character_ready = cpu_kind.is_some();
        self.pending_kind = None;
        self.claimed = false;
        Effect::None
    }

    /// True when the loaded macro plays on whatever CPU is settled, so the
    /// runtime has no recorded identity to wait for.
    pub fn plays_current(&self) -> bool {
        self.loaded.as_ref().is_some_and(|loaded| loaded.plays == Plays::Current)
    }

    fn fighter_pad(&mut self, pad: Pad) -> Effect {
        let Screen::Fighter(screen) = &mut self.screen else { return Effect::None };
        match screen.prompt.on_pad(pad) {
            fighter::Outcome::None => Effect::None,
            fighter::Outcome::Cancelled => self.cancel_fighter(),
            fighter::Outcome::Chose(choice) => self.choose_fighter(choice),
        }
    }

    fn cancel_fighter(&mut self) -> Effect {
        let Screen::Fighter(screen) = std::mem::replace(&mut self.screen, Screen::Closed) else {
            return Effect::None;
        };
        match screen.prompt.origin() {
            FighterOrigin::Clip => Effect::ClipFighter(None),
            FighterOrigin::Combo => Effect::ComboFighter(None),
            FighterOrigin::Recording => Effect::RecordingFighter(None),
            FighterOrigin::Named => {
                // Back to the list the user picked from; the loaded macro never changed.
                if let Some(page) = screen.prior {
                    self.screen = Screen::Library(page);
                }
                Effect::None
            }
        }
    }

    fn choose_fighter(&mut self, choice: FighterChoice) -> Effect {
        let Screen::Fighter(screen) = std::mem::replace(&mut self.screen, Screen::Closed) else {
            return Effect::None;
        };
        let FighterScreen { prompt, cpu_kind, candidate, .. } = *screen;
        let Some((id, recording)) = candidate.filter(|_| prompt.origin() == FighterOrigin::Named) else {
            return match prompt.origin() {
                FighterOrigin::Combo => Effect::ComboFighter(Some(choice)),
                FighterOrigin::Recording => Effect::RecordingFighter(Some(choice)),
                _ => Effect::ClipFighter(Some(choice)),
            };
        };
        self.prompt_commit = Some((id.clone(), recording.name.clone()));
        match choice {
            FighterChoice::Keep => self.install_current(Some(id), recording, cpu_kind),
            // The existing exact-fighter route. The CPU differs by construction, so
            // pass no live kind and let install request the recorded target.
            FighterChoice::Switch => {
                let effect = self.install(Some(id), recording, None)
                    .unwrap_or_else(|error| Effect::Notify(error.to_string()));
                if let Some(loaded) = self.loaded.as_mut() { loaded.plays = Plays::SwitchOnce; }
                effect
            },
        }
    }

    fn shift(&mut self, d_row: i32, d_col: i32) -> Effect {
        match &mut self.screen {
            Screen::Save(draft) => draft.cursor = nudge(draft.cursor, d_row, d_col),
            Screen::Place(draft) => draft.cursor = nudge(draft.cursor, d_row, d_col),
            Screen::Library(page) => {
                page.cursor = move_library(page.cursor, page.rows.len(), d_row, d_col);
            }
            Screen::Closed | Screen::Fighter(_) => {}
        }
        Effect::None
    }

    fn confirm(&mut self) -> Effect {
        match &self.screen {
            Screen::Save(draft) => {
                let key = name_keys(draft.page)[draft.cursor];
                self.activate_name(key)
            }
            Screen::Place(draft) => {
                let key = place_keys()[draft.cursor];
                self.activate_place(key)
            }
            Screen::Library(page) => {
                let id = page.rows.get(page.cursor).map(|row| row.id.clone());
                let forward = !page.binds_slot;
                match id {
                    Some(id) => {
                        let screen = std::mem::replace(&mut self.screen, Screen::Closed);
                        if forward {
                            Effect::ForwardChoose { id }
                        } else {
                            // Keep the list for a Cancelled fighter prompt.
                            if let Screen::Library(page) = screen {
                                self.picked = Some(page);
                            }
                            Effect::Choose(id)
                        }
                    }
                    None => Effect::Notify("No recordings saved yet".to_owned()),
                }
            }
            Screen::Closed | Screen::Fighter(_) => Effect::None,
        }
    }

    fn activate_name(&mut self, key: Key) -> Effect {
        match key {
            Key::Char(ch) => {
                self.push_char(ch);
                Effect::None
            }
            Key::Backspace => {
                self.pop_char();
                Effect::None
            }
            Key::Clear => {
                if let Screen::Save(draft) = &mut self.screen {
                    draft.name.clear();
                    draft.native_cursor = 0;
                }
                Effect::None
            }
            Key::Next => {
                if let Screen::Save(draft) = &mut self.screen {
                    draft.page = 1;
                    draft.cursor = 0;
                }
                Effect::None
            }
            Key::Prev => {
                if let Screen::Save(draft) = &mut self.screen {
                    draft.page = 0;
                    draft.cursor = 0;
                }
                Effect::None
            }
            Key::Save => self.finish_save(),
            Key::Cancel => self.discard(),
            Key::Place => self.open_place_from_save(),
            Key::Momentum => {
                self.toggle_draft_momentum();
                Effect::None
            }
            _ => Effect::None,
        }
    }

    fn activate_place(&mut self, key: Key) -> Effect {
        match key {
            Key::Nudge(x, y) => {
                if let Screen::Place(draft) = &mut self.screen {
                    draft.working.delta_x += f32::from(x) / 4.0;
                    draft.working.delta_y += f32::from(y) / 4.0;
                }
                Effect::None
            }
            Key::Momentum => {
                if let Screen::Place(draft) = &mut self.screen {
                    draft.working.momentum = flip(draft.working.momentum);
                }
                Effect::None
            }
            Key::Keep => self.keep_temporary(),
            Key::Write => self.write_over(),
            Key::SaveAs => self.save_as(),
            Key::Back | Key::Cancel => self.back(),
            _ => Effect::None,
        }
    }

    fn push_char(&mut self, ch: char) {
        let Screen::Save(draft) = &mut self.screen else {
            return;
        };
        if draft.name.chars().count() >= MAX_NAME_CHARS || ch.is_control() {
            return;
        }
        draft.name.push(ch);
        draft.native_cursor = draft.name.encode_utf16().count();
    }

    fn pop_char(&mut self) {
        let Screen::Save(draft) = &mut self.screen else {
            return;
        };
        draft.name.pop();
        draft.native_cursor = draft.name.encode_utf16().count();
    }

    fn extra(&mut self) -> Effect {
        if matches!(self.screen, Screen::Save(_)) {
            self.pop_char();
            return Effect::None;
        }
        let Screen::Library(page) = &self.screen else { return Effect::None };
        if page.alternate.is_none() {
            return Effect::None;
        }
        let binds_slot = page.binds_slot;
        let id = page.rows.get(page.cursor).map(|row| row.id.clone());
        match id {
            Some(id) => {
                self.screen = Screen::Closed;
                if binds_slot {
                    Effect::BindToSlot { id }
                } else {
                    Effect::Alternate { id }
                }
            }
            None => Effect::Notify("No recordings saved yet".to_owned()),
        }
    }

    fn toggle_draft_momentum(&mut self) {
        if let Screen::Save(draft) = &mut self.screen {
            if let NameBody::Macro(body) = &mut draft.body {
                body.placement.momentum = flip(body.placement.momentum);
            }
        }
    }

    fn open_place_from_save(&mut self) -> Effect {
        if matches!(&self.screen, Screen::Save(draft) if matches!(draft.body, NameBody::External { .. })) {
            return Effect::None;
        }
        if self.anchor.is_none() {
            return Effect::Notify("Restore a Training state before editing placement".into());
        }
        let screen = std::mem::replace(&mut self.screen, Screen::Closed);
        let Screen::Save(draft) = screen else {
            self.screen = screen;
            return Effect::None;
        };
        let NameBody::Macro(body) = &draft.body else {
            self.screen = Screen::Save(draft);
            return Effect::None;
        };
        let working = body.placement;
        self.held_save = Some(draft);
        self.screen = Screen::Place(Box::new(PlaceDraft {
            working,
            cursor: 1,
            from_save: true,
        }));
        Effect::None
    }

    fn finish_save(&mut self) -> Effect {
        let external = match &self.screen {
            Screen::Save(draft) => match draft.body {
                NameBody::External { token, title, .. } => Some((token, title, draft.name.clone())),
                NameBody::Macro(_) => None,
            },
            _ => return Effect::None,
        };
        if let Some((token, title, raw)) = external {
            let name = raw.trim();
            if let Err(err) = external_name(name) {
                return Effect::Notify(err.to_string());
            }
            let name = name.to_owned();
            self.screen = Screen::Closed;
            return Effect::NameConfirmed { context: NameContext { token, title }, name };
        }
        let (name, version, start, frames, placement, mii) = match &self.screen {
            Screen::Save(draft) => match &draft.body {
                NameBody::Macro(body) => (
                    draft.name.clone(),
                    body.version.clone(),
                    body.start,
                    body.frames.clone(),
                    body.placement,
                    body.mii,
                ),
                NameBody::External { .. } => return Effect::None,
            },
            _ => return Effect::None,
        };
        let built = MacroRecording::new(name.trim(), version, start, frames);
        let mut recording = match built {
            Ok(recording) => recording,
            Err(err) => return Effect::Notify(err.to_string()),
        };
        if let Some(configuration) = mii {
            if let Err(err) = recording.set_mii(configuration) { return Effect::Notify(err.to_string()); }
        }
        if let Err(err) = recording.set_placement(placement) {
            return Effect::Notify(err.to_string());
        }
        self.screen = Screen::Closed;
        Effect::Save(Box::new(recording))
    }

    fn keep_temporary(&mut self) -> Effect {
        let (placement, from_save) = match &self.screen {
            Screen::Place(draft) => (draft.working, draft.from_save),
            _ => return Effect::None,
        };
        if from_save {
            return self.restore_save_from_place(placement);
        }
        if let Some(editing) = self.editing.take() {
            self.screen = Screen::Closed;
            return Effect::KeepPlacement { id: editing.id, placement };
        }
        if let Some(loaded) = self.loaded.as_mut() {
            if loaded.recording.set_placement(placement).is_err() {
                return Effect::Notify("Placement is not usable".to_owned());
            }
        }
        self.screen = Screen::Closed;
        Effect::Notify("Placement kept for this session".to_owned())
    }

    fn write_over(&mut self) -> Effect {
        let placement = match &self.screen {
            Screen::Place(draft) => draft.working,
            _ => return Effect::None,
        };
        if let Some(editing) = self.editing.as_mut() {
            if editing.recording.set_placement(placement).is_err() {
                return Effect::Notify("Placement is not usable".to_owned());
            }
            let Some(Editing { id, recording }) = self.editing.take() else { return Effect::None };
            self.screen = Screen::Closed;
            return Effect::Overwrite { id, recording: Box::new(recording) };
        }
        let Some(id) = self.loaded.as_ref().and_then(|loaded| loaded.id.clone()) else {
            return Effect::Notify("Save the recording before overwriting a file".to_owned());
        };
        let Some(loaded) = self.loaded.as_mut() else {
            return Effect::None;
        };
        if loaded.recording.set_placement(placement).is_err() {
            return Effect::Notify("Placement is not usable".to_owned());
        }
        let recording = Box::new(loaded.recording.clone());
        self.screen = Screen::Closed;
        Effect::Overwrite { id, recording }
    }

    fn save_as(&mut self) -> Effect {
        let placement = match &self.screen {
            Screen::Place(draft) => draft.working,
            _ => return Effect::None,
        };
        let editing = self.editing.take();
        let Some(source) = editing.as_ref().map(|editing| &editing.recording).or(self.loaded.as_ref().map(|loaded| &loaded.recording)) else {
            return Effect::Notify("Nothing to save".to_owned());
        };
        let draft = NameDraft {
            name: source.name.clone(),
            native_cursor: source.name.encode_utf16().count(),
            page: 0,
            cursor: 31,
            body: NameBody::Macro(Box::new(MacroNameBody {
                frames: source.frames.clone(),
                start: source.start,
                placement,
                version: source.game_version.clone(),
                mii: source.mii,
            })),
            keyboard_title: MACRO_KEYBOARD_TITLE,
        };
        self.held_save = None;
        self.screen = Screen::Save(draft);
        Effect::None
    }

    fn back(&mut self) -> Effect {
        let from_save = matches!(&self.screen, Screen::Place(draft) if draft.from_save);
        if from_save {
            let placement = match &self.screen {
                Screen::Place(draft) => draft.working,
                _ => Placement::default(),
            };
            return self.restore_save_from_place(placement);
        }
        match &self.screen {
            Screen::Place(_) | Screen::Save(_) | Screen::Library(_) => self.discard(),
            Screen::Closed | Screen::Fighter(_) => Effect::None,
        }
    }

    fn restore_save_from_place(&mut self, placement: Placement) -> Effect {
        let Some(mut held) = self.held_save.take() else {
            self.screen = Screen::Closed;
            return Effect::None;
        };
        if let NameBody::Macro(body) = &mut held.body {
            body.placement = placement;
        }
        self.screen = Screen::Save(held);
        Effect::None
    }

    fn discard(&mut self) -> Effect {
        self.held_save = None;
        self.editing = None;
        self.screen = Screen::Closed;
        Effect::None
    }
}

impl Default for MacroUi {
    fn default() -> Self {
        Self::new()
    }
}

fn external_name(name: &str) -> Result<(), MacroError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(MacroError::InvalidName("is empty"));
    }
    if name.chars().any(char::is_control) {
        return Err(MacroError::InvalidName("contains control characters"));
    }
    if name.chars().count() > MAX_NAME_CHARS {
        return Err(MacroError::InvalidName("is too long"));
    }
    Ok(())
}

fn nudge(cursor: usize, d_row: i32, d_col: i32) -> usize {
    let cols = NX_SUBMENU_COLUMNS as i32;
    let row = ((cursor / NX_SUBMENU_COLUMNS) as i32 + d_row).clamp(0, NX_SUBMENU_ROWS as i32 - 1);
    let col = ((cursor % NX_SUBMENU_COLUMNS) as i32 + d_col).clamp(0, cols - 1);
    (row as usize) * NX_SUBMENU_COLUMNS + col as usize
}

fn move_library(cursor: usize, count: usize, d_row: i32, d_col: i32) -> usize {
    if count == 0 {
        return 0;
    }
    let cols = NX_SUBMENU_COLUMNS as i32;
    let mut page = (cursor / PAGE) as i32;
    let local = cursor % PAGE;
    let mut row = (local / NX_SUBMENU_COLUMNS) as i32 + d_row;
    let col = ((local % NX_SUBMENU_COLUMNS) as i32 + d_col).clamp(0, cols - 1);
    if row < 0 {
        if page > 0 {
            page -= 1;
            row = NX_SUBMENU_ROWS as i32 - 1;
        } else {
            row = 0;
        }
    } else if row >= NX_SUBMENU_ROWS as i32 {
        let pages = count.div_ceil(PAGE) as i32;
        if page + 1 < pages {
            page += 1;
            row = 0;
        } else {
            row = NX_SUBMENU_ROWS as i32 - 1;
        }
    }
    let index = page as usize * PAGE + row as usize * NX_SUBMENU_COLUMNS + col as usize;
    index.min(count - 1)
}

fn name_keys(page: usize) -> [Key; PAGE] {
    let mut keys = [Key::Empty; PAGE];
    if page == 0 {
        for (index, ch) in "ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().enumerate() {
            keys[index] = Key::Char(ch);
        }
        keys[26] = Key::Char('0');
        keys[27] = Key::Char('1');
        keys[28] = Key::Backspace;
        keys[29] = Key::Char(' ');
        keys[30] = Key::Next;
        keys[31] = Key::Save;
    } else {
        for (index, ch) in "23456789_-.'".chars().enumerate() {
            keys[index] = Key::Char(ch);
        }
        keys[12] = Key::Backspace;
        keys[13] = Key::Clear;
        keys[14] = Key::Prev;
        keys[16] = Key::Save;
        keys[17] = Key::Cancel;
        keys[18] = Key::Place;
        keys[19] = Key::Momentum;
    }
    keys
}

fn place_keys() -> [Key; PAGE] {
    let mut keys = [Key::Empty; PAGE];
    // Whole units are stored as quarters so the cell value stays an integer.
    keys[0] = Key::Nudge(-4, 0);
    keys[1] = Key::Nudge(4, 0);
    keys[2] = Key::Nudge(0, -4);
    keys[3] = Key::Nudge(0, 4);
    keys[4] = Key::Nudge(-1, 0);
    keys[5] = Key::Nudge(1, 0);
    keys[6] = Key::Nudge(0, -1);
    keys[7] = Key::Nudge(0, 1);
    keys[8] = Key::Momentum;
    keys[9] = Key::Keep;
    keys[10] = Key::Write;
    keys[11] = Key::SaveAs;
    keys[12] = Key::Back;
    keys
}

fn fill_keys(labels: &mut [String], keys: [Key; PAGE]) {
    for (label, key) in labels.iter_mut().zip(keys) {
        *label = key_label(key).to_owned();
    }
}

fn key_label(key: Key) -> &'static str {
    match key {
        Key::Char(' ') => "Space",
        Key::Char(ch) => match ch {
            'A' => "A",
            'B' => "B",
            'C' => "C",
            'D' => "D",
            'E' => "E",
            'F' => "F",
            'G' => "G",
            'H' => "H",
            'I' => "I",
            'J' => "J",
            'K' => "K",
            'L' => "L",
            'M' => "M",
            'N' => "N",
            'O' => "O",
            'P' => "P",
            'Q' => "Q",
            'R' => "R",
            'S' => "S",
            'T' => "T",
            'U' => "U",
            'V' => "V",
            'W' => "W",
            'X' => "X",
            'Y' => "Y",
            'Z' => "Z",
            '0' => "0",
            '1' => "1",
            '2' => "2",
            '3' => "3",
            '4' => "4",
            '5' => "5",
            '6' => "6",
            '7' => "7",
            '8' => "8",
            '9' => "9",
            '_' => "_",
            '-' => "-",
            '.' => ".",
            '\'' => "'",
            _ => "?",
        },
        Key::Backspace => "Del",
        Key::Clear => "Clear",
        Key::Next => "Next",
        Key::Prev => "Prev",
        Key::Save => "Save",
        Key::Cancel => "Cancel",
        Key::Place => "Place",
        Key::Momentum => "Momentum",
        Key::Nudge(-4, 0) => "X-",
        Key::Nudge(4, 0) => "X+",
        Key::Nudge(0, -4) => "Y-",
        Key::Nudge(0, 4) => "Y+",
        Key::Nudge(-1, 0) => "x-",
        Key::Nudge(1, 0) => "x+",
        Key::Nudge(0, -1) => "y-",
        Key::Nudge(0, 1) => "y+",
        Key::Keep => "Keep",
        Key::Write => "Write",
        Key::SaveAs => "New",
        Key::Back => "Back",
        Key::Empty | Key::Nudge(_, _) => "",
    }
}

fn momentum_label(mode: MomentumMode) -> &'static str {
    match mode {
        MomentumMode::PreserveRecorded => "Keep recorded speed",
        MomentumMode::AtRest => "Start at rest",
    }
}

fn flip(mode: MomentumMode) -> MomentumMode {
    match mode {
        MomentumMode::PreserveRecorded => MomentumMode::AtRest,
        MomentumMode::AtRest => MomentumMode::PreserveRecorded,
    }
}

fn shown_name(name: &str) -> String {
    if name.is_empty() {
        "(empty)".to_owned()
    } else {
        name.to_owned()
    }
}

fn clip_label(label: &str) -> String {
    label.chars().take(16).collect()
}

fn clip_version(version: &str) -> String {
    let mut out = String::new();
    for ch in version.chars().filter(|ch| !ch.is_control()) {
        if out.len() + ch.len_utf8() > crate::engine::macros::MAX_GAME_VERSION_BYTES {
            break;
        }
        out.push(ch);
    }
    out
}

pub fn crop_neutral_tail(frames: &mut Vec<InputFrame>) {
    while frames.last().is_some_and(neutral_frame) {
        frames.pop();
    }
}

fn neutral_frame(frame: &InputFrame) -> bool {
    let stick = |value: i8| value.unsigned_abs() < STICK_NEUTRAL as u8;
    (frame.buttons == 0 || frame.buttons == FLICK_JUMP)
        && stick(frame.lstick_x)
        && stick(frame.lstick_y)
        && stick(frame.rstick_x)
        && stick(frame.rstick_y)
}

