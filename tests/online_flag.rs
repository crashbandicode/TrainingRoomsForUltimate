//! The waiting-room clock's call sites. The model (`model::lobby_flag`) decides the flag; these
//! tests lock the wiring that used to run that decision only from Player 1's input poll.

fn source(path: &str) -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path))
        .expect(path)
}

fn body_of<'a>(text: &'a str, signature: &str) -> &'a str {
    let start = text
        .find(signature)
        .unwrap_or_else(|| panic!("{signature} not found"));
    let end = text[start..].find("\n}\n").expect("function end") + start;
    &text[start..end]
}

/// The fighter on-frame hook runs on every station, before the scene is read and before the
/// original frame (death parameters are read in that original). The input poll runs the same
/// clock before any slot test, including when this console maps slot 1.
#[test]
fn the_lobby_clock_runs_from_the_fighter_frame_without_a_slot_gate() {
    let cpu = source("src/runtime/features/cpu.rs");
    let on_frame = body_of(&cpu, "extern \"C\" fn on_frame(");
    let call = on_frame
        .find("crate::runtime::lobby::update()")
        .expect("the fighter frame runs the waiting-room clock");
    let scene = on_frame.find("role_scene()").expect("role_scene");
    let original = on_frame
        .find("let result = original(")
        .expect("original frame");
    assert!(
        call < scene && call < original,
        "the clock runs before Training is read and before the game's frame"
    );
    assert!(
        !on_frame[..call].contains("slot"),
        "no slot test guards the clock"
    );

    let input = source("src/runtime/features/input_delay.rs");
    let note = body_of(&input, "fn note(");
    let call = note
        .find("crate::runtime::lobby::update()")
        .expect("the mapped poll also runs the clock");
    if let Some(slot) = note.find("if slot") {
        assert!(call < slot, "the clock runs before any slot test");
    }
    assert!(
        !note.contains("if slot == 0 {"),
        "slot 0 no longer decides whether the scene is tracked"
    );
}

/// Every `slot == 0` line under `src/runtime`. The lifecycle gate is not among them. The two
/// remaining input-delay gates sit behind `applies`, which is Player 1 and a live practice scene.
#[test]
fn online_slot_assumptions_are_classified() {
    let lines = slot_eq_zero_lines();
    let effect = lines
        .iter()
        .filter(|(file, line)| file.ends_with("effect_texture.rs") && line == "if slot == 0 {")
        .count();
    let chords = lines
        .iter()
        .filter(|(file, line)| file.ends_with("input_delay.rs") && line.contains("controller != 0"))
        .count();
    let plain = lines
        .iter()
        .filter(|(file, line)| file.ends_with("input_delay.rs") && line == "if slot == 0 {")
        .count();
    assert_eq!(effect, 1, "the effect manager's empty address: {lines:?}");
    assert_eq!(
        chords, 1,
        "Player 1's chords while practice is live: {lines:?}"
    );
    assert_eq!(
        plain, 2,
        "Player 1's record, only after applies(); the clock is not a third gate: {lines:?}"
    );
    assert_eq!(
        lines.len(),
        4,
        "a new slot == 0 line needs a verdict in the lane report: {lines:?}"
    );

    let input = source("src/runtime/features/input_delay.rs");
    let apply = body_of(&input, "fn apply(");
    let applies_at = apply.find("input_delay::applies(").expect("applies");
    let first_plain = apply.find("if slot == 0 {").expect("the record gate");
    assert!(
        applies_at < first_plain,
        "the record gates are behind applies(), which is off outside practice"
    );
    let note = body_of(&input, "fn note(");
    if let Some(update) = note.find("lobby::update()") {
        if let Some(slot) = note.find("slot == 0") {
            assert!(update < slot, "the clock is not behind slot 0");
        }
    }
}

/// `is_training` reads the live mode words. The cached flag alone is not the lobby term, and
/// unreadable words (`current()` is `None`) are not a waiting room.
#[test]
fn the_predicate_reads_the_live_scene_not_only_the_cached_flag() {
    let scene = source("src/runtime/scene.rs");
    let facts = body_of(&scene, "pub fn facts(");
    assert!(facts.contains("waiting_room: waiting_room_now()"), "facts pass the live room");
    let now = body_of(&scene, "pub fn waiting_room_now(");
    assert!(now.contains("game_mode::current()"), "the room is the live mode words");
    assert!(now.contains("is_waiting_room("), "the same mask as the clock");
    assert!(now.contains("is_some_and"), "unreadable mode words are not a waiting room");
    let recorder = source("src/runtime/features/recorder.rs");
    let ai = body_of(&recorder, "unsafe fn on_ai_frame(");
    assert!(ai.contains("scene::lobby_term()"), "the recorder does not trust the cached flag alone");
    assert!(!ai.contains("scene::lobby_active()"));
}

/// The carried `lobby::update` delegates to the tested clock. The phrase is the carry-manifest reason.
#[test]
fn the_waiting_room_clock_is_recorded_in_the_carry_manifest() {
    let manifest = source("docs/provenance/pkg6-carry-manifest.json");
    assert!(
        manifest.contains("the waiting-room clock follows the scene on every station"),
        "the lobby.rs carry edit is missing its reason"
    );
    let lobby = source("src/runtime/lobby/training/lobby.rs");
    let update = body_of(&lobby, "pub fn update(");
    assert!(
        update.contains("crate::model::lobby_flag::step("),
        "update() runs the tested step"
    );
    let waiting = body_of(&lobby, "pub fn is_waiting_scene(");
    assert!(
        waiting.contains("lobby_flag::is_waiting_room("),
        "the mask has one copy"
    );
}

fn slot_eq_zero_lines() -> Vec<(String, String)> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let mut files = Vec::new();
    rust_files(&root, &mut files);
    let mut found = Vec::new();
    for path in files {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with('*') {
                continue;
            }
            if trimmed.contains("slot == 0") {
                found.push((path.display().to_string(), trimmed.to_string()));
            }
        }
    }
    found
}

fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display())) {
        let path = entry.unwrap_or_else(|err| panic!("{err}")).path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}
