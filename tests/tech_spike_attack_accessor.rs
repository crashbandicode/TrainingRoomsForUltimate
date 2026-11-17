//! Native setter thunks determine which module the diagnostic's guard must inspect.
//! Red runtime evidence: frozen 776a165a receipt #844 (setter_target), not a new red boot.

mod common;

#[path = "../src/runtime/features/tech_direct_v1_guards.rs"]
mod direct_guards;

#[path = "../src/runtime/features/tech_spike_guards.rs"]
mod guards;
#[path = "../src/runtime/features/tech_direct_v3_guards.rs"]
mod v3_guards;

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
fn direct_launcher_does_not_rewrite_cpu_attacks() {
    let source = include_str!("../src/runtime/features/tech_spike.rs");
    let hook = include_str!("../src/runtime/features/hitbox.rs");
    assert!(!source.contains("pub unsafe fn attack("));
    assert!(!source.contains("\"real-hit\" =>"));
    assert!(!hook.contains("tech_spike::attack("));
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

#[test]
fn direct_v3_windows_cover_owned_magnitude_body_and_trig_imports() {
    let Some(owned) =
        common::owned_main("direct_v3_windows_cover_owned_magnitude_body_and_trig_imports")
    else {
        return;
    };
    for &(address, expected) in v3_guards::DIRECT_V3_GUARDS {
        assert_eq!(
            expected,
            owned.window(address),
            "v3 main+{address:x}: all64 bytes"
        );
    }
    // PLT LDR X17 loads the GOT slot scaled by8; pin actual native storage offsets.
    for (address, slot) in [(0x39c07d0, 0x2c8), (0x39c07e0, 0x2d0)] {
        let bytes = owned.window(address);
        let ldr = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(ldr & 0xffc003ff, 0xf9400211);
        assert_eq!(((ldr >> 10) & 0xfff) as usize * 8, slot);
    }
    assert_eq!(v3_guards::MAIN_IMAGE_END, 0x4f17000 + 0x2530100);
}

#[test]
fn motion_getter_virtual_targets_match_owned_code() {
    let Some(owned) = common::owned_main("motion_getter_virtual_targets_match_owned_code") else {
        return;
    };
    for (name, address) in [
        ("motion_speed_virtual_target", 0x3b3540),
        ("kinetic_get_energy_virtual_target", 0x479d40),
    ] {
        assert_eq!(
            *setter_guard(name, address),
            owned.window(address),
            "{name}: all64 guarded bytes match the owned ELF"
        );
    }
    // These are relocated native virtual slots, not the ELF's zero placeholders.
    assert_eq!(relative_pointer(&owned, 0x4f88568 + 0x60), 0x479d40);
    assert_eq!(relative_pointer(&owned, 0x4f888b8 + 0x20), 0x3b3540);
    // The shared native speed getter returns the vector stored at energy+0x10.
    let bytes = owned.window(0x3b3540);
    assert_eq!(
        u32::from_le_bytes(bytes[..4].try_into().unwrap()),
        0x91004000
    );
    assert_eq!(
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        0xd65f03c0
    );
}

fn relative_pointer(owned: &common::OwnedMain, slot: usize) -> usize {
    let b = &owned.bytes;
    let u16_at = |i: usize| u16::from_le_bytes(b[i..i + 2].try_into().unwrap()) as usize;
    let u32_at = |i: usize| u32::from_le_bytes(b[i..i + 4].try_into().unwrap()) as usize;
    let u64_at = |i: usize| u64::from_le_bytes(b[i..i + 8].try_into().unwrap()) as usize;
    let phoff = u64_at(32);
    let phentsize = u16_at(54);
    let phnum = u16_at(56);
    let file_at = |va: usize, size: usize| {
        (0..phnum)
            .find_map(|i| {
                let h = phoff + i * phentsize;
                let start = u64_at(h + 16);
                let end = start.checked_add(u64_at(h + 32))?;
                (u32_at(h) == 1 && va >= start && va.checked_add(size)? <= end)
                    .then(|| u64_at(h + 8) + va - start)
            })
            .unwrap_or_else(|| panic!("unmapped owned VA {va:#x}"))
    };
    let word = |va: usize| u64_at(file_at(va, 8));
    let mod0 = u32_at(file_at(4, 4));
    assert_eq!(&b[file_at(mod0, 4)..file_at(mod0, 4) + 4], b"MOD0");
    let dyn_rel = u32_at(file_at(mod0 + 4, 4)) as u32 as i32;
    let mut at = mod0.checked_add_signed(dyn_rel as isize).unwrap();
    let (mut rela, mut relasz) = (None, None);
    loop {
        let (tag, value) = (word(at), word(at + 8));
        at += 16;
        match tag {
            0 => break,
            7 => rela = Some(value),
            8 => relasz = Some(value),
            _ => {}
        }
    }
    let rela = rela.unwrap();
    let relasz = relasz.unwrap();
    assert_eq!(relasz % 24, 0);
    let mut found = None;
    for at in (rela..rela + relasz).step_by(24) {
        if word(at) == slot {
            assert_eq!(word(at + 8) & 0xffff_ffff, 0x403);
            assert!(
                found.replace(word(at + 16)).is_none(),
                "duplicate slot relocation"
            );
        }
    }
    found.unwrap_or_else(|| panic!("no RELATIVE relocation for {slot:#x}"))
}

#[path = "../src/runtime/features/tech_info_guards.rs"]
mod info_guards;
#[test]
fn damage_info_fields_match_owned_lua_reader_mapping() {
    let Some(owned) = common::owned_main("damage_info_fields_match_owned_lua_reader_mapping")
    else {
        return;
    };
    for &(name, address, expected) in info_guards::DAMAGE_INFO_LAYOUT_GUARDS {
        assert_eq!(expected, owned.window(address), "{name}: all64 owned bytes");
    }
    // Native Lua hash dispatch loads from Info x20. Pin opcode, destination and offset.
    for (address, instruction) in [
        (0x227f1c4, 0xb9800a88u32),
        (0x227f2d0, 0xbd401280),
        (0x227f0a8, 0xbd401680),
        (0x227f2c0, 0xb9404288),
        (0x227f318, 0xb9804e88), // signed Info hitstop_frame at+0x4c
        (0x404940, 0xb9004d69),  // native transactor stores w9 at log+0x4c
    ] {
        let bytes = owned.window(address);
        assert_eq!(
            u32::from_le_bytes(bytes[..4].try_into().unwrap()),
            instruction,
            "mapping instruction {address:x}"
        );
    }
    let bytes = owned.window(0x3ffdc0);
    assert_eq!(
        u32::from_le_bytes(bytes[..4].try_into().unwrap()),
        0x91020000,
        "Info is inline DamageModule+0x80"
    );
    assert_eq!(
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        0xd65f03c0
    );
}
#[path = "../src/runtime/features/tech_stop_guards.rs"]
mod stop_guards;
#[test]
fn damage_stop_guards_match_owned_export_and_concrete_body() {
    let Some(owned) = common::owned_main("damage_stop_guards_match_owned_export_and_concrete_body")
    else {
        return;
    };
    for &(address, expected) in stop_guards::STOP_GUARDS {
        assert_eq!(expected, owned.window(address), "stop guard {address:x}");
    }
    let export = owned.window(0x2089260);
    assert_eq!(native_module_offset(&export), 0x90);
    assert_eq!(
        u32::from_le_bytes(export[8..12].try_into().unwrap()),
        0xf9406903,
        "export dispatches slot0xd0 into x3"
    );
    assert_eq!(
        u32::from_le_bytes(export[12..16].try_into().unwrap()),
        0x12000042,
        "bool argument is w2 bit0"
    );
    let setter = owned.window(0x4df690);
    assert_eq!(
        u32::from_le_bytes(setter[24..28].try_into().unwrap()),
        0x3900f808,
        "concrete setter stores bool at Stop+0x3e"
    );
}
#[test]
fn info_launch_uses_one_pre_init_damage_stop_without_energy_rewrite() {
    let core = include_str!("../src/runtime/features/tech_launch_native.rs");
    let diag = include_str!("../src/runtime/features/tech_spike.rs");
    let product = include_str!("../src/runtime/features/tech_adapter.rs");
    assert!(diag.contains("core::launch(accessor, plan,"));
    assert!(product.contains("tech_launch_native::launch(accessor, plan,"));
    for source in [core, product] {
        assert!(!source.contains("0x20f8320"));
        assert!(!source.contains("0x20f6560"));
    }
    let arm = core
        .find("natives::set_hit_stop_frame(accessor, plan.frames, true)")
        .unwrap();
    let verify = core.find("if !stopped || !damage").unwrap();
    let init = core
        .find("natives::change_status_force(accessor, fly, false)")
        .unwrap();
    assert!(arm < verify && verify < init);
    // Grounded v9 launches must not manufacture a release action or lose GROUND before init.
    assert!(!core.contains("natives::set_int("));
    let ground_precheck = core
        .find("natives::situation_kind(accessor) != ground")
        .unwrap();
    let first_mutation = core
        .find("state_natives::clear_speed_all(accessor)")
        .unwrap();
    assert!(ground_precheck < first_mutation);
    assert!(core.contains(
        "if !plan.ground_to_air {\n        state_natives::set_situation_kind(accessor, air);"
    ));
    let native_action = core
        .find("let readback = natives::get_int(accessor, action)")
        .unwrap();
    let action_check = core
        .find("plan.ground_to_air && readback != ground_to_air")
        .unwrap();
    assert!(init < native_action && native_action < action_check);
    assert!(core.contains("is_damage(accessor)"));
    assert!(diag.contains("fixed9_fixture"));
    assert!(!product.contains("fixed9"));
}

#[path = "../src/runtime/features/tech_reaction_guard.rs"]
mod reaction_guard;
#[test]
fn product_reaction_guard_matches_owned_and_diag_window() {
    let guard = guards::SITES.iter().find(|x| x.1 == 0x404ec0).unwrap();
    assert_eq!(reaction_guard::REACTION_GUARD, guard.2);
    let Some(owned) = common::owned_main("product_reaction_guard_matches_owned_and_diag_window")
    else {
        return;
    };
    assert_eq!(reaction_guard::REACTION_GUARD, owned.window(0x404ec0));
    for &(offset, expected) in reaction_guard::CORE_GUARDS {
        assert_eq!(expected, owned.window(offset));
        if offset != 0x4df440 {
            assert_eq!(
                expected,
                guards::SITES.iter().find(|x| x.1 == offset).unwrap().2
            );
        }
    }
}

#[test]
fn shared_launch_virtual_targets_match_owned_relocations() {
    let Some(owned) = common::owned_main("shared_launch_virtual_targets_match_owned_relocations")
    else {
        return;
    };
    for (slot, target) in [
        (0x4f8a0e0 + 0xa0, 0x4df440),
        (0x4f88568 + 0x60, 0x479d40),
        (0x4f88968 + 0x20, 0x3b3540),
        (0x4f88a18 + 0x20, 0x6d8cc0),
    ] {
        assert_eq!(relative_pointer(&owned, slot), target);
        assert!(reaction_guard::CORE_GUARDS.iter().any(|x| x.0 == target));
    }
}
