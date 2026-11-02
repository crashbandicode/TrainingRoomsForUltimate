//! The whole CPU-behaviour state and the events that drive it: one `Cpu` value behind one lock in the
//! runtime. It holds the published settings, the plugin's own generator, the constant table, the role
//! table (which module accessor is which CPU body), and one state per fighter body (decision B6).
//! Every entry point is pure: the runtime reads the game first (no game call under the lock), calls
//! one of these, releases the lock and applies the answer.

use super::counter::{CounterId, Counters};
use super::defence::{Defence, SdiAnswer};
use super::facts::{FrameFacts, FrameOut};
use super::ids::Table;
use super::ledge::Ledge;
use super::mash::{BufferContext, Mash};
use super::playback::{covered_setting_applies, PlaybackState};
use super::rng::Rng;
use super::role::{FighterKey, RoleTable, Scene};
use super::settings::{situation, trigger, CpuSettings};
use super::advantage::FrameAdvantage;
use super::command::{self, CmdFacts, CmdOut};
use super::shield::{self, ButtonAnswer, HoldFacts, OosFacts, OosOut, Shield};
use super::status::{self, Situation};
use super::throw::{self, Throw};
use super::tech::{self, Hidden, MotionHashes, Tech, TechChange, TechRequest};
use crate::model::lra_reset::HubStep;
use crate::model::menu::reroll;
use crate::model::settings::Snapshot;

/// What the last frame saw of a body, for the hooks that answer between frames (the air-dodge stick
/// reads "airborne and in hitstun" without calling the game).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seen {
    pub status: i32,
    pub situation: Situation,
    pub prev_situation: Situation,
}

impl Seen {
    pub const fn new() -> Self {
        Self { status: super::ids::UNRESOLVED, situation: Situation::Other, prev_situation: Situation::Other }
    }
}

impl Default for Seen {
    fn default() -> Self {
        Self::new()
    }
}

/// One fighter body's state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fighter {
    pub mash: Mash,
    pub counters: Counters,
    pub defence: Defence,
    pub ledge: Ledge,
    pub tech: Tech,
    pub shield: Shield,
    pub throw: Throw,
    pub seen: Seen,
    /// The fighter kind the frame wrapper last saw on this body (the character-specific completion table).
    pub kind: i32,
    /// `reroll::*` bits a menu change left for this body's next CPU frame (decision Q18 A). A fresh
    /// body starts with the SDI roll pending, so its first frame rolls the direction.
    pub pending: u8,
}

impl Fighter {
    pub const fn new() -> Self {
        Self {
            mash: Mash::new(),
            counters: Counters::new(),
            defence: Defence::new(),
            ledge: Ledge::new(),
            tech: Tech::new(),
            shield: Shield::new(),
            throw: Throw::new(),
            seen: Seen::new(),
            kind: super::ids::UNRESOLVED,
            pending: reroll::SDI_DIRECTION | reroll::SHIELD_TILT,
        }
    }
}

impl Default for Fighter {
    fn default() -> Self {
        Self::new()
    }
}

/// The CPU behaviours' state.
#[derive(Clone, Debug, PartialEq)]
pub struct Cpu {
    pub ids: Table,
    pub settings: CpuSettings,
    pub rng: Rng,
    pub scene: Scene,
    pub roles: RoleTable,
    pub playback: PlaybackState,
    /// A waiting scene with practice not enabled (the fork's lobby filter).
    pub lobby_without_practice: bool,
    pub fighters: [Fighter; FighterKey::COUNT],
    /// The animation hashes the motion hook substitutes.
    pub motion: MotionHashes,
    /// The primary body's hidden-tech record (SPEC-cpu-04 §5).
    pub hidden: Option<Hidden>,
    /// A save state is loading (package 3 sets it): the shield is never held then.
    pub save_loading: bool,
    /// The game's shield damage multiplier as first observed (SPEC-cpu-05 §4); it survives a session
    /// clear so leaving Training can restore it.
    pub shield_damage_cache: Option<f32>,
    /// The frame-advantage measurement (SPEC-cpu-07).
    pub advantage: FrameAdvantage,
    /// The fixed-camera centre the game set at the last setup (SPEC-cpu-04 section 5 "Camera").
    pub stage_default: Option<[f32; 3]>,
}

