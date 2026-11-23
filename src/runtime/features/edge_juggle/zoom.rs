//! Hero's stock Command Selection and Zoom, through the existing practice callbacks.
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::model::{
    cpu::ids::C,
    edge_juggle::{
        perfect::{Correction, FLICK_JUMP, SPECIAL},
        picker::HERO,
        zoom::{force_zoom, Attempt, Flight, Observation, PadIntent, PadState, COMMAND as ZOOM, COMMAND_COUNT},
    },
    sites_pkg2 as sites,
    sites_pkg3,
    sites_pkg4,
};
use crate::runtime::{game::{self, natives}, state_natives};

use super::konst;

// 13.0.5 FighterSpecializer_Brave::get_special_lw_command_from_index, and
// special_lw_open_command's queue writer. Only the current owned Hero uses these fields.
const COMMAND_FROM_INDEX: usize = 0x85c200;
const QUEUE_CAPACITY: usize = 0xb8;
const QUEUE_LEN: usize = 0xc0;
const QUEUE_HEAD: usize = 0xc8;
const QUEUE_STORAGE: usize = 0xd8;
const MP: i32 = 0x53;
const ZOOM_COST: f32 = 8.0;
const CONT_SPECIAL_LW: i32 = 0x1e000066;
const FLYING_START: i32 = 0x1f5;
const FLYING_UP: i32 = 0x1f6;
const FLYING_DOWN: i32 = 0x1f7;
const FLYING_END: i32 = 0x1f8;
const FLYING_STOP_CEIL: i32 = 0x1f9;
const LINK_NO_CAPTURE: i32 = 0;

static BASE: AtomicUsize = AtomicUsize::new(0);

pub(super) fn install() {
    if crate::runtime::seams::seam_install::gate(
        "edge_zoom",
        &["edge_zoom.command_at", "edge_zoom.command_queue", "edge_zoom.ai_command_list"],
        false,
    ) && [
        sites::WORK_GET_FLOAT_OFFSET,
        sites::WORK_SET_FLOAT_OFFSET,
        sites::WORK_GET_INT_OFFSET,
        sites_pkg4::WORK_SET_INT_OFFSET,
        sites::TRANSITION_TERM_HOOK_OFFSET,
        sites_pkg3::STATE_LINK_PARENT_OBJECT_ID_OFFSET,
        crate::model::sites_hitbox::DAMAGE_RECORD_OFFSET,
        crate::model::sites_hitbox::DAMAGE_HIT_IDS_OFFSET,
    ].into_iter().all(game::granted) {
        BASE.store(game::text_base(), Ordering::Release);
    }
}

pub(super) fn installed() -> bool {
    BASE.load(Ordering::Acquire) != 0
}

#[derive(Default)]
pub(super) struct NativeAttempt {
    pub(super) attempt: Attempt,
    opening: bool,
    release_confirm: bool,
    bonked: bool,
    captured: bool,
}

impl NativeAttempt {
    pub(super) unsafe fn zoom_pad(&mut self, cpu: usize, buttons: u32) -> Correction {
        if !installed() {
            return Correction::default();
        }
        let mut correction = Correction {
            stick: Some([0.0, 0.0]),
            release: buttons & !FLICK_JUMP,
            ..Correction::default()
        };
        let base = BASE.load(Ordering::Acquire);
        if natives::fighter_kind(cpu) != Some(HERO as i32) {
            return correction;
        }
        let status = natives::status_kind(cpu);
        if status != konst(C::HeroSelect) {
            self.release_confirm = false;
            let can_open = super::airborne_recovery(cpu, status)
                && natives::is_enable_transition_term(cpu, CONT_SPECIAL_LW);
            let state = if can_open { PadState::CanOpen } else { PadState::Busy };
            if self.attempt.pad(state) != PadIntent::Open {
                self.opening = false;
            } else {
                // Alternate request and release if an actionable frame didn't consume the edge.
                self.opening = !self.opening;
                if self.opening {
                    let mp = natives::get_float(cpu, MP);
                    if mp.is_finite() && mp < ZOOM_COST {
                        natives::set_float(cpu, ZOOM_COST, MP);
                    }
                    correction.stick = Some([0.0, -1.0]);
                    correction.press = SPECIAL;
                }
            }
            return correction;
        }
        let [window, open, index, first, second, third, fourth] = [
            C::HeroWindowState, C::HeroWindowOpen, C::HeroSelectIndex, C::HeroCommand1,
            C::HeroCommand2, C::HeroCommand3, C::HeroCommand4,
        ].map(konst);
        if [window, open, index, first, second, third, fourth].contains(&super::UNRESOLVED)
            || natives::get_int(cpu, window) != open
        {
            return correction;
        }
        let zoom_selected = force_selected(base, cpu, index, [first, second, third, fourth]);
        if self.attempt.pad(PadState::Menu { open: true, zoom_selected }) == PadIntent::Confirm {
            self.release_confirm = !self.release_confirm;
            if self.release_confirm {
                correction.press = SPECIAL;
            }
        }
        correction
    }

