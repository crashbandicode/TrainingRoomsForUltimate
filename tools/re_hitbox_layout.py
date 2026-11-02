#!/usr/bin/env python3
"""Reproduce the AttackData layout evidence of SPEC-core-06 from the owned 13.0.5 `main` ELF (`docs/test-provenance/core-06.md`).

    python3 -I tools/re_hitbox_layout.py <main-13.0.5.elf>

The ELF made from the NSO keeps its vtables as zero words: their targets are in the relocation table that the NSO holds in
`.rodata` (Elf64_Rela, R_AARCH64_RELATIVE = 0x403). The table is found from one known entry (the fighter on-init slot,
`main+0x4f80240 -> main+0x60dfa0`), then AttackModule's vtable is found as the contiguous relocated array whose slots include
every slot the `AttackModule__*` lua_bind thunks load (`ldr xN, [x8, #slot]`, `main+0x1fd00c0..0x1fd1300`) with distinct targets
and whose `set_node` slot (0x148) function stores x2 at `+0x48` of `[module+0x68] + slot * 0x130`. The functions behind the
slots the plugin reads are then disassembled (llvm-objdump) for the doc. Read-only; prints to stdout.
"""
import re
import struct
import subprocess
import sys

TEXT_FILE_OFFSET = 0x1000
RO_ADDR, RO_OFF, RO_SIZE = 0x39C9000, 0x39CA000, 0x154DF60
ANCHOR = (0x4F80240, 0x60DFA0)  # fighter on_init slot -> function (known from sites.rs)
SLOTS_OF_INTEREST = [("set_attack", 0x68), ("is_attack", 0xC8), ("set_size", 0x118), ("set_offset", 0x130), ("set_offset2", 0x138),
                     ("get_offset2", 0x140), ("set_node", 0x148), ("attack_data", 0x1E8)]


def relocs(elf):
    ro = elf[RO_OFF:RO_OFF + RO_SIZE]
    at = ro.find(struct.pack("<QQq", ANCHOR[0], 0x403, ANCHOR[1]))
    if at < 0:
        sys.exit("relocation anchor not found")
    start = at
    while start >= 24 and struct.unpack("<QQq", ro[start - 24:start])[1] == 0x403:
        start -= 24
    end = at
    while end + 24 <= len(ro) and struct.unpack("<QQq", ro[end:end + 24])[1] == 0x403:
        end += 24
    return {off: add for off, _, add in (struct.unpack("<QQq", ro[p:p + 24]) for p in range(start, end, 24))}


def objdump(path, start, stop):
    out = subprocess.run(["/usr/bin/llvm-objdump", "-d", "--no-show-raw-insn", f"--start-address={start:#x}", f"--stop-address={stop:#x}", path],
                         capture_output=True, text=True, check=True).stdout
    return [line for line in out.splitlines() if re.match(r"^\s+[0-9a-f]+:", line)]


def thunk_slots(path):
    out = subprocess.run(["/usr/bin/llvm-objdump", "-d", "--no-show-raw-insn", "--start-address=0x1fd00c0", "--stop-address=0x1fd1300", path],
                         capture_output=True, text=True, check=True).stdout
    slots, current = set(), None
    for line in out.splitlines():
        m = re.match(r"^[0-9a-f]{16} <(.+)>:", line)
        if m:
            current = m.group(1)
            continue
        m = re.search(r"ldr\s+x\d+, \[x8, #(0x[0-9a-f]+)\]", line)
        if m and current and "AttackModule__" in current:
            slots.add(int(m.group(1), 16))
    return sorted(slots)


def main(path):
    elf = open(path, "rb").read()
    rel = relocs(elf)
    slots = thunk_slots(path)
    print(f"{len(rel)} relocations; {len(slots)} distinct AttackModule vtable slots used by the lua_bind thunks")
    starts = [a for a in sorted(rel) if all((a + s) in rel for s in slots) and len({rel[a + s] for s in slots}) >= 0.95 * len(slots)]
    firsts = [a for a in starts if (a - 8) not in set(starts)]
    found = None
    for a in firsts:
        f = rel[a + 0x148]
        code = objdump(path, f, f + 0x14)
        text = " ".join(code)
        if "ldr\tx8, [x0, #0x68]" in text and "#0x130" in text and "str\tx2, [x8, #0x48]" in text:
            found = a
            break
    if found is None:
        sys.exit("AttackModule vtable not found")
    print(f"AttackModule vtable (first virtual slot): main+{found:#x}")
    for name, slot in SLOTS_OF_INTEREST:
        f = rel[found + slot]
        print(f"\n== {name}: slot {slot:#x} -> main+{f:#x}")
        for line in objdump(path, f, f + (0x70 if name == "set_attack" else 0x30)):
            print(line)
            if "\tret" in line:
                break


if __name__ == "__main__":
    main(sys.argv[1])