impl Cpu {
    pub fn new(settings: CpuSettings, seed: u64) -> Self {
        Self {
            ids: Table::unresolved(),
            settings,
            rng: Rng::new(seed),
            scene: Scene::Training,
            roles: RoleTable::new(),
            playback: PlaybackState::default(),
            lobby_without_practice: false,
            fighters: [Fighter::new(); FighterKey::COUNT],
            motion: MotionHashes::new(),
            hidden: None,
            save_loading: false,
            shield_damage_cache: None,
            advantage: FrameAdvantage::new(),
            stage_default: None,
        }
    }

    /// The state at factory settings.
    pub fn factory(seed: u64) -> Self {
        Self::new(CpuSettings::from_snapshot(&Snapshot::factory()), seed)
    }

    fn context(&self) -> BufferContext {
        BufferContext { lobby_without_practice: self.lobby_without_practice, playback: self.playback }
    }

    /// Whether a setting covered by decisions B5 / N4 may act now.
    pub fn covered_applies(&self) -> bool {
        covered_setting_applies(self.playback, self.settings.override_playback)
    }

    // ---- the menu ------------------------------------------------------------------------------

    /// A menu commit (or the first publish): the new settings replace the old ones as a whole;
    /// `rerolls` is the menu outbox's bits (decisions Q18 A, B11). SDI Direction and Shield Tilt are
    /// left pending for each body's next CPU frame; a changed Ledge Options row clears every body's
    /// held ledge case at once, so the CPU rolls again from the new set during the same hang.
    pub fn publish(&mut self, settings: CpuSettings, rerolls: u8) {
        // A menu save that leaves both Mash Toggles and Mash Triggers empty, with at least one of them
        // changed, empties every queue once, so a disabled mash does not keep running (SPEC-cpu-01 §5).
        let disabled = settings.mash_state.is_empty()
            && settings.mash_triggers == 0
            && (self.settings.mash_state != settings.mash_state || self.settings.mash_triggers != settings.mash_triggers);
        self.settings = settings;
        if disabled {
            for fighter in &mut self.fighters {
                fighter.mash.full_reset();
                fighter.mash.latch = None;
            }
        }
        for fighter in &mut self.fighters {
            fighter.pending |= rerolls & (reroll::SDI_DIRECTION | reroll::SHIELD_TILT);
            if rerolls & reroll::LEDGE_OPTIONS != 0 {
                fighter.ledge.clear_case();
            }
            if rerolls & reroll::THROW_OPTIONS != 0 {
                fighter.throw.clear_case();
            }
        }
    }

    // ---- the reset bundle and the clock --------------------------------------------------------

    /// One step of the Training reset hub (SPEC-state-06 §5.2), for every body.
    pub fn hub_step(&mut self, step: HubStep) {
        if step == HubStep::Combo {
            self.advantage.reset();
        }
        let Self { fighters, settings, rng, .. } = self;
        for fighter in fighters.iter_mut() {
            match step {
                HubStep::Mash => {
                    fighter.mash.full_reset();
                    fighter.mash.latch = None;
                    fighter.counters.get(CounterId::Aerial).full_reset();
                }
                HubStep::Sdi => {
                    fighter.defence.sdi_roll(settings, rng);
                    fighter.pending &= !reroll::SDI_DIRECTION;
                }
                HubStep::FrameCounters => fighter.counters.reset_all(),
                HubStep::Ledge => fighter.ledge.reset_delay(),
                // Combo belongs to SPEC-cpu-07; InputDelay is the input-delay feature's.
                HubStep::ShieldTilt => {
                    fighter.shield.reread_tilt(settings);
                    fighter.pending &= !reroll::SHIELD_TILT;
                }
                HubStep::Throw => fighter.throw.reset_delay(),
                // Frame advantage resets once, after the loop (it is not per body).
                HubStep::Combo | HubStep::InputDelay => {}
            }
        }
    }

