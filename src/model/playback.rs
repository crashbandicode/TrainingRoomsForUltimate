//! One playback run per role. A Play chord starts that role's active macro. The start-save warp
//! happens on that transition and not on a later pass. Load and auto-load on death only name a
//! bound save; they do not start a run.

use std::collections::{BTreeMap, BTreeSet};

use super::library::{start_save_state, Chain, LoopCount, RecordingIndex, Role, StartSaveState};
use super::macros::{InputFrame, MacroRecording};
use crate::model::state::settings::StateSettings;

/// What a Play chord, a mash, or a load did to the runs. `notice` stays empty: this slice adds none.
/// The strings borrow the book, so a hook can return them without allocating.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Effect<'a> {
    pub warp: Option<&'a str>,
    pub started: bool,
    pub stopped: bool,
    pub notice: Option<&'static str>,
}

/// One frame of the role that is playing. `recording_id` borrows the book.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Played<'a> {
    pub recording_id: &'a str,
    pub frame_index: usize,
    pub frame: InputFrame,
}

/// A warp to the bound save. Mirroring, damage and buffs are the settings the load already applies.
/// `save_id` borrows the book.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeathWarp<'a> {
    pub save_id: &'a str,
    pub mirroring: crate::model::state::settings::Mirroring,
    pub damage_player: crate::model::state::settings::DamageMode,
    pub damage_cpu: crate::model::state::settings::DamageMode,
    pub limits_player: (u8, u8),
    pub limits_cpu: (u8, u8),
    pub buffs: crate::model::state::settings::Buffs,
}

/// The index plus the files the machine resolves. The machine does not read the card.
#[derive(Clone, Debug)]
pub struct Book {
    pub index: RecordingIndex,
    recordings: BTreeMap<String, MacroRecording>,
    chains: BTreeMap<String, Chain>,
    saves: BTreeSet<String>,
}

struct Plan<'a> {
    chain: bool,
    warp: Option<&'a str>,
}

/// The active id, copied into a fixed buffer so the next step does not allocate.
#[derive(Clone, Copy, Debug)]
struct IdBuf {
    bytes: [u8; crate::model::store::MAX_ID_CHARS],
    len: u8,
}

impl IdBuf {
    fn copy_from(id: &str) -> Self {
        let raw = id.as_bytes();
        let n = raw.len().min(crate::model::store::MAX_ID_CHARS);
        let mut bytes = [0u8; crate::model::store::MAX_ID_CHARS];
        bytes[..n].copy_from_slice(&raw[..n]);
        Self { bytes, len: n as u8 }
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or("")
    }
}

#[derive(Clone, Copy, Debug)]
struct Run {
    id: IdBuf,
    chain: bool,
    step: usize,
    frame: usize,
    /// Passes still to play, including the one in progress. `None` plays until the chord stops it.
    remaining: Option<u32>,
    emitted: bool,
}

/// Player and CPU, independently.
#[derive(Debug, Default)]
pub struct Playback {
    roles: [Option<Run>; 2],
    warps: [u32; 2],
}

impl Book {
    pub fn new(index: RecordingIndex) -> Self {
        Self {
            index,
            recordings: BTreeMap::new(),
            chains: BTreeMap::new(),
            saves: BTreeSet::new(),
        }
    }

    pub fn add_recording(&mut self, id: impl Into<String>, recording: MacroRecording) {
        self.recordings.insert(id.into(), recording);
    }

    pub fn add_chain(&mut self, id: impl Into<String>, chain: Chain) {
        self.chains.insert(id.into(), chain);
    }

