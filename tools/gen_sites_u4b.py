#!/usr/bin/env python3
"""Write `src/model/sites_u4b.rs` from the owned 13.0.5 main ELF.

    python3 -I tools/gen_sites_u4b.py <elf> <out.rs>

The table below is the source of truth for names, kinds and notes (stream u4b: the resident effect-texture helper,
`src/runtime/ui/effect_texture.rs`); the guard bytes come from the ELF. `tests/u4b_sites.rs` re-checks every row against the
same ELF on each `cargo test`.
"""
import struct
import sys

ROWS = [
    ("ui_register_view", 0x35B9360, "Called",
     "The game's descriptor registration `(u64* slot_out, texture_view*, user) -> bool`: takes the next free index of the shared descriptor pool, writes the view's "
     "descriptor there and stores the 8-byte slot. The UI texture update calls it with `(ui_texture + 0x40, view, 0)` and keeps the result at `+0x5c` "
     "(its destroy then releases the slot); the effect manager hands the same function to the particle system for every effect texture."),
    ("effmgr_instance_words", 0x34CB16C, "Relied",
     "The words that load the effect manager: `adrp x8, 0x5334000; ldr x8, [x8, #0x920]; ldr x0, [x8]` then `mov w1, #0x372; bl effect_load` (the game's own call "
     "`effect_load(*lib::Singleton<lib::EffectManager>::instance_, 0x372, &handle)`). The first two words name the slot `main+0x5334920` "
     "(`MANAGER_SLOT`), which the plugin decodes from them; the pointer in the slot points at the instance variable that holds the manager."),
    ("effmgr_entry_hash", 0x3560884, "Relied",
     "In `effect_load` (`main+0x355fea0`): the manager's file entry is `manager + 0x18000 + slot * 0xa8` (`mov w11, #0xa8; madd`); the file's hash40 is stored at "
     "`+8` (`mov w12, #0x8008; movk w12, #1, lsl 16; str x10, [x11, x12]`) and the reference count at `+0x98` (`mov w10, #0x8098`...)."),
    ("effmgr_entry_data", 0x35609AC, "Relied",
     "In `effect_load`: the loaded file's address is stored at entry `+0x10` (`mov w9, #0x8010; movk w9, #1, lsl 16; str x11, [x8, x9]`) and its first four bytes "
     "are checked against `EFFN` (`cmp w8, #0x45` ...)."),
    ("effmgr_system_base", 0x35609FC, "Relied",
     "In `effect_load`: the particle system is `[[manager + 0x193d8] + 0xf8]` (`mov w8, #0x93d8; movk w8, #1, lsl 16; add x9, x21, x8; str x9, [sp, #0x20]`, later `ldr x0, [x8, #0xf8]`), "
     "i.e. `[manager + 0x194d0]`; the `EFFN` header's block index is the short at `+0xe` (`ldrsh w8, [x11, #0xe]`)."),
    ("effmgr_system_resources", 0x3560A98, "Relied",
     "In `effect_load`, after the particle system made the resource: the entry's `+4` is its resource index (`mov w9, #0x8004; movk w9, #1, lsl 16; str w24, [x8, x9]`), and "
     "the resource is `[[system + 0x98] + index * 8]` (`ldr x8, [x8, #0xf8]; ldr x8, [x8, #0x98]; ldr x8, [x8, w24, uxtw #3]`), valid when its byte `+9` is set."),
    ("vfx_texture_table", 0x9A1B8, "Relied",
     "In the particle system's texture section handler: the resource keeps the texture file object at `+0xa8` (`ldr x8, [x19, #0xa8]`), its texture count is the word at "
     "`[file + 0x38] + 4`, stored at resource `+0xa0`, and the textures are the pointers of the array at `file + 0x28` (`ldr x8, [x8, #0x28]; ldr x22, [x8, x21, lsl #3]`)."),
    ("vfx_texture_slot", 0x9ABB4, "Relied",
     "In the resource's descriptor registration (`main+0x9ab70`, called with `main+0x35b9360` by the effect manager): for each texture it calls "
     "`register(&slot, [texture + 0x88] /* the texture view */, user)` and stores the 8-byte slot at `texture + 0x90` (`str x8, [x24, #0x90]`)."),
    ("vfx_emitter_table", 0x9AC08, "Relied",
     "In the same function: the resource's emitter sets are the table at `+0x58` (stride `0x38`; `+4` the emitter count, `+0x18` the emitter array), and each emitter "
     "(stride `0x3e0`) keeps the slots of its three samplers at `+0x48`, `+0x50` and `+0x58`."),
    ("vfx_emitter_sampler", 0x9AC4C, "Relied",
     "In the same function: the emitter's binary is the pointer at emitter `+0x10` (`ldur x8, [x20, #-0x38]`, with `x20` = emitter `+0x48`) and the three sampler texture ids "
     "are read from it at `+0x9f0`, `+0xa10` and `+0xa30`; the slot resolver `main+0x9acd0` stores the matching texture's slot into the emitter."),
]

KIND = {
    "Called": "SeamKind::Called",
    "Relied": "SeamKind::Relied",
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
        "//! Stream u4b's site rows (the resident effect-texture helper, `runtime/ui/effect_texture.rs`): generated by `tools/gen_sites_u4b.py` from the",
        "//! owned 13.0.5 `main` (the guard bytes) and checked against it by `tests/u4b_sites.rs`. Same row type and 64-byte guard rule as",
        "//! the other tables. The helper calls no new game function: every row is an instruction window whose offsets the helper relies on",
        "//! (the helper reads the effect manager's and the particle system's structures, and hands a descriptor slot to the UI texture code",
        "//! of `sites_u4`).",
        "",
        "use super::guard::{SeamKind, GUARD_BYTES};",
        "use super::sites::{Feature, Site};",
        "",
    ]
    for name, off, kind, note in ROWS:
        out.append(f"pub const {name.upper()}_OFFSET: usize = {off:#x};")
    out.append("")
    out.append(f"pub static SITES: [Site; {len(ROWS)}] = [")
    for name, off, kind, note in ROWS:
        data = window(elf, secs, off)
        out.append("    Site {")
        out.append(f'        name: "{name}",')
        out.append("        feature: Feature::Ui,")
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
    out.append("const _: () = assert!(GUARD_BYTES == 64);")
    open(sys.argv[2], "w").write("\n".join(out) + "\n")


main()
