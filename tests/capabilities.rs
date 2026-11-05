//! Review R2-1: native capabilities. The declared closures of `model::capabilities` are checked against the
//! runtime source's call graph (`tests/common/callgraph.rs`), every rejected row is shown to disable every
//! entry point that can reach it, and nothing an installed entry point reaches is a rejected native.
//!
//! The runtime cannot run on the host, so "spy the native-call layer" is done on the call graph the same way:
//! for each poisoned row, the natives an installed entry point may call are computed from the code and every
//! one must be allowed by the `Granted` set the boot would hand the native-call layer.

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

#[path = "common/callgraph.rs"]
mod callgraph;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use callgraph::{Func, Graph, Tok};
use model::capabilities::{self, Granted};
use model::sites::Feature;

/// How a boundary is guarded in the code: a runtime readiness flag that the owning unit's install sets, read by
/// the boundary's first condition (`every_boundary_checks_its_gate_first`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Gate {
    Flag(&'static str),
}

/// A function the call graph stops at. `owner`: the unit whose closure contains what the boundary reaches
/// (the unit that sets the flag).
struct Boundary {
    function: &'static str,
    owner: Option<Feature>,
    gate: Gate,
}

const BOUNDARIES: &[Boundary] = &[
    // The save-state step of the per-fighter frame wrapper: a no-op until boot activates package 3.
    Boundary { function: "states::frame", owner: Some(Feature::StateCore), gate: Gate::Flag("ACTIVE") },
    // The frame wrapper's per-feature facts and appliers: each checks its feature's install flag first.
    Boundary { function: "cpu::frame_advantage", owner: Some(Feature::CpuMash), gate: Gate::Flag("MASH_ON") },
    Boundary { function: "cpu::mash_facts", owner: Some(Feature::CpuMash), gate: Gate::Flag("MASH_ON") },
    Boundary { function: "cpu::mash_player_position", owner: Some(Feature::CpuMash), gate: Gate::Flag("MASH_ON") },
    Boundary { function: "cpu::apply_mash", owner: Some(Feature::CpuMash), gate: Gate::Flag("MASH_ON") },
    Boundary { function: "cpu::ledge_facts", owner: Some(Feature::CpuLedge), gate: Gate::Flag("LEDGE_ON") },
    Boundary { function: "cpu::tech_facts", owner: Some(Feature::CpuTech), gate: Gate::Flag("TECH_ON") },
    Boundary { function: "cpu::apply_tech", owner: Some(Feature::CpuTech), gate: Gate::Flag("TECH_ON") },
    Boundary { function: "cpu::apply_defence", owner: Some(Feature::CpuDefence), gate: Gate::Flag("DEFENCE_ON") },
    // The run-time texture helper (u4): every entry of it checks `usable()` (its rows passed and were granted) first.
    Boundary { function: "texture::load", owner: Some(Feature::UiTexture), gate: Gate::Flag("usable") },
    Boundary { function: "texture::load_resident", owner: Some(Feature::UiTexture), gate: Gate::Flag("usable") },
    Boundary { function: "texture::poll", owner: Some(Feature::UiTexture), gate: Gate::Flag("usable") },
    Boundary { function: "texture::bind", owner: Some(Feature::UiTexture), gate: Gate::Flag("usable") },
    Boundary { function: "texture::unbind", owner: Some(Feature::UiTexture), gate: Gate::Flag("usable") },
    Boundary { function: "texture::release", owner: Some(Feature::UiTexture), gate: Gate::Flag("usable") },
    Boundary { function: "texture::tick", owner: Some(Feature::UiTexture), gate: Gate::Flag("usable") },
    // Package 6's CPU-as-P1 call-ins from the CPU core's frame wrapper and the input hooks: nothing runs until its install says so.
    Boundary { function: "p1::frame", owner: Some(Feature::P1Cpu), gate: Gate::Flag("installed") },
    Boundary { function: "p1::input", owner: Some(Feature::P1Cpu), gate: Gate::Flag("installed") },
    // Package 4, S4-4: the named-macro layer's per-frame and input call-ins from the CPU core's frame wrapper and the input stage: nothing runs until
    // its install (`macros::install`, only when the unit's whole closure passed) has set READY.
    Boundary { function: "macros::frame_before", owner: Some(Feature::NamedMacros), gate: Gate::Flag("ready") },
    Boundary { function: "macros::frame_after", owner: Some(Feature::NamedMacros), gate: Gate::Flag("ready") },
    Boundary { function: "macros::on_p1_input", owner: Some(Feature::NamedMacros), gate: Gate::Flag("ready") },
    // ui-screens: the layer's after-draw call-in from the Training layout tick (the clip editor's held scrub).
    Boundary { function: "macros::after_layout_draw", owner: Some(Feature::NamedMacros), gate: Gate::Flag("ready") },
    // SPEC-core-06: the hitbox visualizer's per-fighter pass, called from the CPU core's frame wrapper: nothing until boot installs it.
    Boundary { function: "hitbox::frame", owner: Some(Feature::Hitbox), gate: Gate::Flag("INSTALLED") },
    // The own orbs' layout-tick draw (decision HB-OWN-ORBS).
    Boundary { function: "hitbox::draw_orbs", owner: Some(Feature::Hitbox), gate: Gate::Flag("INSTALLED") },
    // The input display's reads in the input-delay hook: dormant until boot enables the display.
    Boundary { function: "input_display::on_p1_poll", owner: Some(Feature::InputDisplay), gate: Gate::Flag("ENABLED") },
    // SPEC-input-06: the frame strip's frame, from the CPU core's player-entry frame: dormant until boot enables the display.
    Boundary { function: "input_display::sample_frame", owner: Some(Feature::InputDisplay), gate: Gate::Flag("ENABLED") },
    Boundary { function: "input_display::sampling", owner: Some(Feature::InputDisplay), gate: Gate::Flag("ENABLED") },
    // Its hitstun-left read: the CPU tech unit's work_get_float row (boot sets the flag from that unit's closure).
    Boundary { function: "input_display::reaction_left", owner: Some(Feature::CpuTech), gate: Gate::Flag("REACTION_ON") },
    // The recorder's entry points the other features call: dormant until boot enables the recorder.
    Boundary { function: "recorder::request_playback", owner: Some(Feature::Recording), gate: Gate::Flag("ENABLED") },
    Boundary { function: "recorder::on_state_restored", owner: Some(Feature::Recording), gate: Gate::Flag("ENABLED") },
    Boundary { function: "recorder::on_p1_poll", owner: Some(Feature::Recording), gate: Gate::Flag("ENABLED") },
];

