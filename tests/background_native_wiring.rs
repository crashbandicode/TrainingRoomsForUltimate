//! rc.6 item 3, worker B: Pokémon Trainer and Pyra/Mythra groups built in the background (operator 2026-11-08 "All
//! groups in rc.6"; design `artifacts/phase23/rc6-item3-re/SPECIAL-GROUPS.md` §3, §0 X5). The eligibility rule, the holder
//! step each group takes and the trainer task's layout are host-tested in `src/model/swap/body_bank.rs`; this pins the
//! runtime order: the X5 cell for an Aegis body, native loader state 11's holder step after the bodies settled, its
//! records straight into the group's saved window (never the live entry, never `0x658af0`), the object keys after it,
//! the drain, the gate.

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
const ROWS: &str = include_str!("../src/model/seams/native_seams/rows_a1.rs");
const SPECIAL: &str = include_str!("../src/runtime/roster/special_articles.rs");

/// Inbox 02:42Z (the attach's own rebuild: 209-382 ms the first time, 43-63 ms after, in the switch's setup tick): once a
/// detached PT group's pool is retired and the group holds its object kind again, its pool is built on the task worker
/// (native state 11's trainer task with the plugin's functor: only `0x3a6270`, no G68 registration, and clones that keep
/// the plugin's vtable), under INSTALL; when it settles the kind goes back, as after an attach's rebuild. The attach then
/// rebuilds nothing. Teardowns wait for the task; without the checked slots the attach keeps its own rebuild.
#[test]
fn a_retired_pt_pool_is_prepared_on_the_worker_before_its_attach() {
    let lifecycle = body(BANK, "unsafe fn group_lifecycle(");
    ordered(lifecycle, &["} else if let Some(r) = bank.retiring {", "} else if let Some(p) = bank.preparing {", "Step::PreparePoll(p)",
        "} else if let Some(group) = prepare_candidate(base, bank) {", "Step::Prepare(group",
        "Step::Prepare(group, holder) => {", "super::INSTALL.try_begin(false)", "queue_prepare(base, owner, holder)",
        "bank.preparing = Some(Preparing {", "Step::PreparePoll(p) => {", "fields::read::<i32>(p.task, task::PENDING)",
        "prepare_settled(base, role, owner, p, \"settled\")"]);
    let settled = body(BANK, "unsafe fn prepare_settled(");
    ordered(settled, &["free_tasks(base, &[p.task]);", "super::finish_install(p.token, \"articles prepared\")",
        "return_object_key(base, role, owner, p.group, \"articles prepared on the worker\");"]);
    assert!(body(BANK, "unsafe fn prepare_candidate(").contains("prepare_due("));
    let call = body(BANK, "unsafe extern \"C\" fn prepare_call(");
    ordered(call, &["let Some(task) = fields::child(functor, 8) else { return; };", "articles::build_pool(base, holder, slot);"]);
    assert!(body(BANK, "unsafe extern \"C\" fn prepare_clone_into(").contains("PREPARE_VTABLE.as_ptr() as usize"));
    assert!(body(BANK, "unsafe extern \"C\" fn prepare_clone_heap(").contains("prepare_clone_into(src, dst);"));
    let reachable = body(BANK, "fn group_reachable(");
    assert!(reachable.contains("bank.preparing.is_none()"), "{reachable}");
    ordered(body(BANK, "unsafe fn retire_groups("), &["finish_retirement(base, role, owner, reason);", "finish_preparation(base, role, owner, reason);"]);
    ordered(body(BANK, "unsafe fn leave_group("), &["finish_retirement(base, role, owner, \"left the cycle\");",
        "finish_preparation(base, role, owner, \"left the cycle\");"]);
    ordered(body(BANK, "pub(super) unsafe fn install(base: usize) {"), &["let prepare_ok = native_ok",
        "TRAINER_FUNCTOR_VTABLE + 0x10", "TRAINER_FUNCTOR_VTABLE + 0x18", "slot.store(value, Ordering::Release);",
        "PREPARE_ENABLED.store(prepare_ok, Ordering::Release);"]);
    let rebuild = body(SPECIAL, "pub(crate) unsafe fn rebuild_current(");
    assert!(rebuild.contains("build_pool(base, holder, slot)"), "{rebuild}");
}

