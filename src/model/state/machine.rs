//! The save / load machine (SPEC-state-01 §5): per fighter frame it reads facts the runtime took from the
//! game and answers what to do. No game call here; the runtime applies the answer.
//!
//! One record per (role, slot) carries a step marker. A load request sets both records of the chosen
//! slot to `KillPlayer`; each fighter then runs Kill -> WaitForAlive -> PosMove -> (waits) -> NoAction on
//! its own callback. The decisions of the open questions follow the specs' recommendations: a load
//! request is ignored while a load is in progress and the slot is latched at the request (OQ 5), no
//! random draw while idle (OQ 4), damage ends are (Lower, Upper) for both roles (OQ 6), one
//! recorder hand-off (`Out::restored`, package 4 decides the recording or playback, OQ 7) when both records are done, every marker is cancelled when
//! Enable Save States is turned Off (§5.6).

use crate::model::cpu::rng::Rng;

use super::file::{SaveFile, SLOTS};
use super::held_item::HeldItem;
use super::place;
use super::record::{Charge, Marker, SavedState, Steve};
use super::settings::StateSettings;

/// Which half of a slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Player,
    Cpu,
}

impl Role {
    pub fn for_scene(scene: crate::model::cpu::role::Scene, entry: i32) -> Option<Role> {
        let cpu = match scene {
            crate::model::cpu::role::Scene::Training => 1,
            crate::model::cpu::role::Scene::Lobby => crate::model::cpu::role::LOBBY_CPU_ENTRY,
        };
        if entry == 0 {
            Some(Role::Player)
        } else if entry == cpu {
            Some(Role::Cpu)
        } else {
            None
        }
    }

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
    /// This body is Popo and Nana exists (the runtime reports `false` for a Popo that Solo Popo left without a Nana: there is no
    /// second body to hand the record to).
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
    /// The fighter's record waits on a feature (`ApplyBuff`, `WaitForPokemonSwitch`,
    /// `WaitForCopyAbility`): the runtime runs that feature's step this frame.
    pub wait: Option<Marker>,
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
    /// The recorder has not been told yet that the load finished.
    restore_pending: bool,
    /// Ephemeral restore records. Never part of `file` or the persistence handoff.
    temporary: Option<[SavedState; 2]>,
}

impl Default for Machine {
    fn default() -> Self {
        Self::new(SaveFile::default())
    }
}

