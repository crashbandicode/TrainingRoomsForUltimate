//! Clip selection. Native input consumption, not rendering, advances the preview.
use {crate::engine::macros::FrameWindow, crate::engine::macros::MacroRecording, crate::engine::macros::MoveStart, crate::engine::macros::MoveStartRules, crate::engine::macros::ReplayFrame, crate::engine::macros::ReplaySession, crate::engine::macros::ReplayTrack, crate::engine::macros::MAX_NAME_CHARS};
use crate::engine::macro_ui::macro_panel::Pad;
use crate::engine::macro_ui::clip_marks::{Marks, Span, SHOWN_MARKS};
use crate::model::combo_record::UnitKind;

pub const CLIP_BAR_UNITS: u16 = 1600;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle { Playhead, In, Out }
#[derive(Clone, Copy, Debug)]
pub enum ClipAction {
    SelectIn, SelectOut, Nudge(i32), TogglePlay, Restart, Snap, SwapTarget,
    PreviousFighter, NextFighter, ToggleCapture, UseLeadIn, Confirm, Cancel,
}
pub enum ClipEffect {
    None, StopPreview, Preview(Box<MacroRecording>), Close,
    PreviewFile { start: u32, end: u32, player: bool },
    Export(Box<MacroRecording>), ExportCombo(Box<crate::model::combo_record::ComboRecord>), Error(String),
}

/// Frames the game keeps running after a saved macro's preview took its last input, so the last move is seen finishing.
pub const PREVIEW_TAIL_FRAMES: u64 = 60;

#[derive(Default)]
pub struct PreviewTail { ended: Option<u64> }

impl PreviewTail {
    pub const fn new() -> Self { Self { ended: None } }
    pub fn restart(&mut self) { self.ended = None; }
    /// `running`: the playback still has frames. `frame`: the game's frame number. True once the tail has passed.
    pub fn over(&mut self, running: bool, frame: u64) -> bool {
        if running {
            self.ended = None;
            return false;
        }
        frame >= self.ended.get_or_insert(frame).saturating_add(PREVIEW_TAIL_FRAMES)
    }
}

/// P-TIMER: a selection across a capture break (play resumed after a stretch with no live play: sudden death).
pub const BREAK_CROSSED: &str = "The selection crosses the sudden-death restart. Keep it on one side.";

/// Frames of the same direction before held Left/Right starts repeating.
const SCRUB_REPEAT_DELAY: u32 = 10;
/// Hold time added when the fast modifier is already down, so the scroll
/// starts fast and still picks up from there.
const SCRUB_FAST_BIAS: u32 = 70;

/// Timeline frames moved by one tick after holding a direction this long.
/// A tap stays at one frame; a long hold reaches three minutes of replay per second.
pub fn scrub_step(held_frames: u32) -> i32 {
    match held_frames {
        0..=24 => 1,
        25..=44 => 3,
        45..=69 => 8,
        70..=99 => 20,
        100..=134 => 45,
        135..=174 => 90,
        _ => 180,
    }
}

/// Repeat step for a held direction. Zero during the tap window.
pub fn scrub_repeat_step(held_frames: u32, fast: bool) -> i32 {
    if held_frames < SCRUB_REPEAT_DELAY {
        return 0;
    }
    let paced = if fast {
        held_frames.saturating_add(SCRUB_FAST_BIAS)
    } else {
        held_frames
    };
    scrub_step(paced)
}

/// Horizontal editing wins over a diagonal D-pad. Fighter changes are D-pad
/// Up/Down only, and only while Left/Right is not held.
pub fn direction_action(
    left_edge: bool,
    right_edge: bool,
    up_edge: bool,
    down_edge: bool,
    horizontal_held: bool,
    fast: bool,
) -> Option<ClipAction> {
    let horizontal = i32::from(right_edge) - i32::from(left_edge);
    if horizontal != 0 {
        let step = if fast { scrub_step(SCRUB_FAST_BIAS) } else { 1 };
        return Some(ClipAction::Nudge(horizontal * step));
    }
    if horizontal_held || (up_edge && down_edge) {
        return None;
    }
    if up_edge {
        return Some(ClipAction::PreviousFighter);
    }
    if down_edge {
        return Some(ClipAction::NextFighter);
    }
    None
}

pub struct ClipHelp {
    pub primary: String,
    pub follow: String,
    pub controls: String,
    pub notice: String,
}

/// Button text for the step the crop editor is actually on.
pub fn clip_help(view: &ClipView, coarse: &str, chord: &str, pace: i32) -> ClipHelp {
    let controls = format!("{}{}{}Y snap. A save. B close.",
        if view.context.fighters() { "D-pad Up/Down: fighter. " } else { "" },
        if view.target.is_some() { "ZR Player/CPU. " } else { "" },
        if view.context.preview() { "X preview. " } else { "" });
    let (primary, follow) = match view.handle {
        Handle::Out => (
            "End selected. Hold Left or Right to move it.".to_owned(),
            format!("Hold longer to speed up, or hold {coarse} to start fast."),
        ),
        Handle::In => (
            "Start selected. Hold Left or Right to move it.".to_owned(),
            format!("Hold longer to speed up, or hold {coarse} to start fast."),
        ),
        Handle::Playhead if view.context.preview() && view.shown.is_some() => (
            "Playhead moved. L sets the start here, R the end.".to_owned(),
            "Left or Right browses on. Y goes to the nearest move.".to_owned(),
        ),
        Handle::Playhead if view.context.preview() => (
            "Hold Left or Right to browse the take, or Y for the nearest move.".to_owned(),
            format!("Press L for the start, or R for the end. Hold {coarse} to start fast."),
        ),
        Handle::Playhead => (
            "Press L for the start, or R for the end.".to_owned(),
            format!("Then hold Left or Right. Hold {coarse} to start fast."),
        ),
    };
    let follow = if pace > 1 {
        if pace >= 180 {
            format!("Moving {pace} {} per tick.", view.context.units())
        } else {
            format!("Moving {pace} {} per tick and speeding up.", view.context.units())
        }
    } else {
        follow
    };
    let notice = if let Some(start) = view.move_start {
        format!("Move at {} + {}f setup. Plays from the temporary preview setup.",
            start.action_frame, start.action_frame - start.frame)
    } else if view.clean_start {
        String::new()
    } else {
        format!("Press {chord}+Y to snap to the move start, then X to preview.")
    };
    if let Some(error) = &view.error {
        let [primary, follow] = feedback_lines(error);
        return ClipHelp { primary, follow, controls, notice };
    }
    ClipHelp { primary, follow, controls, notice }
}

/// Keep failures in the wide editor strip. The ordinary notification pane
/// shrinks long text to fit one narrow line, making preview refusals unreadable.
fn feedback_lines(message: &str) -> [String; 2] {
    const WIDTH: usize = 72;
    let mut lines = [String::new(), String::new()];
    let mut row = 0;
    for word in message.split_whitespace() {
        if !lines[row].is_empty() && lines[row].chars().count() + 1 + word.chars().count() > WIDTH {
            if row == 1 { lines[1].push_str("..."); break; }
            row = 1;
        }
        if !lines[row].is_empty() { lines[row].push(' '); }
        let remaining = WIDTH.saturating_sub(lines[row].chars().count());
        lines[row].extend(word.chars().take(remaining));
        if word.chars().count() > remaining { lines[row].push_str("..."); }
    }
    lines
}

/// Own the closing press independently of the draw clock. The native adapter polls
/// this once per P1 input mapping, preserving the physical closing-button level
/// between repeated mappings until the next native pad refresh. Neutralizing the
/// mapped output must not clear this release signal as a side effect.
#[derive(Default)]
pub struct CloseInputGuard { polls: u32, waiting_for_release: bool }

impl CloseInputGuard {
    pub const fn new() -> Self { Self { polls: 0, waiting_for_release: false } }

    pub fn arm(&mut self, grace_polls: u32) {
        self.polls = grace_polls;
        self.waiting_for_release = true;
    }

    pub fn poll(&mut self, in_training: bool, close_held: bool) -> bool {
        if !in_training { *self = Self::new(); return false; }
        self.waiting_for_release &= close_held;
        let owned = self.polls > 0 || self.waiting_for_release;
        self.polls = self.polls.saturating_sub(1);
        owned
    }
}

/// The press that opens the editor belongs to the opener. Native mapping can run
/// several times before the next pad refresh, re-presenting that same edge and
/// held level, so a fresh Confirm needs a release this guard actually observed.
/// Only an unheld sample without a press clears it. That sample never confirms:
/// the next press must arrive separately, with a held level and no release edge.
#[derive(Default)]
pub struct OpenInputGuard { waiting_for_release: bool }

impl OpenInputGuard {
    pub const fn new() -> Self { Self { waiting_for_release: false } }

    pub fn arm(&mut self) { self.waiting_for_release = true; }

    pub fn pending(&self) -> bool { self.waiting_for_release }

    /// Poll once per P1 mapping. Returns whether a Confirm edge may pass.
    pub fn allows_confirm(&mut self, held: bool, released: bool, pressed: bool) -> bool {
        if self.waiting_for_release {
            if !held && !pressed { self.waiting_for_release = false; }
            return false;
        }
        held && pressed && !released
    }
}

/// ZR pressed and released with no other button or direction in between: the Player 1 / CPU toggle. ZR held with a
/// direction is the coarse scrub and never a tap.
#[derive(Default)]
pub struct TargetTap { down: bool, clean: bool }

impl TargetTap {
    pub const fn new() -> Self { Self { down: false, clean: false } }

    /// Poll once per Player 1 input. `other`: any other button or direction is held. True on the release of a tap.
    pub fn tapped(&mut self, zr: bool, other: bool) -> bool {
        if zr {
            self.clean = if self.down { self.clean && !other } else { !other };
            self.down = true;
            return false;
        }
        let tap = self.down && self.clean && !other;
        self.down = false;
        self.clean = false;
        tap
    }
}

