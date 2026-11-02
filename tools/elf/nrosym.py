"""Exports of an NRO and a disassembly. Usage: nrosym.py <nro> [<symbol substring> <count>]"""
import struct
import sys

import capstone

nro = open(sys.argv[1], "rb").read()
text_off, text_sz, ro_off, ro_sz, data_off, data_sz, bss = struct.unpack_from("<IIIIIII", nro, 0x20)
mod_off = struct.unpack_from("<I", nro, 0x4)[0]  # MOD0 offset sits at text+4 (branch word then offset)
mod0 = struct.unpack_from("<I", nro, text_off + 4)[0]
magic, dyn_o, bss_s, bss_e, eh_s, eh_e, rt = struct.unpack_from("<4sIIIIII", nro, text_off + mod0)
assert magic == b"MOD0", magic
dyn = text_off + mod0 + dyn_o
tags = {}
i = 0
while True:
    tag, val = struct.unpack_from("<QQ", nro, dyn + i * 16)
    if tag == 0:
        break
    tags.setdefault(tag, val)
    i += 1
symtab, strtab, hashtab = tags.get(6), tags.get(5), tags.get(4)
gnu_hash = tags.get(0x6FFFFEF5)
# number of symbols: from DT_HASH nchain, else from strtab - symtab distance
nsyms = struct.unpack_from("<II", nro, hashtab)[1] if hashtab else (strtab - symtab) // 24
syms = []
for k in range(nsyms):
    name, info, other, shndx, value, size = struct.unpack_from("<IBBHQQ", nro, symtab + k * 24)
    end = nro.index(b"\0", strtab + name)
    syms.append((value, size, nro[strtab + name:end].decode("latin1"), shndx))
if len(sys.argv) == 2:
    for v, s, n, sh in syms:
        if sh != 0 and v:
            print(f"{v:#x} size={s:#x} {n}")
else:
    needle, count = sys.argv[2], int(sys.argv[3])
    md = capstone.Cs(capstone.CS_ARCH_ARM64, capstone.CS_MODE_ARM)
    for v, s, n, sh in syms:
        if needle in n and sh != 0:
            print(f"== {n} @ {v:#x} size={s:#x}")
            for ins in md.disasm(nro[v:v + min(s, count * 4)], v):
                print(f"{ins.address:#x}: {ins.mnemonic} {ins.op_str}")
            break