/// Inbox 02:00Z (leaving a PT group: 114.9-118.5 ms, a 5-frame gap; Eden rc6-bg-ptx1/ptx2 put all of it in pass 2 of
/// `0x3a6b80`, the trainer's teardown and its own pools): the exit's tick runs pass 1 only (`deactivate_saved`); the
/// group lifecycle's Retire step then releases the trainer's own pools and destroys it, one stage per later tick under
/// INSTALL (a press meanwhile waits). A retiring group is not attached warm, and a teardown, a leave and a rollback
/// finish the stages first. Gated on the split's own seam rows; without them the exit keeps native's whole release.
#[test]
fn a_pt_exit_retires_its_pool_over_later_ticks() {
    let articles = body(BANK, "unsafe fn exchange_articles(");
    ordered(articles, &["finish_retirement(base, role, owner, \"another exit\");", "let split = EXIT_SPLIT.load(Ordering::Acquire);",
        "articles::deactivate_saved(owner, flags, holder, valid, \"detach\")",
        "articles::retire_saved(base, owner, flags, holder, valid, \"detach\")",
        "bank.retiring = Some(Retiring { group, holder, stage: RetireStage::Deactivate, token: None });"]);
    let lifecycle = body(BANK, "unsafe fn group_lifecycle(");
    // After the in-flight polls (they hold INSTALL, which the stages take), before anything new starts.
    ordered(lifecycle, &["if let Some((group, token, since)) = bank.object_fetch {", "} else if let Some((group, token, since)) = bank.prefetch {",
        "} else if let Some(r) = bank.retiring {", "Step::Retire(r)", "} else if let Err(reason) =",
        "Step::Retire(r) => retire_step(base, role, owner, r, \"detach\"),"]);
    assert!(body(BANK, "unsafe fn retire_step(").contains("retire_stage(base, role, owner, r, why, false);"));
    assert!(body(BANK, "unsafe fn finish_retirement(").contains("retire_stage(base, role, owner, r, why, true);"));
    let step = body(BANK, "unsafe fn retire_stage(");
    ordered(step, &["let Some(stage) = r.stage.next() else", "if current != Some(r.holder) || !valid_special_object(base, r.holder, owner) {",
        "super::INSTALL.try_begin(false)", "RetireStage::SubPools => {", "articles::release_sub_pools(base, r.holder)",
        "RetireStage::Destroy => (0, 0, articles::destroy_saved(base, owner, r.holder, why)),",
        "super::finish_install(token, \"articles retired\")"]);
    let reachable = body(BANK, "fn group_reachable(");
    assert!(reachable.contains("attachable(bank.retiring.filter(|r| r.group == i).map(|r| r.stage))"), "{reachable}");
    ordered(body(BANK, "unsafe fn retire_groups("), &["drain_background(entry, owner, role, reason);",
        "finish_retirement(base, role, owner, reason);"]);
    ordered(body(BANK, "unsafe fn leave_group("), &["let base = BASE.load(Ordering::Acquire);",
        "finish_retirement(base, role, owner, \"left the cycle\");"]);
    ordered(body(BANK, "unsafe fn complete_exchange("), &["let Some((group, via)) = rollback else { return; };",
        "finish_retirement(base, role, owner, \"swap rollback\");", "restore_window(entry as usize, &x);"]);
    ordered(body(BANK, "unsafe fn service_locked("), &["else if bank.background.is_some() { Action::Background }",
        "else if bank.retiring.is_some() || bank.preparing.is_some()"]);
    // Pass 2 as native runs it, except that an id `0x3ac560` no longer finds is never handed to `0x3aeb50`.
    ordered(body(SPECIAL, "pub(crate) unsafe fn destroy_saved("), &["let found = lookup(base, id);",
        "if found == 0 || fields::read::<u8>(found, 0x3a).unwrap_or(0) <= 3 { static_init(base); }",
        "if found == 0 {", "destroy(object_id(object).unwrap_or(id));", "fields::write_unchecked(slot, 0, 0usize);",
        "fields::write_unchecked(holder, POOL_COUNT, 0u64);", "fields::write_unchecked(holder, TABLE, 0usize);"]);
    ordered(body(SPECIAL, "pub(crate) unsafe fn release_sub_pools("), &["fields::child(battle, 0x20)",
        ".and_then(|accessor| fields::child(accessor, 0x98))", ".and_then(|module| fields::child(module, 0x240))",
        "fields::read::<usize>(*pool, 0) == Some(base + HOLDER_VTABLE)", "release(base, pool);"]);
    ordered(body(BANK, "pub(super) unsafe fn install(base: usize) {"), &["let split_ok = gate(SEAM_GROUP, EXIT_SPLIT_SEAMS, special_ok",
        "EXIT_SPLIT.store(split_ok, Ordering::Release);"]);
    for label in ["special_groups.object_lookup", "special_groups.object_destroy", "special_groups.static_init"] {
        assert!(ROWS.contains(&format!("label: \"{label}\"")), "{label}");
    }
}