    /// The session ended (scene departure, swap): every body's state and the role table go.
    pub fn clear_session(&mut self) {
        self.fighters = [Fighter::new(); FighterKey::COUNT];
        self.roles.clear();
        // A replacement body must not inherit a stale "hidden" record.
        self.hidden = None;
        self.advantage.reset();
    }

    /// One simulation frame passed: every counter advances once. The runtime calls this from the
    /// primary CPU body's frame only (SPEC-cpu-07 §5), so the clock ticks once per frame whatever the
    /// number of bodies.
    pub fn tick_clock(&mut self) {
        for fighter in &mut self.fighters {
            fighter.counters.tick();
        }
        self.advantage.elapsed.tick();
    }

    // ---- one body's frame ----------------------------------------------------------------------

    /// The once-per-frame dispatch for the CPU-role body `key` (SPEC-cpu-01 §5 "Ordering": dodge
    /// staling, hide tech (primary body), mistech, clatter, ledge, then the DI case reset).
    pub fn frame(&mut self, key: FighterKey, f: &FrameFacts) -> FrameOut {
        let ctx = self.context();
        let covered = self.covered_applies();
        let self_playback_running = self.playback.running;
        let primary = !key.partner && key.entry == self.primary_entry();
        let Self { ids, settings, rng, fighters, hidden, .. } = self;
        let fighter = &mut fighters[key.index()];
        let mut out = FrameOut::default();
        fighter.seen = Seen { status: f.status, situation: f.situation, prev_situation: f.prev_situation };
        if fighter.pending & reroll::SDI_DIRECTION != 0 {
            fighter.defence.sdi_roll(settings, rng);
            fighter.pending &= !reroll::SDI_DIRECTION;
        }
        out.stale_flag = fighter.defence.stale_write(settings);
        if primary {
            let (hide, record) = tech::hide_frame(
                ids,
                settings.tech_hide,
                key.entry,
                f.status,
                f.motion_frame,
                f.tech.end_frame,
                f.tech.cursor_offset,
                *hidden,
            );
            out.hide = hide;
            *hidden = record;
        }
        // Mistech: the lay-down counter waits out Snake's lock-out and resets in any other status.
        if covered {
            let waiting = if tech::Tech::in_wait_status(ids, f.status) {
                let lockout = tech::laydown_lockout(f.tech.laydown_base, f.tech.laydown_max, f.tech.laydown_damage_max, f.tech.damage);
                fighter.counters.get(CounterId::LayDown).should_delay(lockout)
            } else {
                fighter.counters.get(CounterId::LayDown).full_reset();
                false
            };
            if let Some(forced) = fighter.tech.mistech(ids, settings, rng, f.status, waiting) {
                out.force_status = Some(forced);
                if settings.trigger_on(trigger::MISTECH_OPTION) {
                    let row = Some(settings.overrides[situation::TECH_ACTION]).filter(|r| !r.is_empty()).unwrap_or(settings.mash_state);
                    fighter.mash.external_buffer(settings, rng, ctx, &row);
                }
            }
        }
        if status::in_clatter(ids, f.status) {
            out.clatter = fighter.defence.clatter_frame(settings);
        }
        let ledge = fighter.ledge.frame(ids, settings, rng, &mut fighter.counters, &mut fighter.mash, ctx, f);
        out.request_status = ledge.request_status;
        out.playback = ledge.playback;
        if fighter.pending & reroll::SHIELD_TILT != 0 {
            fighter.shield.reread_tilt(settings);
            fighter.pending &= !reroll::SHIELD_TILT;
        }
        fighter.shield.frame(
            settings,
            rng,
            status::idle(ids, f.status) || status::in_hitstun(ids, f.status),
            status::in_shieldstun(ids, f.status, f.prev_status),
            ids.is(super::ids::C::StatusGuard, f.status),
        );
        out.fast_fall = throw::fast_fall_frame(
            ids,
            settings,
            rng,
            &mut fighter.counters,
            &mut fighter.mash,
            f.situation == Situation::Air,
            f.falling,
            f.status,
            f.reserve_dive,
            self_playback_running,
        );
        fighter.defence.di_frame(status::in_hitstun(ids, f.status));
        out
    }

