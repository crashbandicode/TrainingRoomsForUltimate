use crate::engine::macros::{ActionMark, InputFrame, MacroRecording};
use crate::model::frame_strip::Move;

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

pub struct Watch;
impl Watch {
    pub const fn new() -> Self { Self }
    pub fn feed(&mut self, _run: u32, _index: u32, _busy: bool, _mv: Option<Move>) {}
    pub fn run(&self) -> u32 { 0 }
    pub fn spans(&self) -> &[Span] { &[] }
}

pub struct Marks { list: Vec<Mark> }
impl Marks {
    pub fn new() -> Self { Self { list: Vec::new() } }
    pub fn from_inputs(_frames: impl Iterator<Item = InputFrame>) -> Self { Self::new() }
    pub fn from_units(_names: impl Iterator<Item = String>) -> Self { Self::new() }
    pub fn from_recording(_recording: &MacroRecording) -> Self { Self::new() }
    pub fn recorded(&self) -> bool { false }
    pub fn capture(_frames: &[InputFrame], _spans: &[Span]) -> Vec<ActionMark> { Vec::new() }
    pub fn window(_actions: &[ActionMark], _from: u32, _to: u32) -> Vec<ActionMark> { Vec::new() }
    pub fn list(&self) -> &[Mark] { &self.list }
    pub fn forget_watched(&mut self, _from: u32, _to: u32) {}
    pub fn watch(&mut self, _base: u32, _spans: &[Span]) {}
    pub fn snap_in(&self, _at: u32) -> Option<u32> { None }
    pub fn snap_out(&self, _at: u32, _total: u32) -> Option<u32> { None }
    pub fn text(&self, _at: u32, _out_handle: bool) -> String { String::new() }
    pub fn shown(&self, _cursor: u32, _total: u32, _width: u16) -> ([(u16, u16); SHOWN_MARKS], u8) {
        ([(0, 0); SHOWN_MARKS], 0)
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

    fn save_capture(root: &std::path::Path, frames: &[InputFrame], spans: &[Span], trim: bool) -> String {
        let mut capture = Capture::idle();
        rr_runtime::toggle_record(&mut capture, Role::Player);
        for &frame in frames { rr_runtime::push_frame(&mut capture, Role::Player, frame); }
        let RecordStop::Saved { id, .. } = rr_runtime::finish_recording(
            root, &capture, saved_state(0), saved_state(1), (0.0, 0.0), 1, trim, spans).unwrap()
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
        assert!(CLIP_STRIP[3] <= 1080.0 * 0.3);
        let lines = [CLIP_ROW_Y[0], CLIP_ROW_Y[1], CLIP_BAR_Y, CLIP_HELP_Y[0], CLIP_HELP_Y[1], CLIP_HINT_Y];
        assert!(lines.iter().all(|&y| y >= CLIP_STRIP[1] - CLIP_STRIP[3] / 2.0 && y <= CLIP_STRIP[1] + CLIP_STRIP[3] / 2.0));
        assert!(lines.windows(2).all(|p| p[0] - p[1] >= 44.0));
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

    fn up_and_down_swap_the_play_target_between_the_cpu_and_player_1() {
        let mut editor = ClipEditor::macro_file(recording(), false).unwrap();
        for action in [ClipAction::PreviousFighter, ClipAction::NextFighter] {
            editor.on_action(action);
            let expected = matches!(action, ClipAction::PreviousFighter);
            assert_eq!(editor.context(), ClipContext::Macro { player: expected });
        }
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
        watch.feed(3, 12, true, None);
        watch.feed(3, 13, true, Some(Move::Jab));
        watch.feed(3, 40, false, None);
        assert_eq!(watch.run(), 3);
        assert_eq!(watch.spans(), &spans()[..1]);
        let mut marks = Marks::from_inputs(inputs().into_iter());
        marks.watch(0, watch.spans());
        assert_eq!((marks.list()[0].start, marks.list()[0].end, marks.list()[0].name.as_str()), (10, Some(40), "Jab"));
        marks.watch(0, watch.spans());
        assert_eq!(marks.list().len(), 2);
        watch.feed(4, 0, true, Some(Move::SideTilt));
        assert_eq!(watch.spans().len(), 1);
        for frame in 1..100 { watch.feed(4, frame, frame % 2 == 0, None); }
        assert_eq!(watch.spans().len(), WATCH_SPANS);
        assert_eq!(watch.spans()[0].mv, Some(Move::SideTilt));
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
        assert_eq!(view.mark_count, 2);
        assert!(view.move_text.contains("Step 1"));
        let mut editor = editor;
        editor.on_action(ClipAction::SelectIn);
        editor.on_action(ClipAction::Nudge(1));
        editor.on_action(ClipAction::Snap);
        assert_eq!(editor.view().start, 1);
    }

    fn a_combo_crop_marks_every_link_and_names_it() {
        let editor = combo_editor(false);
        let view = editor.view();
        let expected = crate::model::recording_menu::combo_link_lines(&combo(false));
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
        assert_eq!(editor.view().cursor, 36);
    }

    fn a_recorded_take_keeps_its_move_ends_and_out_snaps_to_one_before_any_preview() {
        let root = scratch("first-open");
        let id = save_capture(&root, &inputs(), &spans(), false);
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
        let spans = [Span { start: 12, end: Some(150), mv: Some(Move::Jab) }];
        let id = save_capture(&root, &inputs(), &spans, true);
        let loaded = Library::new(root.join("macros")).load(&id).unwrap();
        assert_eq!(loaded.frames.len(), 91);
        assert_eq!(loaded.actions.first().map(|mark| (mark.start, mark.end)), Some((10, None)));
        let drill = rr_runtime::save_drill_take(&root, inputs(), StartState::neutral(0, 0, 1.0), true, 2, &spans).unwrap().unwrap();
        let drill = Library::new(root.join("macros")).load(&drill).unwrap();
        assert_eq!(drill.frames.len(), 91);
        assert_eq!(drill.actions.first().map(|mark| (mark.start, mark.end)), Some((10, None)));
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
            ("up_and_down_swap_the_play_target_between_the_cpu_and_player_1", up_and_down_swap_the_play_target_between_the_cpu_and_player_1),
            ("swapping_cpu_and_p1_mid_preview_stops_it_and_x_plays_on_the_other_fighter", swapping_cpu_and_p1_mid_preview_stops_it_and_x_plays_on_the_other_fighter),
            ("the_carried_fork_hunks_are_listed_with_commit_file_and_lines", the_carried_fork_hunks_are_listed_with_commit_file_and_lines),
            ("originating_inputs_mark_move_starts_for_any_fighter", originating_inputs_mark_move_starts_for_any_fighter),
            ("a_watched_action_gives_its_start_end_and_name", a_watched_action_gives_its_start_end_and_name),
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
