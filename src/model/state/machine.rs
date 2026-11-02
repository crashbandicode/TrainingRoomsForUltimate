//! The save / load machine (SPEC-state-01 §5): per fighter frame it reads facts the runtime took from the
//! game and answers what to do. No game call here; the runtime applies the answer.
//!
//! One record per (role, slot) carries a step marker. A load request sets both records of the chosen
//! slot to `KillPlayer`; each fighter then runs Kill -> WaitForAlive -> PosMove -> (waits) -> NoAction on
//! its own callback. The decisions of the open questions follow the specs' recommendations: a load
//! request is ignored while a load is in progress and the slot is latched at the request (OQ 5), no
//! random draw while idle (OQ 4), damage ends are (Lower, Upper) for both roles (OQ 6), one
//! recording/playback trigger when both records are done (OQ 7), every marker is cancelled when
//! Enable Save States is turned Off (§5.6).

use crate::model::cpu::rng::Rng;

use super::file::{SaveFile, SLOTS};
use super::held_item::HeldItem;
use super::place;
use super::record::{Charge, Marker, SavedState, Steve};
use super::settings::{RecordTrigger, StateSettings};

/// Which half of a slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Player,
    Cpu,
}

impl Role {
    pub const fn index(self) -> usize {
        match self {
            Role::Player => 0,
            Role::Cpu => 1,
        }
    }

    /// The role of a fighter entry: entry 1 is the CPU, entry 0 the player; others take no part
    /// (SPEC-state-01 §6.13).
    pub const fn of_entry(entry: i32) -> Option<Role> {
        match entry {
            0 => Some(Role::Player),
            1 => Some(Role::Cpu),
            _ => None,
        }
    }
}

/// Where the fighter's status is now, for the placement handshake.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusNow {
    Wait,
    Fall,
    /// `CLIFF_CATCH_MOVE` or `CLIFF_CATCH`.
    CliffCatch,
    Other,
}

/// The saved situation a record carries, as the three the load handles (anything else places at once).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Situation {
    Ground,
    Air,
    Cliff,
    Other,
}

/// The game's situation constants (resolved by name at run time).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SituationIds {
    pub ground: i32,
    pub air: i32,
    pub cliff: i32,
}

impl SituationIds {
    pub fn classify(&self, kind: i32) -> Situation {
        if kind == self.ground {
            Situation::Ground
        } else if kind == self.air {
            Situation::Air
        } else if kind == self.cliff {
            Situation::Cliff
        } else {
            Situation::Other
        }
    }
}

/// What the runtime knows about one fighter body on its frame.
#[derive(Clone, Copy, Debug)]
pub struct FrameIn {
    /// The fighter's entry id (`0` player, `1` CPU; others take no part).
    pub entry: i32,
    /// This body is Nana (the Ice Climbers' second body: shares Popo's record).
    pub nana: bool,
    /// This body is Popo.
    pub popo: bool,
    pub kind: i32,
    /// Dead in the sense of SPEC-state-01 §3 (including the Pokémon switching frame).
    pub dead: bool,
    pub status: StatusNow,
    /// Previous status was `REBIRTH`.
    pub prev_rebirth: bool,
    /// The fighter is in `STANDBY` (Nana before she is placed).
    pub standby: bool,
    pub stage_id: i32,
    pub situation: SituationIds,
    /// The Save chord passed and this callback consumed it.
    pub save_chord: bool,
    /// The Load chord passed and this callback consumed it (non-CPU-operated, non-Nana fighter only).
    pub load_chord: bool,
    pub kirby: bool,
    pub pokemon: bool,
    /// Hero, Cloud, Joker, Little Mac, Sephiroth, Wii Fit Trainer, Shulk, Min Min or Wario.
    pub buffable: bool,
}

/// The per-frame writes of the placement (SPEC-state-01 §5.3 steps 2 to 6).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placing {
    pub x: f32,
    pub y: f32,
    pub lr: f32,
    /// `init_pos` (resets ground contacts at the destination) rather than `set_pos`.
    pub grounded: bool,
    /// The status to request this frame (the handshake), if any.
    pub request: Option<Request>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    Wait,
    Fall,
}