    /// The entry whose first body is the primary CPU body for this scene.
    pub fn primary_entry(&self) -> u8 {
        match self.scene {
            Scene::Training => 1,
            Scene::Lobby => super::role::LOBBY_CPU_ENTRY as u8,
        }
    }

    // ---- the hooks' questions ------------------------------------------------------------------

    /// The game corrects a damage vector for the body `key` (the `common` script hook).
    pub fn di_correction(&mut self, key: FighterKey, player_x: f32, cpu_x: f32) -> Option<(f32, f32)> {
        if !self.covered_applies() {
            return None;
        }
        let Self { settings, rng, fighters, .. } = self;
        fighters[key.index()].defence.di_correction(settings, rng, player_x, cpu_x)
    }

    /// The SDI hook (`check_hit_stop_delay_command`) for the body `key`.
    pub fn sdi_query(&mut self, key: FighterKey, player_x: f32, cpu_x: f32) -> SdiAnswer {
        if !self.covered_applies() {
            return SdiAnswer::Native;
        }
        let Self { settings, fighters, .. } = self;
        fighters[key.index()].defence.sdi_query(settings, player_x, cpu_x)
    }

    /// The air-dodge override on a stick read for the body `key`. "Airborne and in hitstun" is what
    /// the body's last frame saw.
    pub fn air_dodge_stick(&mut self, key: FighterKey, player_x: f32, cpu_x: f32) -> Option<(f32, f32)> {
        if !self.covered_applies() {
            return None;
        }
        let Self { ids, settings, rng, fighters, .. } = self;
        let fighter = &mut fighters[key.index()];
        let airborne_in_hitstun = fighter.seen.situation == Situation::Air && status::in_hitstun(ids, fighter.seen.status);
        fighter.defence.air_dodge_stick(settings, rng, airborne_in_hitstun, player_x, cpu_x)
    }

    /// `ControlModule::start_clatter` was called for the body `key`.
    pub fn clatter_start(&mut self, key: FighterKey, manual_recovery_rate: f32) {
        self.fighters[key.index()].defence.clatter_start(manual_recovery_rate);
    }

    /// The transition-term hook for the body `key`: the answer the hook returns.
    pub fn ledge_transition_term(&self, key: FighterKey, status: i32, term: i32, original: bool) -> bool {
        let fighter = &self.fighters[key.index()];
        fighter.ledge.transition_term(&self.ids, &self.settings, &fighter.counters, self.playback.running, status, term, original)
    }
}

impl Cpu {
    /// The script status-change hook for the body `key` (SPEC-cpu-04 §5 "Tech"). Returns the change and
    /// buffers the Tech Option mash when a tech happened and its trigger is ticked.
    pub fn tech_status_change(&mut self, key: FighterKey, r: &TechRequest) -> TechChange {
        if !self.covered_applies() {
            return TechChange::Unchanged;
        }
        let ctx = self.context();
        let Self { ids, settings, rng, fighters, .. } = self;
        let fighter = &mut fighters[key.index()];
        let (change, teched) = fighter.tech.status_change(ids, settings, rng, r);
        if teched && settings.trigger_on(trigger::TECH_OPTION) {
            let row = Some(settings.overrides[situation::TECH_ACTION]).filter(|r| !r.is_empty()).unwrap_or(settings.mash_state);
            fighter.mash.external_buffer(settings, rng, ctx, &row);
        }
        change
    }

