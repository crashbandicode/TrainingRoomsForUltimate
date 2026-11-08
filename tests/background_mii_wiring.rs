//! rc.6 item 3, Mii groups built in the background (operator 2026-11-08 20:26Z "All groups in rc.6";
//! `artifacts/phase23/rc6-item3-re/SPECIAL-GROUPS.md` §1). The eligibility rule is host-tested in
//! `src/model/swap/body_bank.rs`; this pins the runtime order: the identity bracket and the native Mii setup in the tick,
//! the shadow `R` and the group's values on the worker, the keys released with the identity in `R`, and each window's
//! own Mii set in `R` across every exchange and destroy.

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
const SETUP: &str = include_str!("../src/runtime/roster/mii_setup.rs");
const ROWS: &str = include_str!("../src/model/seams/native_seams/rows_a1.rs");

/// The tick: the live words, entry bytes and faces saved; the group's identity installed; the native setup at the
/// background seam (entry Ready); the third only when it prepared; the shadow taken from `R` with the group's words in
/// it; everything live put back before the window comes back.
#[test]
fn the_tick_brackets_the_identity_and_runs_the_setup() {
    let start = body(BANK, "unsafe fn start_background(");
    ordered(start, &[
        "let words = save_resource_words(alias);",
        "let identity_bytes = save_entry_identity(entry as usize);",
        "let live_faces: [u32; 2] = [fields::read_unchecked(alias, 0x90), fields::read_unchecked(alias, 0x94)];",
        "fields::write_unchecked(alias, 0x90, 0u32);",
        "write_identity(entry as usize, alias, identity);",
        "crate::runtime::roster::mii_setup::prepare_mii_background(base, entry as usize, owner, alias, &identity, &c);",
        "if mii_prepared { models(entry); }",
        "if mii_prepared { contexts(entry); }",
        "std::ptr::copy_nonoverlapping((alias + 0x40) as *const u8, (shadow + 0x40) as *mut u8, 0x68);",
        "write_resource_words(alias, words);",
        "write_entry_identity(entry as usize, identity_bytes);",
        "fields::write_unchecked(alias, 0x90, live_faces[0]);",
        "restore_window(entry as usize, &live);",
        "if !mii_prepared {",
        "bank.background = Some(Background {",
    ]);
    let setup = body(SETUP, "pub(crate) unsafe fn prepare_mii_background(");
    assert!(setup.contains("prepare_at(base, entry, owner, alias, identity, c, At::Background)"), "{setup}");
    assert!(SETUP.contains("fn seam(self) -> Seam { if matches!(self, At::Plan | At::Background) { Seam::Plan } else { Seam::PhaseB } }"));
    assert!(SETUP.contains("fn installed(self) -> bool { matches!(self, At::IdentityInstalled | At::Background) }"));
}

