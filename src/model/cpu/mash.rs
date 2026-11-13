//! The mash queue and its external buffer entry (SPEC-cpu-01 §5 "Queue" and "Buffer a menu mash").
//!
//! One queue of at most two entries per fighter body (decision B6): the main action and, behind it,
//! the follow-up. The ledge, tech, shield and throw specs buffer an action through
//! `Mash::buffer_menu_mash`; the per-frame trigger evaluation and the command flags of each action
//! are the mash spec's own part (a later milestone). The rolls the spec ties to buffering (attack
//! angle, aerial delay, full hop, fast fall, falling aerial) are kept here so every entry point rolls
//! them the same way.

use super::counter::Counters;
use super::ids::Table;
use super::playback::PlaybackState;
use super::rng::Rng;
use super::settings::{CpuSettings, Weights};

/// One of the 30 actions, as the index of the `Action` option set (`menu::options::ACTIONS`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Action(pub u8);

impl Action {
    pub const AIR_DODGE: Action = Action(0);
    pub const JUMP: Action = Action(1);
    pub const SHIELD: Action = Action(2);
    pub const SPOT_DODGE: Action = Action(3);
    pub const ROLL_F: Action = Action(4);
    pub const ROLL_B: Action = Action(5);
    pub const NAIR: Action = Action(6);
    pub const FAIR: Action = Action(7);
    pub const BAIR: Action = Action(8);
    pub const UAIR: Action = Action(9);
    pub const DAIR: Action = Action(10);
    pub const NEUTRAL_B: Action = Action(11);
    pub const SIDE_B: Action = Action(12);
    pub const UP_B: Action = Action(13);
    pub const DOWN_B: Action = Action(14);
    pub const F_SMASH: Action = Action(15);
    pub const U_SMASH: Action = Action(16);
    pub const D_SMASH: Action = Action(17);
    pub const JAB: Action = Action(18);
    pub const F_TILT: Action = Action(19);
    pub const U_TILT: Action = Action(20);
    pub const D_TILT: Action = Action(21);
    pub const GRAB: Action = Action(22);
    pub const DASH: Action = Action(23);
    pub const DASH_ATTACK: Action = Action(24);
    pub const PLAYBACK_FIRST: u8 = 25;
    pub const COUNT: usize = 30;

    pub const fn is_aerial(self) -> bool {
        self.0 >= 6 && self.0 <= 10
    }

    pub const fn is_playback(self) -> bool {
        self.0 >= Self::PLAYBACK_FIRST && (self.0 as usize) < Self::COUNT
    }

    /// The playback slot (0..=4) of a playback action.
    pub const fn playback_slot(self) -> Option<u8> {
        if self.is_playback() {
            Some(self.0 - Self::PLAYBACK_FIRST)
        } else {
            None
        }
    }
}

/// The main action and the follow-up behind it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Queue {
    front: Option<Action>,
    behind: Option<Action>,
}

impl Queue {
    pub const fn new() -> Self {
        Self { front: None, behind: None }
    }

    pub const fn is_empty(&self) -> bool {
        self.front.is_none()
    }

    /// The action being performed (the main action until it completes, then the follow-up).
    pub const fn current(&self) -> Option<Action> {
        self.front
    }

    /// Put an action at the back of the queue; refused when it holds two.
    pub fn push(&mut self, action: Action) -> bool {
        if self.front.is_none() {
            self.front = Some(action);
            true
        } else if self.behind.is_none() {
            self.behind = Some(action);
            true
        } else {
            false
        }
    }

    /// The current action completed: pop it; the follow-up becomes current.
    pub fn complete(&mut self) -> Option<Action> {
        let done = self.front.take();
        self.front = self.behind.take();
        done
    }

    /// Empty the queue.
    pub fn clear(&mut self) {
        *self = Self::new();
    }
}

/// The values rolled when an action is buffered, held until the next roll (SPEC-cpu-06 §5 "Rolls").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rolled {
    /// Held Attack Angle: `None` (neutral or an empty row), or the angle option index (1 up, 2 down).
    pub attack_angle: Option<u8>,
    /// The status the angled action completed in: the angle holds while the body stays in it (the attack's status
    /// reads the angle after the command query that completed the action).
    pub angle_status: Option<i32>,
    pub aerial_delay: u16,
    pub full_hop: bool,
    pub fast_fall: bool,
    pub falling_aerial: bool,
    pub fast_fall_delay: u16,
}

/// What `buffer_menu_mash` did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Buffered {
    /// The queue was not empty or there was nothing to buffer.
    Refused,
    /// The action (and a follow-up, when one was picked) is in the queue.
    Queued { follow_up: bool },
    /// Playback Mash Interrupt: the running playback must be stopped and nothing was buffered; the
    /// mash triggers again on a later frame.
    StopPlayback,
}