impl TargetTap {
    /// The pad's own button set, once per Player 1 input (a GameCube pad's Z arrives as the ZR bit).
    pub fn zr_tap(&mut self, held: crate::model::chords::Buttons) -> bool {
        self.tapped(held.zr(), held.other_than_zr())
    }
}

impl ClipView {
    /// The role a preview plays on and is fed through.
    pub fn play_role(&self) -> crate::engine::library::Role {
        if self.target == Some(true) { crate::engine::library::Role::Player } else { crate::engine::library::Role::Cpu }
    }
    pub fn scrubs(&self) -> bool { !self.playing && (self.handle != Handle::Playhead || self.context.preview()) }
}

/// The same editor changes units and offered actions with its source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipContext { Replay { fighters: bool }, Macro { player: bool }, Combo { unit: UnitKind } }

impl ClipContext {
    pub fn preview(self) -> bool { !matches!(self, Self::Combo { .. }) }
    pub fn fighters(self) -> bool { matches!(self, Self::Replay { fighters: true }) }
    pub fn units(self) -> &'static str { match self { Self::Combo { unit } => unit.plural(), _ => "frames" } }
}

enum FileSource {
    Macro { recording: Box<MacroRecording>, player: bool },
    Combo(Box<crate::model::combo_record::ComboRecord>),
}

#[derive(Clone, Debug)]
pub struct ClipView {
    pub context: ClipContext,
    pub total: u32,
    pub start: u32,
    pub end: u32,
    pub cursor: u32,
    pub shown: Option<u32>,
    pub handle: Handle,
    pub playing: bool,
    pub entry: i32,
    pub fighter_kind: i32,
    pub previous_selected: bool,
    pub previous_available: bool,
    pub clean_start: bool,
    pub move_start: Option<MoveStart>,
    pub quantized: bool,
    pub truncated: bool,
    pub source_name: String,
    pub error: Option<String>,
    pub marks: [(u16, u16); SHOWN_MARKS],
    pub mark_count: u8,
    pub move_text: String,
    /// Every mark of the take, not only the drawn ones, and the mark at or nearest the cursor.
    pub mark_total: u32,
    pub near: Option<(u32, Option<u32>)>,
    /// Where a preview plays: Player 1 (`true`) or the CPU. `None` when the source has no preview.
    pub target: Option<bool>,
}

pub struct ClipEditor {
    session: ReplaySession,
    file: Option<FileSource>,
    previous: Option<ReplaySession>,
    previous_selected: bool,
    track: usize,
    window: FrameWindow,
    handle: Handle,
    playing: bool,
    browsed: bool,
    clean_statuses: Vec<i32>,
    move_rules: Option<MoveStartRules>,
    move_start: Option<MoveStart>,
    source_name: String,
    previous_name: String,
    open_guard: OpenInputGuard,
    error: Option<String>,
    marks: Marks,
    replay_player: bool,
    watch_base: u32,
}

fn prepare(mut session: ReplaySession) -> Option<ReplaySession> {
    session.tracks.retain(|track| !track.frames.is_empty());
    session.tracks.sort_by_key(|track| track.entry);
    (!session.tracks.is_empty()).then_some(session)
}

/// The move names of a saved take or capture: the toggle, and its own fighter's special names when they are known
/// (the identity stored with it, else `clip_marks::take_identity`); the common names otherwise.
fn saved_namer(recording: &MacroRecording) -> crate::model::move_names::Namer<'static> {
    use crate::model::move_names::{Namer, PUBLISHED};
    Namer::recorded(recording.recorded_fighter.or_else(|| crate::engine::macro_ui::clip_marks::take_identity(
        PUBLISHED.player(), recording.start.fighter_kind, recording.mii.as_ref().map(|mii| mii.tokens().specials))))
}

fn track_marks(track: &ReplayTrack) -> Marks {
    Marks::from_inputs(track.frames.iter().map(|frame| frame.input))
}

impl ClipEditor {
    /// The same take or capture with its marks named by `namer` instead of the published toggle and fighter.
    pub fn with_namer(mut self, namer: crate::model::move_names::Namer<'static>) -> Self {
        if let Some(FileSource::Macro { recording, .. }) = &self.file {
            self.marks = Marks::from_recording(recording, namer);
        } else if self.file.is_none() {
            self.marks = Marks::from_inputs(self.session.tracks[self.track].frames.iter().map(|frame| frame.input)).named(namer);
        }
        self
    }

    pub fn new(session: ReplaySession, clean_statuses: Vec<i32>) -> Result<Self, String> {
        let session = prepare(session).ok_or("Play a saved replay in Vault first")?;
        let window = session.tracks[0].window()?;
        let marks = track_marks(&session.tracks[0]);
        let mut open_guard = OpenInputGuard::new();
        open_guard.arm();
        Ok(Self { session, file: None, previous: None, previous_selected: false, track: 0, window,
            handle: Handle::Playhead, playing: false, browsed: false, clean_statuses, move_rules: None, move_start: None,
            source_name: "Latest Vault capture".into(), previous_name: "Previous Vault capture".into(),
            open_guard, error: None, marks, replay_player: false, watch_base: 0 })
    }

    /// Saved files have one side and one start context. A combo has link units,
    /// with no invented frame samples or continuous inputs.
    pub fn macro_file(recording: MacroRecording, player: bool) -> Result<Self, String> {
        recording.validate().map_err(|error| error.to_string())?;
        let total = recording.frames.len() as u32;
        let name = recording.name.clone();
        Self::file(FileSource::Macro { recording: Box::new(recording), player }, total, name)
    }

    pub fn combo_file(recording: crate::model::combo_record::ComboRecord) -> Result<Self, String> {
        recording.validate().map_err(|error| error.to_string())?;
        let total = recording.unit_count();
        let name = recording.name.clone();
        Self::file(FileSource::Combo(Box::new(recording)), total, name)
    }

    fn file(file: FileSource, total: u32, source_name: String) -> Result<Self, String> {
        let marks = match &file {
            FileSource::Macro { recording, .. } => Marks::from_recording(recording, saved_namer(recording)),
            FileSource::Combo(recording) => Marks::from_units(
                crate::model::recording_menu::combo_link_lines(&recording.combo, crate::model::move_names::Namer::recorded(recording.fighter)).into_iter()
                    .filter(|line| crate::model::recording_menu::link_head(line))),
        };
        let mut open_guard = OpenInputGuard::new();
        open_guard.arm();
        Ok(Self { session: ReplaySession { tracks: Vec::new(), truncated: false, breaks: Vec::new() },
            file: Some(file), previous: None, previous_selected: false, track: 0,
            window: FrameWindow::new(total).map_err(|error| error.to_string())?, handle: Handle::Playhead, playing: false, browsed: false,
            clean_statuses: Vec::new(), move_rules: None, move_start: None,
            source_name, previous_name: String::new(), open_guard, error: None, marks, replay_player: false, watch_base: 0 })
    }

    /// Where X previews: a saved macro's play role, a capture's play target, nothing for a combo or a drill.
    pub fn plays_on_player(&self) -> Option<bool> {
        match &self.file {
            Some(FileSource::Macro { player, .. }) => Some(*player),
            Some(FileSource::Combo(_)) => None,
            None => Some(self.replay_player),
        }
    }

    pub fn set_play_target(&mut self, on_player: bool) {
        match &mut self.file {
            Some(FileSource::Macro { player, .. }) => *player = on_player,
            Some(FileSource::Combo(_)) => {}
            None => self.replay_player = on_player,
        }
    }

    pub fn context(&self) -> ClipContext {
        match &self.file {
            Some(FileSource::Macro { player, .. }) => ClipContext::Macro { player: *player },
            Some(FileSource::Combo(recording)) => ClipContext::Combo { unit: recording.unit_kind() },
            None => ClipContext::Replay { fighters: self.session.tracks.len() > 1 },
        }
    }

    /// Physical A edge gate for the native adapter; see `OpenInputGuard`.
    pub fn allows_confirm(&mut self, held: bool, released: bool, pressed: bool) -> bool {
        self.open_guard.allows_confirm(held, released, pressed)
    }

    /// True while the opening press has not been seen released.
    pub fn confirm_release_pending(&self) -> bool { self.open_guard.pending() }

    /// Normalized UI events carry no physical release sample. They must not clear
    /// the opening fence or bypass it when the editor is not in the foreground.
    pub fn action_for_pad(&self, pad: Pad) -> Option<ClipAction> {
        Some(match pad {
            Pad::Confirm if self.confirm_release_pending() => return None,
            Pad::Confirm => ClipAction::Confirm, Pad::Back => ClipAction::Cancel,
            Pad::Left => ClipAction::Nudge(-1), Pad::Right => ClipAction::Nudge(1),
            Pad::Up => ClipAction::PreviousFighter, Pad::Down => ClipAction::NextFighter,
            Pad::Extra => ClipAction::TogglePlay,
            Pad::Rename => return None,
        })
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.source_name = name;
        self
    }

    pub fn with_move_rules(mut self, rules: MoveStartRules) -> Self {
        self.move_rules = Some(rules);
        self
    }

    pub fn with_previous(mut self, previous: Option<ReplaySession>) -> Self {
        self.previous = previous.and_then(prepare);
        self
    }

    pub fn select_kind(&mut self, kind: i32) {
        if self.file.is_some() { return; }
        if let Some(index) = self.session.tracks.iter().position(|t| t.frames[0].start.fighter_kind == kind) {
            self.set_track(index);
        }
    }

    pub fn select_entry(&mut self, entry: i32) -> bool {
        if self.file.is_some() { return false; }
        let Some(index) = self.session.tracks.iter().position(|track| track.entry == entry) else { return false };
        self.set_track(index);
        true
    }