/// Every entry point that is not a `#[skyline::hook]` function, with its install unit.
const EXTRA_ENTRIES: &[(&str, Feature)] = &[
    ("cpu::on_frame", Feature::CpuCore),
    ("reset_hub::on_init", Feature::ResetHub),
    ("cpu_common::di_hook", Feature::CpuCommon),
    ("cpu_common::change_status_hook", Feature::CpuCommon),
    ("cpu_common::sub_guard_cont_hook", Feature::CpuCommon),
];

/// The install unit of every `#[skyline::hook]` function, by module (a module whose hooks belong to several
/// units is listed by function).
fn unit_of_hook(f: &Func) -> Option<Feature> {
    let by_function = match f.key().as_str() {
        "cpu::cpu_sdi_hook" | "cpu::cpu_stick_x_hook" | "cpu::cpu_stick_y_hook" | "cpu::cpu_clatter_start_hook" => Some(Feature::CpuDefence),
        "cpu::cpu_transition_term_hook" => Some(Feature::CpuLedge),
        "cpu::cpu_motion_change_hook" | "cpu::cpu_quake_pos_hook" => Some(Feature::CpuTech),
        "cpu::cpu_check_button_on_hook" | "cpu::cpu_check_button_off_hook" | "cpu::cpu_stick_x_no_clamp_hook" | "cpu::cpu_stick_y_no_clamp_hook" | "cpu::cpu_shield_param_hook" => Some(Feature::CpuShield),
        "cpu::cpu_command_flag_hook" | "cpu::cpu_attack_air_kind_hook" | "cpu::cpu_stick_dir_hook" => Some(Feature::CpuMash),
        "cpu::cpu_common_params_hook" => Some(Feature::CpuShieldDamage),
        "cpu::cpu_camera_mode_hook" | "cpu::cpu_camera_setup_hook" => Some(Feature::CpuCamera),
        _ => None,
    };
    if by_function.is_some() {
        return by_function;
    }
    match f.module.as_str() {
        "input_delay" => Some(Feature::InputDelay),
        "hitbox" => Some(Feature::Hitbox),
        "lra_reset" => Some(Feature::LraReset),
        "stage_hazards" => Some(Feature::StageHazards),
        "recorder" => Some(Feature::Recording),
        "notices" => Some(Feature::Notices),
        "developer" => Some(Feature::DeveloperDebug),
        "states" | "state_restore" => Some(Feature::StateCore),
        "anchors" | "archive" => Some(Feature::Ui),
        "cpu_pilot" => Some(Feature::P1Cpu),
        // Package 4, S4-4: the live replay capture adapter's hooks (the named-macro layer's unit).
        "live_replay" => Some(Feature::NamedMacros),
        _ => None,
    }
}

