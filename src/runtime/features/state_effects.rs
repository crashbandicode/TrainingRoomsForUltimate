//! Executors of the save states' charge plans and buff steps (SPEC-state-02, SPEC-state-03): they turn
//! the pure model's answers into game calls. No allocation, no lock held. NOT YET SWITCH-RUN.

use crate::model::cpu::hash::hash40;
use crate::model::state::buff::{Art, BuffKind, BuffOut, Spell, Waft};
use crate::model::state::charge::{self, Joint, Op, Plan, Step};
use crate::model::state::ids::{Table, C};
use crate::runtime::features::state_restore;
use crate::runtime::game::natives;
use crate::runtime::state_natives as n;

/// A game constant, `None` when it did not resolve.
fn id(ids: &Table, c: C) -> Option<i32> {
    ids.resolved(c).then(|| ids.get(c))
}

unsafe fn follow(accessor: usize, ids: &Table, effect: &str, joint: u64, pos: [f32; 3], rot: [f32; 3], attribute: u32, lifetime: i32, handle: Option<C>) {
    let pos = n::Vector3f { x: pos[0], y: pos[1], z: pos[2] };
    let rot = n::Vector3f { x: rot[0], y: rot[1], z: rot[2] };
    let made = n::effect_req_follow(accessor, hash40(effect), joint, &pos, &rot, attribute, lifetime);
    if let Some(slot) = handle.and_then(|c| id(ids, c)) {
        natives::set_int(accessor, made as i32, slot);
    }
}

/// Execute a charge plan on one fighter (SPEC-state-02 §4).
///
/// # Safety
/// The charge sites passed their guards; `accessor` is a live fighter's module accessor.
pub unsafe fn run_charge(accessor: usize, fighter: usize, ids: &Table, plan: &Plan) {
    for step in plan.iter() {
        match step {
            Step::SetInt(c, value) => {
                if let Some(slot) = id(ids, c) {
                    natives::set_int(accessor, value, slot);
                }
            }
            Step::SetFloat(c, value) => {
                if let Some(slot) = id(ids, c) {
                    natives::set_float(accessor, value, slot);
                }
            }
            Step::SetFlag(c, value) => {
                if let Some(slot) = id(ids, c) {
                    natives::set_flag(accessor, value, slot);
                }
            }
            Step::Visual(Op::ChargeMax) => n::effect_req_common(accessor, hash40("charge_max")),
            Step::Visual(Op::RemoveChargeMax) => n::effect_remove_common(accessor, hash40("charge_max")),
            Step::Visual(Op::FaceMotion) => {
                if let Some(facial) = id(ids, C::FacialSpecial) {
                    n::face_motion(accessor, facial, hash40("special_n_max_face"));
                }
            }
            Step::Visual(Op::Follow { effect, joint, pos, rot, attribute, lifetime, handle }) => {
                let joint = match joint {
                    Joint::Name(name) => hash40(name),
                    Joint::Raw(raw) => raw,
                };
                follow(accessor, ids, effect, joint, pos, rot, attribute, lifetime, handle);
            }
            Step::Visual(Op::FollowRaw { effect, joint, lifetime, handle }) => {
                let zero = n::Vector3f { x: 0.0, y: 0.0, z: 0.0 };
                let made = n::effect_req_follow(accessor, effect, joint, &zero, &zero, 0, lifetime);
                if let Some(slot) = handle.and_then(|c| id(ids, c)) {
                    natives::set_int(accessor, made as i32, slot);
                }
            }
            Step::SheikNeedles(count) => {
                if let Some(article) = id(ids, C::SheikNeedleArticle) {
                    n::article_generate_enable(accessor, article);
                    n::article_set_visibility(accessor, article, hash40("set_main"), hash40(charge::sheik_group(count)));
                }
            }
            Step::OlimarLine(line) => state_restore::olimar_respawn(accessor, ids, &line),
            Step::SoraSpell(saved) => state_restore::sora_spell(accessor, fighter, ids, saved),
            Step::RevengeAura(joint) => {
                follow(accessor, ids, "gaogaen_revenge_aura", joint, [0.0; 3], [0.0; 3], 32768, -1, None);
            }
        }
    }
}

/// The common silencing of every `ApplyBuff` frame (SPEC-state-03 §4).
unsafe fn silence(accessor: usize, ids: &Table) {
    n::stop_all_sound(accessor);
    n::stop_rumble(accessor, false);
    n::set_sleep(accessor, false);
    for quake in [C::QuakeKindM, C::QuakeKindS] {
        if let Some(kind) = id(ids, quake) {
            n::camera_stop_quake(accessor, kind);
        }
    }
}

/// The game's own status ids for a buff's activation.
pub fn buff_kind(ids: &Table, kind: i32) -> Option<BuffKind> {
    [
        (C::KindHero, BuffKind::Hero),
        (C::KindCloud, BuffKind::Cloud),
        (C::KindJoker, BuffKind::Joker),
        (C::KindLittleMac, BuffKind::LittleMac),
        (C::KindSephiroth, BuffKind::Sephiroth),
        (C::KindWiiFit, BuffKind::WiiFit),
        (C::KindShulk, BuffKind::Shulk),
        (C::KindMinMin, BuffKind::MinMin),
        (C::KindWario, BuffKind::Wario),
    ]
    .into_iter()
    .find(|(c, _)| ids.is(*c, kind))
    .map(|(_, k)| k)
}

