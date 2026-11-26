#!/usr/bin/env python3
"""Write `data/tables/rows_cpu_level.ROWS.tsv` (rc.6, the mod's CPU level) from the owned 13.0.5 main ELF.

    python3 -I tools/gen_rows_cpu_level.py <main-13.0.5.elf> <out.tsv>

The rows below are the source of truth for labels, kinds and notes; the guard bytes come from the ELF (.text or .rodata),
and the host test `native_seams::tests::compared_rows_match_the_owned_elf` re-checks them on every `cargo test`.
"""
import struct
import sys

GROUP = "cpu_level"
SCOPES = "Scopes(Scopes::TRAINING.0 | Scopes::LOBBY_WAITING.0)"
# (label, offset, kind, read_only, per_frame, note)
ROWS = [
    ("cpu_level.ai_set_rank", 0x6B8400, "SeamKind::Called", False, True,
     "called: the AI rank setter (controller, rank). `ldr x21,[x20,#0x58]; ldr w8,[x21,#0xc0c]; cmp w8,w19; b.eq`: the AI is "
     "[controller+0x58], its rank is +0xc0c, and an unchanged rank returns at once. Every native rank writer calls it and then "
     "stores the same byte at FE+0x37 (the rows below); the step does the same for the CPUs the setting targets. The pilot "
     "calls it too (`cpu_pilot.ai_set_rank`); nothing hooks it."),
    ("cpu_level.rank_table", 0x4470DC0, "SeamKind::Relied", True, False,
     ".rodata: the ranks of Lv 2..=9, `[15, 21, 31, 42, 48, 60, 75, 100]` as i32 (Lv 1 is rank 0). `model::cpu::level::"
     "rank_for` refuses every level when the live table differs."),
    ("cpu_level.training_rank_choice", 0x155C4E0, "SeamKind::Relied", True, False,
     "Training's CPU apply (`0x155c020`) reads the pause menu [[[[[[main+0x52c3420]+8]]+0xb38]]+0x210]+8]+0x80 (the chain of "
     "`roster_cpu_quantity_root`) and its CPU Behavior +0xb4c: 8 (Control) or 2 (CPU Lv. n) take the rank of Training's "
     "level (`ldr w19,[x8,#0x1a0]`), every other behavior rank 100. The step targets exactly those two."),
    ("cpu_level.training_rank_write", 0x155CE8C, "SeamKind::Relied", False, False,
     "The same apply's write: entry = [FighterManager+0x20+8i], `ldr x0,[entry,#0x4178]` (the controller), `bl 0x6b8400`, "
     "`strb w20,[entry,#0x37]`. The step writes the pair the same way, after this native write."),
    ("cpu_level.training_level_init", 0x13E895C, "SeamKind::Relied", True, False,
     "Each Training entry builds the pause menu (`0x13e7c00`) and stores Training's own CPU level, `str w9,[x8,#0xb50]` "
     "(index 0..=8 from the CPU's select-screen rank through the thresholds 16/22/32/43/49/61/76/101). The notice compares "
     "this value + 1."),
    ("cpu_level.training_level_label", 0x1BBBF80, "SeamKind::Relied", True, False,
     "The menu's CPU Behavior label: value 2 prints `mel_training_cp_lv_n` with `+0xb50 + 1` (`ldr w9,[x0,#0xb50]; add w3,"
     "w9,#1`): CPU (Lv. n) is behavior 2 and +0xb50 is the level index."),
    ("cpu_level.lobby_partner_spawn", 0x151D130, "SeamKind::Relied", False, False,
     "Waiting-room practice: a partner record (entry at +0xc) spawns at min(+0x19, +0x1d): `bl 0x6b8400; strb w21,"
     "[entry,#0x37]`. The step's lobby entries 4..=6 are these records' entries."),
    ("cpu_level.lobby_partner_start", 0x151D6B0, "SeamKind::Relied", False, False,
     "Waiting-room practice start (`0x151d370`): every partner gets the param rank for the partner count (+0x1d and +0x10), "
     "`bl 0x6b8400; strb w21,[entry,#0x37]`."),
    ("cpu_level.lobby_rank_up", 0x151C360, "SeamKind::Relied", False, False,
     "Waiting-room practice rank-up from the record's accumulator (`0x151c2f0`): `bl 0x6b8400; strb w19,[entry,#0x37]`. "
     "The step's override returns the next frame."),
    ("cpu_level.lobby_rank_event", 0x151DA84, "SeamKind::Relied", False, False,
     "Waiting-room practice rank-up per event (`0x151da00`, called from 0x1362768/0x1362e7c/0x13631fc): `bl 0x6b8400; "
     "strb w20,[entry,#0x37]`."),
    ("cpu_level.control_select", 0x64FC60, "SeamKind::Relied", True, False,
     "Fighter control select reads FE+0x35 into the control setter and FE+0x37 into `0x6b8400`: why the step writes FE+0x37 "
     "with the rank, so a native re-select keeps the setting's rank."),
]