    pub(super) unsafe fn observe_zoom(
        &mut self, p1: usize, cpu: usize, status: i32, observed: super::Observation,
    ) -> Observation {
        if !installed() {
            return Observation::default();
        }
        let flight = match status {
            FLYING_START | FLYING_UP => Flight::Ascent,
            FLYING_DOWN | FLYING_END => Flight::Descent,
            FLYING_STOP_CEIL => {
                self.bonked = true;
                Flight::Bonk
            }
            _ if self.bonked && (super::falling(status) || observed.grounded) => Flight::Descent,
            _ => Flight::Other,
        };
        let player = natives::battle_object_id(p1);
        let hit = observed.hit && super::super::input_display::struck(cpu).is_some_and(|(id, _, _)| {
            id == player || super::super::input_display::attacker_owner(id) == Some(player)
        });
        let grabbed = [
            C::ClatterCapturePulled, C::ClatterCaptureWait, C::ClatterCapturePulledYoshi,
            C::ClatterCaptureWaitYoshi,
        ].map(konst).contains(&status)
            && status != super::UNRESOLVED
            && state_natives::link_parent_object_id(cpu, LINK_NO_CAPTURE) as u32 == player;
        let contact = hit || grabbed && !self.captured;
        self.captured = grabbed;
        Observation { flight, grounded: observed.grounded, contact, ended: false }
    }
}

/// Edit the game-owned list position the native cursor already selects. If Zoom
/// rolled elsewhere, exchange the two entries and their signed AI availability.
unsafe fn force_selected(base: usize, cpu: usize, index: i32, work: [i32; COMMAND_COUNT]) -> bool {
    let selected = natives::get_int(cpu, index) as usize;
    if selected >= COMMAND_COUNT {
        return false;
    }
    let fighter = natives::entry_active_body(natives::fighter_entry(super::cpu_entry()));
    if fighter == 0 || natives::fighter_accessor(fighter) != cpu {
        return false;
    }
    let field = |offset| ((fighter + offset) as *const usize).read();
    let capacity = field(QUEUE_CAPACITY);
    let head = field(QUEUE_HEAD);
    let storage = field(QUEUE_STORAGE) as *mut u8;
    if capacity < COMMAND_COUNT || head >= capacity
        || field(QUEUE_LEN) != COMMAND_COUNT || storage.is_null()
    {
        return false;
    }
    let at = |slot| storage.add(head.wrapping_add(slot) % capacity);
    let commands = std::array::from_fn(|slot| at(slot).read());
    let Some(forced) = force_zoom(commands, selected) else { return false };
    let selected_mirror = natives::get_int(cpu, work[selected]);
    for (slot, desired) in forced.into_iter().enumerate() {
        if desired != commands[slot] {
            at(slot).write(desired);
            if slot != selected {
                natives::set_int(cpu, selected_mirror, work[slot]);
            }
        }
    }
    if selected_mirror != i32::from(ZOOM) {
        natives::set_int(cpu, i32::from(ZOOM), work[selected]);
    }
    let get_command: extern "C" fn(usize, i32) -> u8 = std::mem::transmute(base + COMMAND_FROM_INDEX);
    get_command(fighter, selected as i32) == ZOOM
        && natives::get_int(cpu, work[selected]) == i32::from(ZOOM)
}