struct Analysis {
    graph: Graph,
    consts: BTreeMap<String, usize>,
}

impl Analysis {
    fn new() -> Analysis {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        // Package 6's lobby, Mii and roster code and its seam installer have their own guard system (the seam rows of
        // `model::seams`, `seam_install::gate`) and are not part of this model; what they call through `game::natives` and
        // `state_natives` is still behind the grant check.
        let mut graph = Graph::scan(&root.join("src/runtime"), &["lobby", "mii", "roster", "seams"]);
        // Function-pointer tables the text cannot show: the layout tick runs the registered draw callbacks.
        graph.dynamic.push(("hud::run".to_owned(), "hud_draw::draw".to_owned()));
        graph.dynamic.push(("hud::run".to_owned(), "hitbox::draw_orbs".to_owned()));
        // SPEC-core-06: the held-sphere state machine (`model::hitbox::HandleSet`, not scanned) calls back into the runtime's effect module.
        for (from, to) in [("hitbox::frame", "hitbox::sphere_alive"), ("hitbox::frame", "hitbox::sphere_kill_own"), ("hitbox::frame", "hitbox::sphere_spawn"), ("hitbox::hitbox_rebound_hook", "hitbox::sphere_alive"), ("hitbox::hitbox_rebound_hook", "hitbox::sphere_kill_own")] {
            graph.dynamic.push((from.to_owned(), to.to_owned()));
        }
        // The shared production planner calls the runtime's AttackReader too.
        for to in ["hitbox::active", "hitbox::data", "hitbox::read_u64", "hitbox::read_f32"] {
            graph.dynamic.push(("hitbox::frame".to_owned(), to.to_owned()));
        }
        for (from, to) in &graph.dynamic {
            assert_eq!(graph.find(from).len(), 1, "dynamic edge source {from}");
            assert_eq!(graph.find(to).len(), 1, "dynamic edge target {to}");
        }
        let consts = callgraph::model_offsets(&root.join("src/model"));
        Analysis { graph, consts }
    }