# Added to the table for rc.14 Hide CPU (068a3c01), in this generator since P7b.
HIDE_CPU_ROWS = [
    ("hide_cpu.visibility", 0x208C0C0,
     "called: VisibilityModule::set_whole(accessor, bool), the model; the same native the hidden tech animation calls "
     "(`ldr x0,[x0,#0x150]; ldr x8,[x0]; ldr x2,[x8,#0xc0]; and w1,w1,#1; br x2`). "
     "Owned 13.0.5 main ELF sha256 0789f0a4ae9652f3."),
    ("hide_cpu.effects", 0x2017A70,
     "called: EffectModule::set_whole(accessor, bool), every effect of the body "
     "(`ldr x0,[x0,#0x140]; ldr x8,[x0]; ldr x2,[x8,#0x1a8]; and w1,w1,#1; br x2`). "
     "Owned 13.0.5 main ELF sha256 0789f0a4ae9652f3."),
    ("hide_cpu.shadow", 0x2072AF0,
     "called: ShadowModule::set_draw_status(accessor, bool), the body's shadow "
     "(`ldr x0,[x0,#0x180]; ldr x8,[x0]; ldr x2,[x8,#0x50]; and w1,w1,#1; br x2`). "
     "Owned 13.0.5 main ELF sha256 0789f0a4ae9652f3."),
    ("hide_cpu.hurtbox", 0x203DB20,
     "called: HitModule::set_whole(accessor, HitStatus, i32), every hurtbox: status 3 (off) hides them, 0 (normal) gives "
     "them back; the second word is the part filter, 0 for all (`ldr x0,[x0,#0xb0]; ldr x8,[x0]; ldr x3,[x8,#0xd0]; "
     "br x3`). Owned 13.0.5 main ELF sha256 0789f0a4ae9652f3."),
    ("hide_cpu.jostle", 0x2040D40,
     "called: JostleModule::set_status(accessor, bool), the push between bodies: off, a fighter walks through the body "
     "(`ldr x0,[x0,#0x118]; ldr x8,[x0]; ldr x2,[x8,#0x48]; and w1,w1,#1; br x2`). "
     "Owned 13.0.5 main ELF sha256 0789f0a4ae9652f3."),
    ("hide_cpu.attacks", 0x1FD00C0,
     "called: AttackModule::clear_all(accessor), ends every hitbox the body has out, so a frozen attack cannot hit "
     "(`ldr x0,[x0,#0xa0]; ldr x8,[x0]; ldr x1,[x8,#0x50]; br x1`). "
     "Owned 13.0.5 main ELF sha256 0789f0a4ae9652f3."),
    ("hide_cpu.stop", 0x2089260,
     "called: StopModule::set_hit_stop_frame(accessor, i32 frames, bool), the game's own hitlag: the body's status, "
     "motion and kinetics wait while it counts down (`ldr x0,[x0,#0x90]; ldr x8,[x0]; ldr x3,[x8,#0xd0]; "
     "and w2,w2,#1; br x3`). Owned 13.0.5 main ELF sha256 0789f0a4ae9652f3."),
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
    rows = [(label, at, kind, read_only, per_frame, note, GROUP, SCOPES)
            for label, at, kind, read_only, per_frame, note in ROWS]
    rows.extend((label, at, "SeamKind::Called", False, True, note, "hide_cpu", "Scopes::TRAINING")
                for label, at, note in HIDE_CPU_ROWS)
    for label, at, kind, read_only, per_frame, note, group, scopes in rows:
        fields = [f'"{label}"', f'"{group}"', f"{at:#x}", kind, window(elf, secs, at).hex(), '""', scopes,
                  str(read_only).lower(), str(per_frame).lower(), "Sharing::None",
                  '"' + note.replace('\\', '\\\\').replace('"', '\\"') + '"']
        lines.append("\t".join(fields))
    open(sys.argv[2], "w").write("\n".join(lines) + "\n")
    print("wrote", sys.argv[2], len(rows), "rows")


if __name__ == "__main__":
    main()
