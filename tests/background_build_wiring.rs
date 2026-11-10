//! rc.6 item 3, background builds (operator decision 2026-11-08, `artifacts/phase23/INBOX-opus-rc6-hotslots.md` 19:12Z;
//! verdict and addresses in LANE-OPUS-RC6-HOTSLOTS-REPORT 19:46Z). The eligibility rule and the notice mapping are
//! host-tested in `src/model/swap/body_bank.rs`; this pins the runtime order: the one-tick entry-level third over the
//! group's words with the live window saved and put back, the worker's registration skipped, the bodies kept out of the
//! entry, the drain before every destroyer, and the gate.

fn body<'a>(src: &'a str, start: &str) -> &'a str {
    let at = src.find(start).unwrap_or_else(|| panic!("{start}"));
    &src[at..at + src[at..].find("\n}\n").expect("end of the function")]
}

fn ordered(src: &str, steps: &[&str]) {
    let mut from = 0;
    for step in steps {
        let at = src[from..].find(step).unwrap_or_else(|| panic!("missing or out of order: {step}\n{src}"));
        from += at + step.len();
    }
}

const BANK: &str = include_str!("../src/runtime/roster/native_body_bank.rs");
const VIEW: &str = include_str!("../src/runtime/roster/identity_view.rs");

/// Body init's `prepare_entry` (`0x607f30`) gets kind −1 for an owner whose background build is in flight: those
/// registry slots are the live window's.
#[test]
fn body_init_registers_nothing_for_a_background_build() {
    let site = body(VIEW, "unsafe fn init_prepare_entry(");
    ordered(site, &["rekey(ctx, 9, r, 3, Scope::Body(kind));",
        "if object(r) && crate::runtime::roster::character_roster::body_bank::background_skip(word(r, 0x54)) {",
        "ctx.registers[2].set_x(u64::from(u32::MAX));"]);
    let skip = body(BANK, "pub(crate) fn background_skip(");
    assert!(skip.contains("BACKGROUND_OWNERS.load(Ordering::Acquire) & owner_bit(owner) != 0"), "{skip}");
}

/// One game-thread tick: the live window and the info bytes saved and the group's table lent before the entry-level
/// third; the third over the group's words with every context word cleared; the group's window captured; the live
/// window, its indices and the info bytes put back; the owner's skip bit set before any body is queued.
#[test]
fn the_tick_builds_the_group_window_and_puts_the_live_window_back() {
    let start = body(BANK, "unsafe fn start_background(");
    ordered(start, &[
        "let Some(token) = super::INSTALL.try_begin(false) else { return; };",
        "let info_saved = save_info(info);",
        "let live = save_window(entry as usize);",
        "std::mem::swap(&mut bank.members, &mut bank.groups[group].members);",
        "let (lease, _) = r_handle::request_bit(lrole, alias);",
        "detach_dispatcher(entry as usize);",
        "for offset in [0x4178usize, 0x4180, 0x4188, 0x4190, 0x4198] { fields::write_unchecked(entry as usize, offset, 0usize); }",
        "write_group_words(entry as usize, members);",
        "fields::write_unchecked(entry as usize, 0x5918, 0u8);",
        "models(entry);", "contexts(entry);",
        "let mut made = save_window(entry as usize);",
        // SPECIAL-GROUPS §3: a live native window's records stay with it.
        "made.records = [0; 3];",
        "let scratch = scratch_address();",
        "std::ptr::copy_nonoverlapping(entry as *const u8, scratch as *mut u8, 0x100);",
        "std::ptr::copy_nonoverlapping((entry as usize + 0x4178) as *const u8, (scratch + 0x4178) as *mut u8, 0x28);",
        "fields::write_unchecked(scratch, 0x591f, made.flags);",
        "std::mem::swap(&mut bank.members, &mut bank.groups[group].members);",
        "restore_window(entry as usize, &live);",
        "fields::write_unchecked(entry as usize, 0x5918, live.index);",
        "fields::write_unchecked(entry as usize, 0x5919, live.previous);",
        "fields::write_unchecked(entry as usize, 0x5910, live.old);",
        "restore_info(info, &info_saved);",
        "BACKGROUND_TABLE.store(table, Ordering::Release);",
        "let control = queue_task(base, scratch, CONTROL_FUNCTOR_VTABLE, CONTROL_CALLBACK);",
        "bank.background = Some(Background {",
    ]);
    // The control init (22-151 ms on the game thread, rc6-hot-xps9) runs on the worker with the scratch as its object.
    assert!(!start.contains("CONTROL_INIT"), "{start}");
    assert!(!start.contains("queue_task(base, entry"), "{start}");
    assert!(!start.contains("install_cells"), "{start}");
    let table = body(BANK, "unsafe fn table(ctx: &mut InlineCtx, register: usize) {");
    ordered(table, &["let background = BACKGROUND_TABLE.load(Ordering::Acquire);",
        "if background != 0 && entry as usize == scratch_address() {", "ctx.registers[register].set_x(background as u64);",
        "let state = BANKS[role].lock();"]);
    let bodies = body(BANK, "unsafe fn queue_background_bodies(");
    ordered(bodies, &["let selected = fields::read::<usize>(scratch_address(), 0x4178).unwrap_or(0);",
        "BACKGROUND_TABLE.store(0, Ordering::Release);", "if selected == 0 || selected != b.window.group_ctx[0] { return false; }",
        "b.window.ctx[0] = selected;", "BACKGROUND_OWNERS.fetch_or(owner_bit(owner), Ordering::AcqRel);",
        "let (tasks, queued) = queue_detached(base, owner, b.members);"]);
    let settle = body(BANK, "unsafe fn settle_background(");
    ordered(settle, &["let pending = fields::read::<i32>(b.control, task::PENDING).unwrap_or(i32::MAX);",
        "free_tasks(base, &[control]);", "let queued = queue_background_bodies(base, owner, &mut b);"]);
    let queue = body(BANK, "unsafe fn queue_detached(");
    assert!(!queue.contains("install_cells"), "a background body never enters a cell:\n{queue}");
}

