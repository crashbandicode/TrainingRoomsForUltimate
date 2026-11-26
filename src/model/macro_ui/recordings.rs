use crate::engine::macro_ui::{clip_browser, clip_browser::ClipBrowser, macro_panel::{LibraryRow, Pad}};
use crate::engine::macros::{CaptureEntry, ReplaySession};
use crate::model::recording_menu::{capture_side_label, Make, RecordingFile, RecordingSide};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenRequest { pub entry: u8, pub player: bool, pub make: Make, pub play: bool }

pub fn rows(latest: Option<&ReplaySession>, previous: Option<&ReplaySession>, saved: &[CaptureEntry],
    fighter: impl Fn(i32) -> String) -> Vec<RecordingFile> {
    let sides = |values: Vec<(i32, i32)>| {
        let mut result: Vec<_> = values.into_iter().filter_map(|(entry, kind)| {
            u8::try_from(entry).ok().map(|entry| RecordingSide { entry, fighter: fighter(kind) })
        }).collect();
        result.sort_by_key(|side| side.entry);
        result
    };
    let mut files = Vec::new();
    for (id, name, session) in [("latest", "Latest Vault capture", latest), ("previous", "Previous Vault capture", previous)] {
        if let Some(session) = session {
            let frames = session.tracks.iter().map(|track| track.frames.len()).max().unwrap_or(0);
            let entries = session.tracks.iter().filter_map(|track|
                track.frames.first().map(|frame| (track.entry, frame.start.fighter_kind))).collect();
            if frames > 0 {
                files.push(RecordingFile { id: id.into(), name: name.into(), frames: frames as u32,
                    partial: session.truncated, sides: sides(entries) });
            }
        }
    }
    files.extend(saved.iter().filter(|entry| entry.frames > 0).map(|entry| RecordingFile {
        id: format!("saved:{}", entry.id), name: entry.replay_name.clone(), frames: entry.frames as u32,
        partial: entry.truncated, sides: sides(entry.sides.clone()),
    }));
    files
}

/// The captured side as the editor's strip names it; the strip adds where it plays (`editors::clip`).
pub fn editor_label(entry: i32, fighter: &str) -> String {
    format!("{} {fighter}", capture_side_label(entry))
}

pub fn open_one(id: &str, waiting: bool) -> (ClipBrowser, clip_browser::Effect) {
    let mut browser = ClipBrowser::new(vec![LibraryRow { id: id.into(), label: id.into() }], 0);
    let effect = browser.on_pad(Pad::Confirm, waiting);
    (browser, effect)
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::macro_ui::clip_panel::ClipEditor;
    use crate::engine::macros::{InputFrame, ReplayFrame, ReplayTrack, StartState};

    fn capture(buttons: u32) -> ReplaySession {
        ReplaySession { tracks: vec![ReplayTrack { fighter: None, entry: 0, frames: vec![ReplayFrame {
            pose: crate::engine::macros::PackedPose::NONE,
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

    fn a_captured_side_is_named_independently_of_the_play_target() {
        assert_eq!(editor_label(0, "Fox"), "P1 Fox");
        assert_eq!(editor_label(1, "Fox"), "CPU Fox");
        assert_eq!(editor_label(2, "Fox"), "P3 Fox");
        let session = |entry: i32| ReplaySession { tracks: vec![ReplayTrack { fighter: None, entry, frames: vec![ReplayFrame {
            pose: crate::engine::macros::PackedPose::NONE,
            input: InputFrame::default(), start: StartState::neutral(7, 0, 1.0), x: 0.0, y: 0.0 }], quantized_analog: false }],
            truncated: false, breaks: Vec::new() };
        let mut editor = ClipEditor::new(session(1), vec![0]).unwrap();
        for (player, target) in [(false, "CPU"), (true, "Player 1")] {
            editor.set_play_target(player);
            let mut frame = crate::model::frame::Frame::new();
            crate::editors::clip::clip_frame(&editor.view(), &editor_label(editor.view().entry, "Fox"), 0, &mut frame);
            assert_eq!(String::from_utf16_lossy(frame.rows[1].label.units()), format!("Fighter: CPU Fox · plays on {target}"));
        }
    }

    fn legacy_capture_sources_are_selected_by_entry_not_file_order_or_fighter() {
        use crate::engine::macros::CaptureArchive;
        use crate::engine::macro_ui::clip_panel::{ClipAction, ClipEffect};
        let archive = CaptureArchive::new(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vault-recording-sides"));
        let listed = archive.list().unwrap();
        assert!(listed.skipped.is_empty());
        assert_eq!(listed.entries.len(), 2);
        for item in listed.entries {
            let both = item.replay_name == "same-fighter-entry1-first";
            assert!(both || item.replay_name == "entry1-only");
            assert_eq!(item.sides, if both { vec![(1, 0), (0, 0)] } else { vec![(1, 0)] });
            let saved = archive.load(&item.id).unwrap();
            let mut editor = ClipEditor::new(saved.session, vec![0]).unwrap();
            for entry in [0, 1] {
                assert_eq!(editor.select_entry(entry), entry == 1 || both);
                if entry == 0 && !both { continue; }
                let ClipEffect::Export(take) = editor.on_action(ClipAction::Confirm) else { panic!("no export") };
                let inputs = take.frames.iter().map(|f| (f.buttons, f.lstick_x, f.lstick_y)).collect::<Vec<_>>();
                assert_eq!(inputs, if entry == 0 { [(1, 30, 0), (0, 0, 0), (2, -30, 0)] }
                    else { [(2, 0, 40), (1, 0, -40), (0, 0, 0)] });
            }
        }
    }

    /// The waiting room: three seconds (180 layout ticks) with the CPU never idle end the opening; nothing opens later.
    fn a_waiting_opening_that_never_finds_the_cpu_idle_ends_after_three_seconds() {
        let (mut waiting, effect) = open_one("latest", true);
        assert_eq!(effect, clip_browser::Effect::None);
        for _ in 0..179 {
            assert_eq!(waiting.tick(true, false), clip_browser::Effect::None);
            assert!(waiting.pending());
        }
        assert_eq!(waiting.tick(true, false), clip_browser::Effect::TimedOut);
        assert!(!waiting.pending());
        assert_eq!(waiting.tick(true, true), clip_browser::Effect::None);
    }

    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("a_captured_side_is_named_independently_of_the_play_target", a_captured_side_is_named_independently_of_the_play_target as fn()),
            ("legacy_capture_sources_are_selected_by_entry_not_file_order_or_fighter", legacy_capture_sources_are_selected_by_entry_not_file_order_or_fighter as fn()),
            ("an_empty_capture_is_not_listed_and_does_not_open", an_empty_capture_is_not_listed_and_does_not_open as fn()),
            ("the_recordings_rows_are_the_same_in_the_lobby_and_training", the_recordings_rows_are_the_same_in_the_lobby_and_training as fn()),
            ("a_waiting_opening_that_never_finds_the_cpu_idle_ends_after_three_seconds", a_waiting_opening_that_never_finds_the_cpu_idle_ends_after_three_seconds as fn()),
        ]);
    }
    }
}
