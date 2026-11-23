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

fn runtime_sources(dir: &Path) -> Vec<(std::path::PathBuf, String)> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(runtime_sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push((path.clone(), std::fs::read_to_string(path).unwrap()));
        }
    }
    files
}

/// The text of the item that starts with `head`, to its closing brace at column 0.
fn item<'a>(text: &'a str, head: &str) -> &'a str {
    let at = text.find(head).unwrap_or_else(|| panic!("{head} not found"));
    let end = text[at..].find("\n}\n").map_or(text.len(), |e| at + e + 3);
    &text[at..end]
}

#[test]
fn the_role_tabs_draw_the_models_caps() {
    let view = src("src/runtime/ui/view.rs");
    let draw = item(&view, "unsafe fn draw_role_tabs(");
    assert!(draw.contains("p::role_caps(frame.role_keys)"));
    assert!(!draw.contains("const KEYS"));
    assert!(view.contains("self.show(IDX_TITLE, frame.role_tab.is_none())"));
    let recording = src("src/model/recording_menu.rs");
    assert!(item(&recording, "pub fn handle_page(").contains("self.page_overlay_open()"));
}

#[test]
fn recording_roles_reach_the_card_from_one_place() {
    let worker = src("src/runtime/ui/worker.rs");
    let save = item(&worker, "fn save_roles(");
    assert!(save.contains("menu.save_roles(") && save.contains("writer::request_settings()"));
    let handle = item(&worker, "fn handle(");
    let poll = handle.split_once("let Event::Poll(packet)").unwrap().1;
    assert!(poll.find("save_roles(menu);").unwrap() < poll.find("publish_popup(").unwrap());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    for (path, text) in runtime_sources(&root) {
        assert!(!text.contains("set_recording_roles"), "{}", path.display());
        let free_calls = text.match_indices("save_roles(")
            .filter(|(at, _)| *at == 0 || text.as_bytes()[*at - 1] != b'.').count();
        assert_eq!(free_calls, if path.ends_with("ui/worker.rs") { 2 } else { 0 }, "{}", path.display());
    }
}

#[test]
fn the_saved_popup_role_is_adopted_at_boot() {
    let worker = src("src/runtime/ui/worker.rs");
    assert!(item(&worker, "pub fn remember(").contains("rr_host::adopt_saved_role(&settings)"));
    assert!(item(&src("src/runtime/ui/mod.rs"), "pub fn install(").contains("worker::remember(settings.clone())"));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    for (path, text) in runtime_sources(&root) {
        assert_eq!(text.matches("adopt_saved_role(").count(), usize::from(path.ends_with("ui/worker.rs")), "{}", path.display());
    }
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
fn unavailable_egg_zone_scans_fallback_floors_before_spawning() {
    let hitbox = src("src/runtime/features/hitbox.rs");
    let pass = item(&hitbox, "unsafe fn eggs_pass(");
    let fallback = pass.find("run.disable_offstage()").unwrap();
    let floor_scan = pass.find("run.floors_wanted()").unwrap();
    let spawn = pass.find("run.pass(").unwrap();
    assert!(fallback < floor_scan && floor_scan < spawn);
}

#[test]
fn unavailable_egg_zone_messages_remain_visible_with_combo_display_off() {
    let hitbox = src("src/runtime/features/hitbox.rs");
    let pass = item(&hitbox, "unsafe fn eggs_pass(");
    let fallback = &pass[pass.find("run.disable_offstage()").unwrap()..];
    assert!(fallback.contains("notices::post_required(HEADER, reason"));

    let wizard = src("src/runtime/features/egg_zone.rs");
    let frame = item(&wizard, "pub unsafe fn frame(");
    let refusal = &frame[frame.find("if prepare(p1) == Geometry::Unavailable").unwrap()..];
    let refusal = &refusal[..refusal.find("return;").unwrap()];
    assert!(refusal.contains("notices::post_required("));
    assert!(refusal.contains("No usable Off stage zone on this stage."));
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

#[test]
fn edge_juggle_yields_wizard_cpu_control_and_keeps_the_drill_lead_in() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let correction = item(&runtime, "pub(super) unsafe fn ai_correction(");
    let hold = correction.find("run.rounds.phase.held_pad(buttons)").expect("the shared placement hold");
    let wizard = correction.find("run.wizard.is_").expect("the wizard exclusion for live corrections");
    let mode = correction.find("match run.opponent.mode").expect("the opponent mode boundary");
    assert!(wizard < hold && hold < mode, "the shared editor owns the CPU before the drill's lead-in and live dispatch");
}

#[test]
fn edge_wizard_marks_its_cpu_start_readably() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let look = item(&runtime, "pub fn look(");
    assert!(!look.contains("scale: 0.3"));
    let hitbox = src("src/runtime/features/hitbox.rs");
    assert!(item(&hitbox, "pub unsafe fn draw_orbs(").contains("orbs::edge_pointer("));
    let notice = item(&runtime, "fn post_notice(");
    assert!(notice.contains("{} ({x:.0}, {y:.0}) · D-pad choose · A next"));
}

