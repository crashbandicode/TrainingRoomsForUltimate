import struct, sys, re

import capstone
elf = open(sys.argv[1], "rb").read()
start, end = int(sys.argv[2], 16), int(sys.argv[3], 16)
labels = {}
for line in open("/home/intpa/smash-training-lobby/artifacts/enhancements/stage-assets/ParamLabels.csv", encoding="utf-8", errors="replace"):
    p = line.strip().split(",", 1)
    if len(p) == 2 and p[0].startswith("0x"):
        try: labels[int(p[0], 16)] = p[1]
        except ValueError: pass
phoff = struct.unpack_from("<Q", elf, 0x20)[0]
phentsize, phnum = struct.unpack_from("<HH", elf, 0x36)
for i in range(phnum):
    t, fl, off, va, pa, fs, ms, al = struct.unpack_from("<IIQQQQQQ", elf, phoff + i * phentsize)
    if t == 1 and va <= start < va + fs:
        data = elf[off + start - va: off + end - va]
md = capstone.Cs(capstone.CS_ARCH_ARM64, capstone.CS_MODE_ARM)
regs = {}
last_hash = None
out = []
for ins in md.disasm(data, start):
    m = ins.mnemonic
    ops = [o.strip() for o in ins.op_str.split(",")]
    if m == "mov" and ops[0].startswith("x") and ops[1].startswith("#"):
        regs[ops[0]] = int(ops[1][1:], 0)
    elif m == "movk" and ops[0] in regs:
        sh = int(ops[2].split()[-1][1:], 0) if len(ops) > 2 else 0
        regs[ops[0]] = (regs[ops[0]] & ~(0xFFFF << sh)) | (int(ops[1][1:], 0) << sh)
        if sh == 32 and regs[ops[0]] in labels:
            last_hash = labels[regs[ops[0]]]
    elif m in ("str",) and "x19, #" in ins.op_str and last_hash:
        out.append((ins.address, last_hash, ins.op_str)); last_hash = None
for a, n, o in out:
    print(f"{a:#x} {n} {o}")