    /// `(function index, unit)` of every entry point.
    fn entries(&self) -> Vec<(usize, Feature)> {
        let mut out = Vec::new();
        for (i, f) in self.graph.funcs.iter().enumerate() {
            if f.attrs.contains("skyline::hook") {
                let unit = unit_of_hook(f).unwrap_or_else(|| panic!("hook {} has no declared unit", f.key()));
                out.push((i, unit));
            }
        }
        for (key, unit) in EXTRA_ENTRIES {
            let found = self.graph.find(key);
            assert_eq!(found.len(), 1, "entry point {key} must be one function");
            out.push((found[0], *unit));
        }
        out
    }

    fn boundary_keys(&self) -> BTreeSet<String> {
        BOUNDARIES.iter().map(|b| b.function.to_owned()).collect()
    }

    /// The offsets a function names itself (calls it can make or words it reads).
    fn own_offsets(&self, f: &Func) -> BTreeSet<usize> {
        Graph::offset_consts(f).iter().filter_map(|n| self.consts.get(n).copied()).collect()
    }

    /// The game offsets reachable from `start`, not descending into a boundary (boundaries met are returned).
    fn reach(&self, start: usize) -> (BTreeSet<usize>, BTreeSet<String>) {
        let stop = self.boundary_keys();
        let mut seen = BTreeSet::new();
        let mut work = vec![start];
        let mut offsets = BTreeSet::new();
        let mut hit = BTreeSet::new();
        while let Some(i) = work.pop() {
            if !seen.insert(i) {
                continue;
            }
            offsets.extend(self.own_offsets(&self.graph.funcs[i]));
            for callee in self.graph.callees(i) {
                let key = self.graph.funcs[callee].key();
                if stop.contains(&key) {
                    hit.insert(key);
                } else {
                    work.push(callee);
                }
            }
        }
        (offsets, hit)
    }

    /// The offsets of an entry's own hook row.
    fn hook_offset(&self, f: &Func) -> Option<usize> {
        f.hook_offset_const().and_then(|n| self.consts.get(&n).copied())
    }

    /// Whether every row a call of `offset` relies on belongs to `unit` (its own, Training's, or the core's for
    /// the CPU features), so it is not a cross-feature need.
    fn owned_by(&self, offset: usize, unit: Feature) -> bool {
        let core = matches!(unit, Feature::CpuDefence | Feature::CpuLedge | Feature::CpuTech | Feature::CpuShield | Feature::CpuMash | Feature::CpuCommon | Feature::CpuShieldDamage | Feature::CpuCamera);
        // A call holds when every row naming the function passed: it is the unit's own only when they all are.
        capabilities::call_rows(offset).all(|rep| {
            capabilities::equivalent_rows(rep).any(|row| {
                capabilities::unit_of_row(row) == unit || row.feature == Feature::Training || (core && row.feature == Feature::CpuCore)
            })
        })
    }

    /// The computed cross-feature closure of every unit: the call names of what its entry points reach (and what
    /// its boundaries reach) that it does not own.
    fn cross_needs(&self) -> BTreeMap<&'static str, BTreeSet<&'static str>> {
        let mut out: BTreeMap<&'static str, BTreeSet<&'static str>> = BTreeMap::new();
        let mut add = |unit: Feature, offsets: &BTreeSet<usize>| {
            for &offset in offsets {
                if capabilities::call_name(offset).is_some() && !self.owned_by(offset, unit) {
                    out.entry(unit.name()).or_default().insert(capabilities::call_name(offset).unwrap());
                }
            }
        };
        let mut boundaries_hit: BTreeSet<String> = BTreeSet::new();
        for (i, unit) in self.entries() {
            let (offsets, hit) = self.reach(i);
            add(unit, &offsets);
            boundaries_hit.extend(hit);
        }
        for b in BOUNDARIES {
            let Some(owner) = b.owner else { continue };
            for start in self.graph.find(b.function) {
                // A boundary's own body and what it reaches without crossing another boundary: the others are processed
                // on their own, with their own owner.
                let (offsets, _) = self.reach(start);
                add(owner, &offsets);
            }
        }        out
    }
}

