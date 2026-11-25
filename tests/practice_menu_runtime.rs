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

fn member<'a>(text: &'a str, head: &str) -> &'a str {
    item(text, head).split_once("\n    }\n").expect(head).0
}

#[test]
fn f9_runtime_name_wiring() {
    let named = src("src/runtime/macros/named_macros.rs");
    let service = item(&named, "pub fn service_keyboard(");
    assert!(service.contains("NameJob::complete("));
    assert!(service.contains("job.owner == Some(crate::runtime::features::recording_host::NAME_TOKEN)"));
    assert!(item(&named, "fn retire_name(").contains("NameJob::retire(&mut pending, keep_waiting)"));
    let request = item(&named, "fn request_native_name(");
    assert!(request.contains("external_name_token()") && request.contains("owner"));
    let discard = item(&named, "pub fn discard_external_name(");
    assert!(discard.contains("NameJob::retire_owner(&mut pending, token)"));
    assert!(discard.contains("native_keyboard::interrupt(id)") && discard.contains("ui.on_pad(Pad::Back)"));
    assert!(!discard.contains("naming_pending") && !discard.contains("{ on_pad("));
    assert!(!discard.lines().any(|line| line.trim_start().starts_with("on_pad(")));
    let cancelled = service.split_once("KeyboardResult::Cancelled => {").unwrap().1
        .split_once("KeyboardResult::Failed").unwrap().0;
    assert!(cancelled.contains("job.waiting || job.replay || hidden"));
    assert!(cancelled.contains("on_pad(Pad::Back)"));
    let failed = service.split_once("KeyboardResult::Failed(code) => {").unwrap().1
        .split_once("KeyboardResult::Interrupted").unwrap().0;
    assert!(failed.contains("if hidden") && failed.contains("on_pad(Pad::Back)"));
    let host = src("src/runtime/features/recording_host.rs");
    let apply = item(&host, "fn apply(");
    assert!(apply.find("offer_external_name_titled(").unwrap() < apply.find("NAME_ASKED.store(true, Ordering::Release)").unwrap());
    let settle = item(&host, "pub(crate) fn settle_name(");
    assert!(settle.contains("confirmed_name_waiting(NAME_TOKEN)"));
    assert!(settle.contains("external_name_open(NAME_TOKEN)"));
    assert!(settle.contains("discard_external_name(NAME_TOKEN)"));
    let worker = src("src/runtime/ui/worker.rs");
    let run = item(&worker, "fn run()");
    assert!(run.find("port::worker_tick()").unwrap() < run.find("settle_name(shown && naming) || (naming && !shown)").unwrap());
    assert!(run.contains("menu.recording_mut().cancel_name()"));
    assert!(run.contains("publish_frame(&session)"));
}