/// Where the buffer request comes from, for the playback rule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BufferContext {
    pub playback: PlaybackState,
}

/// What one body's rule step works with (the ledge, out-of-shield and throw steps, the fast fall): the game's ids,
/// the settings, the plugin's generator, this body's counters and mash, and the buffer context. `state::Cpu` builds
/// one per call from its own fields.
pub struct Step<'a> {
    pub t: &'a Table,
    pub s: &'a CpuSettings,
    pub rng: &'a mut Rng,
    pub counters: &'a mut Counters,
    pub mash: &'a mut Mash,
    pub ctx: BufferContext,
}

/// One fighter body's mash state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mash {
    pub queue: Queue,
    pub rolled: Rolled,
    /// The dash-transition latch (SPEC-cpu-01 §5 step 3).
    pub transitioning_dash: bool,
    /// The aerial the `get_attack_air_kind` hook turns the next attack press into.
    pub queued_aerial: Option<Action>,
    /// The trigger situation that last buffered a mash (decision B8: one roll per situation entry).
    pub latch: Option<super::command::Situ>,
}

impl Mash {
    pub const fn new() -> Self {
        Self {
            queue: Queue::new(),
            rolled: Rolled { attack_angle: None, angle_status: None, aerial_delay: 0, full_hop: false, fast_fall: false, falling_aerial: false, fast_fall_delay: 0 },
            transitioning_dash: false,
            queued_aerial: None,
            latch: None,
        }
    }

    /// Empty the queue, then clear the aerial delay (the spec's "full reset": the held aerial delay
    /// and its counter go with it; the counter is the caller's).
    pub fn full_reset(&mut self) {
        self.queue.clear();
        self.rolled.aerial_delay = 0;
        self.transitioning_dash = false;
        self.queued_aerial = None;
    }

    /// The current action completed: pop it and clear the aerial delay. Returns the action.
    pub fn complete(&mut self) -> Option<Action> {
        self.rolled.aerial_delay = 0;
        self.queue.complete()
    }

    fn roll_for_action(&mut self, s: &CpuSettings, rng: &mut Rng, action: Action) {
        self.rolled.attack_angle = s.attack_angle.pick(rng).filter(|i| *i != 0).map(|i| i as u8);
        self.rolled.angle_status = None;
        if action.is_aerial() {
            self.rolled.aerial_delay = s.aerial_delay.pick(rng);
        }
    }

    /// SPEC-cpu-01 §5 step 2, "Buffer a menu mash": `action` is the pick the caller made (`None` =
    /// an empty row). Applies the lobby filter's effect on the follow-up, refuses when the queue is
    /// not empty or there is nothing to buffer, honours Playback Mash Interrupt, rolls the angle and
    /// the aerial delay, pushes the action, then a follow-up, and finally rolls full hop, fast fall
    /// and falling aerial.
    pub fn buffer_menu_mash(&mut self, s: &CpuSettings, rng: &mut Rng, ctx: BufferContext, action: Option<Action>) -> Buffered {
        let Some(action) = action else { return Buffered::Refused };
        if !self.queue.is_empty() {
            return Buffered::Refused;
        }
        let pb = ctx.playback;
        if s.playback_mash && pb.running && !pb.recording && !pb.standby && !pb.queued && !action.is_playback() {
            return Buffered::StopPlayback;
        }
        self.roll_for_action(s, rng, action);
        self.queue.push(action);
        let follow_up = s.follow_up.pick(rng).map(|i| Action(i as u8));
        if let Some(next) = follow_up {
            if next.is_aerial() {
                self.rolled.aerial_delay = s.aerial_delay.pick(rng);
            }
            self.queue.push(next);
        }
        self.rolled.full_hop = s.full_hop.pick(rng) == Some(0);
        self.rolled.fast_fall = s.fast_fall.pick(rng) == Some(0);
        self.rolled.falling_aerial = s.falling_aerials.pick(rng) == Some(0);
        Buffered::Queued { follow_up: follow_up.is_some() }
    }

    /// The external entry point (ledge, tech, shield, throw): a full reset first, then the buffer,
    /// with `row` the weighted row the caller picks from (an override row, or `mash_state`).
    pub fn external_buffer(&mut self, s: &CpuSettings, rng: &mut Rng, ctx: BufferContext, row: &Weights<30>) -> Buffered {
        self.full_reset();
        let action = row.pick(rng).map(|i| Action(i as u8));
        self.buffer_menu_mash(s, rng, ctx, action)
    }
}