/// What a fighter that has just been placed gets (§5.3 step 8).
#[derive(Clone, Debug, PartialEq)]
pub struct Done {
    /// The damage to set (heal everything, then add), if the Save Dmg setting says so.
    pub damage: Option<f32>,
    /// The player's held-item snapshot to restore (player record, not Nana).
    pub item: Option<HeldItem>,
    /// Write the saved charges (the saved kind equals the current kind).
    pub charge: Option<Charge>,
    /// Steve's record to write (the runtime ignores it for any other fighter).
    pub steve: Option<Steve>,
    /// Play `se_system_position_reset` (the CPU only).
    pub reset_sound: bool,
    /// The marker the fighter is left in (`NoAction`, or the wait of a feature).
    pub next: Marker,
    /// Pokémon Trainer: raise the trainer's `ENABLE_CHANGE_POKEMON` flag.
    pub pokemon_switch: bool,
}

/// A recording or playback to start now that the load is complete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    /// Start an input recording (Recording Trigger: Save State Load).
    Record,
    /// Start the playback of this slot.
    Playback(u8),
}

/// The answer for one fighter frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Out {
    /// A load was accepted this frame: stop any input-recording playback.
    pub stop_playback: bool,
    /// The Pokémon Trainer pre-death rule (SPEC-state-05 §5.1) applies to this fighter this frame.
    pub pokemon_predeath: bool,
    /// Force this fighter's status to DEAD.
    pub force_dead: bool,
    /// The respawn cleanup (SPEC-state-01 §5.2).
    pub cleanup: bool,
    pub placing: Option<Placing>,
    /// The training reset hub runs this frame (every PosMove frame).
    pub run_hub: bool,
    pub done: Option<Done>,
    /// A Save is captured for this fighter now (`slot`); the runtime reads the values and calls
    /// `Machine::capture`.
    pub capture: Option<usize>,
    /// The Save chord set both records (the slot the notification names).
    pub saved_notice: Option<usize>,
    pub trigger: Option<Trigger>,
    /// Both records of the load are done: `Some(saved_on_ledge)`, once per restore (the recorder's
    /// `on_state_restored`).
    pub restored: Option<bool>,
    /// The Ice Climbers hand-off happened: Nana is placed next.
    pub nana_next: bool,
}

/// The machine: both roles' slots and the load bookkeeping.
#[derive(Clone, Debug)]
pub struct Machine {
    pub file: SaveFile,
    /// The sign of the current load's mirroring (+1 or -1).
    pub mirror_sign: f32,
    /// The last slot drawn by Randomize Slots (initially slot 1).
    pub random_slot: usize,
    /// The slot of the load in progress, latched at the request.
    active: Option<usize>,
    /// A recording / playback trigger is waiting for the load to finish.
    trigger_pending: bool,
}

impl Default for Machine {
    fn default() -> Self {
        Self::new(SaveFile::default())
    }
}

impl Machine {
    pub fn new(file: SaveFile) -> Self {
        Self { file, mirror_sign: 1.0, random_slot: 0, active: None, trigger_pending: false }
    }

    fn records(&self, role: Role) -> &[SavedState; SLOTS] {
        match role {
            Role::Player => &self.file.player,
            Role::Cpu => &self.file.cpu,
        }
    }

    fn records_mut(&mut self, role: Role) -> &mut [SavedState; SLOTS] {
        match role {
            Role::Player => &mut self.file.player,
            Role::Cpu => &mut self.file.cpu,
        }
    }

    pub fn marker(&self, role: Role, slot: usize) -> Marker {
        self.records(role)[slot.min(SLOTS - 1)].marker
    }

    pub fn set_marker(&mut self, role: Role, slot: usize, marker: Marker) {
        self.records_mut(role)[slot.min(SLOTS - 1)].marker = marker;
    }

    pub fn record(&self, role: Role, slot: usize) -> &SavedState {
        &self.records(role)[slot.min(SLOTS - 1)]
    }

    /// The slot a frame consults: the latched slot of the load in progress, else Save State Slot.
    pub fn current_slot(&self, settings: &StateSettings) -> usize {
        self.active.unwrap_or(usize::from(settings.slot)).min(SLOTS - 1)
    }