    /// Entry, recorded fighter kind and capture side (`true` = the previous
    /// capture) of the track `action` would select, without selecting it. Only a
    /// real change of track or capture answers; re-selecting the current track,
    /// or toggling with no previous capture, selects nothing new.
    pub fn peek_selection(&self, action: ClipAction) -> Option<(i32, i32, bool)> {
        if self.file.is_some() { return None; }
        match action {
            ClipAction::PreviousFighter | ClipAction::NextFighter => {
                let count = self.session.tracks.len();
                let offset = if matches!(action, ClipAction::PreviousFighter) { count - 1 } else { 1 };
                let index = (self.track + offset) % count;
                if index == self.track { return None; }
                let track = &self.session.tracks[index];
                Some((track.entry, track.frames[0].start.fighter_kind, self.previous_selected))
            }
            ClipAction::ToggleCapture => {
                let track = self.previous.as_ref()?.tracks.first()?;
                Some((track.entry, track.frames.first()?.start.fighter_kind, !self.previous_selected))
            }
            _ => None,
        }
    }

    pub fn track(&self) -> &ReplayTrack { &self.session.tracks[self.track] }
    pub fn sample(&self) -> ReplayFrame { self.track().frames[self.window.cursor() as usize] }
    pub fn pause(&mut self) { self.playing = false; }
    pub fn report_error(&mut self, message: String) { self.error = Some(message); }

    /// The frame the fighters are shown at while scrubbing: the selected handle's, or the moved playhead's; nobody is held on None.
    pub fn shown_frame(&self) -> Option<u32> {
        (!self.playing && self.context().preview() && (self.handle != Handle::Playhead || self.browsed)).then_some(self.window.cursor())
    }

    pub fn view(&self) -> ClipView {
        let (entry, fighter_kind, clean_start, quantized) = match &self.file {
            Some(FileSource::Macro { recording, player }) =>
                (i32::from(!*player), recording.start.fighter_kind, true, false),
            Some(FileSource::Combo(_)) => (0, -1, true, false),
            None => {
                let track = self.track();
                (track.entry, self.sample().start.fighter_kind,
                    self.move_start.is_some() || self.clean_statuses.contains(&track.frames[self.window.start() as usize].start.status),
                    track.quantized_analog)
            }
        };
        let (marks, mark_count) = self.marks.shown(self.window.cursor(), self.window.total(), CLIP_BAR_UNITS);
        let (at, out_handle) = match self.handle {
            Handle::In => (self.window.start(), false),
            Handle::Out => (self.window.end(), true),
            Handle::Playhead => (self.window.cursor(), false),
        };
        ClipView { context: self.context(), total: self.window.total(), start: self.window.start(), end: self.window.end(),
            cursor: self.window.cursor(), shown: self.shown_frame(), handle: self.handle, playing: self.playing,
            entry, fighter_kind, previous_selected: self.previous_selected, previous_available: self.previous.is_some(),
            clean_start, move_start: self.move_start, quantized, truncated: self.session.truncated,
            source_name: self.source_name.clone(), error: self.error.clone(),
            marks, mark_count, move_text: self.marks.text(at, out_handle),
            mark_total: self.marks.list().len() as u32, near: self.marks.nearest(self.window.cursor()),
            target: self.plays_on_player() }
    }

    /// Report actual input consumption, relative to the selected range. Drawing
    /// the timeline never advances inputs or fabricates a simulated pose.
    pub fn preview_progress(&mut self, consumed: u32) {
        if !self.playing { return; }
        let start = self.window.start();
        let end = self.window.end();
        self.window.seek(start.saturating_add(consumed.saturating_sub(1)).min(end - 1));
        if consumed >= end - start { self.playing = false; }
    }

    pub fn watch(&mut self, spans: &[Span]) {
        if self.playing { self.marks.watch(self.watch_base, spans); }
    }
    pub fn preview_cursor(&mut self, consumed: u32) {
        if self.playing {
            self.window.seek(self.window.start().saturating_add(consumed.saturating_sub(1)).min(self.window.end() - 1));
        }
    }

    fn start_preview(&mut self) -> ClipEffect {
        self.pause();
        let effect = if let Some(FileSource::Macro { player, .. }) = self.file.as_ref() {
            ClipEffect::PreviewFile { start: self.window.start(), end: self.window.end(), player: *player }
        } else {
            let recording = match self.export_selection("Preview") {
                Ok(recording) => recording,
                Err(error) => return ClipEffect::Error(error),
            };
            // The selected window may be a different form from the track's first frame.
            self.marks.set_namer(saved_namer(&recording));
            ClipEffect::Preview(Box::new(recording))
        };
        self.watch_base = self.window.start();
        self.marks.forget_watched(self.watch_base, self.window.end());
        self.window.seek(self.watch_base);
        self.handle = Handle::Playhead;
        self.browsed = false;
        self.playing = true;
        effect
    }

    /// In at the start of the move the status rules find at `at`, with its short setup, kept as `move_start`: X and A
    /// then restart that move from neutral (`export_move`). `None` without rules.
    fn move_at(&mut self, at: u32) -> Option<Result<(), String>> {
        let rules = self.move_rules.as_ref()?;
        Some(match rules.snap(self.track(), at) {
            Ok(start) if start.action_frame < self.window.end() => {
                self.window.set_in(start.frame);
                self.window.seek(start.frame);
                self.handle = Handle::In;
                self.move_start = Some(start);
                Ok(())
            }
            Ok(_) => Err("Move is outside the crop; move Out later".into()),
            Err(error) => Err(error),
        })
    }

    fn export_selection(&self, name: &str) -> Result<MacroRecording, String> {
        if let Some(FileSource::Macro { recording, .. }) = &self.file {
            let mut kept = (**recording).clone();
            kept.name = name.to_owned();
            kept.frames = recording.frames[self.window.start() as usize..self.window.end() as usize].to_vec();
            kept.actions = if self.marks.recorded() {
                Marks::window(&recording.actions, self.window.start(), self.window.end())
            } else {
                Vec::new()
            };
            return Ok(kept);
        }
        // P-TIMER: the inputs after a break (sudden death) never play from a state before it.
        if self.session.break_within(self.window.start(), self.window.end().saturating_sub(1)).is_some() {
            return Err(BREAK_CROSSED.into());
        }
        match self.move_start.as_ref() {
            Some(start) => self.track().export_move(&self.window, name, "13.0.5", start),
            None => self.track().export(&self.window, name, "13.0.5", &self.clean_statuses),
        }
    }

    fn set_track(&mut self, index: usize) {
        let mut window = self.session.tracks[index].window().expect("nonempty track");
        // A full-capture selection means the whole newly selected fighter too.
        // Preserve explicit crops, not the previous fighter's shorter default Out.
        if self.window.start() != 0 || self.window.end() != self.window.total() {
            window.set_out(self.window.end());
            window.set_in(self.window.start());
        }
        window.seek(match self.handle {
            Handle::In => window.start(),
            Handle::Out => window.end() - 1,
            Handle::Playhead => self.window.cursor().min(window.total() - 1),
        });
        self.track = index;
        self.window = window;
        self.marks = track_marks(&self.session.tracks[index]);
        self.playing = false;
        self.browsed = false;
        self.move_start = None;
    }

    pub fn on_action(&mut self, action: ClipAction) -> ClipEffect {
        if self.file.is_some() && matches!(action,
            ClipAction::PreviousFighter | ClipAction::NextFighter | ClipAction::ToggleCapture | ClipAction::UseLeadIn) {
            return ClipEffect::None;
        }
        if matches!(self.file, Some(FileSource::Combo(_)))
            && matches!(action, ClipAction::SwapTarget) {
            return ClipEffect::None;
        }
        if !self.context().preview() && matches!(action, ClipAction::TogglePlay | ClipAction::Restart) {
            return ClipEffect::None;
        }
        self.error = None;
        let effect = self.apply_action(action);
        if let ClipEffect::Error(message) = &effect { self.report_error(message.clone()); }
        effect
    }

