//! The native pause owner's two sites (`model::sites_pause`, review R2 of package 1), checked against the
//! owned 13.0.5 ELF: the guards are the file's bytes; the chain from the state name `StatePause` to the
//! two hooked instructions is the one the module comment states (name -> factory -> vtable -> enter and
//! exit -> the pause flags the enter raises and the exit clears); no site or guard window is near a site
//! of the CPU pilot plugin or overlaps another row of the plugin.

mod common;

#[path = "../src/model.rs"]
#[allow(dead_code)]
mod model;

use model::sites::{self, Feature};
use model::sites_pause::{self, PAUSE_ENTER_OFFSET, PAUSE_EXIT_OFFSET};

/// Offsets of every site of `ssbu-cpu-pilot` (the list `tests/cpu_sites.rs` carries).
const PILOT_SITES: [usize; 23] = [
    0x1750df0, 0x1751ce8, 0x2da180, 0x6b8400, 0x2d8d40, 0x375ef8c, 0x59970, 0x37a2270, 0x376309c, 0x1d59560,
    0x147fb64, 0x150ea38, 0x1d59dc4, 0x1d5b5a8, 0x1d5b650, 0x1d5a590, 0x1d5a840, 0x32f22c4, 0x376db18, 0x32f1630,
    0x30c770, 0x3119d0, 0x1671650,
];

const SCENE_REGISTRATION: usize = 0x14d_1ec0;
const FACTORY: usize = 0x14d_3180;
const VTABLE: usize = 0x506_bd18;
const ENTER_FN: usize = 0x14d_47b0;
const EXIT_FN: usize = 0x14d_47d0;
const PAUSE_BEGIN: usize = 0x150_f0c0;

#[test]
fn the_rows_are_the_two_pause_sites_and_clear_of_everything_else() {
    assert_eq!(sites_pause::SITES.len(), 2);
    assert_eq!(sites_pause::find("pause_enter").unwrap().offset, PAUSE_ENTER_OFFSET);
    assert_eq!(sites_pause::find("pause_exit").unwrap().offset, PAUSE_EXIT_OFFSET);
    assert_eq!(sites_pause::sites_of(Feature::PauseOwner).count(), 2);
    assert_eq!(Feature::PauseOwner.name(), "pause_owner");
    for s in &sites_pause::SITES {
        assert!(s.compared() && s.guard.iter().any(|b| *b != 0), "{}", s.name);
        // The 64-byte windows hold no pilot site and sit 0x40 clear of each.
        for pilot in PILOT_SITES {
            let (lo, hi) = (s.offset.saturating_sub(0x40), s.window_end() + 0x40);
            assert!(!(lo..hi).contains(&pilot), "{} is within 0x40 of the pilot site {pilot:#x}", s.name);
        }
        // No overlap with any other row of this plugin, nor with the other pause row.
        for other in sites::SITES.iter().chain(sites_pause::SITES.iter()).filter(|o| o.name != s.name) {
            assert!(!s.overlaps(other), "{} overlaps {}", s.name, other.name);
        }
        // Row names do not collide with the main table's (the install line keys on them).
        assert!(sites::find(s.name).is_none(), "{}", s.name);
    }
}

fn file_offset(main: &common::OwnedMain, addr: usize) -> usize {
    let section = [main.text, main.rodata, main.data]
        .into_iter()
        .find(|s| addr >= s.addr && addr < s.addr + s.size)
        .unwrap_or_else(|| panic!("{addr:#x} is in no section"));
    section.offset + (addr - section.addr)
}

fn u32_at(main: &common::OwnedMain, addr: usize) -> u32 {
    let at = file_offset(main, addr);
    u32::from_le_bytes(main.bytes[at..at + 4].try_into().unwrap())
}

fn u64_at(main: &common::OwnedMain, addr: usize) -> u64 {
    let at = file_offset(main, addr);
    u64::from_le_bytes(main.bytes[at..at + 8].try_into().unwrap())
}

/// The target of `bl`/`b` at `at`.
fn branch_target(at: usize, word: u32) -> usize {
    let imm = (((word & 0x03ff_ffff) << 6) as i32 >> 6) as i64;
    (at as i64 + imm * 4) as usize
}

