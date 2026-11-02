import struct, sys
elf = open(sys.argv[1], "rb").read()
phoff = struct.unpack_from("<Q", elf, 0x20)[0]
phentsize, phnum = struct.unpack_from("<HH", elf, 0x36)
base = 0x454a7e8
for i in range(phnum):
    t, fl, off, va, pa, fs, ms, al = struct.unpack_from("<IIQQQQQQ", elf, phoff + i * phentsize)
    if t == 1 and va <= base < va + fs:
        seg = (off, va, fs)
off, va, fs = seg
def rec(n):
    at = off + base - va + n * 72
    return struct.unpack_from("<9Q", elf, at)
for n in [int(x) for x in sys.argv[2:]]:
    r = rec(n)
    print(n, " ".join(f"{x:#x}" for x in r))