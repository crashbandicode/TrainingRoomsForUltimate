# U4b: the game's own DI / SDI effect textures on picture panes (stream u4b, 2026-11-02)

SPEC-core-07 section 4.6 (V4-V6), question 17. Static reverse engineering of the owned 13.0.5 `main`
(`research/game-13.0.5/main-13.0.5.elf`, Ghidra 12.1.4 headless over a copy of the project, scripts and outputs in the local
scratch folder `u4b-scratch/`, game-derived and never committed); no upstream code opened. Offsets are `main`-relative.
Evidence for every row is the decompilation or disassembly named; every instruction window the plugin relies on has a guard row in
`src/model/sites_u4b.rs` and a check in `tests/u4b_sites.rs`.

## Answer

The plugin does not have to read `ef_common.eff` at all (no background read, no 4.2 s, no BNTX decode, no GPU upload). The game
already holds the file: its effect manager (`lib::EffectManager`) loaded it, kept the bytes, and its particle system (`nn::vfx`
resource) built every texture of the file's `BNTX` **in place** (memory pool, texture, texture view) and registered each view in the
same descriptor pool the UI draws from. The plugin walks those resident structures (V6: yes, resident), reads the emitters' colours
from the resident bytes (V4, no `.eff` parser beyond the `EFFN` name table), and gives the game's own UI texture code the texture
view the game built for each emitter texture (`texture::load_resident`), which registers it for a picture pane with the game's own
registration function. Binding and release are the existing `texture::bind` / `unbind` / `retire` of stream u4. Effect mods show,
because the resident data is what the game draws. Until the file is resident (or if any guard or shape check fails) the walk answers
`Err` and the chip keeps the clean room's arrow.

## What the game does (RE)

| What | Function (main+) | Evidence |
| --- | --- | --- |
| effect manager instance | slot `0x5334920` (`_ZN3lib9SingletonINS_13EffectManagerEE9instance_E`); read twice: `adrp x8, 0x5334000; ldr x8, [x8, #0x920]; ldr x0, [x8]` (words at `0x34cb16c`, the game's own `effect_load(*instance, 0x372, &handle)` call) | disassembly `0x34cb160..0x34cb184`; ELF symbol |
| effect file load | `0x355fea0(manager, key, file_handle*)` (12.5 KB) | the only caller of the particle system's resource constructor path (`0xa4d90`) |
| file entries | `manager + 0x18000 + slot * 0xa8` (30 slots): `+0` key (-1 free), `+4` resource index (set after the resource was made), `+8` file hash40 (low 40 bits), `+0x10` the loaded file's bytes, `+0x98` reference count | stores at `0x3560884` (`mov w11, #0xa8; ... mov w12, #0x8008; movk 1<<16; str x10, [x11, x12]`), `0x35609ac` (`+0x18010`, then the `EFFN` magic bytes), `0x3560aa4` (`+0x18004`) |
| the file is used in place | `0x3560a78: bl 0xa4d90(system, &heap, pcVar38 + short[EFFN+0xe] * 0x1000, resource_index, ...)`: the `VFXB` block of the loaded `.eff` is passed by pointer, nothing is copied | decompile `0x355fea0`, asm `0x3560a1c..0x3560a78` |
| particle system | `[[manager + 0x193d8] + 0xf8]` = `[manager + 0x194d0]`; resources are `[[system + 0x98] + index * 8]`; a resource is valid when its byte `+9` is set | asm `0x35609fc..0x3560a0c`, `0x3560ab4..0x3560ac8` |
| resource constructor | `0x99560` (`VFXB` header check `0x2020202042584656`, version 0x16): walks the `GRTF` (textures), `PRMA`, `G3PR`, `GRSN`, `ESTA` (emitter sets) sections | decompile |
| textures built in place | `0x9a100` (`GRTF`): `file = 0x31100(bntx)` (same BNTX cast the UI builder uses) stored at resource `+0xa8`; memory pool `0x33920`; per texture `0x3a130` (texture init, `BRTI+0x80`) and `0x3a270` (view init, `BRTI+0x88`); count at resource `+0xa0` | decompile / asm `0x9a1b8` |
| descriptor registration | `0x9ab70(resource, 0x35b9360, 0)` (called by the loader at `0x356153c`): for each texture `register(&slot, BRTI+0x88, 0)`, slot (8 bytes) stored at `BRTI+0x90`; for each emitter the sampler slots are resolved by `0x9acd0` into the emitter `+0x48/+0x50/+0x58` | decompile `0x9ab70`, asm `0x9abb4..0x9ac8c` |
| emitter sets | resource `+0x4c` count, `+0x58` table (stride `0x38`: `+4` emitter count, `+0x18` emitters), emitter stride `0x3e0`, emitter `+0x10` = its `EMTR` binary (`+0x9f0/+0xa10/+0xa30` the three sampler ids) | `0x9a840` (sets in `ESTA` order), `0x9ac08..0x9ac8c` |
| the registration function | `0x35b9360(u64* slot_out, view, user) -> bool`: takes the next free index of the shared descriptor pool, writes the view's descriptor, stores the slot; **the same function the UI texture update calls** (`ui_tex_poll`, `0x37a3510`, with `(ui_texture + 0x40, view, 0)`) and keeps the result at `ui_texture + 0x5c` (the game's destroy `0x37a2aa0` releases the slot only then) | decompile `0x35b9360`, `0x37a2aa0`, `0x37a3510` |

Name to set: the `EFFN` header (`+8` entry count, `+0xe` block index of the `VFXB`, 16-byte entries from `0x10` whose second word is the
1-based emitter-set index, then NUL-terminated upper-case names in entry order; W7's reading, re-checked on the 13.0.5 file:
`SYS_VECTOR` entry 86 -> set 82 `P_CmnVector`, `SYS_HITSTOP_SPARK` entry 106 -> set 101 `P_CmnHitStopSpark`). The plugin takes the table
from the resident bytes (`entry + 0x10`), requires exactly as many names as entries, and refuses otherwise.

## Helper

- `src/model/effect_texture.rs` (pure, host-tested on a synthetic image): offsets, the `Memory` trait, `find_common`, `effect_set`,
  `emitters`, `texture_of_slot`, `EmitterColours::parse` (W9's layout: fixed colours `+0x9a0/+0x9b0`, key tables, sources, scale),
  `tint_rgba`.
- `src/runtime/ui/effect_texture.rs`: `install()` (guards, manager slot decoded from the game's own words), `usable()`, `locate()`,
  `pictures(Visual) -> Pictures` (the distinct textures of the visual's emitters: size, format, tint, colours), `texture_of(&Picture)`
  (a UI `Texture`), `survey` / `describe` (diag lines).
- `src/runtime/ui/texture.rs`: new `load_resident(view, w, h)`: the game's UI texture constructor with the "no handle" case (nothing
  queued, nothing loaded), `0x35b9360` for this texture, then the three fields the UI update would have written (slot by the call,
  size, registered byte). One new called function (`ui_register_view`), nine relied instruction windows.
- Diag proof: `src/runtime/ui/effect_probe.rs` (`[EffTex]` log lines; DI row above, SDI row below, on the menu's tile panes).
- Lifetime: the chip's page asks when it opens (UI thread, once, not per frame), binds, and on close `unbind` then `texture::retire`.
  The registered slot is the UI texture's own and is released by the game's destroy; the view stays the effect system's.
