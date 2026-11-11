//! stream/stack: every thread's stack comes from one table, and the table covers what each thread was measured to need.
//! The Eden s4-07 / u4b-02 fault (`tr-trace-writer`, a 59,712 B frame on a 65,536 B stack) fails `rows_cover_their_need`
//! with that row's numbers (see `the_fault_s_numbers_fail_the_rule`).

// rc.9 test cut: the by-path `src` files' own unit tests run once, in the lib's test binary (`src/lib.rs`), not again here.
macro_rules! lib_unit_tests {
    ($($body:tt)*) => {};
}
#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::hook_stats::{hook_allocations, CountingAllocator, HookId, HookScope};
// The slot is an engine module; it reaches crate::model::store, which the module above provides.
#[path = "../src/model/lobby/trace_slot.rs"]
#[allow(dead_code)]
mod trace_slot;

use trace_slot::{flush_boxed, BoxedSlot};
use model::thread_stacks::{self, ThreadId, PAINT, SDK_RESERVE, THREADS};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[test]
fn rows_are_in_id_order_with_unique_names() {
    for (i, row) in THREADS.iter().enumerate() {
        assert_eq!(row.id as usize, i, "{} is out of order", row.name);
        assert!(!row.name.is_empty() && row.name.len() <= 31, "{}", row.name);
        assert!(!row.symbol.is_empty(), "{}", row.name);
    }
    let mut names: Vec<_> = THREADS.iter().map(|r| r.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), THREADS.len());
}

#[test]
fn rows_cover_their_need() {
    for row in &THREADS {
        assert_eq!(row.bytes % 4096, 0, "{}: whole pages", row.name);
        let need = row.required();
        assert!(
            row.bytes >= need,
            "{}: {} B stack, needs {} B (static {} B, external {} B, measured {} B, +25% + {} B SDK reserve)",
            row.name, row.bytes, need, row.static_bound, row.external_bound, row.measured, SDK_RESERVE
        );
        assert_eq!(row.measured == 0, row.measured_in.is_empty(), "{}: a measurement names its runs", row.name);
    }
}

#[test]
fn the_service_stacks_cover_the_native_service() {
    // rc.7 FLAG 6: the plugin's part of the native service runs on its own stacks, and what stays on the game's 64 KiB
    // worker stack fits the room measured there (`tools/stack_depth.py <elf> table` checks the same row against the ELF).
    let row = thread_stacks::SERVICE_STACK;
    assert!(row.covers(), "{row:?}");
    assert!(row.count >= 2, "a nested dispatch needs a second stack");
    assert_eq!(row.on_stack, &["training_mod_native_service_pre"], "only the poll; the gates and leaves need no claim");
}

#[test]
fn the_native_service_fault_s_numbers_fail_the_rule() {
    // rc.7 FLAG 6, Eden final-surface-acc-d8679593-r2 `acc-rename`: everything ran on the caller's stack, a 64 KiB stack in a
    // heap block with 63,728 B free at the dispatch's entry. The entry's deepest static path was 236,608 B (diag ELF of 0d742313), and
    // `sources::service` alone had a 77,808 B frame: an Unmapped Write64 13.9 KB under that stack.
    let before = thread_stacks::CallbackStack { native_bound: 236_608, ..thread_stacks::SERVICE_STACK };
    assert!(!before.covers());
    assert!(thread_stacks::required(77_808, 0) > before.native_room, "sources::service's frame alone");
}

#[test]
fn a_service_stack_is_claimed_lowest_first_and_never_twice() {
    use thread_stacks::free_stack;
    assert_eq!(free_stack(0b0000, 4), Some(0));
    assert_eq!(free_stack(0b0101, 4), Some(1));
    assert_eq!(free_stack(0b0111, 4), Some(3));
    assert_eq!(free_stack(0b1111, 4), None, "every stack in use: the dispatch skips this poll");
    assert_eq!(free_stack(0b0111, 3), None);
    // A release, then a claim: the lowest stack free again.
    let held = 0b1111u32 & !(1 << 1);
    assert_eq!(free_stack(held, 4), Some(1));
}