/// The settlement clears the skip bit first, keeps the bodies in the group record with the window the tick made, hands
/// the request bit back and releases INSTALL; a destroyer waits for the tasks first.
#[test]
fn the_settlement_keeps_the_bodies_in_the_group_and_every_destroyer_drains_first() {
    let finish = body(BANK, "unsafe fn finish_background(");
    ordered(finish, &[
        "BACKGROUND_OWNERS.fetch_and(!owner_bit(owner), Ordering::AcqRel);",
        "free_tasks(base, &b.tasks[..b.queued]);",
        // SPECIAL-GROUPS §0 X2: the cell byte body init wrote from the live window becomes the body's own cell.
        "cells_before[i] = fields::read_unchecked::<u8>(body.address, BODY_CELL_BYTE);",
        "fields::write_unchecked(body.address, BODY_CELL_BYTE, i as u8);",
        "r_handle::end_request(lrole, b.alias, b.lease)",
        "r_handle::construction_settled(lrole, b.group, &kinds[..b.queued]);",
        // SPECIAL-GROUPS §3: a native group's holder step is proven too (`holder_ok` is true for every other group).
        "Some(g) if g.state == GroupState::Prefetched && valid && holder_ok => {",
        "window.installed = count as u64;",
        "g.saved = Some(window);",
        "g.state = GroupState::Built;",
        "super::finish_install(b.token, \"background build settled\");",
    ]);
    assert!(!finish.contains("install_cells"), "{finish}");
    let drain = body(BANK, "unsafe fn drain_background(");
    ordered(drain, &["bank.background.take()", "wait(handle, control + task::PENDING);", "free_tasks(base, &[control]);",
        "if !queue_background_bodies(base, owner, &mut background) {", "wait(handle, t + task::PENDING);",
        "finish_background(entry, owner, role, background, reason, waited);"]);
    let retire = body(BANK, "unsafe fn retire_groups(");
    ordered(retire, &["drain_construction(entry, owner, role, reason);", "drain_background(entry, owner, role, reason);",
        "let (mut bundles, identities, token, alias, groups, mii_sets) = {"]);
}

