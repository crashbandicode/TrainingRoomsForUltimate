#!/usr/bin/env python3
"""Write `src/model/sites_pkg6.rs` from the owned 13.0.5 main ELF.

    python3 -I tools/gen_sites_pkg6.py <elf> <out.rs>

The table below is the source of truth for names, features, kinds and notes; the guard bytes come from
the ELF. `tests/input_sites.rs` re-checks every row against the same ELF on each `cargo test`.
"""
import struct
import sys

ROWS = [
    ("p1_attack_is_infliction", 0x1FD07D0, "P1Cpu", "Called", "AttackModule::is_infliction_status (lua_bind thunk, symbol _ZN3app8lua_bind39AttackModule__is_infliction_status_implEPNS_26BattleObjectModuleAccessorEj): whether the P1 body's attack connected; the CPU-as-P1 fallback counts completed attacks with it."),
    ("p1_manager_entry_count", 0x2140D60, "P1Cpu", "Called", "FighterManager::entry_count (lua_bind thunk, symbol _ZN3app8lua_bind32FighterManager__entry_count_implEPNS_14FighterManagerE): the number of fighter entries, to find the CPU opponent."),
    ("p1_manager_is_result_mode", 0x2141620, "P1Cpu", "Called", "FighterManager::is_result_mode (lua_bind thunk, symbol _ZN3app8lua_bind35FighterManager__is_result_mode_implEPNS_14FighterManagerE): stands the fallback down on the results screen."),
    ("p1_manager_is_ready_go", 0x21411F0, "P1Cpu", "Called", "FighterManager::is_ready_go (lua_bind thunk, symbol _ZN3app8lua_bind32FighterManager__is_ready_go_implEPNS_14FighterManagerE): the fallback arms only after READY GO."),
    ("p1_entry_current_fighter_id", 0x20CBCB0, "P1Cpu", "Called", "FighterEntry::current_fighter_id (lua_bind thunk, symbol _ZN3app8lua_bind37FighterEntry__current_fighter_id_implEPNS_12FighterEntryE): the battle-object id of an entry, which `sv_battle_object::module_accessor` (guarded by package 3's `state_object_module_accessor`) turns into the accessor."),
    ("roster_have_item_kind", 0x2098CD0, "Roster", "Called", "ItemModule::get_have_item_kind (lua_bind thunk, symbol _ZN3app8lua_bind35ItemModule__get_have_item_kind_implEPNS_26BattleObjectModuleAccessorEi): the kind of the item a fighter holds; the turnip identity read and the item cycle use it."),
    ("roster_posture_pos", 0x206ADE0, "Roster", "Called", "PostureModule::pos (lua_bind thunk, symbol _ZN3app8lua_bind23PostureModule__pos_implEPNS_26BattleObjectModuleAccessorE): the body position the replacement fighter is placed at."),
    ("roster_find_battle_object", 0x1FCA0B0, "Roster", "Called", "BattleObjectManager::is_active_find_battle_object (lua_bind thunk, symbol _ZN3app8lua_bind54BattleObjectManager__is_active_find_battle_object_implEPNS_19BattleObjectManagerEj): the battle object of an id (the Kirby copy cleanup takes the object itself); the manager is the singleton slot at main+0x52b7a00."),
    ("roster_status_is_changing", 0x2087760, "Roster", "Called", "StatusModule::is_changing (lua_bind thunk, symbol _ZN3app8lua_bind30StatusModule__is_changing_implEPNS_26BattleObjectModuleAccessorE): whether the body is between two statuses (the Kirby hat step waits)."),
    ("probe_attack_data", 0x1FD0520, "Probes", "Called", "AttackModule::attack_data (lua_bind thunk, symbol _ZN3app8lua_bind30AttackModule__attack_data_implEPNS_26BattleObjectModuleAccessorEib): the source-owner probe reads an article's attack data (diag only)."),
    ("probe_attack_is_attack", 0x1FD01C0, "Probes", "Called", "AttackModule::is_attack (lua_bind thunk, symbol _ZN3app8lua_bind28AttackModule__is_attack_implEPNS_26BattleObjectModuleAccessorEib): source-owner probe (diag only)."),
    ("probe_attack_power_add_status", 0x1FD0570, "Probes", "Called", "AttackModule::power_add_status (lua_bind thunk, symbol _ZN3app8lua_bind35AttackModule__power_add_status_implEPNS_26BattleObjectModuleAccessorE): source-owner probe (diag only)."),
    ("probe_attack_power_mul", 0x1FD05D0, "Probes", "Called", "AttackModule::power_mul (lua_bind thunk, symbol _ZN3app8lua_bind28AttackModule__power_mul_implEPNS_26BattleObjectModuleAccessorE): source-owner probe (diag only)."),
    ("probe_attack_power_mul_2nd", 0x1FD05F0, "Probes", "Called", "AttackModule::power_mul_2nd (lua_bind thunk, symbol _ZN3app8lua_bind32AttackModule__power_mul_2nd_implEPNS_26BattleObjectModuleAccessorE): source-owner probe (diag only)."),
    ("probe_attack_power_mul_3rd", 0x1FD0610, "Probes", "Called", "AttackModule::power_mul_3rd (lua_bind thunk, symbol _ZN3app8lua_bind32AttackModule__power_mul_3rd_implEPNS_26BattleObjectModuleAccessorE): source-owner probe (diag only)."),
    ("probe_attack_power_mul_4th", 0x1FD0630, "Probes", "Called", "AttackModule::power_mul_4th (lua_bind thunk, symbol _ZN3app8lua_bind32AttackModule__power_mul_4th_implEPNS_26BattleObjectModuleAccessorE): source-owner probe (diag only)."),
    ("probe_attack_power_mul_5th", 0x1FD0650, "Probes", "Called", "AttackModule::power_mul_5th (lua_bind thunk, symbol _ZN3app8lua_bind32AttackModule__power_mul_5th_implEPNS_26BattleObjectModuleAccessorE): source-owner probe (diag only)."),
    ("probe_attack_power_mul_status", 0x1FD0550, "Probes", "Called", "AttackModule::power_mul_status (lua_bind thunk, symbol _ZN3app8lua_bind35AttackModule__power_mul_status_implEPNS_26BattleObjectModuleAccessorE): source-owner probe (diag only)."),
    ("probe_damage_is_damage_lock", 0x200B890, "Probes", "Called", "DamageModule::is_damage_lock (lua_bind thunk, symbol _ZN3app8lua_bind33DamageModule__is_damage_lock_implEPNS_26BattleObjectModuleAccessorE): source-owner probe (diag only)."),
]
KIND = {
    "Called": "SeamKind::Called",
    "Relied": "SeamKind::Relied",
    "Hook": "SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid }",
}


