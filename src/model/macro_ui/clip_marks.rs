use crate::engine::macros::{ActionMark, InputFrame, MacroRecording};
use crate::model::bridge_events::Clip;
use crate::model::combo::{self, Kind, STEPS};
use crate::model::combo_record::{full_move_key, move_of};
use crate::model::frame_strip::{self, Angle, Button, Move, Snap};
use crate::model::move_names::{FighterId, Namer, Normal};

pub const SHOWN_MARKS: usize = 24;
pub const WATCH_SPANS: usize = 32;
pub const CAPTURE_RUN: u32 = u32::MAX;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mark {
    pub start: u32,
    pub end: Option<u32>,
    pub name: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: Option<u32>,
    pub mv: Option<Move>,
    /// The move's variant as the tracker keeps it for a combo link (`Normal`); 0 when the animation is none of them.
    pub variant: u8,
    pub angle: Option<Angle>,
}

pub struct Marks {
    list: Vec<Mark>,
    origins: Vec<Option<String>>,
    bindings: Vec<Option<u32>>,
    keys: Vec<Option<String>>,
    /// Each mark's move, variant and angle, when its move is known.
    descs: Vec<Option<(Move, u8, Option<Angle>)>>,
    namer: Namer<'static>,
    recorded: bool,
}
impl Default for Marks {
    fn default() -> Self {
        Self { list: Vec::new(), origins: Vec::new(), bindings: Vec::new(), keys: Vec::new(), descs: Vec::new(),
            namer: Namer::OFF, recorded: false }
    }
}

/// A mark's name: today's short name with the move-name toggle Off; with it On, what the combo and drill views write
/// for the same move, variant and angle.
pub fn mark_name(namer: Namer<'_>, mv: Move, variant: u8, angle: Option<Angle>) -> String {
    if !namer.is_on() { return mv.name().into(); }
    let mut name = String::new();
    let _ = crate::model::action::ActionDesc { mv, of: None, angle, normal: variant, parts: [None; 2] }.write(&mut name, false, namer);
    name
}

/// The animation the watch reads for the fighter it follows. A sample carries one only for the acting side, so the
/// other fighter's is read, and only while its status is a named move.
pub fn watched_motion(sampled: u64, named: bool, read: impl FnOnce() -> u64) -> u64 {
    if sampled != 0 || !named { sampled } else { read() }
}

/// The three Mii fighter kinds (`mii_setup`): a Mii's specials are its own selection, not its kind's.
pub const MII_KINDS: core::ops::RangeInclusive<i32> = 0x48..=0x4a;

/// The identity a saved take's specials are named by. A take stores its fighter's kind, not the key of the names
/// table, so the live Player 1 serves when it is that kind. For a Mii that is the take's stored selection, and no
/// identity (the common names) when the take stored none.
pub fn take_identity(live: Option<(i32, FighterId)>, kind: i32, mii: Option<[u8; 4]>) -> Option<FighterId> {
    let (_, id) = live.filter(|(live, _)| *live == kind)?;
    if !MII_KINDS.contains(&kind) { return Some(id); }
    mii.map(|specials| FighterId { specials, ..id })
}

fn angle_key(angle: Angle) -> &'static str {
    match angle { Angle::Up => "up", Angle::Down => "down" }
}

fn angle_of_key(key: &str) -> Option<Angle> {
    match key { "up" => Some(Angle::Up), "down" => Some(Angle::Down), _ => None }
}

/// One frame of the watched fighter, as the frame strip sampled it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Seen {
    /// It cannot act and is not in hitstun or shieldstun.
    pub busy: bool,
    pub mv: Option<Move>,
    pub status: i32,
    pub motion_frame: f32,
    /// In hitstop: its animation does not advance.
    pub stop: bool,
    /// The animation's hash40; 0 when it was not read.
    pub motion: u64,
}

pub struct Watch {
    run: u32,
    spans: [Span; WATCH_SPANS],
    len: usize,
    open: bool,
    last: Option<u32>,
    status: i32,
    motion_frame: f32,
    /// The open span has had its variant read.
    told: bool,
}
impl Default for Watch {
    fn default() -> Self {
        Self::new()
    }
}
impl Watch {
    pub const fn new() -> Self {
        Self { run: 0, spans: [Span { start: 0, end: None, mv: None, variant: 0, angle: None }; WATCH_SPANS],
            len: 0, open: false, last: None, status: 0, motion_frame: 0.0, told: false }
    }
    /// A move that starts while the fighter is still busy is its own span, as the tracker counts it: another move, or
    /// the same status with its animation back at an earlier frame. A throw continues its grab.
    pub fn feed(&mut self, run: u32, index: u32, seen: Seen) {
        if self.run != run || self.last.is_some_and(|last| index < last) {
            *self = Self::new();
            self.run = run;
        }
        self.last = Some(index);
        if seen.busy && self.open {
            let current = self.spans[self.len - 1].mv;
            let another = matches!((current, seen.mv), (Some(was), Some(now))
                if was != now && was != Move::Other && !(was == Move::Grab && now == Move::Throw));
            let again = seen.mv.is_some() && seen.status == self.status && seen.motion_frame < self.motion_frame && !seen.stop;
            if another || again {
                self.spans[self.len - 1].end = Some(index);
                self.open = false;
            }
        }
        if seen.busy {
            if !self.open && self.len < WATCH_SPANS {
                self.spans[self.len] = Span { start: index, end: None, mv: seen.mv, variant: 0, angle: None };
                self.len += 1;
                self.open = true;
                self.told = false;
            } else if self.open && self.spans[self.len - 1].mv.is_none() {
                self.spans[self.len - 1].mv = seen.mv;
            }
            if self.open { self.describe(seen); }
        } else if self.open {
            self.spans[self.len - 1].end = Some(index);
            self.open = false;
        }
        self.status = seen.status;
        self.motion_frame = seen.motion_frame;
    }
    /// The open span's variant and angle, by the tracker's rules for an action (`frame_strip::Tracker::step`): the
    /// animation on the first frame the span has its move; a Jab that becomes the rapid jab or its finisher; a grab's
    /// throw. A side attack keeps the last angle its animation named.
    fn describe(&mut self, seen: Seen) {
        let span = &mut self.spans[self.len - 1];
        let (Some(mv), Some(now)) = (span.mv, seen.mv) else { return };
        let normal = Normal::of(seen.motion);
        if !self.told {
            span.variant = normal as u8;
            self.told = true;
        } else if (mv == Move::Jab && now == Move::Jab && matches!(normal, Normal::RapidJab | Normal::RapidJabFinisher))
            || (mv == Move::Grab && now == Move::Throw
                && matches!(normal, Normal::ForwardThrow | Normal::BackThrow | Normal::UpThrow | Normal::DownThrow))
        {
            span.variant = normal as u8;
        }
        if matches!(now, Move::SideTilt | Move::SideSmash) {
            if let Some(angle) = frame_strip::angle_of(seen.motion) { span.angle = Some(angle); }
        }
    }
    pub fn run(&self) -> u32 { self.run }
    pub fn spans(&self) -> &[Span] { &self.spans[..self.len] }
}

impl Marks {
    pub fn new() -> Self {
        Self::default()
    }

    fn insert(&mut self, at: usize, mark: Mark, origin: Option<String>, binding: Option<u32>, key: Option<String>) {
        self.list.insert(at, mark);
        self.origins.insert(at, origin);
        self.bindings.insert(at, binding);
        self.keys.insert(at, key);
        self.descs.insert(at, None);
    }

    /// Names the moves a preview finds as `namer` gives them.
    pub fn named(mut self, namer: Namer<'static>) -> Self {
        self.namer = namer;
        self
    }

    pub fn from_inputs(frames: impl Iterator<Item = InputFrame>) -> Self {
        let mut marks = Self::new();
        let mut previous = None;
        let mut last = None;
        for (index, input) in frames.enumerate() {
            let mut snap = Snap::empty(index as u32);
            snap.buttons = input.buttons;
            snap.lstick_x = input.lstick_x;
            snap.lstick_y = input.lstick_y;
            snap.rstick_x = input.rstick_x;
            snap.rstick_y = input.rstick_y;
            snap.facing = 1;
            if frame_strip::stick_axis(snap.lstick_y) < frame_strip::JUMP_STICK_Y {
                snap.buttons &= !frame_strip::FLICK_JUMP;
            }
            let mut edges = [None; STEPS];
            let mut count = 0;
            combo::push_edges(&mut edges, &mut count, previous, snap, false);
            previous = Some(snap);
            for event in edges.into_iter().flatten() {
                if crate::model::action::originates(event.kind)
                    && (marks.list.is_empty() || last.is_some_and(|last| event.frame.saturating_sub(last) > frame_strip::DROP_MAX))
                {
                    let name = input_name(event.kind).to_owned();
                    marks.insert(marks.list.len(), Mark { start: event.frame, end: None, name: name.clone() },
                        Some(name), None, None);
                }
                last = Some(event.frame);
            }
        }
        marks
    }

    pub fn from_units(names: impl Iterator<Item = String>) -> Self {
        let mut marks = Self::new();
        for (i, name) in names.enumerate() {
            marks.insert(marks.list.len(), Mark { start: i as u32, end: Some(i as u32 + 1), name: name.clone() },
                Some(name), None, None);
        }
        marks
    }

    pub fn from_recording(recording: &MacroRecording, namer: Namer<'static>) -> Self {
        let fallback = Self::from_inputs(recording.frames.iter().copied()).named(namer);
        let mut last = None;
        let valid = !recording.actions.is_empty() && recording.actions.iter().all(|action| {
            let ok = (action.start as usize) < recording.frames.len()
                && last.is_none_or(|last| last < action.start)
                && action.end.is_none_or(|end| end > action.start && (end as usize) <= recording.frames.len());
            last = Some(action.start);
            ok
        });
        if !valid { return fallback; }
        let mut marks = Self::new().named(namer);
        marks.recorded = true;
        for action in &recording.actions {
            let desc = action.mv.as_deref().and_then(move_of).filter(|mv| *mv != Move::Other)
                .map(|mv| (mv, action.variant, action.angle.as_deref().and_then(angle_of_key)));
            let name = desc.map(|(mv, variant, angle)| mark_name(namer, mv, variant, angle))
                .or_else(|| fallback.list.iter().find(|mark| mark.start == action.start).map(|mark| mark.name.clone()))
                .unwrap_or_else(|| "Move".into());
            let at = marks.list.len();
            marks.insert(at, Mark { start: action.start, end: action.end, name }, None, None, action.mv.clone());
            marks.descs[at] = desc;
        }
        marks
    }

    pub fn recorded(&self) -> bool { self.recorded }

    pub fn capture(frames: &[InputFrame], spans: &[Span]) -> Vec<ActionMark> {
        let mut marks = Self::from_inputs(frames.iter().copied());
        marks.watch(0, spans);
        marks.list.iter().zip(&marks.keys).zip(&marks.descs).map(|((mark, key), desc)| ActionMark {
            start: mark.start,
            end: mark.end.filter(|&end| end > mark.start && end <= frames.len() as u32),
            mv: key.clone(),
            variant: desc.map_or(0, |(_, variant, _)| variant),
            angle: desc.and_then(|(_, _, angle)| angle).map(|angle| angle_key(angle).to_owned()),
        }).collect()
    }

    pub fn window(actions: &[ActionMark], from: u32, to: u32) -> Vec<ActionMark> {
        actions.iter().filter(|action| action.start >= from && action.start < to).map(|action| ActionMark {
            start: action.start - from,
            end: action.end.filter(|&end| end <= to).map(|end| end - from),
            ..action.clone()
        }).collect()
    }
    pub fn list(&self) -> &[Mark] { &self.list }

    /// The mark `at` lies in or starts on, else the one whose start or known end is nearest. An end is exclusive, so of
    /// two marks that touch the later one owns the frame between them.
    pub fn nearest(&self, at: u32) -> Option<(u32, Option<u32>)> {
        self.list.iter().min_by_key(|mark| {
            let inside = mark.start <= at && mark.end.is_some_and(|end| at < end);
            let away = mark.end.map_or(u32::MAX, |end| end.abs_diff(at)).min(mark.start.abs_diff(at));
            (!(inside || mark.start == at), away, mark.start)
        }).map(|mark| (mark.start, mark.end))
    }

    pub fn forget_watched(&mut self, from: u32, to: u32) {
        if self.recorded { return; }
        for i in (0..self.list.len()).rev() {
            if self.bindings[i].is_none_or(|start| start < from || start >= to) { continue; }
            if let Some(name) = &self.origins[i] {
                self.list[i].name = name.clone();
                self.list[i].end = None;
                self.bindings[i] = None;
                self.keys[i] = None;
                self.descs[i] = None;
            } else {
                self.list.remove(i);
                self.origins.remove(i);
                self.bindings.remove(i);
                self.keys.remove(i);
                self.descs.remove(i);
            }
        }
    }

