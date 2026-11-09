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
        "holder_due: false, records: [0; 3], trainer: 0 });"]);
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
        "schedule_task(base, t, TRAINER_FUNCTOR_VTABLE, TRAINER_CALLBACK);"]);
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
        "schedule_task(base, t, vtable, callback);"]);
    let schedule = body(BANK, "unsafe fn schedule_task(");
    ordered(schedule, &["let mut functor = [base + vtable, t, 0, 0, 0];", "functor_swap(&mut functor, t + task::FUNCTOR);",
        "schedule(t + task::HANDLE, base + callback, task::SCHEDULE_ARGS.0, task::SCHEDULE_ARGS.1);"]);
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
