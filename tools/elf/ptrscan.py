import struct, sys
elf = open(sys.argv[1], "rb").read()
target = int(sys.argv[2], 16)
phoff = struct.unpack_from("<Q", elf, 0x20)[0]
phentsize, phnum = struct.unpack_from("<HH", elf, 0x36)
pat = struct.pack("<Q", target)
for i in range(phnum):
    t, fl, off, va, pa, fs, ms, al = struct.unpack_from("<IIQQQQQQ", elf, phoff + i * phentsize)
    if t != 1: continue
    pos = elf.find(pat, off, off + fs)
    while pos != -1:
        if pos % 8 == 0:
            print(f"seg flags={fl} data ptr at {va + pos - off:#x}")
        pos = elf.find(pat, pos + 1, off + fs)