    /// The file's own start save: linked, deleted, or none. A chain uses macro 1.
    pub fn start_link(&self, id: &str) -> StartLink<'_> {
        if let Some(chain) = self.chains.get(id) {
            if !chain.start_save_on {
                return StartLink::None;
            }
            let Some(first) = chain.recordings.first() else {
                return StartLink::None;
            };
            let Some(recording) = self.recordings.get(first.as_str()) else {
                return StartLink::None;
            };
            return link_of(recording, &self.saves);
        }
        let Some(recording) = self.recordings.get(id) else {
            return StartLink::None;
        };
        link_of(recording, &self.saves)
    }

    /// The fighter a play should compare. A chain uses macro 1 only.
    pub fn recorded_fighter(&self, id: &str) -> Option<i32> {
        if let Some(chain) = self.chains.get(id) {
            let first = chain.recordings.first()?;
            return self.recordings.get(first.as_str()).map(|recording| recording.start.fighter_kind);
        }
        self.recordings.get(id).map(|recording| recording.start.fighter_kind)
    }

    pub fn is_chain(&self, id: &str) -> bool {
        self.chains.contains_key(id)
    }

    /// A chain's macro 1 (whose start save the chain warps to); `None` for anything else.
    pub fn first_recording(&self, id: &str) -> Option<&str> {
        self.chains.get(id)?.recordings.first().map(String::as_str)
    }

    /// The exact Mii a play of `id` was recorded on (a chain: macro 1's), for Switch; `None` for any other fighter.
    pub fn exact_mii_target(&self, id: &str) -> Option<crate::engine::mii::replay::Target> {
        let first = self.first_recording(id).unwrap_or(id);
        let target = self.recordings.get(first)?.playback_target().ok()?;
        target.configuration().map(|_| target)
    }

    /// The file's placement (Edit placement); the default for a missing file.
    pub fn placement(&self, id: &str) -> super::macros::Placement {
        self.recordings.get(id).map(|recording| recording.placement).unwrap_or_default()
    }

    /// The recording's start state (its recorded start speeds and facing); `None` for anything else.
    pub fn start_state(&self, id: &str) -> Option<super::macros::StartState> {
        self.recordings.get(id).map(|recording| recording.start)
    }

    pub fn add_save(&mut self, id: impl Into<String>) {
        self.saves.insert(id.into());
    }

    fn plan(&self, id: &str) -> Option<Plan<'_>> {
        if let Some(chain) = self.chains.get(id) {
            let playable = chain
                .recordings
                .iter()
                .any(|step| self.recordings.contains_key(step.as_str()));
            if !playable {
                return None;
            }
            return Some(Plan {
                chain: true,
                warp: chain_warp(chain, self),
            });
        }
        let recording = self.recordings.get(id)?;
        Some(Plan {
            chain: false,
            warp: recording_warp(recording, &self.saves),
        })
    }
}

impl Playback {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn playing(&self, role: Role) -> bool {
        self.roles[role_slot(role)].is_some()
    }

    /// How many times this role has warped to a start save. A loop does not add one.
    pub fn warps(&self, role: Role) -> u32 {
        self.warps[role_slot(role)]
    }

