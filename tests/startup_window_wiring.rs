//! P-HOT B (`artifacts/phase23/LANE-HOT-SLOTS-REPORT.md` §12; operator 2026-11-06: "When a match or training starts
//! the characters are just ready from the get go" → "Yes pursue B"): the window built into Training's own start, the
//! way the game builds Pokémon Trainer's three bodies (research boot tr-p23-hot-measure-3). The value rules are
//! host-tested in `src/model/swap/entry_window.rs`; this pins the adapter's four steps and their order.

fn body<'a>(src: &'a str, start: &str) -> &'a str {
    let at = src.find(start).unwrap_or_else(|| panic!("{start}"));
    &src[at..at + src[at..].find("\n}\n").expect("end of the function")]
}

const BANK: &str = include_str!("../src/runtime/roster/native_body_bank.rs");

/// H1: the scene configure's header site, after the header stores and before the request; it writes only under the
/// model's guards, after publishing the pending bank, on its own seam row and scope contract.
#[test]
fn h1_extends_the_scene_header_before_the_request_only_under_its_guards() {
    assert!(BANK.contains("#[skyline::hook(offset = 0x66de14, inline)]\nunsafe fn scene_header("));
    let h1 = body(BANK, "unsafe fn scene_header(");
    let guard = h1.find("let Some(original) = extendable(facts) else { return; };").expect("the guard");
    let scope = h1.find("scope_gate::admitted(\"native_body_bank::scene_header\")").expect("the scope gate");
    let publish = h1.find("if !publish_pending(facts.owner, members, scene) { return; }").expect("the publication");
    let write = h1.find("fields::write_unchecked(r as usize, 0x5c,").expect("the header write");
    assert!(scope < guard && guard < publish && publish < write, "{h1}");
    assert!(h1.contains("object: ctx.registers[13].w()"), "the object kind is read from w13 before native stores it");
    assert!(BANK.contains("fields::read_unchecked::<u32>(base, SCENE_HEADER) == 0xb900828d"));
    assert!(BANK.contains("if gate(SEAM_GROUP, SCENE_HEADER_SEAMS, scene_stock) { skyline::install_hooks!(scene_header); }"));
    let rows = include_str!("../src/model/seams/native_seams/rows_a1.rs");
    assert!(rows.contains("label: \"body_bank.scene_header\", group: \"native_body_bank\", offset: 0x66de14"));
    let scope_rs = include_str!("../src/model/swap/swap_model/scope.rs");
    assert!(scope_rs.contains("name: \"native_body_bank::scene_header\","));
}

/// H2 needs no new hook: the served `0x17e5bc0` finds a bank by owner (no entry), so it requests the pending
/// window's other members. H3 rewrites the creation only for native's single form; H4 binds before the table hooks.
#[test]
fn h2_to_h4_request_rewrite_and_bind_the_pending_window() {
    let resources = body(BANK, "unsafe fn prepare_resources(");
    assert!(resources.contains("bank.owner == owner && bank.phase != Phase::Retired"), "found by owner:\n{resources}");
    let h3 = body(BANK, "pub(crate) unsafe fn creation(");
    let check = h3.find("if !creation_matches(native, window) {").expect("the shape check");
    let rewrite = h3.find("fields::write_unchecked(descriptor as usize, 0x14, window.count);").expect("the rewrite");
    assert!(check < rewrite, "{h3}");
    assert!(h3.contains("bank.entry == 0 && bank.owner == owner && bank.phase == Phase::Building"));
    let lifecycle = include_str!("../src/runtime/roster/character_lifecycle.rs");
    assert!(lifecycle.contains("character_roster::body_bank::creation(ctx.registers[1].x() as u32, ctx.registers[2].x() as *mut u8);"));
    let models = body(BANK, "unsafe fn prepare_models(");
    let original = models.find("original!()(entry);").expect("native first");
    let bind = models.find("bind_pending(entry, role);").expect("the binding");
    let flags = models.find("descriptor(entry, bank)").expect("today's match");
    assert!(original < bind && bind < flags, "{models}");
}