    fn apply_action(&mut self, action: ClipAction) -> ClipEffect {
        match action {
            ClipAction::Snap => {
                let at = match self.handle {
                    Handle::Playhead if self.context().preview() =>
                        self.marks.snap_in(self.window.cursor()).map(|at| at.min(self.window.total() - 1)),
                    Handle::Playhead => return ClipEffect::Error("Press L for the start or R for the end, then Y to snap".into()),
                    Handle::In if self.move_start.is_some() => Some(self.window.start()),
                    Handle::In => self.marks.snap_in(self.window.start()).map(|mark| self.window.set_in(mark)),
                    Handle::Out => self.marks.snap_out(self.window.end(), self.window.total()).map(|mark| self.window.set_out(mark) - 1),
                };
                let Some(at) = at else { return ClipEffect::Error("No move marks yet: X previews and marks them".into()) };
                self.pause();
                self.window.seek(at);
                if self.handle == Handle::Playhead { self.browsed = true; }
                else { self.move_start = self.move_start.filter(|start| start.action_frame < self.window.end()); }
                if self.handle == Handle::In && self.file.is_none() && self.move_start.is_none()
                    && !self.clean_statuses.contains(&self.track().frames[at as usize].start.status) {
                    if let Some(Err(error)) = self.move_at(at) { return ClipEffect::Error(error); }
                }
            }
            ClipAction::SwapTarget => {
                let Some(on_player) = self.plays_on_player() else { return ClipEffect::None };
                self.set_play_target(!on_player);
                let playing = self.playing;
                self.pause();
                return if playing { ClipEffect::StopPreview } else { ClipEffect::None };
            }
            ClipAction::SelectIn | ClipAction::SelectOut => {
                self.pause();
                let handle = if matches!(action, ClipAction::SelectIn) { Handle::In } else { Handle::Out };
                if self.handle == Handle::Playhead && self.browsed {
                    let cursor = self.window.cursor();
                    match handle {
                        Handle::In => {
                            if cursor >= self.window.end() { return ClipEffect::Error("Playhead is past Out: set Out first".into()); }
                            self.window.set_in(cursor);
                            self.move_start = None;
                        }
                        Handle::Out => {
                            if cursor <= self.window.start() { return ClipEffect::Error("Playhead is at or before In: set In first".into()); }
                            self.window.set_out(cursor + 1);
                            self.move_start = self.move_start.filter(|start| start.action_frame < self.window.end());
                        }
                        Handle::Playhead => {}
                    }
                }
                self.browsed = false;
                self.handle = if self.handle == handle { Handle::Playhead } else { handle };
                match self.handle {
                    Handle::In => { self.window.seek(self.window.start()); }
                    Handle::Out => { self.window.seek(self.window.end() - 1); }
                    Handle::Playhead => {}
                }
            }
            ClipAction::Nudge(delta) => {
                self.pause();
                let add = |value: u32| (i64::from(value) + i64::from(delta)).clamp(0, i64::from(u32::MAX)) as u32;
                match self.handle {
                    Handle::In => {
                        self.move_start = None;
                        self.window.set_in(add(self.window.start()));
                        self.window.seek(self.window.start());
                    }
                    Handle::Out => {
                        self.move_start = self.move_start.filter(|start| add(self.window.end()) > start.action_frame);
                        self.window.set_out(add(self.window.end()));
                        self.window.seek(self.window.end() - 1);
                    }
                    Handle::Playhead if self.context().preview() => {
                        self.window.seek(add(self.window.cursor()).min(self.window.total() - 1));
                        self.browsed = true;
                    }
                    Handle::Playhead => {
                        return ClipEffect::Error("Press L for the start or R for the end, then hold Left or Right".into());
                    }
                }
            }
            ClipAction::TogglePlay => {
                if !self.playing { return self.start_preview(); }
                self.pause();
                self.window.seek(self.window.start());
            }
            ClipAction::Restart => { return self.start_preview(); }
            ClipAction::PreviousFighter | ClipAction::NextFighter => {
                let count = self.session.tracks.len();
                let offset = if matches!(action, ClipAction::PreviousFighter) { count - 1 } else { 1 };
                self.set_track((self.track + offset) % count);
            }
            ClipAction::ToggleCapture => {
                self.browsed = false;
                if let Some(previous) = &mut self.previous {
                    std::mem::swap(&mut self.session, previous);
                    std::mem::swap(&mut self.source_name, &mut self.previous_name);
                    self.previous_selected = !self.previous_selected;
                    self.track = 0;
                    self.window = self.session.tracks[0].window().expect("nonempty capture");
                    self.marks = track_marks(&self.session.tracks[0]);
                    self.handle = Handle::Playhead;
                    self.move_start = None;
                    self.pause();
                }
            }
            ClipAction::UseLeadIn => {
                self.pause();
                // Repeated presses keep the same move, rather than treating
                // its included setup as a request for the preceding move.
                let requested = self.move_start.map_or(self.window.start(), |s| s.action_frame);
                match self.move_at(requested) {
                    Some(Ok(())) => {}
                    Some(Err(error)) => return ClipEffect::Error(error),
                    None => if let Some(start) = self.track().previous_clean_start(self.window.start(), &self.clean_statuses) {
                        self.window.set_in(start);
                        self.window.seek(start);
                        self.handle = Handle::In;
                    } else { return ClipEffect::Error("No earlier idle/fall start in this capture".into()); }
                }
            }
            ClipAction::Confirm => {
                self.pause();
                if let Some(FileSource::Combo(recording)) = &self.file {
                    return match recording.crop_units(self.window.start(), self.window.end()) {
                        Ok(saved) => { self.browsed = false; ClipEffect::ExportCombo(Box::new(saved)) }
                        Err(error) => ClipEffect::Error(error.to_string()),
                    };
                }
                if self.file.is_some() {
                    return match self.export_selection(&self.source_name) {
                        Ok(recording) => { self.browsed = false; ClipEffect::Export(Box::new(recording)) }
                        Err(error) => ClipEffect::Error(error),
                    };
                }
                let suffix = format!(" P{}", self.track().entry + 1);
                let prefix: String = self.source_name.chars()
                    .take(MAX_NAME_CHARS.saturating_sub(suffix.chars().count())).collect();
                return match self.export_selection(&format!("{prefix}{suffix}")) {
                    Ok(recording) => { self.browsed = false; ClipEffect::Export(Box::new(recording)) }
                    Err(error) => ClipEffect::Error(error),
                };
            }
            ClipAction::Cancel => {
                if self.playing {
                    self.pause();
                    self.window.seek(self.window.start());
                    return ClipEffect::StopPreview;
                }
                return ClipEffect::Close;
            }
        }
        ClipEffect::StopPreview
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use {crate::engine::macros::InputFrame, crate::engine::macros::MoveFamily, crate::engine::macros::StartState};

    fn snap_rules() -> MoveStartRules {
        MoveStartRules { families: vec![MoveFamily { fighter: None, statuses: vec![45, 46, 47], buttons: 0 }],
            setup: vec![6, 7], ground: 0, air: 2, wait: 0, fall: 14, trigger_buttons: 0 }
    }

    fn snap_capture_with(setup: i32, action: i32, tail: i32) -> ClipEditor {
        let frames = (0..60).map(|i| ReplayFrame {
            input: InputFrame { buttons: if (23..=26).contains(&i) { 1 } else { 0 }, ..InputFrame::default() },
            start: StartState { status: match i { 0..=19 => 0, 20..=24 => setup, 25..=39 => action, _ => tail },
                ..StartState::neutral(74, 0, 1.0) },
            x: 0.0, y: 0.0,
        }).collect();
        ClipEditor::new(ReplaySession { tracks: vec![ReplayTrack {
            entry: 1, fighter: None, quantized_analog: false, frames,
        }], truncated: false, breaks: Vec::new() }, vec![0, 14]).unwrap().with_move_rules(snap_rules())
    }

    fn snap_capture() -> ClipEditor {
        snap_capture_with(7, 45, 0)
    }

    fn cropped_capture() -> ClipEditor {
        let mut e = snap_capture();
        e.on_action(ClipAction::SelectOut);
        e.on_action(ClipAction::Nudge(-20));
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::Nudge(10));
        e.on_action(ClipAction::SelectIn);
        assert_eq!((e.view().start, e.view().end), (10, 40));
        e
    }

    fn the_playhead_browses_the_whole_take_and_leaves_in_and_out() {
        let mut e = cropped_capture();
        let move_start = e.view().move_start;
        assert!(!e.browsed);
        assert_eq!(e.shown_frame(), None);
        assert!(!matches!(e.on_action(ClipAction::Nudge(-5)), ClipEffect::Error(_)));
        assert_eq!(e.view().cursor, 5);
        assert_eq!(e.shown_frame(), Some(5));
        assert_eq!(e.view().shown, Some(5));
        e.on_action(ClipAction::Nudge(500));
        assert_eq!(e.view().cursor, 59);
        e.on_action(ClipAction::Nudge(-500));
        assert_eq!(e.view().cursor, 0);
        assert_eq!((e.view().start, e.view().end), (10, 40));
        assert_eq!(e.view().handle, Handle::Playhead);
        assert_eq!(e.view().move_start, move_start);

        let mut e = saved_macro(false);
        assert_eq!(e.shown_frame(), None);
        assert!(!matches!(e.on_action(ClipAction::Nudge(3)), ClipEffect::Error(_)));
        assert_eq!(e.view().cursor, 3);
        assert_eq!(e.shown_frame(), Some(3));
        e.on_action(ClipAction::Nudge(500));
        assert_eq!(e.view().cursor, e.view().total - 1);
        e.on_action(ClipAction::Nudge(-500));
        assert_eq!(e.view().cursor, 0);
        assert_eq!((e.view().start, e.view().end), (0, 10));
        assert_eq!(e.view().handle, Handle::Playhead);
    }

    fn y_snaps_the_playhead_to_the_nearest_mark() {
        let mut e = snap_capture();
        e.on_action(ClipAction::Nudge(30));
        assert!(!matches!(e.on_action(ClipAction::Snap), ClipEffect::Error(_)));
        assert_eq!(e.view().cursor, 23);
        assert_eq!((e.view().start, e.view().end), (0, 60));
        assert_eq!(e.shown_frame(), Some(23));
    }

