#!/usr/bin/env python3
"""Write `src/model/seams/native_seams/rows_game_speed.rs` (rc.8, the game speed bind) from the owned 13.0.5 main ELF.

    python3 -I tools/gen_rows_game_speed.py <main-13.0.5.elf> <out.rs>

The rows below are the source of truth for labels, kinds and notes; the guard bytes come from the ELF (.text), the host
test `native_seams::tests::compared_rows_match_the_owned_elf` re-checks them on every `cargo test`, and
`model::game_speed`'s tests decode every offset the runtime uses from these guard words. Same tool as
`tools/gen_rows_cpu_level.py`; the design is `docs/design/rc8-binds.md`.
"""
import hashlib
import struct
import sys

GROUP = "game_speed"
SCOPES = "Scopes(Scopes::TRAINING.0 | Scopes::LOBBY_WAITING.0)"
# (label, offset, kind, read_only, per_frame, note)
ROWS = [
    ("game_speed.speed_store_text", 0x1BBAF10, "SeamKind::Relied", True, False,
     "The Training menu setter (`0x1bbacc0`, item 7) for an unchanged value: `add x21,x19,#0x538` (the Speed row's value "
     "pane), `str w20,[x19,#0xb54]`, `bl 0x1bbbd30(menu, value, pane)`: the store and the text with no slide and no sound. "
     "The bind writes the same two things the same way."),
    ("game_speed.speed_text", 0x1BBBD30, "SeamKind::Called", False, False,
     "called: value -> text (`mel_training_speed*`) into the pane in x2: `sub w8,w1,#2; cmp w8,#4; b.hi` (default x1), "
     "jump table `0x44f8a24`, `0x377a120(pane, label)`. Its only callers are the setter's two paths: the menu never "
     "refreshes the row's text by itself."),
    ("game_speed.training_chain", 0x155B4A4, "SeamKind::Relied", True, False,
     "The speed code of `FUN_0155a000` (x25 the Training mode object) finds Training's menu: `ldr w19,[x25,#0x194]` "
     "(the applied value), `[[[main+0x52c3420]+8]]` then `+0xb38`; no object, and it applies 0, which sets no speed. "
     "The game reaches this code only on the pause/resume side (`game_speed.mode_gate`)."),
    ("game_speed.training_value", 0x155B574, "SeamKind::Relied", True, False,
     "The same apply reads the pause menu's Speed: `[[[[x20+0xb38]]+0x210]+8]+0x80`, `ldr w8,[x8,#0xb54]`, then on a "
     "change `str w8,[x25,#0x194]` (edge-triggered): the field the bind writes is the one the game applies."),
    ("game_speed.slot_free", 0x155B5B0, "SeamKind::Relied", True, False,
     "The apply frees the battle's previous slow slot: slot `[x25+0x1b4]` (0xff none) in the table `*(main+0x52c3610)`, "
     "`ldrb [x10,#0x20]!` in_use, `strb wzr` in_use, `strb #1,[x9,#0x21]` divisor, `strb #1,[x8,#0x40]` dirty. The "
     "lobby's free is these steps."),
    ("game_speed.slot_take", 0x155BBA0, "SeamKind::Relied", True, False,
     "The apply's take (here 1/4): `strb #1,[x9]` in_use, `strb #4,[x9,#0x21]` divisor, `strb #1,[x8,#0x40]` dirty, "
     "slot to `[x25+0x1b4]`. The lobby's take is these steps, in this order."),
    ("game_speed.hud_icon", 0x13E54F8, "SeamKind::Relied", True, True,
     "The HUD's speed indicator refresh (x19 the HUD data object): `[[x19+8]+0x80]+0xb54` against 1 and the shown flag "
     "`ldrb w10,[x19,#0x231]`; shown at x1: `out_speed` (`add x0,x8,#0x178`, `bl 0x377aee0`), `strb wzr,[x19,#0x231]`. "
     "It runs only in the menu state (`game_speed.hud_state`), not every frame: the bind runs it after a closed-menu "
     "step."),
    ("game_speed.hud_icon_show", 0x13E5530, "SeamKind::Relied", True, True,
     "The same refresh, not shown and not x1: `in_speed` (`add x0,x8,#0x168`, `bl 0x377aee0`), then `orr w8,wzr,#1`, "
     "`strb w8,[x19,#0x231]`."),
    ("game_speed.hud_state", 0x13AF660, "SeamKind::Relied", True, False,
     "The HUD data object's constructor fills its state table (`+0x40 + 16 * state`): state 5 (play) `0x13e5110` at "
     "`+0x90`, state 6 `0x13e5470` (the refresh's function) at `+0xa0`. Play never reads the Speed; state 6 is the "
     "Training menu state the Speed is edited in (entered from play at `.13e53d0`)."),
    ("game_speed.hud_data", 0x13AF808, "SeamKind::Relied", True, False,
     "The same constructor stores that object at `+0x210` of its owner (`str x20,[x19,#0x210]`): the chain's "
     "`[[[[root+8]]+0xb38]]+0x210` is the refresh's x19, whose `[[+8]+0x80]` is the pause menu."),
    ("game_speed.anim_play", 0x377AEE0, "SeamKind::Called", False, False,
     "called: `0x377aee0(anim, rate)` plays one layout animation (`ldr x8,[x0,#8]`, the layout's animator list at "
     "`+0x80`), as the refresh plays `in_speed`/`out_speed` with rate 1.0. The bind calls it on the UI tick, after the "
     "holder's animation update."),
    ("game_speed.rate_request", 0x1FCA5C0, "SeamKind::Relied", True, True,
     "`BattleObjectSlow__rate_request_impl`: `ldr x0,[main+0x52c3610]`, `ldrb w8,[x0,#0x40]` dirty -> `bl 0x14a2cd0` "
     "(max divisor), else the default byte: a slot in the table slows the battle objects."),
    ("game_speed.max_divisor", 0x14A2CD0, "SeamKind::Relied", True, True,
     "The largest divisor over the in-use slots: `ldrb [x0,#0x20]` in_use, `ldrb [x8,#0x21]` divisor, pairs of two "
     "bytes."),
    ("game_speed.table_create", 0x1345FDC, "SeamKind::Relied", True, False,
     "A table is created if `*(main+0x52c3610)` is 0 (a live one is kept): every slot `{0, 1}` (`0x0100010001000100` "
     "from +0x20), dirty 1 at +0x40. A new battle's table holds no slot of the plugin."),
    ("game_speed.table_destroy", 0x134D52C, "SeamKind::Relied", True, False,
     "The battle teardown frees the table and stores 0 at `main+0x52c3610`, beside the BattleObjectManager teardown: the "
     "practice's table goes with the practice."),
    # The Training apply the bind does itself (the game runs its speed code only on the pause/resume side; Eden
    # 2026-11-11 out-diag6, QCF 09:12Z/09:18Z), and the way to the object it applies to.
    ("game_speed.mode_key", 0x135CA34, "SeamKind::Relied", True, False,
     "The game's own per-update lookup of the Training mode object (`FUN_0135ca00`, every battle task entry): "
     "`[[main+0x52c3910]]` (the object registry), key `0x1022590708` (`mov`/`movk`/`movk`), `bl 0x15435c0` (the "
     "registry find). The battle setup registers the mode factory's object under it (`.14ed3b4`-`.14ed3c4`; QCF "
     "11:16Z). Key `0x1308abad79` holds another object, vtable main+0x5070380 (Eden 2026-11-11 out-mode). The bind "
     "walks the same registry for the same key (Eden out-key: x25, vtable main+0x506fd10)."),
    ("game_speed.mode_find", 0x15435CC, "SeamKind::Relied", True, False,
     "The registry find (a libc++ unordered_map): bucket count `[map+8]` (0: none), hash `key & 0xffffffffff`, bucket "
     "`hash & (n-1)` for a power of two else `hash % n`, `[[map] + 8*bucket]` the bucket's head, `[head]` its first node."),
    ("game_speed.mode_walk", 0x154365C, "SeamKind::Relied", True, False,
     "The node walk: hash at `+8` (a node of another bucket ends the walk), key at `+0x10` compared under the same "
     "40-bit mask, the next node at `+0`."),
    ("game_speed.mode_found", 0x15436A4, "SeamKind::Relied", True, False,
     "A found node: `ldp x9, x19, [x10, #0x18]`, the object at `+0x18` and its control block at `+0x20` (the bind takes "
     "the object only, no reference)."),
    ("game_speed.mode_vtable", 0x1555ED0, "SeamKind::Relied", True, False,
     "The Training mode object's constructor stores its class vtable `main+0x506fd10` at `+0`: the bind writes only to "
     "an object with this vtable (in other modes the same key holds another mode's object)."),
    ("game_speed.mode_gate", 0x155A08C, "SeamKind::Relied", True, False,
     "Why the bind applies the speed itself: `FUN_0155a000` skips its speed code to `.155a6d0` while `HUD+0x232` is set "
     "(each running update sets it) and `[[menu+0x7b0]]+0x25c == 0` or `HUD+0x240 == 2` and `FUN_0159d380()` is false: "
     "in closed-menu play the menu's Speed is never applied."),
    ("game_speed.mode_state", 0x155B4D8, "SeamKind::Relied", True, False,
     "The speed code reads the menu's value only outside one state: `[main+0x532f8b0]` -> `+8`, `+0x138` set and byte "
     "`+0x2b8 == 11` means it applies 0. The bind applies nothing then either."),
    ("game_speed.step_clear", 0x155B5F0, "SeamKind::Relied", True, False,
     "On a change the apply clears bit 2 of `main+0x52b92ec` (the 1 Frame step bit, behind the static guard byte "
     "`main+0x52b92e0`) and, for a value other than 1 Frame, the byte at `[[main+0x53398d8]+8]`. The bind does the same."),
    ("game_speed.rates_slow", 0x155B654, "SeamKind::Relied", True, False,
     "x1 and 1 Frame: rate code `main+0x593b444 = 0`, updates `main+0x523d004 = 60`; 2/3: code 4, updates 120, then a "
     "slot from the table's first free one."),
    ("game_speed.rates_half", 0x155B75C, "SeamKind::Relied", True, False,
     "1/2 and 1/4: code 0, updates 60, then a slot (divisor 2 or 4)."),
    ("game_speed.rates_fast", 0x155B85C, "SeamKind::Relied", True, False,
     "x1.5: code 6, updates 180, then a slot with divisor 2."),
    ("game_speed.slot_divisors", 0x155BB60, "SeamKind::Relied", True, False,
     "The take's divisors 2 and 3: `strb #1` in_use, `orr w11,wzr,#2` / `#3`, then `strb w11,[x9,#0x21]` "
     "(`game_speed.slot_take` holds 4, dirty and `+0x1b4`)."),
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
        "//! rc.8 game speed seam rows, generated by `tools/gen_rows_game_speed.py` from the owned 13.0.5 main ELF",
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
