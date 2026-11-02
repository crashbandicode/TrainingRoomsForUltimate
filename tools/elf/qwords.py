import struct, sys
elf = open(sys.argv[1], "rb").read()
addr, n = int(sys.argv[2], 16), int(sys.argv[3])
phoff = struct.unpack_from("<Q", elf, 0x20)[0]
phentsize, phnum = struct.unpack_from("<HH", elf, 0x36)
for i in range(phnum):
    t, fl, off, va, pa, fs, ms, al = struct.unpack_from("<IIQQQQQQ", elf, phoff + i * phentsize)
    if t == 1 and va <= addr < va + fs:
        for k in range(n):
            print(f"{addr + 8*k:#x}: {struct.unpack_from('<Q', elf, off + addr - va + 8*k)[0]:#x}")