/// X5 (`0x607ba8`): an Aegis body reads the live `FE+0x1c`; beside a one-member window that is −1 and the body would load
/// model kind `0x73`/`0x74`. A background body of an Aegis group reads its group's cell 1, set before any body is queued
/// and cleared at the settlement.
#[test]
fn an_aegis_body_built_in_the_background_reads_its_groups_cell_1() {
    assert!(BANK.contains("#[skyline::hook(offset = 0x607bac, inline)]\nunsafe fn background_partner(ctx: &mut InlineCtx) {"));
    let hook = body(BANK, "unsafe fn background_partner(ctx: &mut InlineCtx) {");
    ordered(hook, &["let cell = BACKGROUND_PARTNER.load(Ordering::Acquire);", "if cell == u32::MAX { return; }",
        "let r = ctx.registers[24].x() as usize;",
        "BACKGROUND_OWNERS.load(Ordering::Acquire) & owner_bit(fields::read_unchecked::<u32>(r, 0x54)) != 0",
        "ctx.registers[8].set_x(u64::from(cell));"]);
    let queue = body(BANK, "unsafe fn queue_background_bodies(");
    ordered(queue, &["BACKGROUND_PARTNER.store(b.members.partner_cell().unwrap_or(u32::MAX), Ordering::Release);",
        "BACKGROUND_OWNERS.fetch_or(owner_bit(owner), Ordering::AcqRel);", "queue_detached(base, owner, b.members);"]);
    let finish = body(BANK, "unsafe fn finish_background(");
    ordered(finish, &["BACKGROUND_OWNERS.fetch_and(!owner_bit(owner), Ordering::AcqRel);",
        "BACKGROUND_PARTNER.store(u32::MAX, Ordering::Release);"]);
    assert!(ROWS.contains("Seam { label: \"background_build.aegis_partner\", group: \"native_body_bank\", offset: 0x607bac,"));
    for label in ["trainer_functor", "trainer_callback", "guard_acquire", "guard_release", "pool_build"] {
        assert!(ROWS.contains(&format!("label: \"background_build.{label}\"")), "{label}");
    }
}

