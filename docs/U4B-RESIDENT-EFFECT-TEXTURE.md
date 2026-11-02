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
| file entries | `manager + 0x18000 + slot * 0xa8` (30 slots): `+0` key (-1 free), `+4` resource index (set after the resource was made), `+8` the hash40 of the file's **folder** (`effect/system/common` = `0x1482d0fc5a`, confirmed by the Eden survey; the entry holds the folder's effect file), `+0x10` the loaded file's bytes, `+0x98` reference count | stores at `0x3560884` (`mov w11, #0xa8; ... mov w12, #0x8008; movk 1<<16; str x10, [x11, x12]`), `0x35609ac` (`+0x18010`, then the `EFFN` magic bytes), `0x3560aa4` (`+0x18004`) |
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

## Eden (Yoga, own profile u4b, display :95, muted; CPU pilot e3033426, tile cursor b8313ca2; outputs under eden/runs/<label>/, gitignored)

| Label | NRO (diag) | Result |
| --- | --- | --- |
| u4b-01 | 47184b85 | all ten u4b guards Stock; manager found (8 resident effect entries, each EFFN); the entry hash is the folder hash, so the file was not found (fixed: EF_COMMON_HASH40 = hash40(ffect/system/common) = 0x1482d0fc5a) |
| u4b-02 boot 1 | db32d344 | entry 3 (key 0x100) holds ef_common: SYS_VECTOR -> set 82, SYS_HITSTOP_SPARK -> set 101 (= W7); walk stopped at the resource's valid byte (+9 is not word aligned, the live reader refused it; fixed: byte reads need only the address range, pointers stay 4-aligned; the byte is tested non-zero as the loader does) |
| u4b-02 boot 2 (same session, NRO swapped, game restarted) | 9b21b6ea | DI: 4 textures 64x128 (tri00), 64x256 (line09), 64x128 (line11), 32x32 (flare01); SDI: 64x128 (line00), 256x128 (impact13), 128x128 (mask00), 64x64 (grade00) = W7's sizes; all 8 built by load_resident (the game's registration) and bound, tinted with the emitter colour; shot shots/110755-eff-c2-1.png (upper row DI wedge, streak, dashes, blue flare; lower row SDI streak, jagged impact band, star mask, gradient); released after 4 UI frames, two cycles identical, allocation count unchanged (31) |

console.log: one boot-time fault per boot (~3.5 s after boot: Unmapped Write64 @ ...b80, then Cannot execute instruction at unmapped address 0) in both u4b-02 boots and in u4b-01's second boot; the same fault appears in pkg4's s4-07 (10:58Z, no u4b code). It happens long before the probe runs and the game continues to Training; not this stream's code. memory.peak 7.58 GB (8 GiB limit).

Not covered: the chip page itself (pkg5 / ui-screens composes the chip from pictures()), the layout destroyed while bound (the page must unbind first, as for u4), an effect mod (the walk reads names, so a mod that keeps the names works; one that renames them answers NoEffect -> arrow).

## Chips in the menu (u4b-03, 2026-11-02 13:00-13:04Z; head 23249ff, diag 5b1d89ca, release a564bcb7)

The DI and SDI direction grids draw the game's own texture on each chip's pointer pane (untime::ui::chips, model::effect_chips):
DI the streak (f_cmn_line09, 64x256, tint (200, 244, 255) = the streak emitter's colour 0 at life 0.3), SDI the burst
(f_cmn_impact13, 256x128, white) stretched x2.0 along the vector (N10); each turned to its chip's direction; Neutral keeps the ring;
Left / Right are In / Out. The chip tile keeps its square and weight colour (the layout has two picture panes per chip: the tile and the
pointer). Shots: den/runs/u4b-03/shots/130155-c1-di.png, 130214-c1-sdi.png. Log: [Chips] visual=Di bound=10 texture=64x256,
[Chips] visual=Sdi bound=10 texture=256x128, [Chips] dropped (retired) on every grid change and close.

Menu open/close: 4 opens and 4 closes ([Ui] menu opened / menu closed), no hang (stream/sync merged), the game running after the last
close (130335-c4-closed.png). Allocations: the chip path is 0 (the count stays 347 across DI bind, drop and SDI bind in one open menu,
387 across SDI then DI in the next). Not the chip path, for the lead: the count grows by 30-40 at each menu open (also an open that
shows no chip grid) and by about 10 per second while the menu is closed in Training (hook_stats delta=90/100 after the last close; also
+236/+101 before the first open). console.log: only the known boot-time fault at 7.27 s (stack stream). memory.peak 6.60 GB.

Layout teardown: chips::lose_layout (called where the layout is re-registered or the renderer is lost, next to 	humbs::lose_layout)
releases the texture without touching a pane; the page's decisions are ffect_chips::plan (host tests: request on open, drop on
close or swap, failure not retried until the grid closes). Not provoked in Eden.