#[test]
fn edge_wizard_notice_keeps_the_pages_the_sealed_rows_read() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let notice = item(&runtime, "fn post_notice(");
    assert!(notice.contains("\"{percent}% · D-pad change · A next\""));
    assert!(notice.contains("\"P1 ({px:.0}, {py:.0}) · Stick move · A next\""));
    let page_text = notice.split_once("let _ = match wizard.step {").unwrap().1;
    for head in ["WizardStep::Area =>", "WizardStep::Percent =>", "WizardStep::Player =>"] {
        let line = page_text.lines().find(|line| line.contains(head)).unwrap();
        assert!(!line.contains("CPU"), "{line}");
    }
    assert!(!page_text.contains("WizardStep::Percent if run.drill == Drill::GuardEdge"));
    assert!(!page_text.contains("WizardStep::Player if run.drill == Drill::GuardEdge"));
}

#[test]
fn edge_wizard_points_only_where_the_notice_shows_the_cpu() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    assert!(item(&runtime, "pub fn look(").contains("wizard.shows_cpu()"));
    let hitbox = src("src/runtime/features/hitbox.rs");
    let draw = item(&hitbox, "pub unsafe fn draw_orbs(");
    assert!(draw.contains(".filter(|_| pointed)"));
    assert!(!draw.contains(".filter(|_| edge_wizard)"));
    let notice = item(&runtime, "fn post_notice(");
    for text in [
        "\"{} ({x:.0}, {y:.0}) · D-pad choose · A next\"",
        "\"CPU x {x:.0} · Height {:+.0} · Stick move · A next\"",
        "\"CPU ({x:.0}, {y:.0}) · Stick move · A next\"",
        "\"CPU ({x:.0}, {y:.0}) · {percent}% · P1 ({px:.0}, {py:.0}) · A/+ start\"",
    ] {
        assert!(notice.contains(text), "{text}");
    }
    let page_text = notice.split_once("let _ = match wizard.step {").unwrap().1;
    for head in ["WizardStep::Percent =>", "WizardStep::Player =>"] {
        let line = page_text.lines().find(|line| line.contains(head)).unwrap();
        assert!(!line.contains("{x:") && !line.contains("{y:"), "{line}");
    }
}

#[test]
fn edge_juggle_remembers_whether_the_start_was_moved() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let poll = item(&runtime, "pub unsafe fn poll(");
    assert!(poll.contains("wizard.unmoved()"));
    assert!(poll.contains("wizard.back_to_defaults(run.defaults)"));
    assert!(!poll.contains("wizard.setup = run.defaults"));
    assert!(item(&runtime, "pub unsafe fn frame(").contains("wizard.remembered("));
}