fn declared() -> BTreeMap<&'static str, BTreeSet<&'static str>> {
    capabilities::CROSS_NEEDS.iter().map(|(unit, names)| (unit.name(), names.iter().copied().collect())).collect()
}

/// Prints `CROSS_NEEDS` as Rust source: `cargo test --test capabilities print_cross_needs -- --ignored --nocapture`.
#[test]
#[ignore]
fn print_cross_needs() {
    let computed = Analysis::new().cross_needs();
    println!("pub static CROSS_NEEDS: [(Feature, &[&str]); {}] = [", computed.len());
    for (unit, names) in &computed {
        println!("    (Feature::{}, &[{}]),", variant(unit), names.iter().map(|n| format!("\"{n}\"")).collect::<Vec<_>>().join(", "));
    }
    println!("];");
}

/// `cpu_core` -> `CpuCore`.
fn variant(name: &str) -> String {
    name.split('_').map(|w| w[..1].to_uppercase() + &w[1..]).collect()
}
#[test]
#[ignore]
fn dump_reach() {
    let a = Analysis::new();
    for (i, unit) in a.entries() {
        let f = &a.graph.funcs[i];
        let (offsets, hit) = a.reach(i);
        let rows: Vec<String> = offsets
            .iter()
            .filter(|&&o| capabilities::call_name(o).is_some())
            .map(|&o| format!("{}{}", capabilities::call_name(o).unwrap(), if a.owned_by(o, unit) { "" } else { "*" }))
            .collect();
        println!("{} [{}] hook={:?} boundaries={:?}\n    {}", f.key(), unit.name(), a.hook_offset(f).and_then(capabilities::call_name), hit, rows.join(" "));
    }
}

fn witness(a: &Analysis, start: usize, offset: usize) -> Vec<String> {
    let stop = a.boundary_keys();
    let mut parent: BTreeMap<usize, usize> = BTreeMap::new();
    let mut queue = std::collections::VecDeque::from([start]);
    let mut seen = BTreeSet::from([start]);
    while let Some(i) = queue.pop_front() {
        if a.own_offsets(&a.graph.funcs[i]).contains(&offset) {
            let mut chain = vec![a.graph.funcs[i].key()];
            let mut at = i;
            while let Some(&p) = parent.get(&at) {
                chain.push(a.graph.funcs[p].key());
                at = p;
            }
            chain.reverse();
            return chain;
        }
        for callee in a.graph.callees(i) {
            if !stop.contains(&a.graph.funcs[callee].key()) && seen.insert(callee) {
                parent.insert(callee, i);
                queue.push_back(callee);
            }
        }
    }
    Vec::new()
}

#[test]
#[ignore]
fn explain() {
    let (Ok(entry), Ok(row)) = (std::env::var("ENTRY"), std::env::var("ROW")) else { return };
    let a = Analysis::new();
    let offset = capabilities::find(&row).expect("row").offset;
    for start in a.graph.find(&entry) {
        println!("{}", witness(&a, start, offset).join(" -> "));
    }
}

// ---- the checks -----------------------------------------------------------------------------------------

#[test]
fn every_entry_point_has_a_declared_unit_and_exists() {
    let a = Analysis::new();
    let entries = a.entries();
    assert!(entries.len() >= 65, "{}", entries.len());
    assert!(entries.iter().any(|(i, _)| a.graph.funcs[*i].name == "peach_prob_8"), "the macro-generated probability hooks are entry points too");
    // Every boundary names one function that exists.
    for b in BOUNDARIES {
        assert_eq!(a.graph.find(b.function).len(), 1, "boundary {}", b.function);
    }
}

#[test]
fn cross_needs_match_the_call_graph() {
    let computed = Analysis::new().cross_needs();
    let declared = declared();
    assert_eq!(declared, computed, "regenerate with `print_cross_needs` and review the difference");
}

