#!/usr/bin/env python3
"""Look up symbols and 64-byte guard windows in the owned 13.0.5 `main` ELF (read-only).

    python3 -I tools/elf_symbols.py find <regex>            # symbols whose name matches, with main+offset
    python3 -I tools/elf_symbols.py window <offset-hex>     # the 64 bytes at main+offset, as Rust bytes
    python3 -I tools/elf_symbols.py rows <row>=<symbol> ... # offset and guard bytes for each, as a JSON list
    python3 -I tools/elf_symbols.py words <offset-hex> <n>  # n instruction words at main+offset (raw hex)

The ELF is `research/game-13.0.5/main-13.0.5.elf` next to the worktrees (never committed). `.text` maps
at address 0, so an address is the `main+offset` the site table uses. Written for package 3 (the same
section/symbol reading as `tests/common/mod.rs`).
"""
import json
import os
import re
import struct
import sys

ELF = os.environ.get(
    "OWNED_ELF",
    os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "research", "game-13.0.5", "main-13.0.5.elf"),
)


def load():
    with open(ELF, "rb") as f:
        elf = f.read()
    assert elf[:4] == b"\x7fELF"
    shoff, = struct.unpack_from("<Q", elf, 0x28)
    shentsize, shnum, shstrndx = struct.unpack_from("<HHH", elf, 0x3A)

    def header(i):
        return shoff + i * shentsize

    stroff, = struct.unpack_from("<Q", elf, header(shstrndx) + 0x18)
    sections = {}
    for i in range(shnum):
        h = header(i)
        name_at = stroff + struct.unpack_from("<I", elf, h)[0]
        name = elf[name_at : elf.index(b"\0", name_at)].decode()
        addr, off, size = struct.unpack_from("<QQQ", elf, h + 0x10)
        sections[name] = (addr, off, size)
    return elf, sections


def symbols(elf, sections):
    symtab, strtab = sections[".symtab"], sections[".strtab"]
    out = []
    for i in range(symtab[2] // 24):
        at = symtab[1] + i * 24
        name_off = struct.unpack_from("<I", elf, at)[0]
        value = struct.unpack_from("<Q", elf, at + 8)[0]
        start = strtab[1] + name_off
        end = elf.index(b"\0", start)
        if end > start:
            out.append((elf[start:end].decode(errors="replace"), value))
    return out


def window(elf, sections, offset, n=64):
    for name in (".text", ".rodata", ".data"):
        addr, off, size = sections[name]
        if addr <= offset and offset + n <= addr + size:
            at = off + (offset - addr)
            return elf[at : at + n]
    raise SystemExit(f"{offset:#x} is outside .text/.rodata/.data")


def main(argv):
    if len(argv) < 2:
        raise SystemExit(__doc__)
    elf, sections = load()
    cmd = argv[1]
    if cmd == "find":
        pattern = re.compile(argv[2])
        for name, value in sorted(symbols(elf, sections), key=lambda s: s[1]):
            if pattern.search(name):
                print(f"{value:#x} {name}")
    elif cmd == "window":
        data = window(elf, sections, int(argv[2], 16))
        print(", ".join(f"0x{b:02x}" for b in data))
    elif cmd == "words":
        n = int(argv[3])
        data = window(elf, sections, int(argv[2], 16), 4 * n)
        print(" ".join(f"{w:08x}" for w in struct.unpack(f"<{n}I", data)))
    elif cmd == "rows":
        table = {}
        for name, value in symbols(elf, sections):
            table.setdefault(name, value)
        rows = []
        for item in argv[2:]:
            row, symbol = item.split("=", 1)
            if symbol not in table:
                raise SystemExit(f"{row}: no symbol {symbol}")
            offset = table[symbol]
            rows.append({"row": row, "symbol": symbol, "offset": offset, "bytes": list(window(elf, sections, offset))})
        print(json.dumps(rows))
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main(sys.argv)
