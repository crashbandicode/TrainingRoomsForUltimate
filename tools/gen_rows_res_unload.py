#!/usr/bin/env python3
"""Write `src/model/seams/native_seams/rows_res_unload.rs` (rc.6, operator option C: the release watch's read of the game's
resource unload) from the owned 13.0.5 main ELF.

    python3 -I tools/gen_rows_res_unload.py <main-13.0.5.elf> <out.rs>

The rows below are the source of truth for labels, kinds and notes; the guard bytes come from the ELF (.text), and the
host test `native_seams::tests::compared_rows_match_the_owned_elf` re-checks them on every `cargo test`. The facts are
LANE-OPUS-RC6-PRELOAD-PRIORITY-REPORT.md F6; `runtime::roster::res_unload::SEAMS` lists the same labels.
"""
import hashlib
import struct
import sys

GROUP = "res_unload"
SCOPES = "Scopes::ALL"
FEATURE = "native-body-bank-alpha"
# (label, offset, note)
ROWS = [
    ("res_unload.fs_global", 0x353F160,
     "The handle release `0x353f120` reads the service's table from `main+0x5332f20` (`adrp; ldr x20,[x8,#0xf20]`), its "
     "directory count +0x48 and directories +0x40 (0x48-byte entries, `mov w9,#0x48; madd`), and releases a directory "
     "whose flags bit 0 is set (`ldrb [x9,#8]; tbz #0`) through `0x353f5a0`. The watch scans the same table."),
    ("res_unload.directory_release", 0x3543330,
     "`0x3543330(directory, out)`: own index at +0 (`0xffffff` free), refs at +4 dropped atomically (`ldaxr`/`stlxr`); "
     "at <= 0 the index is queued, then each child folder the same way. A free only queues."),
    ("res_unload.file_release", 0x3540B30,
     "`0x3540b10(fs, filepath)` under the table lock: filepaths at fs+0x08, data at fs+0x10 (0x18 bytes: refs +8, used "
     "+0xc; count fs+0x1c); a used file whose refs fall to <= 0 joins the file queue fs+0x28 and sets fs+0x68."),
    ("res_unload.pass_entry", 0x353F760,
     "The unload pass `0x353f760(fs)`: `ldrb w8,[x0,#0x68]; cbz` returns when nothing is queued; it takes the table lock "
     "(`ldr x0,[x0]`) and drains the directory queue fs+0x50 and the file queue fs+0x28."),
    ("res_unload.pass_directory_reset", 0x353FA74,
     "The pass resets a queued directory after releasing its files: own index `0xffffff`, `stlr` refs 0, flags "
     "`&0xfe` then `&0xf9`, `stlrb` incoming 0, then `stlrb` state(+9) 0 last. The watch reads state with an acquire "
     "load first."),
    ("res_unload.pass_file_reset", 0x35402AC,
     "The pass's file reset after the free (`0x3540290`): data pointer 0, `stlr` refs 0, used 0, then `stlrb` "
     "state(+0xd) 0, then the data and filepath counts fs+0x24/fs+0x20 fall. The watch reads state with an acquire load "
     "first."),
    ("res_unload.update_thread", 0x3543580,
     "ResUpdateThread `0x3543550` (every 16 ms): when `svc+0x248 == 0` (nothing inflating) and fs+0x68 is set, it runs "
     "the unload pass under the service lock (`bl 0x353f760`). A free's unload ends within 0.5 s (Eden `pp-re-trace-1`)."),
    ("res_unload.inflate_thread", 0x3544ABC,
     "ResInflateThread `0x3544a70`: the same check at the top of its loop, then `svc+0x248 = 1` while it inflates."),
    ("res_unload.mallctl", 0x392EC40,
     "called: jemalloc's `mallctl(name, oldp, oldlenp, newp, newlen)` (`malloc_conf \"percpu_arena:percpu,"
     "dirty_decay_ms:1\"`): the five arguments spilled, then `bl 0x392d020` (initialisation; `mov w8,#0xb` EAGAIN). The "
     "bank's MainThread hook calls `mallctl(\"arena.4096.purge\", 0, 0, 0, 0)` once per settled release."),
    ("res_unload.mallctl_caller", 0x3959B00,
     "jemalloc's statistics printer calls the same function with a name: `adrp/add x0, \"arenas.dirty_decay_ms\"; "
     "x1/x2 out; mov x3,xzr; mov x4,xzr; bl 0x392ec40`."),
    ("res_unload.purge_ctl", 0x394C6C8,
     "`arena.<i>.purge`'s handler (the ctl node whose name slot `0x5296f90` relocates to \"purge\"): the index must fit "
     "32 bits (`ldr w8,[x8,#0xc]`), then `ldr w8,[x8,#0x8]`; `bl 0x3946a40` (tsdn); `mov w2,#0x1; bl 0x394d940` "
     "(decay every page, all = true)."),
    ("res_unload.arena_decay", 0x394D940,
     "`arena_i_decay(tsdn, index, all)`: `subs w8,w8,#0x1, LSL #12` takes index 4096 (`MALLCTL_ARENAS_ALL`) as every "
     "arena, each purged through `0x3931510`; the pages go back through `0x37ca7f0` and `nn::os::FreeMemoryPages`."),
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
    lines = [
        "//! rc.6 release-watch seam rows (operator option C, the game's resource unload), generated by",
        "//! `tools/gen_rows_res_unload.py` from the owned 13.0.5 main ELF",
        f"//! (sha256 {hashlib.sha256(elf).hexdigest()}); regenerate rather than hand-edit.",
        "use super::*;",
        "",
        "pub const ROWS: &[Seam] = &[",
    ]
    for label, at, note in ROWS:
        guard = ", ".join(f"0x{b:02x}" for b in window(elf, secs, at))
        lines += [
            f'    Seam {{ label: "{label}", group: "{GROUP}", offset: {at:#x},',
            f"           kind: SeamKind::{'Called' if note.startswith('called:') else 'Relied'},",
            f"           guard: [{guard}],",
            f'           feature: "{FEATURE}", scopes: {SCOPES}, read_only: true, per_frame: true,',
            "           sharing: Sharing::None,",
            '           note: "' + note.replace('\\', '\\\\').replace('"', '\\"') + '" },',
        ]
    lines.append("];")
    open(sys.argv[2], "w").write("\n".join(lines) + "\n")
    print("wrote", sys.argv[2], len(ROWS), "rows")


if __name__ == "__main__":
    main()