    pub fn watch(&mut self, base: u32, spans: &[Span]) {
        if self.recorded { return; }
        for span in spans {
            let start = base.saturating_add(span.start);
            let existing = self.bindings.iter().position(|bound| *bound == Some(start));
            let available = || self.list.iter().enumerate().rev().find(|(i, mark)| {
                self.origins[*i].is_some() && self.bindings[*i].is_none()
                    && mark.start <= start && start - mark.start <= frame_strip::INPUT_LOOKBACK
            }).map(|(i, _)| i);
            let index = match existing.or_else(available) {
                Some(index) => index,
                None => {
                    let index = self.list.partition_point(|mark| mark.start <= start);
                    self.insert(index, Mark { start, end: None, name: "Move".into() }, None, None, None);
                    index
                }
            };
            self.bindings[index] = Some(start);
            self.list[index].end = span.end.map(|end| base.saturating_add(end));
            if let Some(mv) = span.mv.filter(|mv| *mv != Move::Other) {
                self.list[index].name = mark_name(self.namer, mv, span.variant, span.angle);
                self.keys[index] = Some(full_move_key(mv).into());
                self.descs[index] = Some((mv, span.variant, span.angle));
            }
        }
    }

    pub fn snap_in(&self, at: u32) -> Option<u32> {
        if self.list.is_empty() { return None; }
        std::iter::once(0).chain(self.list.iter().map(|mark| mark.start))
            .min_by_key(|&boundary| (boundary.abs_diff(at), boundary))
    }

    pub fn snap_out(&self, at: u32, total: u32) -> Option<u32> {
        if self.list.is_empty() || total == 0 { return None; }
        std::iter::once(total).chain(self.list.iter().enumerate().map(|(i, mark)| {
            mark.end.unwrap_or_else(|| self.list.get(i + 1).map_or(total, |next| next.start)).min(total)
        })).filter(|&boundary| boundary != 0)
            .min_by_key(|&boundary| (boundary.abs_diff(at), std::cmp::Reverse(boundary)))
    }

    pub fn text(&self, at: u32, out_handle: bool) -> String {
        if self.list.is_empty() { return "No move marks yet: X previews and marks them".into(); }
        let start = || self.list.iter().find(|mark| mark.start == at).map(|mark| format!("Start of {}", mark.name));
        let end = || self.list.iter().find(|mark| mark.end == Some(at)).map(|mark| format!("End of {}", mark.name));
        if let Some(text) = if out_handle { end() } else { start() } { return text; }
        for (i, mark) in self.list.iter().enumerate() {
            let end = mark.end.or_else(|| self.list.get(i + 1).map(|next| next.start));
            if at > mark.start && end.is_none_or(|end| at < end) {
                let n = at - mark.start;
                return format!("{n} frame{} into {}", if n == 1 { "" } else { "s" }, mark.name);
            }
        }
        (if out_handle { start() } else { end() }).unwrap_or_else(|| "No move here".into())
    }

    pub fn shown(&self, cursor: u32, total: u32, width: u16) -> ([(u16, u16); SHOWN_MARKS], u8) {
        let mut selected: Vec<_> = self.list.iter().collect();
        selected.sort_by_key(|mark| (mark.start.abs_diff(cursor), mark.start));
        selected.truncate(SHOWN_MARKS);
        selected.sort_by_key(|mark| mark.start);
        let mut shown = [(0, 0); SHOWN_MARKS];
        let scale = |frame: u32| -> u16 {
            if total == 0 { 0 } else { ((u64::from(frame.min(total)) * u64::from(width)) / u64::from(total)) as u16 }
        };
        for (slot, mark) in shown.iter_mut().zip(&selected) {
            *slot = (scale(mark.start), scale(mark.end.unwrap_or(mark.start)));
        }
        (shown, selected.len() as u8)
    }
}

/// The editor's view as the bridge's `clip` line carries it; `ctx` names a strip that is not the editor's own (`drill`).
pub fn bridge_clip(view: &crate::engine::macro_ui::clip_panel::ClipView, ctx: Option<&'static str>, running: bool) -> Clip {
    use crate::engine::macro_ui::clip_panel::{ClipContext, Handle};
    Clip {
        ctx: ctx.unwrap_or(match view.context {
            ClipContext::Replay { .. } => "replay",
            ClipContext::Macro { player: true } => "macro.p1",
            ClipContext::Macro { player: false } => "macro.cpu",
            ClipContext::Combo { .. } => "combo",
        }),
        total: view.total, start: view.start, end: view.end, cursor: view.cursor,
        playing: view.playing,
        handle: match view.handle { Handle::Playhead => "playhead", Handle::In => "in", Handle::Out => "out" },
        running,
        target: match view.target { Some(true) => "p1", Some(false) => "cpu", None => "none" },
        marks: view.mark_total, mark: view.near, text: crate::model::bridge_events::MoveText::new(&view.move_text),
    }
}

