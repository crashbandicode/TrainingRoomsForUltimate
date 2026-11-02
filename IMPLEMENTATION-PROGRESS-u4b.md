# Stream u4b progress (resident effect textures for the DI / SDI chips), 2026-11-02

Brief: lead 0caa72b8, operator decision Q17. Branch `stream/u4b` (worktree `TrainingRoomsForUltimate-u4b`, from `work/implementation` 98972b8). Doc: `docs/U4B-RESIDENT-EFFECT-TEXTURE.md`.

- RE result: ef_common.eff is resident (effect manager entry named by the folder hash `effect/system/common` = `0x1482d0fc5a`, Eden u4b-01 survey); its BNTX textures are built in place by the particle system and registered with the game's own descriptor function `0x35b9360`. The plugin reads the structures, never the file.
- Helper: `model::effect_texture` (pure, host-tested), `runtime::ui::effect_texture` (`pictures(Visual)`, `texture_of`), `texture::load_resident`; 10 guard rows in `sites_u4b.rs` (`tests/u4b_sites.rs`).
- Diag proof: `runtime::ui::effect_probe` (`[EffTex]`, `survey`, `describe` lines); Eden harness `eden/u4b/`.
- Evidence log: see the doc's "Eden" section.