/// Native loader state 11 after the bodies settled: the holder (and PT's object and pool task) into the build, polled
/// like the bodies; the records into the group's saved window; the object kind returned only after the step. A live
/// native window's records stay in the entry; the group's window takes none from it.
#[test]
fn the_holder_step_follows_the_bodies_and_its_records_go_into_the_group_window() {
    let start = body(BANK, "unsafe fn start_background(");
    ordered(start, &["if members.carries_records() && !NATIVE_BACKGROUND.load(Ordering::Acquire) {",
        "refuse_background(role, group, owner, \"native rows off\");",
        "let mut made = save_window(entry as usize);", "made.records = [0; 3];",
        "restore_window(entry as usize, &live);",
        // The Mii branch's `pads` field follows in the same initialiser (integration merge 2026-11-09).
        "holder_due: false, records: [0; 3], trainer: 0"]);
    let settle = body(BANK, "unsafe fn settle_background(");
    ordered(settle, &["if b.trainer != 0 {", "let pending = fields::read::<i32>(b.trainer, task::PENDING).unwrap_or(i32::MAX);",
        "if !task::settled(&pending[..b.queued]) {", "bank.background.take().unwrap()",
        "!background.holder_due && background.queued == background.members.count()",
        "start_holder(base, entry as usize, owner, &mut background, step);", "if background.trainer != 0 {",
        "bank.background = Some(background);", "finish_background(entry, owner, role, background, \"settled\", 0);"]);
    let holder = body(BANK, "unsafe fn start_holder(");
    ordered(holder, &["b.holder_due = true;", "let holder = heap_alloc(HOLDER_ALIGN, HOLDER_SIZE);", "init_holder(base, holder);",
        "HolderStep::Partner => {", "partner_guards(base);",
        "build(holder, base + PARTNER_TABLE, 0, u64::from(u32::MAX), owner, 0);", "b.records = [holder, 0, 0];",
        "HolderStep::Trainer => {", "let object = heap_alloc(HOLDER_ALIGN, trainer_task::OBJECT_SIZE);",
        "b.records = [holder, 0, object];", "b.trainer = queue_trainer(base, entry, owner, holder);"]);
    let init = body(BANK, "unsafe fn init_holder(");
    ordered(init, &["fields::write_unchecked(holder, 0x00, base + SPECIAL_OBJECT_VTABLE);",
        "fields::write_unchecked(holder, 0x08, 0x40u64);", "fields::write_unchecked(holder, 0x18, holder + 0x20);",
        "fields::write_unchecked(holder, 0x230, u32::MAX);", "fields::write_unchecked(holder, 0x234, 6u8);"]);
    let trainer = body(BANK, "unsafe fn queue_trainer(");
    ordered(trainer, &["let t = heap_alloc(task::ALIGN, trainer_task::SIZE);",
        "std::ptr::write_bytes(t as *mut u8, 0, trainer_task::SIZE);",
        "fields::write_unchecked(t, trainer_task::HOLDER, holder);", "fields::write_unchecked(t, trainer_task::OWNER, owner);",
        "fields::write_unchecked(t, task::QUEUE, task::QUEUE_ID);",
        "fields::write_unchecked(t, trainer_task::COSTUME, fields::read_unchecked::<u8>(entry, 0x41));",
        "trainer_task::flag(fields::read_unchecked::<u8>(entry, 0x52), fields::read_unchecked::<u8>(entry, 0x53))",
        "fields::write_unchecked(t, trainer_task::BUILD, 1u8);",
        "schedule_task(base, t, base + TRAINER_FUNCTOR_VTABLE, TRAINER_CALLBACK);"]);
    let guards = body(BANK, "unsafe fn partner_guards(");
    ordered(guards, &["acquire(base + first) != 0", "release(base + first);", "acquire(base + second) != 0",
        "fields::write_unchecked(base, PARTNER_TABLE, base + PARTNER_ENTRY);", "release(base + second);"]);
    // The live entry's records are never written, and `0x658af0` (196 ms of the game thread for PT) is never called.
    for f in [holder, init, trainer, guards] {
        assert!(!f.contains("0x5a88") && !f.contains("SYNC_CONSTRUCT") && !f.contains("sync_construct"), "{f}");
    }
    let finish = body(BANK, "unsafe fn finish_background(");
    ordered(finish, &["free_tasks(base, &b.tasks[..b.queued]);", "free_tasks(base, &[b.trainer]);",
        "let holder_ok = !b.holder_due || (valid_special_object(base, b.records[0], owner)",
        "Some(g) if g.state == GroupState::Prefetched && valid && holder_ok => {", "window.records = b.records;",
        "g.saved = Some(window);",
        "if let Some(kind) = b.members.object_kind().filter(|_| stored && b.holder_due && alias_ok) {",
        "r_handle::release(base, lrole, Some(b.group), Some(&object[..]), &Occupancy::default(), \"background object release\");",
        "super::finish_install(b.token, \"background build settled\");"]);
    let drain = body(BANK, "unsafe fn drain_background(");
    ordered(drain, &["wait(handle, t + task::PENDING);",
        "if background.trainer != 0 && fields::read::<i32>(background.trainer, task::PENDING).unwrap_or(0) > 0 {",
        "wait(handle, background.trainer + task::PENDING);", "finish_background(entry, owner, role, background, reason, waited);"]);
    // The body task's scheduling is shared with the trainer task, unchanged.
    let queue = body(BANK, "unsafe fn queue_task(");
    ordered(queue, &["let t = heap_alloc(task::ALIGN, task::SIZE);", "fields::write_unchecked(t, task::BODY, object);",
        "schedule_task(base, t, base + vtable, callback);"]);
    let schedule = body(BANK, "unsafe fn schedule_task(");
    ordered(schedule, &["let mut functor = [vtable, t, 0, 0, 0];", "functor_swap(&mut functor, t + task::FUNCTOR);",
        "schedule(t + task::HANDLE, base + callback, task::SCHEDULE_ARGS.0, task::SCHEDULE_ARGS.1);"]);
}