fn input_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Press(Button::Attack) => "Attack",
        Kind::Press(Button::Special) => "Special",
        Kind::Press(Button::Jump) => "Jump",
        Kind::Press(Button::Shield) => "Shield",
        Kind::Press(Button::Grab) => "Grab",
        Kind::CStick => "C-stick",
        Kind::Flick(_) => "Flick",
        _ => "Move",
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::macro_ui::clip_panel::{self, ClipAction, ClipContext, ClipEditor, ClipEffect};
    use crate::engine::macros::{Library, StartState};
    use crate::model::combo::{Combo, LinkRec};
    use crate::model::combo_record::{move_key, ComboRecord};
    use crate::model::frame_strip::Verdict;
    use crate::engine::library::Role;
    use crate::engine::rr_runtime::{self, Capture, RecordStop};
    use crate::model::state::record::SavedState;

    fn inputs() -> Vec<InputFrame> {
        let mut frames = vec![InputFrame::default(); 200];
        frames[10].buttons = 1;
        frames[90].buttons = 2;
        frames
    }

    fn recording() -> MacroRecording {
        MacroRecording::new("Crop", "13.0.5", StartState::neutral(0, 0, 1.0), inputs()).unwrap()
    }

    fn recorded() -> MacroRecording {
        let mut recording = recording();
        recording.actions = vec![
            ActionMark { start: 10, end: Some(40), mv: Some(move_key(Move::Jab).into()), ..ActionMark::default() },
            ActionMark { start: 90, end: None, mv: None, ..ActionMark::default() },
        ];
        recording
    }

    fn spans() -> [Span; 2] {
        [Span { start: 12, end: Some(40), mv: Some(Move::Jab), ..Span::default() },
         Span { start: 92, end: None, mv: None, ..Span::default() }]
    }

    fn scratch(tag: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!("tr-replay-editor-{tag}-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn saved_state(kind: i32) -> SavedState {
        let mut state = SavedState::never_saved();
        state.fighter_kind = kind;
        state
    }

    fn snapshot(root: &std::path::Path) -> Vec<(std::ffi::OsString, Vec<u8>)> {
        let mut files: Vec<_> = std::fs::read_dir(root).unwrap().map(|entry| {
            let entry = entry.unwrap();
            (entry.file_name(), std::fs::read(entry.path()).unwrap())
        }).collect();
        files.sort_by(|a, b| a.0.cmp(&b.0));
        files
    }

    fn save_capture(root: &std::path::Path, frames: &[InputFrame], spans: &[Span], trim: crate::model::trim::Trim) -> String {
        let mut capture = Capture::idle();
        rr_runtime::toggle_record(&mut capture, Role::Player);
        for &frame in frames { rr_runtime::push_frame(&mut capture, Role::Player, frame); }
        capture.set_spans(spans);
        let RecordStop::Saved { id, .. } = rr_runtime::finish_recording(
            root, &capture, saved_state(0), saved_state(1), (0.0, 0.0), 1, trim).unwrap()
        else { panic!("recording not saved") };
        id
    }

    fn set_out(editor: &mut ClipEditor, at: u32) {
        editor.on_action(ClipAction::SelectOut);
        let current = editor.view().end;
        editor.on_action(ClipAction::Nudge(at as i32 - current as i32));
    }

    fn combo(input_only: bool) -> Combo {
        let mut combo = Combo::from_links([
            LinkRec::presses(Move::Jab, Move::SideTilt, Verdict::True(None), None, &[]),
            LinkRec::presses(Move::SideTilt, Move::NeutralSpecial, Verdict::True(None), None, &[]),
        ]).unwrap();
        combo.input_only = input_only;
        combo
    }

    fn combo_editor(input_only: bool) -> ClipEditor {
        ClipEditor::combo_file(ComboRecord::new("Combo".into(), combo(input_only), None)).unwrap()
    }

    fn the_editor_is_a_strip_at_the_bottom_and_leaves_the_fighters_visible() {
        use crate::model::panes::{CLIP_STRIP, CLIP_ROW_Y, CLIP_BAR_Y, CLIP_HELP_Y, CLIP_HINT_Y};
        let editor = ClipEditor::macro_file(recording(), true).unwrap();
        let mut frame = crate::model::frame::Frame::new();
        crate::editors::clip::clip_frame(&editor.view(), "Mario", 0, &mut frame);
        assert_eq!(frame.dialog_box().map(|v| v.0), Some(CLIP_STRIP));
        assert_eq!(CLIP_STRIP[1] - CLIP_STRIP[3] / 2.0, -540.0);
        const { assert!(CLIP_STRIP[3] <= 1080.0 * 0.3) };
        let lines = [CLIP_ROW_Y[0], CLIP_ROW_Y[1], CLIP_BAR_Y, CLIP_HELP_Y[0], CLIP_HELP_Y[1], CLIP_HINT_Y];
        let strip = CLIP_STRIP[1] - CLIP_STRIP[3] / 2.0..=CLIP_STRIP[1] + CLIP_STRIP[3] / 2.0;
        assert!(lines.iter().all(|y| strip.contains(y)));
        assert!(lines.windows(2).all(|p| p[0] - p[1] >= 44.0));
        use crate::model::panes::{CLIP_LABEL_BOX, CLIP_VALUE_BOX};
        let half = f32::from(clip_panel::CLIP_BAR_UNITS) / 2.0;
        assert_eq!(CLIP_LABEL_BOX.0 - CLIP_LABEL_BOX.1 / 2.0, -half);
        assert_eq!(CLIP_VALUE_BOX.0 + CLIP_VALUE_BOX.1 / 2.0, half);
        assert!(CLIP_LABEL_BOX.0 + CLIP_LABEL_BOX.1 / 2.0 <= CLIP_VALUE_BOX.0 - CLIP_VALUE_BOX.1 / 2.0);
        let view = include_str!("../../runtime/ui/view.rs");
        assert!(view.contains("frame.dialog_box()") && view.contains("IDX_BG"));
    }

    fn the_strip_stays_up_while_a_preview_plays() {
        let mut editor = ClipEditor::macro_file(recording(), true).unwrap();
        assert!(matches!(editor.on_action(ClipAction::TogglePlay), ClipEffect::PreviewFile { .. }));
        assert!(editor.view().playing);
        let worker = include_str!("../../runtime/ui/worker.rs");
        assert!(!worker.contains("v.hides_page()"));
    }

    /// tr-primary-2 00:36:47Z: a ZR tap is the Player 1 / CPU toggle, as on the Recording page; ZR held with a direction
    /// stays the coarse scrub; D-pad Up and Down no longer switch a saved macro's target.
    fn a_zr_tap_swaps_the_play_target_between_the_cpu_and_player_1() {
        use clip_panel::TargetTap;
        let press = |tap: &mut TargetTap, editor: &mut ClipEditor, samples: &[(bool, bool)]| {
            for &(zr, other) in samples {
                if tap.tapped(zr, other) { editor.on_action(ClipAction::SwapTarget); }
            }
        };
        let target = |editor: &ClipEditor| bridge_clip(&editor.view(), None, false).text();
        let mut editor = ClipEditor::macro_file(recorded(), false).unwrap();
        let mut tap = TargetTap::new();
        press(&mut tap, &mut editor, &[(true, false), (true, false), (false, false)]);
        assert_eq!(editor.context(), ClipContext::Macro { player: true });
        assert!(target(&editor).contains(" target=p1 "));
        press(&mut tap, &mut editor, &[(true, false), (false, false)]);
        assert_eq!(editor.context(), ClipContext::Macro { player: false });
        assert!(target(&editor).contains(" target=cpu "));
        // Held with Left or Right (on a GameCube pad ZR is the only modifier): the scrub moves, the target stays.
        let holds: [&[(bool, bool)]; 3] = [&[(true, false), (true, true), (true, false), (false, false)],
            &[(true, true), (false, false)], &[(true, false), (false, true)]];
        for samples in holds {
            press(&mut tap, &mut editor, samples);
            assert_eq!(editor.context(), ClipContext::Macro { player: false });
        }
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(clip_panel::scrub_step(70)));
        assert_eq!(editor.view().start, 20);
        for action in [ClipAction::PreviousFighter, ClipAction::NextFighter] {
            editor.on_action(action);
            assert_eq!(editor.context(), ClipContext::Macro { player: false });
        }
        assert!(matches!(editor.on_action(ClipAction::TogglePlay), ClipEffect::PreviewFile { player: false, .. }));
        press(&mut tap, &mut editor, &[(true, false), (false, false)]);
        assert!(!editor.view().playing);
        assert!(matches!(editor.on_action(ClipAction::TogglePlay), ClipEffect::PreviewFile { player: true, .. }));
        let controls = clip_panel::clip_help(&editor.view(), "ZL or ZR", "ZL", 0).controls;
        assert!(controls.starts_with("ZR Player/CPU. X preview. Y snap."), "{controls}");
        let adapter = include_str!("../../runtime/macros/replay_capture.rs");
        assert!(adapter.contains("TARGET_TAP.lock().unwrap().zr_tap(held)"));
        // rv-preview-vis 01:28:15Z: the pad's own button set. Any other bit, a stick's Up or Down included, spoils the tap.
        use crate::model::chords::{bit, Buttons};
        let tapped = |samples: &[u32]| {
            let mut tap = TargetTap::new();
            let mut hit = false;
            for &bits in samples { hit |= tap.tapped(Buttons(bits).zr(), Buttons(bits).other_than_zr()); }
            hit
        };
        assert!(tapped(&[bit::ZR, bit::ZR, 0]));
        for other in [bit::L_UP, bit::L_DOWN, bit::R_UP, bit::R_DOWN, bit::L_LEFT, bit::R_RIGHT, bit::DPAD_UP, bit::DPAD_LEFT,
            bit::A, bit::X, bit::L, bit::ZL, bit::STICK_L, bit::STICK_R, bit::PLUS, bit::MINUS, bit::LEFT_SL, bit::RIGHT_SR,
            bit::REAL_DIGITAL_L, bit::REAL_DIGITAL_R] {
            for samples in [&[bit::ZR, bit::ZR | other, 0][..], &[bit::ZR | other, 0], &[bit::ZR, bit::ZR | other, bit::ZR, 0],
                &[other, bit::ZR | other, bit::ZR, 0], &[bit::ZR, other]] {
                assert!(!tapped(samples), "{other:#x} {samples:x?}");
            }
        }
        assert!(!adapter.contains("controller.just_down.dpad_up() || controller.just_down.dpad_down()"));
    }

    fn swapping_cpu_and_p1_mid_preview_stops_it_and_x_plays_on_the_other_fighter() {
        let mut editor = ClipEditor::macro_file(recording(), false).unwrap();
        assert!(matches!(editor.on_action(ClipAction::TogglePlay), ClipEffect::PreviewFile { player: false, .. }));
        assert!(matches!(editor.on_action(ClipAction::SwapTarget), ClipEffect::StopPreview));
        assert!(!editor.view().playing);
        assert!(matches!(editor.on_action(ClipAction::TogglePlay), ClipEffect::PreviewFile { player: true, .. }));
    }

    fn the_carried_fork_hunks_are_listed_with_commit_file_and_lines() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/provenance/replay-editor.md");
        let text = std::fs::read_to_string(path).unwrap_or_default();
        for part in ["eb4eec2", "training_mod_tui/src/clip_panel.rs", "+16-124", "src/replay_capture.rs",
                     "15ada37", "training_mod_macros/src/move_start.rs", "+1-256",
                     "6f250bb", "src/training/save_states.rs", "src/replay_preview.rs"] {
            assert!(text.contains(part), "missing {part}");
        }
    }

    fn originating_inputs_mark_move_starts_for_any_fighter() {
        let marks = Marks::from_inputs(inputs().into_iter());
        let starts: Vec<_> = marks.list().iter().map(|m| (m.start, m.name.as_str(), m.end)).collect();
        assert_eq!(starts, vec![(10, "Attack", None), (90, "Special", None)]);
        let mut mario = recording();
        let mut other = recording();
        mario.start.fighter_kind = 0;
        other.start.fighter_kind = 8;
        assert_eq!(Marks::from_recording(&mario, Namer::OFF).list(), Marks::from_recording(&other, Namer::OFF).list());
    }

    fn a_watched_action_gives_its_start_end_and_name() {
        let mut watch = Watch::new();
        let busy = |mv| Seen { busy: true, mv, ..Seen::default() };
        watch.feed(3, 12, busy(None));
        watch.feed(3, 13, busy(Some(Move::Jab)));
        watch.feed(3, 40, Seen::default());
        assert_eq!(watch.run(), 3);
        assert_eq!(watch.spans(), &spans()[..1]);
        let mut marks = Marks::from_inputs(inputs().into_iter());
        marks.watch(0, watch.spans());
        assert_eq!((marks.list()[0].start, marks.list()[0].end, marks.list()[0].name.as_str()), (10, Some(40), "Jab"));
        marks.watch(0, watch.spans());
        assert_eq!(marks.list().len(), 2);
        watch.feed(4, 0, busy(Some(Move::SideTilt)));
        assert_eq!(watch.spans().len(), 1);
        for frame in 1..100 { watch.feed(4, frame, Seen { busy: frame % 2 == 0, ..Seen::default() }); }
        assert_eq!(watch.spans().len(), WATCH_SPANS);
        assert_eq!(watch.spans()[0].mv, Some(Move::SideTilt));
    }

    /// rv-preview-vis 23:49:00Z: a second Jab that starts while the fighter is still busy (the same status, its animation
    /// back at frame 0) is its own move; one continuous Jab is one.
    fn a_move_restarted_while_busy_is_its_own_recorded_mark() {
        let jab = |motion_frame: f32| Seen { busy: true, mv: Some(Move::Jab), status: 0x27, motion_frame, ..Seen::default() };
        let take = |restart: bool| {
            let mut watch = Watch::new();
            for index in 12..55u32 {
                let frame = if restart && index >= 30 { index - 30 } else { index - 12 };
                watch.feed(CAPTURE_RUN, index, jab(frame as f32));
            }
            watch.feed(CAPTURE_RUN, 55, Seen::default());
            let root = scratch(if restart { "restart" } else { "continuous" });
            let id = save_capture(&root, &inputs(), watch.spans(), crate::model::trim::Trim::Off);
            Library::new(root.join("macros")).load(&id).unwrap()
        };
        let twice = take(true);
        let ends: Vec<_> = twice.actions.iter().map(|mark| (mark.start, mark.end)).collect();
        assert_eq!(ends, vec![(10, Some(30)), (30, Some(55)), (90, None)]);
        let mut editor = ClipEditor::macro_file(twice, true).unwrap();
        assert_eq!(editor.view().mark_count, 3);
        // rv-preview-vis 01:10:02Z: the two Jabs touch. At 29 the playhead is in the first, at 30 in the second.
        editor.on_action(ClipAction::TogglePlay);
        editor.preview_cursor(30);
        assert_eq!((editor.view().cursor, editor.view().near), (29, Some((10, Some(30)))));
        editor.preview_cursor(31);
        assert_eq!((editor.view().cursor, editor.view().near), (30, Some((30, Some(55)))));
        assert!(bridge_clip(&editor.view(), None, false).text().contains(" cursor=30 "));
        assert!(bridge_clip(&editor.view(), None, false).text().contains(" marks=3 mark=30-55 "));
        editor.on_action(ClipAction::TogglePlay);
        let open = [ActionMark { start: 10, end: Some(30), mv: None, ..ActionMark::default() }, ActionMark { start: 30, end: None, mv: None, ..ActionMark::default() }];
        let mut touching = recording();
        touching.frames[30].buttons = 1;
        touching.actions = open.to_vec();
        let touching = Marks::from_recording(&touching, Namer::OFF);
        assert!(touching.recorded());
        assert_eq!((touching.nearest(29), touching.nearest(30)), (Some((10, Some(30))), Some((30, None))));
        set_out(&mut editor, 35);
        editor.on_action(ClipAction::Snap);
        assert_eq!(editor.view().end, 30);
        editor.on_action(ClipAction::Nudge(20));
        editor.on_action(ClipAction::Snap);
        assert_eq!(editor.view().end, 55);
        let once = take(false);
        let ends: Vec<_> = once.actions.iter().map(|mark| (mark.start, mark.end)).collect();
        assert_eq!(ends, vec![(10, Some(55)), (90, None)]);
        let mut hitstop = Watch::new();
        hitstop.feed(1, 12, jab(5.0));
        hitstop.feed(1, 13, Seen { stop: true, ..jab(4.0) });
        assert_eq!(hitstop.spans().len(), 1);
        let mut grab = Watch::new();
        grab.feed(1, 12, Seen { mv: Some(Move::Grab), ..jab(0.0) });
        grab.feed(1, 20, Seen { mv: Some(Move::Throw), status: 0x40, ..jab(0.0) });
        assert_eq!(grab.spans().len(), 1);
        grab.feed(1, 30, Seen { mv: Some(Move::UpTilt), status: 0x29, ..jab(0.0) });
        assert_eq!(grab.spans().len(), 2);
    }

    /// tr-primary-2's edge: a move on the take's first frame and one on its last are recorded, reloaded, drawn and snapped to.
    fn marks_at_frame_0_and_at_the_last_frame_are_kept_and_snapped_to() {
        let mut frames = vec![InputFrame::default(); 200];
        frames[0].buttons = 1;
        frames[199].buttons = 2;
        let root = scratch("edges");
        let spans = [Span { start: 0, end: Some(30), mv: Some(Move::Jab), ..Span::default() }, Span { start: 199, end: None, mv: None, ..Span::default() }];
        let id = save_capture(&root, &frames, &spans, crate::model::trim::Trim::Off);
        let loaded = Library::new(root.join("macros")).load(&id).unwrap();
        assert_eq!(loaded.actions.iter().map(|mark| (mark.start, mark.end)).collect::<Vec<_>>(), vec![(0, Some(30)), (199, None)]);
        let mut editor = ClipEditor::macro_file(loaded.clone(), true).unwrap();
        let view = editor.view();
        assert_eq!((view.mark_count, view.marks[0].0, view.marks[1].0), (2, 0, 1592));
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(3));
        editor.on_action(ClipAction::Snap);
        assert_eq!(editor.view().start, 0);
        assert!(editor.view().move_text.starts_with("Start of"), "{}", editor.view().move_text);
        set_out(&mut editor, 195);
        editor.on_action(ClipAction::Snap);
        assert_eq!(editor.view().end, 200);
        let mut ended = loaded;
        ended.actions[1].end = Some(200);
        assert_eq!(Marks::from_recording(&ended, Namer::OFF).list()[1].end, Some(200));
        ended.actions[1].end = Some(201);
        assert!(!Marks::from_recording(&ended, Namer::OFF).recorded());
    }

    /// tr-primary-2's edge: In or Out cutting through a move, and a snap whose target lies across the other handle.
    fn a_crop_or_a_snap_across_a_mark_keeps_the_marks_true_to_the_frames() {
        let jab = Some(move_key(Move::Jab).to_owned());
        let crop = |editor: &mut ClipEditor| {
            let ClipEffect::Export(saved) = editor.on_action(ClipAction::Confirm) else { panic!("crop") };
            saved.actions
        };
        let mut out_cuts = ClipEditor::macro_file(recorded(), true).unwrap();
        set_out(&mut out_cuts, 30);
        assert_eq!(crop(&mut out_cuts), vec![ActionMark { start: 10, end: None, mv: jab.clone(), ..ActionMark::default() }]);
        out_cuts.on_action(ClipAction::Snap);
        assert_eq!(out_cuts.view().end, 40);
        assert_eq!(crop(&mut out_cuts), vec![ActionMark { start: 10, end: Some(40), mv: jab.clone(), ..ActionMark::default() }]);
        let mut in_cuts = ClipEditor::macro_file(recorded(), true).unwrap();
        in_cuts.on_action(ClipAction::SelectIn);
        in_cuts.on_action(ClipAction::Nudge(20));
        assert_eq!(crop(&mut in_cuts), vec![ActionMark { start: 70, end: None, mv: None, ..ActionMark::default() }]);
        in_cuts.on_action(ClipAction::Snap);
        assert_eq!(in_cuts.view().start, 10);
        assert_eq!(crop(&mut in_cuts), vec![ActionMark { start: 0, end: Some(30), mv: jab, ..ActionMark::default() }, ActionMark { start: 80, end: None, mv: None, ..ActionMark::default() }]);
        let mut crossing = ClipEditor::macro_file(recorded(), true).unwrap();
        crossing.on_action(ClipAction::SelectIn);
        crossing.on_action(ClipAction::Nudge(95));
        set_out(&mut crossing, 96);
        crossing.on_action(ClipAction::Snap);
        assert_eq!((crossing.view().start, crossing.view().end), (95, 96));
        crossing.on_action(ClipAction::SelectIn);
        crossing.on_action(ClipAction::Snap);
        assert_eq!((crossing.view().start, crossing.view().end), (90, 96));
    }

    /// tr-primary-2 00:31:58Z: the line carries the play target, the mark count, the mark at or nearest the cursor and the
    /// strip's move text, quoted and bounded; the drill take preview's strip sends the same line as `ctx=drill`.
    fn a_clip_line_names_the_play_target_the_marks_and_a_bounded_quoted_move_text() {
        use crate::engine::macros::{ReplayFrame, ReplaySession, ReplayTrack};
        let mut frames = vec![InputFrame::default(); 200];
        frames[10].buttons = 1;
        frames[90].buttons = 2;
        let mut recording = MacroRecording::new("Crop", "13.0.5", StartState::neutral(0, 0, 1.0), frames).unwrap();
        recording.actions = vec![ActionMark { start: 10, end: Some(40), mv: None, ..ActionMark::default() }, ActionMark { start: 90, end: None, mv: None, ..ActionMark::default() }];
        let mut editor = ClipEditor::macro_file(recording, false).unwrap();
        let line = bridge_clip(&editor.view(), None, false).text();
        assert!(line.starts_with("clip ctx=macro.cpu total=200 in=0 out=200 cursor=0 playing=0 handle=playhead running=0 "), "{line}");
        assert!(line.ends_with("target=cpu marks=2 mark=10-40 move=\"No move here\""), "{line}");
        editor.on_action(ClipAction::SwapTarget);
        editor.on_action(ClipAction::SelectOut);
        let line = bridge_clip(&editor.view(), None, false).text();
        assert!(line.contains("ctx=macro.p1") && line.contains("cursor=199") && line.contains("handle=out"), "{line}");
        assert!(line.ends_with("target=p1 marks=2 mark=90-none move=\"110 frames into Special\""), "{line}");
        assert!(bridge_clip(&editor.view(), Some("drill"), true).text().starts_with("clip ctx=drill total=200 "));
        let capture = ReplaySession { tracks: vec![ReplayTrack { fighter: None, entry: 1, quantized_analog: false, frames: (0..40).map(|_| ReplayFrame {
            input: InputFrame::default(), start: StartState::neutral(74, 0, 1.0), x: 0.0, y: 0.0 }).collect() }],
            truncated: false, breaks: Vec::new() };
        let mut vault = ClipEditor::new(capture, vec![0]).unwrap();
        assert!(bridge_clip(&vault.view(), None, false).text().contains("ctx=replay"));
        assert!(bridge_clip(&vault.view(), None, false).text().contains(" target=cpu marks=0 mark=none move="));
        vault.on_action(ClipAction::SwapTarget);
        assert!(bridge_clip(&vault.view(), None, false).text().contains(" target=p1 "));
    }

    /// tr-primary-2 02:31:21Z, the Vault seam: a capture's preview plays on the editor's play target. The Recordings
    /// screen's "Plays on" seeds it, a ZR tap swaps it, and the bridge, the strip and the preview request follow.
    fn a_vault_capture_takes_the_zr_tap_and_previews_on_the_chosen_fighter() {
        use crate::engine::macros::{ReplayFrame, ReplaySession, ReplayTrack};
        use crate::model::chords::{bit, logical, logical_set, Buttons, STYLE_GAMECUBE};
        use clip_panel::TargetTap;
        let track = |entry: i32| ReplayTrack { fighter: None, entry, quantized_analog: false, frames: (0..40u32).map(|i| ReplayFrame {
            input: InputFrame { buttons: i + 1, ..InputFrame::default() }, start: StartState::neutral(entry + 7, 0, 1.0),
            x: 0.0, y: 0.0 }).collect() };
        let capture = || ReplaySession { tracks: vec![track(0), track(1)], truncated: false, breaks: Vec::new() };
        let mut editor = ClipEditor::new(capture(), vec![0]).unwrap();
        let mut tap = TargetTap::new();
        let mut press = |editor: &mut ClipEditor, samples: &[u32]| {
            let mut effect = None;
            for &bits in samples {
                if tap.zr_tap(Buttons(bits)) { effect = Some(editor.on_action(ClipAction::SwapTarget)); }
            }
            effect
        };
        let line = |editor: &ClipEditor| bridge_clip(&editor.view(), None, false).text();
        assert_eq!((editor.view().target, editor.view().play_role()), (Some(false), Role::Cpu));
        assert!(line(&editor).contains("ctx=replay") && line(&editor).contains(" target=cpu "));
        let controls = clip_panel::clip_help(&editor.view(), "ZL or ZR", "ZL", 0).controls;
        assert_eq!(controls, "D-pad Up/Down: fighter. ZR Player/CPU. X preview. Y snap. A save. B close.");
        // A tap swaps the target; X then previews the capture's kept inputs for Player 1.
        assert!(matches!(press(&mut editor, &[bit::ZR, bit::ZR, 0]), Some(ClipEffect::None)));
        assert_eq!((editor.view().target, editor.view().play_role()), (Some(true), Role::Player));
        assert!(line(&editor).contains(" target=p1 "));
        let ClipEffect::Preview(preview) = editor.on_action(ClipAction::TogglePlay) else { panic!("a capture previews its own clip") };
        assert_eq!(preview.frames.len(), 40);
        // The playhead follows the frames the play role took, as for a saved macro.
        editor.preview_cursor(12);
        assert_eq!(editor.view().cursor, 11);
        // A tap mid-preview stops it; the next X plays on the other fighter.
        assert!(matches!(press(&mut editor, &[bit::ZR, 0]), Some(ClipEffect::StopPreview)));
        assert!(!editor.view().playing);
        assert_eq!(editor.view().play_role(), Role::Cpu);
        assert!(matches!(editor.on_action(ClipAction::TogglePlay), ClipEffect::Preview(_)));
        editor.on_action(ClipAction::TogglePlay);
        // Held with a direction it is the coarse scrub, and D-pad Up/Down still pick the captured player.
        assert!(press(&mut editor, &[bit::ZR, bit::ZR | bit::DPAD_LEFT, bit::ZR, 0]).is_none());
        assert!(press(&mut editor, &[bit::ZR, bit::ZR | bit::L_UP, 0]).is_none());
        editor.set_play_target(true);
        let before = editor.view().entry;
        editor.on_action(ClipAction::NextFighter);
        assert_ne!(editor.view().entry, before);
        assert_eq!(editor.view().target, Some(true));
        // The strip names the captured side and the target once.
        let mut frame = crate::model::frame::Frame::new();
        crate::editors::clip::clip_frame(&editor.view(), "CPU Fox", 0, &mut frame);
        assert_eq!(String::from_utf16_lossy(frame.rows[1].label.units()), "Fighter: CPU Fox · plays on Player 1");
        // A GameCube pad: its Z arrives as the raw ZR bit (the editor's modifier there). The same rule holds, and the
        // full presses of L and R, which pick the handles, are no tap.
        assert_eq!(logical_set(STYLE_GAMECUBE, bit::ZR), 1 << logical::R);
        let mut gamecube = ClipEditor::new(capture(), vec![0]).unwrap();
        assert!(press(&mut gamecube, &[bit::ZR, 0]).is_some());
        assert_eq!(gamecube.view().target, Some(true));
        for samples in [&[bit::ZR, bit::ZR | bit::L_LEFT, bit::ZR, 0][..], &[bit::ZR | bit::DPAD_RIGHT, 0],
            &[bit::R | bit::REAL_DIGITAL_R, 0], &[bit::L | bit::REAL_DIGITAL_L, 0], &[bit::ZR, bit::ZR | bit::REAL_DIGITAL_R, 0]] {
            assert!(press(&mut gamecube, samples).is_none(), "{samples:x?}");
        }
        assert_eq!(gamecube.view().target, Some(true));
        // The adapter: one play target for the preview request, the fed cursor, Make and the label; "Plays on" seeds it.
        let adapter = include_str!("../../runtime/macros/replay_capture.rs");
        for wanted in ["TARGET_TAP.lock().unwrap().zr_tap(held)", "clip_view().is_some_and(|v| v.target.is_some())",
            "request_clip_preview(*recording, clip_play_role())", "editor.set_play_target(request.player);",
            "let role = before.play_role();", "editor_label(view.entry, &fighter),",
            "map_or(Role::Cpu, |editor| editor.view().play_role())"] {
            assert!(adapter.contains(wanted), "missing {wanted}");
        }
        for stale in ["is_some_and(|request| request.player)", "if request.player { crate::engine::library::Role::Player }",
            "matches!(v.context, ClipContext::Macro { .. })) {\n        on_action(ClipAction::SwapTarget)"] {
            assert!(!adapter.contains(stale), "stale {stale}");
        }
    }

    fn the_timeline_draws_move_marks() {
        let editor = ClipEditor::macro_file(recording(), true).unwrap();
        let view = editor.view();
        let mut frame = crate::model::frame::Frame::new();
        crate::editors::clip::clip_frame(&view, "Mario", 0, &mut frame);
        assert_eq!(frame.clip.mark_count, 2);
        assert!(frame.clip.marks[0].0 < frame.clip.marks[1].0);
        let drawing = include_str!("../../runtime/ui/view.rs");
        assert!(drawing.contains("clip.mark_count") && drawing.contains("IDX_TILE_STRIPE"));
    }

    fn an_empty_recording_has_no_marks() {
        let marks = Marks::from_inputs(std::iter::empty());
        assert!(marks.list().is_empty());
        assert_eq!(marks.snap_in(0), None);
        assert_eq!(marks.snap_out(0, 0), None);
        let mut empty = recording();
        empty.frames.clear();
        assert!(ClipEditor::macro_file(empty, true).is_err());
    }

    fn the_preview_watch_reads_the_previewing_fighter_only_while_a_preview_runs() {
        let source = include_str!("../../runtime/features/input_display.rs");
        assert!(source.contains("rr_host::preview_role()"));
        assert!(source.contains("rr_host::fed_run(role)"));
        assert!(source.contains("rr_host::watch_feed("));
    }

    fn y_snaps_the_selected_handle_to_the_nearest_move_boundary() {
        let mut editor = ClipEditor::macro_file(recorded(), true).unwrap();
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(13));
        editor.on_action(ClipAction::Snap);
        assert_eq!(editor.view().start, 10);
        set_out(&mut editor, 50);
        editor.on_action(ClipAction::Snap);
        assert_eq!(editor.view().end, 40);
        editor.on_action(ClipAction::TogglePlay);
        editor.watch(&[Span { start: 12, end: Some(80), mv: Some(Move::UpTilt), ..Span::default() }]);
        editor.on_action(ClipAction::TogglePlay);
        set_out(&mut editor, 50);
        editor.on_action(ClipAction::Snap);
        assert_eq!(editor.view().end, 40);
    }

    fn snapping_at_the_first_and_last_frame_stays_inside_the_take() {
        let marks = Marks::from_recording(&recorded(), Namer::OFF);
        assert_eq!(marks.snap_in(0), Some(0));
        assert_eq!(marks.snap_out(200, 200), Some(200));
    }

    fn a_move_with_no_clear_end_snaps_out_to_the_next_start_or_the_takes_end() {
        let marks = Marks::from_inputs(inputs().into_iter());
        assert_eq!(marks.snap_out(50, 200), Some(90));
        assert_eq!(marks.snap_out(190, 200), Some(200));
    }

    fn scrub_acceleration_is_bounded_from_one_to_180_a_tick() {
        for n in 0..10 { assert_eq!(clip_panel::scrub_repeat_step(n, false), 0); }
        assert!(matches!(clip_panel::direction_action(true, false, false, false, true, false), Some(ClipAction::Nudge(-1))));
        for n in 10..300 { assert!((1..=180).contains(&clip_panel::scrub_repeat_step(n, false))); }
        assert_eq!(clip_panel::scrub_repeat_step(300, false), 180);
        let mut editor = ClipEditor::macro_file(recording(), true).unwrap();
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(-180));
        assert_eq!(editor.view().start, 0);
        editor.on_action(ClipAction::Nudge(180));
        editor.on_action(ClipAction::Nudge(180));
        editor.on_action(ClipAction::SelectOut);
        editor.on_action(ClipAction::Nudge(180));
        assert_eq!(editor.view().end, 200);
        editor.on_action(ClipAction::Nudge(-180));
        let v = editor.view();
        assert!(v.start < v.end && v.end <= v.total);
    }

    fn an_unsaved_crop_previews_its_own_window_and_writes_no_file() {
        let root = scratch("unsaved");
        let library = Library::new(&root);
        let id = library.save_new(&recording()).unwrap();
        let before = snapshot(&root);
        let mut editor = ClipEditor::macro_file(library.load(&id).unwrap(), true).unwrap();
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(10));
        set_out(&mut editor, 120);
        assert!(matches!(editor.on_action(ClipAction::TogglePlay), ClipEffect::PreviewFile { start: 10, end: 120, player: true }));
        assert_eq!(snapshot(&root), before);
    }

    fn the_editor_reads_no_scene_so_lobby_and_training_match() {
        // The names of the marks come from the same two files as the combo views' (`move_names.rs`, `action.rs`).
        for text in [include_str!("clip_marks.rs"), include_str!("clip_panel.rs"), include_str!("../../editors/clip.rs"),
            include_str!("../move_names.rs"), include_str!("../action.rs")] {
            let code = text.split("#[cfg(test)]").next().unwrap();
            assert!(!code.contains("is_in_training") && !code.contains("is_in_lobby") && !code.contains("current_scene"));
        }
        let sample = include_str!("../../runtime/features/input_display.rs");
        let callback = include_str!("../../runtime/features/cpu.rs");
        assert!(sample.contains("rr_host::watch_feed("));
        // The one feed, in either scene, gives the watch the animation the variant is read from.
        assert!(sample.contains("clip_marks::watched_motion(s.motion, mv.is_some(), "));
        assert!(callback.contains("input_display::sample_frame("));
        // The capture's identity must follow the same body as its start pose (Training entry 1, lobby entry 4).
        let host = include_str!("../../runtime/features/recording_host.rs");
        let reader = host.split_once("fn role_fighter(").unwrap().1.split_once("\n}").unwrap().0;
        assert!(reader.contains("Role::Cpu => super::recorder::cpu_body()"), "CPU take identity bypasses the scene's practice CPU");
        assert!(reader.contains("Role::Player => super::recorder::body_of(0)"), "Player take identity must read entry 0");
    }

    /// A drill's crop names each step as the Recording list does (`combo_link_lines`, the recorded fighter): since the
    /// drill-step moves that is the step's move or input, not "Step N". The fixture's steps hold a real press each; a
    /// step with no event has no title and cannot come from a drill file.
    fn a_drill_crop_marks_every_step_and_names_it() {
        use crate::model::combo::{Aim, Event};
        let step = |button| {
            let mut step = LinkRec::INPUT;
            step.events[0] = Some(Event::press(button, Aim::Neutral, 0));
            step
        };
        let mut drill = Combo::from_links([step(Button::Attack), step(Button::Special)]).unwrap();
        drill.input_only = true;
        let heads: Vec<String> = crate::model::recording_menu::combo_link_lines(&drill, crate::model::move_names::Namer::recorded(None))
            .into_iter().filter(|line| crate::model::recording_menu::link_head(line)).collect();
        assert_eq!(heads.len(), 2);
        assert!(heads[0].eq_ignore_ascii_case("S1 attack") && heads[1].eq_ignore_ascii_case("S2 special"), "{heads:?}");
        let mut editor = ClipEditor::combo_file(ComboRecord::new("Drill".into(), drill, None)).unwrap();
        let view = editor.view();
        assert_eq!(view.mark_count, 2);
        assert_eq!(view.move_text, format!("Start of {}", heads[0]));
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(1));
        editor.on_action(ClipAction::Snap);
        assert_eq!(editor.view().start, 1);
        assert_eq!(editor.view().move_text, format!("Start of {}", heads[1]));
    }

    fn a_combo_crop_marks_every_link_and_names_it() {
        let editor = combo_editor(false);
        let view = editor.view();
        let expected = crate::model::recording_menu::combo_link_lines(&combo(false), crate::model::move_names::Namer::recorded(None));
        assert_eq!(view.mark_count, 2);
        assert!(view.move_text.contains(&expected[0]));
    }

    fn a_drill_preview_shows_the_strip_with_its_playhead() {
        let worker = include_str!("../../runtime/ui/worker.rs");
        let tick = include_str!("../../runtime/ui/tick.rs");
        assert!(worker.contains("Previewing the drill's take."));
        assert!(worker.contains("fed_frames(Role::Player)"));
        assert!(!tick.contains("&& !state::PREVIEW_HIDES.load(Ordering::Acquire)"));
        let mut editor = ClipEditor::macro_file(recording(), true).unwrap();
        editor.on_action(ClipAction::TogglePlay);
        editor.preview_cursor(36);
        assert_eq!(editor.view().cursor, 35);
    }

    fn a_saved_take_keeps_the_names_of_a_ledge_attack_a_throw_and_a_final_smash() {
        let root = scratch("saved-move-names");
        let moves = [Move::LedgeAttack, Move::Throw, Move::FinalSmash,
            Move::Grab, Move::GetupAttack, Move::TripAttack];
        let keys = ["ledge_attack", "throw", "final_smash", "grab", "getup_attack", "trip_attack"];
        let mut frames = vec![InputFrame::default(); 181];
        let spans: Vec<_> = moves.iter().enumerate().map(|(i, &mv)| {
            let start = (i * 30) as u32;
            frames[start as usize].buttons = 1;
            Span { start, end: Some(start + 10), mv: Some(mv), ..Span::default() }
        }).collect();
        let id = save_capture(&root, &frames, &spans, crate::model::trim::Trim::Off);
        let loaded = Library::new(root.join("macros")).load(&id).unwrap();

        assert_eq!(loaded.actions.iter().map(|mark| mark.mv.as_deref()).collect::<Vec<_>>(),
            keys.into_iter().map(Some).collect::<Vec<_>>());
        let marks = Marks::from_recording(&loaded, Namer::OFF);
        assert_eq!(marks.list().iter().map(|mark| mark.name.as_str()).collect::<Vec<_>>(),
            moves.iter().map(|mv| mv.name()).collect::<Vec<_>>());
        let editor = ClipEditor::macro_file(loaded, true).unwrap();
        assert_eq!(editor.view().move_text, "Start of Ledge attack");
        assert!(bridge_clip(&editor.view(), None, false).text().ends_with("move=\"Start of Ledge attack\""));
    }

    fn a_recorded_take_keeps_its_move_ends_and_out_snaps_to_one_before_any_preview() {
        let root = scratch("first-open");
        let id = save_capture(&root, &inputs(), &spans(), crate::model::trim::Trim::Off);
        let library = Library::new(root.join("macros"));
        let loaded = library.load(&id).unwrap();
        assert_eq!(loaded.actions.first().and_then(|mark| mark.end), Some(40));
        let mut editor = ClipEditor::macro_file(loaded, true).unwrap();
        set_out(&mut editor, 50);
        editor.on_action(ClipAction::Snap);
        assert_eq!(editor.view().end, 40);
        let path = root.join("macros").join(format!("{id}.json"));
        let mut json: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        json.as_object_mut().unwrap().remove("actions");
        std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
        let mut legacy = ClipEditor::macro_file(library.load(&id).unwrap(), true).unwrap();
        set_out(&mut legacy, 50);
        legacy.on_action(ClipAction::Snap);
        assert_eq!(legacy.view().end, 90);
    }

    fn a_file_without_recorded_marks_falls_back_to_input_starts_and_watched_ends() {
        let mut editor = ClipEditor::macro_file(recording(), true).unwrap();
        assert_eq!(editor.view().mark_count, 2);
        editor.on_action(ClipAction::TogglePlay);
        editor.watch(&spans()[..1]);
        editor.on_action(ClipAction::TogglePlay);
        set_out(&mut editor, 50);
        editor.on_action(ClipAction::Snap);
        assert_eq!(editor.view().end, 40);
    }

    fn a_crop_keeps_the_recorded_marks_inside_its_window() {
        let mut editor = ClipEditor::macro_file(recorded(), true).unwrap();
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(5));
        set_out(&mut editor, 90);
        let ClipEffect::Export(saved) = editor.on_action(ClipAction::Confirm) else { panic!("crop") };
        assert_eq!(saved.actions, vec![ActionMark { start: 5, end: Some(35), mv: Some(move_key(Move::Jab).into()), ..ActionMark::default() }]);
        assert!(Marks::from_recording(&saved, Namer::OFF).recorded());
        assert_eq!(Marks::window(&recorded().actions, 5, 120).len(), 2);
    }

    fn a_record_time_trim_keeps_the_marks_inside_the_kept_frames() {
        let root = scratch("trim");
        let spans = [Span { start: 12, end: Some(40), mv: Some(Move::Jab), ..Span::default() },
            Span { start: 92, end: Some(150), mv: Some(Move::SideTilt), ..Span::default() }];
        let id = save_capture(&root, &inputs(), &spans, crate::model::trim::Trim::End);
        let loaded = Library::new(root.join("macros")).load(&id).unwrap();
        let drill = rr_runtime::save_drill_take(&root, inputs(), StartState::neutral(0, 0, 1.0), crate::model::trim::Trim::End, 2, &spans).unwrap().unwrap();
        let drill = Library::new(root.join("macros")).load(&drill).unwrap();
        for take in [loaded, drill] {
            assert_eq!(take.frames.len(), 91);
            assert_eq!(take.actions.iter().map(|m| (m.start, m.end)).collect::<Vec<_>>(), [(10, Some(40)), (90, Some(91))]);
            let mut editor = ClipEditor::macro_file(take, true).unwrap();
            set_out(&mut editor, 80);
            editor.on_action(ClipAction::Snap);
            assert_eq!(editor.view().end, 91);
            assert!(editor.view().move_text.starts_with("End of "));
        }
        let open = [ActionMark { start: 90, end: None, mv: None, ..ActionMark::default() }];
        assert_eq!(Marks::window(&open, 5, 91)[0].end, None);
    }

    fn damaged_marks_in_a_file_are_ignored_not_fatal() {
        let root = scratch("bad-marks");
        let library = Library::new(&root);
        let invalid = [
            vec![ActionMark { start: 200, end: None, mv: None, ..ActionMark::default() }],
            vec![ActionMark { start: 10, end: Some(10), mv: None, ..ActionMark::default() }],
            vec![ActionMark { start: 90, end: None, mv: None, ..ActionMark::default() }, ActionMark { start: 10, end: None, mv: None, ..ActionMark::default() }],
        ];
        for actions in invalid {
            let mut rec = recording();
            rec.actions = actions;
            let id = library.save_new(&rec).unwrap();
            let loaded = library.load(&id).unwrap();
            assert!(loaded.validate().is_ok());
            let marks = Marks::from_recording(&loaded, Namer::OFF);
            assert!(!marks.recorded());
            assert_eq!(marks.list().iter().map(|m| m.start).collect::<Vec<_>>(), vec![10, 90]);
        }
        let id = library.save_new(&recording()).unwrap();
        let path = root.join(format!("{id}.json"));
        let mut json: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        json["actions"] = serde_json::json!([{"start": "bad"}]);
        std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
        let loaded = library.load(&id);
        assert!(loaded.is_ok());
        assert!(!Marks::from_recording(&loaded.unwrap(), Namer::OFF).recorded());
    }

    fn the_capture_watch_follows_the_recorded_fighter_while_a_take_is_open() {
        let input = include_str!("../../runtime/features/input_display.rs");
        let host = include_str!("../rr_host.rs");
        assert!(input.contains("rr_host::capture_role()"));
        assert!(input.contains("rr_host::captured_frames()"));
        assert!(input.contains("CAPTURE_RUN"));
        assert!(input.contains("rr_host::watch_feed("));
        assert!(host.contains("watch_clear()"));
    }

    type Part = (u32, u32, Move, i32, &'static str);

    fn on() -> Namer<'static> { Namer::new(true, None, None) }

    /// Each part is frames `from..to` of one animation of a move, counted from its start, in one status; at `end` the
    /// fighter can act again.
    fn watched(parts: &[Part], end: u32) -> Watch {
        let mut watch = Watch::new();
        for &(from, to, mv, status, motion) in parts {
            for index in from..to {
                watch.feed(CAPTURE_RUN, index, Seen { busy: true, mv: Some(mv), status, motion_frame: (index - from) as f32,
                    stop: false, motion: crate::model::hash40::hash40(motion) });
            }
        }
        watch.feed(CAPTURE_RUN, end, Seen::default());
        watch
    }

    fn take(frames: usize, actions: Vec<ActionMark>) -> MacroRecording {
        let mut take = MacroRecording::new("Take", "13.0.5", StartState::neutral(6, 0, 1.0), vec![InputFrame::default(); frames]).unwrap();
        take.actions = actions;
        take
    }

    fn names(take: &MacroRecording, namer: Namer<'static>) -> Vec<String> {
        Marks::from_recording(take, namer).list().iter().map(|mark| mark.name.clone()).collect()
    }

    /// What the combo and drill views write for a move (`ActionDesc::write`).
    fn written(namer: Namer<'_>, mv: Move, variant: u8, angle: Option<Angle>) -> String {
        let mut text = String::new();
        crate::model::action::ActionDesc { mv, of: None, angle, normal: variant, parts: [None; 2] }.write(&mut text, false, namer).unwrap();
        text
    }

    /// The final install-4 session's take (`i4-final-yoga-01`, `macro-9`): Kirby, Attack held on frames 37 to 41, busy until
    /// 56, restarted on 42. Here the second animation is the second jab's; whether the game's is, the Eden row decides.
    fn a_held_attack_on_kirby_is_recorded_as_jab_1_then_jab_2() {
        let mut frames = vec![InputFrame::default(); 138];
        for frame in &mut frames[37..42] { frame.buttons = 1; }
        let watch = watched(&[(37, 42, Move::Jab, 0x27, "attack_11"), (42, 56, Move::Jab, 0x27, "attack_12")], 56);
        let root = scratch("kirby-jabs");
        let id = save_capture(&root, &frames, watch.spans(), crate::model::trim::Trim::Off);
        let json: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("macros").join(format!("{id}.json"))).unwrap()).unwrap();
        assert_eq!(json["actions"], serde_json::json!([
            {"start": 37, "end": 42, "move": "jab", "variant": 1}, {"start": 42, "end": 56, "move": "jab", "variant": 2}]));
        let loaded = Library::new(root.join("macros")).load(&id).unwrap();
        assert_eq!(names(&loaded, on()), ["Jab 1", "Jab 2"]);
        assert_eq!(names(&loaded, Namer::OFF), ["Jab", "Jab"]);
        let mut editor = ClipEditor::macro_file(loaded, true).unwrap().with_namer(on());
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(40));
        editor.on_action(ClipAction::Snap);
        assert_eq!((editor.view().start, editor.view().move_text.as_str()), (42, "Start of Jab 2"));
        assert!(bridge_clip(&editor.view(), None, false).text().ends_with("move=\"Start of Jab 2\""));
        editor.on_action(ClipAction::Nudge(-4));
        editor.on_action(ClipAction::Snap);
        assert_eq!((editor.view().start, editor.view().move_text.as_str()), (37, "Start of Jab 1"));
    }

    fn a_three_hit_jab_is_jab_1_jab_2_and_jab_3() {
        let watch = watched(&[(10, 16, Move::Jab, 0x27, "attack_11"), (16, 24, Move::Jab, 0x27, "attack_12"),
            (24, 40, Move::Jab, 0x27, "attack_13")], 40);
        let marks = Marks::capture(&[InputFrame::default(); 60], watch.spans());
        assert_eq!(marks.iter().map(|mark| (mark.start, mark.end, mark.variant)).collect::<Vec<_>>(),
            [(10, Some(16), 1), (16, Some(24), 2), (24, Some(40), 3)]);
        assert_eq!(names(&take(60, marks), on()), ["Jab 1", "Jab 2", "Jab 3"]);
    }

    fn a_rapid_jab_and_its_finisher_are_named() {
        let watch = watched(&[(10, 16, Move::Jab, 0x27, "attack_11"), (16, 20, Move::Jab, 0x2a, "attack_100_start"),
            (20, 26, Move::Jab, 0x2a, "attack_100"), (26, 32, Move::Jab, 0x2a, "attack_100"),
            (32, 50, Move::Jab, 0x2a, "attack_100_end")], 50);
        let marks = Marks::capture(&[InputFrame::default(); 60], watch.spans());
        // The first jab flows into the rapid jab in another status, with no restart: one mark, renamed in place as the
        // combo link is (tr-primary-2 13:54:02Z, choice 3). Each loop and the finisher restart, so each is its own mark.
        assert_eq!(marks.iter().map(|mark| (mark.start, mark.variant)).collect::<Vec<_>>(), [(10, 4), (20, 4), (26, 4), (32, 5)]);
        assert_eq!(names(&take(60, marks), on()), ["Rapid jab", "Rapid jab", "Rapid jab", "Rapid jab finisher"]);
        // Only a Jab is renamed in place: another move keeps the variant it started with.
        let tilt = watched(&[(10, 16, Move::UpTilt, 0x29, "attack_hi3"), (16, 20, Move::UpTilt, 0x2b, "attack_100")], 20);
        assert_eq!((tilt.spans().len(), tilt.spans()[0].variant), (1, Normal::UpTilt as u8));
    }

    /// tr-primary-2 13:54:02Z, choice 2: a take saved before the variant existed has none. Off, it reads as it did. On,
    /// the Namer names it with no variant, and a special by the fighter's own name. Loading and saving do not change it.
    fn an_old_take_with_no_variant_reads_as_before_off_and_through_the_namer_on() {
        let moves = [Move::Jab, Move::SideTilt, Move::NeutralSpecial, Move::UpSpecial, Move::Aerial, Move::FinalSmash];
        let old = serde_json::Value::Array(moves.iter().enumerate().map(|(i, &mv)|
            serde_json::json!({"start": i * 20, "end": i * 20 + 10, "move": full_move_key(mv)})).collect());
        let root = scratch("old-take");
        let library = Library::new(&root);
        let id = library.save_new(&take(200, Vec::new())).unwrap();
        let path = root.join(format!("{id}.json"));
        let mut json: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        json["actions"] = old.clone();
        std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
        let loaded = library.load(&id).unwrap();
        assert!(Marks::from_recording(&loaded, on()).recorded());
        assert_eq!(names(&loaded, Namer::OFF), moves.map(|mv| mv.name()));
        // A legacy Jab alone cannot tell the two paths apart; the side tilt and the specials can.
        assert_eq!(names(&loaded, on()), ["Jab", "Forward tilt", "Neutral special", "Up special", "Aerial", "Final Smash"]);
        let table: &'static crate::model::move_names::Names =
            Box::leak(Box::new(crate::model::move_names::Names::build(&crate::model::move_names::tests::shared_file()).unwrap()));
        let alpha = crate::model::move_names::FighterId { chara: crate::model::hash40::hash40("ui_chara_alpha") & 0xff_ffff_ffff, specials: [0; 4] };
        let own = Namer::new(true, Some(table), Some(alpha));
        assert_eq!(names(&loaded, own), ["Jab", "Forward tilt", "Test Pulse", "Test Rising Kick", "Aerial", "Test Grand Finish"]);
        for namer in [on(), own] {
            assert_eq!(names(&loaded, namer), moves.map(|mv| written(namer, mv, 0, None)));
        }
        let again = library.save_new(&loaded).unwrap();
        let saved: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join(format!("{again}.json"))).unwrap()).unwrap();
        assert_eq!(saved["actions"], old);
    }

    fn with_move_names_off_a_mark_reads_as_before() {
        let marks = vec![
            ActionMark { start: 0, end: Some(10), mv: Some(full_move_key(Move::Jab).into()), variant: 2, angle: None },
            ActionMark { start: 20, end: Some(30), mv: Some(full_move_key(Move::SideTilt).into()), variant: 7, angle: Some("up".into()) },
            ActionMark { start: 40, end: Some(50), mv: Some(full_move_key(Move::Aerial).into()), variant: 14, angle: None },
            ActionMark { start: 60, end: Some(70), mv: Some(full_move_key(Move::Grab).into()), variant: 22, angle: None },
            ActionMark { start: 80, end: Some(90), mv: Some(full_move_key(Move::NeutralSpecial).into()), variant: 0, angle: None },
        ];
        let stripped: Vec<_> = marks.iter().map(|mark| ActionMark { variant: 0, angle: None, ..mark.clone() }).collect();
        assert_eq!(names(&take(100, marks.clone()), Namer::OFF), ["Jab", "Side tilt", "Aerial", "Grab", "Neutral B"]);
        assert_eq!(names(&take(100, marks), Namer::OFF), names(&take(100, stripped), Namer::OFF));
        for mv in [Move::Jab, Move::SideSmash, Move::Throw, Move::LedgeAttack, Move::FinalSmash] {
            assert_eq!(mark_name(Namer::OFF, mv, 5, Some(Angle::Down)), mv.name());
        }
    }

    fn an_angled_forward_tilt_is_named_with_its_angle() {
        type Angled = (&'static [Part], u8, Option<Angle>, &'static str, &'static str);
        let cases: [Angled; 4] = [
            (&[(10, 30, Move::SideTilt, 0x28, "attack_s3_hi")], 7, Some(Angle::Up), "Forward tilt ↑", "Side tilt"),
            (&[(10, 30, Move::SideTilt, 0x28, "attack_s3_s")], 7, None, "Forward tilt", "Side tilt"),
            (&[(10, 30, Move::SideTilt, 0x28, "attack_s3_lw")], 7, Some(Angle::Down), "Forward tilt ↓", "Side tilt"),
            (&[(10, 20, Move::SideSmash, 0x2c, "attack_s4_hold"), (20, 40, Move::SideSmash, 0x2d, "attack_s4_lw")],
                10, Some(Angle::Down), "Forward smash ↓", "Side smash"),
        ];
        for (parts, variant, angle, shown, short) in cases {
            let watch = watched(parts, parts.last().unwrap().1);
            assert_eq!(watch.spans().len(), 1);
            let marks = Marks::capture(&[InputFrame::default(); 60], watch.spans());
            let key = angle.map(|angle| match angle { Angle::Up => "up", Angle::Down => "down" });
            assert_eq!((marks[0].variant, marks[0].angle.as_deref()), (variant, key), "{shown}");
            assert_eq!(serde_json::to_value(&marks[0]).unwrap().get("angle").and_then(|angle| angle.as_str()), key);
            let saved = take(60, marks);
            assert_eq!(names(&saved, on()), [shown]);
            assert_eq!(shown, written(on(), parts[0].2, variant, angle));
            assert_eq!(names(&saved, Namer::OFF), [short]);
        }
    }

    /// A take saved without marks learns its moves from a preview, on either fighter. rv-preview-vis 13:45:17Z: the
    /// fighter the watch follows need not be the acting side, whose sample alone carries the animation.
    fn a_preview_on_the_cpu_or_player_1_names_the_moves_of_a_take_without_marks() {
        let found = [Span { start: 12, end: Some(40), mv: Some(Move::Jab), variant: 2, angle: None }];
        for player in [true, false] {
            for (namer, name) in [(on(), "Jab 2"), (Namer::OFF, "Jab")] {
                let mut editor = ClipEditor::macro_file(recording(), player).unwrap().with_namer(namer);
                assert!(matches!(editor.on_action(ClipAction::TogglePlay), ClipEffect::PreviewFile { player: target, .. } if target == player));
                editor.watch(&found);
                editor.preview_cursor(11);
                assert_eq!(editor.view().move_text, format!("Start of {name}"), "player {player}");
            }
        }
        let reads = std::cell::Cell::new(0);
        let read = || { reads.set(reads.get() + 1); 9 };
        // The acting side's sample has its animation: nothing more is read.
        assert_eq!((watched_motion(7, true, read), reads.get()), (7, 0));
        // The other side, in a named move: its animation is read once.
        assert_eq!((watched_motion(0, true, read), reads.get()), (9, 1));
        // Not in a named move: no read.
        assert_eq!((watched_motion(0, false, read), reads.get()), (0, 1));
        let feed = include_str!("../../runtime/features/input_display.rs");
        for wanted in [
            "motion: crate::engine::macro_ui::clip_marks::watched_motion(s.motion, mv.is_some(), || unsafe { p1_motion(accessor) }),",
            "if take == 1 { (&p1, player) } else { (&cpu, cpu_accessor) }",
            "if role == Role::Player { (&p1, player) } else { (&cpu, cpu_accessor) }",
            // Which side is graded, and the guard on the animation read, are as they were.
            "let p1 = sample(player, &ids, order[0] == 0, spots);",
            "let cpu = sample(cpu, &ids, order[0] == 1, spots);",
            "if !ANGLE_ON.load(Ordering::Acquire) {\n        return 0;\n    }\n    natives::motion_kind(accessor)",
        ] {
            assert!(feed.contains(wanted), "missing {wanted}");
        }
    }

    fn every_variant_names_a_mark_as_the_combo_views_do() {
        use Normal as N;
        let table: [(&str, Move, Normal); 28] = [
            ("attack_11", Move::Jab, N::Jab1), ("attack_12", Move::Jab, N::Jab2), ("attack_13", Move::Jab, N::Jab3),
            ("attack_100_start", Move::Jab, N::RapidJab), ("attack_100_end", Move::Jab, N::RapidJabFinisher),
            ("attack_dash", Move::DashAttack, N::DashAttack), ("attack_s3_s", Move::SideTilt, N::ForwardTilt),
            ("attack_hi3", Move::UpTilt, N::UpTilt), ("attack_lw3", Move::DownTilt, N::DownTilt),
            ("attack_s4_s", Move::SideSmash, N::ForwardSmash), ("attack_hi4", Move::UpSmash, N::UpSmash),
            ("attack_lw4", Move::DownSmash, N::DownSmash), ("attack_air_n", Move::Aerial, N::NeutralAir),
            ("attack_air_f", Move::Aerial, N::ForwardAir), ("attack_air_b", Move::Aerial, N::BackAir),
            ("attack_air_hi", Move::Aerial, N::UpAir), ("attack_air_lw", Move::Aerial, N::DownAir),
            ("catch", Move::Grab, N::Grab), ("catch_dash", Move::Grab, N::DashGrab), ("catch_turn", Move::Grab, N::PivotGrab),
            ("catch_attack", Move::Grab, N::Pummel), ("throw_f", Move::Throw, N::ForwardThrow),
            ("throw_b", Move::Throw, N::BackThrow), ("throw_hi", Move::Throw, N::UpThrow), ("throw_lw", Move::Throw, N::DownThrow),
            ("cliff_attack_quick", Move::LedgeAttack, N::LedgeAttack), ("down_attack_u", Move::GetupAttack, N::GetupAttack),
            ("slip_attack", Move::TripAttack, N::TripAttack),
        ];
        let mut covered = [false; 29];
        for (motion, mv, normal) in table {
            let watch = watched(&[(10, 30, mv, 0x30, motion)], 30);
            let marks = Marks::capture(&[InputFrame::default(); 40], watch.spans());
            assert_eq!((marks[0].variant, marks[0].mv.as_deref()), (normal as u8, Some(full_move_key(mv))), "{motion}");
            // The tracker's own reading of the same animation, which a combo link stores.
            assert_eq!(marks[0].variant, crate::model::action::variant(crate::model::hash40::hash40(motion)));
            let name = names(&take(40, marks), on()).remove(0);
            assert_eq!(name, normal.name().unwrap(), "{motion}");
            assert_eq!(name, written(on(), mv, normal as u8, None));
            covered[normal as usize] = true;
        }
        assert!(covered[1..].iter().all(|row| *row), "a variant of the move names has no row here");
        assert_eq!(Normal::from_u8(29), Normal::Unknown);
        // An animation that is none of them: no variant and no key, and the Namer's name without one.
        let special = watched(&[(10, 30, Move::NeutralSpecial, 0x1de, "special_n")], 30);
        let marks = Marks::capture(&[InputFrame::default(); 40], special.spans());
        assert_eq!(serde_json::to_value(&marks[0]).unwrap(),
            serde_json::json!({"start": 10, "end": 30, "move": full_move_key(Move::NeutralSpecial)}));
        assert_eq!(names(&take(40, marks), on()), ["Neutral special"]);
    }

    /// tr-primary-2 13:54:02Z, choice 4: a throw continues its grab's mark, whose move and saved key stay the grab's;
    /// its variant is the throw's, as the tracker's is.
    fn a_throw_names_its_grab_mark() {
        let watch = watched(&[(10, 20, Move::Grab, 0x3a, "catch_dash"), (20, 40, Move::Throw, 0x40, "throw_f"),
            (40, 44, Move::Throw, 0x41, "wait")], 44);
        assert_eq!(watch.spans().len(), 1);
        let marks = Marks::capture(&[InputFrame::default(); 60], watch.spans());
        assert_eq!((marks[0].mv.as_deref(), marks[0].variant, marks[0].end),
            (Some(full_move_key(Move::Grab)), Normal::ForwardThrow as u8, Some(44)));
        let thrown = take(60, marks);
        assert_eq!(names(&thrown, on()), ["Forward throw"]);
        assert_eq!(names(&thrown, Namer::OFF), ["Grab"]);
        let whiff = watched(&[(10, 40, Move::Grab, 0x3a, "catch_dash")], 40);
        let marks = Marks::capture(&[InputFrame::default(); 60], whiff.spans());
        assert_eq!(names(&take(60, marks), on()), ["Dash grab"]);
    }

    fn a_variant_of_the_wrong_shape_reads_as_no_marks() {
        let root = scratch("bad-variant");
        let library = Library::new(&root);
        let load = |actions: serde_json::Value| {
            let id = library.save_new(&recording()).unwrap();
            let path = root.join(format!("{id}.json"));
            let mut json: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            json["actions"] = actions;
            std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
            library.load(&id).unwrap()
        };
        let tilt = full_move_key(Move::SideTilt);
        for bad in [serde_json::json!({"variant": "two"}), serde_json::json!({"variant": 300}), serde_json::json!({"variant": -1}),
            serde_json::json!({"angle": 3})] {
            let mut action = serde_json::json!({"start": 10, "end": 40, "move": "jab"});
            action.as_object_mut().unwrap().extend(bad.as_object().unwrap().clone());
            let loaded = load(serde_json::json!([action]));
            assert!(loaded.validate().is_ok());
            let marks = Marks::from_recording(&loaded, on());
            assert!(!marks.recorded(), "{bad}");
            assert_eq!(marks.list().iter().map(|mark| mark.start).collect::<Vec<_>>(), [10, 90]);
        }
        // A number the move names do not know, an angle that is no angle and a key of a later build: the marks stay.
        let odd = load(serde_json::json!([
            {"start": 10, "end": 40, "move": "jab", "variant": 99},
            {"start": 90, "end": 120, "move": tilt, "variant": 7, "angle": "sideways", "later": true}]));
        assert!(Marks::from_recording(&odd, on()).recorded());
        assert_eq!(names(&odd, on()), ["Jab", "Forward tilt"]);
        assert_eq!(names(&odd, Namer::OFF), ["Jab", "Side tilt"]);
    }

    fn a_crop_and_a_record_time_trim_keep_the_variants() {
        let tilt = full_move_key(Move::SideTilt);
        let mut source = recording();
        source.actions = vec![
            ActionMark { start: 10, end: Some(40), mv: Some("jab".into()), variant: 1, angle: None },
            ActionMark { start: 90, end: Some(150), mv: Some(tilt.into()), variant: 7, angle: Some("up".into()) }];
        let mut editor = ClipEditor::macro_file(source, true).unwrap();
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(5));
        let ClipEffect::Export(saved) = editor.on_action(ClipAction::Confirm) else { panic!("crop") };
        assert_eq!(saved.actions, vec![
            ActionMark { start: 5, end: Some(35), mv: Some("jab".into()), variant: 1, angle: None },
            ActionMark { start: 85, end: Some(145), mv: Some(tilt.into()), variant: 7, angle: Some("up".into()) }]);
        assert_eq!(names(&saved, on()), ["Jab 1", "Forward tilt ↑"]);
        let root = scratch("trim-variants");
        let spans = [Span { start: 12, end: Some(40), mv: Some(Move::Jab), variant: 1, angle: None },
            Span { start: 92, end: Some(150), mv: Some(Move::SideTilt), variant: 7, angle: Some(Angle::Down) }];
        let id = save_capture(&root, &inputs(), &spans, crate::model::trim::Trim::End);
        let drill = rr_runtime::save_drill_take(&root, inputs(), StartState::neutral(0, 0, 1.0), crate::model::trim::Trim::End, 2, &spans).unwrap().unwrap();
        for id in [id, drill] {
            let loaded = Library::new(root.join("macros")).load(&id).unwrap();
            assert_eq!(loaded.actions.iter().map(|mark| (mark.start, mark.variant, mark.angle.as_deref())).collect::<Vec<_>>(),
                [(10, 1, None), (90, 7, Some("down"))]);
            assert_eq!(names(&loaded, on()), ["Jab 1", "Forward tilt ↓"]);
        }
    }

    /// rv-preview-vis 13:55:17Z and 14:04:42Z: the editor as it is reopened, with the published toggle, table and
    /// Player 1 and no injected Namer. A special is named by the fighter the take was recorded on, or by the common
    /// name: never by another fighter's, and never by another Mii's selection.
    fn the_reopened_editor_names_a_special_only_by_the_fighter_it_was_recorded_on() {
        use crate::model::move_names::{FighterId, Names, PUBLISHED};
        const CHILD: &str = "TR_MARK_NAMES_CHILD";
        let name = "the_reopened_editor_names_a_special_only_by_the_fighter_it_was_recorded_on";
        if std::env::var_os(CHILD).is_none() {
            // The published names are one per process: this case runs alone in a child.
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "engine::macro_ui::clip_marks::tests::case_table", "--nocapture"])
                .env("TR_TABLE_CASE", name).env(CHILD, "1").output().unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(output.status.success() && stdout.contains("running 1 test"), "{stdout}\n{}", String::from_utf8_lossy(&output.stderr));
            return;
        }
        let alpha = FighterId { chara: crate::model::hash40::hash40("ui_chara_alpha") & 0xff_ffff_ffff, specials: [0; 4] };
        let saved = |kind: i32, action: ActionMark, press: bool| {
            let mut frames = vec![InputFrame::default(); 60];
            if press { frames[0].buttons = 1; }
            let mut take = MacroRecording::new("Take", "13.0.5", StartState::neutral(kind, 0, 1.0), frames).unwrap();
            take.actions = vec![action];
            let root = scratch("reopen");
            let library = Library::new(&root);
            let id = library.save_new(&take).unwrap();
            ClipEditor::macro_file(library.load(&id).unwrap(), true).unwrap().view().move_text
        };
        let mark = |mv: Option<Move>, variant: u8| ActionMark { start: 0, end: Some(20), mv: mv.map(|mv| full_move_key(mv).into()), variant, angle: None };
        let special = |kind: i32| saved(kind, mark(Some(Move::NeutralSpecial), 0), false);
        let rows = |special_name: &str, jab: &str, tilt: &str| {
            assert_eq!(special(6), format!("Start of {special_name}"));
            assert_eq!(saved(6, mark(Some(Move::Jab), 2), false), format!("Start of {jab}"));
            // An old take's side tilt has no variant.
            assert_eq!(saved(6, mark(Some(Move::SideTilt), 0), false), format!("Start of {tilt}"));
            // A mark with no move keeps the name of the input that started it; none is inferred from the button.
            assert_eq!(saved(6, mark(None, 0), true), "Start of Attack");
        };
        PUBLISHED.set_player(Some((6, alpha)));
        rows("Neutral B", "Jab", "Side tilt");
        PUBLISHED.set_on(true);
        // On with no names table: the common names.
        rows("Neutral special", "Jab 2", "Forward tilt");
        PUBLISHED.publish(Box::new(Names::build(&crate::model::move_names::tests::shared_file()).unwrap()));
        rows("Test Pulse", "Jab 2", "Forward tilt");
        // Player 1 is now another fighter, or none: the take's special is not named by it.
        PUBLISHED.set_player(Some((7, alpha)));
        assert_eq!(special(6), "Start of Neutral special");
        PUBLISHED.set_player(None);
        assert_eq!(special(6), "Start of Neutral special");
        // A Mii take that stored no selection, with a Mii of that kind as Player 1: its selection is not the take's.
        PUBLISHED.set_player(Some((0x48, alpha)));
        assert_eq!(special(0x48), "Start of Neutral special");
        PUBLISHED.set_on(false);
        rows("Neutral B", "Jab", "Side tilt");
    }

    fn a_takes_specials_are_named_by_its_own_fighter_or_by_the_common_names() {
        use crate::model::move_names::FighterId;
        let id = |specials| FighterId { chara: 0x12_3456_789a, specials };
        let live = id([2, 0, 1, 2]);
        // An ordinary fighter of the take's kind is the take's fighter.
        assert_eq!(take_identity(Some((6, id([0; 4]))), 6, None), Some(id([0; 4])));
        assert_eq!(take_identity(Some((7, id([0; 4]))), 6, None), None);
        assert_eq!(take_identity(None, 6, None), None);
        // Two Miis of one kind differ by their selection: the take's stored one names it, never the live one.
        for kind in [0x48, 0x49, 0x4a] {
            assert_eq!(take_identity(Some((kind, live)), kind, None), None);
            assert_eq!(take_identity(Some((kind, live)), kind, Some([0, 1, 2, 0])), Some(id([0, 1, 2, 0])));
            assert_eq!(take_identity(Some((kind - 3, live)), kind, Some([0, 1, 2, 0])), None);
        }
        assert_eq!(MII_KINDS, 0x48..=0x4a);
        assert!(include_str!("../mii/mii_setup.rs").contains("if !(0x48..=0x4a).contains(&self.kind)"));
        let panel = include_str!("clip_panel.rs").split("#[cfg(test)]").next().unwrap();
        assert!(panel.contains("saved_namer(recording.start.fighter_kind, recording.mii.as_ref().map(|mii| mii.tokens().specials), recording.recorded_fighter)"));
        assert!(!panel.contains("recorded_identity("));
    }

    /// tr-primary-2 14:33:53Z and 14:41:51Z: a take that stored its fighter, and a Vault capture's side, name their
    /// specials by that fighter whoever Player 1 is. The capture is built by the hook's own events (`VaultArchive`).
    fn a_take_and_a_vault_side_name_their_specials_by_their_own_fighter() {
        use crate::engine::macros::handoff::{VaultArchive, VaultEvent};
        use crate::engine::macros::ReplayFrame;
        use crate::model::move_names::{FighterId, Names, PUBLISHED};
        const CHILD: &str = "TR_MARK_NAMES_CHILD";
        let name = "a_take_and_a_vault_side_name_their_specials_by_their_own_fighter";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "engine::macro_ui::clip_marks::tests::case_table", "--nocapture"])
                .env("TR_TABLE_CASE", name).env(CHILD, "1").output().unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(output.status.success() && stdout.contains("running 1 test"), "{stdout}\n{}", String::from_utf8_lossy(&output.stderr));
            return;
        }
        let id = |suffix: &str| FighterId { chara: crate::model::hash40::hash40(&format!("ui_chara_{suffix}")) & 0xff_ffff_ffff, specials: [0; 4] };
        let (alpha, beta) = (id("alpha"), id("beta"));
        PUBLISHED.publish(Box::new(Names::build(&crate::model::move_names::tests::shared_file()).unwrap()));
        PUBLISHED.set_on(true);
        // Player 1 is another fighter than the one recorded.
        PUBLISHED.set_player(Some((6, alpha)));
        let root = scratch("own-fighter");
        let library = Library::new(root.join("macros"));
        let reopened = |fighter: Option<FighterId>| {
            let mut take = MacroRecording::new("Take", "13.0.5", StartState::neutral(8, 0, 1.0), vec![InputFrame::default(); 60]).unwrap();
            take.actions = vec![ActionMark { start: 0, end: Some(20), mv: Some(full_move_key(Move::UpSpecial).into()), ..ActionMark::default() }];
            take.recorded_fighter = fighter;
            let id = library.save_new(&take).unwrap();
            ClipEditor::macro_file(library.load(&id).unwrap(), false).unwrap().view().move_text
        };
        assert_eq!(reopened(Some(beta)), "Start of Test Lunar Lift");
        assert_eq!(reopened(None), "Start of Up special");
        // The Vault playback hook's events: both entries sampled, the CPU side's identity posted once after its first sample.
        let mut archive = VaultArchive::new();
        archive.apply(VaultEvent::Begin { epoch: 1 }, 0);
        for frame in 0..40u32 {
            for (entry, kind) in [(0, 6), (1, 8)] {
                let sample = ReplayFrame { input: InputFrame { buttons: if frame == 5 { 2 } else { 0 }, ..InputFrame::default() },
                    start: StartState::neutral(kind, 0, 1.0), x: 0.0, y: 0.0 };
                archive.apply(VaultEvent::Sample { epoch: 1, entry, sample, quantized: false }, 0);
                if frame == 0 && entry == 1 { archive.apply(VaultEvent::Fighter { epoch: 1, entry, fighter: beta }, 0); }
            }
        }
        archive.apply(VaultEvent::End { epoch: 1, truncated: false }, 0);
        let mut editor = ClipEditor::new(archive.session.clone(), vec![0]).unwrap();
        assert!(editor.select_entry(1));
        let ClipEffect::Export(exported) = editor.on_action(ClipAction::Confirm) else { panic!("export") };
        assert_eq!((exported.start.fighter_kind, exported.recorded_fighter), (8, Some(beta)));
        for make in [crate::model::recording_menu::Make::Macro, crate::model::recording_menu::Make::Drill] {
            let source = match rr_runtime::generate(&root, (*exported).clone(), "Made", make, Role::Cpu, 1).unwrap() {
                rr_runtime::Generated::Macro(id) => id,
                rr_runtime::Generated::Pass(ask) => ask.source,
            };
            assert_eq!(library.load(&source).unwrap().recorded_fighter, Some(beta), "{make:?}");
        }
        assert!(matches!(editor.on_action(ClipAction::TogglePlay), ClipEffect::Preview(preview) if preview.recorded_fighter == Some(beta)));
        editor.watch(&[Span { start: 6, end: Some(30), mv: Some(Move::UpSpecial), ..Span::default() }]);
        editor.preview_cursor(6);
        assert_eq!(editor.view().move_text, "Start of Test Lunar Lift");
        // The capture's Player 1 side had no identity posted: its export has no key.
        let mut other = ClipEditor::new(archive.session.clone(), vec![0]).unwrap();
        assert!(other.select_entry(0));
        let ClipEffect::Export(plain) = other.on_action(ClipAction::Confirm) else { panic!("export") };
        assert_eq!(plain.recorded_fighter, None);
        // A crop wholly after a transformation is valid; the first form cannot name the later one's special.
        archive.apply(VaultEvent::Begin { epoch: 2 }, 0);
        for frame in 0..40 {
            let sample = ReplayFrame { input: InputFrame::default(), x: 0.0, y: 0.0,
                start: StartState::neutral(if frame < 20 { 8 } else { 9 }, 0, 1.0) };
            archive.apply(VaultEvent::Sample { epoch: 2, entry: 1, sample, quantized: false }, 0);
            if frame == 0 { archive.apply(VaultEvent::Fighter { epoch: 2, entry: 1, fighter: beta }, 0); }
        }
        archive.apply(VaultEvent::End { epoch: 2, truncated: false }, 0);
        let mut later = ClipEditor::new(archive.session, vec![0]).unwrap();
        later.on_action(ClipAction::SelectIn);
        later.on_action(ClipAction::Nudge(20));
        let ClipEffect::Preview(preview) = later.on_action(ClipAction::TogglePlay) else { panic!("later form preview") };
        assert_eq!((preview.start.fighter_kind, preview.recorded_fighter), (9, None));
        later.watch(&[Span { start: 2, end: Some(8), mv: Some(Move::UpSpecial), ..Span::default() }]);
        later.preview_cursor(3);
        assert_eq!(later.view().cursor, 22);
        assert_eq!(later.view().move_text, "Start of Up special");
        // The three producers: each reads the identity once, on the game's own thread.
        for (text, wanted) in [
            (include_str!("../../runtime/features/recording_host.rs"), "rr_host::set_fighter_reader(role_fighter);"),
            (include_str!("../../runtime/features/recording_host.rs"), "super::move_names::fighter_id(body.accessor, kind)"),
            (include_str!("../../runtime/macros/replay_capture.rs"), "VaultEvent::Fighter { epoch: hook.epoch, entry, fighter }"),
            (include_str!("../../runtime/macros/live_replay.rs"), "capture.pool.name(pending.key, entry, fighter)"),
            (include_str!("../rr_host.rs"), "cpu: CaptureOwner::new(start.poses[1].fighter_kind, read_fighter(Role::Cpu))"),
        ] {
            assert!(text.contains(wanted), "missing {wanted}");
        }
    }

    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("the_editor_is_a_strip_at_the_bottom_and_leaves_the_fighters_visible", the_editor_is_a_strip_at_the_bottom_and_leaves_the_fighters_visible),
            ("the_strip_stays_up_while_a_preview_plays", the_strip_stays_up_while_a_preview_plays),
            ("a_zr_tap_swaps_the_play_target_between_the_cpu_and_player_1", a_zr_tap_swaps_the_play_target_between_the_cpu_and_player_1),
            ("swapping_cpu_and_p1_mid_preview_stops_it_and_x_plays_on_the_other_fighter", swapping_cpu_and_p1_mid_preview_stops_it_and_x_plays_on_the_other_fighter),
            ("the_carried_fork_hunks_are_listed_with_commit_file_and_lines", the_carried_fork_hunks_are_listed_with_commit_file_and_lines),
            ("originating_inputs_mark_move_starts_for_any_fighter", originating_inputs_mark_move_starts_for_any_fighter),
            ("a_watched_action_gives_its_start_end_and_name", a_watched_action_gives_its_start_end_and_name),
            ("a_move_restarted_while_busy_is_its_own_recorded_mark", a_move_restarted_while_busy_is_its_own_recorded_mark),
            ("marks_at_frame_0_and_at_the_last_frame_are_kept_and_snapped_to", marks_at_frame_0_and_at_the_last_frame_are_kept_and_snapped_to),
            ("a_crop_or_a_snap_across_a_mark_keeps_the_marks_true_to_the_frames", a_crop_or_a_snap_across_a_mark_keeps_the_marks_true_to_the_frames),
            ("a_clip_line_names_the_play_target_the_marks_and_a_bounded_quoted_move_text", a_clip_line_names_the_play_target_the_marks_and_a_bounded_quoted_move_text),
            ("a_vault_capture_takes_the_zr_tap_and_previews_on_the_chosen_fighter", a_vault_capture_takes_the_zr_tap_and_previews_on_the_chosen_fighter),
            ("the_timeline_draws_move_marks", the_timeline_draws_move_marks),
            ("an_empty_recording_has_no_marks", an_empty_recording_has_no_marks),
            ("the_preview_watch_reads_the_previewing_fighter_only_while_a_preview_runs", the_preview_watch_reads_the_previewing_fighter_only_while_a_preview_runs),
            ("y_snaps_the_selected_handle_to_the_nearest_move_boundary", y_snaps_the_selected_handle_to_the_nearest_move_boundary),
            ("snapping_at_the_first_and_last_frame_stays_inside_the_take", snapping_at_the_first_and_last_frame_stays_inside_the_take),
            ("a_move_with_no_clear_end_snaps_out_to_the_next_start_or_the_takes_end", a_move_with_no_clear_end_snaps_out_to_the_next_start_or_the_takes_end),
            ("scrub_acceleration_is_bounded_from_one_to_180_a_tick", scrub_acceleration_is_bounded_from_one_to_180_a_tick),
            ("an_unsaved_crop_previews_its_own_window_and_writes_no_file", an_unsaved_crop_previews_its_own_window_and_writes_no_file),
            ("the_editor_reads_no_scene_so_lobby_and_training_match", the_editor_reads_no_scene_so_lobby_and_training_match),
            ("a_drill_crop_marks_every_step_and_names_it", a_drill_crop_marks_every_step_and_names_it),
            ("a_combo_crop_marks_every_link_and_names_it", a_combo_crop_marks_every_link_and_names_it),
            ("a_drill_preview_shows_the_strip_with_its_playhead", a_drill_preview_shows_the_strip_with_its_playhead),
            ("a_saved_take_keeps_the_names_of_a_ledge_attack_a_throw_and_a_final_smash", a_saved_take_keeps_the_names_of_a_ledge_attack_a_throw_and_a_final_smash),
            ("a_recorded_take_keeps_its_move_ends_and_out_snaps_to_one_before_any_preview", a_recorded_take_keeps_its_move_ends_and_out_snaps_to_one_before_any_preview),
            ("a_file_without_recorded_marks_falls_back_to_input_starts_and_watched_ends", a_file_without_recorded_marks_falls_back_to_input_starts_and_watched_ends),
            ("a_crop_keeps_the_recorded_marks_inside_its_window", a_crop_keeps_the_recorded_marks_inside_its_window),
            ("a_record_time_trim_keeps_the_marks_inside_the_kept_frames", a_record_time_trim_keeps_the_marks_inside_the_kept_frames),
            ("damaged_marks_in_a_file_are_ignored_not_fatal", damaged_marks_in_a_file_are_ignored_not_fatal),
            ("the_capture_watch_follows_the_recorded_fighter_while_a_take_is_open", the_capture_watch_follows_the_recorded_fighter_while_a_take_is_open),
            ("a_held_attack_on_kirby_is_recorded_as_jab_1_then_jab_2", a_held_attack_on_kirby_is_recorded_as_jab_1_then_jab_2),
            ("a_three_hit_jab_is_jab_1_jab_2_and_jab_3", a_three_hit_jab_is_jab_1_jab_2_and_jab_3),
            ("a_rapid_jab_and_its_finisher_are_named", a_rapid_jab_and_its_finisher_are_named),
            ("an_old_take_with_no_variant_reads_as_before_off_and_through_the_namer_on", an_old_take_with_no_variant_reads_as_before_off_and_through_the_namer_on),
            ("with_move_names_off_a_mark_reads_as_before", with_move_names_off_a_mark_reads_as_before),
            ("an_angled_forward_tilt_is_named_with_its_angle", an_angled_forward_tilt_is_named_with_its_angle),
            ("a_preview_on_the_cpu_or_player_1_names_the_moves_of_a_take_without_marks", a_preview_on_the_cpu_or_player_1_names_the_moves_of_a_take_without_marks),
            ("every_variant_names_a_mark_as_the_combo_views_do", every_variant_names_a_mark_as_the_combo_views_do),
            ("a_throw_names_its_grab_mark", a_throw_names_its_grab_mark),
            ("a_variant_of_the_wrong_shape_reads_as_no_marks", a_variant_of_the_wrong_shape_reads_as_no_marks),
            ("a_crop_and_a_record_time_trim_keep_the_variants", a_crop_and_a_record_time_trim_keep_the_variants),
            ("the_reopened_editor_names_a_special_only_by_the_fighter_it_was_recorded_on", the_reopened_editor_names_a_special_only_by_the_fighter_it_was_recorded_on),
            ("a_takes_specials_are_named_by_its_own_fighter_or_by_the_common_names", a_takes_specials_are_named_by_its_own_fighter_or_by_the_common_names),
            ("a_take_and_a_vault_side_name_their_specials_by_their_own_fighter", a_take_and_a_vault_side_name_their_specials_by_their_own_fighter),
        ]);
    }
    }
}