#[test]
fn gj1_runtime_return_wiring() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let frame = item(&runtime, "pub unsafe fn frame(");
    assert!(frame.contains("let p1_situation = natives::situation_kind(p1);"));
    assert!(frame.contains("(p1_situation == konst(C::SituationCliff)).then(|| side(natives::pos_x(p1)))"));
    let obs = frame.split_once("let obs = Observation {").unwrap().1.split_once("\n    };").unwrap().0;
    assert!(obs.contains("player_back: p1_situation == konst(C::SituationGround) || p1_situation == konst(C::SituationCliff)"));
    assert!(obs.contains("player_ko: player_ko(p1)"));
    assert!(frame.contains("if before == Phase::Live && run.rounds.phase != Phase::Live {\n            run.route_history.complete();"));
    assert!(item(&runtime, "fn post_notice(").contains("let outcome = phase.card();"));
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
fn the_popup_y_commits_place_first_to_the_live_settings() {
    let worker = src("src/runtime/ui/worker.rs");
    let press = item(&worker, "fn press(");
    let popup = press.split_once("if popup && !open {").unwrap().1.split_once("        return;").unwrap().0;
    assert!(popup.contains("take_place_first()"));
    assert!(popup.contains("commit_place_first("));
    assert!(press.contains("let _ = m.take_place_first();"));
    let commit = item(&worker, "fn commit_place_first(");
    let publish = commit.find("published::publish(").unwrap();
    let replace = commit.find("replace_menu_settings(").unwrap();
    let request = commit.find("writer::request_settings()").unwrap();
    assert!(publish < replace && replace < request);
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
fn the_drills_run_in_a_waiting_room_against_the_first_training_partner() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    assert!(!runtime.contains("scene::is_native_training()"));
    let cpu_entry = item(&runtime, "fn cpu_entry(");
    assert!(cpu_entry.contains("LOBBY_CPU_ENTRY") && cpu_entry.contains("scene::practice_scene()"));
    assert!(member(&runtime, "unsafe fn current()").contains("natives::module_accessor(cpu_entry())"));
    assert!(item(&runtime, "pub unsafe fn frame(").contains("natives::is_operation_cpu(cpu_entry())"));
    assert!(item(&runtime, "fn controls_cpu(").contains("natives::module_accessor(cpu_entry())"));

    let zoom = src("src/runtime/features/edge_juggle/zoom.rs");
    assert!(zoom.contains("natives::fighter_entry(super::cpu_entry())"));
    assert!(!zoom.contains("fighter_entry(1)"));
    let picker = src("src/runtime/features/edge_juggle/practice_cpu.rs");
    assert!(picker.contains("scene::practice_scene().is_some()"));
    assert!(!picker.contains("is_native_training"));

    let level = src("src/runtime/features/cpu_level.rs");
    let poll = item(&level, "fn poll_drill_locked(");
    assert!(poll.contains("scene::practice_scene()"));
    let partner = poll.find("step_drill_partner(").unwrap();
    let panel = poll.find("step_drill_panel(").unwrap();
    assert!(partner < panel);
    let step = item(&level, "unsafe fn step_drill_partner(");
    assert!(step.contains("state.drill_partner.step("));
    assert!(step.contains("state.drill_partner.absent("));
    assert!(!step.contains("PartnerLease::new()"));
    assert!(step.contains("AI_SET_BEHAVIOR") && step.contains("ENTRY_BEHAVIOR"));
    assert!(!step.contains("PANEL_"));
    assert!(item(&level, "pub(crate) unsafe fn frame(").contains("rules::drill_target("));

    let reset = src("src/runtime/features/states/quick_reset.rs");
    let request = item(&reset, "fn request(");
    assert!(request.find("begin_temporary(").unwrap() < request.find("edge_juggle::restart();").unwrap());
    let cpu = src("src/runtime/features/cpu.rs");
    assert!(item(&cpu, "pub fn entry_departed(").contains("owner == LOBBY_CPU_ENTRY as u32"));

    let rows = src("data/tables/rows_edge_juggle.ROWS.tsv");
    let scope = "\tScopes(Scopes::TRAINING.0 | Scopes::LOBBY_WAITING.0)\t";
    for row in rows.lines().filter(|line| line.starts_with("\"edge_juggle.attack_forecast_")) {
        assert!(row.contains(scope), "{row}");
    }
    for name in ["ai_set_behavior", "special_available_snapshot", "special_available", "escape_slide_distance",
        "escape_cancel_frame", "special_height", "special_length"] {
        let prefix = format!("\"edge_juggle.{name}\"");
        assert!(rows.lines().find(|line| line.starts_with(&prefix)).unwrap().contains(scope), "{name}");
    }
    for row in rows.lines().filter(|line| line.starts_with("\"edge_juggle.training_")
        || line.starts_with("\"edge_juggle.live_") || line.starts_with("\"edge_juggle.pause_hud_")) {
        assert!(row.contains("\tScopes::TRAINING\t"), "{row}");
    }
    let zoom_rows = src("data/tables/rows_edge_zoom.ROWS.tsv");
    assert_eq!(zoom_rows.lines().filter(|line| line.starts_with("\"edge_zoom.") && line.contains(scope)).count(), 3);
}