#[test]
fn plugin_threads_keep_off_the_main_threads_core() {
    use thread_stacks::{plugin_core_mask, MAIN_CORE};
    // An application allowed cores 0-2 (or 0-3): the plugin threads get 1-2 (1-3), ideal core 1.
    assert_eq!(plugin_core_mask(0b0111, MAIN_CORE), Some((1, 0b0110)));
    assert_eq!(plugin_core_mask(0b1111, MAIN_CORE), Some((1, 0b1110)));
    // Never core 0, never a core the process may not use.
    for available in 0..16u64 {
        if let Some((ideal, mask)) = plugin_core_mask(available, MAIN_CORE) {
            assert_eq!(mask & 1, 0);
            assert_eq!(mask & !available, 0);
            assert_ne!(mask & (1 << ideal), 0);
        }
    }
    // Only the MainThread's core available: the SDK default stays (the creator's core).
    assert_eq!(plugin_core_mask(0b0001, MAIN_CORE), None);
    // A MainThread found elsewhere is left alone too.
    assert_eq!(plugin_core_mask(0b0111, 1), Some((2, 0b0100)));
}

#[test]
fn the_fault_s_numbers_fail_the_rule() {
    // tr-trace-writer before the fix: thread_start 112 + shim 16 + body 59,712 B (diag ELF of 27d84f4) on 64 KiB.
    assert!(thread_stacks::required(59_840, 0) > 64 * 1024);
}

#[test]
fn the_combo_crop_fault_s_numbers_fail_the_rule() {
    // rc.6 final-surface-cov-4e336da4, cov-crop (Recording > Combos & Sequences > Combo 1 > Crop): tr-menu's last paint mark was
    // 54,960 B of its 56 KiB; its SDK frame then stored 0x130 B below the stack's base, so it needed at least 57,648 B. Its
    // deepest static path was 53,344 B on 4e336da4 (50,208 B before and after the whole-Sample copy `strip_sample` made), against
    // a stale table row of 32,432 B static, 30,848 B measured.
    assert!(thread_stacks::required(53_344, 57_648) > 56 * 1024);
    assert!(thread_stacks::required(50_208, 0) > 56 * 1024, "the static path alone already needed more than 56 KiB");
    assert!(thread_stacks::required(32_432, 30_848) <= 56 * 1024, "the stale row is why the rule passed");
    let menu = thread_stacks::of(ThreadId::Menu);
    assert!(menu.static_bound >= 50_208 && menu.measured >= 57_648, "tr-menu's row lost the crop's numbers: {menu:?}");
    assert_eq!(menu.bytes, thread_stacks::required(50_208, 57_648), "88 KiB: the rule for the overflow's 57,648 B");
}

#[test]
fn probe_accounts_for_the_actual_external_read_path_even_when_paint_misses_it() {
    // ui-16's 128 KiB subtraction jumped below the 24 KiB painted allocation.
    // Addressed ABI frames: adapter64 + read320 + parent10368 + decoder131904
    // + nested720. The owned scanner's 3440 remains a separate contribution.
    let mut probe = *thread_stacks::of(ThreadId::Probe);
    assert_eq!(probe.static_bound, 3_440);
    assert_eq!(probe.external_bound, 64 + 320 + 10_368 + 131_904 + 720);
    assert!(thread_stacks::required(probe.static_bound, 0) <= 24 * 1024,
        "the old owned-only rule admitted the overflowing allocation");
    // Reconstruct the earlier static-only contract separately from fresh ui-17 paint.
    probe.measured = 0;
    probe.bytes = 200_704;
    assert_eq!(probe.required(), 200_704);
    probe.measured = 18_560; // The misleading in-allocation ui-16 paint result.
    assert!(24 * 1024 < probe.required());
    assert_eq!(probe.required(), 200_704);
    assert_eq!(probe.bytes, probe.required());
    probe.external_bound += 4096;
    assert!(probe.required() > probe.bytes, "a growing external path must fail again");
    for row in &THREADS {
        if row.id != ThreadId::Probe { assert_eq!(row.external_bound, 0); }
    }
}