#[test]
fn edge_juggle_settles_a_hit_player_before_ready() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    assert!(!runtime.contains("fn settle_player"));
    let place = item(&runtime, "unsafe fn try_place(");
    assert!(place.contains("write_player(bodies.p1"));
    assert!(place.contains("Some([x, y])"));
    assert!(!place.contains("state::init_pos(bodies.p1"));
    let on_start = item(&runtime, "unsafe fn on_start(");
    assert!(on_start.contains("-> Option<bool>"));
    assert!(on_start.contains("return Some(false);"));
    assert!(on_start.contains("natives::pos_x(p1) != x"));
    assert!(on_start.contains(".height(x)"));
    assert!(on_start.contains("Some(floor.height(x)? == y)"));
    assert!(!on_start.contains("snap_player("));
    let frame = item(&runtime, "pub unsafe fn frame(");
    let watch = frame.find("placed_player(p1).watch()").expect("watch before the write");
    let place = frame.find("try_place(").expect("start write");
    assert!(watch < place);
    assert!(frame.contains("run.settle = watch.then_some(at);"));
    assert!(!frame.contains("then_some((at, 0))"));
    assert_eq!(frame.matches("run.rounds.player_rewritten()").count(), 1);
    assert!(!runtime.contains("run.settle.next_attempt()"));
    let model = src("src/model/edge_juggle.rs");
    let placed = item(&model, "pub fn placed(").split_once("\n    }\n").expect("placed body").0;
    assert!(placed.contains("self.rewrites = 0;"));
    let restart = item(&model, "pub fn restart_drill(").split_once("\n    }\n").expect("restart body").0;
    assert!(restart.contains("Self::new("));
    assert!(!restart.contains("rewrites"));
    let settle = frame.find("PlacedPlayer::before_ready(").expect("judge before Ready");
    let ready = frame.find("run.rounds.placed()").expect("Ready transition");
    assert!(settle < ready);
    assert!(frame.contains("write_player(p1, at,"));
    assert!(!frame.contains("Placement::Airborne if"));
    assert!(!item(&runtime, "unsafe fn hold_cpu(").contains("p1"));
}

