#!/usr/bin/env python3
"""Generate the cut-file seam row from the owned 13.0.5 main ELF."""
import struct
import sys

GROUP = "cut_file"
SCOPES = "Scopes::ALL"
HOOK = "Hook { form: HookForm::Inline, placement: Placement::Mid }"
ROWS = [
    ("cut_file.picture_copied", 0x37A2DD4, HOOK, False,
     "Skyline writes one word (`b`), or a far jump, here 20 bytes because the site is 4 past an 8-byte boundary: "
     "`0x37a2dd4..0x37a2de8`. Those five words are the row's guard span. They contain one PC-relative word, "
     "`bl main+0x31100`, which Skyline's trampoline relocates. No direct branch in `.text` targets "
     "`0x37a2dd8..0x37a2de8` and no word of the function `0x37a2c70..0x37a2e18` names x16 or x17."),
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
    for name in (".text", ".rodata"):
        addr, off, size = secs[name]
        if addr <= at and at + 64 <= addr + size:
            return elf[off + at - addr: off + at - addr + 64]
    raise SystemExit(f"{at:#x} is in neither .text nor .rodata")


def main():
    elf = open(sys.argv[1], "rb").read()
    secs = sections(elf)
    lines = ["#! Seam\tlabel\tgroup\toffset\tkind\tguard\tfeature\tscopes\tread_only\tper_frame\tsharing\tnote"]
    for label, at, kind, per_frame, note in ROWS:
        fields = [f'"{label}"', f'"{GROUP}"', f"{at:#x}", f"SeamKind::{kind}", window(elf, secs, at).hex(), '""',
                  SCOPES, "true", str(per_frame).lower(), "Sharing::None", '"' + note.replace('"', '\\"') + '"']
        lines.append("\t".join(fields))
    open(sys.argv[2], "w").write("\n".join(lines) + "\n")
    print("wrote", sys.argv[2], len(ROWS), "rows")


if __name__ == "__main__":
    main()