/// Every game offset the runtime names through a constant is a row (so a native cannot be called without a
/// guard), except the data slots listed here, which are read, never called.
#[test]
fn every_native_the_runtime_names_is_a_guarded_row() {
    let a = Analysis::new();
    // Data slots and field offsets: read, never called.
    let data_slots: BTreeSet<&str> = [
        "FIGHTER_MANAGER_SLOT_OFFSET",
        "ITEM_MANAGER_SLOT_OFFSET",
        "UI_MANAGER_SLOT_OFFSET",
        "FLAG_OFFSET",
        "FIGHTER_ENTRY_ID_FIELD",
        "WORK_CURSOR_OFFSET",
        "LOG_HEAD_OFFSET",
        "TOOL_DURABILITY_OFFSET",
        "TOOL_MATERIAL_OFFSET",
        "UI_TEX_SERVICE_SLOT_WORDS_OFFSET",
    ]
    .into();
    let mut unguarded = BTreeSet::new();
    for f in &a.graph.funcs {
        if f.attrs.contains("skyline::hook") {
            continue;
        }
        for name in Graph::offset_consts(f) {
            if data_slots.contains(name.as_str()) {
                continue;
            }
            match a.consts.get(&name) {
                Some(offset) if capabilities::call_name(*offset).is_some() => {}
                _ => {
                    unguarded.insert(format!("{} names {name}", f.key()));
                }
            }
        }
    }
    assert!(unguarded.is_empty(), "offset constants with no row: {unguarded:#?}");
}

/// Poison each row alone (as `Foreign`). Every entry point whose code can call the row's function is in a unit
/// that is then blocked; a unit whose entry points reach a boundary owned by a blocked unit never runs that
/// boundary (its flag stays off); and no entry point of an installed unit can reach a native the grant set
/// refuses. The last two are the spy: the natives each installed entry point may call, computed from the
/// code, against what the native-call layer would allow.
#[test]
fn a_poisoned_row_disables_every_entry_point_that_can_reach_it_and_no_installed_one_calls_it() {
    let a = Analysis::new();
    let entries = a.entries();
    let mut checked = 0;
    for poisoned in capabilities::all_sites() {
        // Rows with the same bytes and kind of guard on one function pass and fail together (two packages' tables
        // naming one function), so a poisoned row poisons them all.
        let poisoned_names: Vec<&str> = capabilities::equivalent_rows(poisoned).map(|r| r.name).collect();
        let usable = |site: &model::sites::Site| !poisoned_names.contains(&site.name);
        let granted = Granted::from_usable(&usable);
        let unit_blocked = |unit: Feature| capabilities::blocked_by(unit, &usable).is_some();
        for &(i, unit) in &entries {
            let f = &a.graph.funcs[i];
            let (offsets, _) = a.reach(i);
            let reaches = |offsets: &BTreeSet<usize>| offsets.iter().any(|&o| capabilities::call_rows(o).any(|r| poisoned_names.contains(&r.name)));
            if reaches(&offsets) {
                assert!(unit_blocked(unit), "{} ({}) can call {} but its unit stays installed", f.key(), unit.name(), poisoned.name);
            }
            // The entry's own hook row is the unit's own row.
            if a.hook_offset(f).is_some_and(|o| capabilities::all_sites().any(|r| r.offset == o && poisoned_names.contains(&r.name) && capabilities::unit_of_row(r) == unit)) {
                assert!(unit_blocked(unit), "{}: its own row {} is rejected", f.key(), poisoned.name);
            }
            if !unit_blocked(unit) {
                // Installed: every native its hard reach can call is granted.
                for &o in &offsets {
                    if capabilities::call_name(o).is_some() {
                        assert!(granted.allows(o), "{} is installed but can call the refused native at {o:#x} (poisoned {})", f.key(), poisoned.name);
                    }
                }
            }
            checked += 1;
        }
    }
    // A boundary runs only when its owner is installed: whatever it can call (without crossing another boundary) blocks the owner.
    for poisoned in capabilities::all_sites() {
        let poisoned_names: Vec<&str> = capabilities::equivalent_rows(poisoned).map(|r| r.name).collect();
        let usable = |site: &model::sites::Site| !poisoned_names.contains(&site.name);
        for b in BOUNDARIES {
            let Some(owner) = b.owner else { continue };
            for start in a.graph.find(b.function) {
                let (offsets, _) = a.reach(start);
                if offsets.iter().any(|&o| capabilities::call_rows(o).any(|r| poisoned_names.contains(&r.name))) {
                    assert!(capabilities::blocked_by(owner, &usable).is_some(), "boundary {} reaches {} but its owner {} stays installed", b.function, poisoned.name, owner.name());
                }
            }
        }
    }
    assert!(checked > 5000, "{checked}");
}
/// The two named counterexamples of the review, as the model answers them.
#[test]
fn the_reviews_counterexamples_are_disabled() {
    let usable_without = |name: &'static str| move |s: &model::sites::Site| s.name != name;
    // Rejecting `motion_kind` (a CpuCommon row) also disables CpuMash, whose command hook calls it.
    let u = usable_without("motion_kind");
    assert!(capabilities::blocked_by(Feature::CpuCommon, &u).is_some());
    assert_eq!(capabilities::blocked_by(Feature::CpuMash, &u), Some("motion_kind"));
    // Rejecting the ledge row disables every unit whose entry points call `is_enable_transition_term`.
    let u = usable_without("transition_term_hook");
    assert!(capabilities::blocked_by(Feature::CpuLedge, &u).is_some());
    assert_eq!(capabilities::blocked_by(Feature::CpuMash, &u), Some("transition_term_hook"));
    assert_eq!(capabilities::blocked_by(Feature::Recording, &u), Some("transition_term_hook"));
    assert_eq!(capabilities::blocked_by(Feature::CpuCommon, &u), Some("transition_term_hook"));
}