    /// Toggle the role. No active id, or an id that is not a playable file, changes nothing.
    pub fn play_chord<'a>(&mut self, role: Role, book: &'a Book) -> Effect<'a> {
        let Some(id) = book.index.active(role) else {
            return Effect::default();
        };
        let slot = role_slot(role);
        if self.roles[slot].is_some() {
            self.roles[slot] = None;
            return Effect {
                stopped: true,
                ..Effect::default()
            };
        }
        let Some(plan) = book.plan(id) else {
            return Effect::default();
        };
        if plan.warp.is_some() {
            self.warps[slot] += 1;
        }
        let remaining = passes(book.index.loop_count(role));
        let warp = plan.warp;
        self.roles[slot] = Some(Run {
            id: IdBuf::copy_from(id),
            chain: plan.chain,
            step: 0,
            frame: 0,
            remaining,
            emitted: false,
        });
        Effect {
            warp,
            started: true,
            ..Effect::default()
        }
    }

    /// Play one file once, whichever file the menu named. The role's active id and loop stay as they are.
    pub fn play_file<'a>(&mut self, role: Role, book: &'a Book, id: &str) -> Effect<'a> {
        let Some(plan) = book.plan(id) else {
            return Effect::default();
        };
        let slot = role_slot(role);
        if plan.warp.is_some() {
            self.warps[slot] += 1;
        }
        let warp = plan.warp;
        self.roles[slot] = Some(Run {
            id: IdBuf::copy_from(id),
            chain: plan.chain,
            step: 0,
            frame: 0,
            remaining: Some(1),
            emitted: false,
        });
        Effect {
            warp,
            started: true,
            ..Effect::default()
        }
    }

    /// The next frame of `role`, or nothing when that role is idle or has finished its count.
    /// A missing chain id is skipped and left in the chain. The id is an index into the book, not a copy.
    pub fn step<'a>(&mut self, role: Role, book: &'a Book) -> Option<Played<'a>> {
        let slot = role_slot(role);
        loop {
            let run = self.roles[slot]?;
            let id = run.id.as_str();
            let step_count = if run.chain {
                match book.chains.get(id) {
                    Some(chain) => chain.recordings.len(),
                    None => {
                        self.roles[slot] = None;
                        return None;
                    }
                }
            } else {
                1
            };
            if run.step >= step_count {
                if !run.emitted || matches!(run.remaining, Some(0 | 1)) {
                    self.roles[slot] = None;
                    return None;
                }
                let run = self.roles[slot].as_mut().unwrap();
                if let Some(left) = run.remaining {
                    run.remaining = Some(left - 1);
                }
                run.step = 0;
                run.frame = 0;
                run.emitted = false;
                continue;
            }
            let step_name = if run.chain {
                IdBuf::copy_from(&book.chains.get(id).unwrap().recordings[run.step])
            } else {
                run.id
            };
            let at = run.frame;
            let frame = book
                .recordings
                .get(step_name.as_str())
                .and_then(|recording| recording.frames.get(at).copied());
            if let Some(frame) = frame {
                let recording_id = book
                    .recordings
                    .get_key_value(step_name.as_str())
                    .map(|(key, _)| key.as_str())
                    .unwrap_or("");
                let run = self.roles[slot].as_mut().unwrap();
                run.frame = at + 1;
                run.emitted = true;
                return Some(Played {
                    recording_id,
                    frame_index: at,
                    frame,
                });
            }
            let run = self.roles[slot].as_mut().unwrap();
            run.step += 1;
            run.frame = 0;
        }
    }

    /// Stop a playing role when mash interrupt is on. A chain run stops as one macro.
    pub fn mash(&mut self, role: Role, interrupt: bool) -> bool {
        let slot = role_slot(role);
        if interrupt && self.roles[slot].is_some() {
            self.roles[slot] = None;
            true
        } else {
            false
        }
    }

    /// Load names the bound save and does not touch either run.
    pub fn load<'a>(&self, book: &'a Book, settings: &StateSettings) -> Option<DeathWarp<'a>> {
        bound_warp(book, settings)
    }

    /// Auto-load on death names the bound save when the row is on, and nothing when none is bound.
    pub fn auto_load_on_death<'a>(&self, book: &'a Book, settings: &StateSettings) -> Option<DeathWarp<'a>> {
        if !settings.autoload {
            return None;
        }
        bound_warp(book, settings)
    }
}

fn role_slot(role: Role) -> usize {
    match role {
        Role::Player => 0,
        Role::Cpu => 1,
    }
}

fn passes(count: LoopCount) -> Option<u32> {
    match count {
        LoopCount::Infinite => None,
        LoopCount::Finite(count) => Some(u32::from(count)),
    }
}