/// Inbox 00:18Z (a console-crash class, at the base too): a retired PT pool is rebuilt at the group's next warm attach and
/// the trainer article reads its file through the object kind `0x72`. A detached group with a retired pool takes the kind
/// again (a lifecycle step holding INSTALL, as a prefetch does), is reachable warm only while it holds it, and gives it
/// back right after the attach's rebuild; every drain and teardown gives it back too, so no object token outlives the
/// detached time (IC-PAIR-STALL).
#[test]
fn a_retired_trainer_group_takes_its_object_kind_again_and_returns_it_after_the_rebuild() {
    let lifecycle = body(BANK, "unsafe fn group_lifecycle(");
    ordered(lifecycle, &["if let Some((group, token, since)) = bank.object_fetch {", "Step::RearmPoll(group, token, since",
        "Step::Rearm(group,", "Step::Rearm(group, members, alias) => {", "let Some(token) = super::INSTALL.try_begin(false) else { return; };",
        "r_handle::acquire(base, lrole, group, kind, Holder::Prefetch(GroupId(group as u8)), member_recipe(members, kind), &occupancy)",
        "bank.object_fetch = Some((group, token, now));", "Step::RearmPoll(group, token, since, kind, alias) => {",
        "g.object_held = outcome == GroupState::Prefetched;", "super::finish_install(token, \"object kind re-acquired\");"]);
    let due = body(BANK, "unsafe fn rearm_candidate(");
    assert!(due.contains("object_rearm_due(g.state, *g.members, detached_pool(base, owner, g), g.object_held)"), "{due}");
    let articles = body(BANK, "unsafe fn exchange_articles(");
    ordered(articles, &["articles::rebuild_current(base, owner, entry, current_holder_valid(base, entry, owner), \"warm attach\");",
        "return_object_key(base, role, owner, group, \"warm attach rebuilt its articles\");"]);
    let back = body(BANK, "unsafe fn return_object_key(");
    ordered(back, &["let held = std::mem::replace(&mut g.object_held, false);", "if !held { return; }",
        "r_handle::release(base, ledger_role(role), Some(group), Some(&[kind][..]), &Occupancy::default(), why);"]);
    let reachable = body(BANK, "fn group_reachable(");
    ordered(reachable, &["bank.object_fetch.is_none()", "object_ready(*g.members, unsafe { detached_pool(base, owner, g) }, g.object_held)"]);
    let switch = body(BANK, "pub(super) unsafe fn switch(");
    ordered(switch, &["let drain = {", "return_object_key(base, role, owner, group, why);", "let mut state = BANKS[role].lock();"]);
    let leave = body(BANK, "unsafe fn leave_group(");
    assert!(leave.contains("g.object_held = false;"), "{leave}");
    // Eden rc6-bg-pt4: a teardown released no re-acquired token (a Built group has no held kinds, so the plan's key step
    // never ran) and the next Reload's state 7 refused on `R+0x18 == 2`. Both teardowns name it a held kind.
    assert!(leave.contains("if g.object_held { detached.held_kinds.push(PT_OBJECT_KIND); }"), "{leave}");
    assert!(body(BANK, "unsafe fn retire_groups(").contains("if group.object_held { detached.held_kinds.push(PT_OBJECT_KIND); }"));
    let retire = body(BANK, "unsafe fn retire_groups(");
    ordered(retire, &["let token = bank.prefetch.take().map(|p| p.1);", "let rearm = bank.object_fetch.take().map(|f| f.1);",
        "if let Some(token) = rearm { super::finish_install(token, \"object re-acquire retired\"); }"]);
    let service = body(BANK, "unsafe fn service_locked(");
    assert!(service.contains("else if bank.object_fetch.is_some() || rearm_candidate(BASE.load(Ordering::Acquire), bank).is_some() { Action::Lifecycle }"), "{service}");
}

