#!/usr/bin/env python3
"""Print `guard` byte arrays (64 bytes each) for site offsets, read from the owned 13.0.5 main ELF.

    python3 -I tools/gen_site_rows.py <elf> name=0xOFFSET [name=0xOFFSET ...]

Offsets are from the start of `.text` (which maps at 0). Used to write `src/model/sites_pkg4.rs`; the
host test `tests/input_sites.rs` re-checks every row against the same ELF.
"""
import struct
import sys


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


def main():
    elf = open(sys.argv[1], "rb").read()
    secs = sections(elf)
    for arg in sys.argv[2:]:
        name, off = arg.split("=")
        off = int(off, 16)
        for sec in (".text", ".rodata", ".data"):
            addr, offset, size = secs[sec]
            if addr <= off and off + 64 <= addr + size:
                at = offset + off - addr
                data = elf[at:at + 64]
                break
        else:
            raise SystemExit(f"{name}: {off:#x} is not inside one section")
        print(f"// {name} {off:#x}")
        print("guard: [")
        for i in range(0, 64, 16):
            print("    " + ", ".join(f"0x{b:02x}" for b in data[i:i + 16]) + ",")
        print("],")


main()