#[test]
fn edge_juggle_keeps_a_settled_selection_through_its_own_form_change() {
    let service = src("src/runtime/features/edge_juggle/practice_cpu.rs");
    assert!(service.contains("picker::tick(settled(), selected, original, "));
    assert!(service.contains("static SETTLED"));
    assert!(service.contains("pub(in crate::runtime) fn cpu_entry_departed()"));
    assert!(!service.contains("fn clear_session"));
    assert!(!service.contains("observed != target"));
    assert!(!service.contains("cpu_loaded_kind"));
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let frame = item(&runtime, "pub unsafe fn frame(");
    assert!(frame.contains("practice_cpu::answers(wanted, kind as u32)"));
    assert!(!frame.contains("wanted != kind as u32"));
    assert!(!frame.contains("followed_swap"));
    assert!(!frame.contains("run.opponent.fighter = Some(kind as u32)"));
    assert!(!frame.contains("cpu_loaded_kind"));
    let roster = src("src/runtime/roster/character_roster.rs");
    assert!(!roster.contains("LOADED_AS"));
    let training = src("src/runtime/lobby/training/mod.rs");
    assert!(!item(&training, "pub(crate) fn clear_session_state(").contains("practice_cpu"));
    let cpu = src("src/runtime/features/cpu.rs");
    let departure = item(&cpu, "pub fn entry_departed(");
    assert!(departure.contains("if owner == 1"));
    assert!(departure.contains("practice_cpu::cpu_entry_departed()"));
    fn find_departure_sites(dir: &Path, found: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let file_type = entry.file_type().unwrap();
            let path = entry.path();
            if file_type.is_dir() {
                find_departure_sites(&path, found);
            } else if file_type.is_file() && path.extension().is_some_and(|ext| ext == "rs")
                && std::fs::read_to_string(&path).unwrap().contains("cpu_entry_departed()")
            {
                found.push(path);
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let mut found = Vec::new();
    find_departure_sites(&root, &mut found);
    assert_eq!(found.len(), 2);
    assert!(found.contains(&root.join("features/cpu.rs")));
    assert!(found.contains(&root.join("features/edge_juggle/practice_cpu.rs")));
    assert!(!Path::new(env!("CARGO_MANIFEST_DIR")).join("src/model/swap/forms.rs").exists());
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

/// rc.15 practice menu, the Parry entry (primary 23:22Z slot 6): Parry practice runs while it is the running game. Its
/// session row is its mode (latched at Start); a settings publish no longer starts it, stops another game or arms the
/// replay host, and every other Start and every Stop disarms it.
#[test]
fn parry_practice_runs_while_it_is_the_running_game() {
    let parry = src("src/runtime/features/parry.rs");
    let publish = item(&parry, "pub fn publish(");
    assert!(!publish.contains("practice::stop") && !publish.contains("set_parry"), "a publish only records the row:\n{publish}");
    assert!(publish.contains("PUBLISHED.store("), "{publish}");
    let start = item(&parry, "pub fn start(");
    assert!(start.contains("SESSION.store(") && start.contains("PUBLISHED.load") && start.contains("set_parry(true)"), "{start}");
    assert!(item(&parry, "pub fn stopped()").contains("set_parry(false)"));
    assert!(item(&parry, "pub fn on()").contains("super::practice::is(PracticeGame::Parry)"));
    let practice = src("src/runtime/features/practice.rs");
    let begin = item(&practice, "pub fn start(");
    assert!(begin.contains("Game::Parry => super::parry::start(how.resume_parry)"), "{begin}");
    assert!(begin.contains("super::parry::stopped()"), "another game's Start disarms Parry:\n{begin}");
    assert!(item(&practice, "pub fn stop()").contains("super::parry::stopped()"));
    assert!(item(&practice, "pub fn running()").contains("Game::Parry => super::parry::on()"));
}

#[test]
fn egg_zone_camera_tracks_the_live_wizard() {
    let camera = src("src/runtime/features/btt_camera.rs");
    let frame = item(&camera, "pub(super) fn update_placement(");
    let lease = frame
        .find("OWNERS.request(model::EGG_ZONE_OWNER, super::egg_zone::wizard_active());")
        .expect("the shared camera consumes the zone wizard's live lease");
    assert!(lease < frame.find("OWNERS.camera_wanted(").unwrap());
    let zone = src("src/runtime/features/egg_zone.rs");
    assert!(item(&zone, "pub fn wizard_active()").contains("LIVE.load(Ordering::Acquire) == epoch()"));
    assert!(item(&zone, "fn close()").contains("LIVE.swap(0,"));
    assert!(item(&zone, "pub fn menu_opened()").contains("close();"));
    assert!(item(&zone, "pub fn reset()").contains("menu_opened();"));
    assert!(item(&zone, "pub unsafe fn prepare(").contains("close();"));
    let zone_frame = item(&zone, "pub unsafe fn frame(");
    assert!(zone_frame.contains("!scene::is_training()") && zone_frame.contains("close();"));
    let poll = item(&zone, "pub unsafe fn poll(");
    for exit in ["Outcome::Cancel", "Outcome::Apply"] {
        let arm = &poll[poll.find(exit).unwrap()..];
        assert!(arm[..arm.find("        }").unwrap()].contains("close();"), "{exit}");
    }
    let cpu = src("src/runtime/features/cpu.rs");
    let on_frame = item(&cpu, "extern \"C\" fn on_frame(");
    let hitbox = on_frame.find("features::hitbox::frame(").unwrap();
    let placement = on_frame.find("btt_camera::update_placement(").unwrap();
    let render = on_frame.find("btt_camera::frame(want_camera)").unwrap();
    assert!(hitbox < placement && placement < render);
}

#[test]
fn the_strip_asks_the_shared_wizard_state() {
    let hud = src("src/runtime/features/hud_draw.rs");
    let draw = item(&hud, "pub unsafe fn draw(");
    assert!(draw.contains("WIZARDS.hides_strip(owners)"));
    assert!(draw.contains("let menu_open = owners.covers_hud()"));
    assert!(draw.contains("let chip = (!menu_open"));
    assert!(draw.contains("combo::hud_tick(strip_shown"));
    assert!(draw.contains("combo::card(strip_shown)"));
    assert!(draw.contains("strip_shown.then(parry::view)"));
    assert!(draw.contains("input_display::hud_tick(\n            strip_shown"));
}

#[test]
fn every_wizard_reports_through_the_one_shared_path() {
    let model = src("src/model/wizard.rs");
    let all = model.split_once("pub const ALL:").unwrap().1.split_once("= [").unwrap().1.split_once("];").unwrap().0;
    let listed: Vec<_> = all.split(',').map(str::trim).filter(|s| !s.is_empty())
        .map(|s| s.strip_prefix("Self::").or_else(|| s.strip_prefix("Kind::")).unwrap()).collect();
    let cases = [
        ("DrillStart", "src/runtime/features/edge_juggle.rs"),
        ("EggZone", "src/runtime/features/egg_zone.rs"),
        ("TechWizard", "src/runtime/features/tech.rs"),
        ("TechPlacing", "src/runtime/features/tech.rs"),
        ("BttPlacing", "src/runtime/features/btt.rs"),
    ];
    assert_eq!(listed.len(), cases.len());
    for kind in &listed { assert_eq!(cases.iter().filter(|(name, _)| name == kind).count(), 1, "{kind}"); }
    for (kind, path) in cases {
        let text = src(path);
        let call = format!("WIZARDS.set_up(Kind::{kind},");
        let args: Vec<_> = text.match_indices(&call).map(|(at, _)| text[at + call.len()..].trim_start()).collect();
        assert!(!args.is_empty(), "{kind}: no shared writer");
        let up = args.iter().any(|arg| arg.starts_with("true") || (!arg.starts_with("false") && !arg.starts_with("0")));
        let down = args.iter().any(|arg| arg.starts_with("false") || (!arg.starts_with("true") && !arg.starts_with("1")));
        assert!(up && down, "{kind}: report both up and down");
    }
    let tech = src("src/runtime/features/tech.rs");
    let edge = src("src/runtime/features/edge_juggle.rs");
    assert!(!tech.contains("static WIZARD_ACTIVE:") && !tech.contains("static PLACING:"));
    assert!(!edge.contains("static WIZARD:"));
    for path in ["src/runtime/features/chords.rs", "src/runtime/ui/input.rs"] {
        let text = src(path);
        for reader in ["tech::wizard_active()", "edge_juggle::wizard_active()", "egg_zone::wizard_active()"] {
            assert_eq!(text.matches(reader).count(), 1, "{path}: {reader}");
        }
        assert_eq!(text.matches("::wizard_active()").count(), 3, "{path}: exactly the three prompt readers");
        assert!(!text.contains("WIZARDS"), "{path}: prompt ownership uses the wizard readers");
    }
}

fn lowers_directly_or_through(body: &str, module: &str, implementation: &str, kind: &str) -> bool {
    let needle = format!("WIZARDS.set_up(Kind::{kind}, false)");
    if body.contains(&needle) { return true; }
    let calls = body.split_once('{').map_or(body, |(_, rest)| rest);
    implementation.lines().filter(|line| (line.starts_with("fn ") || line.starts_with("pub") || line.starts_with("unsafe fn ")) && line.contains("fn ")).any(|line| {
        let Some((_, after_fn)) = line.split_once("fn ") else { return false };
        let name: String = after_fn.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        let call = format!("{name}(");
        let qualified = format!("{module}::{call}");
        let head = format!("{}fn {name}(", line.split_once("fn ").unwrap().0);
        let local = implementation.contains(body) && calls.match_indices(&call).any(|(at, _)| {
            at == 0 || !matches!(calls.as_bytes()[at - 1], b':' | b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')
        });
        (calls.contains(&qualified) || local) && item(implementation, &head).contains(&needle)
    })
}

#[test]
fn guard_the_edge_runtime_enters_and_leaves_through_the_shared_path() {
    let edge = src("src/runtime/features/edge_juggle.rs");
    assert!(!edge.contains("static WIZARD:"));
    let poll = item(&edge, "pub unsafe fn poll(");
    let back = poll.split_once("if key_down(key::B)").unwrap().1.split_once("if key_down(key::X)").unwrap().0;
    let cancel = back.split_once("if !wizard.back()").unwrap().1;
    assert!(cancel.contains("WIZARDS.set_up(Kind::DrillStart, false)"), "Cancel lowers the bit");
    let apply = poll.split_once("run.wizard = None;").unwrap().1;
    assert!(apply.contains("WIZARDS.set_up(Kind::DrillStart, false)"), "Apply lowers the bit");
    let frame = item(&edge, "pub unsafe fn frame(");
    let scene_exit = frame.split_once("if !scene::is_native_training()").unwrap().1.split_once("if published::HOT_SWAP_IN_PROGRESS").unwrap().0;
    assert!(scene_exit.contains("WIZARDS.set_up(Kind::DrillStart, false)"), "scene exit");
    let no_body = frame.split_once("let Some((drill, body))").unwrap().1.split_once("let cpu = body.accessor").unwrap().0;
    assert!(no_body.contains("WIZARDS.set_up(Kind::DrillStart, false)"), "ended drill or missing body");
    let stage_change = frame.split_once("if engine.run.as_ref().is_some_and(|run| run.wizard.is_some() && run.stage_key != stage_key)").unwrap().1
        .split_once("let Some((stage, p1_start)) = setup_stage(p1)").unwrap().0;
    assert!(stage_change.contains("WIZARDS.set_up(Kind::DrillStart, false)"), "stage change clears even if new setup fails");
    assert!(frame.contains("WIZARDS.set_up(Kind::DrillStart, editing)"), "wizard entry and non-editing start");
    let worker = src("src/runtime/ui/worker.rs");
    assert!(item(&worker, "fn handle(").contains("edge_juggle::menu_opened()"));
    assert!(item(&worker, "fn press(").contains("m.take_stop()"));
    assert!(item(&worker, "fn press(").contains("practice::stop()"), "Stop follows menu opening");
    let practice = src("src/runtime/features/practice.rs");
    assert!(lowers_directly_or_through(item(&practice, "pub fn stop()"), "edge_juggle", &edge, "DrillStart"), "Stop ends the drill");
    assert!(frame.contains("if drill.is_none() && !ENGINE_ACTIVE.load(Ordering::Acquire)"), "a live engine keeps its frame until the frame lowers the bit");
    assert!(!src("src/runtime/features/recorder.rs").contains("WIZARDS"), "a session clear leaves each wizard to its own frame");
}

#[test]
fn egg_zone_runtime_enters_and_leaves_through_the_shared_path() {
    let zone = src("src/runtime/features/egg_zone.rs");
    let frame = item(&zone, "pub unsafe fn frame(");
    let entry = frame.find("WIZARDS.set_up(Kind::EggZone, true)").expect("entry");
    assert_eq!(zone.matches("WIZARDS.set_up(Kind::EggZone, true)").count(), 1, "one entry");
    assert_eq!(zone.matches("LIVE.store(").count(), 1, "one lease publication");
    assert!(frame.rfind("compare_exchange(generation, 0").unwrap() < entry);
    assert!(entry < frame.find("LIVE.store(").unwrap());
    let close = item(&zone, "fn close()");
    let swap = close.find("LIVE.swap(0,").unwrap();
    assert!(swap < close.find("WIZARDS.set_up(Kind::EggZone, false)").unwrap());
    assert_eq!(zone.matches("WIZARDS.set_up(Kind::EggZone, false)").count(), 1, "close is the only exit");
    for head in ["pub fn menu_opened()", "pub unsafe fn prepare(", "pub unsafe fn frame("] {
        assert!(item(&zone, head).contains("close();"), "{head}");
    }
    assert!(item(&zone, "pub fn reset()").contains("menu_opened();"));
    let poll = item(&zone, "pub unsafe fn poll(");
    for outcome in ["Outcome::Cancel", "Outcome::Apply"] {
        let arm = poll.split_once(outcome).unwrap().1.split_once("        }").unwrap().0;
        assert!(arm.contains("close();"), "{outcome}");
    }
    let worker = src("src/runtime/ui/worker.rs");
    assert!(item(&worker, "fn handle(").contains("egg_zone::menu_opened()"));
    assert!(item(&worker, "fn press(").contains("m.take_stop()"));
    assert!(item(&worker, "fn press(").contains("practice::stop()"), "Stop follows menu opening");
    let left = frame.split_once("if !scene::is_training()").unwrap().1.split_once("return;").unwrap().0;
    assert!(left.contains("close();"), "leaving Training closes on the next frame");
    let departure = src("src/runtime/lobby/training/mod.rs");
    assert!(item(&departure, "pub(crate) fn clear_session_state()").contains("egg_zone::reset()"), "scene departure");
}

#[test]
fn every_wizard_is_lowered_by_the_menu_and_by_its_own_frame() {
    let tech = src("src/runtime/features/tech.rs");
    let btt = src("src/runtime/features/btt.rs");
    let worker = src("src/runtime/ui/worker.rs");
    let opening = item(&worker, "fn handle(");
    let press = item(&worker, "fn press(");
    assert!(press.contains("m.take_stop()") && press.contains("practice::stop()"));
    for (kind, module, implementation) in [
        ("TechWizard", "tech", &tech),
        ("TechPlacing", "tech", &tech),
    ] {
        assert!(lowers_directly_or_through(item(implementation, "pub fn menu_opened()"), module, implementation, kind), "{kind}: menu opening");
        assert!(opening.contains(&format!("{module}::menu_opened()")), "{kind}: Stop follows opening");
    }
    assert!(opening.contains("btt::menu_opened()"));
    let wizard = item(&tech, "fn wizard_poll(");
    for arm in ["WizardOutcome::Start", "WizardOutcome::Cancelled"] {
        let body = wizard.split_once(arm).unwrap().1.split_once("        }").unwrap().0;
        assert!(body.contains("WIZARDS.set_up(Kind::TechWizard, false)"), "{arm}");
    }
    let frame = item(&tech, "pub unsafe fn frame(");
    let left = frame.split_once("if crate::model::scene::scene(facts).is_none()").unwrap().1.split_once("release_camera_requests();").unwrap().0;
    for kind in ["TechWizard", "TechPlacing"] { assert!(left.contains(&format!("WIZARDS.set_up(Kind::{kind}, false)")), "{kind}: a frame outside the practice scenes"); }
    let interrupted = frame.split_once("if wizard_active() && interrupted").unwrap().1.split_once("BACK_MENU.store").unwrap().0;
    assert!(interrupted.contains("WIZARDS.set_up(Kind::TechWizard, false)"), "an interrupted live draft closes");
    let reset = frame.split_once("if run.epoch != Some(epoch) || reset").unwrap().1.split_once("if view.cancelled").unwrap().0;
    assert!(reset.contains("WIZARDS.set_up(Kind::TechPlacing, false)"), "an interrupted placing view closes");
    let poll = item(&btt, "pub unsafe fn poll(");
    let pass = item(&btt, "unsafe fn pass(");
    assert!(pass.contains("mode.close_menu(on, place)"), "the stopped game leaves Placing");
    assert!(pass.contains("mode.close_menu(false, false)"), "a pass outside the practice scenes leaves Placing");
    assert!(item(&btt, "pub fn reading()").contains("running()") && item(&btt, "pub fn idle()").contains("!reading()"), "a published phase keeps the pass that lowers it");
    let paired = |body: &str| ["phase", "mode.phase()"].iter().any(|value| {
        body.contains(&format!("PHASE.store({value} as u8"))
            && (body.contains(&format!("WIZARDS.set_up(Kind::BttPlacing, {value} == Phase::Placing)"))
                || body.contains(&format!("WIZARDS.set_up(Kind::BttPlacing, matches!({value}, Phase::Placing))")))
    });
    let functions: Vec<_> = btt.lines().filter(|line| (line.starts_with("fn ") || line.starts_with("pub") || line.starts_with("unsafe fn ")) && line.contains("fn ")).filter_map(|line| {
        let (prefix, tail) = line.split_once("fn ")?;
        let name: String = tail.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        let head = format!("{prefix}fn {name}(");
        Some((name, head))
    }).collect();
    let setter = functions.iter().find_map(|(name, head)| {
        let receives_poll = poll.contains(&format!("{name}(mode.phase())"))
            || poll.contains("let phase = mode.phase()") && poll.contains(&format!("{name}(phase)"));
        (paired(item(&btt, head)) && receives_poll && pass.contains(&format!("{name}(phase)"))).then_some(name)
    });
    assert!(paired(poll) && paired(pass) || setter.is_some(), "each phase publication carries the same Placing bit");
    let phase_off = |body: &str| setter.is_some_and(|name| body.contains(&format!("{name}(Phase::Off)")))
        || body.contains("PHASE.store(Phase::Off as u8") && body.contains("WIZARDS.set_up(Kind::BttPlacing, false)");
    for (_, head) in &functions {
        let body = item(&btt, head);
        if body.contains("PHASE.store(") {
            assert!(paired(body) || phase_off(body), "{head}: every phase writer shares its value");
        }
    }
}

#[test]
fn zone_overlay_uses_dark_backing_for_live_and_inset_guides() {
    let zone = src("src/runtime/features/egg_zone.rs");
    let draw = item(&zone, "pub fn draw(");
    assert!(draw.contains("stroke_layers(width, color)"), "world marks draw the dark and colored layers");
    assert!(draw.contains("INSET_BACKING"), "every inset mark has a dark background");
    assert!(item(&zone, "fn prompt(").contains("notices::show_now("), "the built-in notice includes the dark label plate");
}

#[test]
fn location_editors_use_one_camera_and_cpu_session() {
    let camera = src("src/runtime/features/btt_camera.rs");
    let frame = item(&camera, "pub(super) fn update_placement(");
    assert!(frame.contains("model::EGG_ZONE_OWNER, super::egg_zone::wizard_active()"));
    assert!(frame.contains("model::EDGE_JUGGLE_OWNER, super::edge_juggle::placement_active()"));
    let btt = src("src/runtime/features/btt.rs");
    assert!(item(&btt, "unsafe fn pass(").contains("btt::BTT_PLACEMENT_OWNER"));
    let tech = src("src/runtime/features/tech.rs");
    assert!(item(&tech, "pub unsafe fn frame(").contains("model::TECH_PLACEMENT_OWNER"));
    let recorder = src("src/runtime/features/recorder.rs");
    let ai = item(&recorder, "unsafe fn on_ai_frame(");
    let controller = ai.find("ENTRY_CONTROLLER_FIELD").unwrap();
    let placement = ai.find("btt_camera::placement_pad").expect("the shared Place hold");
    let drill = ai.find("drill_owns_pad").unwrap();
    assert!(controller < placement && placement < drill);
    assert!(src("src/runtime/features/reset_hub.rs").contains("btt_camera::interrupt_placement()"));
    assert!(src("src/runtime/ui/worker.rs").contains("btt_camera::interrupt_placement()"));
    let cpu = src("src/runtime/features/cpu.rs");
    assert!(item(&cpu, "fn role_of(").contains("btt_camera::placing()"));
    assert!(item(&cpu, "unsafe fn fighter_frame(").contains("btt_camera::placing()"));
}

#[test]
fn lucas_missing_special_reach_preserves_the_independent_detour_rule() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let recovery = item(&runtime, "unsafe fn recovery_pad(");
    let (before_detour, detour_and_timing) = recovery
        .split_once("    if attack && !can_punish {")
        .expect("the envelope-independent R5 detour");
    let (exception, _) = before_detour
        .rsplit_once("&& special.is_none()")
        .expect("the existing Ness-only stock fallback");
    assert_eq!(
        exception.trim_end().lines().last().unwrap().trim(),
        "if kind == perfect::ness::KIND",
        "an invalid Lucas envelope must not skip the existing R5 detour"
    );
    let (detour, _) = detour_and_timing
        .split_once("    if !recovery::generic_timing_allowed(kind, special, matches!(recovery, Some((_, false)))) {")
        .expect("guard timing corrections after the independent detour");
    assert!(detour.contains("PerfectRule::Detour"));
}
