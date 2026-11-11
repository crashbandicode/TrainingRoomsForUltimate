//! Value-only replay observations. The runtime supplies real simulation samples;
//! this module neither decodes proprietary replay bytes nor invents action state.
use {crate::engine::macros::FrameWindow, crate::engine::macros::InputFrame, crate::engine::macros::MacroRecording, crate::engine::macros::MoveStart, crate::engine::macros::StartState};

/// Input and start-context observation. Native simulation provides the preview;
/// no animation/pose samples are retained in the per-frame capture buffer.
#[derive(Clone, Copy, Debug)]
pub struct ReplayFrame {
    pub input: InputFrame,
    pub start: StartState,
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug)]
pub struct ReplayTrack {
    pub entry: i32,
    pub frames: Vec<ReplayFrame>,
    /// Some observed analog values needed rounding to controller resolution.
    pub quantized_analog: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ReplaySession {
    pub tracks: Vec<ReplayTrack>,
    pub truncated: bool,
    /// Frame indices (shared by every track) where play resumed after a stretch of the
    /// native record with no live play (the TIME! to sudden-death transition, a pause).
    /// Ascending; the state is discontinuous across each one.
    pub breaks: Vec<u32>,
}

impl ReplaySession {
    /// The break that a selection of frames `start..=end` crosses, if any: a
    /// recording made from it would play the inputs after the break from a state
    /// before it.
    pub fn break_within(&self, start: u32, end: u32) -> Option<u32> {
        self.breaks.iter().copied().find(|&b| start < b && b <= end)
    }

    /// Associate only values from the same immutable committed capture. Native
    /// CFMK row numbers and local live entry IDs are different namespaces.
    pub fn mii_for_track(&self, miis: &crate::engine::mii::replay::ReplayMiis,
        entry: i32, kind: u32) -> Result<crate::engine::mii::replay::RawMii, &'static str>
    {
        let raw = miis.unique_class(kind)?;
        let mut matching = self.tracks.iter().filter(|track|
            track.frames.iter().any(|frame| frame.start.fighter_kind as u32 == kind));
        let track = matching.next().ok_or("Capture has no track for this Mii class")?;
        if matching.next().is_some() { return Err("Capture has ambiguous same-class Mii tracks"); }
        if track.entry != entry || track.frames.iter().any(|frame| frame.start.fighter_kind as u32 != kind) {
            return Err("Capture Mii track identity changes or selection differs");
        }
        Ok(*raw)
    }
}

impl ReplayTrack {
    pub fn window(&self) -> Result<FrameWindow, String> {
        FrameWindow::new(self.frames.len() as u32).map_err(|e| e.to_string())
    }

    /// The game adapter supplies its verified idle/fall status IDs. There is no
    /// portable assumption that a numeric status represents an actionable state.
    pub fn previous_clean_start(&self, frame: u32, clean_statuses: &[i32]) -> Option<u32> {
        let limit = (frame as usize).min(self.frames.len().saturating_sub(1));
        (0..=limit).rev()
            .find(|&i| self.frames.get(i).is_some_and(|f| clean_statuses.contains(&f.start.status)))
            .map(|i| i as u32)
    }

    pub fn export(&self, window: &FrameWindow, name: &str, game_version: &str,
                  clean_statuses: &[i32]) -> Result<MacroRecording, String> {
        let first = self.frames.get(window.start() as usize).ok_or("No frame at In")?;
        if !clean_statuses.contains(&first.start.status) {
            return Err("In is mid-action. Snap to a move start before previewing or saving.".into());
        }
        self.export_inputs(window, name, game_version, first.start)
    }

    /// Explicitly restart a snapped input sequence from neutral. No source frame
    /// is rewritten and no animation/work-state restore is claimed. Ordinary
    /// macro JSON already persists this context and the exact selected inputs.
    pub fn export_move(&self, window: &FrameWindow, name: &str, game_version: &str,
        start: &MoveStart) -> Result<MacroRecording, String> {
        if start.frame != window.start() || start.action_frame < start.frame
            || start.action_frame >= window.end()
            || self.frames.get(start.action_frame as usize).is_none_or(|frame|
                frame.start.status != start.action_status || frame.start.fighter_kind != start.context.fighter_kind) {
            return Err("Move alignment changed; snap the start again".into());
        }
        self.export_inputs(window, name, game_version, start.context)
    }

    fn export_inputs(&self, window: &FrameWindow, name: &str, game_version: &str,
        start: StartState) -> Result<MacroRecording, String> {
        let range = window.range();
        let selected = self.frames.get(range.start as usize..range.end as usize)
            .ok_or("Window exceeds captured replay")?;
        if selected.iter().any(|f| f.start.fighter_kind != start.fighter_kind) {
            return Err("Window crosses a fighter transformation; select one fighter form.".into());
        }
        let inputs: Vec<InputFrame> = selected.iter().map(|f| f.input).collect();
        MacroRecording::new(name, game_version, start, inputs)
            .map_err(|e| e.to_string())
    }
}

