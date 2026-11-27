#!/usr/bin/env python3
"""Write the speed-label guard rows from the owned 13.0.5 main ELF.

    python3 -I tools/gen_rows_speed_label.py <main-13.0.5.elf> data/tables/rows_speed_label.ROWS.tsv

The guard bytes come from the ELF's .text, .rodata or .rodata.1 sections; `native_seams::tests::compared_rows_match_the_owned_elf`
re-checks them, and `speed_label_rows_tests` decodes the constants. The design is `docs/DECISIONS.md#rc15-fix-f5-f3`.
"""
import struct
import sys

GROUP = "speed_label"
SCOPES = "Scopes(Scopes::TRAINING.0 | Scopes::LOBBY_WAITING.0)"
ROWS = [
    ("speed_label.frame_text", 0x1BBBD78, "Relied", False,
     "The Speed text function `0x1bbbd30`'s case for value 2, 1 Frame (jump table `0x44f8a24` entry 0): "
     "`ldr w8,[x22,#0x30]` (the menu's controller glyph style), then the button code in w20: "
     "`[main+0x44681a0 + 4*(style-3)]` for styles 3..6, else `0xc` (`.1bbbe34`). "
     "The bind lends the menu Player 1's style for this call."),
    ("speed_label.frame_set", 0x1BBBEB4, "Relied", False,
     "The same case sets the menu row: `0x377a280(pane, mel_training_speed1, 1, w20)`, then reads `[x22,#0x30]` again "
     "and stores it into the stopwatch text object's `+0x25c` (`str w8,[x21,#0x25c]`) before "
     "`mel_training_speed_control_frame` is set the same way (`.1bbbf4c`-`.1bbbf78`)."),
    ("speed_label.style_source", 0x13E4E0C, "Relied", False,
     "The Training HUD's menu-open step (`FUN_013e4de0`), the opening controller's index in x1 (-1 for none): "
     "the controller `[main+0x5339860 + 8*index]` and its style `[[c+0x10]+0x9c]` "
     "(`ldr x8,[x8,#0x10]; ldr w8,[x8,#0x9c]`), the field Player 1's poll reads."),
    ("speed_label.style_store", 0x13E4E4C, "Relied", False,
     "The same step: styles 1..7 through the table `main+0x4753de0` (`speed_label.glyph_styles`) into w22, "
     "0 for any other, `bl 0x1bb8b20(menu, index)` and `str w22,[x21,#0x30]`. "
     "The constructor leaves 0 there (`.1bb16b0`); `0x1bb8b20` stores it too (`.1bb8db4`)."),
    ("speed_label.glyph_styles", 0x4753DE0, "Relied", False,
     "The glyph style by controller style 1..7: `1,2,3,4,5,6,6` (the GameCube pad, 7, shares 6)."),
]


def sections(elf):
    shoff, = struct.unpack_from("<Q", elf, 0x28)
    shentsize, shnum, shstrndx = struct.unpack_from("<HHH", elf, 0x3A)
    headers = [struct.unpack_from("<IIQQQQ", elf, shoff + i * shentsize) for i in range(shnum)]
    strtab = headers[shstrndx][4]
    out = {}
    for name, _t, _f, addr, off, size in headers:
        end = elf.index(b"\0", strtab + name)
        out[elf[strtab + name:end].decode()] = (addr, off, size)
    return out


def window(elf, secs, at):
    for name in (".text", ".rodata", ".rodata.1"):
        addr, off, size = secs[name]
        if addr <= at and at + 64 <= addr + size:
            return elf[off + at - addr: off + at - addr + 64]
    raise SystemExit(f"{at:#x} is in neither .text nor read-only data")


def main():
    elf = open(sys.argv[1], "rb").read()
    secs = sections(elf)
    lines = ["#! Seam\tlabel\tgroup\toffset\tkind\tguard\tfeature\tscopes\tread_only\tper_frame\tsharing\tnote"]
    for label, at, kind, per_frame, note in ROWS:
        read_only = "true" if kind == "Relied" else "false"
        fields = [f'"{label}"', f'"{GROUP}"', f"{at:#x}", f"SeamKind::{kind}", window(elf, secs, at).hex(), '""', SCOPES,
                  read_only, str(per_frame).lower(), "Sharing::None", '"' + note.replace('"', '\\"') + '"']
        lines.append("\t".join(fields))
    open(sys.argv[2], "w").write("\n".join(lines) + "\n")
    print("wrote", sys.argv[2], len(ROWS), "rows")


if __name__ == "__main__":
    main()
