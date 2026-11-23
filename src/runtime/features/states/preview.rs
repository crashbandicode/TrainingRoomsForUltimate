//! Observe the live pair and its identity for setup operations.
use super::{Raw, Role};
use crate::engine::mii::lifetime::Configuration;
use crate::runtime::{game::natives, lobby::training::practice};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Identity {
    pub(super) entries: [usize; 2],
    pub(super) kinds: [i32; 2],
    pub(super) stage: i32,
    pub(super) miis: [Option<Configuration>; 2],
}

const NOT_READY: &str = "Wait for both fighters to finish loading";

pub(super) unsafe fn observe() -> Result<(Identity, [Raw; 2]), &'static str> {
    if !practice::enabled() || !practice::owned() {
        return Err(NOT_READY);
    }
    let context = practice::context().ok_or(NOT_READY)?;
    let entries = [
        practice::entry(0).ok_or(NOT_READY)?,
        practice::entry(1).ok_or(NOT_READY)?,
    ];
    let mut raws = [None, None];
    let mut miis = [None; 2];
    for (index, role) in [Role::Player, Role::Cpu].into_iter().enumerate() {
        // The running body, the one the Save chord records: after a live swap the entry's first body is not the fighter on
        // screen, and a setup of the fighters on screen was refused (CODE-FINDINGS #25).
        let fighter = natives::entry_active_body(entries[index]);
        let accessor = natives::fighter_accessor(fighter);
        if fighter == 0 || accessor == 0 {
            return Err(NOT_READY);
        }
        let kind = ((fighter + super::FIGHTER_KIND_FIELD) as *const i32).read();
        if crate::model::state::setup::MII_KINDS.contains(&kind) {
            miis[index] =
                crate::runtime::roster::character_roster::current_mii_configuration(index)
                    .filter(|mii| mii.canonical().class as i32 == kind);
            if miis[index].is_none() {
                return Err("Wait for the exact Mii before previewing");
            }
        }
        raws[index] = Some(Raw {
            entry: if index == 0 { 0 } else { context.cpu_entry() },
            role,
            kind,
            accessor,
            fighter,
            status: natives::status_kind(accessor),
            prev_status: natives::prev_status_kind(accessor, 0),
            situation: natives::situation_kind(accessor),
        });
    }
    let [Some(player), Some(cpu)] = raws else {
        return Err(NOT_READY);
    };
    let identity = Identity {
        entries,
        kinds: [player.kind, cpu.kind],
        stage: natives::stage_id(),
        miis,
    };
    Ok((identity, [player, cpu]))
}
