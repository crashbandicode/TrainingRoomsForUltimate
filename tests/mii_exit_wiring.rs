//! rc.6 warm exit from a Mii group (operator decision 2026-11-08, `artifacts/phase23/INBOX-opus-rc6-hotslots.md` 15:58Z;
//! console F6 `#106`: Mii Gunner → Pichu, whose body the Mii group had kept built, took a 5.9 s Reload). The byte rule
//! is host-tested in `src/model/swap/body_bank.rs`; this pins the planner's opt-in path, the kept publication and the
//! settle order in both adapters.

fn body<'a>(src: &'a str, start: &str) -> &'a str {
    let at = src.find(start).unwrap_or_else(|| panic!("{start}"));
    &src[at..at + src[at..].find("\n}\n").expect("end of the function")]
}

const BANK: &str = include_str!("../src/runtime/roster/native_body_bank.rs");
const PLAYER: &str = include_str!("../src/runtime/mii/mii_ordinary_player.rs");
const PLAYER_NATIVE: &str = include_str!("../src/runtime/mii/mii_ordinary_native.rs");
const CPU: &str = include_str!("../src/runtime/mii/mii_ordinary_cpu.rs");
const CPU_NATIVE: &str = include_str!("../src/runtime/mii/mii_ordinary_cpu_native.rs");

/// `plan()` keeps today's rule; only an adapter holding its Mii group's publication asks for the exit. Since inbox
/// 21:27Z it is served by any Built ordinary group: the displaced window with its saved identity move, any other with
/// the owner's ordinary baseline (`Bank::ordinary_identity`).
#[test]
fn the_planner_serves_the_exit_only_on_request_and_from_any_built_ordinary_group() {
    assert!(BANK.contains("pub(super) unsafe fn plan(entry: *const u8, owner: u32, descriptor: [u8; 0x70], upcoming: &[u32]) -> Plan {\n    plan_in(entry, owner, descriptor, upcoming, false)\n}"));
    assert!(BANK.contains("pub(super) unsafe fn plan_mii_exit(entry: *const u8, owner: u32, descriptor: [u8; 0x70], upcoming: &[u32]) -> Plan {\n    plan_in(entry, owner, descriptor, upcoming, true)\n}"));
    let plan = body(BANK, "unsafe fn plan_in(");
    assert!(plan.contains("let exit = (mii_exit && in_group && !(0x48..=0x4a).contains(&target)).then(|| mii_exit_saved(owner, target, forms)).flatten();"), "{plan}");
    assert!(plan.contains("|| exit.is_some_and(|saved| mii_exit_bytes_match(&current, physical.bytes(), &saved))"), "{plan}");
    let saved = body(BANK, "fn mii_exit_saved(");
    assert!(saved.contains("if bank.identity.is_none() { return None; }"), "the window is a Mii group:\n{saved}");
    assert!(saved.contains("g.state.reachable() && group_serves(*g.members, kind, forms).is_some()"), "the group `switch` takes:\n{saved}");
    assert!(saved.contains("group.state != GroupState::Built || group.identity.is_some()"), "{saved}");
    assert!(saved.contains("group.saved.as_ref()?.identity.or(bank.ordinary_identity).map(|(_, saved)| saved.entry)"), "{saved}");
}

/// The exchange: the first identity move out of an ordinary window records the owner's ordinary baseline; an attach of
/// an ordinary group that saved no identity move, out of a Mii window, writes that baseline with its own header.
#[test]
fn the_exchange_writes_the_ordinary_baseline_for_any_ordinary_target() {
    let swap = body(BANK, "unsafe fn perform_swap(");
    let at = swap.find("let x_identity = std::mem::replace(&mut bank.identity, bank.groups[swap.group].identity);").expect("x_identity");
    let tail = &swap[at..];
    let mut from = 0;
    for step in [
        "saved_x.identity = Some((save_resource_words(bank.alias), save_entry_identity(entry as usize)));",
        "if x_identity.is_none() { bank.ordinary_identity = saved_x.identity; }",
        "let baseline = bank.ordinary_identity.filter(|_| x_identity.is_some());",
        "(GroupState::Built, Some(y)) => {",
        // rc6-hot-xps15: only into or out of a Mii window (`body_bank::identity_move`), never between two ordinary ones.
        "identity_move(target_identity.is_some(), x_identity.is_some(), y.identity, baseline)",
        "(None, IdentityMove::Ordinary((words, saved))) => {",
        "let words = baseline_words(words, target_members);",
        "write_resource_words(alias, &words);",
        "write_entry_identity(entry as usize, &saved);",
        "(None, _) => None,",
    ] {
        let found = tail[from..].find(step).unwrap_or_else(|| panic!("missing or out of order: {step}\n{tail}"));
        from += found + step.len();
    }
    assert!(BANK.contains("ordinary_identity: None,"), "every new bank starts without a baseline");
}

/// Eden rc6-hot-xps16: after an exit through the baseline the Mii group sat in another group record, whose configuration
/// was not its own; `reachable_mii_group` (identity and configuration of one record) stopped matching and the next switch
/// to the Mii took the canonical Reload. The configuration moves with the identity at every exchange and rollback.
#[test]
fn the_configuration_moves_with_its_identity() {
    let swap = body(BANK, "unsafe fn perform_swap(");
    let mut from = 0;
    for step in [
        "let x_identity = std::mem::replace(&mut bank.identity, bank.groups[swap.group].identity);",
        "let x_configuration = std::mem::replace(&mut bank.configuration, bank.groups[swap.group].configuration);",
        "group.identity = x_identity;",
        "group.configuration = x_configuration;",
        "mii_build = target_identity.zip(bank.configuration)",
        "bank.identity = x_identity;",
        "bank.configuration = x_configuration;",
    ] {
        let found = swap[from..].find(step).unwrap_or_else(|| panic!("missing or out of order: {step}\n{swap}"));
        from += found + step.len();
    }
    let complete = body(BANK, "unsafe fn complete_exchange(");
    assert!(complete.contains("bank.configuration = std::mem::replace(&mut record.configuration, target_configuration);"), "{complete}");
    assert!(BANK.contains("identity: None, configuration: None, ordinary_identity: None,"));
}

