#!/usr/bin/env python3
"""Write `data/tables/rows_speed_frame.ROWS.tsv` (rc.15: 1 Frame on the game speed bind) from the owned 13.0.5 main ELF.

    python3 -I tools/gen_rows_speed_frame.py <main-13.0.5.elf> data/tables/rows_speed_frame.ROWS.tsv

The rows below are the source of truth for labels, kinds and notes; the guard bytes come from the ELF (.text or .rodata),
the host test `native_seams::tests::compared_rows_match_the_owned_elf` re-checks them on every `cargo test`, and
`speed_frame_rows_tests` decodes every offset and constant the runtime uses from these guard words. The design is
`docs/design/rc15-speed-frame.md`.
"""
import struct
import sys

GROUP = "speed_frame"
SCOPES = "Scopes(Scopes::TRAINING.0 | Scopes::LOBBY_WAITING.0)"
HOOK = "Hook { form: HookForm::Inline, placement: Placement::Mid }"
# (label, offset, kind, per_frame, note)
ROWS = [
    ("speed_frame.entry_hold", 0x135CA38, HOOK, True,
     "Mid-function inline hook on `ldr x8,[x8,#0x910]` in the battle task entry `FUN_0135ca00`, two words after it loaded "
     "the paused word into w22 (`ldr w22,[main+0x52b92e8]`, `lobby_speed.paused_flag`) and with `this` in x20 "
     "(`mov x20,x0`). For the waiting area's entry (`[x20]` = vtable `main+0x506d4d8`) of the live practice at 1 Frame, "
     "with no step due, the callback sets x22 = 1 for this call: the entry skips its slow step (`cbnz w22`, "
     "`speed_frame.entry_skip`) and runs the scheduler paused (`cmp w22,#0; cset w1,ne`, `lobby_speed.task_entry`). It "
     "writes no game memory. 8-byte aligned: a far form replaces 0x135ca38..0x135ca48 (ldr x8; ldr x1,[x8]; mov x2; movk "
     "x2: no PC-relative word). No direct branch in .text targets 0x135ca3c..0x135ca48, and `FUN_0135ca00` "
     "(0x135ca00..0x135cc44) has no x16/x17 operand (2026-11-21 scan)."),
    ("speed_frame.entry_skip", 0x135CA94, "Relied", True,
     "The same entry after its registry lookup: `cbnz w22` (`.135cab4`) jumps over the whole slow step to the scheduler "
     "call: a held entry advances no logic frame, no sub-counter and no rate."),
    ("speed_frame.entry_skip_last", 0x135CB5C, "Relied", True,
     "The same entry's other path, after it drops the last reference of the looked-up object: `cbnz w22` (`.135cb74`) "
     "to the scheduler call, else back to the slow step."),
    ("speed_frame.walker_paused", 0x1358B94, "Relied", True,
     "The registry walker's per-object callback `FUN_01358ad0`: it skips an object only while the global paused word "
     "`[main+0x52b92e8]` is set and the object's `+0xb` byte is set (`cbz w8` then `ldrb w8,[x20,#11]; cbnz`). The hold "
     "leaves the global word 0, so every registry object (the Quickplay room `0x506fec0` and the mode objects) ticks on "
     "a held entry."),
    ("speed_frame.pause_sync", 0x555DA0, "Called", False,
     "called: `FUN_00555da0()`, the pause sync: when the paused word `[main+0x52b92e8]` differs from the requests "
     "`[main+0x52b92ec]` it runs the pause/resume side (`0x555f20`, `0x35a0460`) and copies the requests in. Its only "
     "callers are Training's mode object (`.1556b84`, `.1556c78`, `.155a21c`, `.155a258`, `.155a718`) and `.22deb68`."),
    ("speed_frame.step_mode", 0x1556A00, "Relied", True,
     "The Training mode object's 1 Frame step `FUN_015569e0`: `ldr w9,[x0,#0x194]; cmp w9,#2` (the applied speed is 1 "
     "Frame), then `b.ne` past it all: the game pauses and steps only while the applied value is 2."),
    ("speed_frame.step_input", 0x1556A40, "Relied", True,
     "The same step's press check: `adrp x21,0x509e000; add x21,x21,#0xbd0` (the button functor's vtable `0x509ebd0`), "
     "`mov w3,wzr` (mode 0: newly pressed) and `bl 0x177e8f0`, which tests `just_down & current` of each of the 10 "
     "controllers against the functor's mask."),
    ("speed_frame.step_hold", 0x1556AD8, "Relied", True,
     "The same step's held check: `orr w3,wzr,#0x1` (mode 1: held) and `bl 0x177e8f0` with the same functor."),
    ("speed_frame.step_repeat", 0x1556B10, "Relied", True,
     "Held: `ldr w8,[x19,#0x1bc]; add w8,w8,#1; cmp w8,#18; str; b.lo` (still paused): from the 18th held update on the "
     "game runs while the button is held."),
    ("speed_frame.step_run", 0x1556B64, "Relied", True,
     "A step runs: `ldr w8,[main+0x52b92e8]` (the paused word), `tbz w8,#2` (not Training's pause: nothing), then request "
     "bit 2 cleared (`ldr w9,[main+0x52b92ec]; and w9,w9,#0xfffffffb; str`) and `bl 0x555da0`. The bind does the same "
     "when it leaves 1 Frame."),
    ("speed_frame.step_pause", 0x1556C64, "Relied", True,
     "No step: `tbnz w8,#2` (paused already), else request bit 2 set (`orr w9,w9,#0x4`) and `bl 0x555da0`."),
    ("speed_frame.step_mask", 0x177FE20, "Relied", True,
     "The step button functor's mask (`0x509ebd0` slot 6): controller `[main+0x5339860 + 8*index]`, its style "
     "`[[c+0x10]+0x9c]` 1..7, the mask `[main+0x45434f0 + 4*(style-1)]` (`speed_frame.step_buttons`)."),
    ("speed_frame.step_buttons", 0x45434F0, "Relied", False,
     "The step button masks by controller style 1..7: `0x400` (ZL) for styles 1, 2, 5, 6; `0x100` (the raw L bit) for 3, "
     "4 and 7, the GameCube pad's L trigger."),
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
    for name in (".text", ".rodata"):
        addr, off, size = secs[name]
        if addr <= at and at + 64 <= addr + size:
            return elf[off + at - addr: off + at - addr + 64]
    raise SystemExit(f"{at:#x} is in neither .text nor .rodata")


def main():
    elf = open(sys.argv[1], "rb").read()
    secs = sections(elf)
    lines = ["#! Seam\tlabel\tgroup\toffset\tkind\tguard\tfeature\tscopes\tread_only\tper_frame\tsharing\tnote"]
    for label, at, kind, per_frame, note in ROWS:
        read_only = "true" if kind == "Relied" else "false"
        fields = [f'"{label}"', f'"{GROUP}"', f"{at:#x}", f"SeamKind::{kind}", window(elf, secs, at).hex(), '""', SCOPES,
                  read_only, str(per_frame).lower(), "Sharing::None", '"' + note.replace('"', '\\"') + '"']
        lines.append("\t".join(fields))
    open(sys.argv[2], "w").write("\n".join(lines) + "\n")
    print("wrote", sys.argv[2], len(ROWS), "rows")


if __name__ == "__main__":
    main()
