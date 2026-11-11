#!/usr/bin/env python3
"""Write `src/model/seams/native_seams/rows_lobby_speed.rs` (rc.8 R2: the waiting-room practice's 2/3 and x1.5) from the
owned 13.0.5 main ELF.

    python3 -I tools/gen_rows_lobby_speed.py <main-13.0.5.elf> <out.rs>

The rows below are the source of truth for labels, kinds and notes; the guard bytes come from the ELF (.text), the host
test `native_seams::tests::compared_rows_match_the_owned_elf` re-checks them on every `cargo test`, and
`model::lobby_speed`'s tests decode every offset the runtime uses from these guard words. Same tool as
`tools/gen_rows_game_speed.py`; the research is QCF's `RESEARCH-rc8-speed.md` section 9.
"""
import hashlib
import struct
import sys

GROUP = "lobby_speed"
SCOPES = "Scopes::LOBBY_WAITING"
HOOK = "SeamKind::Hook { form: HookForm::Inline, placement: Placement::Mid }"
# (label, offset, kind, read_only, per_frame, note)
ROWS = [
    ("lobby_speed.class_update", 0x151B170, HOOK, False, True,
     "Mid-function inline hook on `mov x0,x20` in the waiting area's own update `FUN_0151b150` (vtable `0x506d4d8` `+0x50`, "
     "its only reference), after its prologue: the battle task entry `FUN_0135ca00` calls it after the slow step and the "
     "battle scheduler of the same update (`lobby_speed.task_entry`). The callback paces the 2/3 slot at the slow periods' "
     "ends and runs x1.5's extra scheduler pass; it never changes the context. 8-byte aligned: a far form replaces "
     "0x151b170..0x151b180 (mov x0,x20; mov x19,x1; bl 0x14f8940; adrp x8: the bl and the adrp relocated). No direct "
     "branch in .text targets 0x151b174..0x151b180, the function 0x151b150..0x151b354 has no x16/x17 operand, NZCV is dead "
     "here (the bl after it). Card scan 2026-11-11 over the rc.7 card copy (722525a5): no plugin refers to it."),
    ("lobby_speed.task_entry", 0x135CBFC, "SeamKind::Relied", True, True,
     "The battle task entry `FUN_0135ca00` after its slow step: `str w9,[main+0x523d138]` (the rate), then "
     "`ldr x8,[main+0x52c33d8]; ldr x0,[x8]` (the scheduler), `cmp w22,#0; cset w1,ne; bl 0x137b3f0` (the scheduler's "
     "pass, paused when w22), then the class's own update `ldr x8,[x20]; ldr x8,[x8,#0x50]; blr x8`: the hooked update "
     "runs after this update's world tasks."),
    ("lobby_speed.paused_flag", 0x135CA14, "SeamKind::Relied", True, True,
     "The same entry reads the paused word: the static guard byte `main+0x52b92e0` (`ldarb`, `tbz` skips the whole entry "
     "until it is set), then `ldr w22,[main+0x52b92e8]`; non-zero runs just the tasks marked to run when paused. The "
     "extra pass needs it at 0."),
    ("lobby_speed.slow_step", 0x135CB08, "SeamKind::Relied", True, True,
     "The slow step with one update per period (`ldrb w8,[main+0x52c5188]; cmp w8,#1`): base `main+0x52c5184`, the logic "
     "frame counter `+0` += 1, the target `+5` (0x52c5189) latched into the current `+4`, the sub-counters `+6`/`+7` "
     "cleared (`strh wzr`), the rate `+0xc`, the second counter `+8` += 1. An extra world step at x1 is these two "
     "increments."),
    ("lobby_speed.slow_period", 0x135CB7C, "SeamKind::Relied", True, True,
     "The slow step with longer periods: the sub-counter `main+0x52c518a` += 1, and at `>= N` the counter `+0` += 1, "
     "`strb wzr,[x9,#6]` (sub 0), the target latched (`strb w8,[x9,#4]`): a period ends on the update whose step leaves "
     "the sub-counter at 0, and the next period's length is the target at that moment. The 2/3 pace reads these."),
    ("lobby_speed.sched_setup", 0x137B408, "SeamKind::Relied", True, True,
     "The battle scheduler `FUN_0137b3f0(S, paused)`: `mov x19,x0` (S), `mov x20,xzr` (phase 0), `mov w21,#-1`, "
     "`orr w22,wzr,#0x30` (a phase's stride), `tbz w1,#0` to the unpaused walk."),
    ("lobby_speed.sched_phase", 0x137B500, "SeamKind::Relied", True, True,
     "The unpaused walk, per phase: `and w8,w20,#0x7fffffff; sub w9,w8,#1; cmp w9,#8; b.hs` (phases 0 and 9 store -1, 10 "
     "ends), `madd x10,x20,x22,x19; str w8,[x19]` (the current phase), the deque at `S+0x30*phase`: start `+0x78`, "
     "`ldp` begin/end `+0x60`/`+0x68`, `lsr #5` + `and` (256 16-byte elements per block)."),
    ("lobby_speed.sched_entry", 0x137B540, "SeamKind::Relied", True, True,
     "The same walk: size `ldr x10,[x10,#0x80]`, the end element, then per element `ldr x8,[x24]` (the entry) and "
     "`ldrsw x9,[x8,#8]` (its group), `cmp w9,#9; b.gt` (no mask above 9)."),
    ("lobby_speed.sched_task", 0x137B580, "SeamKind::Relied", True, True,
     "The same walk: the group's mask `ldr w9,[S+4*group+4]; cbnz` (masked: skipped), the task `ldr x0,[x8,#0x40]` "
     "(null aborts), its tick `ldr x8,[x0]; ldr x8,[x8,#0x30]; blr x8`, the next element `add x24,x24,#16`, a block of "
     "0x1000 bytes."),
    ("lobby_speed.walker_tick", 0x1358970, "SeamKind::Relied", True, True,
     "The registry walker task's tick (`__func` vtable `0x5061b98`, slot `+0x30`): `ldr x8,[main+0x52c3910]` (the "
     "registry), the inner lambda `0x50621a0`, `bl 0x1543710`: every enabled registry object's update, the Quickplay room "
     "object's (`0x506fec0`, `FUN_01563520`) among them. The extra pass skips the task whose tick is this function."),
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
    lines = [
        "//! rc.8 R2 lobby speed seam rows, generated by `tools/gen_rows_lobby_speed.py` from the owned 13.0.5 main ELF",
        f"//! (sha256 {hashlib.sha256(elf).hexdigest()}); regenerate rather than hand-edit.",
        "use super::*;",
        "",
        "pub const ROWS: &[Seam] = &[",
    ]
    for label, at, kind, read_only, per_frame, note in ROWS:
        guard = ", ".join(f"0x{b:02x}" for b in window(elf, secs, at))
        lines += [
            f'    Seam {{ label: "{label}", group: "{GROUP}", offset: {at:#x},',
            f"           kind: {kind},",
            f"           guard: [{guard}],",
            f'           feature: "", scopes: {SCOPES}, read_only: {str(read_only).lower()}, '
            f"per_frame: {str(per_frame).lower()},",
            "           sharing: Sharing::None,",
            f'           note: "{note}" }},',
        ]
    lines.append("];")
    open(sys.argv[2], "w").write("\n".join(lines) + "\n")
    print("wrote", sys.argv[2], len(ROWS), "rows")


if __name__ == "__main__":
    main()
