#!/usr/bin/env python3
"""Write `data/tables/rows_custom_stages.ROWS.tsv` (rc.14 custom stages: a Stage Builder stage loaded into the rule by the
game's own loader, then requested as stage 325; `src/runtime/lobby/custom_stages.rs`) from the owned 13.0.5 main ELF.

    python3 -I tools/gen_rows_custom_stages.py <main-13.0.5.elf> <out.tsv>

The rows below are the source of truth for labels, kinds and notes; the guard bytes come from the ELF (.text), and the
host test `native_seams::tests::compared_rows_match_the_owned_elf` re-checks them on every `cargo test`. The facts are
artifacts/rc14-custom-stages/DESIGN.md (static disassembly, Eden probes cs-probe-x1..x3); `runtime::lobby::custom_stages::SEAMS`
lists the same labels.
"""
import struct
import sys

GROUP = "custom_stages"
FEATURE = ""
# (label, offset, kind, note)
ROWS = [
    ("custom_stages.loader", 0x17D0120, "Called",
     "called: `0x17d0120(holder, id, &stage_entry, &out)`, the game's custom-stage loader: finds the 0x32198-byte record "
     "`id` in the holder's vector `+0x70/+0x78`, copies record bytes 0x134/0x138 to out+0/+1, u32 0x150 to out+4, bytes "
     "0x154/0x156 to out+8/+9, the u64 at 0x140 to stage_entry+0x18, and the layout data [0x198, 0x198 + u32 0x15c) into "
     "the vector out+0x10/+0x18/+0x20 (game heap `0x392e290`). TR passes a holder on its stack over its own read of the file."),
    ("custom_stages.free", 0x392EB40, "Called",
     "called: the game heap's free (`0x392eb40(ptr)`, null-safe), with which `0x2317840` frees the rule's old vectors; TR "
     "frees only vectors it took out of the rule itself."),
    ("custom_stages.game_load_holder", 0x2317858, "Relied",
     "The game's own load `0x2317840`: holder `*(*(*(main+0x5314510))+0x298)`, the rule index (`main+0x530a968` unless the "
     "caller's +0x194 is set)."),
    ("custom_stages.game_load_idle", 0x2317898, "Relied",
     "Its idle condition before the loader: storage `[holder+0x68]` null, or `[storage+0x448]` 0 and `ldar [[main+0x593b218]+0x50]` "
     "0; then index < 3 and the stage entry `main+0x530a7b8 + r*0x20`. TR loads only when this holds."),
    ("custom_stages.game_load_call", 0x23178D8, "Relied",
     "The loader call: the entry copied to the stack, the out block zeroed (u16 +0, u32 +4, u16 +8, two vectors +0x10/+0x28), "
     "the id from `main+0x530a8fc + r*0x30`, `bl 0x17d0120`."),
    ("custom_stages.game_load_scalars", 0x2317998, "Relied",
     "The rule writes after the loader: rule `main+0x52c5180 + r*0x40` + 0x45698 (u16), +0x4569c (u32), +0x456a0 (u16), "
     "then the data vector +0x456a8.. swapped with the out vector."),
    ("custom_stages.game_load_vectors", 0x23179D8, "Relied",
     "The vector swap: data `+0x456a8/+0x456b0/+0x456b8`, extra `+0x456c0/+0x456c8/+0x456d0` take the out block's, the old "
     "ones go back to the stack for `0x392eb40`."),
    ("custom_stages.game_load_vectors+0x40", 0x2317A18, "Relied",
     "The rest of the swap (extra vector)."),
    # The game's own read of a saved stage, as local wireless runs it (cs-probe-y1: outside Stage Builder only the headers
    # are in memory, so TR reads the record through these four calls; it never opens a file).
    ("custom_stages.queue", 0x17CFD40, "Called",
     "called: `0x17cfd40(H, id)` appends a stage id to the stage save object's request list `H+0x88/+0x90/+0x98`."),
    ("custom_stages.begin", 0x17CFEB0, "Called",
     "called: `0x17cfeb0(H)` (nothing queued: returns): creates the storage `H+0x68` (`0x17cf610`), builds the "
     "`stage_%d.bin` paths (`0x17cff70`), queues commands 6 (read) and 3 (finish) and enters the first state; the game's "
     "frames run the storage, the record lands in `H+0x70/+0x78`."),
    ("custom_stages.cleanup", 0x17CFBC0, "Called",
     "called: `0x17cfbc0(H+0x68)` after the load: frees the storage and the read's vectors (`H+0x70`, `H+0x88`)."),
    ("custom_stages.lw_request", 0x2336710, "Relied",
     "Local wireless's own read request `0x2336560`: `H = *(*(*(main+0x5314510))+0x298)`, `0x17cfd40(H, id)`, "
     "`0x17cfeb0(H)`: the call order TR repeats."),
    ("custom_stages.lw_cleanup", 0x23369E0, "Relied",
     "Local wireless's load `0x23367c0` after `0x17d0120` and the rule writes: `0x17cfbc0(H+0x68)`."),
    ("custom_stages.header_list", 0x17AE0E8, "Relied",
     "`0x17ad770` reads the saved stages from the stage save object: headers `H+0x18/+0x20` (0x198 each, id @0, name "
     "UTF-16 @0x110), else the full records `H+0/+8` (0x32198 each). TR's list reads the same, in place."),
    ("custom_stages.record_list", 0x17AE134, "Relied",
     "The records branch of the same read: `H+0/+8`, stride 0x32198 (filled inside Stage Builder only)."),
    ("custom_stages.setter_save_byte", 0x178B4A0, "Relied",
     "The stage setter `0x178a9c0` for stage 325 (`cmp w24,#0x145`): copies the rule byte `main+0x52c5180 + r*0x40 + 0x45699` "
     "(record byte 0x138) into the descriptor `main+0x52c55d0 + slot*0xb0` + 0x98. TR's in-place requests carry it the same way."),
]


def sections(elf):
    shoff, = struct.unpack_from("<Q", elf, 0x28)
    shentsize, shnum, shstrndx = struct.unpack_from("<HHH", elf, 0x3A)
    headers = [struct.unpack_from("<IIQQQQ", elf, shoff + i * shentsize) for i in range(shnum)]
    strtab = headers[shstrndx][4]
    out = {}
    for name, _t, _f, addr, off, size in headers:
        end = elf.index(b"\0", strtab + name)
        out[elf[strtab + name:end].decode()] = (addr, off, size)
    return out


def window(elf, secs, at):
    addr, off, size = secs[".text"]
    if addr <= at and at + 64 <= addr + size:
        return elf[off + at - addr: off + at - addr + 64]
    raise SystemExit(f"{at:#x} is not in .text")


def main():
    elf = open(sys.argv[1], "rb").read()
    secs = sections(elf)
    lines = ["#! Seam\tlabel\tgroup\toffset\tkind\tguard\tfeature\tscopes\tread_only\tper_frame\tsharing\tnote"]
    for label, at, kind, note in ROWS:
        read_only = "true" if kind == "Relied" else "false"
        fields = [f'"{label}"', f'"{GROUP}"', f"{at:#x}", f"SeamKind::{kind}", window(elf, secs, at).hex(),
                  f'"{FEATURE}"', "Scopes::ALL", read_only, "false", "Sharing::None", '"' + note.replace('"', '\\"') + '"']
        lines.append("\t".join(fields))
    open(sys.argv[2], "w").write("\n".join(lines) + "\n")
    print("wrote", sys.argv[2], len(ROWS), "rows")


if __name__ == "__main__":
    main()