#[test]
fn edge_wizard_poll_binds_the_side_defaults_and_refuses_a_start_over_the_stage() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let poll = item(&runtime, "pub unsafe fn poll(");
    assert!(poll.contains("wizard.set_side_default(run.stage, false)"));
    let y = poll.find("key_down(key::Y) && run.drill == Drill::GuardEdge && wizard.step == WizardStep::Cpu").unwrap();
    let other = poll[y..].find("wizard.set_side_default(run.stage, true)").unwrap() + y;
    // A on the CPU page only records the point; the next fighter frame asks the off-stage rule and refuses there.
    let check = poll.find("run.confirm_cpu = Some(wizard.setup.cpu);").unwrap();
    let back = poll[check..].find("return true;").unwrap() + check;
    let save = poll.find("run.setup = wizard.setup").unwrap();
    assert!(y < other && other < check && check < back && back < save);
    let frame = item(&runtime, "pub unsafe fn frame(");
    let asked = frame.find("if let Some(at) = run.confirm_cpu.take()").unwrap();
    assert!(frame[asked..].contains("Drill::GuardEdge => notices::refuse(\"Pick a point off the stage\"), // restriction: edge-juggle-start-over-stage"));
}

#[test]
fn every_refusal_goes_through_the_queue_s_refusal_post() {
    let notices = src("src/runtime/features/notices.rs");
    let refuse = item(&notices, "pub fn refuse(");
    assert!(refuse.contains("send(Mail::Refuse(\"Input Recording\", text, 120));"));
    assert!(!refuse.contains("post("));
    let take = item(&notices, "fn take_mail(");
    assert!(take.contains("Mail::Refuse(header, text, length) => queue.post_refusal(header, Body::copied(text), length, DEFAULT_COLOR)"));
    let save = item(&notices, "pub fn save_state_refused(");
    assert!(save.contains("send(Mail::Refuse(\"Save State\", text, 150));"));
    assert!(!save.contains("post("));
    let chords = src("src/runtime/features/chords.rs");
    let refused = chords.find("if recorder_refused {").unwrap();
    assert!(chords[refused..].contains("notices::refuse(\"Stop the drill to start recording or play inputs\")"));
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let poll = item(&runtime, "pub unsafe fn poll(");
    assert!(item(&runtime, "pub unsafe fn frame(").contains("notices::refuse(\"Pick a point off the stage\"), // restriction: edge-juggle-start-over-stage"));
    assert!(!poll.contains("notices::show_now("));
}

#[test]
fn the_bridge_publishes_the_focused_list_and_still_omits_rows() {
    let worker = src("src/runtime/ui/worker.rs");
    let publish = item(&worker, "fn publish_bridge_state(");
    assert!(publish.contains("published_list("));
    assert!(publish.contains("flow::model_takes_presses("));
    assert!(publish.contains("POPUP.load("));
    assert!(!publish.contains("m.filter(|_| open)"));
    for field in ["depth: m.map_or(0, |m| m.depth())", "list: &list.id", "list_count: list.count", "list_index: list.index", "list_value: &list.value", "rows_omitted: true"] {
        assert!(publish.contains(field), "{field}");
    }
}

#[test]
fn lobby_practice_off_stops_the_drill_and_waits_for_the_partner() {
    let lobby = src("src/runtime/lobby/training/lobby.rs");
    let update = item(&lobby, "pub fn update(");
    assert!(!lobby.contains("uptime_ms"));
    assert!(!lobby.contains("RELEASE_SINCE"));
    assert!(!src("src/model/lobby_flag.rs").contains("RELEASE_MS"));
    assert!(update.find("lobby_flag::release(").unwrap() < update.find("lobby_flag::step(").unwrap());
    assert!(update.contains("edge_juggle::stop_for_release()"));
    let step_at = update.find("lobby_flag::step(").unwrap();
    assert!(update[step_at..].starts_with("lobby_flag::step(crate::model::lobby_flag::released_clock(clock, releasing), observed, practice_on)"));
    assert_eq!(update.matches("lobby_flag::step(").count(), 1);
    assert!(item(&src("src/model/lobby_flag.rs"), "pub const fn released_clock(").contains("clock.ui_ready || releasing"));
    let ready = item(&lobby, "fn ui_ready(");
    assert!(ready.contains("UI_READY.load(") && ready.contains("RELEASING.load("));
    assert!(item(&lobby, "pub fn active(").contains("requested() && ui_ready()"));
    assert!(item(&lobby, "pub fn try_active(").contains("try_requested()? && ui_ready()"));
    assert!(item(&lobby, "pub fn requested(").contains("practice_setting("));
    assert!(item(&lobby, "pub fn try_requested(").contains("practice_setting("));
    let edge = src("src/runtime/features/edge_juggle.rs");
    let holds = item(&edge, "pub(crate) fn holds_partner(");
    for term in ["wanted().is_some()", "ENGINE_ACTIVE.load(", "super::cpu_level::partner_held()", "practice_cpu::restoring()"] {
        assert!(holds.contains(term), "{term}");
    }
    let level = src("src/runtime/features/cpu_level.rs");
    assert_eq!(level.matches("DRILL_PARTNER_HELD.store(").count(), 1);
    let poll = item(&level, "fn poll_drill_locked(");
    let branch = &poll[poll.find("if scene.is_none() {").unwrap()..poll.find("let level =").unwrap()];
    assert!(branch.contains("PartnerLease::new()"));
    assert!(!branch.contains("set_behavior") && !branch.contains("write_unchecked"));
}