    /// Either record of the load's slot is not `NoAction` (SPEC-state-01 §5.2).
    pub fn is_loading(&self) -> bool {
        self.active.is_some_and(|slot| {
            [Role::Player, Role::Cpu].iter().any(|role| self.marker(*role, slot) != Marker::NoAction)
        })
    }

    /// Either record of the load's slot is in `KillPlayer` or `WaitForAlive`.
    pub fn is_killing(&self) -> bool {
        self.active.is_some_and(|slot| {
            [Role::Player, Role::Cpu]
                .iter()
                .any(|role| matches!(self.marker(*role, slot), Marker::KillPlayer | Marker::WaitForAlive))
        })
    }

    /// Cancel every load in progress: all markers back to `NoAction` (Enable turned Off, scene
    /// departure, hot swap: SPEC-state-01 §5.6, §6.11).
    pub fn cancel_all(&mut self) {
        for role in [Role::Player, Role::Cpu] {
            for record in self.records_mut(role).iter_mut() {
                record.marker = Marker::NoAction;
            }
        }
        self.active = None;
        self.trigger_pending = false;
    }

    /// Accept a load request if none is in progress (OQ 5): pick the slot (a weighted random one when
    /// Randomize Slots has any, remembered), set both records to `KillPlayer`, choose the mirror sign.
    /// Returns whether the request was accepted.
    pub fn begin_load(&mut self, settings: &StateSettings, rng: &mut Rng) -> bool {
        if self.is_loading() {
            return false;
        }
        let slot = match settings.randomize.pick(rng) {
            Some(slot) => {
                self.random_slot = slot.min(SLOTS - 1);
                self.random_slot
            }
            None => usize::from(settings.slot).min(SLOTS - 1),
        };
        self.active = Some(slot);
        for role in [Role::Player, Role::Cpu] {
            self.set_marker(role, slot, Marker::KillPlayer);
        }
        self.mirror_sign = place::next_mirror_sign(settings.mirroring, self.mirror_sign, rng);
        self.trigger_pending = true;
        true
    }

    /// Store a Save's captured values (the marker stays `NoAction`). Returns whether the whole file is
    /// to be written now: neither record of the slot is still `Save` (§5.5 step 2).
    pub fn capture(&mut self, role: Role, slot: usize, mut state: SavedState) -> bool {
        let slot = slot.min(SLOTS - 1);
        state.marker = Marker::NoAction;
        self.records_mut(role)[slot] = state;
        [Role::Player, Role::Cpu].iter().all(|r| self.marker(*r, slot) != Marker::Save)
    }

    /// Run one fighter body's frame.
    pub fn frame(&mut self, input: &FrameIn, settings: &StateSettings, rng: &mut Rng) -> Out {
        let mut out = Out::default();
        let Some(role) = Role::of_entry(input.entry) else { return out };
        if !settings.enable {
            return out;
        }
        let slot = self.current_slot(settings);
        let marker = self.marker(role, slot);

        // 5.1 Load request.
        let auto = settings.autoload && marker == Marker::NoAction && input.dead;
        if (auto || input.load_chord) && !input.nana {
            out.stop_playback = self.begin_load(settings, rng);
            return out;
        }

        let slot = self.current_slot(settings);
        let marker = self.marker(role, slot);
        match marker {
            Marker::KillPlayer if !input.nana => {
                out.pokemon_predeath = input.pokemon;
                out.force_dead = !input.dead;
                self.set_marker(role, slot, Marker::WaitForAlive);
            }
            Marker::WaitForAlive => {
                out.pokemon_predeath = input.pokemon;
                if !input.dead && !input.nana {
                    out.cleanup = true;
                    self.set_marker(role, slot, Marker::PosMove);
                    self.place(role, slot, input, settings, rng, &mut out);
                }
            }
            Marker::PosMove if !input.nana => self.place(role, slot, input, settings, rng, &mut out),
            Marker::NanaPosMove if input.nana && !input.standby => self.place(role, slot, input, settings, rng, &mut out),
            _ => {}
        }

        // 5.5 Save, at the end of every callback.
        if input.save_chord {
            self.mirror_sign = 1.0;
            let save_slot = usize::from(settings.slot).min(SLOTS - 1);
            for r in [Role::Player, Role::Cpu] {
                self.set_marker(r, save_slot, Marker::Save);
            }
            out.saved_notice = Some(save_slot);
        }
        let save_slot = usize::from(settings.slot).min(SLOTS - 1);
        if self.marker(role, save_slot) == Marker::Save && !input.nana {
            self.set_marker(role, save_slot, Marker::NoAction);
            out.capture = Some(save_slot);
        }

        // 10. One recording / playback trigger when both records are done.
        if self.trigger_pending && !self.is_loading() {
            self.trigger_pending = false;
            if let Some(slot) = self.active.take() {
                // The restore is complete: the recorder is told once (pkg4 interface), with whether
                // the CPU was saved on the ledge; then the recording / playback trigger.
                out.restored = Some(self.record(Role::Cpu, slot).situation_kind == input.situation.cliff);
                out.trigger = self.trigger_for(slot, settings, rng);
            }
        }
        if !self.is_loading() {
            self.active = None;
        }
        out
    }