def sections(elf):
    shoff, = struct.unpack_from("<Q", elf, 0x28)
    shentsize, shnum, shstrndx = struct.unpack_from("<HHH", elf, 0x3A)
    headers = []
    for i in range(shnum):
        o = shoff + i * shentsize
        name, _type, _flags, addr, offset, size = struct.unpack_from("<IIQQQQ", elf, o)
        headers.append((name, addr, offset, size))
    strtab = headers[shstrndx][2]
    out = {}
    for name, addr, offset, size in headers:
        end = elf.index(b"\0", strtab + name)
        out[elf[strtab + name:end].decode()] = (addr, offset, size)
    return out


def window(elf, secs, off):
    addr, offset, size = secs[".text"]
    assert addr <= off and off + 64 <= addr + size, hex(off)
    at = offset + off - addr
    return elf[at:at + 64]


def main():
    elf = open(sys.argv[1], "rb").read()
    secs = sections(elf)
    out = [
        "//! Package 6's site rows (game calls the carried code makes through smash:: that no other package guards: the CPU-as-P1 fallback's, the roster cycle's, and the diag-only source-owner probe's): generated by `tools/gen_sites_pkg6.py` from the",
        "//! owned 13.0.5 `main` (the guard bytes) and checked against it by `tests/pkg6_sites.rs`. The",
        "//! install line reports these after `sites::SITES`.",
        "",
        "use super::guard::{SeamKind, GUARD_BYTES};",
        "use super::sites::{Feature, Site};",
        "",
    ]
    for name, off, feature, kind, note in ROWS:
        if kind == "Hook":
            const = name.upper() + "_OFFSET"
            out.append(f"pub const {const}: usize = {off:#x};")
    out.append("")
    out.append(f"pub static SITES: [Site; {len(ROWS)}] = [")
    for name, off, feature, kind, note in ROWS:
        data = window(elf, secs, off)
        out.append("    Site {")
        out.append(f'        name: "{name}",')
        out.append(f"        feature: Feature::{feature},")
        out.append(f"        offset: {off:#x},")
        out.append(f"        kind: {KIND[kind]},")
        out.append("        guard: [")
        for i in range(0, 64, 16):
            out.append("            " + ", ".join(f"0x{b:02x}" for b in data[i:i + 16]) + ",")
        out.append("        ],")
        out.append(f'        note: "{note}",')
        out.append("    },")
    out.append("];")
    out.append("")
    out.append("/// The package's sites of one feature (all of them must be usable for the feature to install).")
    out.append("pub fn sites_of(feature: Feature) -> impl Iterator<Item = &'static Site> {")
    out.append("    SITES.iter().filter(move |site| site.feature == feature)")
    out.append("}")
    out.append("")
    out.append("const _: () = assert!(GUARD_BYTES == 64);")
    open(sys.argv[2], "w").write("\n".join(out) + "\n")


main()