/// Primary 2026-11-09 05:38Z: under a live Kirby the D-KIRBY1 branch still hands back every Prefetched group's keys
/// first and defers every Planned group (fighter keys), but a detached PT group's object re-arm
/// (`body_bank::object_rearm_admitted`) runs there, as the same `Step::Rearm` with its ledger call unchanged (the
/// occupancy names the live copy; the token is the group's `Holder::Prefetch`).
#[test]
fn a_trainer_object_rearm_runs_under_a_live_kirby_through_the_same_ledger_path() {
    let lifecycle = body(BANK, "unsafe fn group_lifecycle(");
    ordered(lifecycle, &["} else if let Err(reason) = crate::engine::swap::body_bank::prefetch_admitted(live) {",
        "match bank.groups.iter().position(|g| g.state == GroupState::Prefetched) {",
        "Some(group) => Step::Release(group, *bank.groups[group].members, window, alias),",
        "None => match kirby_rearm(base, bank, live) {", "Some(group) => Step::Rearm(group, *bank.groups[group].members, alias),",
        "prefetch deferred owner={} group={} kinds={:x?} reason={} (D-KIRBY1)",
        "} else if let Some(group) = rearm_candidate(base, bank) {", "Step::Rearm(group, *bank.groups[group].members, bank.alias)",
        "Step::Rearm(group, members, alias) => {",
        "let copy = ready_members(entry, owner).and_then(|window| live_kirby_copy_kind(entry, owner, window));",
        "let occupancy = occupancy_snapshot(role, copy);",
        "r_handle::acquire(base, lrole, group, kind, Holder::Prefetch(GroupId(group as u8)), member_recipe(members, kind), &occupancy)"]);
    let gate = body(BANK, "unsafe fn kirby_rearm(");
    ordered(gate, &["let group = rearm_candidate(base, bank)?;", "bank.groups[group].members.object_kind()",
        "crate::engine::swap::body_bank::object_rearm_admitted(live, kind)"]);
}

