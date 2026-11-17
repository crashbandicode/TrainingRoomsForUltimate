# Compliance report: rc15-tech-seam

| Requirement | Tag | Result | Evidence |
| --- | --- | --- | --- |
| crate-type is ["cdylib"] only (no rlib) | M | pass | Cargo.toml crate-type = ['cdylib'] |
| release profile opt-level "s" (or "z", measured), lto, codegen-units 1, panic abort | M | pass | Cargo.toml [profile.release] as required |
| opt-level "z" measured: resident before and after; the stack analysis and the svc-clobber check on the "z" ELF, both following OUTLINED_FUNCTION_* bodies; a timed Eden run of the frame-precise rows | M | not authorized | Canonical validation is primary-owned; no worker build or runtime acceptance. |
| skyline pinned to =0.6.0 | M | pass | Cargo.toml |
| diag cargo feature declared | M | pass | Cargo.toml [features] |
| toolchain 1.98.1 | M | pass | rust-toolchain.toml |
| runtime only under cfg(target_os = "switch") | M | pass | src/lib.rs |
| supported version 13.0.5 in model | M | pass | src/model/archive_recipe.rs, src/model/cpu/level.rs, src/model/developer_debug.rs |
| every site guard is 64 bytes | M | pass | 3 guard(s) in src/, sizes [64] |
| counting allocator reports heap_allocations | M | pass | src/**/*.rs |
| host tests in model (cargo test) | M | pass | 377 #[test] in src/** |
| README carries the AI-assistance disclaimer | M | pass | README.md |
| README has a GIF, build commands and a license line | M | pass | README.md |
| GIF change-stage.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.3 s |
| GIF combo-helper.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.4 s |
| GIF cpu-kirby-hat.gif: 1280 wide, under 10 s | M | pass | 1280 px, 4.2 s |
| GIF di-none.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.5 s |
| GIF di-sdi-chips.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.0 s |
| GIF di-up.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.5 s |
| GIF frame-strip.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.7 s |
| GIF hitbox-overlay.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.0 s |
| GIF menu-cpu.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF menu-defaults.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF menu-fighters.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF menu-recording.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF menu-section-jump.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.0 s |
| GIF menu-shortcut.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.5 s |
| GIF menu-stages.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.7 s |
| GIF menu-tabs.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.9 s |
| GIF menu-tour.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.5 s |
| GIF mii-editor-list.gif: 1280 wide, under 10 s | M | pass | 1280 px, 4.5 s |
| GIF mii-editor.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.3 s |
| GIF own-orbs-two-ridleys.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.5 s |
| GIF p1-kirby-hat.gif: 1280 wide, under 10 s | M | pass | 1280 px, 4.1 s |
| GIF rc10-footer-tag.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF rc10-record-play-cpu.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc10-save-load.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF rc11-after-an-attempt.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.2 s |
| GIF rc11-auto-load-on-death.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.0 s |
| GIF rc11-buffs-on-load.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc11-button-chords.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc11-change-stage.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc11-cpu-airdodge-direction.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.6 s |
| GIF rc11-cpu-character.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.6 s |
| GIF rc11-cpu-clatter-strength.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF rc11-cpu-crouch.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.6 s |
| GIF rc11-cpu-falling-aerials-aerial-delay.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.2 s |
| GIF rc11-cpu-followup.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF rc11-cpu-full-hop.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.6 s |
| GIF rc11-cpu-hide-tech-animations.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.6 s |
| GIF rc11-cpu-ledge-delay.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.6 s |
| GIF rc11-cpu-ledge-options.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.6 s |
| GIF rc11-cpu-level.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.7 s |
| GIF rc11-cpu-mash-triggers.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.6 s |
| GIF rc11-cpu-mash.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF rc11-cpu-mistech-options.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF rc11-cpu-p1-cpu-control.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF rc11-cpu-p1-cpu-level.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF rc11-cpu-shield-tilt.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.6 s |
| GIF rc11-cpu-shield.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF rc11-cpu-situation-overrides.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF rc11-cpu-tech-options.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF rc11-cpu-throw-delay.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.6 s |
| GIF rc11-cpu-throw.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.6 s |
| GIF rc11-cycle-switch.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.4 s |
| GIF rc11-damage-on-load.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.6 s |
| GIF rc11-damage-range.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc11-di-sdi-chips.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.5 s |
| GIF rc11-di.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.6 s |
| GIF rc11-edit-placement.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.4 s |
| GIF rc11-footer-tag.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.1 s |
| GIF rc11-frame-advantage.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.3 s |
| GIF rc11-game-speed.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.4 s |
| GIF rc11-give-item.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.2 s |
| GIF rc11-hitbox.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.4 s |
| GIF rc11-hud.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF rc11-input-delay-sbs.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.0 s |
| GIF rc11-lobby-combos.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.2 s |
| GIF rc11-lobby-practice.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.0 s |
| GIF rc11-lobby-speed.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.2 s |
| GIF rc11-lobby-stages.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.6 s |
| GIF rc11-lra-reset.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc11-mash-interrupts.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.6 s |
| GIF rc11-menu-shortcut-2.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.1 s |
| GIF rc11-menu-shortcut.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.8 s |
| GIF rc11-menu-tour.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.2 s |
| GIF rc11-mii-gunner.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.3 s |
| GIF rc11-mii-swordfighter.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.6 s |
| GIF rc11-mirroring.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.2 s |
| GIF rc11-on-time-tolerance.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.5 s |
| GIF rc11-permanent-shortcut.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.6 s |
| GIF rc11-player-swapping.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF rc11-popup-hot-five.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.7 s |
| GIF rc11-popup-loop-count.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.7 s |
| GIF rc11-popup-player-cpu.gif: 1280 wide, under 10 s | M | pass | 1280 px, 4.3 s |
| GIF rc11-replay-capture.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.3 s |
| GIF rc11-replay-window.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.2 s |
| GIF rc11-save-setup.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.7 s |
| GIF rc11-settings-override.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.4 s |
| GIF rc11-setup-library.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc11-solo-popo.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF rc11-stage-rotation.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.7 s |
| GIF rc11-strip-under-menu.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF rc11-waiting-menu.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.9 s |
| GIF rc11-waiting-reentry.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.4 s |
| GIF rc12-ask-before-switching-fighter.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF rc12-attack-angle.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.0 s |
| GIF rc12-crop-trailing-neutral.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.4 s |
| GIF rc12-cycle-preload.gif: 1280 wide, under 10 s | M | pass | 1280 px, 2.7 s |
| GIF rc12-fast-fall-delay.gif: 1280 wide, under 10 s | M | pass | 1280 px, 2.9 s |
| GIF rc12-fast-fall.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.6 s |
| GIF rc12-group-swap.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.6 s |
| GIF rc12-hitstun-timing.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.2 s |
| GIF rc12-lobby-items.gif: 1280 wide, under 10 s | M | pass | 1280 px, 1.8 s |
| GIF rc12-loop.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc12-oos-offset.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.0 s |
| GIF rc12-player-character.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.5 s |
| GIF rc12-preload-hats.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc12-preload.gif: 1280 wide, under 10 s | M | pass | 1280 px, 2.7 s |
| GIF rc12-pummel-delay.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc12-reaction-time.gif: 1280 wide, under 10 s | M | pass | 1280 px, 2.2 s |
| GIF rc12-ready-fighter-bodies.gif: 1280 wide, under 10 s | M | pass | 1280 px, 2.7 s |
| GIF rc12-ready-fighters.gif: 1280 wide, under 10 s | M | pass | 1280 px, 2.7 s |
| GIF rc12-sdi-strength.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.2 s |
| GIF rc12-stage-hazards.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF rc12-swap-groups.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.6 s |
| GIF rc13-aerial-drift.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.2 s |
| GIF rc13-chain-picker.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.6 s |
| GIF rc13-clatter-strength.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.9 s |
| GIF rc13-crop-macro.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.4 s |
| GIF rc13-defaults-reset-page.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.8 s |
| GIF rc13-di-sdi-chips.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.2 s |
| GIF rc13-di.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.4 s |
| GIF rc13-dodge-staling.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.0 s |
| GIF rc13-fast-fall-delay-quarter.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.2 s |
| GIF rc13-grab-boxes.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.6 s |
| GIF rc13-input-delay-sbs.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.0 s |
| GIF rc13-left-right-toggle.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.5 s |
| GIF rc13-macro-rename.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc13-mash-dodge-always.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.0 s |
| GIF rc13-mii-pick.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc13-pills.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.7 s |
| GIF rc13-player-swapping-gate.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.7 s |
| GIF rc13-projectile-hitboxes.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.4 s |
| GIF rc13-replay-capture.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.4 s |
| GIF rc13-reset-page-cpu.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc13-reset-page-fighters-stages.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc13-reset-page-recording.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.4 s |
| GIF rc13-save-duplicate-delete.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc13-sdi-direction.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.4 s |
| GIF rc13-stage-forms.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.7 s |
| GIF rc13-stage-pick.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc13-taunts-kirby-hat.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc13-taunts-off.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.4 s |
| GIF rc13-wario-bike.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.7 s |
| GIF rc13-your-defaults.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.0 s |
| GIF rc14-btt-layouts-list.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.4 s |
| GIF rc14-btt-layouts.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc14-btt-place.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.7 s |
| GIF rc14-btt-run.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.2 s |
| GIF rc14-chain-edit.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.1 s |
| GIF rc14-cpu-during-a-game.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.2 s |
| GIF rc14-eggs-1-minute.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc14-eggs-cpu-target.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.7 s |
| GIF rc14-eggs-n-targets.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.3 s |
| GIF rc14-eggs-speed.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.9 s |
| GIF rc14-hide-cpu.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.6 s |
| GIF rc14-hitbox.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.0 s |
| GIF rc14-menu-tour.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.6 s |
| GIF rc14-playback-slot.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.2 s |
| GIF rc14-save-setup-here.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.8 s |
| GIF rc14-save-setup.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.6 s |
| GIF rc14-setup-library.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.6 s |
| GIF rc14-setup-load-target.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.1 s |
| GIF rc14-setup-own-screen.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.7 s |
| GIF rc14-start-save-onoff.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.8 s |
| GIF rc14-swap-groups-aegis-groups.gif: 1280 wide, under 10 s | M | pass | 1280 px, 2.8 s |
| GIF rc14-swap-groups-orig-bank.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.6 s |
| GIF rc14-swap-groups-pt-groups.gif: 1280 wide, under 10 s | M | pass | 1280 px, 2.8 s |
| GIF rc14-waiting-menu.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.6 s |
| GIF rc14-windbox-krool-vacuum.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.0 s |
| GIF rc14-windboxes.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.0 s |
| GIF rc15-article-hurtboxes.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.8 s |
| GIF rc15-crop-editor.gif: 1280 wide, under 10 s | M | pass | 1280 px, 4.6 s |
| GIF rc5-chain-play.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.0 s |
| GIF rc5-fighter-mismatch.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.6 s |
| GIF rc5-frame-strip-advantage-hit-crop.gif: 1280 wide, under 10 s | M | pass | 1280 px, 4.6 s |
| GIF rc5-frame-strip-advantage-hit.gif: 1280 wide, under 10 s | M | pass | 1280 px, 4.6 s |
| GIF rc5-frame-strip-advantage-shield-crop.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.0 s |
| GIF rc5-frame-strip-advantage-shield.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.0 s |
| GIF rc5-frame-strip-chips.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.9 s |
| GIF rc5-frame-strip-cstick.gif: 1280 wide, under 10 s | M | pass | 1280 px, 2.7 s |
| GIF rc5-frame-strip-drop-crop.gif: 1280 wide, under 10 s | M | pass | 1280 px, 4.6 s |
| GIF rc5-frame-strip-drop.gif: 1280 wide, under 10 s | M | pass | 1280 px, 4.6 s |
| GIF rc5-frame-strip-exchange-crop.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.0 s |
| GIF rc5-frame-strip-exchange.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.0 s |
| GIF rc5-frame-strip-freeze-crop.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.0 s |
| GIF rc5-frame-strip-freeze-stick.gif: 1280 wide, under 10 s | M | pass | 1280 px, 4.7 s |
| GIF rc5-frame-strip-freeze.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.0 s |
| GIF rc5-frame-strip-input-crop.gif: 1280 wide, under 10 s | M | pass | 1280 px, 4.6 s |
| GIF rc5-frame-strip-input.gif: 1280 wide, under 10 s | M | pass | 1280 px, 4.6 s |
| GIF rc5-frame-strip-lanes.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.4 s |
| GIF rc5-frame-strip-last-crop.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.1 s |
| GIF rc5-frame-strip-last.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.1 s |
| GIF rc5-frame-strip-link-crop.gif: 1280 wide, under 10 s | M | pass | 1280 px, 4.2 s |
| GIF rc5-frame-strip-link.gif: 1280 wide, under 10 s | M | pass | 1280 px, 4.2 s |
| GIF rc5-frame-strip-stick-row.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.0 s |
| GIF rc5-grade-not-true-crop.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.1 s |
| GIF rc5-grade-not-true.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.1 s |
| GIF rc5-loop-no-warp.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.2 s |
| GIF rc5-migration-crop.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.1 s |
| GIF rc5-migration.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.1 s |
| GIF rc5-play-chord-crop.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.6 s |
| GIF rc5-play-chord.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.6 s |
| GIF rc5-popup-trim.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.5 s |
| GIF rc5-popup.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.0 s |
| GIF rc5-record-start-save.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.0 s |
| GIF rc5-recording-page-trim.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.5 s |
| GIF rc5-recording-page.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.0 s |
| GIF rc5-save-warp-crop.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.0 s |
| GIF rc5-save-warp.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.0 s |
| GIF rc6-autoload.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.6 s |
| GIF rc6-both-play.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.0 s |
| GIF rc6-crop-combo.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.1 s |
| GIF rc6-cycle-switch.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.6 s |
| GIF rc6-lobby-combos.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.9 s |
| GIF rc6-macro-set-active.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.3 s |
| GIF rc6-make-combo.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.5 s |
| GIF rc6-mii-same.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.0 s |
| GIF rc6-named-mii.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.6 s |
| GIF rc6-replay-window.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.4 s |
| GIF rc6-save-last-combo.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.3 s |
| GIF rc6-sequence-drill.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.0 s |
| GIF rc6-waiting-partners.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.6 s |
| GIF rc6-waiting-reentry.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.9 s |
| GIF rc7-chain-play.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.6 s |
| GIF rc7-change-stage.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.4 s |
| GIF rc7-combo-helper.gif: 1280 wide, under 10 s | M | pass | 1280 px, 4.8 s |
| GIF rc7-cpu-level-notice.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.7 s |
| GIF rc7-di-up.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.0 s |
| GIF rc7-fighter-mismatch.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.0 s |
| GIF rc7-frame-strip-advantage-hit.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.0 s |
| GIF rc7-frame-strip-advantage-shield.gif: 1280 wide, under 10 s | M | pass | 1280 px, 2.9 s |
| GIF rc7-frame-strip-chips.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.2 s |
| GIF rc7-frame-strip-cstick.gif: 1280 wide, under 10 s | M | pass | 1280 px, 2.7 s |
| GIF rc7-frame-strip-drop.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.4 s |
| GIF rc7-frame-strip-exchange.gif: 1280 wide, under 10 s | M | pass | 1280 px, 4.0 s |
| GIF rc7-frame-strip-freeze-stick.gif: 1280 wide, under 10 s | M | pass | 1280 px, 4.7 s |
| GIF rc7-frame-strip-freeze.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.0 s |
| GIF rc7-frame-strip-input.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.0 s |
| GIF rc7-frame-strip-lanes.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.8 s |
| GIF rc7-frame-strip-last.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.6 s |
| GIF rc7-frame-strip-link.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.0 s |
| GIF rc7-frame-strip-stick-row.gif: 1280 wide, under 10 s | M | pass | 1280 px, 2.7 s |
| GIF rc7-hitbox.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.8 s |
| GIF rc7-kirby-hats.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.1 s |
| GIF rc7-lobby-stages-rotation.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.8 s |
| GIF rc7-loop-no-warp.gif: 1280 wide, under 10 s | M | pass | 1280 px, 3.8 s |
| GIF rc7-menu-shortcut.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.2 s |
| GIF rc7-menu-tour.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.5 s |
| GIF rc7-mii-editor.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.3 s |
| GIF rc7-mismatch-player1.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.8 s |
| GIF rc7-play-chord.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.5 s |
| GIF rc7-record-macro.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.6 s |
| GIF rc7-record.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.6 s |
| GIF rc7-replay-capture.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.4 s |
| GIF rc7-respawn-off.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.5 s |
| GIF rc7-respawn-on.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.6 s |
| GIF rc7-save-warp.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.3 s |
| GIF rc7-sequence-drill.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.0 s |
| GIF rc8-footer-tag.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.0 s |
| GIF rc8-game-speed-chord.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.1 s |
| GIF rc8-input-display-chord.gif: 1280 wide, under 10 s | M | pass | 1280 px, 7.8 s |
| GIF rc8-speed-hud-fixed.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.1 s |
| GIF record-refused.gif: 1280 wide, under 10 s | M | pass | 1280 px, 4.7 s |
| GIF replay-capture.gif: 1280 wide, under 10 s | M | pass | 1280 px, 9.5 s |
| GIF respawn-off.gif: 1280 wide, under 10 s | M | pass | 1280 px, 5.6 s |
| GIF respawn-on.gif: 1280 wide, under 10 s | M | pass | 1280 px, 6.9 s |
| GIF waiting-menu.gif: 1280 wide, under 10 s | M | pass | 1280 px, 8.0 s |
| dated evidence doc present | M | pass | CARRY-OVER.md, MENU-DRAFT.md, NEXT.md, PROVENANCE.md, docs/ASSET-REPRODUCTION-R15.md, docs/BUILD.md, docs/CODE-REDUCTION-rc10.md, docs/COVERAGE-rc9.md, docs/DECISIONS.md, docs/DIFFERENCES-FROM-THE-FORK.md, docs/EVIDENCE-rc15-crop-editor.md, docs/FEATURE-LEDGER.md, docs/MEDIA.md, docs/MII.md, docs/RELEASE-NOTES-rc6.md, docs/RESTRICTIONS.md, docs/ROADMAP.md, docs/SIZE-PLAN.md, docs/TECHNICAL.md, docs/TEST-CUT-LEDGER-rc9.md, docs/menu-redesign.md, docs/resource-costs.md |
| numbers in the evidence come from tested scripts over the logs | C | pass | tools/: carry_pkg4.py, carry_pkg6.py, carry_ui_items.py, case_cover.py, check_external_links.py, coverage.py, crop_menu_captures.py, docs_check.py, draw_egg_target.py, elf_symbols.py, f6_matched_table.py, f6_split_table.py, feature_groups.py, feature_matrix.py, game_thread_wait_check.py, gen_catalog.py, gen_hitbox_sites.py, gen_rows_cpu_level.py, gen_rows_custom_stages.py, gen_rows_game_speed.py, gen_rows_lobby_items.py, gen_rows_lobby_persist.py, gen_rows_lobby_speed.py, gen_rows_module_pins.py, gen_rows_res_unload.py, gen_sites_pkg3.py, gen_sites_pkg4.py, gen_sites_pkg6.py, gen_sites_u4.py, gen_sites_u4b.py, gen_stage_textures.py, loc_raw.py, pkg6_deps.py, pkg6_inventory.py, provenance_scan.py, rc5_merge_accept.py, re_hitbox_layout.py, render_decisions.py, render_decisions_1113.py, render_guide_diagrams.py, render_menu_mockup.py, render_rc13_memory.py, render_rc8_binds.py, render_recording_diagrams.py, render_sdi_stretch.py, rows_tsv.py, sdi_streak_measure.py, seam_census.py, size_gate.py, sloc.py, stack_depth.py, stack_marks.py, stack_table.py, svc_clobber_check.py, switch_times.py, test_check_external_links.py, test_cut.py, test_docs_check.py, test_f6_rows.py, test_f6_split_table.py, test_loc_raw.py, test_rows_tsv.py, test_size_gate.py, test_stack_depth.py, test_stack_marks.py, test_stack_table.py, test_svc_clobber_check.py, test_switch_times.py |
| handoff file present | M | pass | HANDOFF.md |
| remote is Forgejo, not GitHub | M | pass | https://git.syntaxbyexample.com/intpa/TrainingRoomsForUltimate.git |
| working tree clean | M | fail | git status: dirty |
| no script mounts, unmounts or ejects the card (the user connects and disconnects it) | M | pass | none in the tracked scripts or the playbook's tools/switch |
| build identity | M | info | HEAD bd530693; no tag on HEAD |
| shipped NRO identity and diag strings | M | not authorized | Canonical validation is primary-owned; no worker build or runtime acceptance. |
| New user-facing refusals cite an approved restriction (// restriction: ID, docs/RESTRICTIONS.md) | M | pass | 0 added refusal(s) since f84e0de4, all cite approved IDs |
| Design report traces every operator clause to a test that exists | M | pass | 3 clause(s), every test found |
| Gates run: test, clippy -D warnings, no-default-features, skyline release, diag build | M | not authorized | Canonical validation is primary-owned; no worker build or runtime acceptance. |
| Fixes: root cause fixed, or each workaround approved by the operator before it shipped and listed with its date | M | pass | FOLLOWUP-SOURCE.json: exact three missing dependencies added, none removed; no workaround. |
| Card scan for the site (and vtable slot) over a dated card copy | M | N/A: no offsets or ABI changed | Three already guarded natives enter optional TechPractice closure. |
| Hook sharing or chaining; both stand-in modes and load orders in Eden | C | N/A: no new hooks or sites | Only existing CROSS_NEEDS data and host test changed. |
| Hook after the prologue or the function called instead | M | N/A: no new hooks or sites | Only existing CROSS_NEEDS data and host test changed. |
| Hooks never block on a lock a plugin thread can hold: lock-reach test passing (fails on a seeded violation), or why none applies | M | not authorized | Current UI lock issues/init ordering remain open; no new hook call added, no audit weakened. |
| Shipped NRO writes only its install line (plus a logging plugin's data files) | M | not authorized | Canonical validation is primary-owned; no worker build or runtime acceptance. |
| Online: read-only rules reviewed hook by hook | C | N/A: source-only worker, no Eden/build/online operation | No runtime, target directory, profile, claim or card mutation. |
| Online: replay A/B with the desync detector identical | C | N/A: source-only worker, no Eden/build/online operation | No runtime, target directory, profile, claim or card mutation. |
| Online: two consoles past the changed path, desync_seen=0 at the same frame | C | N/A: source-only worker, no Eden/build/online operation | No runtime, target directory, profile, claim or card mutation. |
| Eden: claim written and released, also after a failed run | M | N/A: source-only worker, no Eden/build/online operation | No runtime, target directory, profile, claim or card mutation. |
| Eden: at most two instances per Linux host, one on the Ally; MemAvailable >= 10 GiB; not profile a or :61 | M | N/A: source-only worker, no Eden/build/online operation | No runtime, target directory, profile, claim or card mutation. |
| Eden: profile copy with hard-linked game content; only the copy changed | M | N/A: source-only worker, no Eden/build/online operation | No runtime, target directory, profile, claim or card mutation. |
| Copies: byte-identical files linked, differing or written-in-place files copied | M | N/A: source-only worker, no Eden/build/online operation | No runtime, target directory, profile, claim or card mutation. |
| Disk: one target dir per source copy (never shared between copies), cargo home shared per host and toolchain, df checked (>= 40 GB), finished build products deleted with a manifest | M | N/A: source-only worker, no Eden/build/online operation | No runtime, target directory, profile, claim or card mutation. |
| Eden: waits gated on a log line or screenshot check | M | N/A: source-only worker, no Eden/build/online operation | No runtime, target directory, profile, claim or card mutation. |
| Eden: test run plays a runner dry-run before Up (recorded segments + sha256, automated gates, per-row receipts, rows chained on end state); manual input only to build a missing segment | M | N/A: source-only worker, no Eden/build/online operation | No runtime, target directory, profile, claim or card mutation. |
| Eden: rows grouped into as few boots as hosts/builds allow; a failed row is fixed or deferred on the same boot; per-run RESULTS.md <= 20 lines, compliance once per phase | M | N/A: source-only worker, no Eden/build/online operation | No runtime, target directory, profile, claim or card mutation. |
| Eden: failed precondition aborted and retried under the next label | M | N/A: source-only worker, no Eden/build/online operation | No runtime, target directory, profile, claim or card mutation. |
| Eden: game-side facts from guest tracing (persistent trace.* points or the one-shot tracepoint) on a run already happening; diag rebuild or extra trace boot only with a stated reason | M | N/A: source-only worker, no Eden/build/online operation | No runtime, target directory, profile, claim or card mutation. |
| Eden: profile restored, temp files removed, host released | M | N/A: source-only worker, no Eden/build/online operation | No runtime, target directory, profile, claim or card mutation. |
| Eden: the run's copies pruned at Down once its files were in out/ (the Prune line); Up refused below 60 GB free | M | N/A: source-only worker, no Eden/build/online operation | No runtime, target directory, profile, claim or card mutation. |
| Eden: console.log searched for panic/abort/crash; run.json and memory.peak kept | M | N/A: source-only worker, no Eden/build/online operation | No runtime, target directory, profile, claim or card mutation. |
| Every local command under the memory and time guard | M | fail | Bootstrap/read-only/git commands use required pwsh but not every one had resource wrapper; scanner/source proof guarded. |
| Evidence doc states run labels, build, what was done, what the log showed | M | pass | FOLLOWUP-HANDOFF.md and FOLLOWUP-SOURCE.json identify source and limitations; no runtime claims. |
| GIF caption gives date and build; clip shows start, action and result | M | N/A: no runtime or visual claim | No new GIF. |
| No operator quotes, no claims beyond the tests | M | pass | Claims limited to source receipts; design table intentionally retains requirement clauses. |
| Handoff file current (branch, commit, evidence, live state, next commands) | M | pass | FOLLOWUP-HANDOFF.md records base/head, init blocker, dependencies, next checks. |
| Release: version bump, annotated tag, assets verified by anonymous download | C | not authorized | No push, tag, release or card mutation. |
| Release: rebuild of the tag byte-identical to the released NRO | C | not authorized | No push, tag, release or card mutation. |
| Card: MTP preflight, rollback copy, readbacks, receipts not re-run | C | not authorized | No push, tag, release or card mutation. |
| Card: the user connected and disconnected it; the agent never mounted, unmounted, ejected or remounted it, nor started or stopped DBI | M | pass | No device, mount, DBI, push, tag or release operation. |
| Nothing pushed, tagged or written to the card without the operator's order | M | pass | No device, mount, DBI, push, tag or release operation. |
| Model/effort gate and watcher per the routing authority | M | pass | Exact thread Sol medium verified; watcher1437382 reaped without change marker. |


Scanner native_exit=1: one machine failure (working tree dirty while evidence receipts were generated). Other machine rows passed. Manual dispositions above retain unrun gates, init blocker and local-command resource-wrapper failure. This is not acceptance.

Source commit bd5306934304b47d89fee6c81c0969872e087169. Initialization is not implemented; no UI/BTT followup was imported because readiness lifecycle and dependency integration remain unresolved.
