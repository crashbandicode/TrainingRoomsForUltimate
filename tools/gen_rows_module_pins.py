#!/usr/bin/env python3
"""Write `data/tables/rows_module_pins.ROWS.tsv` (rc.13, (a′) proper: a kept group's fighter module is pinned across its
owner's Reload; `src/runtime/roster/module_pins.rs`) from the owned 13.0.5 main ELF.

    python3 -I tools/gen_rows_module_pins.py <main-13.0.5.elf> <out.tsv>

The rows below are the source of truth for labels, kinds and notes; the guard bytes come from the ELF (.text), and the
host test `native_seams::tests::compared_rows_match_the_owned_elf` re-checks them on every `cargo test`. The facts are
wa-fixes-logs/a-prime-re.md; `runtime::roster::module_pins::SEAMS` lists the same labels.
"""
import struct
import sys

GROUP = "native_body_bank"
FEATURE = "native-body-bank-alpha"
# (label, offset, kind, note)
ROWS = [
    ("module_pins.nro_request", 0x17E4900, "Called",
     "called: `0x17e4900(R, kind)`, the NRO request `0x17e64d0` ends with: a node `kind * 0x2a` in R's NRO tree "
     "(`R+0x4b0/0x4b8/0x4c0`, handle at node+0x28) unless present, and one reference on the kind's module record."),
    ("module_pins.module_release", 0x22B6F60, "Called",
     "called: `0x22b6f60(manager, &handle)` under the manager lock `+0x60`: refs `record+0x134` - 1; at 0 `+0x133 = 1`, "
     "command 5 (unload) queued and the record erased from the map `+0x38/+0x40/+0x48`."),
    ("module_pins.mutex_lock", 0x39C1A40, "Called",
     "called: the import stub `0x17e4900` and `0x22b6f60` lock the module manager's mutex `manager+0x60` with."),
    ("module_pins.mutex_unlock", 0x39C1A50, "Called",
     "called: the import stub that unlocks it."),
    ("module_pins.tree_clear", 0x17E48A0, "Relied",
     "Loader state 3's `0x17e48a0(*(R+0x4b8))`: each node's handle (`+0x28`, 40 bits) goes to `0x22b6f60` with the "
     "manager `**(main+0x5327cd0)`; a kept group's kinds lose R's reference here."),
    ("module_pins.found_path", 0x17E4974, "Relied",
     "`0x17e4900` takes a found record's reference: manager `**(main+0x5327cd0)`, lock `+0x60`, map root `+0x40`, node "
     "key `& 0xffffffffff`, record node+0x28, `refs +0x134 += 1`. The pin takes one the same way."),
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