    fn l_and_r_cut_here_after_the_playhead_moved() {
        let mut e = cropped_capture();
        e.on_action(ClipAction::Nudge(10));
        e.on_action(ClipAction::SelectIn);
        assert_eq!(e.view().start, 20);
        assert_eq!(e.view().handle, Handle::In);
        assert_eq!(e.shown_frame(), Some(20));
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::Nudge(15));
        e.on_action(ClipAction::SelectOut);
        assert_eq!((e.view().start, e.view().end, e.view().cursor), (20, 36, 35));
        assert_eq!(e.view().handle, Handle::Out);
    }

    fn l_and_r_only_select_when_the_playhead_has_not_moved() {
        let mut e = cropped_capture();
        e.on_action(ClipAction::SelectIn);
        assert_eq!((e.view().handle, e.view().start), (Handle::In, 10));
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::SelectOut);
        assert_eq!((e.view().handle, e.view().end), (Handle::Out, 40));
        let mut e = cropped_capture();
        e.on_action(ClipAction::Nudge(10));
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::SelectOut);
        assert_eq!((e.view().handle, e.view().end), (Handle::Out, 40));
    }

    fn cut_here_never_reorders_the_handles() {
        let mut e = cropped_capture();
        e.on_action(ClipAction::Nudge(35));
        let ClipEffect::Error(text) = e.on_action(ClipAction::SelectIn) else { panic!("past Out was accepted") };
        assert_eq!(text, "Playhead is past Out: set Out first");
        assert_eq!((e.view().start, e.view().end, e.view().handle), (10, 40, Handle::Playhead));
        assert_eq!(e.shown_frame(), Some(45));
        e.on_action(ClipAction::SelectOut);
        assert_eq!(e.view().end, 46);

        let mut e = cropped_capture();
        e.on_action(ClipAction::Nudge(0));
        let ClipEffect::Error(text) = e.on_action(ClipAction::SelectOut) else { panic!("at In was accepted") };
        assert_eq!(text, "Playhead is at or before In: set In first");
        assert_eq!((e.view().start, e.view().end, e.view().handle), (10, 40, Handle::Playhead));
        e.on_action(ClipAction::Nudge(-5));
        let ClipEffect::Error(text) = e.on_action(ClipAction::SelectOut) else { panic!("before In was accepted") };
        assert_eq!(text, "Playhead is at or before In: set In first");
        assert_eq!((e.view().start, e.view().end, e.view().handle), (10, 40, Handle::Playhead));
        e.on_action(ClipAction::Nudge(34));
        e.on_action(ClipAction::SelectIn);
        assert_eq!(e.view().start, 39);
    }

    fn a_cut_here_in_mid_action_then_y_then_x_works() {
        let mut e = snap_capture();
        e.on_action(ClipAction::Nudge(30));
        e.on_action(ClipAction::SelectIn);
        assert_eq!(e.view().start, 30);
        assert!(e.view().move_start.is_none());
        let ClipEffect::Error(text) = e.on_action(ClipAction::TogglePlay) else { panic!("preview accepted mid-action") };
        assert_eq!(text, "In is mid-action. Snap to a move start before previewing or saving.");
        assert!(!matches!(e.on_action(ClipAction::Snap), ClipEffect::Error(_)));
        assert_eq!(e.view().move_start.unwrap().action_frame, 25);
        assert!(matches!(e.on_action(ClipAction::TogglePlay), ClipEffect::Preview(_)));
    }

    fn x_and_a_end_the_browse_and_a_refusal_keeps_it() {
        let mut e = snap_capture();
        assert!(!matches!(e.on_action(ClipAction::Nudge(20)), ClipEffect::Error(_)));
        assert!(matches!(e.on_action(ClipAction::TogglePlay), ClipEffect::Preview(_)));
        assert_eq!(e.shown_frame(), None);
        e.preview_progress(60);
        e.on_action(ClipAction::SelectIn);
        assert_eq!((e.view().handle, e.view().start), (Handle::In, 0));

        let mut e = snap_capture();
        e.on_action(ClipAction::Nudge(20));
        assert!(matches!(e.on_action(ClipAction::Confirm), ClipEffect::Export(_)));
        e.on_action(ClipAction::SelectIn);
        assert_eq!((e.view().handle, e.view().start), (Handle::In, 0));

        let mut e = snap_capture();
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::Nudge(30));
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::Nudge(-18));
        assert!(matches!(e.on_action(ClipAction::TogglePlay), ClipEffect::Error(_)));
        assert_eq!(e.shown_frame(), Some(12));
        e.on_action(ClipAction::SelectIn);
        assert_eq!(e.view().start, 12);
    }

    fn combos_and_drills_have_no_free_playhead() {
        let mut e = combo_editor();
        let ClipEffect::Error(text) = e.on_action(ClipAction::Nudge(1)) else { panic!("combo browsed") };
        assert_eq!(text, "Press L for the start or R for the end, then hold Left or Right");
        let ClipEffect::Error(text) = e.on_action(ClipAction::Snap) else { panic!("combo snapped") };
        assert_eq!(text, "Press L for the start or R for the end, then Y to snap");
        assert_eq!(e.shown_frame(), None);
        assert!(!e.view().scrubs());
        e.on_action(ClipAction::SelectIn);
        assert_eq!(e.shown_frame(), None);
        assert!(e.view().scrubs());
    }

    fn the_key_line_names_browse_cut_here_and_snap() {
        let mut e = snap_capture();
        let help = clip_help(&e.view(), "ZL/ZR", "ZL", 0);
        assert_eq!(help.primary, "Hold Left or Right to browse the take, or Y for the nearest move.");
        assert_eq!(help.follow, "Press L for the start, or R for the end. Hold ZL/ZR to start fast.");
        assert!(help.primary.chars().count() <= 75 && help.follow.chars().count() <= 75);
        e.on_action(ClipAction::Nudge(1));
        let help = clip_help(&e.view(), "ZL/ZR", "ZL", 0);
        assert_eq!(help.primary, "Playhead moved. L sets the start here, R the end.");
        assert_eq!(help.follow, "Left or Right browses on. Y goes to the nearest move.");
        assert!(help.primary.chars().count() <= 75 && help.follow.chars().count() <= 75);
        let help = clip_help(&combo_editor().view(), "ZL/ZR", "ZL", 0);
        assert_eq!(help.primary, "Press L for the start, or R for the end.");
        assert_eq!(help.follow, "Then hold Left or Right. Hold ZL/ZR to start fast.");
    }

    fn held_repeat_runs_for_a_browsing_playhead_and_never_during_a_preview() {
        let mut e = snap_capture();
        assert!(e.view().scrubs());
        e.on_action(ClipAction::SelectIn);
        assert!(e.view().scrubs());
        assert!(matches!(e.on_action(ClipAction::TogglePlay), ClipEffect::Preview(_)));
        assert!(!e.view().scrubs());
        let source = include_str!("../../runtime/macros/replay_capture.rs");
        let playing = source.split_once("if clip_view().is_some_and(|v| v.playing) {").unwrap().1
            .split_once("return;").unwrap().0;
        assert!(playing.contains("*HOLD.lock().unwrap() = (0, 0, false);"));
        assert_eq!(source.matches("view.scrubs()").count(), 2);
        assert!(!source.contains("view.handle != Handle::Playhead"));
    }

    fn long_action_capture(last: u32) -> ClipEditor {
        let frames = (0..60).map(|i| ReplayFrame {
            input: InputFrame { buttons: u32::from(i == 5 || i == 36), ..InputFrame::default() },
            start: StartState { status: if (8..=last).contains(&i) { 45 } else { 0 },
                ..StartState::neutral(74, 0, 1.0) }, x: 0.0, y: 0.0,
        }).collect();
        ClipEditor::new(ReplaySession { tracks: vec![ReplayTrack {
            entry: 0, fighter: None, quantized_analog: false, frames,
        }], truncated: false, breaks: Vec::new() }, vec![0, 14]).unwrap().with_move_rules(snap_rules())
    }

    fn second_y_does_not_reselect_a_different_mark_near_the_action_frame() {
        // A later press in the same long move is farther from its start than the preceding idle mark.
        let mut e = long_action_capture(40);
        assert_eq!(e.marks.list().iter().map(|m| m.start).collect::<Vec<_>>(), [5, 36]);
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::Nudge(36));
        assert!(matches!(e.on_action(ClipAction::Snap), ClipEffect::StopPreview));
        let selected = e.view().move_start.unwrap();
        assert_eq!((selected.frame, selected.action_frame), (7, 8));
        assert!(matches!(e.on_action(ClipAction::Snap), ClipEffect::StopPreview));
        assert_eq!((e.view().start, e.view().move_start), (7, Some(selected)));
    }

    fn y_checks_the_in_applied_by_a_shortened_out() {
        let mut e = long_action_capture(32);
        e.on_action(ClipAction::SelectOut);
        e.on_action(ClipAction::Nudge(-30));
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::Nudge(29));
        assert_eq!(e.marks.snap_in(29), Some(36)); // clean, but outside [In, Out)
        assert!(matches!(e.on_action(ClipAction::Snap), ClipEffect::StopPreview));
        assert_eq!(e.view().end, 30);
        assert!(matches!(e.on_action(ClipAction::TogglePlay), ClipEffect::Preview(_)), "Y left the applied In mid-action");
        e.preview_progress(30);
        assert!(matches!(e.on_action(ClipAction::Confirm), ClipEffect::Export(_)));
    }

    fn a_refused_x_leaves_the_watched_mark_available_to_y() {
        let mut e = long_action_capture(32);
        assert!(matches!(e.on_action(ClipAction::TogglePlay), ClipEffect::Preview(_)));
        e.watch(&[Span { start: 28, ..Span::default() }]);
        e.preview_progress(60);
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::Nudge(28));
        assert_eq!(e.marks.snap_in(28), Some(28));
        assert!(matches!(e.on_action(ClipAction::TogglePlay), ClipEffect::Error(_)));
        assert_eq!(e.marks.snap_in(28), Some(28), "no preview started; its mark should still be available to Y");
        assert!(matches!(e.on_action(ClipAction::Snap), ClipEffect::StopPreview));
        assert_eq!(e.view().move_start.unwrap().action_frame, 8);
    }

    /// Operator 2026-11-25 (install 5): "I used snap but it still gives that message". Y on In, then X, then A.
    fn y_snap_on_a_mid_action_in_previews_and_saves() {
        let mut e = snap_capture();
        assert_eq!(e.marks.list().len(), 1);
        assert_eq!(e.marks.list()[0].start, 23);
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::Nudge(30));
        assert_eq!(e.view().start, 30);
        assert!(!matches!(e.on_action(ClipAction::Snap), ClipEffect::Error(_)));
        assert!(e.view().error.is_none());
        let want = snap_rules().snap(e.track(), 23).unwrap();
        assert_eq!(want.action_frame, 25);
        assert_eq!(e.view().move_start, Some(want));
        assert_eq!(e.view().start, want.frame);
        assert!(e.view().clean_start);
        let inputs: Vec<_> = e.track().frames[want.frame as usize..60].iter().map(|frame| frame.input).collect();
        let ClipEffect::Preview(preview) = e.on_action(ClipAction::TogglePlay) else { panic!("preview refused") };
        assert_eq!(preview.frames, inputs);
        e.preview_progress(60 - want.frame);
        let ClipEffect::Export(saved) = e.on_action(ClipAction::Confirm) else { panic!("save refused") };
        assert_eq!(saved.frames, preview.frames);
        assert_eq!(saved.start, preview.start);
    }

    /// Operator 2026-11-25 (install 5): X refuses, then "I used snap but it still gives that message".
    fn y_after_a_refused_preview_snaps_previews_and_saves() {
        let mut e = snap_capture();
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::Nudge(30));
        let ClipEffect::Error(text) = e.on_action(ClipAction::TogglePlay) else { panic!("preview did not refuse") };
        assert_eq!(text, "In is mid-action. Snap to a move start before previewing or saving.");
        assert_eq!(e.view().handle, Handle::In);
        let ClipEffect::Error(saved_error) = e.on_action(ClipAction::Confirm) else { panic!("save did not refuse") };
        assert_eq!(saved_error, text);
        assert_eq!(e.view().handle, Handle::In);
        assert!(!matches!(e.on_action(ClipAction::Snap), ClipEffect::Error(_)));
        let want = snap_rules().snap(e.track(), 23).unwrap();
        assert_eq!(e.view().move_start, Some(want));
        assert!(matches!(e.on_action(ClipAction::TogglePlay), ClipEffect::Preview(_)));
        e.preview_progress(60 - want.frame);
        assert!(matches!(e.on_action(ClipAction::Confirm), ClipEffect::Export(_)));
    }

    fn a_second_y_keeps_the_same_move() {
        let mut e = snap_capture();
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::Nudge(30));
        assert!(!matches!(e.on_action(ClipAction::Snap), ClipEffect::Error(_)));
        let want = snap_rules().snap(e.track(), 23).unwrap();
        assert_eq!(e.view().move_start, Some(want));
        let first = (e.view().start, e.view().move_start);
        for _ in 0..2 {
            assert!(!matches!(e.on_action(ClipAction::Snap), ClipEffect::Error(_)));
            assert_eq!((e.view().start, e.view().move_start), first);
        }
    }

    fn y_snap_to_an_idle_mark_stays_a_plain_start() {
        let mut e = snap_capture_with(0, 45, 0);
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::Nudge(30));
        e.on_action(ClipAction::Snap);
        assert_eq!(e.view().start, 23);
        assert!(e.view().move_start.is_none());
        let ClipEffect::Preview(preview) = e.on_action(ClipAction::TogglePlay) else { panic!("preview refused") };
        assert_eq!(preview.start.status, 0);
    }

    fn y_snap_where_no_move_can_start_says_why() {
        let mut e = snap_capture_with(99, 99, 99);
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::Nudge(30));
        let ClipEffect::Error(text) = e.on_action(ClipAction::Snap) else { panic!("snap did not refuse") };
        assert_eq!(text, snap_rules().snap(e.track(), 23).unwrap_err());
        assert!(!text.contains("mid-action"));
        assert_eq!(e.view().start, 23);
        assert!(e.view().move_start.is_none());
        assert_eq!(e.view().error, Some(text));
    }

    /// The workaround given for install 5: ZL/ZR + Y, then X, then A. It passes before the fix too.
    fn the_lead_in_key_then_x_and_a_work_on_the_same_take() {
        let mut e = snap_capture();
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::Nudge(30));
        assert!(matches!(e.on_action(ClipAction::TogglePlay), ClipEffect::Error(_)));
        assert!(!matches!(e.on_action(ClipAction::UseLeadIn), ClipEffect::Error(_)));
        let want = snap_rules().snap(e.track(), 30).unwrap();
        assert_eq!((want.frame, want.action_frame), (19, 25));
        assert_eq!(e.view().move_start, Some(want));
        let ClipEffect::Preview(preview) = e.on_action(ClipAction::TogglePlay) else { panic!("preview refused") };
        e.preview_progress(60 - want.frame);
        let ClipEffect::Export(saved) = e.on_action(ClipAction::Confirm) else { panic!("save refused") };
        assert_eq!(saved.frames, preview.frames);
    }

    fn y_snap_on_a_saved_macro_is_unchanged() {
        let mut e = saved_macro(false);
        assert!(!e.marks.list().is_empty());
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::Nudge(2));
        let boundary = e.marks.snap_in(2).unwrap();
        assert!(matches!(e.on_action(ClipAction::Snap), ClipEffect::StopPreview));
        assert_eq!(e.view().start, boundary);
    }

    fn move_snap_previews_and_exports_the_same_local_inputs_and_is_idempotent() {
        use {crate::engine::macros::MoveFamily, crate::engine::macros::MoveStartRules};
        let mut frames: Vec<_> = (0..40).map(|i| ReplayFrame {
            input: InputFrame { buttons: i, ..Default::default() },
            start: StartState { status: match i { 0 => 0, 28..=29 => 6,
                30..=31 => 7, 32 => 45, 33 => 46, 34..=39 => 47, _ => 486 },
                ..StartState::neutral(74, 0, 1.0) }, x: 0.0, y: 0.0,
        }).collect();
        // Keep original input bytes as the independent crop oracle.
        frames[27].input.lstick_x = -127;
        let wanted: Vec<_> = frames[27..38].iter().map(|f| f.input).collect();
        let mut e = ClipEditor::new(ReplaySession { tracks: vec![ReplayTrack {
            fighter: None,
            entry: 1, frames, quantized_analog: false }], truncated: false, breaks: Vec::new() }, vec![0, 14]).unwrap()
            .with_move_rules(MoveStartRules { families: vec![MoveFamily { fighter: None,
                statuses: vec![45, 46, 47], buttons: 0 }], setup: vec![6, 7], ground: 0, air: 2,
                wait: 0, fall: 14, trigger_buttons: 0 });
        e.window.set_in(35); e.window.set_out(38);
        assert!(matches!(e.on_action(ClipAction::TogglePlay), ClipEffect::Error(_)));
        e.on_action(ClipAction::UseLeadIn);
        assert_eq!((e.view().start, e.view().end), (27, 38));
        assert!(clip_help(&e.view(), "ZL/ZR", "ZL", 1).notice.contains("5f setup"));
        e.on_action(ClipAction::UseLeadIn);
        assert_eq!(e.view().start, 27);
        let ClipEffect::Preview(preview) = e.on_action(ClipAction::TogglePlay) else { panic!("preview refused") };
        assert_eq!(preview.frames, wanted);
        assert_eq!(preview.start.status, 0);
        e.preview_progress(6); assert_eq!(e.view().cursor, 32);
        e.preview_progress(11); assert!(!e.view().playing);
        let ClipEffect::Export(saved) = e.on_action(ClipAction::Confirm) else { panic!("save refused") };
        assert_eq!(saved.frames, preview.frames);
        assert_eq!(saved.start, preview.start);
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::Nudge(1));
        assert!(e.view().move_start.is_none());
        assert!(matches!(e.on_action(ClipAction::TogglePlay), ClipEffect::Error(_)));
    }

    fn saved_macro(player: bool) -> ClipEditor {
        let replay = editor();
        let recording = MacroRecording::new("Recorded", "13.0.5", replay.track().frames[0].start,
            replay.track().frames.iter().map(|frame| frame.input).collect()).unwrap();
        ClipEditor::macro_file(recording, player).unwrap()
    }

    fn b_stops_a_clip_preview_without_leaving() {
        for mut e in [editor(), saved_macro(false), saved_macro(true)] {
            e.on_action(ClipAction::SelectIn);
            e.on_action(ClipAction::Nudge(2));
            for pad in [Pad::Back, Pad::Extra] {
                assert!(matches!(e.on_action(ClipAction::TogglePlay), ClipEffect::Preview(_) | ClipEffect::PreviewFile { .. }));
                e.preview_cursor(4);
                assert!(e.view().playing);
                let action = e.action_for_pad(pad).unwrap();
                assert!(matches!(e.on_action(action), ClipEffect::StopPreview));
                assert!(!e.view().playing);
                assert_eq!(e.view().cursor, e.view().start);
            }
        }
    }

    fn a_second_b_leaves_the_editor() {
        for mut e in [editor(), saved_macro(false), saved_macro(true)] {
            let back = e.action_for_pad(Pad::Back).unwrap();
            assert!(matches!(e.on_action(back), ClipEffect::Close));
            assert!(matches!(e.on_action(ClipAction::TogglePlay), ClipEffect::Preview(_) | ClipEffect::PreviewFile { .. }));
            let view = e.view();
            e.preview_cursor(view.end - view.start);
            assert_eq!(e.view().cursor, view.end - 1);
            assert!(e.view().playing);
            let back = e.action_for_pad(Pad::Back).unwrap();
            assert!(matches!(e.on_action(back), ClipEffect::StopPreview));
            assert!(!e.view().playing);
            let back = e.action_for_pad(Pad::Back).unwrap();
            assert!(matches!(e.on_action(back), ClipEffect::Close));
            assert!(matches!(e.on_action(ClipAction::TogglePlay), ClipEffect::Preview(_) | ClipEffect::PreviewFile { .. }));
            let back = e.action_for_pad(Pad::Back).unwrap();
            assert!(matches!(e.on_action(back), ClipEffect::StopPreview));
            let back = e.action_for_pad(Pad::Back).unwrap();
            assert!(matches!(e.on_action(back), ClipEffect::Close));
        }
    }

    fn replay_and_saved_macro_controls_have_parity() {
        for mut e in [editor(), saved_macro(false), saved_macro(true)] {
            let macro_file = matches!(e.context(), ClipContext::Macro { .. });
            e.on_action(ClipAction::SelectIn);
            e.on_action(ClipAction::Nudge(2));
            e.on_action(ClipAction::SelectOut);
            e.on_action(ClipAction::Nudge(-3));
            assert_eq!((e.view().start, e.view().end), (2, 7));
            let wanted = match &e.file {
                Some(FileSource::Macro { recording, .. }) => recording.frames[2..7].to_vec(),
                _ => e.track().frames[2..7].iter().map(|frame| frame.input).collect(),
            };
            let preview = e.on_action(ClipAction::TogglePlay);
            if macro_file {
                assert!(matches!(preview, ClipEffect::PreviewFile { start: 2, end: 7, .. }));
            } else {
                let ClipEffect::Preview(preview) = preview else { panic!("preview") };
                assert_eq!(preview.frames.iter().map(|frame| frame.buttons).collect::<Vec<_>>(), vec![2, 3, 4, 5, 6]);
            }
            e.on_action(ClipAction::TogglePlay);
            let ClipEffect::Export(saved) = e.on_action(ClipAction::Confirm) else { panic!("export") };
            assert_eq!(saved.frames, wanted);
            assert!(matches!(e.on_action(ClipAction::Cancel), ClipEffect::Close));
        }
    }

    /// A saved macro previews its kept window on the role it was opened for.
    fn a_saved_macro_previews_its_kept_window_on_its_role() {
        let mut e = saved_macro(true);
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::Nudge(2));
        e.on_action(ClipAction::SelectOut);
        e.on_action(ClipAction::Nudge(-3));
        assert!(matches!(e.on_action(ClipAction::TogglePlay), ClipEffect::PreviewFile { start: 2, end: 7, .. }));
        assert!(e.view().playing);
        assert_eq!(e.context(), ClipContext::Macro { player: true });
        assert_eq!(e.view().entry, 0);
        assert!(matches!(e.on_action(ClipAction::TogglePlay), ClipEffect::StopPreview));
        assert!(!e.view().playing);
        e.on_action(ClipAction::TogglePlay);
        e.preview_progress(u32::MAX);
        assert!(!e.view().playing);
    }

    fn a_replay_preview_stays_playing() {
        let mut e = editor();
        assert!(matches!(e.on_action(ClipAction::TogglePlay), ClipEffect::Preview(_)));
        assert!(e.view().playing);
    }

    /// The tail starts when the last input finishes and resets on a restart.
    fn the_preview_tail_waits_sixty_frames_after_the_last_input() {
        let mut tail = PreviewTail::new();
        assert!(!tail.over(true, 100));
        assert!(!tail.over(false, 110));
        assert!(!tail.over(false, 169));
        assert!(tail.over(false, 170));
        assert!(!tail.over(true, 171));
        assert!(!tail.over(false, 172));
        tail.restart();
        assert!(!tail.over(false, 500));
        assert!(tail.over(false, 560));
    }

    fn combo_editor() -> ClipEditor {
        use crate::model::{combo::{Combo, LinkRec}, combo_record::ComboRecord, frame_strip::{Move, Verdict}};
        let moves = [(Move::Aerial, Move::SideTilt), (Move::SideTilt, Move::NeutralSpecial), (Move::NeutralSpecial, Move::Aerial)];
        let combo = Combo::from_links(moves.map(|(from, to)| LinkRec::presses(from, to, Verdict::True(None), None, &[]))).unwrap();
        ClipEditor::combo_file(ComboRecord::new("Combo source".into(), combo, Some("Mario".into()))).unwrap()
    }

    fn combo_clip_handles_snap_to_whole_links() {
        let mut e = combo_editor();
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::Nudge(1));
        e.on_action(ClipAction::SelectOut);
        e.on_action(ClipAction::Nudge(-1));
        assert_eq!((e.view().start, e.view().end), (1, 2));
        let ClipEffect::ExportCombo(saved) = e.on_action(ClipAction::Confirm) else { panic!("combo") };
        assert_eq!(saved.combo.len(), 1);
        assert_eq!(saved.combo.get(0).unwrap().from, crate::model::frame_strip::Move::SideTilt);
        assert_eq!(saved.recorded_on.as_deref(), Some("Mario"));
        // The source and its three links are retained for a second crop.
        assert_eq!(e.view().total, 3);
        e.on_action(ClipAction::Nudge(-100));
        assert_eq!((e.view().start, e.view().end), (1, 2));
    }

    fn combo_clip_has_no_preview_action_or_help() {
        let mut e = combo_editor();
        let before = (e.view().start, e.view().end, e.view().handle);
        for action in [ClipAction::TogglePlay, ClipAction::Restart] {
            assert!(matches!(e.on_action(action), ClipEffect::None));
            assert!(!e.view().playing);
            assert!(e.view().error.is_none());
        }
        assert_eq!((e.view().start, e.view().end, e.view().handle), before);
        let help = clip_help(&e.view(), "ZL/ZR", "ZL", 1);
        assert_eq!(help.controls, "Y snap. A save. B close.");
        assert!(!help.notice.contains("preview"));
    }

    fn file_clip_single_side_does_not_change_selection() {
        for mut e in [saved_macro(false), saved_macro(true), combo_editor()] {
            e.on_action(ClipAction::SelectIn);
            e.on_action(ClipAction::Nudge(1));
            for action in [ClipAction::NextFighter, ClipAction::PreviousFighter, ClipAction::ToggleCapture] {
                assert!(e.peek_selection(action).is_none());
                assert!(matches!(e.on_action(action), ClipEffect::None));
                assert_eq!(e.view().start, 1);
            }
        }
    }

    /// One native P1 mapping sample of physical A: (held level, release edge, press edge).
    /// Returns what the editor would do with the Confirm slot of that sample.
    fn poll_a(e: &mut ClipEditor, held: bool, released: bool, pressed: bool) -> Option<ClipEffect> {
        e.allows_confirm(held, released, pressed).then(|| e.on_action(ClipAction::Confirm))
    }
    fn exported(effect: Option<ClipEffect>) -> Option<Vec<u32>> {
        match effect {
            Some(ClipEffect::Export(recording)) => Some(recording.frames.iter().map(|f| f.buttons).collect()),
            _ => None,
        }
    }
    fn normalized_pad_cannot_bypass_or_clear_opening_press_ownership() {
        let mut e = editor();
        for _ in 0..8 { assert!(e.action_for_pad(Pad::Confirm).is_none()); }
        assert!(e.confirm_release_pending());
        let preview = e.action_for_pad(Pad::Extra).unwrap();
        assert!(matches!(e.on_action(preview), ClipEffect::Preview(_)));
        let cancel = e.action_for_pad(Pad::Back).unwrap();
        assert!(matches!(e.on_action(cancel), ClipEffect::StopPreview));
        assert!(e.confirm_release_pending());
        assert!(poll_a(&mut e, false, false, false).is_none());
        let confirm = e.action_for_pad(Pad::Confirm).unwrap();
        assert_eq!(exported(Some(e.on_action(confirm))).unwrap(), (0..10).collect::<Vec<_>>());
    }
    fn editor() -> ClipEditor {
        let frames = (0..10).map(|i| ReplayFrame { input: InputFrame { buttons: i, ..InputFrame::default() },
            start: StartState { status: if i == 4 { 99 } else { 14 }, ..StartState::neutral(18, 0, 1.0) },
            x: i as f32, y: 0.0 }).collect();
        ClipEditor::new(ReplaySession { tracks: vec![ReplayTrack { fighter: None, entry: 1, frames,
            quantized_analog: false }], truncated: false, breaks: Vec::new() }, vec![14]).unwrap()
    }
    fn invalid_start_requires_explicit_lead_in_and_keeps_selection() {
        let mut e = editor();
        e.on_action(ClipAction::SelectIn); e.on_action(ClipAction::Nudge(4));
        assert!(matches!(e.on_action(ClipAction::Confirm), ClipEffect::Error(_)));
        assert!(matches!(e.on_action(ClipAction::TogglePlay), ClipEffect::Error(_)));
        assert!(!e.view().playing);
        assert_eq!(e.view().start, 4);
        let feedback = clip_help(&e.view(), "ZL/ZR", "ZL", 0);
        assert!(feedback.primary.contains("mid-action"));
        assert_eq!(feedback.primary, "In is mid-action. Snap to a move start before previewing or saving.");
        assert!(feedback.primary.chars().count() <= 75 && feedback.follow.chars().count() <= 75);
        assert!(feedback.notice.contains("ZL+Y") && feedback.notice.contains("X to preview"));
        e.on_action(ClipAction::UseLeadIn);
        assert_eq!(e.view().start, 3);
        assert!(e.view().error.is_none());
        let ClipEffect::Preview(preview) = e.on_action(ClipAction::TogglePlay) else { panic!() };
        assert_eq!(preview.frames.iter().map(|f| f.buttons).collect::<Vec<_>>(), (3..10).collect::<Vec<_>>());
        let ClipEffect::Export(recording) = e.on_action(ClipAction::Confirm) else { panic!() };
        assert_eq!(recording.frames.iter().map(|f| f.buttons).collect::<Vec<_>>(), (3..10).collect::<Vec<_>>());
    }
    fn saved_capture_name_survives_crop_without_exceeding_macro_name_limit() {
        let mut e = editor().with_name("試合".repeat(40));
        e.on_action(ClipAction::SelectIn);
        e.on_action(ClipAction::Nudge(3));
        let ClipEffect::Export(recording) = e.on_action(ClipAction::Confirm) else { panic!() };
        assert_eq!(recording.name.chars().count(), MAX_NAME_CHARS);
        assert!(recording.name.ends_with(" P2"));
        assert!(recording.name.starts_with("試合"));
        assert_eq!(recording.frames.iter().map(|f| f.buttons).collect::<Vec<_>>(), (3..10).collect::<Vec<_>>());
    }

    fn unequal_tracks() -> ClipEditor {
        let mut session = editor().session;
        let mut short = session.tracks[0].clone();
        short.entry = 0;
        short.frames.truncate(3);
        for sample in &mut short.frames { sample.start.fighter_kind = 8; }
        session.tracks.push(short);
        ClipEditor::new(session, vec![14]).unwrap()
    }

    fn automatic_cpu_selection_and_full_range_fighter_switch_keep_all_inputs() {
        let mut e = unequal_tracks();
        assert_eq!(e.view().end, 3);
        e.select_kind(18);
        let ClipEffect::Export(recording) = e.on_action(ClipAction::Confirm) else { panic!() };
        assert_eq!(recording.frames.iter().map(|frame| frame.buttons).collect::<Vec<_>>(),
                   (0..10).collect::<Vec<_>>());
        e.on_action(ClipAction::NextFighter);
        assert_eq!((e.view().start, e.view().end), (0, 3));
        e.on_action(ClipAction::NextFighter);
        assert_eq!((e.view().start, e.view().end), (0, 10));
    }

    fn peeking_a_selection_names_the_candidate_and_changes_nothing() {
        let mut e = unequal_tracks();
        let view = e.view();
        let before = (view.entry, view.fighter_kind, view.start, view.end);
        // Track 0 is entry 0 (fighter 8); the other is entry 1 (fighter 18).
        assert_eq!(e.peek_selection(ClipAction::NextFighter), Some((1, 18, false)));
        assert_eq!(e.peek_selection(ClipAction::PreviousFighter), Some((1, 18, false)));
        let view = e.view();
        assert_eq!((view.entry, view.fighter_kind, view.start, view.end), before);
        // Only a track or capture change selects anything.
        assert!(e.peek_selection(ClipAction::Confirm).is_none());
        assert!(e.peek_selection(ClipAction::ToggleCapture).is_none());
        assert!(editor().peek_selection(ClipAction::NextFighter).is_none(), "one track has nothing to re-select");
        // The peek agrees with the real selection.
        e.on_action(ClipAction::NextFighter);
        assert_eq!((e.view().entry, e.view().fighter_kind), (1, 18));
    }

    fn peeking_the_capture_toggle_names_the_previous_capture_without_swapping() {
        let mut previous = editor().session;
        previous.tracks[0].entry = 0;
        for sample in &mut previous.tracks[0].frames { sample.start.fighter_kind = 8; }
        let mut e = editor().with_previous(Some(previous));
        assert_eq!(e.peek_selection(ClipAction::ToggleCapture), Some((0, 8, true)));
        assert!(!e.view().previous_selected);
        assert_eq!((e.view().entry, e.view().fighter_kind), (1, 18));
        e.on_action(ClipAction::ToggleCapture);
        assert!(e.view().previous_selected);
        assert_eq!((e.view().entry, e.view().fighter_kind), (0, 8));
        // From the previous capture, the same toggle names the latest one.
        assert_eq!(e.peek_selection(ClipAction::ToggleCapture), Some((1, 18, false)));
    }

    fn holding_left_moves_the_end_instead_of_the_fighter() {
        let mut editor = unequal_tracks();
        editor.on_action(ClipAction::SelectOut);
        let before = editor.view().fighter_kind;
        let Some(ClipAction::Nudge(delta)) = direction_action(true, false, true, false, true, false) else {
            panic!("a diagonal left press must edit the crop");
        };
        assert_eq!(delta, -1);
        editor.on_action(ClipAction::Nudge(delta));
        assert_eq!(editor.view().fighter_kind, before);
        assert!(editor.view().end < editor.view().total);
        assert!(direction_action(false, false, true, false, true, false).is_none());
        assert!(matches!(
            direction_action(false, false, true, false, false, false),
            Some(ClipAction::PreviousFighter)
        ));
        assert!(matches!(
            direction_action(false, false, false, true, false, false),
            Some(ClipAction::NextFighter)
        ));
    }

    fn held_scrub_speeds_up_and_the_fast_modifier_starts_ahead() {
        assert_eq!(scrub_repeat_step(0, false), 0);
        assert_eq!(scrub_repeat_step(9, false), 0);
        let early = scrub_repeat_step(12, false);
        let middle = scrub_repeat_step(80, false);
        let late = scrub_repeat_step(200, false);
        assert!(early < middle && middle < late);
        assert_eq!(late, 180);
        assert!(scrub_repeat_step(12, true) > early);
        let Some(ClipAction::Nudge(fast)) = direction_action(false, true, false, false, true, true) else {
            panic!("right press");
        };
        assert!(fast > 1);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("the_playhead_browses_the_whole_take_and_leaves_in_and_out", the_playhead_browses_the_whole_take_and_leaves_in_and_out as fn()),
            ("y_snaps_the_playhead_to_the_nearest_mark", y_snaps_the_playhead_to_the_nearest_mark as fn()),
            ("l_and_r_cut_here_after_the_playhead_moved", l_and_r_cut_here_after_the_playhead_moved as fn()),
            ("l_and_r_only_select_when_the_playhead_has_not_moved", l_and_r_only_select_when_the_playhead_has_not_moved as fn()),
            ("cut_here_never_reorders_the_handles", cut_here_never_reorders_the_handles as fn()),
            ("a_cut_here_in_mid_action_then_y_then_x_works", a_cut_here_in_mid_action_then_y_then_x_works as fn()),
            ("x_and_a_end_the_browse_and_a_refusal_keeps_it", x_and_a_end_the_browse_and_a_refusal_keeps_it as fn()),
            ("combos_and_drills_have_no_free_playhead", combos_and_drills_have_no_free_playhead as fn()),
            ("the_key_line_names_browse_cut_here_and_snap", the_key_line_names_browse_cut_here_and_snap as fn()),
            ("held_repeat_runs_for_a_browsing_playhead_and_never_during_a_preview", held_repeat_runs_for_a_browsing_playhead_and_never_during_a_preview as fn()),
            ("b_stops_a_clip_preview_without_leaving", b_stops_a_clip_preview_without_leaving as fn()),
            ("a_second_b_leaves_the_editor", a_second_b_leaves_the_editor as fn()),
            ("a_saved_macro_previews_its_kept_window_on_its_role", a_saved_macro_previews_its_kept_window_on_its_role as fn()),
            ("a_replay_preview_stays_playing", a_replay_preview_stays_playing as fn()),
            ("the_preview_tail_waits_sixty_frames_after_the_last_input", the_preview_tail_waits_sixty_frames_after_the_last_input as fn()),
            ("replay_and_saved_macro_controls_have_parity", replay_and_saved_macro_controls_have_parity as fn()),
            ("combo_clip_handles_snap_to_whole_links", combo_clip_handles_snap_to_whole_links as fn()),
            ("combo_clip_has_no_preview_action_or_help", combo_clip_has_no_preview_action_or_help as fn()),
            ("file_clip_single_side_does_not_change_selection", file_clip_single_side_does_not_change_selection as fn()),
            ("move_snap_previews_and_exports_the_same_local_inputs_and_is_idempotent", move_snap_previews_and_exports_the_same_local_inputs_and_is_idempotent as fn()),
            ("y_snap_on_a_mid_action_in_previews_and_saves", y_snap_on_a_mid_action_in_previews_and_saves as fn()),
            ("y_after_a_refused_preview_snaps_previews_and_saves", y_after_a_refused_preview_snaps_previews_and_saves as fn()),
            ("a_second_y_keeps_the_same_move", a_second_y_keeps_the_same_move as fn()),
            ("second_y_does_not_reselect_a_different_mark_near_the_action_frame", second_y_does_not_reselect_a_different_mark_near_the_action_frame as fn()),
            ("y_checks_the_in_applied_by_a_shortened_out", y_checks_the_in_applied_by_a_shortened_out as fn()),
            ("a_refused_x_leaves_the_watched_mark_available_to_y", a_refused_x_leaves_the_watched_mark_available_to_y as fn()),
            ("y_snap_to_an_idle_mark_stays_a_plain_start", y_snap_to_an_idle_mark_stays_a_plain_start as fn()),
            ("y_snap_where_no_move_can_start_says_why", y_snap_where_no_move_can_start_says_why as fn()),
            ("the_lead_in_key_then_x_and_a_work_on_the_same_take", the_lead_in_key_then_x_and_a_work_on_the_same_take as fn()),
            ("y_snap_on_a_saved_macro_is_unchanged", y_snap_on_a_saved_macro_is_unchanged as fn()),
            ("normalized_pad_cannot_bypass_or_clear_opening_press_ownership", normalized_pad_cannot_bypass_or_clear_opening_press_ownership as fn()),
            ("invalid_start_requires_explicit_lead_in_and_keeps_selection", invalid_start_requires_explicit_lead_in_and_keeps_selection as fn()),
            ("saved_capture_name_survives_crop_without_exceeding_macro_name_limit", saved_capture_name_survives_crop_without_exceeding_macro_name_limit as fn()),
            ("automatic_cpu_selection_and_full_range_fighter_switch_keep_all_inputs", automatic_cpu_selection_and_full_range_fighter_switch_keep_all_inputs as fn()),
            ("peeking_a_selection_names_the_candidate_and_changes_nothing", peeking_a_selection_names_the_candidate_and_changes_nothing as fn()),
            ("peeking_the_capture_toggle_names_the_previous_capture_without_swapping", peeking_the_capture_toggle_names_the_previous_capture_without_swapping as fn()),
            ("holding_left_moves_the_end_instead_of_the_fighter", holding_left_moves_the_end_instead_of_the_fighter as fn()),
            ("held_scrub_speeds_up_and_the_fast_modifier_starts_ahead", held_scrub_speeds_up_and_the_fast_modifier_starts_ahead as fn()),
        ]);
    }
    }
}