#[test]
fn a_player_point_with_no_floor_is_refused_and_the_floor_read_stays_the_card_build_s() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let frame = item(&runtime, "pub unsafe fn frame(");
    let read = frame.find("wizard.settle_player(").unwrap();
    let found = frame[read..].find("if placed && confirm {").unwrap() + read;
    let missing = frame[found..].find("} else if confirm {").unwrap() + found;
    let refusal = frame[missing..].find("notices::refuse(\"Pick a point above a floor\"); // restriction: edge-juggle-player-no-floor").unwrap() + missing;
    let notice = frame[refusal..].find("post_notice(capture, run").unwrap() + refusal;
    assert!(read < found && found < missing && missing < refusal && refusal < notice);
    assert!(!frame.contains("wizard.setup.p1 ="));

    let placement = src("src/runtime/features/placement.rs");
    let floor = item(&placement, "pub(super) unsafe fn floor_under(");
    assert!(floor.contains("let top = if platforms { (y + 1.0).min(top) } else { top };"));
    assert!(floor.contains("let floor = ray.floor(x, [top, bottom], platforms)?;"));
    assert_eq!(floor.matches("ray.floor(").count(), 1);

    let restrictions = src("docs/RESTRICTIONS.md");
    assert!(restrictions.contains("`edge-juggle-player-no-floor`"));
    assert!(restrictions.contains("\"Pick a point above a floor\""));
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
    assert!(draw.contains("marks.pointer = None;"));
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
    // The kept setup carries whether the start was moved, and Start's reuse of it restores an unmoved default.
    assert!(poll.contains("wizard.kept(run.stage)"));
    assert!(poll.contains("wizard.back_to_defaults(run.defaults)"));
    assert!(!poll.contains("wizard.setup = run.defaults"));
    assert!(item(&runtime, "pub unsafe fn frame(").contains("wizard.kept_setup(stage, kept, config.options)"));
    let model = src("src/model/edge_juggle.rs");
    assert!(model.contains("Kept { run: stage.ledges, setup: self.setup, unmoved: self.unmoved(), under: self.under }"));
    assert!(model.contains("self.remembered(stage, kept.setup, kept.unmoved)"));
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
    assert!(snap.contains("let x = stage.player_x(drill, x);"));
    assert!(snap.contains("placement::floor_under(p1, [x, y], drill == Drill::Juggler)"));
    assert!(!snap.contains("FloorRay::new("));
    assert!(!snap.contains("ray.floor("));
    assert!(!snap.contains(".clamp("));
}

