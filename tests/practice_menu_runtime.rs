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
fn edge_wizard_poll_binds_the_side_defaults_and_refuses_a_start_over_the_stage() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let poll = item(&runtime, "pub unsafe fn poll(");
    assert!(poll.contains("wizard.set_side_default(run.stage, false)"));
    let y = poll.find("key_down(key::Y) && run.drill == Drill::GuardEdge && wizard.step == WizardStep::Cpu").unwrap();
    let other = poll[y..].find("wizard.set_side_default(run.stage, true)").unwrap() + y;
    let check = poll.find("!wizard.accepts_cpu(run.stage)").unwrap();
    let refusal = poll[check..].find("notices::refuse(\"Pick a point off the stage\"); // restriction: edge-juggle-start-over-stage").unwrap() + check;
    let back = poll[refusal..].find("return true;").unwrap() + refusal;
    let save = poll.find("run.setup = wizard.setup").unwrap();
    assert!(y < other && other < check && check < refusal && refusal < back && back < save);
}

#[test]
fn edge_wizard_notices_name_three_pages() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let notice = item(&runtime, "fn post_notice(");
    for text in [
        "Edge · 1/3", "Edge · 2/3", "Edge · 3/3",
        "CPU ({x:.0}, {y:.0}) · Stick or D-pad move · A next · X default start · Y other side",
        "CPU x {x:.0} · Height {:+.0} · Stick or D-pad move · A next",
        "P1 ({px:.0}, {py:.0}) · Stick or D-pad move · A next",
        "{percent}% · D-pad change · A next",
        "CPU ({x:.0}, {y:.0}) · {percent}% · P1 ({px:.0}, {py:.0}) · A/+ start",
    ] {
        assert!(notice.contains(text), "{text}");
    }
}

#[test]
fn edge_bests_key_uses_the_bucket() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let frame = item(&runtime, "pub unsafe fn frame(");
    assert!(frame.contains("run.stage.bucket(run.setup.cpu)"));
    assert!(!frame.contains("run.setup.area"));
}

#[test]
fn edge_wizard_marks_its_cpu_start_readably() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let look = item(&runtime, "pub fn look(");
    assert!(!look.contains("scale: 0.3"));
    let hitbox = src("src/runtime/features/hitbox.rs");
    let draw = item(&hitbox, "pub unsafe fn draw_orbs(");
    assert!(draw.contains("orbs::edge_pointer("));
    assert!(draw.contains("looks[4..9].iter_mut().zip(pointer)"));
    assert!(draw.contains("size: 14.0"));
    let notice = item(&runtime, "fn post_notice(");
    assert!(notice.contains("CPU ({x:.0}, {y:.0}) · Stick or D-pad move · A next"));
}

#[test]
fn edge_wizard_notice_keeps_the_pages_the_sealed_rows_read() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let notice = item(&runtime, "fn post_notice(");
    assert!(notice.contains("\"{percent}% · D-pad change · A next\""));
    assert!(notice.contains("\"P1 ({px:.0}, {py:.0}) · Stick or D-pad move · A next\""));
    let page_text = notice.split_once("let _ = match wizard.step {").unwrap().1;
    for head in ["WizardStep::Percent =>", "WizardStep::Player =>"] {
        let line = page_text.lines().find(|line| line.contains(head)).unwrap();
        assert!(!line.contains("CPU"), "{line}");
    }
    assert!(!page_text.contains("WizardStep::Area"));
    assert!(!page_text.contains("WizardStep::Percent if run.drill == Drill::GuardEdge"));
    assert!(!page_text.contains("WizardStep::Player if run.drill == Drill::GuardEdge"));
}

