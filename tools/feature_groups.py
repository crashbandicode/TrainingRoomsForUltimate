#!/usr/bin/env python3
"""Group the provenance scan rows into features by path rules.

Usage: python3 -I feature_groups.py <provenance-files.json> <out.md> <out.json>
"""
import fnmatch
import json
import sys

# (feature, disposition, [glob patterns]) — first match wins.
RULES = [
    ("Diagnostics, probes and traces (diag feature; shipped NRO carries only the install line)", "ours: carry over (diag feature only)", [
        "src/*_probe.rs", "src/*probe*.rs", "src/hang_trace.rs", "src/diag_ring.rs", "src/practice_trace*.rs",
        "src/heap_metrics.rs", "src/process_memory.rs", "src/swap_native/seam_drill.rs", "src/swap_native/observe.rs",
        "src/mii_*_probe.rs", "src/mii_*_observer*.rs", "src/source_*probe*.rs", "src/kirby_resource_probe.rs",
        "src/turnip_*_probe.rs", "src/item_state_probe.rs", "src/stage_asset_probe.rs", "src/body_bank_probe.rs",
        "src/console_*_probe.rs", "training_mod_consts/tests/diag_*.rs", "training_mod_consts/src/native_seams/drill.rs",
        "diagnostics/*",
    ]),
    ("Seam guards (byte-guarded native sites, version guard, authoritative rows)", "ours: carry over", [
        "src/training/seams.rs", "training_mod_consts/src/native_seams/*", "src/native_fields.rs", "src/effect_hook_chain.rs",
        "src/native_gc_guard.rs", "src/native_unlink_*.rs", "src/native_install.rs", "src/native_service.*",
    ]),
    ("Mii fighters (saved Miis, native editor bridge, HUD portraits, ordinary/CPU Mii, LDN qualification)", "ours: carry over", [
        "src/mii_*", "training_mod_mii/*", "src/swap_native/mii_setup.rs", "training_mod_consts/src/mii_*",
        "training_mod_tui/src/mii_menu.rs", "training_mod_tui/tests/test_mii_catalog.rs", "docs/mii-*", "docs/feature-walkthroughs/mii-*",
        "training_mod_consts/tests/mii_hud_portrait_policy.rs", "src/native_keyboard*", "src/native_keyboard/*",
    ]),
    ("Hot-swap engine (windows, groups, swap bank, identity view, ledger, memory admission, coordinator)", "ours: carry over", [
        "src/swap_native/*", "src/native_body_bank.rs", "src/native_fighter_install.rs", "src/native_hud_refresh.rs",
        "src/native_reload_pose.rs", "src/native_source*.rs", "src/source_*.rs", "src/character_lifecycle.rs",
        "src/character_memory.rs", "src/character_roster.rs", "src/character_sources.rs", "src/model.rs",
        "training_mod_consts/src/swap_model/*", "training_mod_consts/src/body_bank*", "training_mod_consts/src/body_bank/*",
        "training_mod_consts/src/identity_view.rs", "training_mod_consts/src/capacity_creation.rs",
        "training_mod_consts/src/character_forms.rs", "training_mod_consts/src/ordered_roster.rs", "training_mod_consts/src/trio.rs",
        "training_mod_consts/src/source_load.rs", "training_mod_consts/src/roster_menu_tests.rs",
        "training_mod_consts/tests/swap_replay.rs", "training_mod_consts/tests/fixtures/swap/*", "training_mod_consts/tests/scope_callbacks.rs",
        "training_mod_consts/tests/native_reload_pose.rs", "training_mod_consts/tests/model_facade.rs", "training_mod_consts/tests/open_create_new.rs",
        "training_mod_tui/src/roster_grid.rs", "src/training/ui/roster_grid.rs", "scripts/roster-grid/*",
        "docs/fighter-hotswap*", "docs/fighter-hotswap/*", "docs/reference/fighter-hotswap.md", "docs/character-changes.md",
        "docs/character-cycles-validation.md", "docs/solo-popo.md", "src/turnip_identity.rs",
    ]),
    ("Kirby hats (hat orders, hat cycle, copy packages)", "ours: carry over", [
        "src/kirby_hat*.rs", "training_mod_consts/tests/kirby_hat_cycle_policy.rs", "docs/kirby-hats.md", "docs/feature-walkthroughs/kirby-*",
    ]),
    ("Cycle preload (fighter/hat preloading, capacity, admission grace, preload metrics, group swap config)", "ours: carry over", [
        "src/cycle_preload*.rs", "src/preload_metrics.rs", "training_mod_consts/src/cycle_*", "training_mod_consts/tests/cycle_preload_policy.rs",
        "training_mod_consts/tests/preload_metrics_policy.rs", "docs/cycle-preload.md", "tools/group_memory_costs.py", "tools/tests/test_group_memory_costs.py",
    ]),
    ("CPU-as-P1 handoff (cpu_pilot; defers to libssbu_cpu_pilot when present)", "ours: carry over", [
        "src/cpu_pilot*", "src/cpu_pilot/*", "training_mod_consts/src/p1_cpu_menu_tests.rs",
    ]),
    ("Practice stages, lobby stages and Training stage rotation", "ours: carry over", [
        "src/practice_stage*.rs", "src/lobby_stage*.rs", "src/training_stage_rotation*.rs", "training_mod_consts/src/lobby_stages.rs",
        "training_mod_consts/tests/practice_stage_forms_policy.rs", "training_mod_consts/tests/stage_forms_restore.rs",
        "training_mod_consts/tests/training_rotation_restore.rs", "training_mod_consts/tests/waiting_hazards.rs",
        "training_mod_tui/src/stage_*.rs", "src/training/ui/stage_grid.rs", "scripts/stage-grid/*", "docs/practice-stage*", "docs/practice-stages/*",
        "docs/feature-walkthroughs/stage-rotation.png", "src/hazard_manager/policy.rs",
    ]),
    ("Lobby practice (waiting-room practice, waiting UI patch, automation helper)", "ours: carry over", [
        "src/training/lobby.rs", "src/training/practice*.rs", "src/training/ui/waiting_archive.rs", "src/training/ui/archive_*.rs",
        "src/training/ui/anchors.rs", "src/training/ui/layout_registry.rs", "src/training/ui/native_menu.rs", "src/static/waiting_ui.patch",
        "scripts/make_waiting_patch.py", "scripts/waiting_patch_manifest.json", "src/automation_helper/*", "scripts/eden_automation/*",
        "src/common/game_mode.rs", "training_mod_consts/src/practice_setup.rs", "docs/lobby-practice*", "docs/lobby-practice/*",
        "docs/reference/lobby-practice.md", "docs/plans/lobby-parity.md", "docs/lobby-parity-release.md", "docs/feature-walkthroughs/lobby-enable.png",
        "docs/elite-counterpick.md", "docs/feature-walkthroughs/elite-*", "docs/REVISION29_DRAW_DECOUPLING.md", "docs/REVISION30_HOOKPROOF_PROLOGUE.md",
        "docs/REVISION31_HANG_TRACE.md", "docs/native-operations.md", "docs/nro-session.md", "tools/nro_session.py", "tools/tests/test_nro_session.py",
    ]),
    ("Named macros, replay import, record slots, clip editor, live replay", "ours: carry over", [
        "training_mod_macros/*", "src/training/named_macros.rs", "src/replay_*.rs", "src/live_replay.rs", "src/capture_plugin.rs",
        "src/training/ui/clip_editor.rs", "training_mod_tui/src/clip_*.rs", "training_mod_tui/src/macro_panel*", "training_mod_tui/src/macro_panel/*",
        "training_mod_tui/src/replay_panel.rs", "training_mod_tui/src/resume.rs", "docs/named-macros*", "docs/named-macros/*", "docs/reference/named-macros.md",
        "docs/media/record-slot-bind.gif", "docs/media-provenance.json",
    ]),
    ("Setup library and JSON saves (named setups, save_states.json, training_modpack.json, port tool)", "ours: carry over", [
        "src/training/setup_library.rs", "training_mod_consts/src/menu_json.rs", "training_mod_consts/tests/json_*.rs",
        "training_mod_consts/tests/fixtures/ported-json/*", "tools/port_save_states.py", "docs/setup-library*", "docs/setup-library/*",
    ]),
    ("Character items (menu icons from game assets, turnip face, item choice)", "ours: carry over", [
        "src/character_items.rs", "training_mod_consts/src/character_items.rs", "scripts/item-icons/*", "docs/character-items*", "docs/character-items/*",
        "docs/feature-walkthroughs/character-items.gif",
    ]),
    ("Menu shortcuts INI and fork menu additions", "ours: carry over", [
        "training_mod_consts/src/menu_shortcuts.rs", "docs/menu-shortcuts.md",
    ]),
    ("Input display renderer optimisation (fork fix on an upstream feature)", "ours: carry over as the required behaviour (spec SPEC-input-02)", [
        "scripts/input-display-bench/*", "docs/input-display-*.md",
    ]),
    ("Fork release/size docs", "ours: carry over (history)", [
        "docs/REVISION32_SMALL_PROFILE.md", "docs/REVISION33_STARTUP_MEMORY.md", "docs/experimental-release.md",
    ]),
    ("Upstream artwork and layout (unlicensed assets)", "upstream: replace (redraw) — never copy", [
        "src/static/img/*.svg", "src/static/layout.arc", "photos/*",
    ]),
    ("Upstream documentation", "upstream: drop (write our own README)", ["README.md", "CODEOWNERS"]),
    ("Build configuration", "mixed: new Cargo workspace per playbook (fork feature list is ours)", [
        "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "*/Cargo.toml", "*/Cargo.lock", "scripts/roster-grid/Cargo.*",
    ]),
    ("Upstream: CPU behaviour (mash, followup, triggers, overrides, defensive, ledge, tech, shield, aerial/throw timing, frame advantage)", "upstream: clean-room spec (SPEC-cpu-*)", [
        "src/training/mash.rs", "src/training/directional_influence.rs", "src/training/sdi.rs", "src/training/air_dodge_direction.rs",
        "src/training/ledge.rs", "src/training/tech.rs", "src/training/shield.rs", "src/training/shield_tilt.rs", "src/training/crouch.rs",
        "src/training/clatter.rs", "src/training/attack_angle.rs", "src/training/throw.rs", "src/training/fast_fall.rs", "src/training/full_hop.rs",
        "src/training/combo.rs", "src/training/frame_counter.rs",
    ]),
    ("Upstream: save states, reset, charges, buffs, items, fighter-specific restore, stage hazards", "upstream: clean-room spec (SPEC-state-*)", [
        "src/training/save_states.rs", "src/training/reset.rs", "src/training/charge.rs", "src/training/buff.rs",
        "src/training/character_specific/*", "src/hazard_manager/mod.rs",
    ]),
    ("Upstream: input recording/playback, input display, input delay, button chords, notifications", "upstream: clean-room spec (SPEC-input-*)", [
        "src/training/input_record.rs", "src/training/input_log.rs", "src/training/ui/input_log.rs", "src/training/input_delay.rs",
        "src/common/input.rs", "src/common/button_config.rs", "src/training/ui/notifications.rs", "src/training/ui/damage.rs", "src/training/ui/display.rs",
    ]),
    ("Upstream: boot, offsets, logging, menu system, persistence, layout archive, hitbox visualizer, dev config", "upstream: clean-room spec (SPEC-core-*)", [
        "src/lib.rs", "src/logging.rs", "src/common/mod.rs", "src/common/menu.rs", "src/common/filesystem.rs", "src/common/dev_config.rs",
        "src/common/offsets.rs", "src/common/consts.rs", "src/common/raygun_printer.rs", "src/hitbox_visualizer/mod.rs", "src/training/mod.rs",
        "src/training/ui/menu.rs", "src/training/ui/mod.rs", "src/training/debug.rs", "training_mod_tui/*", "training_mod_sync/*",
        "training_mod_consts/src/lib.rs", "training_mod_consts/src/options.rs", "training_mod_consts/src/config.rs", "training_mod_consts/src/files.rs",
        "training_mod_metrics/*",
    ]),
]