/// The window's ticks settle a build in flight; one owed starts from the group lifecycle after every planned prefetch.
/// The notice counts an owed group as loading. The gate needs the Phase B′ rows and the identity view.
#[test]
fn the_lifecycle_starts_and_settles_it_and_the_gate_needs_both_paths() {
    let service = body(BANK, "unsafe fn service_locked(");
    ordered(service, &["else if bank.background.is_some() { Action::Background }",
        "|| background_candidate(bank, Some(members.kinds()[usize::from(index)])).is_some() {"]);
    // Inbox 22:17Z: each entry step names itself for its `slow step` line.
    assert!(BANK.contains("Action::Background => { settle_background(entry, owner, role); \"background settle\" }"));
    let lifecycle = body(BANK, "unsafe fn group_lifecycle(");
    ordered(lifecycle, &["(Some(group), _) if bank.groups[group].state == GroupState::Planned =>",
        "(_, Some(group)) => Step::Background(group),", "Step::Background(group) => start_background(entry, owner, role, group),"]);
    let candidate = body(BANK, "fn background_candidate(");
    assert!(candidate.contains("|| bank.background.is_some() { return None; }"), "{candidate}");
    // SPECIAL-GROUPS §0 X1/X4: the live window takes part in the rule; X6, with the owner's ordinary baseline beside a Mii.
    assert!(candidate.contains("*g.members, g.identity.is_some())), *bank.members, live_identity(bank),"), "{candidate}");
    assert!(BANK.contains("LiveIdentity::of(bank.identity.is_some(), bank.ordinary_identity.is_some())"));
    // D-KIRBY1's "live kirby" while the G68 step is native's; rc.7 F1 owns it, and then a live Kirby defers no build.
    assert!(candidate.contains("live_kirby_gate(live),"), "{candidate}");
    assert!(body(BANK, "fn live_kirby_gate(")
        .contains("crate::engine::swap::body_bank::d_kirby1_live(live, crate::runtime::roster::g68_step::enabled()) == Some(KIRBY)"));
    assert!(BANK.contains("const BODY_CELL_BYTE: usize = 0xf816;"));
    assert!(candidate.contains("bank.prefetch.is_some() || bank.construction.is_some()"), "{candidate}");
    let progress = body(BANK, "pub(crate) fn progress(");
    // Primary 2026-11-09 16:06Z: a prefetch held for memory shows as loading while a free's room is on its way.
    assert!(progress.contains("shown_state(arriving_room(g.state, coming), background_owed(bank, g, Some(active)))"), "{progress}");
    let install = body(BANK, "pub(super) unsafe fn install(");
    ordered(install, &["let identity_view = crate::runtime::roster::identity_view::install(base);",
        "BACKGROUND_ENABLED.store(async_ok && identity_view, Ordering::Release);"]);
}

const ROSTER: &str = include_str!("../src/runtime/roster/character_roster.rs");
const MII_PLAYER: &str = include_str!("../src/runtime/mii/mii_ordinary_player.rs");
const MII_CPU: &str = include_str!("../src/runtime/mii/mii_ordinary_cpu.rs");

const HATS: &str = include_str!("../src/runtime/roster/kirby_hats.rs");
const MODEL: &str = include_str!("../src/model/swap/body_bank.rs");

