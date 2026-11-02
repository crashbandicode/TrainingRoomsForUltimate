import struct, sys
elf = open(sys.argv[1], "rb").read()
addr = int(sys.argv[2], 16)
phoff = struct.unpack_from("<Q", elf, 0x20)[0]
phentsize, phnum = struct.unpack_from("<HH", elf, 0x36)
for i in range(phnum):
    t, fl, off, va, pa, fs, ms, al = struct.unpack_from("<IIQQQQQQ", elf, phoff + i * phentsize)
    if t == 1 and va <= addr < va + fs:
        d = elf[off + addr - va: off + addr - va + 64]
        for k in range(0, 64, 16):
            print("            " + ", ".join(f"0x{b:02x}" for b in d[k:k+16]) + ",")