#[test]
fn edge_juggle_waits_for_a_knocked_out_player() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let ko = item(&runtime, "unsafe fn player_ko(p1: usize) -> bool");
    for status in ["C::StatusDead", "C::StatusStandby", "C::StatusRebirth"] {
        assert!(ko.contains(status), "{status}");
    }
    assert!(ko.contains(".map(konst)"));
    assert!(ko.contains("natives::status_kind(p1)"));
    assert!(ko.contains("!= UNRESOLVED"));
    let frame = item(&runtime, "pub unsafe fn frame(");
    assert!(frame.contains("player_ko: player_ko(p1),"));
    let restart = frame.split_once("PlacementAction::RestartPlacement => {").unwrap().1
        .split_once("PlacementAction::ResetResources =>").unwrap().0;
    assert!(restart.contains("if held_cpu.take() == Some(body)"));
    assert!(restart.contains("state::set_sleep(cpu, false);"));
    assert!(restart.find("state::set_sleep(cpu, false);").unwrap()
        < restart.find("run.placement = Placement::ResetResources;").unwrap());
    let model = src("src/model/edge_juggle.rs");
    let action = item(&model, "pub fn action(self, frame: PlacementFrame)");
    assert!(action.contains("if frame.dead_or_rebirthing || frame.player_ko"));
    let ids = src("src/model/cpu/ids.rs");
    assert!(ids.contains("StatusStandby = \"FIGHTER_STATUS_KIND_STANDBY\","));
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
    let ground = item(&helpers, "pub(super) unsafe fn ground(");
    ordered(ground, &["state::clear_speed_all(", "state::init_pos(", "state::set_lr(",
        "state::set_situation_kind("]);
    assert_eq!(ground.matches("state::").count(), 5);
    assert!(ground.contains("konst(C::SituationGround)"));
    let stand = item(&helpers, "pub(super) unsafe fn stand(");
    ordered(stand, &["ground(body, at, facing)", "state::status_request("]);
    assert_eq!(stand.matches("state::").count(), 1);
    assert!(stand.contains("konst(C::StatusWait)"));
    let lift = item(&helpers, "pub(super) unsafe fn lift(");
    ordered(lift, &["state::clear_speed_all(", "state::init_pos(", "state::set_lr(",
        "state::set_situation_kind("]);
    assert_eq!(lift.matches("state::").count(), 5);
    assert!(lift.contains("konst(C::SituationAir)"));
    assert!(!lift.contains("state::set_pos("));
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
    assert!(item(&placement, "pub(super) unsafe fn stand(").contains("ground(body, at, facing)"));
    assert!(item(&placement, "pub(super) unsafe fn ground(").contains("state::init_pos(body"));
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
fn the_strip_asks_the_shared_wizard_state() {
    let hud = src("src/runtime/features/hud_draw.rs");
    let draw = item(&hud, "pub unsafe fn draw(");
    assert!(draw.contains("WIZARDS.hides_strip(owners)"));
    assert!(draw.contains("let menu_open = owners.covers_hud()"));
    assert!(draw.contains("let row = (!menu_open).then(|| hud_row::hud_row("));
    assert!(draw.contains("let chip = row.as_ref().map(|row| hud_row::hud_row_box("));
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
        ("RecordPlacing", "src/runtime/features/record_place.rs"),
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
        for reader in ["tech::wizard_active()", "edge_juggle::wizard_active()", "egg_zone::wizard_active()", "record_place::wizard_active()"] {
            assert_eq!(text.matches(reader).count(), 1, "{path}: {reader}");
        }
        assert_eq!(text.matches("::wizard_active()").count(), 4, "{path}: exactly the four prompt readers");
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
    let scene_exit = frame.split_once("if scene::practice_scene().is_none()").unwrap().1.split_once("if published::HOT_SWAP_IN_PROGRESS").unwrap().0;
    assert!(scene_exit.contains("WIZARDS.set_up(Kind::DrillStart, false)"), "scene exit");
    let no_body = frame.split_once("let Some((drill, body))").unwrap().1.split_once("let cpu = body.accessor").unwrap().0;
    assert!(no_body.contains("WIZARDS.set_up(Kind::DrillStart, false)"), "ended drill or missing body");
    let stage_change = frame.split_once("if engine.run.as_ref().is_some_and(|run| run.wizard.is_some() && run.stage_key != stage_key)").unwrap().1
        .split_once("let (stage, p1_start) = match setup_stage(p1, drill)").unwrap().0;
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
fn egg_zone_reads_its_stage_through_the_off_stage_rule() {
    let zone = src("src/runtime/features/egg_zone.rs");
    let read = item(&zone, "unsafe fn read_stage(");
    assert!(read.contains("RayProbe::new(p1)"));
    assert!(read.contains("Stage::read("));
    assert!(!read.contains("stage_floor("));
    assert!(!read.contains("scene::"));
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
fn recording_lists_use_the_menus_list_jump() {
    let recording = src("src/model/recording_menu.rs");
    let implementation = recording.split_once("#[cfg(test)]\nmod tests").unwrap().0;
    assert!(!implementation.contains("fn page_by(") && !implementation.contains("const PAGE:"));
    for head in ["fn page_section(", "fn press_file(", "fn press_picker("] {
        let body = member(implementation, head);
        assert!(body.contains("jumped(") || body.contains("jumped_within("), "{head}");
    }
    assert!(src("src/model/menu/mod.rs").contains("fn jumped_within("));
}

#[test]
fn the_menu_hook_holds_repeats_while_the_worker_is_behind() {
    let input = src("src/runtime/ui/input.rs");
    let poll = item(&input, "pub fn on_p1_poll(");
    let backlog = poll.find("adapter.set_backlog(worker::input_pending())").expect("backlog");
    assert!(backlog < poll.find("adapter.poll(").unwrap());
    let worker = src("src/runtime/ui/worker.rs");
    assert!(item(&worker, "pub fn input_pending()").contains("INPUT.is_empty()"));
}

#[test]
fn zone_overlay_dims_the_live_view_and_backs_the_inset_guide() {
    let zone = src("src/runtime/features/egg_zone.rs");
    let draw = item(&zone, "pub fn draw(");
    assert!(draw.contains("crop_window(crop_view(&map), inset, display.step)"), "the live view's marks are the crop window");
    assert!(!draw.contains("stroke_layers"));
    assert!(draw.contains("INSET_BACKING"), "every inset mark has a dark background");
    assert!(item(&zone, "fn prompt(").contains("notices::show_now("), "the built-in notice includes the dark label plate");
}

#[test]
fn the_bridge_reports_the_committed_zone_not_a_queued_apply() {
    let worker = src("src/runtime/ui/worker.rs");
    let publish = item(&worker, "fn publish_bridge_state(");
    assert!(publish.contains("zone_saved: zone.and_then(|z| committed_egg_zone(z.stage))"));
    let committed = item(&worker, "fn committed_egg_zone(");
    assert!(!committed.contains("EGG_ZONE_WRITE"), "a queued Apply is not saved yet");
    assert!(committed.contains("EGG_ZONE_PRESENT") && committed.contains("EGG_ZONES[stage]"));
    let game = item(&worker, "pub fn egg_zone(");
    assert!(game.contains("EGG_ZONE_WRITE") && game.contains("committed_egg_zone(stage)"), "the game still plays a queued Apply");
}

#[test]
fn cpu_think_counts_follow_the_hooks_training_or_lobby_gate() {
    let recorder = src("src/runtime/features/recorder.rs");
    assert!(recorder.contains("note_ai_think(matched.map(|body| body.entry_id), |entry| {"));
    assert!(recorder.contains("native_training && owns_controller(natives::fighter_entry(entry))"));
    assert!(!recorder.contains("if native_training {"));
}

#[test]
fn the_sample_mailbox_uses_the_models_slot_count() {
    let bridge = src("src/runtime/bridge_state.rs");
    assert!(bridge.contains("Mailbox<PracticeSample, { practice::SAMPLE_SLOTS }>"));
    assert!(!bridge.contains("Mailbox<PracticeSample, 8>"));
}

#[test]
fn practice_sample_projection_is_gated_and_stale_posts_are_dropped() {
    let bridge = src("src/runtime/bridge_state.rs");
    let capture = item(&bridge, "impl PracticeCapture {");
    let submit = capture.split("    pub fn submit(").nth(1).unwrap().split("    /// Mash out").next().unwrap();
    assert!(submit.find("if self.admission != 0 {").unwrap() < submit.find("self.post(false, &sample());").unwrap());
    let result = capture.split("    pub fn result(").nth(1).unwrap().split("    /// One copy").next().unwrap();
    assert!(result.find("if self.admission != 0 {").unwrap() < result.find("if let Some(fields) = sample() {").unwrap());
    assert!(result.contains("self.post(true, &fields);"));
    let post = capture.split("    #[inline(never)]").nth(1).unwrap();
    assert!(post.contains("fn post(self, result: bool, fields: &Fields)"));
    assert!(post.find("activation().0 == self.activation").unwrap() < post.find("PRACTICE.post(").unwrap());
}

#[test]
fn the_row_121_grade_is_held_by_the_worker_and_reposted() {
    let worker = src("src/runtime/features/mashout/worker.rs");
    let arm = &worker[worker.find("Event::TechGrade(generation, grade, kind, best) => {").unwrap()..];
    let arm = &arm[..arm.find("Event::Cancel(").unwrap()];
    assert!(arm.contains("worker.tech_grade = Some((generation, grade, best));"));
    assert!(!arm.contains(".result("));
    let tail = &worker[worker.find("worker.session.finish();").unwrap()..];
    let held = tail.find("Some((generation, grade, best)) if generation == worker.generation =>").unwrap();
    assert!(held < tail.find("owned(generation).result(").unwrap());
}

#[test]
fn a_pending_drill_start_does_not_publish_the_previous_run() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let frame = item(&runtime, "pub unsafe fn frame(");
    let branch = &frame[frame.find("if !owns_setup() {").unwrap()..];
    let branch = &branch[..branch.find("return;").unwrap()];
    let guard = branch.find("if START.load(Ordering::Acquire) == 0 {").expect("a pending Start belongs to the next run");
    assert!(guard < branch.find("report_drill_state(capture, run, None, None);").unwrap());
}

#[test]
fn btt_poll_publishes_input_owned_changes() {
    let runtime = src("src/runtime/features/btt.rs");
    let poll = item(&runtime, "pub unsafe fn poll(");
    assert!(poll.contains("PracticeCapture::of("));
    assert!(poll.contains("if !START.load(Ordering::Acquire) {"));
    assert!(poll.contains("capture.submit(|| mode.practice_fields());"));
    let pass = item(&runtime, "unsafe fn pass(");
    assert!(pass.find("BTT.lock()").unwrap() < pass.find("START.swap(false, Ordering::AcqRel)").unwrap());
}

#[test]
fn a_pending_start_never_publishes_the_previous_run() {
    let mashout = src("src/runtime/features/mashout.rs");
    let frame = item(&mashout, "pub unsafe fn frame(");
    let guard = frame.find("if !START.load(Ordering::Acquire) {").expect("Mash out waits for Start");
    assert!(guard < frame.find("capture.submit(|| game.practice_fields());").unwrap());

    let tech = src("src/runtime/features/tech.rs");
    let sample = item(&tech, "fn with_practice_frame(");
    assert!(sample.contains("if !START.load(Ordering::Acquire)"));
    assert!(sample.contains("if run.restart_pending { &Session::IDLE }"));
    let update = item(&tech, "pub unsafe fn frame(");
    let restart = &update[update.find("if run.restart_pending ||").unwrap()..];
    let fighter = restart.find("let Some(fighter_kind) = natives::fighter_kind(at) else {").unwrap();
    let clear = restart.find("run.restart_pending = false;").unwrap();
    let start = restart.find("run.controller.start(").unwrap();
    assert!(fighter < clear && clear < start);

    let eggs = src("src/runtime/features/hitbox.rs");
    let pass = item(&eggs, "unsafe fn eggs_pass(");
    assert!(pass.find("RUN.lock()").unwrap() < pass.find("EGG_START.swap(false, Ordering::AcqRel)").unwrap());

    let bridge = src("src/runtime/bridge_state.rs");
    assert!(bridge.contains("practice::adopt_sample(&mut self.fields, sample.fields);"));
    assert!(!bridge.contains("self.fields.last_outcome = last_outcome"));
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

#[test]
fn the_drill_picker_holds_the_cpu_at_the_picked_point() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let frame = item(&runtime, "pub unsafe fn frame(");
    let picker = frame.split_once("if let Some(wizard) = &mut run.wizard {").expect("the picker branch").1;
    let (picker, started) = picker
        .split_once("post_notice(capture, run, best_time(bests, key), best_time(streaks, key));\n        return;\n    }\n")
        .expect("the picker reports and returns");
    assert!(picker.contains("run.cpu_view = hold_preview(preview, held_cpu, body, wizard.setup.cpu);"));
    // A started drill wakes the CPU for the placement that follows.
    assert!(started.trim_start().starts_with("if preview.take().is_some() && held_cpu.take() == Some(body) {"));
    let hold = item(&runtime, "unsafe fn hold_preview(");
    assert!(hold.contains("placement::preview(preview, body.accessor, body.object_id, at) == Some(true)"));
    assert!(hold.contains("*held_cpu = Some(body);"));
    assert!(hold.contains("crosshair::cpu_view(preview.as_ref())"));
    let placement = src("src/runtime/features/placement.rs");
    let preview = item(&placement, "pub(super) unsafe fn preview(");
    assert!(preview.contains("crosshair::preview_step("));
    assert!(preview.find("PreviewAct::Lift").unwrap() < preview.find("lift(body, at").unwrap());
    assert!(preview.find("PreviewAct::Hold").unwrap() < preview.find("hold(body, at)").unwrap());
    assert!(preview.contains("konst(C::WorkFlagIsLoupe)") && preview.contains("natives::is_flag(body,"));
    // The chevron goes while the body is there and the game HUD can show its magnifier; the state says which.
    assert!(item(&runtime, "pub fn look(").contains("wizard.marks().shown(engine.preview.is_some() && published::hud())"));
    assert!(item(&runtime, "fn report_drill_state(").contains("fields.wizard_cpu = run.cpu_view;"));
}

#[test]
fn leaving_the_picker_puts_the_cpu_back_where_it_stood() {
    let runtime = src("src/runtime/features/edge_juggle.rs");
    let wake = item(&runtime, "unsafe fn wake_cpu(");
    assert!(wake.contains("self.preview.take()"), "a wake ends the preview");
    let ended = wake.split_once("self.preview.take()").unwrap().1;
    // A loading save state places the fighters itself.
    assert!(ended.contains("!states::is_loading()"));
    assert!(ended.contains("placement::restore(current.accessor, preview.origin());"));
    let placement = src("src/runtime/features/placement.rs");
    let restore = item(&placement, "pub(super) unsafe fn restore(");
    assert!(restore.find("origin.grounded").unwrap() < restore.find("stand(body, origin.at, origin.facing)").unwrap());
    assert!(restore.contains("lift(body, origin.at, origin.facing)") && restore.contains("fall(body)"));
    // The end of the scene drops the preview with the bodies.
    let frame = item(&runtime, "pub unsafe fn frame(");
    let scene_end = frame.split_once("if scene::practice_scene().is_none() {").unwrap().1.split_once("return;").unwrap().0;
    assert!(scene_end.contains("engine.preview = None;"));
}

#[test]
fn the_original_cpu_comes_back_after_the_picker_put_the_body_back() {
    // The roster gives the incoming fighter the outgoing body's place. A picker that holds the drill's own fighter at
    // its picked start puts it back on Player 1's next frame; the original is not asked for before that.
    let service = src("src/runtime/features/edge_juggle/practice_cpu.rs");
    let serve = item(&service, "pub(in crate::runtime) fn service_practice_cpu_selection(");
    assert!(serve.contains("|| picker::observable(selected, super::leaving_picker, || {"));
    assert!(serve.contains("(!roster::busy() && roster::cpu_reload_ready()).then(roster::cpu_kind).flatten()"));
    let runtime = src("src/runtime/features/edge_juggle.rs");
    // A busy engine reads as held: the service asks again on its next tick. B's menu request counts too: the menu
    // opens on Player 1's next poll, and a request the roster finds waiting under an open menu is dropped with a notice.
    let leaving = item(&runtime, "fn leaving_picker(");
    assert!(leaving.contains("BACK.load(Ordering::Acquire) || ENGINE.try_lock().is_none_or(|engine| engine.preview.is_some())"));
    assert!(item(&runtime, "pub fn take_menu_request(").contains("BACK.swap(false, Ordering::AcqRel)"));
    // Under the reopened menu the roster is not ready, so the original comes back when the menu closes.
    let roster = src("src/runtime/roster/character_roster.rs");
    assert!(item(&roster, "fn cpu_reload_ready_with(").contains("|| menu::captures_input()"));
    // The engine lets go of the body before it ends the run, so a hold never outlives the frames that take it.
    let frame = item(&runtime, "pub unsafe fn frame(");
    let ended = frame.split_once("let Some((drill, body)) = drill.zip(body) else {").unwrap().1;
    assert!(ended.find("engine.wake_cpu(body);").unwrap() < ended.find("ENGINE_ACTIVE.store(false, Ordering::Release);").unwrap());
    assert!(item(&runtime, "unsafe fn wake_cpu(").contains("self.preview.take()"));
}