fn bound_warp<'a>(book: &'a Book, settings: &StateSettings) -> Option<DeathWarp<'a>> {
    if !settings.enable {
        return None;
    }
    let id = book.index.bound()?;
    if !book.saves.contains(id) {
        return None;
    }
    Some(DeathWarp {
        save_id: id,
        mirroring: settings.mirroring,
        damage_player: settings.damage_player,
        damage_cpu: settings.damage_cpu,
        limits_player: settings.limits_player,
        limits_cpu: settings.limits_cpu,
        buffs: settings.buffs,
    })
}

fn recording_warp<'a>(recording: &'a MacroRecording, saves: &BTreeSet<String>) -> Option<&'a str> {
    let id = recording.start_save.as_deref()?;
    if !recording.start_save_on {
        return None;
    }
    linked(id, recording.start_save_on, saves)
}

fn chain_warp<'a>(chain: &Chain, book: &'a Book) -> Option<&'a str> {
    if !chain.start_save_on {
        return None;
    }
    let first = chain.recordings.first()?;
    let recording = book.recordings.get(first.as_str())?;
    let id = recording.start_save.as_deref()?;
    linked(id, true, &book.saves)
}

/// Where a named file's start save stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartLink<'a> {
    None,
    Linked(&'a str),
    Deleted,
}

fn link_of<'a>(recording: &'a MacroRecording, saves: &BTreeSet<String>) -> StartLink<'a> {
    let Some(id) = recording.start_save.as_deref() else {
        return StartLink::None;
    };
    if !recording.start_save_on {
        return StartLink::None;
    }
    if saves.contains(id) {
        StartLink::Linked(id)
    } else {
        StartLink::Deleted
    }
}

fn linked<'a>(id: &'a str, on: bool, saves: &BTreeSet<String>) -> Option<&'a str> {
    if start_save_state(Some(id), on, saves.contains(id)) == StartSaveState::Linked {
        Some(id)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    lib_unit_tests! {
    use super::*;
    use crate::engine::library::{RecordingIndex, Role};
    use crate::engine::macros::{InputFrame, MacroRecording, StartState};
    
    
    
    

    fn frames(buttons: &[u32]) -> Vec<InputFrame> {
        buttons
            .iter()
            .map(|buttons| InputFrame {
                buttons: *buttons,
                ..InputFrame::default()
            })
            .collect()
    }

    fn recording(name: &str, buttons: &[u32], save: Option<&str>, on: bool) -> MacroRecording {
        let mut recording = MacroRecording::new(
            name,
            "13.0.5",
            StartState::neutral(8, 0, 1.0),
            frames(buttons),
        )
        .unwrap();
        if let Some(id) = save {
            recording.set_start_save(Some(id)).unwrap();
            recording.set_start_save_on(on);
        }
        recording
    }

    fn the_play_chord_with_no_active_macro_does_nothing() {
        let mut index = RecordingIndex::fresh();
        index.set_active(Role::Cpu, Some("macro-a"), 1).unwrap();
        let mut book = Book::new(index);
        book.add_recording("macro-a", recording("A", &[7, 8], Some("save-a"), true));
        book.add_save("save-a");
        let mut machine = Playback::new();
        let effect = machine.play_chord(Role::Player, &book);
        assert_eq!(effect, Effect::default());
        assert!(!machine.playing(Role::Player));
        assert_eq!(machine.warps(Role::Player), 0);
        assert!(machine.play_chord(Role::Cpu, &book).started);
        let again = machine.play_chord(Role::Player, &book);
        assert_eq!(again, Effect::default());
        assert!(machine.playing(Role::Cpu));
        assert!(!machine.playing(Role::Player));
        assert_eq!(machine.step(Role::Cpu, &book).unwrap().frame.buttons, 7);
    }

    /// rc.9: this module's cases in one table. Each case runs on its own thread, as the test harness ran it.
    #[test]
    fn case_table() {
        crate::test_table::run(&[
            ("the_play_chord_with_no_active_macro_does_nothing", the_play_chord_with_no_active_macro_does_nothing as fn()),
        ]);
    }
    }
}