/// The address `adrp` + `add` (64-bit, no shift) at `at` and `at + 4` compute.
fn adrp_add(main: &common::OwnedMain, at: usize) -> usize {
    let (adrp, add) = (u32_at(main, at), u32_at(main, at + 4));
    assert_eq!(adrp & 0x9f00_0000, 0x9000_0000, "adrp at {at:#x}");
    assert_eq!(add & 0xffc0_0000, 0x9100_0000, "add at {}", at + 4);
    let imm = (((adrp >> 5) & 0x7_ffff) << 2) | ((adrp >> 29) & 3);
    let imm = ((imm << 11) as i32 >> 11) as i64;
    (((at & !0xfff) as i64 + (imm << 12)) as usize) + ((add >> 10) & 0xfff) as usize
}

/// The `R_AARCH64_RELATIVE` relocations of the NSO (what its loader writes into `.data` pointers): the
/// ELF keeps the table, not the result. Returns `(slot address, value)`.
fn relative_relocations(main: &common::OwnedMain) -> std::collections::HashMap<usize, usize> {
    // MOD0 sits at the offset in the second word of `.text`; its first field is the dynamic section's
    // offset from the MOD0 header.
    let mod0 = u32_at(main, 4) as usize;
    assert_eq!(&main.bytes[file_offset(main, mod0)..file_offset(main, mod0) + 4], b"MOD0");
    let dynamic = (mod0 as i64 + i64::from(u32_at(main, mod0 + 4) as i32)) as usize;
    let (mut rela, mut size) = (0usize, 0usize);
    for i in 0.. {
        let (tag, value) = (u64_at(main, dynamic + 16 * i), u64_at(main, dynamic + 16 * i + 8));
        match tag {
            0 => break,
            7 => rela = value as usize,
            8 => size = value as usize,
            _ => {}
        }
    }
    assert!(rela != 0 && size % 24 == 0);
    (0..size / 24)
        .filter_map(|i| {
            let at = rela + 24 * i;
            (u64_at(main, at + 8) & 0xffff_ffff == 1027).then(|| (u64_at(main, at) as usize, u64_at(main, at + 16) as usize))
        })
        .collect()
}