impl Machine {
    pub fn new(file: SaveFile) -> Self {
        Self {
            file,
            mirror_sign: 1.0,
            random_slot: 0,
            active: None,
            restore_pending: false,
            temporary: None,
        }
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

    /// Record of the running restore; numbered-slot readers keep using `record`.
    pub fn restoring_record(&self, role: Role, settings: &StateSettings) -> &SavedState {
        self.load_record(role, self.current_slot(settings))
    }

    fn load_record(&self, role: Role, slot: usize) -> &SavedState {
        self.temporary
            .as_ref()
            .map_or_else(|| self.record(role, slot), |pair| &pair[role.index()])
    }

    fn load_marker(&self, role: Role, slot: usize) -> Marker {
        self.load_record(role, slot).marker
    }

    fn set_load_marker(&mut self, role: Role, slot: usize, marker: Marker) {
        if let Some(pair) = &mut self.temporary {
            pair[role.index()].marker = marker;
        } else {
            self.set_marker(role, slot, marker);
        }
    }

    pub fn temporary_active(&self) -> bool {
        self.temporary.is_some()
    }

    /// Start an already-captured pair without borrowing a numbered slot, drawing
    /// a random slot or changing its mirror history. Ownership moves, no clone.
    pub fn begin_temporary(&mut self, mut pair: [SavedState; 2]) -> bool {
        if self.is_loading()
            || self.temporary.is_some()
            || self
                .file
                .player
                .iter()
                .chain(self.file.cpu.iter())
                .any(|r| r.marker == Marker::Save)
        {
            return false;
        }
        for record in &mut pair {
            record.marker = Marker::KillPlayer;
        }
        self.temporary = Some(pair);
        // This index is only bookkeeping for the existing per-role machine;
        // every load access above selects the temporary pair while it exists.
        self.active = Some(0);
        self.restore_pending = true;
        true
    }

    /// The slot a frame consults: the latched slot of the load in progress, else Save State Slot.
    pub fn current_slot(&self, settings: &StateSettings) -> usize {
        self.active
            .unwrap_or(usize::from(settings.slot))
            .min(SLOTS - 1)
    }

    /// Either record of the load's slot is not `NoAction` (SPEC-state-01 §5.2).
    pub fn is_loading(&self) -> bool {
        self.active.is_some_and(|slot| {
            [Role::Player, Role::Cpu]
                .iter()
                .any(|role| self.load_marker(*role, slot) != Marker::NoAction)
        })
    }

    /// Either record of the load's slot is in `KillPlayer` or `WaitForAlive`.
    pub fn is_killing(&self) -> bool {
        self.active.is_some_and(|slot| {
            [Role::Player, Role::Cpu].iter().any(|role| {
                matches!(
                    self.load_marker(*role, slot),
                    Marker::KillPlayer | Marker::WaitForAlive
                )
            })
        })
    }

    /// A feature's wait is over: the load's record for `role` goes back to `NoAction`.
    pub fn finish_wait(&mut self, role: Role) {
        if let Some(slot) = self.active {
            self.set_load_marker(role, slot, Marker::NoAction);
        }
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
        self.restore_pending = false;
        self.temporary = None;
    }

    /// Accept a load request if none is in progress (OQ 5): pick the slot (a weighted random one when
    /// Randomize Slots has any, remembered), set both records to `KillPlayer`, choose the mirror sign.
    /// Returns whether the request was accepted.
    pub fn begin_load(&mut self, settings: &StateSettings, rng: &mut Rng) -> bool {
        if self.is_loading() || self.temporary.is_some() {
            return false;
        }
        let slot = match settings.randomize.pick(rng) {
            Some(slot) => {
                self.random_slot = slot.min(SLOTS - 1);
                self.random_slot
            }
            None => usize::from(settings.slot).min(SLOTS - 1),
        };
        // The active slot is the bound save: nothing bound leaves it empty, and an empty slot loads nothing
        // (RECORDING.md: "If none is bound, it does nothing").
        if !self.record(Role::Player, slot).is_saved() && !self.record(Role::Cpu, slot).is_saved() {
            return false;
        }
        self.start_load(slot, settings, rng);
        true
    }

    fn start_load(&mut self, slot: usize, settings: &StateSettings, rng: &mut Rng) {
        self.active = Some(slot);
        for role in [Role::Player, Role::Cpu] {
            self.set_marker(role, slot, Marker::KillPlayer);
        }
        self.mirror_sign = place::next_mirror_sign(settings.mirroring, self.mirror_sign, rng);
        self.restore_pending = true;
    }

    /// A setup transaction never replaces half a pair or redirects a pending
    /// save/load. Checked again at commit, under the same game-owner lock.
    pub fn setup_busy(&self) -> bool {
        self.temporary.is_some() || self.file.player.iter().chain(self.file.cpu.iter())
            .any(|r| r.marker != Marker::NoAction)
    }

    /// Commit an already validated JSON-free setup to the frozen slot. A restore
    /// uses the normal death/rebirth machine, with this explicit slot rather
    /// than drawing from Randomize Slots or changing the selected setting.
    pub fn bind_setup(&mut self, slot: usize, mut pair: [SavedState; 2], restore: bool,
        settings: &StateSettings, rng: &mut Rng) -> bool
    {
        if slot >= SLOTS || self.setup_busy() || pair.iter().any(|r| r.mii.is_some()) {
            return false;
        }
        for record in &mut pair { record.marker = Marker::NoAction; }
        let [player, cpu] = pair;
        self.file.player[slot] = player;
        self.file.cpu[slot] = cpu;
        if restore { self.start_load(slot, settings, rng); }
        true
    }

    /// Store a Save's captured values (the marker stays `NoAction`). Returns whether the whole file is
    /// to be written now: neither record of the slot is still `Save` (§5.5 step 2).
    pub fn capture(&mut self, role: Role, slot: usize, mut state: SavedState) -> bool {
        let slot = slot.min(SLOTS - 1);
        state.marker = Marker::NoAction;
        self.records_mut(role)[slot] = state;
        [Role::Player, Role::Cpu]
            .iter()
            .all(|r| self.marker(*r, slot) != Marker::Save)
    }

    /// Run one fighter body's frame.
    pub fn frame(&mut self, input: &FrameIn, settings: &StateSettings, rng: &mut Rng) -> Out {
        let mut out = Out::default();
        let Some(role) = Role::of_entry(input.entry) else {
            return out;
        };
        if !settings.enable && !self.temporary_active() {
            return out;
        }
        let slot = self.current_slot(settings);
        let marker = self.load_marker(role, slot);

        // 5.1 Load request.
        let auto = settings.autoload && marker == Marker::NoAction && input.dead;
        if (auto || input.load_chord) && !input.nana && !self.temporary_active() {
            out.stop_playback = self.begin_load(settings, rng);
            return out;
        }

        let slot = self.current_slot(settings);
        let marker = self.load_marker(role, slot);
        match marker {
            Marker::KillPlayer if !input.nana => {
                out.pokemon_predeath = input.pokemon;
                out.force_dead = !input.dead;
                self.set_load_marker(role, slot, Marker::WaitForAlive);
            }
            Marker::WaitForAlive => {
                out.pokemon_predeath = input.pokemon;
                if !input.dead && !input.nana {
                    out.cleanup = true;
                    self.set_load_marker(role, slot, Marker::PosMove);
                    self.place(role, slot, input, settings, rng, &mut out);
                }
            }
            Marker::PosMove if !input.nana => {
                self.place(role, slot, input, settings, rng, &mut out)
            }
            Marker::NanaPosMove if input.nana && !input.standby => {
                self.place(role, slot, input, settings, rng, &mut out)
            }
            // The waits of the features: the runtime steps them and calls `finish_wait` when done.
            Marker::ApplyBuff | Marker::WaitForPokemonSwitch | Marker::WaitForCopyAbility
                if !input.nana =>
            {
                out.wait = Some(marker)
            }
            _ => {}
        }

        // 5.5 Save, at the end of every callback.
        if input.save_chord && !self.temporary_active() {
            self.mirror_sign = 1.0;
            let save_slot = usize::from(settings.slot).min(SLOTS - 1);
            for r in [Role::Player, Role::Cpu] {
                self.set_marker(r, save_slot, Marker::Save);
            }
            out.saved_notice = Some(save_slot);
        }
        let save_slot = usize::from(settings.slot).min(SLOTS - 1);
        if self.marker(role, save_slot) == Marker::Save && !input.nana && !self.temporary_active() {
            self.set_marker(role, save_slot, Marker::NoAction);
            out.capture = Some(save_slot);
        }

        // 10. The recorder is told once when both records are done.
        if self.restore_pending && !self.is_loading() {
            self.restore_pending = false;
            if let Some(slot) = self.active.take() {
                // The restore is complete: the recorder is told once (pkg4 interface), with whether
                // the CPU was saved on the ledge. Package 4 then records or plays per its own settings.
                out.restored =
                    Some(self.load_record(Role::Cpu, slot).situation_kind == input.situation.cliff);
            }
            self.temporary = None;
        }
        if !self.is_loading() {
            self.active = None;
        }
        out
    }

    /// Steps 2 to 9 of §5.3 for one body.
    fn place(
        &mut self,
        role: Role,
        slot: usize,
        input: &FrameIn,
        settings: &StateSettings,
        rng: &mut Rng,
        out: &mut Out,
    ) {
        // Copy the values out (no clone: a record may hold a heap-backed Mii identity and this runs in a
        // frame hook).
        let (
            saved_x,
            saved_y,
            saved_lr,
            saved_situation,
            saved_percent,
            saved_kind,
            saved_charge,
            saved_steve,
            saved_item,
        ) = {
            let record = self.load_record(role, slot);
            (
                record.x,
                record.y,
                record.lr,
                record.situation_kind,
                record.percent,
                record.fighter_kind,
                record.charge,
                record.steve,
                record.held_item,
            )
        };
        let situation = input.situation.classify(saved_situation);
        let mut y = saved_y;
        if situation == Situation::Cliff {
            y += place::ledge_raise(input.kind);
        }
        let mirror = if self.temporary_active() {
            1.0
        } else {
            self.mirror_sign
        };
        let x = place::target_x(saved_x, mirror, input.stage_id);
        let lr = place::target_lr(saved_lr, mirror);
        let request = match situation {
            Situation::Ground if input.status != StatusNow::Wait => Some(Request::Wait),
            Situation::Air if input.status != StatusNow::Fall => Some(Request::Fall),
            Situation::Cliff
                if !matches!(input.status, StatusNow::Fall | StatusNow::CliffCatch) =>
            {
                Some(Request::Fall)
            }
            _ => None,
        };
        out.placing = Some(Placing {
            x,
            y,
            lr,
            grounded: situation == Situation::Ground,
            request,
        });
        out.run_hub = true;
        if request.is_some() {
            return;
        }

        // Placed: 8a-8g.
        let (mut mode, limits) = match role {
            Role::Player => (settings.damage_player, settings.limits_player),
            Role::Cpu => (settings.damage_cpu, settings.limits_cpu),
        };
        if self.temporary_active() {
            mode = super::settings::DamageMode::SaveState;
        }
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
            item: (role == Role::Player && !input.nana)
                .then_some(saved_item)
                .flatten(),
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
        self.set_load_marker(role, slot, next);
    }
}
