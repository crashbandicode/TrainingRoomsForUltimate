#!/usr/bin/env python3
"""Regenerate `src/model/sites_hitbox.rs` rows from the owned 13.0.5 `main` ELF (SPEC-core-06 section 4).

    python3 -I tools/gen_hitbox_sites.py <main-13.0.5.elf> > /tmp/rows.rs

Prints the `Site` rows (64 guard bytes read from `.text` at each offset, which maps at 0) in the order of
`sites_hitbox::SITES`. `tests/core06_hitbox.rs` re-reads the ELF and compares every row, so a hand edit of
the table that drifts from the binary fails. Game bytes are read at run time from the operator's own dump and
are never committed apart from the 64-byte guards the playbook requires.
"""
import sys

# (name, offset, kind, note) -- kind is the Rust SeamKind expression.
HOOK = "SeamKind::Hook { form: HookForm::Replace, placement: Placement::Entry }"
CALLED = "SeamKind::Called"
RELIED = "SeamKind::Relied"

ROWS = [
    ("hitbox_attack", 0x2284B20, HOOK,
     "`app::sv_animcmd::ATTACK(lua_State*)`: the animation command that creates a hitbox. Replace hook at the entry: the plugin part runs first (it only reads the Lua stack and draws), then the original."),
    ("hitbox_catch", 0x22AC290, HOOK,
     "`app::sv_animcmd::CATCH(lua_State*)`: the animation command that creates a grab box. Replace hook at the entry, plugin part first."),
    ("hitbox_set_rebound", 0x2027BB0, HOOK,
     "`GrabModule::set_rebound(accessor, bool)`, a lua_bind thunk (`ldr x0,[x0,#0x158]; ldr x8,[x0]; ldr x2,[x8,#0x100]; and w1,w1,#1; br x2`) with the neighbouring thunks inside its 64-byte window, which boot compares before anything is written. Replace hook; the original runs first."),
    ("attack_is_attack", 0x1FD01C0, CALLED,
     "`AttackModule::is_attack(accessor, slot, bool)` (lua_bind thunk to vtable slot 0xc8): whether an attack slot is active (the slot's `+0x108` word is non-zero)."),
    ("attack_data", 0x1FD0520, CALLED,
     "`AttackModule::attack_data(accessor, slot, bool)` (lua_bind thunk to vtable slot 0x1e8): the slot's `AttackData` (stride 0x130 from the pointer at module `+0x68`). Layout in `model::hitbox::layout`, derived in `docs/test-provenance/core-06.md`."),
    ("effect_req_follow", 0x2017730, CALLED,
     "`EffectModule::req_follow(accessor, Hash40 effect, Hash40 joint, pos&, rot&, scale, follow, attribute, 0, 0, 0, 0, bool, bool)`: spawns the sphere (the game's `sys_shield` effect) attached to the hitbox's bone."),
    ("effect_kill_kind", 0x2017860, CALLED,
     "`EffectModule::kill_kind(accessor, Hash40 effect, bool, bool)`: removes the previous frame's spheres."),
    ("effect_set_rgb_partial_last", 0x2017C80, CALLED,
     "`EffectModule::set_rgb_partial_last(accessor, r, g, b)`: tints the sphere just requested."),
    ("effect_set_rate_last", 0x2017CE0, CALLED,
     "`EffectModule::set_rate_last(accessor, rate)`: plays the sphere just requested at 8x so it does not pulse."),
    ("motion_animcmd_set_sleep_effect", 0x2069140, CALLED,
     "`MotionAnimcmdModule::set_sleep_effect(accessor, bool)`: puts the fighter's animation-command effects to sleep or restores them."),
    ("sv_battle_object_module_accessor", 0x227DAF0, CALLED,
     "`app::sv_system::battle_object_module_accessor(lua_State*)`: the `BattleObjectModuleAccessor` of the object running the animation command."),
    ("hitbox_attack_stack", 0x2284CD0, RELIED,
     "Inside the `ATTACK` implementation (`main+0x2284b80`): the Lua stack read the plugin repeats, `ldr x8,[L,#0x20]; ldr x8,[x8]; ldr x9,[L,#0x10]; add x8,x8,#0x40` (slot of argument 4 = base + 0x10 * index, tested against the top at L+0x10, else the nil sentinel), then the value tags (3 float, 0x13 integer). Relied on, never written."),
    ("hitbox_catch_stack", 0x22AC2EC, RELIED,
     "Inside `CATCH`: the same Lua stack read for argument 3 (`add x8,x8,#0x30`), the size. Relied on, never written."),
]


def main(path):
    elf = open(path, "rb").read()
    text_off = 0x1000  # `.text` file offset of the owned ELF (maps at address 0)
    for name, offset, kind, note in ROWS:
        guard = elf[text_off + offset:text_off + offset + 64]
        assert len(guard) == 64
        print("    Site {")
        print(f'        name: "{name}",')
        print("        feature: Feature::Hitbox,")
        print(f"        offset: {offset:#x},")
        print(f"        kind: {kind},")
        print("        guard: [")
        for i in range(0, 64, 16):
            print("            " + ", ".join(f"0x{b:02x}" for b in guard[i:i + 16]) + ",")
        print("        ],")
        print(f'        note: "{note}",')
        print("    },")


if __name__ == "__main__":
    main(sys.argv[1])