    /// The motion hook: the animation to play instead of `hash` for the body `key`.
    pub fn substitute_motion(&self, key: FighterKey, hash: u64) -> u64 {
        if !self.covered_applies() {
            return hash;
        }
        self.fighters[key.index()].tech.substitute_motion(&self.motion, &self.settings, hash)
    }

    /// The quake hook: whether to request no quake for a CPU body in `status`.
    pub fn quake_suppressed(&self, status: i32) -> bool {
        tech::quake_suppressed(&self.ids, &self.settings, status)
    }
}

impl Cpu {
    /// Whether the body `key` should hold its shield now (SPEC-cpu-05 §5).
    fn hold_for(&self, key: FighterKey) -> bool {
        let fighter = &self.fighters[key.index()];
        let grounded = fighter.seen.situation == Situation::Ground;
        let facts = HoldFacts {
            playback_running: self.playback.running,
            standby: self.playback.standby,
            save_state_loading: self.save_loading,
            was_airborne: fighter.seen.prev_situation == Situation::Air,
            queue_requests_shield: shield::queue_requests_shield(fighter.mash.queue.current(), grounded),
        };
        shield::should_hold(self.settings.shield_state, &facts)
    }

    /// The guard-button hook for the body `key`; `on_query` is true for `check_button_on` and false
    /// for `check_button_off`.
    pub fn guard_button(&self, key: FighterKey, on_query: bool) -> ButtonAnswer {
        let suspended = self.fighters[key.index()].shield.suspended;
        shield::guard_button(on_query, self.hold_for(key), suspended)
    }

    /// The parameter hook: the value to answer for `shield_dec1` / `shield_recovery1`, or `None` for
    /// the original. `decay` is true for `shield_dec1`, false for `shield_recovery1`.
    pub fn shield_param(&self, key: FighterKey, decay: bool) -> Option<f32> {
        if self.playback.running {
            return None;
        }
        let shield = &self.fighters[key.index()].shield;
        shield::decay_override_active(self.settings.shield_state, shield.decay_enabled)
            .then_some(if decay { shield::SHIELD_DECAY_OVERRIDE } else { shield::SHIELD_RECOVERY_OVERRIDE })
    }

    /// The tilted stick for an unclamped stick read of the body `key`.
    pub fn shield_tilt_stick(&self, key: FighterKey, player_x: f32, cpu_x: f32) -> Option<(f32, f32)> {
        self.fighters[key.index()].shield.tilt_stick(player_x, cpu_x)
    }

    /// The script continuation after the original, for the body `key` (the `common` hook).
    pub fn out_of_shield(&mut self, key: FighterKey, facts: &OosFacts) -> OosOut {
        let ctx = self.context();
        let Self { ids, settings, rng, fighters, .. } = self;
        let fighter = &mut fighters[key.index()];
        fighter.shield.out_of_shield(ids, settings, rng, &mut fighter.counters, &mut fighter.mash, ctx, facts)
    }

    /// The multiplier to write this frame (SPEC-cpu-05 §4): the first call caches what the game has.
    /// The game is setting the Training fixed-camera values (game_centre is the centre it passes): remember it as the
    /// stage default and answer the centre to use instead, if any (SPEC-cpu-04 section 5 "Camera").
    pub fn camera_setup(&mut self, class: tech::StageClass, game_centre: [f32; 3]) -> Option<[f32; 3]> {
        self.stage_default = Some(game_centre);
        tech::fixed_camera_centre(&self.settings, class, self.stage_default).filter(|centre| *centre != game_centre)
    }

    /// The camera mode changes: the centre to write into the camera manager first, if any (only the fixed mode).
    pub fn camera_mode(&self, class: tech::StageClass, mode: i32) -> Option<[f32; 3]> {
        if mode != tech::CAMERA_MODE_FIXED {
            return None;
        }
        tech::fixed_camera_centre(&self.settings, class, self.stage_default)
    }

