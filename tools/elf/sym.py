"""Symbols of the owned ELF. Usage: sym.py <elf> near <vaddr hex> | name <substring> | callers <vaddr hex>"""
import array
import struct
import sys

elf = open(sys.argv[1], "rb").read()
shoff = struct.unpack_from("<Q", elf, 0x28)[0]
shentsize, shnum, shstrndx = struct.unpack_from("<HHH", elf, 0x3A)
secs = [struct.unpack_from("<IIQQQQIIQQ", elf, shoff + i * shentsize) for i in range(shnum)]
syms = []
for name, typ, flags, addr, off, size, link, info, align, entsize in secs:
    if typ in (2, 11):  # SYMTAB / DYNSYM
        stroff = secs[link][4]
        for i in range(size // 24):
            n, inf, oth, shndx, value, sz = struct.unpack_from("<IBBHQQ", elf, off + i * 24)
            end = elf.index(b"\0", stroff + n)
            syms.append((value, sz, elf[stroff + n:end].decode("latin1")))
syms.sort()
mode = sys.argv[2]
if mode == "near":
    target = int(sys.argv[3], 16)
    best = [s for s in syms if s[0] <= target and s[0] != 0][-3:]
    for v, sz, n in best:
        print(f"{v:#x} size={sz:#x} {n}")
elif mode == "name":
    for v, sz, n in syms:
        if sys.argv[3] in n:
            print(f"{v:#x} size={sz:#x} {n}")
elif mode == "callers":
    target = int(sys.argv[3], 16)
    phoff = struct.unpack_from("<Q", elf, 0x20)[0]
    phentsize, phnum = struct.unpack_from("<HH", elf, 0x36)
    for i in range(phnum):
        t, fl, off, va, pa, fs, ms, al = struct.unpack_from("<IIQQQQQQ", elf, phoff + i * phentsize)
        if t == 1 and fl & 1:
            words = array.array("I")
            words.frombytes(elf[off:off + (fs & ~3)])
            for k, w in enumerate(words):
                if (w & 0xFC000000) in (0x94000000, 0x14000000):
                    imm = w & 0x3FFFFFF
                    if imm & 0x2000000:
                        imm -= 0x4000000
                    if va + k * 4 + imm * 4 == target:
                        print(f"{'bl' if w >> 26 == 0x25 else 'b'} at {va + k * 4:#x}")