/// The order against the scene change: a window the new scene's configure published is not departed, and its ledger
/// and copy holdings are not reset, by that scene's own `scene_changed`.
#[test]
fn the_scene_change_keeps_the_new_scenes_start_up_window() {
    let changed = body(BANK, "pub(super) fn scene_changed(");
    assert!(changed.contains("fresh[role] = bank.scene.is_some() && bank.scene == now && bank.phase != Phase::Retired;"));
    assert!(changed.contains("if !fresh[role] { bank.departed = true; }"));
    assert!(changed.contains("for role in (0..2).filter(|role| !fresh[*role]) { r_handle::reset("));
    let publish = body(BANK, "unsafe fn publish_pending(");
    assert!(publish.contains("construction: None, tick: TickGap::default(), held_turn: 0, scene,"), "{publish}");
}

/// rc6-hot-xps1 (2026-11-08, XPS): the first group exchange from either start-up window fell back with "group owner
/// receipt changed" and the player got the window's cell 0. H1 stamps the bank with the owner generation before the
/// scene's own `scene_changed`, whose Mii adapter step then opens a new one, so `group_owner` saw a stale epoch. The
/// window kept for the new scene joins that generation after the adapter's step.
#[test]
fn a_kept_start_up_window_joins_the_new_owner_generation() {
    let roster = include_str!("../src/runtime/roster/character_roster.rs");
    let changed = body(roster, "pub fn scene_changed(mode: game_mode::Mode) {");
    let bank = changed.find("body_bank::scene_changed();").expect("the bank's step");
    let mii = changed.find("mii_saved_alpha::scene_changed();").expect("the adapter's step");
    let adopt = changed.find("body_bank::adopt_scene_generation();").expect("the kept window joins the generation");
    assert!(bank < mii && mii < adopt, "{changed}");
    let adopt = body(BANK, "pub(super) fn adopt_scene_generation(");
    assert!(adopt.contains("let fresh = !bank.departed && bank.phase != Phase::Retired && bank.scene.is_some() && bank.scene == now;"),
        "{adopt}");
    assert!(adopt.contains("scene_generation(bank.epoch, super::mii_saved_alpha::current_generation(role as u32), fresh)"),
        "{adopt}");
    let owner = body(BANK, "unsafe fn group_owner(");
    assert!(owner.contains("if epoch != bank.epoch || resource != bank.alias { return None; }"), "the proof it satisfies");
}

/// rc6-hot-xps2 (2026-11-08, XPS): the origin both Mii adapters capture is the live entry after H3, the start-up window.
/// Staged as "back to the original" it reloaded three ordinary bodies natively and a construction thread died on an
/// unloaded resource. Both adapters stage native's single form for the original (`entry_window::original_request`).
#[test]
fn back_to_the_original_stages_natives_single_form() {
    let player = include_str!("../src/runtime/mii/mii_ordinary_player.rs");
    let arm = &player[player.find("            mii_cycle::Target::Original=>{\n                if let Some(c)=origin_saved {native::target_rows(c)?;}")
        .expect("the Player's Original staging")..];
    assert!(arm[..400].contains("entry_window::original_request(origin.descriptor,origin.original_kind)"), "{}", &arm[..400]);
    let cpu = include_str!("../src/runtime/mii/mii_ordinary_cpu.rs");
    let arm = &cpu[cpu.find("            mii_cycle::Target::Original=>{\n                if let Some(c)=origin_saved {native::target_rows(c)?;}")
        .expect("the CPU's Original staging")..];
    assert!(arm[..400].contains("descriptor=crate::engine::swap::entry_window::original_request(descriptor,origin.original_kind);"),
        "{}", &arm[..400]);
    // The returned original is recognised as the original: the Player's `at_origin`, the CPU's Fighter staging and the
    // CPU commit's `descriptor_equals_origin`.
    let at_origin = body(player, "unsafe fn at_origin(");
    assert!(at_origin.contains("expected.descriptor==crate::engine::swap::entry_window::original_request(origin.descriptor,origin.original_kind)"),
        "{at_origin}");
    assert!(cpu.contains("|| descriptor==crate::engine::swap::entry_window::original_request(live_origin,origin.original_kind)) {Target::ORIGINAL}"));
    let native = include_str!("../src/runtime/mii/mii_ordinary_cpu_native.rs");
    assert!(native.contains("|| now.descriptor==crate::engine::swap::entry_window::original_request(live_origin,owner.origin.original_kind),"));
}