    pub fn damage_multiplier(&mut self, in_training: bool, observed: f32) -> Option<f32> {
        let cached = *self.shield_damage_cache.get_or_insert(observed);
        shield::damage_multiplier(self.settings.shield_state, in_training, Some(cached))
    }
}

impl Cpu {
    /// Whether a command query of the body `key` has anything to do (the runtime skips the game reads
    /// otherwise): a mash is set or queued, or a throw or pummel is set.
    pub fn wants_commands(&self, key: FighterKey, cat2: bool) -> bool {
        let s = &self.settings;
        let fighter = &self.fighters[key.index()];
        if cat2 {
            return !s.throw_state.is_empty() || s.pummel_delay.is_set() || fighter.throw.case.is_some();
        }
        !fighter.mash.queue.is_empty()
            || !s.mash_state.is_empty()
            || s.overrides.iter().any(|o| !o.is_empty())
            || fighter.mash.queued_aerial.is_some()
            || fighter.mash.rolled.attack_angle.is_some()
    }

    /// A command-flag query of the body `key` (`get_command_flag_cat`): the bits to OR into the game's
    /// answer. Category 1 runs the mash (choose, then perform); category 2 delivers the throw bits.
    pub fn command_flags(&mut self, key: FighterKey, cat2: bool, f: &CmdFacts) -> CmdOut {
        let ctx = self.context();
        let applies = self.covered_applies();
        let save_loading = self.save_loading;
        let Self { ids, settings, rng, fighters, .. } = self;
        let fighter = &mut fighters[key.index()];
        // The hooks that answer between frames read what the latest query saw.
        fighter.seen.status = f.status;
        fighter.seen.situation = f.situation;
        if cat2 {
            if !applies {
                return CmdOut::default();
            }
            return fighter.throw.step(ids, settings, rng, &mut fighter.counters, &mut fighter.mash, ctx, true, f);
        }
        let mut out = fighter.mash.check_buffer(ids, settings, rng, ctx, f, save_loading);
        let done = fighter.mash.perform(ids, settings, &mut fighter.counters, f);
        out.bits = done.bits;
        out.request_status = done.request_status;
        out
    }

    /// `get_attack_air_kind`: the aerial command kind to answer instead of the game's, if any.
    pub fn attack_air_kind(&self, key: FighterKey) -> Option<i32> {
        if self.playback.running {
            return None;
        }
        let action = self.fighters[key.index()].mash.queued_aerial?;
        let v = self.ids.get(command::aerial_kind(action)?);
        (v != super::ids::UNRESOLVED).then_some(v)
    }

    /// `get_stick_dir`: the angled-attack value for the body `key`.
    pub fn attack_angle(&self, key: FighterKey) -> Option<f32> {
        let fighter = &self.fighters[key.index()];
        throw::attack_angle_value(&fighter.mash, fighter.seen.situation == Situation::Cliff, self.playback.running)
    }

    /// `get_stick_y`: crouch.
    pub fn crouch_stick(&self, key: FighterKey) -> Option<f32> {
        let fighter = &self.fighters[key.index()];
        throw::crouch_applies(&self.ids, &self.settings, fighter.seen.status).then_some(-1.0)
    }

    /// The jump buttons for the full hop.
    pub fn full_hop_button(&self, key: FighterKey, on_query: bool) -> Option<bool> {
        let fighter = &self.fighters[key.index()];
        throw::full_hop_button(&self.ids, &fighter.mash, fighter.seen.status, on_query, self.playback.running)
    }
}

impl Cpu {
    /// Frame advantage on the player entry's frame: whether each fighter is actionable now; the
    /// advantage to show, if one was measured (SPEC-cpu-07 §5).
    pub fn frame_advantage(&mut self, player_actionable: bool, cpu_actionable: bool) -> Option<i32> {
        let (on, empty) = (self.settings.frame_advantage, self.settings.mash_state.is_empty());
        self.advantage.step(on, empty, player_actionable, cpu_actionable)
    }
}