#[test]
fn the_spec_constants_are_the_table_s() {
    assert_eq!(thread_stacks::of(ThreadId::IniWriter).bytes, model::settings::WRITER_STACK_BYTES);
    assert_eq!(thread_stacks::of(ThreadId::SettingsLoad).bytes, model::settings::LOADER_STACK_BYTES);
}

#[test]
fn high_water_arithmetic() {
    let mut words = vec![PAINT; 64];
    words[40] = 0;
    assert_eq!(thread_stacks::untouched(&words), 320);
    assert_eq!(thread_stacks::used(512, 320), 192);
    assert_eq!(thread_stacks::untouched(&[0, PAINT]), 0);
    assert!(thread_stacks::low(64 * 1024, 64 * 1024 - SDK_RESERVE + 1));
    assert!(!thread_stacks::low(64 * 1024, 64 * 1024 - SDK_RESERVE));
}

/// Every Rust source file under `dir`.
fn sources(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn the_runtime_starts_threads_only_through_the_table() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime");
    let mut files = Vec::new();
    sources(&root, &mut files);
    let mut used = vec![false; THREADS.len()];
    for file in &files {
        let text = std::fs::read_to_string(file).unwrap();
        let own = file.ends_with("runtime/threads.rs");
        for (n, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if !own {
                for banned in ["stack_size(", "thread::Builder", "thread::spawn("] {
                    assert!(!code.contains(banned), "{}:{}: `{banned}` outside runtime::threads", file.display(), n + 1);
                }
            }
        }
        for row in &THREADS {
            let call = if thread_stacks::joined(row.id) { "run_joined" } else { "spawn" };
            if text.contains(&format!("threads::{call}(crate::model::thread_stacks::ThreadId::{:?},", row.id))
                || text.contains(&format!("threads::{call}(ThreadId::{:?},", row.id))
            {
                used[row.id as usize] = true;
            }
        }
    }
    for row in &THREADS {
        assert!(used[row.id as usize], "{} has no spawn site of its kind (joined: run_joined, else spawn)", row.name);
    }
}