/// The gate: the X5 site's words, the trainer functor through its relocated vtable slot, its callback and the guard
/// functions, all or nothing, after the background build's own gate.
#[test]
fn native_groups_build_in_the_background_only_with_their_rows() {
    let install = body(BANK, "pub(super) unsafe fn install(");
    ordered(install, &["BACKGROUND_ENABLED.store(async_ok && identity_view, Ordering::Release);",
        "let trainer_slot = fields::read_unchecked::<usize>(base, TRAINER_FUNCTOR_VTABLE + 0x30) == base + TRAINER_FUNCTOR;",
        "let native_ok = gate(SEAM_GROUP, NATIVE_BACKGROUND_SEAMS, async_ok && identity_view && special_ok && trainer_slot",
        "fingerprint(base, PARTNER_SITE - 4, &[0xb9401ea8, 0x3100051f, 0x54000380, 0x52800ea2], \"GroupSwap\")",
        "if native_ok { skyline::install_hooks!(background_partner); }",
        "NATIVE_BACKGROUND.store(native_ok, Ordering::Release);"]);
    assert!(BANK.contains("const PARTNER_SITE: usize = 0x607bac;"));
    assert!(BANK.contains("const TRAINER_FUNCTOR_VTABLE: usize = 0x4f81498;"));
    assert!(BANK.contains("const TRAINER_CALLBACK: usize = 0x651770;"));
    assert!(BANK.contains("const PARTNER_TABLE: usize = 0x52b94c8;"));
}

/// rc.6 item 3, a session that starts on Pokémon Trainer or Pyra/Mythra (INBOX-opus-rc6-hotslots 01:09Z; model rules
/// `entry_window::a_native_group_start_is_its_own_window_lead`, `a_native_group_original_is_a_bank_member_while_native_groups_are_on`,
/// `body_bank::a_live_native_window_plans_what_it_serves`). H1 makes the native group its own window lead only with
/// native groups on, the original bank on and the group as slot 0's requested form; H3 keeps native's creation (its
/// starting form included) and writes nothing; H4 binds on its words. Both Mii adapters' Original gates and the planner's
/// canonical-original exclusion ask the native arm of `original_form_bankable`, so the return to that original is a warm
/// switch, never the Reload that retires the role's bank.
#[test]
fn the_start_up_window_serves_a_native_group_start() {
    let h1 = body(BANK, "unsafe fn scene_header(");
    ordered(h1, &["let Some(original) = extendable(facts) else { return; };",
        "if Members::native_group(original).is_some() && !(special_policy(&settings).native && settings.original_bank_enabled()",
        "&& super::requested_forms_at(&menu, role, original, Some(0)) == Forms::starting(original)) { return; }",
        "let Some(members) = entry_window(", "if !publish_pending(facts.owner, members, scene) { return; }"]);
    let h3 = body(BANK, "pub(crate) unsafe fn creation(");
    ordered(h3, &["if !creation_matches(native, window) {", "if keeps_native_creation(window) {", "return;",
        "fields::write_unchecked(descriptor as usize, 0x14, window.count);"]);
    let h4 = body(BANK, "unsafe fn bind_pending(");
    assert!(h4.contains("read::<u32>(entry, 0x28) == bank.members.actor_count()"), "{h4}");
    let member = body(BANK, "pub(crate) fn original_member(");
    ordered(member, &["if kind != POPO && Members::native_group(kind).is_none() { return original_bankable(kind); }",
        "original_form_bankable(kind, forms, pair_enabled(&settings), special_policy(&settings).native)"]);
    let excluded = body(BANK, "fn excluded_original(");
    assert!(excluded.contains("special_policy(settings).native)))"), "{excluded}");
    let budget = body(BANK, "pub(crate) fn plan_budget(");
    assert!(budget.contains("crate::engine::swap::body_bank::window_targets(grouped, live[role], originals[role], next)"), "{budget}");
}
