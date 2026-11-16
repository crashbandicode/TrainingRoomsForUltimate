//! Native setter thunks determine which module the diagnostic's guard must inspect.
//! Red runtime evidence: frozen 776a165a receipt #844 (setter_target), not a new red boot.

mod common;

#[path = "../src/runtime/features/tech_direct_v1_guards.rs"]
mod direct_guards;
#[path = "../src/runtime/features/tech_spike_guards.rs"]
mod guards;

const SETTERS: [(&str, usize); 5] = [
    ("realhit_set_power", 0x1fd0270),
    ("realhit_set_vector", 0x1fd02b0),
    ("realhit_set_reaction_effect", 0x1fd03c0),
    ("realhit_set_reaction_fix", 0x1fd0400),
    ("realhit_set_reaction_add", 0x1fd0440),
];

fn setter_guard(name: &str, offset: usize) -> &[u8; 64] {
    let matches: Vec<_> = guards::SITES.iter().filter(|site| site.0 == name).collect();
    assert_eq!(matches.len(), 1, "one guarded native setter named {name}");
    assert_eq!(matches[0].1, offset, "{name}: owned setter export address");
    &matches[0].2
}

fn native_module_offset(bytes: &[u8; 64]) -> usize {
    let instruction = u32::from_le_bytes(bytes[..4].try_into().unwrap());
    // AArch64 unsigned scaled-immediate LDR X0,[X0,#imm12*8]. Keep opcode and both registers.
    assert_eq!(
        instruction & 0xffc0_03ff,
        0xf940_0000,
        "native setter must load its module from BOMA x0"
    );
    ((instruction >> 10) & 0xfff) as usize * 8
}

#[test]
fn attack_guard_inspects_the_module_loaded_by_all_native_setters() {
    let source = include_str!("../src/runtime/features/tech_spike.rs");
    let attack = source
        .split_once("pub unsafe fn attack(")
        .expect("diagnostic ATTACK callback")
        .1;
    let statement = attack
        .split_once("let module =")
        .expect("CPU-hit guard module load")
        .1
        .split(';')
        .next()
        .unwrap();
    let compact: String = statement.chars().filter(|c| !c.is_whitespace()).collect();
    let offset = compact
        .split_once("accessor+")
        .expect("module load is BOMA-relative")
        .1;
    let literal = offset
        .split(')')
        .next()
        .unwrap()
        .strip_prefix("0x")
        .expect("explicit module offset");
    let source_offset = usize::from_str_radix(literal, 16).expect("hex module offset");
    for (name, address) in SETTERS {
        let native_offset = native_module_offset(setter_guard(name, address));
        assert_eq!(native_offset, 0xa0, "{name}: owned AttackModule layout");
        assert_eq!(
            source_offset, native_offset,
            "{name}: guard must inspect the native AttackModule, not WorkModule"
        );
    }
}

#[test]
fn attack_setter_guards_match_the_owned_native_export_windows() {
    let Some(owned) =
        common::owned_main("attack_setter_guards_match_the_owned_native_export_windows")
    else {
        return;
    };
    for (name, address) in SETTERS {
        let guard = setter_guard(name, address);
        assert_eq!(
            *guard,
            owned.window(address),
            "{name}: all64 guarded bytes match the owned ELF"
        );
        assert_eq!(
            native_module_offset(guard),
            0xa0,
            "{name}: owned LDR X0,[X0,#0xa0]"
        );
    }
}

#[test]
fn direct_v1_guards_match_owned_code_and_cover_the_virtual_targets() {
    let Some(owned) =
        common::owned_main("direct_v1_guards_match_owned_code_and_cover_the_virtual_targets")
    else {
        return;
    };
    for &(address, expected) in direct_guards::DIRECT_V1_GUARDS {
        assert_eq!(
            expected,
            owned.window(address),
            "direct-v1 main+{address:x}: all64 bytes"
        );
    }
    for &(_, _, targets) in direct_guards::DIRECT_V1_VIRTUAL_TARGETS {
        for &target in targets {
            assert!(
                direct_guards::DIRECT_V1_GUARDS
                    .iter()
                    .any(|&(address, _)| address == target),
                "virtual target main+{target:x} has its own guard"
            );
        }
    }
}
