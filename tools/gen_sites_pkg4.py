#!/usr/bin/env python3
"""Write `src/model/sites_pkg4.rs` from the owned 13.0.5 main ELF.

    python3 -I tools/gen_sites_pkg4.py <elf> <out.rs>

The table below is the source of truth for names, features, kinds and notes; the guard bytes come from
the ELF. `tests/input_sites.rs` re-checks every row against the same ELF on each `cargo test`.
"""
import struct
import sys

ROWS = [
    ("posture_lr", 0x206AF10, "Recording", "Called", "PostureModule::lr (lua_bind thunk): the CPU's facing, +1.0 or -1.0. Symbol _ZN3app8lua_bind22PostureModule__lr_implEPNS_26BattleObjectModuleAccessorE."),
    ("stop_is_stop", 0x20891F0, "Recording", "Called", "StopModule::is_stop (lua_bind thunk): whether the CPU is in hitstop, for the mash-triggered playback timing."),
    ("work_set_int", 0x208DBD0, "Recording", "Called", "WorkModule::set_int (lua_bind thunk): lifts the ledge rest timer while the recorder holds the CPU on the ledge."),
    ("model_set_color_rgb", 0x2057790, "Recording", "Called", "ModelModule::set_color_rgb (lua_bind thunk): the recorder phase tint of the CPU model."),
    ("fighter_manager_get_entry", 0x2140F90, "Recording", "Called", "FighterManager::get_fighter_entry (lua_bind thunk): the CPU entry, whose +0x4178 is its controller and +0x4160 its first body."),
    ("ai_epilogue", 0x2DBC38, "Recording", "Hook", "Epilogue of the AI control function (main+0x2da180, whose entry the CPU pilot hooks): its one path ends here after the controller write; x19 is the AI object and the CPU controller is the pointer at x19+0x8. Every branch of the function joins the controller write, so this runs once per AI control frame."),
    ("combo_display_a_pin", 0x13E5498, "Notices", "Relied", "The ldrb of PauseMenu+0xb6c (Combo Display) whose result the first observer reads."),
    ("combo_display_a", 0x13E549C, "Notices", "Hook", "Inline observer after that ldrb: w20 is the Combo Display byte; the value is copied, the pointer is never retained."),
    ("combo_display_b_pin", 0x13E54D4, "Notices", "Relied", "The second ldrb of PauseMenu+0xb6c."),
    ("combo_display_b", 0x13E54D8, "Notices", "Hook", "Inline observer at the strb that stores it: w8 is the Combo Display byte."),
    ("stale_moves_toggle", 0x13E88C4, "Notices", "Hook", "Inline at the call after the game clears the stale-moves toggle (PauseMenu+0xb60, x22 is the menu object): the toggle is set to 1 so stale moves are on in Training."),
    ("control_get_button", 0x20013E0, "NamedMacros", "Called", "ControlModule::get_button (lua_bind thunk): the CPU's button word, for the named playback observation lines."),
    ("control_get_stick_x", 0x2001260, "NamedMacros", "Called", "ControlModule::get_stick_x (lua_bind thunk): the clamped stick x of the replay capture's observed input."),
    ("control_get_stick_y", 0x2001280, "NamedMacros", "Called", "ControlModule::get_stick_y (lua_bind thunk): the clamped stick y of the observed input."),
    ("control_get_stick_x_no_clamp", 0x2001600, "NamedMacros", "Called", "ControlModule::get_stick_x_no_clamp (lua_bind thunk): the raw stick x the capture quantises."),
    ("control_get_stick_y_no_clamp", 0x2001610, "NamedMacros", "Called", "ControlModule::get_stick_y_no_clamp (lua_bind thunk): the raw stick y."),
    ("control_get_sub_stick_x", 0x2001370, "NamedMacros", "Called", "ControlModule::get_sub_stick_x (lua_bind thunk): the C-stick x of the observed input."),
    ("control_get_sub_stick_y", 0x2001390, "NamedMacros", "Called", "ControlModule::get_sub_stick_y (lua_bind thunk): the C-stick y of the observed input."),
    ("kinetic_get_energy", 0x2043970, "NamedMacros", "Called", "KineticModule::get_energy (lua_bind thunk): the energy object the named start pose sets the speed of."),
    ("kinetic_get_sum_speed_x", 0x2043A00, "NamedMacros", "Called", "KineticModule::get_sum_speed_x (lua_bind thunk): the start state's and the capture's x speed."),
    ("kinetic_get_sum_speed_y", 0x2043A10, "NamedMacros", "Called", "KineticModule::get_sum_speed_y (lua_bind thunk): the start state's and the capture's y speed."),
    ("stop_is_special_stop", 0x2089200, "NamedMacros", "Called", "StopModule::is_special_stop (lua_bind thunk): whether the panel already holds a fighter still."),
    ("stop_set_special_stop", 0x2089290, "NamedMacros", "Called", "StopModule::set_special_stop (lua_bind thunk): holds a practice fighter still while the named panel or the clip editor is open."),
    ("kinetic_energy_normal_set_speed", 0x20F8320, "NamedMacros", "Called", "KineticEnergyNormal::set_speed (lua_bind thunk): the x start speed Play gives the playing role (PLAY-MOMENTUM) and the named start pose's. Symbol _ZN3app8lua_bind35KineticEnergyNormal__set_speed_implEPNS_19KineticEnergyNormalERKN3phx8Vector2fE."),
    ("kinetic_energy_gravity_set_speed", 0x2120230, "NamedMacros", "Called", "FighterKineticEnergyGravity::set_speed (lua_bind thunk): the y start speed of an aerial start (PLAY-MOMENTUM, the named start pose). Symbol _ZN3app8lua_bind43FighterKineticEnergyGravity__set_speed_implEPNS_27FighterKineticEnergyGravityEf."),
]

# Called rows other streams name by offset constant (kept from the hand-edited file of S4-1).
OFFSET_CONSTS = {"posture_lr", "stop_is_stop", "work_set_int", "model_set_color_rgb", "fighter_manager_get_entry"}

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
        "//! Package 4's site rows (recording, notifications): generated by `tools/gen_sites_pkg4.py` from the",
        "//! owned 13.0.5 `main` (the guard bytes) and checked against it by `tests/input_sites.rs`. The",
        "//! install line reports these after `sites::SITES`.",
        "",
        "use super::guard::{HookForm, Placement, SeamKind, GUARD_BYTES};",
        "use super::sites::{Feature, Site};",
        "",
    ]
    for name, off, feature, kind, note in ROWS:
        if kind == "Hook" or name in OFFSET_CONSTS:
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
