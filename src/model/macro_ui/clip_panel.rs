//! Clip selection. Native input consumption, not rendering, advances the preview.
use {crate::engine::macros::FrameWindow, crate::engine::macros::MacroRecording, crate::engine::macros::MoveStart, crate::engine::macros::MoveStartRules, crate::engine::macros::ReplayFrame, crate::engine::macros::ReplaySession, crate::engine::macros::ReplayTrack, crate::engine::macros::MAX_NAME_CHARS};
use crate::engine::macro_ui::macro_panel::Pad;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle { Playhead, In, Out }
#[derive(Clone, Copy, Debug)]
pub enum ClipAction {
    SelectIn, SelectOut, Nudge(i32), TogglePlay, Restart,
    PreviousFighter, NextFighter, ToggleCapture, UseLeadIn, Confirm, Cancel,
}
pub enum ClipEffect {
    None, StopPreview, Preview(Box<MacroRecording>), Close,
    Export(Box<MacroRecording>), Error(String),
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
    let controls = "D-pad Up/Down: fighter. X preview. A save. B close.".to_owned();
    if view.playing {
        return ClipHelp {
            primary: "Previewing the selection.".to_owned(),
            follow: "Press X to stop. Press L or R to edit the crop.".to_owned(),
            controls,
            notice: String::new(),
        };
    }
    let (primary, follow) = match view.handle {
        Handle::Out => (
            "End selected. Hold Left or Right to move it.".to_owned(),
            format!("Hold longer to speed up, or hold {coarse} to start fast."),
        ),
        Handle::In => (
            "Start selected. Hold Left or Right to move it.".to_owned(),
            format!("Hold longer to speed up, or hold {coarse} to start fast."),
        ),
        Handle::Playhead => (
            "Press L for the start, or R for the end.".to_owned(),
            format!("Then hold Left or Right. Hold {coarse} to start fast."),
        ),
    };
    let follow = if pace > 1 {
        if pace >= 180 {
            format!("Moving {pace} frames per tick.")
        } else {
            format!("Moving {pace} frames per tick and speeding up.")
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

#[derive(Clone, Debug)]
pub struct ClipView {
    pub total: u32,
    pub start: u32,
    pub end: u32,
    pub cursor: u32,
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
}

pub struct ClipEditor {
    session: ReplaySession,
    previous: Option<ReplaySession>,
    previous_selected: bool,
    track: usize,
    window: FrameWindow,
    handle: Handle,
    playing: bool,
    clean_statuses: Vec<i32>,
    move_rules: Option<MoveStartRules>,
    move_start: Option<MoveStart>,
    source_name: String,
    previous_name: String,
    open_guard: OpenInputGuard,
    error: Option<String>,
}

fn prepare(mut session: ReplaySession) -> Option<ReplaySession> {
    session.tracks.retain(|track| !track.frames.is_empty());
    session.tracks.sort_by_key(|track| track.entry);
    (!session.tracks.is_empty()).then_some(session)
}

impl ClipEditor {
    pub fn new(session: ReplaySession, clean_statuses: Vec<i32>) -> Result<Self, String> {
        let session = prepare(session).ok_or("Play a saved replay in Vault first")?;
        let window = session.tracks[0].window()?;
        let mut open_guard = OpenInputGuard::new();
        open_guard.arm();
        Ok(Self { session, previous: None, previous_selected: false, track: 0, window,
            handle: Handle::Playhead, playing: false, clean_statuses, move_rules: None, move_start: None,
            source_name: "Latest Vault capture".into(), previous_name: "Previous Vault capture".into(),
            open_guard, error: None })
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
        if let Some(index) = self.session.tracks.iter().position(|t| t.frames[0].start.fighter_kind == kind) {
            self.set_track(index);
        }
    }

    /// Entry, recorded fighter kind and capture side (`true` = the previous
    /// capture) of the track `action` would select, without selecting it. Only a
    /// real change of track or capture answers; re-selecting the current track,
    /// or toggling with no previous capture, selects nothing new.
    pub fn peek_selection(&self, action: ClipAction) -> Option<(i32, i32, bool)> {
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

    pub fn view(&self) -> ClipView {
        let track = self.track();
        ClipView { total: self.window.total(), start: self.window.start(), end: self.window.end(),
            cursor: self.window.cursor(), handle: self.handle, playing: self.playing,
            entry: track.entry, fighter_kind: self.sample().start.fighter_kind,
            previous_selected: self.previous_selected, previous_available: self.previous.is_some(),
            clean_start: self.move_start.is_some() || self.clean_statuses.contains(&track.frames[self.window.start() as usize].start.status),
            move_start: self.move_start,
            quantized: track.quantized_analog, truncated: self.session.truncated,
            source_name: self.source_name.clone(), error: self.error.clone() }
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

    fn start_preview(&mut self) -> ClipEffect {
        self.pause();
        self.window.seek(self.window.start());
        self.handle = Handle::Playhead;
        match self.export_selection("Preview") {
            Ok(recording) => {
                self.playing = true;
                ClipEffect::Preview(Box::new(recording))
            }
            Err(error) => ClipEffect::Error(error),
        }
    }

    fn export_selection(&self, name: &str) -> Result<MacroRecording, String> {
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
        window.seek(self.window.cursor().min(window.total() - 1));
        self.track = index;
        self.window = window;
        self.playing = false;
        self.move_start = None;
    }

    pub fn on_action(&mut self, action: ClipAction) -> ClipEffect {
        self.error = None;
        let effect = self.apply_action(action);
        if let ClipEffect::Error(message) = &effect { self.report_error(message.clone()); }
        effect
    }

    fn apply_action(&mut self, action: ClipAction) -> ClipEffect {
        match action {
            ClipAction::SelectIn | ClipAction::SelectOut => {
                self.pause();
                let handle = if matches!(action, ClipAction::SelectIn) { Handle::In } else { Handle::Out };
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
                if let Some(previous) = &mut self.previous {
                    std::mem::swap(&mut self.session, previous);
                    std::mem::swap(&mut self.source_name, &mut self.previous_name);
                    self.previous_selected = !self.previous_selected;
                    self.track = 0;
                    self.window = self.session.tracks[0].window().expect("nonempty capture");
                    self.handle = Handle::Playhead;
                    self.move_start = None;
                    self.pause();
                }
            }
            ClipAction::UseLeadIn => {
                self.pause();
                if let Some(rules) = &self.move_rules {
                    // Repeated presses keep the same move, rather than treating
                    // its included setup as a request for the preceding move.
                    let requested = self.move_start.map_or(self.window.start(), |s| s.action_frame);
                    match rules.snap(self.track(), requested) {
                        Ok(start) if start.action_frame < self.window.end() => {
                            self.window.set_in(start.frame);
                            self.window.seek(start.frame);
                            self.handle = Handle::In;
                            self.move_start = Some(start);
                        }
                        Ok(_) => return ClipEffect::Error("Move is outside the crop; move Out later".into()),
                        Err(error) => return ClipEffect::Error(error),
                    }
                } else if let Some(start) = self.track().previous_clean_start(self.window.start(), &self.clean_statuses) {
                    self.window.set_in(start);
                    self.window.seek(start);
                    self.handle = Handle::In;
                } else { return ClipEffect::Error("No earlier idle/fall start in this capture".into()); }
            }
            ClipAction::Confirm => {
                self.pause();
                let suffix = format!(" P{}", self.track().entry + 1);
                let prefix: String = self.source_name.chars()
                    .take(MAX_NAME_CHARS.saturating_sub(suffix.chars().count())).collect();
                return match self.export_selection(&format!("{prefix}{suffix}")) {
                    Ok(recording) => ClipEffect::Export(Box::new(recording)),
                    Err(error) => ClipEffect::Error(error),
                };
            }
            ClipAction::Cancel => { self.pause(); return ClipEffect::Close; }
        }
        ClipEffect::StopPreview
    }
}