/// Primary 2026-11-09 07:17Z and 07:34Z (Eden `rc6-hot-xps31`/`32`: a press made during the background builds waited for
/// each in turn; then the 5 s release, a timeout guard, made exact). The rule is host-tested (`body_bank`
/// `a_queued_press_is_served_before_the_next_background_build`, `a_waiting_press_lets_its_targets_prefetch_through`).
/// Here: the lifecycle reads the foreground before the bank lock and puts the step serving the press's own target, or
/// none, in place of any chosen step that takes INSTALL; a serving prefetch is no speculation for the burst grace. The
/// foreground counts a request only while INSTALL is all it waits for (the roster's outside the menu, a Mii owner's with
/// every other `begin` blocker clear, a hat's queued source load). A Mii owner's press waits the tick its group's
/// CharInfo goes on the record, and no other CharInfo replaces it before that group's prefetch. No timeout is left.
#[test]
fn a_waiting_press_holds_every_speculative_step_but_its_own() {
    let lifecycle = body(BANK, "unsafe fn group_lifecycle(");
    ordered(lifecycle, &["let foreground = super::foreground(role, entry, owner);", "note_foreground(owner, foreground);",
        "let mut state = BANKS[role].lock();", "(_, Some(group)) => Step::Background(group),",
        "Step::Start(..) | Step::Background(_) | Step::Rearm(..) | Step::Prepare(..)",
        "if speculative_step_admitted(foreground.waiting).is_err() =>",
        "match foreground.target.and_then(|target| serving_group(base, bank, target, live)) {",
        "Some((group, Serving::Prefetch)) => Step::Start(group, *bank.groups[group].members, bank.alias, true),",
        "Some((group, Serving::ObjectRearm)) => Step::Rearm(group, *bank.groups[group].members, bank.alias),",
        "None => Step::Idle,", "let started = now_us();", "Step::Start(group, members, alias, serving) => {",
        "if !serving && crate::runtime::roster::cycle_preload::speculative_admission().is_none() { return; }"]);
    ordered(body(BANK, "unsafe fn serving_group("), &[
        "let live = crate::engine::swap::body_bank::d_kirby1_live(live, crate::runtime::roster::g68_step::enabled());",
        "bank.configuration == target.configuration { return None; }",
        "Some(c) => g.configuration == Some(c), None => g.identity.is_none()",
        "object_rearm_due(g.state, *g.members, detached_pool(base, bank.owner, g), g.object_held)",
        "object_rearm_admitted(live, kind)", "serving_step(g.state, g.identity.is_none() || g.charinfo_published, rearm_due)?;",
        "prefetch_admitted(live).is_ok()"]);
    assert!(body(BANK, "pub(super) fn pending_mii_publication(")
        .contains("if bank.groups.iter().any(|g| g.charinfo_published && g.state.startable()) { return None; }"));
    ordered(body(ROSTER, "pub(crate) unsafe fn foreground("), &["crate::runtime::roster::kirby_hats::foreground_pending()",
        "if !menu::captures_input() {", ".pending.filter(|p| !matches!(p.selection, Selection::RefreshPartners));",
        "cycle_step(&setting(&MENU), r, current, state_for(r).lock().cursor)",
        "mii_ordinary_player::foreground(role == Role::Player as usize)", "mii_ordinary_cpu::foreground(role == Role::Cpu as usize)"]);
    assert!(body(ROSTER, "unsafe fn service_body(").contains("let Some((kind, slot)) = cycle_step(&m, role, current, state.cursor) else {"));
    assert!(body(HATS, "pub(crate) fn foreground_pending(").contains("p.leases_released && p.lease.is_none() && !p.started"));
    // A press its owner cannot begin (no owner or catalog, a departed CPU) waits for that, not INSTALL: no foreground.
    ordered(body(MII_PLAYER, "pub(super) unsafe fn foreground("), &["if !permitted_but_install() {return None;}",
        "(s.owner.as_ref(),s.catalog.as_ref()) else {return None;};", "if !resolve {return Some(None);}"]);
    assert!(MII_PLAYER.contains("unsafe fn permitted()->bool {permitted_but_install() && !INSTALL.active()}"));
    ordered(body(MII_CPU, "pub(super) unsafe fn foreground("), &["if blocker(false).is_some() {return None;}",
        "(s.owner.as_ref(),s.catalog.as_ref(),s.departed) else {return None;};", "if !resolve {return Some(None);}"]);
    assert!(MII_CPU.contains("unsafe fn begin_blocker()->Option<Blocker> {blocker(true)}"));
    assert!(body(MII_CPU, "unsafe fn blocker(").contains("else if install && INSTALL.active() {Some(Blocker::Install)}"));
    for (owner, publication) in [(MII_PLAYER, "if mii_group_record_publication(entry) {return false;}"),
        (MII_CPU, "if mii_group_record_publication(entry,owner) {return false;}")] {
        ordered(body(owner, "pub(super) unsafe fn service("), &[publication, "begin(entry)"]);
    }
    for src in [BANK, ROSTER, MII_PLAYER, MII_CPU, HATS, MODEL] {
        assert!(!src.contains("FOREGROUND_WAIT_CAP_US") && !src.contains("FOREGROUND_SINCE"), "a timeout is left");
    }
}
