"""Find code that builds a 40-bit hash constant (MOVZ/MOVK sequences) in the owned 13.0.5 main ELF.
Usage: hashscan.py <elf> <hash hex> [<hash hex> ...]   (output: vaddr of the first instruction of each match)"""
import struct
import sys

elf = open(sys.argv[1], "rb").read()
phoff = struct.unpack_from("<Q", elf, 0x20)[0]
phentsize, phnum = struct.unpack_from("<HH", elf, 0x36)
segs = []
for i in range(phnum):
    p_type, p_flags, p_offset, p_vaddr, p_paddr, p_filesz, p_memsz, p_align = struct.unpack_from("<IIQQQQQQ", elf, phoff + i * phentsize)
    if p_type == 1:
        segs.append((p_offset, p_vaddr, p_filesz, p_flags))


def vaddr(off):
    for o, v, s, _ in segs:
        if o <= off < o + s:
            return v + off - o
    return None


def words(off0, n):
    return struct.unpack_from(f"<{n}I", elf, off0)


def movz_x(rd, imm, hw):
    return 0xD2800000 | (hw << 21) | (imm << 5) | rd


def movk_x(rd, imm, hw):
    return 0xF2800000 | (hw << 21) | (imm << 5) | rd


def movz_w(rd, imm, hw):
    return 0x52800000 | (hw << 21) | (imm << 5) | rd


def movk_w(rd, imm, hw):
    return 0x72800000 | (hw << 21) | (imm << 5) | rd


text = [(o, s) for o, v, s, f in segs if f & 1]
for h in map(lambda x: int(x, 16), sys.argv[2:]):
    lo16, mid16, hi = h & 0xFFFF, (h >> 16) & 0xFFFF, h >> 32
    found = []
    for rd in range(31):
        # either the 32-bit low part first (movz low, movk mid) then a movk hi at lsl 32, in either order
        for first in (movz_w(rd, lo16, 0), movz_x(rd, lo16, 0)):
            pat = struct.pack("<I", first)
            for o, s in text:
                pos = elf.find(pat, o, o + s)
                while pos != -1:
                    if pos % 4 == 0:
                        window = words(pos, min(8, (o + s - pos) // 4))
                        mids = {movk_x(rd, mid16, 1), movk_w(rd, mid16, 1)}
                        his = {movk_x(rd, hi, 2)} if hi else set()
                        if any(w in mids for w in window[1:]) and (not hi or any(w in his for w in window[1:])):
                            found.append(vaddr(pos))
                    pos = elf.find(pat, pos + 4, o + s)
    # a plain 32-bit `ldr` literal is rare; report only the movz/movk matches
    print(f"{h:#x}: " + ", ".join(f"{v:#x}" for v in sorted(set(found))))
