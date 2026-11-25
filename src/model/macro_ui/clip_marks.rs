use crate::engine::macros::{ActionMark, InputFrame, MacroRecording};
use crate::model::bridge_events::Clip;
use crate::model::combo::{self, Kind, STEPS};
use crate::model::combo_record::{move_key, move_of};
use crate::model::frame_strip::{self, Button, Move, Snap};

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
}

#[derive(Default)]
pub struct Marks {
    list: Vec<Mark>,
    origins: Vec<Option<String>>,
    bindings: Vec<Option<u32>>,
    keys: Vec<Option<String>>,
    recorded: bool,
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
}

pub struct Watch {
    run: u32,
    spans: [Span; WATCH_SPANS],
    len: usize,
    open: bool,
    last: Option<u32>,
    status: i32,
    motion_frame: f32,
}
impl Default for Watch {
    fn default() -> Self {
        Self::new()
    }
}
impl Watch {
    pub const fn new() -> Self {
        Self { run: 0, spans: [Span { start: 0, end: None, mv: None }; WATCH_SPANS],
            len: 0, open: false, last: None, status: 0, motion_frame: 0.0 }
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
                self.spans[self.len] = Span { start: index, end: None, mv: seen.mv };
                self.len += 1;
                self.open = true;
            } else if self.open && self.spans[self.len - 1].mv.is_none() {
                self.spans[self.len - 1].mv = seen.mv;
            }
        } else if self.open {
            self.spans[self.len - 1].end = Some(index);
            self.open = false;
        }
        self.status = seen.status;
        self.motion_frame = seen.motion_frame;
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

    pub fn from_recording(recording: &MacroRecording) -> Self {
        let fallback = Self::from_inputs(recording.frames.iter().copied());
        let mut last = None;
        let valid = !recording.actions.is_empty() && recording.actions.iter().all(|action| {
            let ok = (action.start as usize) < recording.frames.len()
                && last.is_none_or(|last| last < action.start)
                && action.end.is_none_or(|end| end > action.start && (end as usize) <= recording.frames.len());
            last = Some(action.start);
            ok
        });
        if !valid { return fallback; }
        let mut marks = Self::new();
        marks.recorded = true;
        for action in &recording.actions {
            let name = action.mv.as_deref().and_then(move_of).filter(|mv| *mv != Move::Other)
                .map(|mv| mv.name().to_owned())
                .or_else(|| fallback.list.iter().find(|mark| mark.start == action.start).map(|mark| mark.name.clone()))
                .unwrap_or_else(|| "Move".into());
            marks.insert(marks.list.len(), Mark { start: action.start, end: action.end, name }, None, None, action.mv.clone());
        }
        marks
    }

    pub fn recorded(&self) -> bool { self.recorded }

    pub fn capture(frames: &[InputFrame], spans: &[Span]) -> Vec<ActionMark> {
        let mut marks = Self::from_inputs(frames.iter().copied());
        marks.watch(0, spans);
        marks.list.iter().zip(&marks.keys).map(|(mark, key)| ActionMark {
            start: mark.start,
            end: mark.end.filter(|&end| end > mark.start && end <= frames.len() as u32),
            mv: key.clone(),
        }).collect()
    }

    pub fn window(actions: &[ActionMark], from: u32, to: u32) -> Vec<ActionMark> {
        actions.iter().filter(|action| action.start >= from && action.start < to).map(|action| ActionMark {
            start: action.start - from,
            end: action.end.filter(|&end| end <= to).map(|end| end - from),
            mv: action.mv.clone(),
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
            } else {
                self.list.remove(i);
                self.origins.remove(i);
                self.bindings.remove(i);
                self.keys.remove(i);
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
                self.list[index].name = mv.name().into();
                self.keys[index] = Some(move_key(mv).into());
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
            ActionMark { start: 10, end: Some(40), mv: Some(move_key(Move::Jab).into()) },
            ActionMark { start: 90, end: None, mv: None },
        ];
        recording
    }

    fn spans() -> [Span; 2] {
        [Span { start: 12, end: Some(40), mv: Some(Move::Jab) },
         Span { start: 92, end: None, mv: None }]
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
        assert!(adapter.contains("TARGET_TAP.lock().unwrap().tapped(held.zr(), held.other_than_zr())"));
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
        assert_eq!(Marks::from_recording(&mario).list(), Marks::from_recording(&other).list());
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
        let jab = |motion_frame: f32| Seen { busy: true, mv: Some(Move::Jab), status: 0x27, motion_frame, stop: false };
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
        let open = [ActionMark { start: 10, end: Some(30), mv: None }, ActionMark { start: 30, end: None, mv: None }];
        let mut touching = recording();
        touching.frames[30].buttons = 1;
        touching.actions = open.to_vec();
        let touching = Marks::from_recording(&touching);
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
        let spans = [Span { start: 0, end: Some(30), mv: Some(Move::Jab) }, Span { start: 199, end: None, mv: None }];
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
        assert_eq!(Marks::from_recording(&ended).list()[1].end, Some(200));
        ended.actions[1].end = Some(201);
        assert!(!Marks::from_recording(&ended).recorded());
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
        assert_eq!(crop(&mut out_cuts), vec![ActionMark { start: 10, end: None, mv: jab.clone() }]);
        out_cuts.on_action(ClipAction::Snap);
        assert_eq!(out_cuts.view().end, 40);
        assert_eq!(crop(&mut out_cuts), vec![ActionMark { start: 10, end: Some(40), mv: jab.clone() }]);
        let mut in_cuts = ClipEditor::macro_file(recorded(), true).unwrap();
        in_cuts.on_action(ClipAction::SelectIn);
        in_cuts.on_action(ClipAction::Nudge(20));
        assert_eq!(crop(&mut in_cuts), vec![ActionMark { start: 70, end: None, mv: None }]);
        in_cuts.on_action(ClipAction::Snap);
        assert_eq!(in_cuts.view().start, 10);
        assert_eq!(crop(&mut in_cuts), vec![ActionMark { start: 0, end: Some(30), mv: jab }, ActionMark { start: 80, end: None, mv: None }]);
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
        recording.actions = vec![ActionMark { start: 10, end: Some(40), mv: None }, ActionMark { start: 90, end: None, mv: None }];
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
        let capture = ReplaySession { tracks: vec![ReplayTrack { entry: 1, quantized_analog: false, frames: (0..40).map(|_| ReplayFrame {
            input: InputFrame::default(), start: StartState::neutral(74, 0, 1.0), x: 0.0, y: 0.0 }).collect() }],
            truncated: false, breaks: Vec::new() };
        let mut vault = ClipEditor::new(capture, vec![0]).unwrap();
        assert!(bridge_clip(&vault.view(), None, false).text().contains("ctx=replay"));
        assert!(bridge_clip(&vault.view(), None, false).text().contains(" target=cpu marks=0 mark=none move="));
        vault.on_action(ClipAction::SwapTarget);
        assert!(bridge_clip(&vault.view(), None, false).text().contains(" target=p1 "));
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
        editor.watch(&[Span { start: 12, end: Some(80), mv: Some(Move::UpTilt) }]);
        editor.on_action(ClipAction::TogglePlay);
        set_out(&mut editor, 50);
        editor.on_action(ClipAction::Snap);
        assert_eq!(editor.view().end, 40);
    }

    fn snapping_at_the_first_and_last_frame_stays_inside_the_take() {
        let marks = Marks::from_recording(&recorded());
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
        for text in [include_str!("clip_marks.rs"), include_str!("clip_panel.rs"), include_str!("../../editors/clip.rs")] {
            let code = text.split("#[cfg(test)]").next().unwrap();
            assert!(!code.contains("is_in_training") && !code.contains("is_in_lobby") && !code.contains("current_scene"));
        }
        let sample = include_str!("../../runtime/features/input_display.rs");
        let callback = include_str!("../../runtime/features/cpu.rs");
        assert!(sample.contains("rr_host::watch_feed("));
        assert!(callback.contains("input_display::sample_frame("));
    }

    fn a_drill_crop_marks_every_step_and_names_it() {
        let editor = combo_editor(true);
        let view = editor.view();
        let expected = crate::model::recording_menu::combo_link_lines(&combo(true), crate::model::move_names::Namer::recorded(None));
        assert_eq!(view.mark_count, 2);
        assert!(expected[0].starts_with("S1 "));
        assert!(view.move_text.contains(&expected[0]));
        let mut editor = editor;
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(1));
        editor.on_action(ClipAction::Snap);
        assert_eq!(editor.view().start, 1);
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
        assert_eq!(saved.actions, vec![ActionMark { start: 5, end: Some(35), mv: Some(move_key(Move::Jab).into()) }]);
        assert!(Marks::from_recording(&saved).recorded());
        assert_eq!(Marks::window(&recorded().actions, 5, 120).len(), 2);
    }

    fn a_record_time_trim_keeps_the_marks_inside_the_kept_frames() {
        let root = scratch("trim");
        let spans = [Span { start: 12, end: Some(40), mv: Some(Move::Jab) },
            Span { start: 92, end: Some(150), mv: Some(Move::SideTilt) }];
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
        let open = [ActionMark { start: 90, end: None, mv: None }];
        assert_eq!(Marks::window(&open, 5, 91)[0].end, None);
    }

    fn damaged_marks_in_a_file_are_ignored_not_fatal() {
        let root = scratch("bad-marks");
        let library = Library::new(&root);
        let invalid = [
            vec![ActionMark { start: 200, end: None, mv: None }],
            vec![ActionMark { start: 10, end: Some(10), mv: None }],
            vec![ActionMark { start: 90, end: None, mv: None }, ActionMark { start: 10, end: None, mv: None }],
        ];
        for actions in invalid {
            let mut rec = recording();
            rec.actions = actions;
            let id = library.save_new(&rec).unwrap();
            let loaded = library.load(&id).unwrap();
            assert!(loaded.validate().is_ok());
            let marks = Marks::from_recording(&loaded);
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
        assert!(!Marks::from_recording(&loaded.unwrap()).recorded());
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
            ("a_recorded_take_keeps_its_move_ends_and_out_snaps_to_one_before_any_preview", a_recorded_take_keeps_its_move_ends_and_out_snaps_to_one_before_any_preview),
            ("a_file_without_recorded_marks_falls_back_to_input_starts_and_watched_ends", a_file_without_recorded_marks_falls_back_to_input_starts_and_watched_ends),
            ("a_crop_keeps_the_recorded_marks_inside_its_window", a_crop_keeps_the_recorded_marks_inside_its_window),
            ("a_record_time_trim_keeps_the_marks_inside_the_kept_frames", a_record_time_trim_keeps_the_marks_inside_the_kept_frames),
            ("damaged_marks_in_a_file_are_ignored_not_fatal", damaged_marks_in_a_file_are_ignored_not_fatal),
            ("the_capture_watch_follows_the_recorded_fighter_while_a_take_is_open", the_capture_watch_follows_the_recorded_fighter_while_a_take_is_open),
        ]);
    }
    }
}
