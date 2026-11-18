//! rc.15 practice menu, the runtime side of tr-review-astra-3's 16:37Z findings (menu d85cda42), read from the sources the
//! Switch build compiles (the runtime is not built on the host):
//! 1. a menu visit and B keep a running Tech practice's session (the open branch suspends, it never stops);
//! 2. settings changed while a game runs apply at its next Start: every rule a running game reads is the one latched by
//!    that Start (Break the Targets projectiles, the Eggs-ercise rows, CPU during a game, the Tech camera), never the one
//!    the menu close publishes.
use std::path::Path;

fn src(path: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap()
}

/// The text of the item that starts with `head`, to its closing brace at column 0.
fn item<'a>(text: &'a str, head: &str) -> &'a str {
    let at = text.find(head).unwrap_or_else(|| panic!("{head} not found"));
    let end = text[at..].find("\n}\n").map_or(text.len(), |e| at + e + 3);
    &text[at..end]
}

#[test]
fn a_menu_visit_suspends_tech_practice_and_b_resumes_it() {
    let tech = src("src/runtime/features/tech.rs");
    let frame = item(&tech, "pub unsafe fn frame(");
    let open = &frame[frame.find("        if open {").expect("the open branch")..];
    let open = &open[..open.find("\n        }\n").expect("its end")];
    assert!(open.contains("controller.pause_for_menu("), "the open branch suspends the session:\n{open}");
    assert!(!open.contains("controller.stop("), "the open branch must not stop the session:\n{open}");
    assert!(open.contains("run.pending = true"), "the suspended session asks for its next attempt:\n{open}");
}

#[test]
fn break_the_targets_projectiles_are_the_rule_latched_at_start() {
    let btt = src("src/runtime/features/btt.rs");
    assert!(item(&btt, "pub fn projectiles()").contains("PROJECTILES_RUN.load"), "the hit path reads the run's rule");
    assert!(item(&btt, "pub fn start(").contains("PROJECTILES_RUN.store(PROJECTILES.load"), "Start latches the rule");
    assert!(!item(&btt, "pub fn publish(").contains("PROJECTILES_RUN"), "a menu close publishes, it does not apply");
    assert_eq!(btt.matches("PROJECTILES_RUN.store").count(), 1);
}

#[test]
fn eggsercise_rows_and_cpu_during_a_game_are_latched_at_start() {
    let hitbox = src("src/runtime/features/hitbox.rs");
    let pass = item(&hitbox, "unsafe fn eggs_pass(");
    // Every game's Start latches the rows (Break the Targets reads Breaks on too: btt.rs `egg_rule()`), in one place.
    let started = item(&hitbox, "pub fn practice_started()");
    assert!(started.contains("EGG_LIVE.store(EGG_ROWS.load"), "a Start latches the rows:\n{started}");
    assert_eq!(hitbox.matches("EGG_LIVE.store").count(), 1, "one latch site");
    assert!(src("src/runtime/features/btt.rs").contains("super::hitbox::egg_rule()"), "BTT's Breaks on is the latched one");
    assert!(pass.contains("EGG_LIVE.load"), "the run reads the latched rows");
    assert!(!pass.contains("EGG_ROWS.load(Ordering::Acquire).to_le_bytes()"), "never the published rows");
    assert!(pass.contains("PRACTICE_HIDE_RUN.load"), "CPU during a game is the Start's");
    assert!(item(&hitbox, "pub(crate) fn egg_rule()").contains("EGG_LIVE.load"), "Breaks on is the run's");
    let publish = item(&hitbox, "pub fn publish(");
    assert!(!publish.contains("EGG_LIVE") && !publish.contains("PRACTICE_HIDE_RUN"), "a close publishes only");
    let practice = src("src/runtime/features/practice.rs");
    assert!(item(&practice, "pub fn start(").contains("hitbox::practice_started()"), "every Start latches CPU during a game");
    assert!(item(&hitbox, "pub fn practice_started()").contains("PRACTICE_HIDE_RUN.store(PRACTICE_HIDE.load"));
}

#[test]
fn the_tech_camera_follows_the_running_session() {
    let tech = src("src/runtime/features/tech.rs");
    let frame = item(&tech, "pub unsafe fn frame(");
    let request = &frame[frame.rfind("btt_camera::request(").expect("the camera request")..];
    assert!(request.contains("camera == 1"), "{request}");
    assert!(!request.contains("config.camera"), "the camera is the running session's, not the published one:\n{request}");
    assert!(frame.contains("camera = run.config.camera"), "the session's camera comes from its latched config");
}

/// tr-review / gate ed64bb9d (release and DIAG exit 101): Mash out reads the running game through one function, its
/// submodules included, and the models Game never shadows the runtime struct Game of mashout.rs.
#[test]
fn mash_out_runs_while_it_is_the_running_game() {
    let root = src("src/runtime/features/mashout.rs");
    assert!(root.contains("struct Game {"), "the runtime struct this import must not clash with");
    assert!(!root.contains("use crate::model::practice_games::Game;"), "imported under another name");
    assert!(item(&root, "fn wanted()").contains("super::practice::is(PracticeGame::Mashout)"));
    for path in ["src/runtime/features/mashout.rs", "src/runtime/features/mashout/worker.rs", "src/runtime/features/mashout/interposer.rs"] {
        let text = src(path);
        assert!(!text.contains("WANTED"), "{path} still reads the removed row-On flag");
    }
    assert!(src("src/runtime/features/mashout/worker.rs").contains("let wanted = wanted();"));
    assert!(src("src/runtime/features/mashout/interposer.rs").contains("let wanted = wanted()"));
}