#[test]
fn statepause_chain_from_the_state_name_to_the_two_hooked_instructions() {
    let Some(main) = common::owned_main("statepause_chain_from_the_state_name_to_the_two_hooked_instructions") else {
        return;
    };
    for s in &sites_pause::SITES {
        assert_eq!(main.window(s.offset), s.guard, "{}", s.name);
    }

    // The Training scene's state registration (it also registers `StateResetTraining`) names `StatePause`
    // and registers the factory next to it.
    let string_at = |addr: usize| {
        let at = file_offset(&main, addr);
        let end = main.bytes[at..].iter().position(|b| *b == 0).unwrap();
        String::from_utf8_lossy(&main.bytes[at..at + end]).into_owned()
    };
    let (mut named, mut reset_training, mut factory) = (false, false, 0);
    for at in (SCENE_REGISTRATION..SCENE_REGISTRATION + 0x600).step_by(4) {
        let word = u32_at(&main, at);
        if word & 0x9f00_0000 == 0x9000_0000 && u32_at(&main, at + 4) & 0xffc0_0000 == 0x9100_0000 {
            let target = adrp_add(&main, at);
            if target >= main.rodata.addr && target < main.rodata.addr + main.rodata.size {
                match string_at(target).as_str() {
                    "StatePause" => {
                        named = true;
                        factory = adrp_add(&main, at + 8);
                    }
                    "StateResetTraining" => reset_training = true,
                    _ => {}
                }
            }
        }
    }
    assert!(named && reset_training, "the Training scene's registration names StatePause");
    assert_eq!(factory, FACTORY);

    // The factory builds the object with the vtable at VTABLE (adrp x9 / add x9 near its end).
    assert_eq!(adrp_add(&main, FACTORY + 0x58), VTABLE);

    // The vtable's slots 2 and 3 (after the offset-to-top and type-info words) are the enter and the exit.
    let relocated = relative_relocations(&main);
    assert_eq!(relocated.get(&(VTABLE + 8 * 2)), Some(&ENTER_FN));
    assert_eq!(relocated.get(&(VTABLE + 8 * 3)), Some(&EXIT_FN));

    // The enter: stp / mov x29 / adrp+ldr (the pause manager) / `ldr x0, [x8]` (the hook site) / `bl`
    // to the pause-begin routine / `ldp x29, x30` / ret.
    assert_eq!(u32_at(&main, ENTER_FN), 0xa9bf_7bfd, "stp x29, x30, [sp, #-0x10]!");
    assert_eq!(u32_at(&main, PAUSE_ENTER_OFFSET), 0xf940_0100, "ldr x0, [x8]");
    assert_eq!(branch_target(PAUSE_ENTER_OFFSET + 4, u32_at(&main, PAUSE_ENTER_OFFSET + 4)), PAUSE_BEGIN);
    assert_eq!(u32_at(&main, PAUSE_ENTER_OFFSET + 4) >> 26, 0b100101, "bl");
    assert_eq!(u32_at(&main, PAUSE_ENTER_OFFSET + 8), 0xa8c1_7bfd, "ldp x29, x30, [sp], #0x10");
    assert_eq!(u32_at(&main, PAUSE_ENTER_OFFSET + 12), 0xd65f_03c0, "ret");

    // The begin routine raises the pause flags (value 1 in w9) the exit clears (wzr), the same stores
    // through the same registers and offsets: +0xa9, the byte at +0x568e, and +0x2d.
    assert_eq!(u32_at(&main, PAUSE_BEGIN + 0xe0), 0x3200_03e9, "begin: orr w9, wzr, #1 (value 1)");
    for (begin, end, mask) in [(0x150_f1a4, 0x14d_4814, !0x1f), (0x150_f1bc, 0x14d_482c, 0xffe0_fc00u32), (0x150_f1cc, 0x14d_483c, !0x1f)] {
        let (set, clear) = (u32_at(&main, begin), u32_at(&main, end));
        // The `+0x568e` store is `strb w, [xBase, xOffset]`: the two functions use different registers
        // for the base and the offset, so only its opcode class is compared (and the constant below).
        assert_eq!(set & mask & !0x1f, clear & mask & !0x1f, "the same store at {begin:#x} and {end:#x}");
        assert_eq!(set & 0x1f, 9, "the begin stores w9 (1)");
        assert_eq!(clear & 0x1f, 31, "the exit stores wzr (0)");
    }
    // The register offset of that store is the constant 0x568e in both (`mov w11/w10, #0x568e`).
    assert_eq!(u32_at(&main, 0x150_f1b8) & !0x1f, 0x528a_d1c0);
    assert_eq!(u32_at(&main, 0x14d_4828) & !0x1f, 0x528a_d1c0);

    // Every path of the exit function ends in the epilogue the exit hook sits on: it is the function's
    // only `ret`.
    assert_eq!(u32_at(&main, PAUSE_EXIT_OFFSET), 0xa8c1_7bfd, "ldp x29, x30, [sp], #0x10");
    assert_eq!(u32_at(&main, PAUSE_EXIT_OFFSET + 4), 0xd65f_03c0, "ret");
    let rets: Vec<usize> = (EXIT_FN..PAUSE_EXIT_OFFSET + 8).step_by(4).filter(|a| u32_at(&main, *a) == 0xd65f_03c0).collect();
    assert_eq!(rets, [PAUSE_EXIT_OFFSET + 4]);
    // The exit function has no early return: every unconditional `b` inside it stays within it and the
    // cold block that follows its epilogue, so every path reaches the hooked epilogue.
    for at in (EXIT_FN..PAUSE_EXIT_OFFSET).step_by(4) {
        let word = u32_at(&main, at);
        if word >> 26 == 0b000101 {
            let target = branch_target(at, word);
            assert!((EXIT_FN..=PAUSE_EXIT_OFFSET + 0x10).contains(&target), "branch at {at:#x} leaves the function");
        }
    }
    // The pause-begin routine is called from four places caller of the pause-begin routine in the Training scene's StatePause: the enter's call is
    // one of the four sites that call it, and the hook is on this one.
    let callers: Vec<usize> = (0..main.text.size).step_by(4).filter(|&at| u32_at(&main, at) >> 26 == 0b100101 && branch_target(at, u32_at(&main, at)) == PAUSE_BEGIN).collect();
    assert_eq!(callers.len(), 4);
    assert!(callers.contains(&(PAUSE_ENTER_OFFSET + 4)));
}