/// The worker (SPECIAL-GROUPS §1 (b), the 21:10Z intervals): the hooks act only for the armed build's Mii body; the
/// shadow in body init from `0x607aac` to `0x607e74` and in construct from `0x60b98c` to `0x60de68` and at `0x60bf30`;
/// the group's `FE+0x52`, `D+0x72` and `FE+0x41`; kind -1 for the tail's release; the owner-indexed readers redirected
/// only on the worker thread building that body, marked until each function's single epilogue. Every background body's
/// tail takes the per-kind release (X1). Armed before the bodies are queued (the real R last), disarmed at the settlement.
#[test]
fn the_worker_reads_the_shadow_and_the_groups_values() {
    let sites = [("0x607aac", "bg_mii_init_enter"), ("0x607d80", "bg_mii_init_flags"), ("0x607e74", "bg_mii_init_leave"),
        ("0x60b5c8", "bg_mii_init_exit"), ("0x60b7a8", "bg_mii_construct_index_a"), ("0x60b834", "bg_mii_construct_index_b"),
        ("0x60b98c", "bg_mii_construct_enter"), ("0x60bf30", "bg_mii_construct_path"), ("0x60de68", "bg_mii_construct_tail"),
        ("0x60dec4", "bg_mii_tail_release"), ("0x60df04", "bg_mii_construct_exit"), ("0x33a05a8", "bg_mii_reader_costume"),
        ("0x344fcd4", "bg_mii_reader_hat"), ("0x6e655c", "bg_mii_reader_6e655c"), ("0x6e6ccc", "bg_mii_reader_6e6ccc"),
        ("0x33c3468", "bg_mii_reader_33c3468")];
    let install = body(BANK, "pub(super) unsafe fn install(");
    for (offset, name) in sites {
        assert!(BANK.contains(&format!("#[skyline::hook(offset = {offset}, inline)]\nunsafe fn {name}(")), "{name}");
        assert!(ROWS.contains(&format!("group: \"native_body_bank\", offset: {offset},")), "{name}: seam row");
        assert!(install.contains(&format!("({offset}, 0x")), "{name}: its first word checked at install");
        assert!(install.contains(name), "{name}: installed");
    }
    let enter = body(BANK, "unsafe fn bg_mii_init_enter(");
    ordered(enter, &["ctx.registers[24].set_x(shadow_r_address() as u64);", "MII_THREAD.store(current_thread(), Ordering::Release);"]);
    let leave = body(BANK, "unsafe fn bg_mii_init_leave(");
    assert!(leave.contains("ctx.registers[24].set_x(MII_REAL_R.load(Ordering::Acquire) as u64);"), "{leave}");
    let construct = body(BANK, "unsafe fn bg_mii_construct_enter(");
    ordered(construct, &["ctx.registers[20].set_x(shadow_r_address() as u64);", "ctx.registers[1].set_x(shadow_r_address() as u64);",
        "ctx.registers[3].set_x(", "MII_THREAD.store(current_thread(), Ordering::Release);"]);
    let path = body(BANK, "unsafe fn bg_mii_construct_path(");
    assert!(path.contains("ctx.registers[1].set_x(shadow_r_address() as u64);"), "{path}");
    let tail = body(BANK, "unsafe fn bg_mii_construct_tail(");
    ordered(tail, &["ctx.registers[20].set_x(MII_REAL_R.load(Ordering::Acquire) as u64);",
        "if ctx.registers[8].w() == POPO && background_body(ctx.registers[19].x() as usize) {",
        "ctx.registers[8].set_x(u64::from(u32::MAX));"]);
    let release = body(BANK, "unsafe fn bg_mii_tail_release(");
    assert!(release.contains("ctx.registers[1].set_x(u64::from(u32::MAX));"), "{release}");
    let exit = body(BANK, "unsafe fn bg_mii_exit(");
    assert!(exit.contains("MII_THREAD.compare_exchange(thread, 0, Ordering::AcqRel, Ordering::Acquire)"), "{exit}");
    let reads = body(BANK, "fn mii_worker_reads(");
    assert!(reads.contains("real != 0 && r == real && MII_THREAD.load(Ordering::Acquire) == current_thread()"), "{reads}");
    let costume = body(BANK, "unsafe fn bg_mii_reader_costume(");
    ordered(costume, &["if mii_worker_reads(ctx.registers[8].x() as usize) {", "ctx.registers[8].set_x(shadow_r_address() as u64);"]);
    let hat = body(BANK, "unsafe fn bg_mii_reader_hat(");
    ordered(hat, &["if mii_worker_reads(ctx.registers[21].x() as usize) {",
        "ctx.registers[3].set_x(u64::from(fields::read_unchecked::<u32>(shadow_r_address(), 0x70)));"]);
    let queue = body(BANK, "unsafe fn queue_background_bodies(");
    ordered(queue, &["MII_VALUES.store(m.values, Ordering::Release);", "MII_OWNER.store(owner, Ordering::Release);",
        "MII_REAL_R.store(b.alias, Ordering::Release);", "let (tasks, queued) = queue_detached(base, owner, b.members);"]);
    let finish = body(BANK, "unsafe fn finish_background(");
    ordered(finish, &["MII_REAL_R.store(0, Ordering::Release);", "MII_THREAD.store(0, Ordering::Release);",
        "let object = [fields::read_unchecked::<usize>(shadow, 0x98), fields::read_unchecked::<usize>(shadow, 0xa0)];",
        "mii_keys = Some(release_group_keys_of(base, role, b.alias, b.group, &occupancy, owner, \"background Mii build\"));",
        "window.mii_r = mii_set;", "if let (false, Some(set)) = (stored, mii_set) { release_mii_r(base, set); }"]);
    ordered(install, &["let mii_ok = gate(SEAM_GROUP, BACKGROUND_MII_SEAMS,", "BACKGROUND_MII_ENABLED.store(true, Ordering::Release);"]);
}

/// SPECIAL-GROUPS §1 (c): a window's Mii set (`R+0x90..+0xa0`) is taken into its saved window at every detach and put
/// back at its attach; a failed exchange releases what it left; every destroy of a detached group releases its set.
#[test]
fn every_window_keeps_its_own_mii_set() {
    let swap = body(BANK, "unsafe fn perform_swap(");
    ordered(swap, &["(GroupState::Built, Some(y)) => {", "saved_x.mii_r = Some(take_mii_r(alias));", "restore_window(entry as usize, &y);",
        "put_mii_r(alias, y.mii_r.unwrap_or_default());", "(GroupState::Prefetched, None) => {", "saved_x.mii_r = Some(take_mii_r(alias));"]);
    let complete = body(BANK, "unsafe fn complete_exchange(");
    ordered(complete, &["restore_window(entry as usize, &x);", "let failed_set = take_mii_r(alias);",
        "put_mii_r(alias, x.mii_r.unwrap_or_default());", "release_mii_r(base, failed_set);"]);
    let retire = body(BANK, "unsafe fn retire_groups(");
    ordered(retire, &["filter_map(|g| g.saved.and_then(|w| w.mii_r))", "graph::execute(", "for set in mii_sets.iter() { release_mii_r(base, *set); }"]);
    let leave = body(BANK, "unsafe fn leave_group(");
    ordered(leave, &["graph::execute(", "if let Some(set) = mii_set { release_mii_r(base, set); }"]);
    let release = body(BANK, "unsafe fn release_mii_r(");
    ordered(release, &["if handle != 0 && holder != 0 && handle.wrapping_add(1) >= 3 { release_face(holder, handle); }",
        "if owners.fetch_sub(1, Ordering::AcqRel) == 0 {", "dispose(block);", "release_weak(block);"]);
}
