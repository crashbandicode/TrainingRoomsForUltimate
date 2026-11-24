use crate::engine::macro_ui::{clip_browser, clip_browser::ClipBrowser};
use crate::engine::macros::{CaptureEntry, ReplaySession};
use crate::model::recording_menu::{Make, RecordingFile};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenRequest { pub entry: u8, pub player: bool, pub make: Make, pub play: bool }

pub fn rows(_latest: Option<&ReplaySession>, _previous: Option<&ReplaySession>, _saved: &[CaptureEntry],
    _fighter: impl Fn(i32) -> String) -> Vec<RecordingFile> {
    Vec::new()
}

pub fn editor_label(_entry: i32, fighter: &str, _player: bool) -> String { fighter.to_owned() }

pub fn open_one(_id: &str, _waiting: bool) -> (ClipBrowser, clip_browser::Effect) {
    (ClipBrowser::new(Vec::new(), 0), clip_browser::Effect::None)
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::macro_ui::clip_panel::ClipEditor;
    use crate::engine::macros::{InputFrame, ReplayFrame, ReplayTrack, StartState};

    fn capture(buttons: u32) -> ReplaySession {
        ReplaySession { tracks: vec![ReplayTrack { entry: 0, frames: vec![ReplayFrame {
            input: InputFrame { buttons, ..InputFrame::default() },
            start: StartState::neutral(7, 0, 1.0), x: 0.0, y: 0.0,
        }], quantized_analog: false }], truncated: false, breaks: Vec::new() }
    }

    fn an_empty_capture_is_not_listed_and_does_not_open() {
        let empty = ReplaySession { tracks: Vec::new(), truncated: false, breaks: Vec::new() };
        let good = capture(101);
        let listed = rows(Some(&empty), Some(&good), &[], |kind| kind.to_string());
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "previous");
        assert!(ClipEditor::new(empty, vec![0]).is_err());
    }

    fn the_recordings_rows_are_the_same_in_the_lobby_and_training() {
        let session = capture(1);
        let first = rows(Some(&session), None, &[], |kind| kind.to_string());
        let second = rows(Some(&session), None, &[], |kind| kind.to_string());
        assert_eq!(first, second);
        assert_eq!(first.len(), 1);
        let (mut training, effect) = open_one(&first[0].id, false);
        assert_eq!(effect, clip_browser::Effect::Open("latest".into()));
        assert!(training.opening());
        assert_eq!(training.tick(false, true), clip_browser::Effect::None);
        let (mut waiting, effect) = open_one(&first[0].id, true);
        assert_eq!(effect, clip_browser::Effect::None);
        assert!(waiting.pending());
        assert_eq!(waiting.tick(true, false), clip_browser::Effect::None);
        assert_eq!(waiting.tick(true, true), clip_browser::Effect::Open("latest".into()));
    }

    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("an_empty_capture_is_not_listed_and_does_not_open", an_empty_capture_is_not_listed_and_does_not_open as fn()),
            ("the_recordings_rows_are_the_same_in_the_lobby_and_training", the_recordings_rows_are_the_same_in_the_lobby_and_training as fn()),
        ]);
    }
    }
}