def classify(path):
    for feature, disposition, patterns in RULES:
        for pat in patterns:
            if fnmatch.fnmatchcase(path, pat):
                return feature, disposition
    return "UNMAPPED", "review"


def main():
    src, out_md, out_json = sys.argv[1:4]
    with open(src, encoding="utf-8") as f:
        data = json.load(f)
    groups = {}
    for row in data["files"]:
        feature, disposition = classify(row["path"])
        g = groups.setdefault(feature, {"disposition": disposition, "files": [], "origins": {}, "stage_lines": 0, "fork_only_lines": 0, "retained": 0})
        g["files"].append(row)
        g["origins"][row["origin"]] = g["origins"].get(row["origin"], 0) + 1
        g["stage_lines"] += row.get("stage_lines", 0) or 0
        if row["origin"] == "mixed":
            g["fork_only_lines"] += row.get("fork_only_lines", 0)
            g["retained"] += row.get("retained_upstream_lines", 0)
        elif row["origin"] == "ours-new":
            g["fork_only_lines"] += row.get("stage_lines", 0) or 0
        elif row["origin"] == "upstream-identical":
            g["retained"] += row.get("stage_lines", 0) or 0
    order = [r[0] for r in RULES] + ["UNMAPPED"]
    with open(out_md, "w", encoding="utf-8") as f:
        f.write("| Feature | Disposition | Files | ours-new | mixed | upstream-identical | other | fork-only lines | retained upstream lines |\n")
        f.write("| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n")
        for feature in order:
            g = groups.get(feature)
            if not g:
                continue
            o = g["origins"]
            other = sum(v for k, v in o.items() if k not in ("ours-new", "mixed", "upstream-identical"))
            f.write("| %s | %s | %d | %d | %d | %d | %d | %d | %d |\n" % (
                feature, g["disposition"], len(g["files"]), o.get("ours-new", 0), o.get("mixed", 0), o.get("upstream-identical", 0), other,
                g["fork_only_lines"], g["retained"]))
        f.write("\n## Mixed files by feature (fork-only ranges are stage line numbers)\n\n")
        for feature in order:
            g = groups.get(feature)
            if not g:
                continue
            mixed = [r for r in g["files"] if r["origin"] == "mixed"]
            if not mixed:
                continue
            f.write("### %s\n\n| File | stage lines | retained upstream | fork-only | fork-only ranges |\n| --- | ---: | ---: | ---: | --- |\n" % feature)
            for r in mixed:
                f.write("| `%s` | %d | %d | %d | %s |\n" % (r["path"], r["stage_lines"], r["retained_upstream_lines"], r["fork_only_lines"], r["fork_only_ranges"]))
            f.write("\n")
        un = groups.get("UNMAPPED")
        if un:
            f.write("## Unmapped\n\n")
            for r in un["files"]:
                f.write("- `%s` (%s)\n" % (r["path"], r["origin"]))
    with open(out_json, "w", encoding="utf-8") as f:
        json.dump({k: {"disposition": v["disposition"], "origins": v["origins"], "files": [r["path"] for r in v["files"]]} for k, v in groups.items()}, f, indent=1)
    for feature in order:
        g = groups.get(feature)
        if g:
            print("%4d  %s" % (len(g["files"]), feature))
    if "UNMAPPED" in groups:
        print("UNMAPPED:")
        for r in groups["UNMAPPED"]["files"]:
            print("   ", r["path"], r["origin"])


if __name__ == "__main__":
    main()