/// Every game call goes through the layer's grant: the functions that form a native address (`at`) check it,
/// and nothing else in the runtime transmutes `text_base() + offset` into a function.
#[test]
fn the_native_call_layer_checks_the_grant() {
    let a = Analysis::new();
    let mentions = |f: &Func, name: &str| f.body.iter().any(|t| matches!(t, Tok::Ident(s) if s == name));
    let ats: Vec<&Func> = a.graph.funcs.iter().filter(|f| f.key() == "natives::at" || f.key() == "state_natives::at").collect();
    assert_eq!(ats.len(), 2, "game::natives::at and state_natives::at");
    for f in ats {
        assert!(mentions(f, "granted"), "{} must check the grant", f.key());
    }
    // The run-time texture helper (u4) forms its addresses with its own `at` and calls through them in these functions: each checks
    // `usable()`, which asks the boot's grant for every row it calls.
    for f in a.graph.funcs.iter().filter(|f| f.module == "texture" && mentions(f, "transmute")) {
        assert!(mentions(f, "usable"), "{} calls a native without checking usable()", f.key());
    }
    for f in &a.graph.funcs {
        if mentions(f, "text_base") && mentions(f, "transmute") {
            assert!(f.name == "at" || mentions(f, "granted") || f.name == "lua_const", "{} forms a native address without the grant", f.key());
        }
    }
}

#[test]
#[ignore]
fn dup_rows() {
    let rows: Vec<_> = capabilities::all_sites().collect();
    for (i, a) in rows.iter().enumerate() {
        for b in &rows[i + 1..] {
            if a.offset == b.offset {
                println!("{:#x} {} ({:?}, {}) / {} ({:?}, {}) same_guard={}", a.offset, a.name, a.kind, a.feature.name(), b.name, b.kind, b.feature.name(), a.guard == b.guard);
            }
        }
    }
}

