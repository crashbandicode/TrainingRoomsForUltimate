use super::{apply_combo_action, saved_combos_footer, ComboEntry, ComboNotices, SkipPost};
use crate::model::frame::Text;
use crate::model::notify::{Queue, DEFAULT_COLOR};
use crate::model::store::SkippedFile;
use core::fmt::Write;

fn skipped(file: &str) -> Vec<SkippedFile> {
    vec![SkippedFile { file: file.into(), reason: "malformed file".into() }]
}

#[test]
fn saved_combos_show_the_focused_combos_full_label() {
    let label = ComboEntry {
        id: "shared".into(),
        name: "Shared Utilt".into(),
        links: 2,
        recorded_on: Some("Ryu".into()),
    }
    .label();
    assert_eq!(label, "Shared Utilt · 2 links · recorded on Ryu");
    let library = format!("{label}\npage 1/1  skipped 1    A loads, B closes.");
    let footer = saved_combos_footer(&library);
    let mut shown = Text::<160>::new();
    let _ = write!(shown, "{}", footer.lines().next().unwrap_or(""));
    assert_eq!(shown.string_lossy(), label, "the focused row's full label is the footer's first line");
    assert!(footer.contains("A Target"));
    let runtime = include_str!("../runtime/ui/combos.rs");
    assert!(runtime.contains("saved_combos_footer"), "Saved combos draws this footer");
}

#[test]
fn the_target_note_shows_after_a_skipped_file_notice() {
    let mut queue = Queue::new();
    let mut notes = ComboNotices::new();
    let SkipPost::Post(text) = notes.post_skipped(&skipped("typo.json")) else {
        panic!("the first skipped set posts");
    };
    queue.upsert(ComboNotices::SKIPPED_ID, "Saved combos", crate::model::notify::Body::copied(&text), 300, DEFAULT_COLOR);
    for _ in 0..400 {
        let hidden = queue.tick(true, true);
        assert!(!hidden.visible, "an open menu keeps the notice hidden and does not finish it");
    }
    apply_combo_action(&mut queue, "Target: Shared Utilt (2 links, recorded on Ryu)");
    let shown = queue.tick(true, false);
    let body = shown.draw.map(|draw| draw.body.as_str().to_owned()).unwrap_or_default();
    assert!(body.contains("recorded on Ryu"), "the notice drawn once the menu closes was {body:?}");
    assert!(!queue.is_queued(ComboNotices::SKIPPED_ID));
    let worker = include_str!("../runtime/ui/worker.rs");
    assert!(worker.contains("ComboNotices::SKIPPED_ID") && worker.contains("notices::clear"));
    assert!(worker.contains("ComboNotices::ACTION_ID"));
}

#[test]
fn the_skipped_notice_posts_once_per_change() {
    let mut notes = ComboNotices::new();
    assert!(matches!(notes.post_skipped(&skipped("typo.json")), SkipPost::Post(_)));
    assert_eq!(notes.post_skipped(&skipped("typo.json")), SkipPost::Same);
    assert!(matches!(notes.post_skipped(&skipped("other.json")), SkipPost::Post(_)));
    let runtime = include_str!("../runtime/ui/combos.rs");
    assert!(runtime.contains("post_skipped"));
}
