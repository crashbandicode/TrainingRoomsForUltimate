"""Disassemble a range of the owned ELF. Usage: dis.py <elf> <vaddr hex> <count>"""
import struct
import sys

import capstone

elf = open(sys.argv[1], "rb").read()
phoff = struct.unpack_from("<Q", elf, 0x20)[0]
phentsize, phnum = struct.unpack_from("<HH", elf, 0x36)
segs = []
for i in range(phnum):
    t, fl, off, va, pa, fs, ms, al = struct.unpack_from("<IIQQQQQQ", elf, phoff + i * phentsize)
    if t == 1:
        segs.append((off, va, fs))
start, count = int(sys.argv[2], 16), int(sys.argv[3])
for off, va, fs in segs:
    if va <= start < va + fs:
        data = elf[off + start - va: off + start - va + count * 4]
        md = capstone.Cs(capstone.CS_ARCH_ARM64, capstone.CS_MODE_ARM)
        for ins in md.disasm(data, start):
            print(f"{ins.address:#x}: {ins.mnemonic} {ins.op_str}")
        break