/// Both adapters: a settled Mii group switch keeps its publication; a request out of that live Mii plans the exit
/// only while the owner's receipt is the one that switch adopted; the exit settles through `exit_settled`.
#[test]
fn both_adapters_keep_the_publication_and_settle_the_exit_through_it() {
    for (name, adapter, live) in [("Player", PLAYER, "active.is_some()"), ("CPU", CPU, "active.is_some()")] {
        assert!(adapter.contains("mii_exit:None,"), "{name}: STATE starts without a kept publication");
        assert!(adapter.contains("struct MiiExit {mii:BankTrack,configuration:Configuration,published:native::BankPublished,adopted:native::Snapshot}"), "{name}");
        let settle = body(adapter, "unsafe fn bank_settle(");
        // rc.8 §7: the track is borrowed (`&BankTrack`, `as_ref`), and `exit_settled` reads the kept exit itself.
        let kept = settle.find("let published=STATE.lock().bank.as_ref().filter(|t|t.ticket==b.ticket && t.token==b.token).and_then(|t|t.published);")
            .unwrap_or_else(|| panic!("{name}: the publication is read before the settle drops the track:\n{settle}"));
        let settled = settle.find("if b.exit {native::exit_settled(b)} else {native::bank_settled(b)}")
            .unwrap_or_else(|| panic!("{name}:\n{settle}"));
        let record = settle.find("(Some(c),Some((published_c,published))) if published_c==c=>Some(MiiExit {mii:*b,configuration:c,published,adopted:now}),")
            .unwrap_or_else(|| panic!("{name}:\n{settle}"));
        let outcome = settle.find("settle_outcome(Outcome::Applied").unwrap();
        assert!(kept < settled && settled < record && record < outcome, "{name}:\n{settle}");
        assert!(adapter.contains(&format!("let exit={live} && publish.is_none()\n        && STATE.lock().mii_exit.is_some_and(|x|x.mii.ticket.epoch==current.binding.epoch\n            && STATE_OWNER_EXPECTED")
            .replace("STATE_OWNER_EXPECTED", "x.adopted==expected)")), "{name}: the exit is planned only with its record");
        assert!(adapter.contains("if exit {super::body_bank::plan_mii_exit(entry,"), "{name}");
        assert!(adapter.contains("exit:exit && reuse"), "{name}: the BankTrack names the exit");
    }
}

/// The exit settle: the bank's exchange receipt proves the descriptor before anything is written, then the kept
/// publication goes back through `bank_restore` against the receipt from before the Mii, then the owner adopts.
#[test]
fn the_exit_settle_proves_the_exchange_restores_the_publication_then_adopts() {
    for (name, native, adopt, receipt) in [
        ("Player", PLAYER_NATIVE, "owner.expected=now;owner.active=None;", "before:Snapshot {descriptor:now.descriptor,"),
        ("CPU", CPU_NATIVE, "owner.live=bank.destination;owner.expected=now;", "before:Snapshot {descriptor,control:before.control,"),
    ] {
        let exit = body(native, "pub(super) unsafe fn exit_settled(");
        let proof = exit.find("group_rebaseline(").unwrap_or_else(|| panic!("{name}:\n{exit}"));
        // rc.8 §7: the kept exit is read here, once, before the proof, and borrowed for this ticket's epoch.
        let read = exit.find("let kept=super::STATE.lock().mii_exit;").unwrap_or_else(|| panic!("{name}:\n{exit}"));
        assert!(read < proof && exit.contains("let exit=kept.as_ref().filter(|x|x.mii.ticket.epoch==before.binding.epoch)"), "{name}:\n{exit}");
        let restore = exit.find("bank_restore(&restore,exit.configuration,exit.published)").unwrap_or_else(|| panic!("{name}:\n{exit}"));
        let adopted = exit.find(adopt).unwrap_or_else(|| panic!("{name}:\n{exit}"));
        assert!(proof < restore && restore < adopted, "{name}:\n{exit}");
        assert!(exit.contains(receipt), "{name}: the read-back compares the receipt from before the Mii");
        assert!(exit.contains("..exit.mii.before}"), "{name}");
        assert!(exit.contains("state.bank=None;state.mii_exit=None;"), "{name}");
    }
}

/// rc6-hot-xps5 (XPS, diag `dde68833`): the CPU's exit settled `NativeSettledUnowned` with `CPU current index mirrors
/// differ`. Until the restore, `E+0x1a8` names the Mii's destination record while FE and D name the window's index, so
/// an owner walk before `bank_restore` refuses; the descriptor is read directly under G, as `bank_restore` reads.
#[test]
fn the_cpu_exit_reads_no_owner_walk_before_the_restore() {
    let exit = body(CPU_NATIVE, "pub(super) unsafe fn exit_settled(");
    let restore = exit.find("bank_restore(&restore,").expect("the restore");
    assert!(!exit[..restore].contains("current_locked("), "{exit}");
    assert!(exit[..restore].contains("let descriptor=rd::<[u8;0x70]>(before.binding.entry,0);"), "{exit}");
}
