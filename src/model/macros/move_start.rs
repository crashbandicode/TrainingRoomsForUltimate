//! Move-aligned input crops from existing captures. This is not a pose restore:
//! the selected move and its short input setup restart from neutral Training.
//! Engine status IDs/families are supplied by the adapter, never guessed here.
use {crate::engine::macros::ReplayTrack, crate::engine::macros::StartState};

#[derive(Clone, Debug)]
pub struct MoveFamily {
    pub fighter: Option<i32>,
    pub statuses: Vec<i32>,
    /// Required observed attack/special/grab input. Zero disables this check for
    /// adapters that cannot classify mapped controls, never for the game adapter.
    pub buttons: u32,
}

#[derive(Clone, Debug)]
pub struct MoveStartRules {
    pub families: Vec<MoveFamily>,
    /// Locomotion, jump and shield setup whose inputs may precede the attack.
    /// Damage, grabs by the opponent, respawn and ledge states are not setup.
    pub setup: Vec<i32>,
    pub ground: i32,
    pub air: i32,
    pub wait: i32,
    pub fall: i32,
    pub trigger_buttons: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoveStart {
    /// First input, including setup. This becomes the visible In handle.
    pub frame: u32,
    /// First observed frame of the selected move, not an animation seek target.
    pub action_frame: u32,
    pub action_status: i32,
    /// Explicit neutral restart context, persisted by the ordinary macro format.
    pub context: StartState,
}

/// Product bound on included preparation, not an engine input-buffer constant.
/// Never turn a small crop into minutes of unrelated match playback.
pub const MAX_MOVE_SETUP: u32 = 120;

impl MoveStartRules {
    fn family(&self, fighter: i32, status: i32) -> Option<usize> {
        self.families.iter().position(|family|
            family.fighter.is_none_or(|kind| kind == fighter) && family.statuses.contains(&status))
    }

    /// Pick the move containing In, the move immediately after its setup, or a
    /// nearby earlier move. Never hunt indefinitely backward through the match.
    /// Smash start/charge/release and multi-phase specials share a family. Walk
    /// back through only the immediately adjacent setup, not through earlier
    /// attacks or damage to a distant WAIT. One preceding input is retained:
    /// state was sampled before the fighter callback, controls after it.
    pub fn snap(&self, track: &ReplayTrack, frame: u32) -> Result<MoveStart, String> {
        let at = usize::try_from(frame).ok().filter(|&i| i < track.frames.len())
            .ok_or("No captured frame at In")?;
        let fighter = track.frames[at].start.fighter_kind;
        let mut action = at;
        if self.setup.contains(&track.frames[at].start.status) {
            while action < track.frames.len()
                && track.frames[action].start.fighter_kind == fighter
                && self.setup.contains(&track.frames[action].start.status)
                && action - at < MAX_MOVE_SETUP as usize { action += 1; }
            if action >= track.frames.len() || action - at >= MAX_MOVE_SETUP as usize
                || track.frames[action].start.fighter_kind != fighter
                || self.family(fighter, track.frames[action].start.status).is_none() {
                return Err("No move immediately after this setup; move In nearer an attack".into());
            }
        }
        let family = loop {
            let sample = &track.frames[action];
            if sample.start.fighter_kind != fighter {
                return Err("No move before In in this fighter form".into());
            }
            if let Some(family) = self.family(fighter, sample.start.status) { break family; }
            if action == 0 || at.saturating_sub(action) >= MAX_MOVE_SETUP as usize {
                return Err("No recorded move near In; move In nearer an attack".into());
            }
            action -= 1;
        };
        while action > 0 {
            let before = &track.frames[action - 1];
            if before.start.fighter_kind != fighter
                || self.family(fighter, before.start.status) != Some(family) { break; }
            // A new starter after the preceding move's recovery is a new move,
            // even when both share the same multi-phase family (e.g. specials).
            if track.frames[action].start.status == self.families[family].statuses[0]
                && before.start.status != track.frames[action].start.status { break; }
            action -= 1;
        }
        if action == 0 {
            return Err("Capture begins inside this move; choose a later move".into());
        }
        let mut setup = action;
        while setup > 0 {
            let before = &track.frames[setup - 1];
            if before.start.fighter_kind != fighter || !self.setup.contains(&before.start.status) { break; }
            setup -= 1;
            if action - setup >= MAX_MOVE_SETUP as usize {
                return Err("Move setup exceeds two seconds; choose a later move start".into());
            }
        }
        let mut input = setup.saturating_sub(1);
        let required = self.families[family].buttons;
        if required != 0 && !track.frames[input..=action].iter().any(|f| f.input.buttons & required != 0) {
            let prior_status = track.frames[input].start.status;
            loop {
                if input == 0 || action - (input - 1) > MAX_MOVE_SETUP as usize {
                    return Err("No triggering input near this move; choose another start".into());
                }
                let before = &track.frames[input - 1];
                if before.start.fighter_kind != fighter || before.start.status != prior_status {
                    return Err("No triggering input captured for this move; choose another start".into());
                }
                input -= 1;
                if before.input.buttons & required != 0 { break; }
            }
        }
        // Buffered direction can disappear before the observed transition.
        // Keep the initiating press within the preceding status, including its
        // earlier stick direction (e.g. down-smash buffered during a roll).
        let held = track.frames[input].input.buttons & self.trigger_buttons;
        let prior_status = track.frames[input].start.status;
        while input > 0 && held != 0 {
            let before = &track.frames[input - 1];
            if before.start.fighter_kind != fighter || before.start.status != prior_status
                || before.input.buttons & held == 0 { break; }
            if action - (input - 1) > MAX_MOVE_SETUP as usize {
                return Err("Buffered move setup exceeds two seconds; choose another start".into());
            }
            input -= 1;
        }
        let mut context = track.frames[input].start;
        if context.fighter_kind != fighter {
            return Err("Move setup crosses a fighter transformation".into());
        }
        // A preceding endlag sample is evidence of the triggering input, not a
        // claim that endlag/work variables are restored. Start from neutral and
        // replay the short setup through normal native simulation instead.
        if context.situation == self.ground {
            context.status = self.wait;
            context.speed_x = 0.0;
            context.speed_y = 0.0;
        } else if context.situation == self.air {
            context.status = self.fall;
        } else {
            return Err("Move starts on a ledge or outside normal play; choose another start".into());
        }
        Ok(MoveStart { frame: input as u32, action_frame: action as u32,
            action_status: track.frames[action].start.status, context })
    }
}