    fn trigger_for(&self, slot: usize, settings: &StateSettings, rng: &mut Rng) -> Option<Trigger> {
        if settings.record_trigger == RecordTrigger::SaveStateLoad {
            return Some(Trigger::Record);
        }
        // Ledge playback is left to the ledge-option feature: no trigger for a saved ledge state. The
        // CPU's half names the situation (the playback runs on the CPU).
        let ledge = self.record(Role::Cpu, slot).situation_kind;
        let _ = ledge;
        settings.playback.pick(rng).map(|slot| Trigger::Playback(slot as u8))
    }

    /// Steps 2 to 9 of §5.3 for one body.
    fn place(&mut self, role: Role, slot: usize, input: &FrameIn, settings: &StateSettings, rng: &mut Rng, out: &mut Out) {
        // Copy the values out (no clone: a record may hold a heap-backed Mii identity and this runs in a
        // frame hook).
        let (saved_x, saved_y, saved_lr, saved_situation, saved_percent, saved_kind, saved_charge, saved_steve, saved_item) = {
            let record = self.record(role, slot);
            (record.x, record.y, record.lr, record.situation_kind, record.percent, record.fighter_kind, record.charge, record.steve, record.held_item)
        };
        let situation = input.situation.classify(saved_situation);
        let mut y = saved_y;
        if situation == Situation::Cliff {
            y += place::ledge_raise(input.kind);
        }
        let x = place::target_x(saved_x, self.mirror_sign, input.stage_id);
        let lr = place::target_lr(saved_lr, self.mirror_sign);
        let request = match situation {
            Situation::Ground if input.status != StatusNow::Wait => Some(Request::Wait),
            Situation::Air if input.status != StatusNow::Fall => Some(Request::Fall),
            Situation::Cliff if !matches!(input.status, StatusNow::Fall | StatusNow::CliffCatch) => Some(Request::Fall),
            _ => None,
        };
        out.placing = Some(Placing { x, y, lr, grounded: situation == Situation::Ground, request });
        out.run_hub = true;
        if request.is_some() {
            return;
        }

        // Placed: 8a-8g.
        let (mode, limits) = match role {
            Role::Player => (settings.damage_player, settings.limits_player),
            Role::Cpu => (settings.damage_cpu, settings.limits_cpu),
        };
        let mut next = Marker::NoAction;
        let charge = (saved_kind == input.kind).then_some(saved_charge);
        if input.kirby {
            next = Marker::WaitForCopyAbility;
        }
        if input.buffable {
            next = Marker::ApplyBuff;
        }
        let pokemon_switch = input.pokemon;
        if pokemon_switch {
            next = Marker::WaitForPokemonSwitch;
        }
        out.done = Some(Done {
            damage: place::load_damage(mode, saved_percent, limits, rng),
            item: (role == Role::Player && !input.nana).then_some(saved_item).flatten(),
            charge,
            steve: saved_steve,
            reset_sound: role == Role::Cpu,
            next,
            pokemon_switch,
        });
        // 9. The Ice Climbers hand-off: Popo placed out of REBIRTH hands the record to Nana.
        if input.popo && input.prev_rebirth {
            next = Marker::NanaPosMove;
            out.nana_next = true;
        }
        self.set_marker(role, slot, next);
    }
}