#[test]
fn edge_wizard_points_only_where_the_notice_shows_the_cpu() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    assert!(item(&runtime, "pub fn look(").contains("wizard.marks()"));
    let hitbox = src("src/runtime/features/hitbox.rs");
    let draw = item(&hitbox, "pub unsafe fn draw_orbs(");
    assert!(draw.contains("marks.pointer"));
    assert!(!draw.contains(".filter(|_| edge_wizard)"));
    let notice = item(&runtime, "fn post_notice(");
    for text in [
        "\"CPU x {x:.0} · Height {:+.0} · Stick or D-pad move · A next\"",
        "\"CPU ({x:.0}, {y:.0}) · Stick or D-pad move · A next · X default start · Y other side\"",
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
fn drill_cpu_behavior_between_ai_kinds_skips_the_native_pause() {
    let runtime = src("src/runtime/features/cpu_level.rs");
    let step = item(&runtime, "unsafe fn step_drill_panel(");
    assert!(step.contains(".direct(entry_type, ai_type, cpu, menu, state.drill_pause)"));
    let direct = step.find(".direct(entry_type, ai_type, cpu, menu, state.drill_pause)").expect("direct AI apply");
    let setter = step.find("set_behavior(controller, i32::from(control))").expect("native setter");
    let check = step.find("if field::read_unchecked::<u8>(ai, AI_BEHAVIOR) == control {").expect("setter readback");
    let mirror = step.find("field::write_unchecked(entry, ENTRY_BEHAVIOR, control)").expect("entry mirror");
    let branch = &step[check..];
    let branch = &branch[..branch.find("\n            }\n").expect("readback block")];
    let level = check + branch.find("field::write_unchecked(panel, PANEL_LEVEL, desired.level)").expect("level");
    let behavior = check + branch.find("field::write_unchecked(panel, PANEL_BEHAVIOR, desired.menu)").expect("behavior");
    let done = check + branch.find("return false;").expect("no pause");
    let pulse = step.find("state.drill_pause.pulse(").expect("stock pause fallback");
    assert!(direct < setter && setter < check && check < mirror && mirror < level);
    assert!(level < behavior && behavior < done && done < pulse);
    assert!(branch.contains("field::write_unchecked(entry, ENTRY_BEHAVIOR, control)"));
    let table = src("data/tables/rows_edge_juggle.ROWS.tsv");
    let row = table.lines().find(|line| line.starts_with("\"edge_juggle.ai_set_behavior\"")).unwrap();
    assert!(row.contains("\tSeamKind::Called\t"));
    let chords = src("src/runtime/features/chords.rs");
    let injection = chords.find("if super::cpu_level::poll_drill(").expect("pause poll");
    let body = &chords[injection..];
    let body = &body[..body.find("\n    }\n").unwrap()];
    assert!(body.contains("at.read_volatile() | chords::bit::PLUS"));
    assert_eq!(chords.matches("at.read_volatile() | chords::bit::PLUS").count(), 1);
}

#[test]
fn a_crosshair_below_a_platform_finds_the_floor_under_it() {
    let helpers = src("src/runtime/features/placement.rs");
    assert!(helpers.contains("pub(super) unsafe fn floor_under("));
    let floor = item(&helpers, "pub(super) unsafe fn floor_under(");
    let mut previous = 0;
    for call in [
        "natives::camera_limit_range()",
        "FloorRay::new(body)",
        "ray.floor(x, [top, bottom], platforms)",
        "floor.height(x)",
    ] {
        let at = floor.find(call).expect(call);
        assert!(at >= previous, "{call}");
        previous = at + call.len();
    }
    assert!(floor.contains("if platforms { (y + 1.0).min(top) } else { top }"));
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let snap = item(&runtime, "unsafe fn snap_player(");
    assert!(snap.contains("placement::floor_under(p1, [x, y], platforms)"));
    assert!(!snap.contains("FloorRay::new("));
    assert!(!snap.contains("ray.floor("));
    assert!(snap.contains("x.clamp(stage.ledges[0][0], stage.ledges[1][0])"));
}

#[test]
fn placement_helpers_keep_the_drills_native_write_order() {
    fn ordered(body: &str, calls: &[&str]) {
        let mut previous = 0;
        for call in calls {
            assert_eq!(body.matches(call).count(), 1, "{call}");
            let at = body.find(call).unwrap();
            assert!(at >= previous, "{call}");
            previous = at + call.len();
        }
    }
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let helpers = src("src/runtime/features/placement.rs");
    let stand = item(&helpers, "pub(super) unsafe fn stand(");
    ordered(stand, &["state::clear_speed_all(", "state::init_pos(", "state::set_lr(",
        "state::set_situation_kind(", "state::status_request("]);
    assert_eq!(stand.matches("state::").count(), 6);
    assert!(stand.contains("konst(C::SituationGround)"));
    assert!(stand.contains("konst(C::StatusWait)"));
    let lift = item(&helpers, "pub(super) unsafe fn lift(");
    ordered(lift, &["state::clear_speed_all(", "state::set_pos(", "state::set_lr(",
        "state::set_situation_kind("]);
    assert_eq!(lift.matches("state::").count(), 5);
    assert!(lift.contains("konst(C::SituationAir)"));
    let fall = item(&helpers, "pub(super) unsafe fn fall(");
    ordered(fall, &["state::status_request("]);
    assert_eq!(fall.matches("state::").count(), 1);
    assert!(fall.contains("konst(C::StatusFall)"));
    let hold = item(&helpers, "pub(super) unsafe fn hold(");
    ordered(hold, &["state::set_sleep(", "state::clear_speed_all(", "state::set_pos("]);
    assert_eq!(hold.matches("state::").count(), 4);
    assert!(!hold.contains("status_request"));
    let place = item(&runtime, "unsafe fn try_place(");
    ordered(place, &["placement::stand(bodies.p1", "placement::lift(bodies.cpu",
        "natives::set_int(", "state::heal(", "state::add_damage(", "placement::fall(bodies.cpu)"]);
    assert!(!runtime.contains("fn write_player("));
    assert!(!runtime.contains("fn hold_cpu("));
}

#[test]
fn edge_juggle_settles_a_hit_player_before_ready() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    assert!(!runtime.contains("fn settle_player"));
    let place = item(&runtime, "unsafe fn try_place(");
    assert!(place.contains("placement::stand(bodies.p1"));
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
    assert!(frame.contains("placement::stand(p1, at,"));
    assert!(!frame.contains("Placement::Airborne if"));
    let placement = src("src/runtime/features/placement.rs");
    assert!(!item(&placement, "pub(super) unsafe fn hold(").contains("p1"));
    assert!(item(&placement, "pub(super) unsafe fn stand(").contains("state::init_pos(body"));
    assert!(frame.contains("placement::hold(cpu, run.setup.cpu)"));
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