/// A boundary is only a boundary if the code stops at it: a flag-gated one reads its flag before anything else
/// (the flag is the first statement's condition), a layer-gated one is listed as such.
#[test]
fn every_boundary_checks_its_gate_first() {
    let a = Analysis::new();
    for b in BOUNDARIES {
        let f = &a.graph.funcs[a.graph.find(b.function)[0]];
        match b.gate {
            Gate::Flag(token) => {
                // The flag is read in the first `if` of the body (`if !FLAG.load(..) { return .. }`), before any
                // other call.
                let first_if = f.body.iter().position(|t| matches!(t, Tok::Ident(s) if s == "if")).expect("a gate");
                let first_call = f.body.iter().position(|t| matches!(t, Tok::Punct('('))).unwrap_or(usize::MAX);
                let flag_at = f.body.iter().position(|t| matches!(t, Tok::Ident(s) if s == token)).unwrap_or_else(|| panic!("{} never reads {token}", b.function));
                assert!(first_if < flag_at && flag_at < first_call, "{}: {token} must be read by the first condition, before any call", b.function);
            }
        }
    }
}

/// Row names are unique across all four tables (boot looks a row up by name), and two rows on one function with
/// the same kind of guard carry the same bytes (the equivalence the closures rely on).
#[test]
fn row_names_are_unique_and_rows_on_one_function_agree() {
    let rows: Vec<_> = capabilities::all_sites().collect();
    let names: BTreeSet<&str> = rows.iter().map(|r| r.name).collect();
    assert_eq!(names.len(), rows.len(), "a row name is used in two tables");
    for (i, a) in rows.iter().enumerate() {
        for b in &rows[i + 1..] {
            let writes = |s: &model::sites::Site| matches!(s.kind, model::guard::SeamKind::Hook { .. });
            if a.offset == b.offset && writes(a) == writes(b) {
                assert_eq!(a.guard, b.guard, "{} and {} name one function with different bytes", a.name, b.name);
            }
        }
    }
}

/// The install line names the rejected row of a feature that stayed out because of another feature's row.
#[test]
fn the_install_line_names_the_row_that_blocked_a_feature() {
    use model::install_line::FeatureReport;
    let usable = |s: &model::sites::Site| s.name != "motion_kind";
    let site = capabilities::blocked_by(Feature::CpuMash, &usable);
    assert_eq!(site, Some("motion_kind"));
    let report = FeatureReport { feature: Feature::CpuMash.name(), installed: false, reason: None, site };
    let line = model::install_line::InstallLine {
        tick: 1,
        version: "0.1.0",
        display: b"13.0.5",
        build_id: None,
        verdict: model::version::Verdict::Supported,
        sites: &[],
        features: &[report],
        seams: None,
        developer: None,
        settings: None,
        migration: None,
        heap_allocations: Some(0),
    }
    .to_string();
    assert!(line.contains(" cpu_mash=not_installed reason=site:motion_kind "), "{line}");
}

/// Input Delay installs on its own rows: the input display's reads in its hook are behind the display's flag
/// (lead ruling 2026-11-02), so rejecting `status_kind` or `fighter_manager_get_entry` turns the display off and
/// leaves Input Delay installed.
#[test]
fn the_input_display_degrades_apart_from_input_delay() {
    for row in ["status_kind", "fighter_manager_get_entry"] {
        let usable = |s: &model::sites::Site| s.name != row;
        assert_eq!(capabilities::blocked_by(Feature::InputDelay, &usable), None, "{row} must not block Input Delay");
        assert_eq!(capabilities::blocked_by(Feature::InputDisplay, &usable), Some(row));
    }
    // The CPU core's role test is the core's own: its two reset-hub rows stay in its closure.
    let usable = |s: &model::sites::Site| s.name != "is_operation_cpu";
    assert_eq!(capabilities::blocked_by(Feature::CpuCore, &usable), Some("is_operation_cpu"));
}
