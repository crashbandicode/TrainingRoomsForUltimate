#!/usr/bin/env python3
"""Generate src/model/sites_pkg3.rs: the guarded site rows of package 3 (save states), with their 64-byte
windows read from the owned 13.0.5 ELF.

    python3 -I tools/gen_sites_pkg3.py > src/model/sites_pkg3.rs

Each row is (name, feature, kind, symbol-or-offset, note). `kind` is one of: called, replace (a replace
hook on a function entry), inline-mid, relied, vtable. A symbol is looked up in the ELF's symbol table; an
offset is `0x...`. The rows and notes are the source of truth; regenerate after editing this file.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import elf_symbols as e  # noqa: E402

LUA = "_ZN3app8lua_bind"
# The symbols, exactly as the ELF names them (the generator checks each exists).
CALLED = [
    ("state_pos_y", "_ZN3app8lua_bind25PostureModule__pos_y_implEPNS_26BattleObjectModuleAccessorE", "PostureModule::pos_y: read at a Save."),
    ("state_lr", "_ZN3app8lua_bind22PostureModule__lr_implEPNS_26BattleObjectModuleAccessorE", "PostureModule::lr: read at a Save."),
    ("state_set_lr", "_ZN3app8lua_bind26PostureModule__set_lr_implEPNS_26BattleObjectModuleAccessorEf", "PostureModule::set_lr: the mirrored facing of a placement."),
    ("state_set_pos", "_ZN3app8lua_bind27PostureModule__set_pos_implEPNS_26BattleObjectModuleAccessorERKN3phx8Vector3fE", "PostureModule::set_pos: an airborne placement."),
    ("state_init_pos", "_ZN3app8lua_bind28PostureModule__init_pos_implEPNS_26BattleObjectModuleAccessorERKN3phx8Vector3fEbb", "PostureModule::init_pos: a grounded placement (resets ground contacts at the destination)."),
    ("state_clear_speed_all", "_ZN3app8lua_bind35KineticModule__clear_speed_all_implEPNS_26BattleObjectModuleAccessorE", "KineticModule::clear_speed_all: every placement frame."),
    ("state_heal", "_ZN3app8lua_bind23DamageModule__heal_implEPNS_26BattleObjectModuleAccessorEfi", "DamageModule::heal: sets the loaded damage (heal everything, then add)."),
    ("state_add_damage", "_ZN3app8lua_bind29DamageModule__add_damage_implEPNS_26BattleObjectModuleAccessorEfi", "DamageModule::add_damage: sets the loaded damage."),
    ("state_set_sleep", "_ZN3app8lua_bind35MotionAnimcmdModule__set_sleep_implEPNS_26BattleObjectModuleAccessorEb", "MotionAnimcmdModule::set_sleep: asleep after a respawn, awake while placing."),
    ("state_stop_all_sound", "_ZN3app8lua_bind32SoundModule__stop_all_sound_implEPNS_26BattleObjectModuleAccessorE", "SoundModule::stop_all_sound: silences the respawn and the placement."),
    ("state_pause_se_all", "_ZN3app8lua_bind30SoundModule__pause_se_all_implEPNS_26BattleObjectModuleAccessorEb", "SoundModule::pause_se_all."),
    ("state_play_se_no3d", "_ZN3app8lua_bind30SoundModule__play_se_no3d_implEPNS_26BattleObjectModuleAccessorEN3phx6Hash40Ebb", "SoundModule::play_se_no3d: se_system_position_reset (the CPU only)."),
    ("state_stop_rumble", "_ZN3app8lua_bind31ControlModule__stop_rumble_implEPNS_26BattleObjectModuleAccessorEb", "ControlModule::stop_rumble."),
    ("state_slow_clear_whole", "_ZN3app8lua_bind28SlowModule__clear_whole_implEPNS_26BattleObjectModuleAccessorE", "SlowModule::clear_whole: no lingering slow motion after a load."),
    ("state_camera_zoom_out", "_ZN3app8lua_bind27CameraModule__zoom_out_implEPNS_26BattleObjectModuleAccessorEi", "CameraModule::zoom_out."),
    ("state_camera_stop_quake", "_ZN3app8lua_bind29CameraModule__stop_quake_implEPNS_26BattleObjectModuleAccessorEi", "CameraModule::stop_quake: every quake kind."),
    ("state_effect_kill_kind", "_ZN3app8lua_bind28EffectModule__kill_kind_implEPNS_26BattleObjectModuleAccessorEN3phx6Hash40Ebb", "EffectModule::kill_kind: the critical-hit screen effect."),
    ("state_effect_remove_common", "_ZN3app8lua_bind32EffectModule__remove_common_implEPNS_26BattleObjectModuleAccessorEN3phx6Hash40E", "EffectModule::remove_common: the Shulk art flash cleanup."),
    ("state_effect_remove_screen", "_ZN3app8lua_bind32EffectModule__remove_screen_implEPNS_26BattleObjectModuleAccessorEN3phx6Hash40Ei", "EffectModule::remove_screen."),
    ("state_article_is_exist", "_ZN3app8lua_bind28ArticleModule__is_exist_implEPNS_26BattleObjectModuleAccessorEi", "ArticleModule::is_exist: the respawn cleanup visits every article index."),
    ("state_article_get_article", "_ZN3app8lua_bind31ArticleModule__get_article_implEPNS_26BattleObjectModuleAccessorEi", "ArticleModule::get_article."),
    ("state_article_remove_object", "_ZN3app8lua_bind42ArticleModule__remove_exist_object_id_implEPNS_26BattleObjectModuleAccessorEj", "ArticleModule::remove_exist_object_id."),
    ("state_article_object_id", "_ZN3app8lua_bind34Article__get_battle_object_id_implEPNS_7ArticleE", "Article::get_battle_object_id."),
    ("state_item_active_count", "_ZN3app8lua_bind44ItemManager__get_num_of_active_item_all_implEPNS_11ItemManagerE", "ItemManager::get_num_of_active_item_all: the items a cleanup removes."),
    ("state_item_active_item", "_ZN3app8lua_bind33ItemManager__get_active_item_implEPNS_11ItemManagerEm", "ItemManager::get_active_item."),
    ("state_item_object_id", "_ZN3app8lua_bind31Item__get_battle_object_id_implEPNS_4ItemE", "Item::get_battle_object_id."),
    ("state_item_remove", "_ZN3app8lua_bind37ItemManager__remove_item_from_id_implEPNS_11ItemManagerEj", "ItemManager::remove_item_from_id."),
    ("state_work_get_int", "_ZN3app8lua_bind24WorkModule__get_int_implEPNS_26BattleObjectModuleAccessorEi", "WorkModule::get_int."),
    ("state_work_set_int", "_ZN3app8lua_bind24WorkModule__set_int_implEPNS_26BattleObjectModuleAccessorEii", "WorkModule::set_int."),
    ("state_work_on_flag", "_ZN3app8lua_bind24WorkModule__on_flag_implEPNS_26BattleObjectModuleAccessorEi", "WorkModule::on_flag."),
    ("state_work_off_flag", "_ZN3app8lua_bind25WorkModule__off_flag_implEPNS_26BattleObjectModuleAccessorEi", "WorkModule::off_flag."),
    ("state_status_request", "_ZN3app8lua_bind40StatusModule__change_status_request_implEPNS_26BattleObjectModuleAccessorEib", "StatusModule::change_status_request: the placement handshake (WAIT / FALL)."),
    ("state_stage_id", "_ZN3app14sv_information8stage_idEv", "sv_information::stage_id: the stage a state is saved on and mirrored across."),
]

REPLACE = [
    ("state_get_param_int", "_ZN3app8lua_bind30WorkModule__get_param_int_implEPNS_26BattleObjectModuleAccessorEmm", "WorkModule::get_param_int: the respawn parameters of a death (SPEC-state-01 §4.1). A leaf thunk of 16 bytes: Skyline's far jump replaces exactly it."),
    ("state_set_dead_rumble", "_ZN3app15sv_fighter_util15set_dead_rumbleEP9lua_State", "sv_fighter_util::set_dead_rumble: no death rumble in Training (returns 0 without calling the original)."),
    ("state_req_quake", "_ZN3app8lua_bind28CameraModule__req_quake_implEPNS_26BattleObjectModuleAccessorEi", "CameraModule::req_quake: no quake while killing."),
    ("state_play_se", "_ZN3app8lua_bind25SoundModule__play_se_implEPNS_26BattleObjectModuleAccessorEN3phx6Hash40EbbbbNS_11SoundModule8enSETypeE", "SoundModule::play_se: se_silent while killing (death explosion, misfoot)."),
    ("state_effect_req", "_ZN3app8lua_bind22EffectModule__req_implEPNS_26BattleObjectModuleAccessorEN3phx6Hash40ERKNS3_8Vector3fES7_fjibi", "EffectModule::req: size 0 while loading (throw explosions, Ice Climbers squall)."),
    ("state_effect_req_follow", "_ZN3app8lua_bind29EffectModule__req_follow_implEPNS_26BattleObjectModuleAccessorEN3phx6Hash40ES4_RKNS3_8Vector3fES7_fbjiiiibb", "EffectModule::req_follow: sys_score_aura at size 0 while loading."),
]

OFFSETS = [
    # name, offset, kind, note
    ("state_no_star_ko", 0x2610A40, "replace", "The stage's can-star-KO query (the fork's can_futtobi_back): false in Training, so a death is followed by the respawn at once."),
]

# Milestone 2: charges (SPEC-state-02) and buffs (SPEC-state-03). (feature, name, symbol or offset, note)
CALLED_M2 = [
    ("StateCharge", "state_effect_req_common", "_ZN3app8lua_bind29EffectModule__req_common_implEPNS_26BattleObjectModuleAccessorEN3phx6Hash40Ef", "EffectModule::req_common: the generic charge_max flash of a full charge."),
    ("StateCharge", "state_face_motion", "_ZN3app11FighterUtil27set_face_motion_by_priorityERNS_26BattleObjectModuleAccessorENS_13FighterFacialEN3phx6Hash40E", "FighterUtil::set_face_motion_by_priority: Donkey Kong's full-charge face."),
    ("StateCharge", "state_article_generate_enable", "_ZN3app8lua_bind43ArticleModule__generate_article_enable_implEPNS_26BattleObjectModuleAccessorEibi", "ArticleModule::generate_article_enable: Sheik's needles."),
    ("StateCharge", "state_article_set_visibility", "_ZN3app8lua_bind34ArticleModule__set_visibility_implEPNS_26BattleObjectModuleAccessorEiN3phx6Hash40ES4_NS_22ArticleOperationTargetE", "ArticleModule::set_visibility: Sheik's needle count."),
    ("StateBuff", "state_motion_set_rate", "_ZN3app8lua_bind27MotionModule__set_rate_implEPNS_26BattleObjectModuleAccessorEf", "MotionModule::set_rate: Hero, Shulk and Wii Fit play their activation fast."),
    ("StateBuff", "state_jack_add_rebel", "_ZN3app23FighterSpecializer_Jack15add_rebel_gaugeERNS_26BattleObjectModuleAccessorENS_14FighterEntryIDEf", "FighterSpecializer_Jack::add_rebel_gauge: Joker's Arsene."),
    ("StateBuff", "state_visibility_set_status_default", "_ZN3app8lua_bind41VisibilityModule__set_status_default_implEPNS_26BattleObjectModuleAccessorEN3phx6Hash40ES4_", "VisibilityModule::set_status_default: Min Min's dragon arm."),
    ("StateBuff", "state_visibility_set", "_ZN3app8lua_bind26VisibilityModule__set_implEPNS_26BattleObjectModuleAccessorEN3phx6Hash40ES4_", "VisibilityModule::set: Min Min's dragon arm."),
    ("StateBuff", "state_effect_kill", "_ZN3app8lua_bind23EffectModule__kill_implEPNS_26BattleObjectModuleAccessorEjbb", "EffectModule::kill: Min Min's old arm effect."),
    ("StateBuff", "state_effect_set_scale", "_ZN3app8lua_bind28EffectModule__set_scale_implEPNS_26BattleObjectModuleAccessorEjRKN3phx8Vector3fE", "EffectModule::set_scale: Min Min's arm effect."),
    ("StateBuff", "state_motion_add_partial", "_ZN3app8lua_bind37MotionModule__add_motion_partial_implEPNS_26BattleObjectModuleAccessorEiN3phx6Hash40Effbbfbbb", "MotionModule::add_motion_partial: Min Min's dragon arm motion."),
]
OFFSETS_M2 = [
    ("StateBuff", "state_cloud_add_limit", 0x8DC160, "SeamKind::Called", "The game's add-limit routine (f32 limit, Fighter*, int): called to give Cloud his Limit; never hooked here."),
    ("StateBuff", "state_joker_summon", 0xB308D0, "REPLACE", "FighterSpecializer_Jack::check_doyle_summon_dispatch: no summon cutscene while buffing (returns 0xFFFFFFFF)."),
    ("StateBuff", "state_mac_meter", 0x68CDA0, "REPLACE", "The shared UI-update routine (entry data pointer, int value): Little Mac's KO meter reads 100 while a load with KO runs."),
    ("StateBuff", "state_wiifit_heal", 0x3FF9C0, "REPLACE", "The damage module's add-damage routine (module pointer, f32 amount, int kind): Wii Fit does not heal while buffing."),
    ("StateBuff", "state_req_screen", 0x2017BB0, "REPLACE", "EffectModule::req_screen: Sephiroth's wing background is replaced by bg while buffing."),
    ("StateBuff", "state_fighter_play_se", 0x4CF6C0, "REPLACE", "The fighter sound module's play_se: every sound is se_silent while a record is buffing."),
]

# Milestone 3: character items (SPEC-state-04) and the fighter-specific restores (SPEC-state-05).
CALLED_M3 = [
    ("StateItems", "state_is_have_item", "_ZN3app8lua_bind29ItemModule__is_have_item_implEPNS_26BattleObjectModuleAccessorEi", "ItemModule::is_have_item: whether the player holds an item."),
    ("StateItems", "state_get_have_item_id", "_ZN3app8lua_bind33ItemModule__get_have_item_id_implEPNS_26BattleObjectModuleAccessorEi", "ItemModule::get_have_item_id: the held item's object id."),
    ("StateItems", "state_remove_item", "_ZN3app8lua_bind28ItemModule__remove_item_implEPNS_26BattleObjectModuleAccessorEi", "ItemModule::remove_item: the restore replaces the held item."),
    ("StateItems", "state_set_have_item_action", "_ZN3app8lua_bind37ItemModule__set_have_item_action_implEPNS_26BattleObjectModuleAccessorEifi", "ItemModule::set_have_item_action: a turnip's special-init action with its power."),
    ("StateItems", "state_object_category", "_ZN3app7utility12get_categoryEj", "utility::get_category: the held object is an item."),
    ("StateItems", "state_object_kind", "_ZN3app7utility8get_kindEPKNS_26BattleObjectModuleAccessorE", "utility::get_kind: the held item's kind."),
    ("StateItems", "state_have_item", "_ZN3app8lua_bind26ItemModule__have_item_implEPNS_26BattleObjectModuleAccessorENS_8ItemKindEiibb", "ItemModule::have_item: path A of an item grant."),
    ("StateItems", "state_article_generate", "_ZN3app8lua_bind36ArticleModule__generate_article_implEPNS_26BattleObjectModuleAccessorEibi", "ArticleModule::generate_article: a turnip, and an Olimar Pikmin."),
    ("StateRestore", "state_sub_material_num", "_ZN3app25FighterSpecializer_Pickel16sub_material_numERNS_26BattleObjectModuleAccessorEii", "FighterSpecializer_Pickel::sub_material_num: subtracting zero refreshes Steve's HUD bar."),
    ("StateRestore", "state_work_get_int64", "_ZN3app8lua_bind26WorkModule__get_int64_implEPNS_26BattleObjectModuleAccessorEi", "WorkModule::get_int64: Steve's tool buffer and Olimar's troops manager."),
    ("StateRestore", "state_article_remove_exist", "_ZN3app8lua_bind32ArticleModule__remove_exist_implEPNS_26BattleObjectModuleAccessorEiNS_22ArticleOperationTargetE", "ArticleModule::remove_exist: Olimar's Pikmin before the line is rebuilt."),
    ("StateRestore", "state_article_active_num", "_ZN3app8lua_bind34ArticleModule__get_active_num_implEPNS_26BattleObjectModuleAccessorEi", "ArticleModule::get_active_num: gates the three Pikmin spawns."),
    ("StateRestore", "state_object_module_accessor", "_ZN3app16sv_battle_object15module_accessorEj", "sv_battle_object::module_accessor: a Pikmin's accessor from its battle object id."),
    ("StateRestore", "state_object_is_active", "_ZN3app16sv_battle_object9is_activeEj", "sv_battle_object::is_active: only active Pikmin contribute a colour."),
    ("StateRestore", "state_sora_change_magic", "_ZN3app24FighterSpecializer_Trail12change_magicERNS_7FighterE", "FighterSpecializer_Trail::change_magic: advances Sora's spell one step."),
]
REPLACE_M3 = [
    ("StateRestore", "state_pikmin_autonomy", 0x34B6CD0, "The Pikmin 'activate autonomy' routine (weapon, work module): called only for thrown or attacking Pikmin, so idle Pikmin keep their order."),
]
TURNIP_PROBES = [("daisy", 0x165BE40), ("peach", 0x1668C90)]

# The ItemManager singleton pointer is read through this guarded instruction pair (adrp + ldr of the
# singleton's slot, 0x52c4070).
RELIED = [
    ("state_item_manager_ref", 0x2DDA50, "The adrp / ldr pair that reads the ItemManager singleton's slot (`manager_slot_offset` recomputes the slot from these two words); relied on, never written."),
]


def rust_bytes(data):
    lines = []
    for i in range(0, 64, 16):
        lines.append("            " + ", ".join(f"0x{b:02x}" for b in data[i : i + 16]) + ",")
    return "\n".join(lines)


def main():
    elf, sections = e.load()
    table = {}
    for name, value in e.symbols(elf, sections):
        table.setdefault(name, value)
    out = []
    out.append("//! Package 3's site rows (save states): generated by `tools/gen_sites_pkg3.py` from the owned 13.0.5")
    out.append("//! ELF (symbol offsets and 64-byte windows). Do not edit by hand; edit the generator.")
    out.append("")
    out.append("use super::guard::{HookForm, Placement, SeamKind};")
    out.append("use super::sites::{Feature, Site};")
    out.append("")
    rows = []

    def emit(name, feature, kind, offset, note):
        data = e.window(elf, sections, offset)
        rows.append((name, offset))
        out.append("    Site {")
        out.append(f'        name: "{name}",')
        out.append(f"        feature: Feature::{feature},")
        out.append(f"        offset: {offset:#x},")
        out.append(f"        kind: {kind},")
        out.append("        guard: [")
        out.append(rust_bytes(data))
        out.append("        ],")
        out.append(f'        note: "{note}",')
        out.append("    },")

    body_start = len(out)
    for name, symbol, note in CALLED:
        emit(name, "StateCore", "SeamKind::Called", table[symbol], note)
    for name, symbol, note in REPLACE:
        emit(name, "StateCore", "SeamKind::Hook { form: HookForm::Replace, placement: Placement::Entry }", table[symbol], note)
    for name, offset, kind, note in OFFSETS:
        emit(name, "StateCore", "SeamKind::Hook { form: HookForm::Replace, placement: Placement::Entry }", offset, note)
    for feature, name, symbol, note in CALLED_M2:
        emit(name, feature, "SeamKind::Called", table[symbol], note)
    replace = "SeamKind::Hook { form: HookForm::Replace, placement: Placement::Entry }"
    for feature, name, offset, kind, note in OFFSETS_M2:
        emit(name, feature, replace if kind == "REPLACE" else kind, offset, note)
    for feature, name, symbol, note in CALLED_M3:
        emit(name, feature, "SeamKind::Called", table[symbol], note)
    for feature, name, offset, note in REPLACE_M3:
        emit(name, feature, replace, offset, note)
    for who, first in TURNIP_PROBES:
        for n in range(1, 9):
            emit(f"state_{who}_prob_{n}", "StateItems", replace, first + 0x20 * (n - 1), f"{who.capitalize()}'s turnip face {n} probability getter: certain for the chosen face while a grant is generating (SPEC-state-04 §5.3).")
    for who, first in (("daisy", 0x165BF40), ("peach", 0x1668D90)):
        for n in range(1, 9):
            emit(f"state_{who}_power_{n}", "StateItems", "SeamKind::Called", first + 0x20 * (n - 1), f"{who.capitalize()}'s turnip face {n} power getter: the power a restored turnip is thrown with.")
    for name, offset, note in RELIED:
        emit(name, "StateCore", "SeamKind::Relied", offset, note)
    body = out[body_start:]
    del out[body_start:]
    out.append(f"pub static STATE_SITES: [Site; {len(rows)}] = [")
    out.extend(body)
    out.append("];")
    out.append("")
    out.append("/// Offsets the runtime names (each is a row above; `tests/state_sites.rs` checks them against the ELF).")
    for name, offset in rows:
        out.append(f"pub const {name.upper()}_OFFSET: usize = {offset:#x};")
    print("\n".join(out))


if __name__ == "__main__":
    main()