/// The static stacks of `runtime::threads` (`slots! { NAME = [Id, …]; … }`): every thread has exactly one, only the boot
/// loaders share one, and the sizes are the table's (`slot_bytes` of the row ids, the only size expression in the macro).
#[test]
fn every_thread_has_one_static_stack_from_the_table() {
    let text = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime/threads.rs")).unwrap();
    let start = text.find("    slots! {").expect("slots! block");
    let block = &text[start..start + text[start..].find("\n    }\n").expect("end of slots!")];
    let mut seen = vec![0; THREADS.len()];
    let mut shared = Vec::new();
    for line in block.lines().skip(1) {
        let Some((_, ids)) = line.split_once("= [") else { continue };
        let ids: Vec<&str> = ids.trim_end_matches("];").split(',').map(str::trim).collect();
        for id in &ids {
            let row = THREADS.iter().find(|r| format!("{:?}", r.id) == *id).unwrap_or_else(|| panic!("unknown thread {id}"));
            seen[row.id as usize] += 1;
        }
        if ids.len() > 1 {
            shared.push(ids.join(","));
        }
    }
    for row in &THREADS {
        assert_eq!(seen[row.id as usize], 1, "{}: {} static stacks", row.name, seen[row.id as usize]);
    }
    // Only the boot stack is shared: the joined loaders, one after another, then its one detached last user.
    let boot: Vec<String> = thread_stacks::BOOT_STACK.iter().map(|id| format!("{id:?}")).collect();
    assert_eq!(shared, [boot.join(",")], "only the boot stack is shared");
    let (last, loaders) = thread_stacks::BOOT_STACK.split_last().unwrap();
    assert_eq!(*last, thread_stacks::BOOT_STACK_LAST);
    assert!(!thread_stacks::joined(*last), "the last user is the one detached thread on the boot stack");
    assert_eq!(loaders, thread_stacks::BOOT_LOADERS);
    for id in thread_stacks::BOOT_LOADERS {
        assert!(thread_stacks::joined(id), "{id:?} shares a stack before the last user, so it must be joined");
    }
    assert!(text.contains("Slot<{ thread_stacks::slot_bytes(&[$(ThreadId::$sid),+]) }>"), "the shared slot's size comes from the table");
    assert!(text.contains("Slot<{ thread_stacks::of(ThreadId::$id).bytes }>"), "each slot's size comes from the table");
    // Names are inline consts per arm: only the threads a build has put their names into it.
    assert!(text.contains("const { thread_stacks::of(ThreadId::$id).c_name }"));
    // The diag-only and feature-only threads have their stacks only in those builds.
    for (slot, gate) in [("STATE_DIAG", "diag"), ("DIAG_RELOAD", "diag"), ("PROBE", "diag"), ("MACRO_DIAG", "diag"),
        ("PRACTICE_TRACE", "console-practice-trace"), ("STALL_WATCH", "diag")]
    {
        assert!(block.contains(&format!("#[cfg(feature = \"{gate}\")] {slot} = [")), "{slot} must be behind `{gate}`");
    }
    assert_eq!(
        thread_stacks::slot_bytes(&thread_stacks::BOOT_STACK),
        thread_stacks::BOOT_STACK.iter().map(|id| thread_stacks::of(*id).bytes).max().unwrap()
    );
}

/// The boot stack's last user starts after the install line (boot's last step), so no joined loader can find it busy.
#[test]
fn the_captures_start_after_the_install_line() {
    let boot = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/runtime/boot.rs")).unwrap();
    let write = boot.rfind("install::write(&line);").expect("install line");
    let captures = boot.rfind("import::start_background(").expect("captures start");
    assert!(captures > write, "the captures start after the install line");
}

#[test]
fn names_are_the_c_names() {
    for row in &THREADS {
        assert_eq!(row.c_name.to_str().unwrap(), row.name);
    }
}

#[derive(Clone, Copy)]
struct Big {
    n: u32,
    pad: [u64; 1400],
}

#[test]
fn a_prepared_boxed_slot_allocates_nothing() {
    let slot = BoxedSlot::new();
    slot.prepare(&Big { n: 0, pad: [0; 1400] });
    let value = Big { n: 5, pad: [5; 1400] };
    let before = hook_allocations();
    {
        let _scope = HookScope::enter_hook(HookId::UiArchive);
        for _ in 0..3 {
            assert!(slot.put(&value));
            let taken = slot.take().unwrap();
            assert_eq!(taken.n, 5);
            slot.finish(taken, false);
            assert!(flush_boxed(&slot, |v| if v.pad[1399] == 5 { Ok(()) } else { Err(std::io::Error::other("bad")) }).unwrap());
        }
    }
    assert_eq!(hook_allocations(), before, "put, take, finish and flush_boxed moved buffers only");
}

#[test]
fn completed_arc_probe_high_water_keeps_the_full_margin_policy() {
    let probe = thread_stacks::of(ThreadId::Probe);
    assert_eq!(probe.measured, 151_920);
    assert!(probe.measured_in.contains("ui-17"));
    assert_eq!(probe.required(), 208_896);
    assert!(200_704 < probe.required(), "the old allocation had SDK room but missed the quarter margin");
    assert_eq!(probe.bytes, probe.required());
}