/// Execute one `ApplyBuff` frame's answer.
///
/// # Safety
/// The buff sites passed their guards; `accessor` / `fighter` are a live fighter's.
pub unsafe fn run_buff(accessor: usize, fighter: usize, entry: i32, ids: &Table, out: &BuffOut) {
    silence(accessor, ids);
    if let Some(spell) = out.hero_cast {
        let command = match spell {
            Spell::Accelerate => C::HeroSpellAccelerate,
            Spell::Oomph => C::HeroSpellOomph,
            Spell::Psyche => C::HeroSpellPsyche,
            Spell::Bounce => C::HeroSpellBounce,
        };
        if let (Some(slot), Some(value), Some(status)) = (id(ids, C::HeroDecideCommand), id(ids, command), id(ids, C::HeroStatusSpecialLwStart)) {
            natives::set_int(accessor, value, slot);
            // `true` blocks shielding out of the cast.
            natives::change_status_force(accessor, status, true);
        }
    }
    if out.add_limit {
        n::cloud_add_limit(accessor, 100.0);
    }
    if out.add_rebel {
        n::jack_add_rebel(accessor, entry, 120.0);
    }
    if out.ko_gauge {
        if let Some(slot) = id(ids, C::LittleMacKoGauge) {
            natives::set_float(accessor, 100.0, slot);
        }
    }
    for (wanted, on) in [(out.wing_flags_on, true), (out.wing_flags_off, false)] {
        if wanted {
            for flag in [C::SephirothRuleHp, C::SephirothSuddenDeath] {
                if let Some(slot) = id(ids, flag) {
                    natives::set_flag(accessor, on, slot);
                }
            }
        }
    }
    if let Some(level) = out.waft {
        let (param, level_c) = match level {
            Waft::Mini => ("gass_middle_time", C::WarioGassLevelM),
            Waft::Half => ("gass_large_time", C::WarioGassLevelL),
            Waft::Full => ("gass_max_time", C::WarioGassLevelFly),
        };
        let seconds = natives::get_param_float(accessor, hash40("param_special_lw"), hash40(param));
        if let (Some(count_id), Some(level_id), Some(level_value)) = (id(ids, C::WarioGassCount), id(ids, C::WarioGassLevel), id(ids, level_c)) {
            natives::set_int(accessor, (seconds * 60.0) as i32, count_id);
            natives::set_int(accessor, level_value, level_id);
        }
    }
    if let Some(art) = out.shulk_write {
        let art_c = match art {
            Art::Jump => C::ShulkArtJump,
            Art::Speed => C::ShulkArtSpeed,
            Art::Shield => C::ShulkArtShield,
            Art::Buster => C::ShulkArtBuster,
            Art::Smash => C::ShulkArtSmash,
        };
        if let (Some(type_id), Some(art_value), Some(timer), Some(flag), Some(status)) = (
            id(ids, C::ShulkTypeSelect),
            id(ids, art_c),
            id(ids, C::ShulkSelectTimer),
            id(ids, C::ShulkSelectFlag),
            id(ids, C::ShulkStatusAction),
        ) {
            natives::set_int(accessor, art_value, type_id);
            natives::set_int(accessor, 29, timer);
            n::work_on_flag(accessor, flag);
            natives::change_status_force(accessor, status, true);
        }
    }
    if out.wii_force {
        if let Some(status) = id(ids, C::WiiFitStatusSuccess) {
            natives::change_status_force(accessor, status, false);
        }
    }
    if let Some(rate) = out.motion_rate {
        n::motion_set_rate(accessor, rate);
    }
    if out.minmin {
        min_min_dragon(accessor, ids);
    }
}

/// Min Min's Power Dragon arm: the sequence of the game's own routine, with its raw hash40 values as
/// opaque constants (SPEC-state-03 open question 6).
unsafe fn min_min_dragon(accessor: usize, ids: &Table) {
    n::visibility_set_status_default(accessor, 0x59a6ef56c, 0x9b7cb3e40);
    n::visibility_set_status_default(accessor, 0xa9ffaf181, 0xef190b4e8);
    if let Some(spiral) = id(ids, C::MinMinSpiralLeft) {
        if n::article_get(accessor, spiral) != 0 {
            n::visibility_set_status_default(accessor, 0x6ec1f4d21, 0xa9aba8db6);
            n::visibility_set(accessor, 0x59a6ef56c, 0xadd214353);
        }
    }
    let big = n::get_param_int(accessor, 0xdf05c072b, 0xf4fd45d48);
    if let (Some(slot), Some(flag), Some(handle_slot), Some(part)) =
        (id(ids, C::MinMinArmBigFrame), id(ids, C::MinMinDragonizeL), id(ids, C::MinMinReinforceHandle), id(ids, C::MinMinPartDragon))
    {
        natives::set_int(accessor, big, slot);
        n::work_on_flag(accessor, flag);
        let old = natives::get_int(accessor, handle_slot);
        if old != 0 {
            n::effect_kill(accessor, old as u32);
        }
        let handle = natives::effect_req(accessor, 0x12600df9d4, &[0.0; 3], &[0.0; 3], 1.0, (0, -1, false, 0));
        n::effect_set_scale_one(accessor, handle as u32);
        natives::set_int(accessor, handle as i32, handle_slot);
        n::motion_add_partial_dragon(accessor, part, 0xc86296416);
    }
}
