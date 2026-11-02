# U4: a game-file texture on a picture pane (stream u4, 2026-11-02)

SPEC-core-07 §4.5 U1-U4 and §4.6 (stage thumbnails, fighter portraits and the DI / SDI visual are read at run time, never baked).
Static reverse engineering of the owned 13.0.5 `main` (`research/game-13.0.5/main-13.0.5.elf`, Ghidra 12.1.4 headless over a copy of
the project, plus small ELF scans; no upstream code was opened). Offsets are `main`-relative. Evidence for every row is the
decompilation of the function named; the guards are in `src/model/sites_u4.rs` (`tests/u4_sites.rs` re-checks them against the ELF).

## Answer

The plugin does not read, decode or upload a BNTX. The game already has a loader that turns a file's hash40 into a texture-info a
picture pane takes, and it runs on the game's own file-resource service, which is the layer ARCropolis hooks: a mod's replacement file is
what loads (U1-U3 answered by construction; the game's stage select shows a stage mod's picture the same way). The plugin asks for the
texture of `hash40("ui/replace/stage/stage_1/stage_1_<name_id>.bntx")`, polls it each UI frame, binds it with the game's own TexMap
override and releases it with the game's own shared-pointer release. The API therefore takes a path hash, not file bytes.

## What the game does (RE)

| What | Function (main+) | Evidence |
| --- | --- | --- |
| path hash of a UI texture (the stage kind) | `0x3238c50` (key), `0x3239920` (prefix table at `0x5181998`: `stage/stage_1/stage_1_` is kind 0xd), `0x323a1f0..` | the `%s.bntx` formats and CRC table; the loader's callers build the key with them |
| the UI asset loader's request | `0x323c4e0` | looks the key up (`0x353da30`), requests the load (`0x35465b0`), wraps the handle (`0x37852a0`) and stores the shared texture in its map; `0x323f4a0` returns it |
| file path hash -> resource handle, one reference taken | `0x353da30(u32* out, u64 hash40)`; `0x35404b0` is the reference-taking lookup it calls | decompiled; 0xffffff when the path is not in the resource table |
| UI texture constructor | `0x37852a0(shared_ptr_out[2], const u32* handle)` -> `0x37a31e0` | allocates the 0x68-byte object, copies the handle, takes its own reference (`0x3540a00`), queues the file for loading (`0x35465b0(service, handle, 1)`); writes `{holder, control}` |
| UI texture layout | `0x37a31e0`, `0x37a3510`, `0x37a2aa0` | `+0x00` handle (0xffffff once consumed), `+0x08` loaded copy, `+0x18` pending job, `+0x20` resource texture wrapper (its own vtable), `+0x38` the embedded texture-info (vtable `main+0x5182f40`): the descriptor, `+0x58` life state, `+0x60` job-state object |
| load completion | `0x37a3510(ui_texture*)` | when the file resource reaches state 3 it copies the bytes into a 0x1000-aligned buffer, builds the GPU memory pool, `FUN_00065670` makes the texture and texture view, `0x35b9360` registers the view in the descriptor pool (slot at `+0x40`), and the handle is released |
| bind to a picture pane | `0x379e9c0(pane, descriptor, index)` (pkg5's decode) and the game's own setter `0x379e580` | the setter runs `0x37a3510`, then binds `[holder+8]+0x38` when `[tex+0x18]==0 && [tex]==0xffffff && ([tex+0x60]==0 || [[tex+0x60]+0x18]==3)` (words `0x379e610..0x379e64c`) |
| unbind | `0x379e470(pane)` | restores the saved TexMap pointers of both indices, drops the pane's cached shared texture, clears its pending flag |
| release | control block: `[ctrl+8]` is owners-1; the last owner runs `[[ctrl]+0x10](ctrl)` = `0x37855a0` -> `0x37853b0(holder)`, which only marks the texture "to destroy" (`+0x58 = 5`) and queues it on the UI manager (`+0x1d0`, under its lock): the game destroys it on a later frame | `0x37855a0`, `0x37853b0`; `0x39c2670` is the `__release_weak` import stub |
| the shared pointer | holder `{ vtable main+0x52414c0, ui_texture* }`, control block `{ vtable main+0x523dba0, owners-1, weak, holder*, deleter }` | `0x37852a0` |

`RenderTargetTextureInfo`, `PlacementTextureInfo` and `ArcResourceAccessor` (the layout archive's own BNTX path: `__Combined.bntx`
loaded by `0x41710`, `ResTexture::Initialize` = `FUN_00065670`) were decoded too; they are the layout's own textures, not what a pane
takes at run time.

## Helper (`src/runtime/ui/texture.rs`, Switch only; pure half `src/model/ui_texture.rs`)

- `install() -> [GuardOutcome; 9]`: compares every row of `sites_u4::SITES` (seven called functions, the bind-readiness words, the
  constructor) and decodes the file-resource service slot (`main+0x5332f20`) from the lookup's own `adrp`/`ldr`; the helper is usable only
  when all pass.
- `load(hash40) -> Option<Texture>`: lookup, construct, drop the lookup's reference, check the holder and control-block vtables.
- `Texture::poll() -> Readiness`: runs the game's update and applies the game's readiness test (`model::ui_texture::is_ready`).
- `bind(pane, &Texture) -> bool`, `unbind(pane)`: the game's override and reset.
- `retire(texture)` + `tick()`: release after 8 UI frames; `release(texture)` releases at once.

All calls are on the UI job thread inside the layout tick, where the game makes the same calls. Nothing allocates on the plugin heap
(`heap_allocations` unchanged, logged as `allocations=0`). `load` is cheap (the file loads in the background), so a page can request its
tiles' textures when it opens and bind each when `poll` says Ready; the page releases them when it closes (unbind, then `retire`).

## What pkg5 does with it

Stage tile `i`: `texture::load(hash40(path))` when the page opens (path from the carried catalog: `ui/replace[_patch]/stage/stage_1/stage_1_<name_id>.bntx`),
`poll` each frame until Ready, then `bind(tile_pane, &texture)` with the tile's vertex colour white; on close `unbind(tile_pane)`
and `retire(texture)`; call `texture::tick()` once per frame (one layout's tick). A `load` that returns `None`, or a texture that is not
ready after a few seconds, leaves the text tile. The same functions serve fighter portraits (kind `chara_1/chara_1_<...>`, SPEC-core-07 Q16;
the key builders are `0x3238a70` and the table at `0x5181998`) and the DI / SDI chip pictures only if the effect textures are packaged as
UI textures (they are not: `ef_common.eff` carries them; §4.6 still needs its own decode).

Limits: only files in the game's resource table can be requested (a stage mod that adds a new path is out of scope, U8); the texture's
format and size are whatever the file holds (the picture pane scales it, the stage files are 108 x 90).
